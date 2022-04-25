extern crate libc;

use std::ffi::{c_char, CString};

pub fn pledge(promises: &str) -> std::io::Result<()> {
    let promises_c_string = CString::new(promises).unwrap();
    let promises_ptr = promises_c_string.as_ptr();
    if unsafe { libc::pledge(promises_ptr, 0 as *const c_char) } < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}


pub fn pledge_with_execpromises(promises: &str, execpromises: &str) -> std::io::Result<()> {
    let promises_c_string = CString::new(promises).unwrap();
    let promises_ptr = promises_c_string.as_ptr();
    let execpromises_c_string = CString::new(execpromises).unwrap();
    let execpromises_ptr = execpromises_c_string.as_ptr();
    if unsafe { libc::pledge(promises_ptr, execpromises_ptr) } < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn unveil(path: &str, permissions: &str) -> std::io::Result<()> {
    let path_c_string = CString::new(path).unwrap();
    let path_ptr = path_c_string.as_ptr();
    let permissions_c_string = CString::new(permissions).unwrap();
    let permissions_ptr = permissions_c_string.as_ptr();
    if unsafe { libc::unveil(path_ptr, permissions_ptr) } < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn lock_veil() -> std::io::Result<()> {
    if unsafe { libc::unveil(0 as *const c_char, 0 as *const std::ffi::c_char) } < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
