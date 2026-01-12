use std::ffi::CStr;

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
    let error_message: &CStr = match error.daemon_error() {
        DaemonError::IoctlFDCreation => c"Failed to create the ioctl file descriptor",
        DaemonError::IndexLookup => c"Failed to lookup the index for awdl0",
        DaemonError::ReadFDCreation => c"Failed to create the read file descriptor",
        DaemonError::ReadFDNonBlocking => c"Failed to set read file descriptor to be non-blocking",
        DaemonError::KqueueFileDescriptor => c"Failed to initialize kqueue",
        DaemonError::AddReadFD => c"Failed to register for read events",
        DaemonError::AddSigint => c"Failed to register SIGINT",
        DaemonError::AddSigterm => c"Failed to register SIGTERM",
        DaemonError::Poll => c"Error during polling",
        DaemonError::Read => c"Failed to read route message",
        DaemonError::Disable => c"Failed to disable awdl0",
        DaemonError::ControllerStatusStart => c"Initial check for awdl0 status failed",
        DaemonError::ControllerStatusEnd => c"Failed to check awdl0 status during exit",
        DaemonError::InitialDisable => c"Error when first disabling awdl0",
        DaemonError::ReenableOnExit => c"Error when re-enabling awdl0",
        DaemonError::UnknownSignal => c"Received an event for an unknown signal",
        DaemonError::UnknownEvent => c"Received an unknown event",
    };

    match error.daemon_error() {
        // These two don't have a corresponding errno
        DaemonError::UnknownSignal | DaemonError::UnknownEvent => {
            logger.error(error_message);
        }
        _ => {
            logger.error_with_errno(error_message, error.posix_error().code());
        }
    }
}
