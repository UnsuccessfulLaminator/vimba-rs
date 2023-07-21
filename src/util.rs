use std::ffi::CStr;



pub unsafe fn pointer_to_str<'a>(p: *const i8) -> &'a str {
    CStr::from_ptr(p).to_str().expect("Vimba returned bad (non-UTF8) string data")
}
