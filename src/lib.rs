// Private modules
mod vimba_sys;
mod vimba;
mod error;
mod util;
mod format;

// Public modules
pub mod camera;
pub mod feature;



pub use error::Error;
pub use vimba::Vimba;
pub use format::PixelFormat;

pub type Result<T> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! vmbcall {
    ($func: ident $(, $arg: expr)*) => {
        {
            use crate::error::error_code_to_result;
            error_code_to_result(unsafe { $func($($arg),*) })
        }
    }
}



pub mod prelude {
    pub use crate::feature::HasFeatures;
    pub use crate::vimba::Vimba;
    pub use crate::camera::{Camera, AccessMode, Frame, StreamContinue};
    pub use crate::format::PixelFormat;
}
