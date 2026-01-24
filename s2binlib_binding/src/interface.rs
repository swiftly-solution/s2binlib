use std::ffi::CStr;
use std::ffi::{c_char, c_void};

use crate::compat::s2binlib001::s2binlib001_create;
use crate::compat::s2binlib002::s2binlib002_create;
use crate::compat::s2binlib003::s2binlib003_create;

#[unsafe(no_mangle)]
pub extern "C" fn S2BinLib_CreateInterface(name: *const c_char) -> *mut c_void {
    if name.is_null() {
        return std::ptr::null_mut();
    }

    let name_str = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };

    match name_str {
        "S2BINLIB001" => s2binlib001_create(),
        "S2BINLIB002" => s2binlib002_create(),
        "S2BINLIB003" => s2binlib003_create(),
        _ => std::ptr::null_mut(),
    }
}
