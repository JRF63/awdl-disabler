use std::{ffi::CStr, fmt::Write};

use crate::os_log::Logger;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosixError {
    inner: libc::c_int,
}

impl PosixError {
    #[cold]
    pub const fn from_raw(code: libc::c_int) -> Self {
        PosixError { inner: code }
    }

    pub fn code(&self) -> libc::c_int {
        self.inner
    }

    pub fn last() -> Self {
        let inner = unsafe { *libc::__error() };
        PosixError { inner }
    }
}

pub struct Error {
    posix_error: PosixError,
    daemon_error: DaemonError,
}

#[test]
fn test_error_size() {
    assert_eq!(std::mem::size_of::<Error>(), std::mem::size_of::<u64>(),);
}

impl Error {
    #[cold]
    pub const fn new(posix_error: PosixError, daemon_error: DaemonError) -> Self {
        Error {
            posix_error,
            daemon_error,
        }
    }

    pub fn last(daemon_error: DaemonError) -> Self {
        Error::new(PosixError::last(), daemon_error)
    }

    pub fn posix_error(&self) -> PosixError {
        self.posix_error
    }

    pub fn daemon_error(&self) -> DaemonError {
        self.daemon_error
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonError {
    IoctlFDCreation,
    IndexLookup,
    ReadFDCreation,
    ReadFDNonBlocking,
    KqueueFileDescriptor,
    AddReadFD,
    AddSigint,
    AddSigterm,
    Poll,
    Read,
    Disable,
    ControllerStatusStart,
    ControllerStatusEnd,
    InitialDisable,
    ReenableOnExit,
    UnknownSignal,
    UnknownEvent,
}

pub fn log_error(logger: &mut Logger, error: &Error) {
    let mut string_buffer = String::new();

    let err_str = match error.daemon_error() {
        DaemonError::IoctlFDCreation => "Failed to create the ioctl file descriptor",
        DaemonError::IndexLookup => "Failed to lookup the index for awdl0",
        DaemonError::ReadFDCreation => "Failed to create the read file descriptor",
        DaemonError::ReadFDNonBlocking => "Failed to set read file descriptor to be non-blocking",
        DaemonError::KqueueFileDescriptor => "Failed to initialize kqueue",
        DaemonError::AddReadFD => "Failed to register for read events",
        DaemonError::AddSigint => "Failed to register SIGINT",
        DaemonError::AddSigterm => "Failed to register SIGTERM",
        DaemonError::Poll => "Error during polling",
        DaemonError::Read => "Failed to read route message",
        DaemonError::Disable => "Failed to disable awdl0",
        DaemonError::ControllerStatusStart => "Initial check for awdl0 status failed",
        DaemonError::ControllerStatusEnd => "Failed to check awdl0 status during exit",
        DaemonError::InitialDisable => "Error when first disabling awdl0",
        DaemonError::ReenableOnExit => "Error when re-enabling awdl0",
        DaemonError::UnknownSignal => "Received an event for an unknown signal",
        DaemonError::UnknownEvent => "Received an unknown event",
    };

    string_buffer.push_str(err_str);

    let buffer = match error.daemon_error() {
        // These two don't have a corresponding errno; there's no need for further formatting
        DaemonError::UnknownSignal | DaemonError::UnknownEvent => {
            string_buffer.push('\0'); // Add nul terminator
            unsafe { string_buffer.as_mut_vec() }
        }

        // Append ": (<errno>) <errno description>"
        _ => {
            write!(&mut string_buffer, ": ({}) ", error.posix_error().code()).unwrap();
            unsafe {
                let buffer = string_buffer.as_mut_vec();

                let ptr = libc::strerror(error.posix_error().code());
                let cstr = CStr::from_ptr(ptr);
                let cstr_bytes = cstr.to_bytes_with_nul();
                buffer.extend_from_slice(cstr_bytes); // buffer is now also nul terminated
                buffer
            }
        }
    };

    let final_err_msg = unsafe { CStr::from_ptr(buffer.as_ptr() as *const i8) };
    logger.error(final_err_msg);
}
