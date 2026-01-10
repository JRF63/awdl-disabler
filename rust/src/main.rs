mod awdl_controller;
mod error;
mod event;
mod os_log;

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use awdl_controller::AWDLController;
use error::{DaemonError, Error, PosixError};
use event::{EventQueue, PollAction};
use os_log::Logger;

fn err_mapper(daemon_error: DaemonError) -> impl Fn(PosixError) -> Error {
    move |posix_error| Error::new(posix_error, daemon_error)
}

fn disable_awdl(logger: &mut Logger) -> Result<(), Error> {
    let mut awdl_controller = AWDLController::new()?;

    // Disable awdl0 at program start
    {
        let (is_up, current_flags) = awdl_controller
            .current_status()
            .map_err(err_mapper(DaemonError::ControllerStatusStart))?;
        if is_up {
            awdl_controller
                .disable(current_flags)
                .map_err(err_mapper(DaemonError::InitialDisable))?;
            logger.log(c"Disabling awdl0");
        } else {
            logger.log(c"awdl0 already disabled");
        }
    }

    // Must be AF_ROUTE + SOCK_RAW and non-blocking
    let read_fd = {
        let raw_fd = unsafe { libc::socket(libc::AF_ROUTE, libc::SOCK_RAW, 0) };
        if raw_fd == -1 {
            return Err(Error::last(DaemonError::ReadFDCreation));
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        unsafe {
            if libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) == -1 {
                return Err(Error::last(DaemonError::ReadFDNonBlocking));
            }
        }
        fd
    };

    let mut event_queue = EventQueue::new_with_read_fd(&read_fd)?;
    event_queue
        .add_signal(libc::SIGINT)
        .map_err(err_mapper(DaemonError::AddSigint))?;
    event_queue
        .add_signal(libc::SIGTERM)
        .map_err(err_mapper(DaemonError::AddSigterm))?;

    logger.log(c"Watching for awdl0 interface changes");

    let mut stop_loop = false;
    while !stop_loop {
        event_queue.poll(|event: &libc::kevent| {
            match event.filter {
                libc::EVFILT_SIGNAL => {
                    if let libc::SIGINT | libc::SIGTERM = event.ident as i32 {
                        stop_loop = true;
                        Ok(PollAction::Stop)
                    } else {
                        // Unknown signal we didn't register for
                        Err(Error::new(
                            PosixError::from_raw(0),
                            DaemonError::UnknownSignal,
                        ))
                    }
                }
                libc::EVFILT_READ => {
                    let mut latest_flags = 0;
                    loop {
                        // The read buffer. This just gets overwritten so leave it uninit.
                        let mut header = std::mem::MaybeUninit::<libc::if_msghdr>::uninit();

                        // Intentionally read only `if_msghdr`; routing message payload is discarded.
                        // One read == one message for AF_ROUTE + SOCK_RAW. The remaining data is discarded and
                        // does not get buffered for subsequent reads.
                        let len = unsafe {
                            libc::read(
                                read_fd.as_raw_fd(),
                                header.as_mut_ptr() as *mut libc::c_void,
                                std::mem::size_of::<libc::if_msghdr>(),
                            )
                        };
                        if len == -1 {
                            let posix_error = PosixError::last();
                            match posix_error.code() {
                                libc::EINTR => continue, // Try again
                                libc::EAGAIN => break, // Back to polling (note: EAGAIN == EWOULDBLOCK)
                                _ => {
                                    // Actual read error
                                    return Err(Error::new(posix_error, DaemonError::Read));
                                }
                            }
                        }

                        if (len as usize) < std::mem::size_of::<libc::if_msghdr>() {
                            // Avoids garbage data when `header` is accessed
                            continue;
                        } else {
                            // Else the header has valid data
                            let valid_header = unsafe { header.assume_init() };

                            // Save flags if this is awdl0
                            if awdl_controller.is_awdl(&valid_header) {
                                latest_flags = valid_header.ifm_flags;
                            }
                        }
                    }

                    if (latest_flags & libc::IFF_UP) != 0 {
                        awdl_controller
                            .disable(latest_flags as libc::c_short)
                            .map_err(err_mapper(DaemonError::Disable))?;
                    }

                    Ok(PollAction::Continue)
                }
                _ => Err(Error::new(
                    PosixError::from_raw(0),
                    DaemonError::UnknownEvent,
                )),
            }
        })?;
    }

    // Re-enable awdl0 on exit
    {
        let (is_up, current_flags) = awdl_controller
            .current_status()
            .map_err(err_mapper(DaemonError::ControllerStatusEnd))?;
        if !is_up {
            awdl_controller
                .enable(current_flags)
                .map_err(err_mapper(DaemonError::ReenableOnExit))?;
            logger.log(c"Re-enabling awdl0");
        } else {
            logger.log(c"awdl0 already re-enabled");
        }
    }

    Ok(())
}

fn main() {
    let mut logger = Logger::new(c"awdldisabler.app", c"daemon");
    logger.log(c"AWDLDaemon started");

    let exit_code = 'main: {
        if unsafe { libc::getuid() } != 0 {
            logger.error(c"AWDLDaemon requires root");
            break 'main 1;
        }

        if let Err(error) = disable_awdl(&mut logger) {
            error::log_error(&mut logger, &error);
            break 'main error.posix_error().code();
        }

        0 // No error
    };
    logger.log(c"AWDLDaemon exiting");
    std::process::exit(exit_code);
}
