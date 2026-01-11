#![allow(non_camel_case_types)]
use std::ffi::{c_char, c_void};

const OS_LOG_TYPE_DEFAULT: u8 = 0x00;
// const OS_LOG_TYPE_INFO: u8 = 0x01;
// const OS_LOG_TYPE_DEBUG: u8 = 0x02;
const OS_LOG_TYPE_ERROR: u8 = 0x10;
// const OS_LOG_TYPE_FAULT: u8 = 0x11;

macro_rules! os_log_call_with_format {
    ($fun:expr, $log:expr, $typ:expr, $fmt:tt $(, $arg:expr)*) => {
        if os_log_type_enabled($log, $typ) {
            const FMT_STR_BYTES: &[u8] = concat!($fmt, "\0").as_bytes();

            #[unsafe(link_section = "__TEXT,__oslogstring,cstring_literals")]
            static OS_FMT_STR: [u8; FMT_STR_BYTES.len()] =
                unsafe { *(FMT_STR_BYTES.as_ptr() as *const [u8; FMT_STR_BYTES.len()]) };

            const BUFFER_SIZE: usize = os_log_format_buffer_size!($fmt $(,$arg)*);
            let mut os_fmt_buf = AlignedBuffer::<BUFFER_SIZE>::new();
            os_log_format!(os_fmt_buf, $fmt $(,$arg)*);

            $fun(
                &raw mut __dso_handle as *mut c_void,
                $log,
                $typ,
                OS_FMT_STR.as_ptr() as *const c_char,
                os_fmt_buf.as_mut_ptr() as *mut u8,
                os_fmt_buf.len() as u32,
            );
        }
    };
}

macro_rules! os_log {
    ($log:expr, $fmt:tt $(, $arg:expr)*) => {
        os_log_call_with_format!(_os_log_impl, $log, OS_LOG_TYPE_DEFAULT, $fmt $(,$arg)*);
    };
}

macro_rules! os_log_error {
    ($log:expr, $fmt:tt $(, $arg:expr)*) => {
        os_log_call_with_format!(_os_log_error_impl, $log, OS_LOG_TYPE_ERROR, $fmt $(,$arg)*);
    };
}

macro_rules! os_log_format_buffer_size {
    ("%{public}s", $arg:expr) => {
        12
    };
    ($fmt:tt $(, $arg:expr)*) => {
        // TODO: Only works for `"%{public}s", message`
        compile_error!("os_log_format_buffer_size doesn't handle these arguments yet")
    };
}

macro_rules! os_log_format {
    ($os_fmt_buf:expr, "%{public}s", $arg:expr) => {
        $os_fmt_buf[0] = 0x02;
        $os_fmt_buf[1] = 0x01;
        $os_fmt_buf[2] = 0x22;
        $os_fmt_buf[3] = 0x08;

        let target = &raw mut $os_fmt_buf[4] as *mut *const c_char;
        target.write_unaligned($arg);
    };
    ($os_fmt_buf:expr, $fmt:tt $(, $arg:expr)*) => {
        // TODO: Only works for `"%{public}s", message`
        compile_error!("os_log_format doesn't handle these arguments yet")
    };
}

#[repr(align(16))]
struct AlignedBuffer<const N: usize>([u8; N]);

impl<const N: usize> std::ops::Deref for AlignedBuffer<N> {
    type Target = [u8; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> std::ops::DerefMut for AlignedBuffer<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> AlignedBuffer<N> {
    pub fn new() -> Self {
        let array = std::mem::MaybeUninit::uninit();
        Self(unsafe { array.assume_init() })
    }
}

// https://doc.rust-lang.org/nomicon/ffi.html#representing-opaque-structs
#[repr(C)]
pub struct os_log_s {
    _data: (),
    _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

#[test]
fn test_opaque_zst() {
    assert_eq!(std::mem::size_of::<os_log_s>(), 0);
}

type os_log_t = *mut os_log_s;
type os_log_type_t = u8;

#[repr(C)]
struct mach_header {
    magic: u32,
    cputype: i32,
    cpusubtype: i32,
    filetype: u32,
    ncmds: u32,
    sizeofcmds: u32,
    flags: u32,
}

unsafe extern "C" {
    unsafe fn os_log_create(subsystem: *const c_char, category: *const c_char) -> os_log_t;
    unsafe fn os_release(object: *mut c_void);

    unsafe fn os_log_type_enabled(oslog: os_log_t, typ: u8) -> bool;

    unsafe fn _os_log_impl(
        dso: *mut c_void,
        log: os_log_t,
        typ: os_log_type_t,
        format: *const c_char,
        buf: *mut u8,
        size: u32,
    );
    unsafe fn _os_log_error_impl(
        dso: *mut c_void,
        log: os_log_t,
        typ: os_log_type_t,
        format: *const c_char,
        buf: *mut u8,
        size: u32,
    );

    unsafe static mut __dso_handle: mach_header;
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
    pub fn new(subsystem: &std::ffi::CStr, category: &std::ffi::CStr) -> Self {
        // Always succeeds
        let inner = unsafe { os_log_create(subsystem.as_ptr(), category.as_ptr()) };

        Logger { inner }
    }

    pub fn log(&mut self, message: &std::ffi::CStr) {
        unsafe {
            os_log!(self.inner, "%{public}s", message.as_ptr());
        }
    }

    pub fn error(&mut self, message: &std::ffi::CStr) {
        unsafe {
            os_log_error!(self.inner, "%{public}s", message.as_ptr());
        }
    }
}
