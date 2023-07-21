#![allow(dead_code)]

use crate::vimba_sys::*;
use crate::feature::*;
use crate::error::Error;
use crate::vimba::Vimba;
use crate::util::pointer_to_str;
use crate::{Result, vmbcall, Flow};
use std::mem;
use std::sync::mpsc;
use std::pin::Pin;
use bitflags::bitflags;



const FRAME_SIZE: u32 = mem::size_of::<VmbFrame_t>() as u32;



bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AccessMode: u32 {
        const NONE = VmbAccessModeType::VmbAccessModeNone;
        const FULL = VmbAccessModeType::VmbAccessModeFull;
        const READ = VmbAccessModeType::VmbAccessModeRead;
        const CONFIG = VmbAccessModeType::VmbAccessModeConfig;
        const LITE = VmbAccessModeType::VmbAccessModeLite;
    }
}



#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub id: String,
    pub name: String,
    pub serial: String,
    pub model_name: String,
    pub interface_id: String,
    pub access_mode: AccessMode,
}

impl CameraInfo {
    pub fn from_c_struct(info: VmbCameraInfo_t) -> Self {
        Self {
            id: unsafe { pointer_to_str(info.cameraIdString).to_string() },
            name: unsafe { pointer_to_str(info.cameraName).to_string() },
            serial: unsafe { pointer_to_str(info.serialString).to_string() },
            model_name: unsafe { pointer_to_str(info.modelName).to_string() },
            interface_id: unsafe { pointer_to_str(info.interfaceIdString).to_string() },
            access_mode: AccessMode::from_bits_truncate(info.permittedAccess)
        }
    }
}




pub trait CameraCallback: Send + FnMut(&[u8]) -> Flow {}

impl<T> CameraCallback for T where T: Send + FnMut(&[u8]) -> Flow {}

struct CameraCallbackContext {
    handler: Box<dyn CameraCallback>,
    frame: VmbFrame_t,
    buffer: Vec::<u8>,
    stop_tx: mpsc::Sender::<()>,
    stop_rx: mpsc::Receiver::<()>,
    stopped: bool
}



pub struct Camera<'a> {
    vimba: &'a Vimba,
    handle: VmbHandle_t,
    open: bool,
    cb_ctx: Option<Pin<Box<CameraCallbackContext>>>
}

impl<'a> Camera<'a> {
    pub fn from_handle(handle: VmbHandle_t, vimba: &'a Vimba) -> Self {
        Self { vimba, handle, open: true, cb_ctx: None }
    }

    pub fn close(&mut self) -> Result<()> {
        if self.open {
            let res = vmbcall!(VmbCameraClose, self.handle);

            if res.is_ok() {
                self.open = false;
                self.cb_ctx = None;
            }
            
            res
        }
        else { Ok(()) }
    }

    pub fn get_frame(&mut self) -> Result<Vec<u8>> {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();

        self.stream(move |frame| {
            tx.send(frame.to_vec()).unwrap();
            Flow::Break
        })?;

        Ok(rx.recv().unwrap())
    }
    
    // This is the most horrible thing I have ever written. God bless.
    pub fn start_streaming<F>(&mut self, handler: F) -> Result<()>
    where F: CameraCallback + 'static {
        if self.cb_ctx.is_some() { return Err(Error::DeviceBusy) }
        
        let size = self.get_feature_int("PayloadSize")?;
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        // All this stuff mustn't move while the streaming thread is running so,
        // regrettably, it does need to be enclosed in a pin.
        let mut cb_ctx = Box::pin(CameraCallbackContext {
            handler: Box::new(handler),
            frame: VmbFrame_t::default(),
            buffer: vec![0u8; size as usize],
            stop_tx,
            stop_rx,
            stopped: false
        });

        // Now that it's pinned, we can take pointers without worrying about them
        // becoming invalid later on. The context pointers will be read inside the
        // streaming thread to reference things on the rust side from the C side.
        cb_ctx.frame.buffer = cb_ctx.buffer.as_mut_ptr() as *mut std::ffi::c_void;
        cb_ctx.frame.bufferSize = size as u32;
        cb_ctx.frame.context[0] = cb_ctx.handler.as_mut() as *mut dyn CameraCallback
                                                          as *mut std::ffi::c_void;
        cb_ctx.frame.context[1] = &mut cb_ctx.stop_rx as *mut _ as *mut std::ffi::c_void;
        cb_ctx.frame.context[2] = &mut cb_ctx.stopped as *mut bool as *mut std::ffi::c_void;
        
        // This is the actual Vimba callback. It'll run the given handler until it
        // returns Flow::Break, or until we tell streaming to stop by sending ()
        // down the stop_tx --> stop_rx channel.
        unsafe extern "C" fn wrapper<F>(cam: VmbHandle_t, frame: *mut VmbFrame_t)
        where F: CameraCallback {
            let stopped = (*frame).context[2] as *mut bool;
            let stop_rx = &mut *((*frame).context[1] as *mut mpsc::Receiver::<()>);
            
            if stop_rx.try_recv() == Ok(()) { *stopped = true; }
            if *stopped { return; }

            let handler = &mut *((*frame).context[0] as *mut F);
            let buffer_size = (*frame).bufferSize as usize;
            let buffer_ptr = (*frame).buffer as *const u8;
            let buffer = std::slice::from_raw_parts(buffer_ptr, buffer_size);
            
            if handler(buffer) == Flow::Break {
                *stopped = true;
            }
            else {
                vmbcall!(VmbCaptureFrameQueue, cam, frame, Some(wrapper::<F>)).unwrap();
            }
        }
        
        // Want the AcquisitionStatus feature to hold whether we're acquiring, and
        // since we're intending to stream, the acquisition had better be continuous.
        self.set_feature_enum("AcquisitionStatusSelector", "AcquisitionActive")?;
        self.set_feature_enum("AcquisitionMode", "Continuous")?;
        
        // Tell Vimba the frame exists, then enter capture mode and queue this frame
        // to be the next one an image should be put into.
        vmbcall!(VmbFrameAnnounce, self.handle, &cb_ctx.frame, FRAME_SIZE)?;
        vmbcall!(VmbCaptureStart, self.handle)?;
        vmbcall!(VmbCaptureFrameQueue, self.handle, &cb_ctx.frame, Some(wrapper::<F>))?;

        // Save the callback context so it exists while streaming
        self.cb_ctx = Some(cb_ctx);
        
        // Try to start acquiring images. If it fails, deallocate the context.
        let res = self.run_command("AcquisitionStart");

        if res.is_err() { self.cb_ctx = None; }

        res
    }

    pub fn stop_streaming(&mut self) -> Result<()> {
        if let Some(cb_ctx) = &self.cb_ctx {
            // Send a message telling the streaming callback to stop executing
            // the handler. Ideally this shouldn't be necessary; we should be able
            // just to run AcquisitionStop and then wait for it to stop. But doing
            // that can sometimes cut the callback off halfway through, while it's
            // still modifying data, which is very bad.
            cb_ctx.stop_tx.send(()).expect("Couldn't send to streaming thread");
            
            self.run_command("AcquisitionStop")?;
            
            // AcquisitionStatusMode was set to AcquisitionActive previously, so we
            // can now check AcquisitionStatus to sleep until acquisition is done.
            while self.get_feature_bool("AcquisitionStatus")? {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            
            // End the capture and tell Vimba the frame can't be used any more
            vmbcall!(VmbCaptureEnd, self.handle)?;
            vmbcall!(VmbFrameRevoke, self.handle, &cb_ctx.frame)?;
            
            // Deallocate the callback context
            self.cb_ctx = None;
        }

        Ok(())
    }

    pub fn stream<F: CameraCallback + 'static>(&mut self, mut handler: F) -> Result<()> {
        let (tx, rx) = mpsc::channel::<()>();
        let wrapper = move |frame: &[u8]| {
            let action = handler(frame);

            if action == Flow::Break { tx.send(()).unwrap(); }

            action
        };

        self.start_streaming(wrapper)?;
        rx.recv().unwrap();
        self.stop_streaming()
    }

    /*pub fn stream<F: CameraCallback>(&self, mut handler: F) -> Result<()> {
        if self.cb_ctx.is_some() { return Err(Error::DeviceBusy) }

        let size = self.get_feature_int("PayloadSize")?;
        let mut frame = VmbFrame_t::default();
        let mut buffer = vec![0u8; size as usize];
        let (mut stop_tx, stop_rx) = mpsc::channel::<()>();
        let mut stopped = false;
        
        frame.buffer = buffer.as_mut_ptr() as *mut std::ffi::c_void;
        frame.bufferSize = size as u32;
        frame.context[0] = &mut handler as *mut F as *mut std::ffi::c_void;
        frame.context[1] = &mut stop_tx as *mut _ as *mut std::ffi::c_void;
        frame.context[2] = &mut stopped as *mut bool as *mut std::ffi::c_void;

        unsafe extern "C" fn wrapper<F>(cam: VmbHandle_t, frame: *mut VmbFrame_t)
        where F: CameraCallback {
            let stopped = (*frame).context[2] as *mut bool;

            if *stopped { return; }

            let handler = &mut *((*frame).context[0] as *mut F);
            let stop_tx = &mut *((*frame).context[1] as *mut mpsc::Sender::<()>);
            let buffer_size = (*frame).bufferSize as usize;
            let buffer_ptr = (*frame).buffer as *const u8;
            let buffer = std::slice::from_raw_parts(buffer_ptr, buffer_size);
            
            let action = handler(buffer);

            if *stopped == false && action == Flow::Break {
                *stopped = true;
                stop_tx.send(()).unwrap()
            }

            vmbcall!(VmbCaptureFrameQueue, cam, frame, Some(wrapper::<F>)).unwrap();
        }
        
        self.set_feature_enum("AcquisitionStatusSelector", "AcquisitionActive")?;
        self.set_feature_enum("AcquisitionMode", "Continuous")?;

        vmbcall!(VmbFrameAnnounce, self.handle, &frame, FRAME_SIZE)?;
        vmbcall!(VmbCaptureStart, self.handle)?;
        vmbcall!(VmbCaptureFrameQueue, self.handle, &frame, Some(wrapper::<F>))?;

        self.run_command("AcquisitionStart")?;
        stop_rx.recv().unwrap();
        self.run_command("AcquisitionStop")?;

        while self.get_feature_bool("AcquisitionStatus")? {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        
        vmbcall!(VmbCaptureEnd, self.handle)?;

        Ok(())
    }*/
}

impl<'a> HasFeatures for Camera<'a> {
    fn get_feature(&self, name: &str) -> Result<FeatureValue> {
        self.handle.get_feature(name)
    }

    fn set_feature(&self, name: &str, value: FeatureValue) -> Result<()> {
        self.handle.set_feature(name, value)
    }

    fn list_features(&self) -> Result<Vec<FeatureInfo>> {
        self.handle.list_features()
    }

    fn run_command(&self, name: &str) -> Result<()> {
        self.handle.run_command(name)
    }

    fn is_command_done(&self, name: &str) -> Result<bool> {
        self.handle.is_command_done(name)
    }
}

impl<'a> Drop for Camera<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            panic!("Could not close camera during drop due to error: {e:?}");
        }
    }
}
