//! Helper for macOS' logging.

use std::ffi::{CStr, c_char, c_void};

// https://doc.rust-lang.org/nomicon/ffi.html#representing-opaque-structs
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct os_log_s {
    _data: (),
    _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

#[test]
fn test_opaque_zst() {
    assert_eq!(std::mem::size_of::<os_log_s>(), 0);
}

#[allow(non_camel_case_types)]
type os_log_t = *mut os_log_s;

unsafe extern "C" {
    unsafe fn os_log_create(subsystem: *const c_char, category: *const c_char) -> os_log_t;
    unsafe fn os_release(object: *mut c_void);

    unsafe fn rust_os_log(log: os_log_t, message: *const c_char);
    unsafe fn rust_os_log_error(log: os_log_t, message: *const c_char);
}

pub struct Logger {
    inner: os_log_t,
}

impl Drop for Logger {
    fn drop(&mut self) {
        unsafe {
            os_release(self.inner as *mut c_void);
        }
    }
}

impl Logger {
    pub fn new(subsystem: &CStr, category: &CStr) -> Self {
        // Always succeeds
        let inner = unsafe { os_log_create(subsystem.as_ptr(), category.as_ptr()) };

        Logger { inner }
    }

    pub fn log(&mut self, message: &CStr) {
        unsafe { rust_os_log(self.inner, message.as_ptr()) }
    }

    pub fn error(&mut self, message: &CStr) {
        unsafe { rust_os_log_error(self.inner, message.as_ptr()) }
    }
}
