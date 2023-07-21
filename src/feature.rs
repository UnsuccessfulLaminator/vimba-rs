#![allow(non_upper_case_globals,dead_code)]

use crate::vimba_sys::*;
use crate::util::pointer_to_str;
use crate::{vmbcall, Result};
use std::ffi::{CStr, CString};
use std::{ptr, mem};
use bitflags::bitflags;
use enum_as_inner::EnumAsInner;



const FEATURE_INFO_SIZE: u32 = mem::size_of::<VmbFeatureInfo_t>() as u32;



#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeatureType {
    Int, Float, Enum, String, Bool, Command, Raw, None
}

impl TryFrom<u32> for FeatureType {
    type Error = ();

    fn try_from(v: u32) -> std::result::Result<Self, Self::Error> {
        use VmbFeatureDataType::*;
        use FeatureType::*;

        match v {
            VmbFeatureDataInt => Ok(Int),
            VmbFeatureDataFloat => Ok(Float),
            VmbFeatureDataEnum => Ok(Enum),
            VmbFeatureDataString => Ok(String),
            VmbFeatureDataBool => Ok(Bool),
            VmbFeatureDataCommand => Ok(Command),
            VmbFeatureDataRaw => Ok(Raw),
            VmbFeatureDataNone => Ok(None),
            _ => Err(())
        }
    }
}



#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum FeatureValue<'a> {
    Int(i64),
    Float(f64),
    Enum(&'a str),
    String(String),
    Bool(bool),
    Raw(Vec<u8>)
}

impl<'a> FeatureValue<'a> {
    pub fn feature_type(&self) -> FeatureType {
        match self {
            FeatureValue::Int(_) => FeatureType::Int,
            FeatureValue::Float(_) => FeatureType::Float,
            FeatureValue::Enum(_) => FeatureType::Enum,
            FeatureValue::String(_) => FeatureType::String,
            FeatureValue::Bool(_) => FeatureType::Bool,
            FeatureValue::Raw(_) => FeatureType::Raw
        }
    }
}



bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FeatureFlag: u32 {
        const NONE = VmbFeatureFlagsType::VmbFeatureFlagsNone;
        const READ = VmbFeatureFlagsType::VmbFeatureFlagsRead;
        const WRITE = VmbFeatureFlagsType::VmbFeatureFlagsWrite;
        const VOLATILE = VmbFeatureFlagsType::VmbFeatureFlagsVolatile;
        const MODIFY_WRITE = VmbFeatureFlagsType::VmbFeatureFlagsModifyWrite;
    }
}



#[derive(Debug, Clone)]
pub struct FeatureInfo {
    pub name: String,
    pub data_type: FeatureType,
    pub flags: FeatureFlag
}

impl FeatureInfo {
    pub fn from_c_struct(info: VmbFeatureInfo_t) -> Self {
        Self {
            name: unsafe { pointer_to_str(info.name).to_string() },
            data_type: match FeatureType::try_from(info.featureDataType) {
                Ok(t) => t,
                Err(_) => panic!("Unknown Vimba feature type {}", info.featureDataType)
            },
            flags: FeatureFlag::from_bits_truncate(info.featureFlags)
        }
    }
}



pub trait HasFeatures {
    fn list_features(&self) -> Result<Vec<FeatureInfo>>;
    fn get_feature(&self, name: &str) -> Result<FeatureValue>;
    fn set_feature(&self, name: &str, value: FeatureValue) -> Result<()>;
    fn run_command(&self, name: &str) -> Result<()>;
    fn is_command_done(&self, name: &str) -> Result<bool>;

    fn set_feature_int(&self, name: &str, v: i64) -> Result<()> {
        self.set_feature(name, FeatureValue::Int(v))
    }

    fn set_feature_float(&self, name: &str, v: f64) -> Result<()> {
        self.set_feature(name, FeatureValue::Float(v))
    }

    fn set_feature_enum(&self, name: &str, v: &str) -> Result<()> {
        self.set_feature(name, FeatureValue::Enum(v))
    }

    fn set_feature_string(&self, name: &str, v: &str) -> Result<()> {
        self.set_feature(name, FeatureValue::String(v.to_string()))
    }

    fn set_feature_bool(&self, name: &str, v: bool) -> Result<()> {
        self.set_feature(name, FeatureValue::Bool(v))
    }

    fn set_feature_raw(&self, name: &str, v: Vec<u8>) -> Result<()> {
        self.set_feature(name, FeatureValue::Raw(v))
    }

    fn get_feature_int(&self, name: &str) -> Result<i64> {
        Ok(self.get_feature(name)?.into_int().unwrap())
    }

    fn get_feature_float(&self, name: &str) -> Result<f64> {
        Ok(self.get_feature(name)?.into_float().unwrap())
    }

    fn get_feature_enum(&self, name: &str) -> Result<&str> {
        Ok(self.get_feature(name)?.into_enum().unwrap())
    }

    fn get_feature_string(&self, name: &str) -> Result<String> {
        Ok(self.get_feature(name)?.into_string().unwrap())
    }

    fn get_feature_bool(&self, name: &str) -> Result<bool> {
        Ok(self.get_feature(name)?.into_bool().unwrap())
    }

    fn get_feature_raw(&self, name: &str) -> Result<Vec<u8>> {
        Ok(self.get_feature(name)?.into_raw().unwrap())
    }
}

impl HasFeatures for VmbHandle_t {
    fn get_feature(&self, name: &str) -> Result<FeatureValue> {
        use FeatureValue::*;
        use VmbFeatureDataType::*;

        let name = CString::new(name).expect("name cannot have internal zeros");
        let name_ptr = name.as_ptr();
        let mut info = VmbFeatureInfo_t::default();

        vmbcall!(VmbFeatureInfoQuery, *self, name_ptr, &mut info, FEATURE_INFO_SIZE)?;

        match info.featureDataType {
            VmbFeatureDataInt => {
                let mut v: i64 = 0;
                vmbcall!(VmbFeatureIntGet, *self, name_ptr, &mut v)?;

                Ok(Int(v))
            },
            VmbFeatureDataFloat => {
                let mut v: f64 = 0.0;
                vmbcall!(VmbFeatureFloatGet, *self, name_ptr, &mut v)?;
                
                Ok(Float(v))
            },
            VmbFeatureDataEnum => {
                let mut v: *const i8 = ptr::null();
                vmbcall!(VmbFeatureEnumGet, *self, name_ptr, &mut v)?; 
                let cstr = unsafe { CStr::from_ptr(v) };

                Ok(Enum(cstr.to_str().unwrap()))
            },
            VmbFeatureDataString => {
                let mut len: u32 = 0;

                vmbcall!(
                    VmbFeatureStringGet,
                    *self, name_ptr, ptr::null_mut(), 0, &mut len
                )?;

                let mut buf = vec![0u8; len as usize];

                vmbcall!(
                    VmbFeatureStringGet,
                    *self, name_ptr, buf.as_mut_ptr() as *mut i8, len, ptr::null_mut()
                )?;

                match std::string::String::from_utf8(buf) {
                    Ok(s) => Ok(String(s)),
                    Err(_) => panic!("")
                }
            },
            VmbFeatureDataBool => {
                let mut v: VmbBool_t = 0;
                vmbcall!(VmbFeatureBoolGet, *self, name_ptr, &mut v)?;
                
                Ok(Bool(v != 0))
            },
            VmbFeatureDataRaw => {
                let mut len: u32 = 0;
                vmbcall!(VmbFeatureRawLengthQuery, *self, name_ptr, &mut len)?;

                let mut buf = vec![0u8; len as usize];
                let mut filled: u32 = 0;

                vmbcall!(
                    VmbFeatureRawGet,
                    *self, name_ptr, buf.as_mut_ptr() as *mut i8, len, &mut filled
                )?;

                Ok(Raw(buf))
            },
            _ => panic!("Unknown Vimba feature code {}", info.featureDataType)
        }
    }

    fn set_feature(&self, name: &str, value: FeatureValue) -> Result<()> {
        use FeatureValue::*;

        let name_cstr = CString::new(name).expect("name cannot have internal zeros");
        let name_ptr = name_cstr.as_ptr();

        match value {
            Int(v) => vmbcall!(VmbFeatureIntSet, *self, name_ptr, v),
            Float(v) => vmbcall!(VmbFeatureFloatSet, *self, name_ptr, v),
            Enum(v) => {
                let v = CString::new(v).expect("value cannot have internal zeros");
                
                vmbcall!(VmbFeatureEnumSet, *self, name_ptr, v.as_ptr())
            },
            String(v) => {
                let v = CString::new(v).expect("value cannot have internal zeros");
                
                vmbcall!(VmbFeatureStringSet, *self, name_ptr, v.as_ptr())
            },
            Bool(v) => vmbcall!(VmbFeatureBoolSet, *self, name_ptr, v as i8),
            Raw(v) => {
                let ptr = v.as_ptr() as *const i8;
                let len = v.len() as u32;

                vmbcall!(VmbFeatureRawSet, *self, name_ptr, ptr, len)
            }
        }
    }

    fn list_features(&self) -> Result<Vec<FeatureInfo>> {
        let mut n: u32 = 0;
        
        vmbcall!(
            VmbFeaturesList,
            *self, ptr::null_mut(), 0, &mut n, FEATURE_INFO_SIZE
        )?;

        let mut features = vec![VmbFeatureInfo_t::default(); n as usize];
        
        vmbcall!(
            VmbFeaturesList,
            *self, features.as_mut_ptr(), n, ptr::null_mut(), FEATURE_INFO_SIZE
        )?;

        Ok(features.into_iter().map(FeatureInfo::from_c_struct).collect())
    }
    
    fn run_command(&self, name: &str) -> Result<()> {
        let name_cstr = CString::new(name).expect("name cannot have internal zeros");
        let name_ptr = name_cstr.as_ptr();

        vmbcall!(VmbFeatureCommandRun, *self, name_ptr)
    }
    
    fn is_command_done(&self, name: &str) -> Result<bool> {
        let name_cstr = CString::new(name).expect("name cannot have internal zeros");
        let name_ptr = name_cstr.as_ptr();
        let mut done: i8 = 0;

        vmbcall!(VmbFeatureCommandIsDone, *self, name_ptr, &mut done)?;

        Ok(done != 0)
    }
}
