mod awdl_controller;
mod error;
mod event;
mod os_log;

use std::{
    ffi::CStr,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use awdl_controller::AWDLController;
use error::{Error, Result};
use event::{EventQueue, PollAction};
use os_log::Logger;

fn disable_awdl(logger: &mut Logger) -> Result<()> {
    let mut awdl_controller = AWDLController::new()?;

    // Disable awdl0 at program start
    awdl_controller.handle_if_up(|is_up, tmp, current_flags| {
        if is_up {
            tmp.disable(current_flags)?;
            logger.log(c"Disabling awdl0");
        } else {
            logger.log(c"awdl0 already disabled");
        }
        Ok(())
    })?;

    // Must be AF_ROUTE + SOCK_RAW and non-blocking
    let read_fd = {
        let raw_fd = unsafe { libc::socket(libc::AF_ROUTE, libc::SOCK_RAW, 0) };
        if raw_fd == -1 {
            return Err(Error::last());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        unsafe {
            if libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) == -1 {
                return Err(Error::last());
            }
        }
        fd
    };

    let mut event_queue = EventQueue::new_with_read_fd(&read_fd)?;
    event_queue.add_signal(libc::SIGINT)?;
    event_queue.add_signal(libc::SIGTERM)?;

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
                        Err(Error::last())
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
                            let errno = Error::last().raw_error();
                            match errno {
                                libc::EINTR => continue, // Try again
                                libc::EAGAIN => break,   // Back to polling
                                e => return Err(Error::from_raw(e)),
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
                        awdl_controller.disable(latest_flags)?;
                    }

                    Ok(PollAction::Continue)
                }
                _ => Ok(PollAction::Continue), // Ignore unknown event
            }
        })?;
    }

    // Re-enable awdl0 on exit
    awdl_controller.handle_if_up(|is_up, tmp, current_flags| {
        if !is_up {
            tmp.enable(current_flags)?;
            logger.log(c"Re-enabling awdl0");
        } else {
            logger.log(c"awdl0 already re-enabled");
        }
        Ok(())
    })?;

    Ok(())
}

fn main() {
    let mut logger = Logger::new(c"awdldisabler.app", c"daemon");

    let exit_code = 'main: {
        if unsafe { libc::getuid() } != 0 {
            logger.error(c"AWDLDaemon requires root");
            break 'main 1;
        }

        if let Err(e) = disable_awdl(&mut logger) {
            let ptr = unsafe { libc::strerror(e.raw_error()) };
            let cstr = unsafe { CStr::from_ptr(ptr) };
            logger.error(cstr);
            break 'main e.raw_error();
        }

        0 // No error
    };
    logger.log(c"AWDLDaemon exiting");
    std::process::exit(exit_code);
}
