#![allow(dead_code)]

use crate::vimba_sys::*;
use crate::feature::*;
use crate::error::Error;
use crate::vimba::VimbaContext;
use crate::util::pointer_to_str;
use crate::{Result, vmbcall};
use std::mem;
use std::sync::mpsc;
use std::pin::Pin;
use std::sync::Arc;
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
    pub(crate) fn from_c_struct(info: VmbCameraInfo_t) -> Self {
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



pub struct Frame<T: AsRef<[u8]>> {
    pub data: T,
    pub width: usize,
    pub height: usize,
    pub offset_x: usize,
    pub offset_y: usize,
    pub id: u64,
    pub timestamp: u64
}

impl<T: AsRef<[u8]>> Frame<T> {
    fn from_c_struct(frame: &VmbFrame_t, data: T) -> Self {
        Self {
            data,
            width: frame.width as usize,
            height: frame.height as usize,
            offset_x: frame.offsetX as usize,
            offset_y: frame.offsetY as usize,
            id: frame.frameID,
            timestamp: frame.timestamp
        }
    }

    pub fn map_data<'a, U, F>(&'a self, f: F) -> Frame<U>
    where U: AsRef<[u8]>, F: FnOnce(&'a T) -> U {
        Frame::<U> {
            data: f(&self.data),
            width: self.width,
            height: self.height,
            offset_x: self.offset_x,
            offset_y: self.offset_y,
            id: self.id,
            timestamp: self.timestamp
        }
    }

    pub fn with_ref_data(&self) -> Frame<&[u8]> {
        self.map_data(AsRef::as_ref)
    }

    pub fn with_vec_data(&self) -> Frame<Vec<u8>> {
        self.map_data(|data| data.as_ref().to_vec())
    }
}

impl Frame<&[u8]> {
    unsafe fn from_c_struct_ref_data(frame: &VmbFrame_t) -> Self {
        let buffer_ptr = frame.buffer as *const u8;
        let buffer_size = frame.bufferSize as usize;
        let data = std::slice::from_raw_parts(buffer_ptr, buffer_size);

        Self::from_c_struct(frame, data)
    }
}

impl<T> Default for Frame<T> where T: Default + AsRef<[u8]> {
    fn default() -> Self {
        Self {
            data: T::default(),
            width: 0,
            height: 0,
            offset_x: 0,
            offset_y: 0,
            id: 0,
            timestamp: 0
        }
    }
}



pub trait CameraCallback: Send + FnMut(Frame<&[u8]>) -> StreamContinue {}

impl<T> CameraCallback for T where T: Send + FnMut(Frame<&[u8]>) -> StreamContinue {}

struct CameraCallbackContext {
    handler: Box<dyn CameraCallback>,
    frames: Vec<VmbFrame_t>,
    buffers: Vec<Vec<u8>>,
    stop_tx: mpsc::Sender<()>,
    stop_rx: mpsc::Receiver<()>,
    stopped: bool
}



#[derive(PartialEq, Clone, Copy, Debug)]
pub struct StreamContinue(pub bool);



pub struct Camera {
    vimba_ctx: Arc<VimbaContext>,
    handle: VmbHandle_t,
    open: bool,
    cb_ctx: Option<Pin<Box<CameraCallbackContext>>>
}

impl Camera {
    pub(crate) fn from_handle(handle: VmbHandle_t, vimba_ctx: Arc<VimbaContext>) -> Self {
        Self { vimba_ctx, handle, open: true, cb_ctx: None }
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

    pub fn get_frame(&mut self) -> Result<Frame<Vec<u8>>> {
        let (tx, rx) = mpsc::channel::<Frame<_>>();
        let handler = move |frame: Frame<&[u8]>| {
            tx.send(frame.with_vec_data()).unwrap();

            StreamContinue(false)
        };
        
        // Using 2 buffers here in case streaming doesn't stop fast enough
        self.stream(handler, 2)?;

        Ok(rx.recv().unwrap())
    }
    
    // This is the most horrible thing I have ever written. God bless.
    pub fn start_streaming<F>(&mut self, handler: F, buffers: usize) -> Result<()>
    where F: CameraCallback + 'static {
        if self.cb_ctx.is_some() { return Err(Error::DeviceBusy) }
        
        let size = self.get_feature_int("PayloadSize")?;
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        // All this stuff mustn't move while the streaming thread is running so,
        // regrettably, it does need to be enclosed in a pin.
        let mut cb_ctx = Box::pin(CameraCallbackContext {
            handler: Box::new(handler),
            frames: vec![VmbFrame_t::default(); buffers],
            buffers: vec![vec![0u8; size as usize]; buffers],
            stop_tx,
            stop_rx,
            stopped: false
        });

        // Now that it's pinned, we can take pointers without worrying about them
        // becoming invalid later on. The context pointers will be read inside the
        // streaming thread to reference things on the rust side from the C side.
        let stop_rx_ptr = &mut cb_ctx.stop_rx as *mut _ as *mut std::ffi::c_void;
        let stopped_ptr = &mut cb_ctx.stopped as *mut bool as *mut std::ffi::c_void;
        let handler_ptr = cb_ctx.handler.as_mut() as *mut dyn CameraCallback
                                                  as *mut std::ffi::c_void;

        for i in 0..buffers {
            cb_ctx.frames[i].buffer = cb_ctx.buffers[i].as_mut_ptr() as *mut std::ffi::c_void;
            cb_ctx.frames[i].bufferSize = size as u32;
            cb_ctx.frames[i].context[0] = handler_ptr;
            cb_ctx.frames[i].context[1] = stop_rx_ptr;
            cb_ctx.frames[i].context[2] = stopped_ptr;
            
            // Tell vimba this frame exists
            vmbcall!(VmbFrameAnnounce, self.handle, &cb_ctx.frames[i], FRAME_SIZE)?;
        }
        
        // This is the actual Vimba callback. It'll run the given handler until it
        // returns StreamContinue(false), or until we tell streaming to stop by sending
        // a () down the stop_tx --> stop_rx channel.
        unsafe extern "C" fn wrapper<F>(cam: VmbHandle_t, frame: *mut VmbFrame_t)
        where F: CameraCallback {
            let stopped = (*frame).context[2] as *mut bool;
            let stop_rx = &mut *((*frame).context[1] as *mut mpsc::Receiver::<()>);
            
            if stop_rx.try_recv() == Ok(()) { *stopped = true; }
            if *stopped { return; }

            let handler = &mut *((*frame).context[0] as *mut F);
            let frame_rs = Frame::from_c_struct_ref_data(&*frame);
            
            if handler(frame_rs) == StreamContinue(false) {
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
        
        // Enter capture mode and queue all the frames to be filled in order
        vmbcall!(VmbCaptureStart, self.handle)?;

        for frame in &cb_ctx.frames {
            vmbcall!(VmbCaptureFrameQueue, self.handle, frame, Some(wrapper::<F>))?;
        }

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
            
            // End the capture and flush out any remaining queued frames. Flushing
            // is needed because trying to revoke a queued frame will cause Vimba
            // to emit its very cryptic "Other" error.
            vmbcall!(VmbCaptureEnd, self.handle)?;
            vmbcall!(VmbCaptureQueueFlush, self.handle)?;

            // Tell Vimba these frames cannot be used any more
            for frame in &cb_ctx.frames {
                vmbcall!(VmbFrameRevoke, self.handle, frame)?;
            }
            
            // Deallocate the callback context
            self.cb_ctx = None;
        }

        Ok(())
    }

    pub fn stream<F: CameraCallback + 'static>(&mut self, mut handler: F, buffers: usize)
    -> Result<()> {
        let (tx, rx) = mpsc::channel::<()>();
        let wrapper = move |frame: Frame<&[u8]>| {
            let action = handler(frame);

            if action == StreamContinue(false) { tx.send(()).unwrap(); }

            action
        };

        self.start_streaming(wrapper, buffers)?;
        rx.recv().unwrap();
        self.stop_streaming()
    }

    pub fn start_streaming_queue(&mut self, sender: mpsc::Sender<Frame<Vec<u8>>>, buffers: usize)
    -> Result<()> {
        let handler = move |frame: Frame<&[u8]>| {
            let res = sender.send(frame.with_vec_data());
            StreamContinue(res.is_ok())
        };

        self.start_streaming(handler, buffers)
    }
}

impl HasFeatures for Camera {
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

impl Drop for Camera {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            panic!("Could not close camera during drop due to error: {e:?}");
        }
    }
}
