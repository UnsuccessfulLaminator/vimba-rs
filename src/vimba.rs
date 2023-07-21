use crate::vimba_sys::*;
use crate::feature::*;
use crate::camera::*;
use crate::{vmbcall, Result};
use std::ffi::CString;
use std::sync::atomic;
use std::{mem, ptr};



const VERSION_INFO_SIZE: u32 = mem::size_of::<VmbVersionInfo_t>() as u32;
const CAMERA_INFO_SIZE: u32 = mem::size_of::<VmbCameraInfo_t>() as u32;
const GLOBAL_HANDLE: VmbHandle_t = 1 as VmbHandle_t;






static VIMBA_CONTEXTS: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

pub struct Vimba;

impl Vimba {
    pub fn new() -> Result<Self> {
        vmbcall!(VmbStartup)?;
        VIMBA_CONTEXTS.fetch_add(1, atomic::Ordering::SeqCst);

        Ok(Self {})
    }

    pub fn get_version(&self) -> Result<String> {
        let mut version = VmbVersionInfo_t::default();
        
        vmbcall!(VmbVersionQuery, &mut version, VERSION_INFO_SIZE)?;

        Ok(format!("{}.{}.{}", version.major, version.minor, version.patch))
    }

    pub fn get_num_cameras(&self) -> Result<usize> {
        let mut n: u32 = 0;

        vmbcall!(VmbCamerasList, ptr::null_mut(), 0, &mut n, CAMERA_INFO_SIZE)?;

        Ok(n as usize)
    }

    pub fn list_cameras(&self) -> Result<Vec<CameraInfo>> {
        let mut n = self.get_num_cameras()? as u32;
        let mut cameras = vec![VmbCameraInfo_t::default(); n as usize];

        vmbcall!(VmbCamerasList, cameras.as_mut_ptr(), n, &mut n, CAMERA_INFO_SIZE)?;

        Ok(cameras.into_iter().map(CameraInfo::from_c_struct).collect())
    }

    pub fn open_camera(&self, id: &str, access_mode: AccessMode) -> Result<Camera> {
        let id = CString::new(id).expect("id cannot have internal zeros");
        let mut handle: VmbHandle_t = ptr::null_mut();

        vmbcall!(VmbCameraOpen, id.as_ptr(), access_mode.bits(), &mut handle)?;

        Ok(Camera::from_handle(handle, self))
    }
}

impl HasFeatures for Vimba {
    fn get_feature(&self, name: &str) -> Result<FeatureValue> {
        GLOBAL_HANDLE.get_feature(name)
    }

    fn set_feature(&self, name: &str, value: FeatureValue) -> Result<()> {
        GLOBAL_HANDLE.set_feature(name, value)
    }

    fn list_features(&self) -> Result<Vec<FeatureInfo>> {
        GLOBAL_HANDLE.list_features()
    }
    
    fn run_command(&self, name: &str) -> Result<()> {
        GLOBAL_HANDLE.run_command(name)
    }

    fn is_command_done(&self, name: &str) -> Result<bool> {
        GLOBAL_HANDLE.is_command_done(name)
    }
}

impl Drop for Vimba {
    fn drop(&mut self) {
        let prev_count = VIMBA_CONTEXTS.fetch_sub(1, atomic::Ordering::SeqCst);

        if prev_count == 1 { unsafe { VmbShutdown(); } }
    }
}
