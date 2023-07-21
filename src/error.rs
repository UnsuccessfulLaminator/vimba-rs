#![allow(non_upper_case_globals,dead_code)]

use crate::vimba_sys::VmbErrorType;
use crate::Result;



pub fn error_code_to_result(code: i32) -> Result<()> {
    if code == VmbErrorType::VmbErrorSuccess { Ok(()) }
    else {
        match Error::try_from(code) {
            Ok(e) => Err(e),
            Err(_) => panic!("Unknown Vimba error code {code}")
        }
    }
}



#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    // Vimba's own errors
    InternalFault,
    ApiNotStarted,
    NotFound,
    BadHandle,
    DeviceNotOpen,
    InvalidAccess,
    BadParameter,
    StructSize,
    MoreData,
    WrongType,
    InvalidValue,
    Timeout,
    Other,
    Resources,
    InvalidCall,
    NoTL,
    NotImplemented,
    NotSupported,
    Incomplete,
    IO,

    // My additional errors
    DeviceBusy
}

impl TryFrom<i32> for Error {
    type Error = ();

    fn try_from(v: i32) -> std::result::Result<Self, Self::Error> {
        use VmbErrorType::*;
        use Error::*;

        match v {
            VmbErrorInternalFault => Ok(InternalFault),
            VmbErrorApiNotStarted => Ok(ApiNotStarted),
            VmbErrorNotFound => Ok(NotFound),
            VmbErrorBadHandle => Ok(BadHandle),
            VmbErrorDeviceNotOpen => Ok(DeviceNotOpen),
            VmbErrorInvalidAccess => Ok(InvalidAccess),
            VmbErrorBadParameter => Ok(BadParameter),
            VmbErrorStructSize => Ok(StructSize),
            VmbErrorMoreData => Ok(MoreData),
            VmbErrorWrongType => Ok(WrongType),
            VmbErrorInvalidValue => Ok(InvalidValue),
            VmbErrorTimeout => Ok(Timeout),
            VmbErrorOther => Ok(Other),
            VmbErrorResources => Ok(Resources),
            VmbErrorInvalidCall => Ok(InvalidCall),
            VmbErrorNoTL => Ok(NoTL),
            VmbErrorNotImplemented => Ok(NotImplemented),
            VmbErrorNotSupported => Ok(NotSupported),
            VmbErrorIncomplete => Ok(Incomplete),
            VmbErrorIO => Ok(IO),
            _ => Err(())
        }
    }
}
