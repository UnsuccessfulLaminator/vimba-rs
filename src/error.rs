#![allow(non_upper_case_globals,dead_code)]

use crate::vimba_sys::VmbErrorType;
use crate::Result;
use std::fmt;



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

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        let msg = match *self {
            InternalFault => "internal fault",
            ApiNotStarted => "API not started (open Vimba context needed)",
            NotFound => "device or feature not found",
            BadHandle => "invalid handle",
            DeviceNotOpen => "device not open",
            InvalidAccess => "invalid access (due to access mode or current API state)",
            BadParameter => "invalid parameter value",
            StructSize => "invalid struct size for this version of Vimba",
            MoreData => "not all data was read",
            WrongType => "incorrect feature type",
            InvalidValue => "invalid feature value (out of range or bad increment)",
            Timeout => "timed out",
            Other => "other unknown error",
            Resources => "resources (e.g. memory) not available",
            InvalidCall => "call is invalid in the current context",
            NoTL => "transport layer(s) not found",
            NotImplemented => "not implemented",
            NotSupported => "not supported",
            Incomplete => "operation was not completed",
            IO => "transport layer I/O error",

            // My additional errors
            DeviceBusy => "device busy"
        };

        write!(fmt, "Vimba error {:?}: {}", self, msg)
    }
}
