use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use crate::error::{Error, Result};

pub struct EventQueue<'a> {
    kq_fd: OwnedFd,
    events: Vec<libc::kevent>,
    phantom: std::marker::PhantomData<&'a OwnedFd>,
}

impl<'a> EventQueue<'a> {
    pub fn new_with_read_fd(read_fd: &'a OwnedFd) -> Result<Self> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            Err(Error::last())
        } else {
            let kq_fd = unsafe { OwnedFd::from_raw_fd(fd) };
            let mut event_queue = EventQueue {
                kq_fd,
                events: Vec::new(), // Empty for now
                phantom: std::marker::PhantomData,
            };

            let event = libc::kevent {
                ident: read_fd.as_raw_fd() as libc::uintptr_t,
                filter: libc::EVFILT_READ,
                flags: libc::EV_ADD,
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            };
            event_queue.add_event(&event)?;

            Ok(event_queue)
        }
    }

    fn add_event(&mut self, event: &libc::kevent) -> Result<()> {
        let changes = std::slice::from_ref(event);
        let result = unsafe {
            libc::kevent(
                self.kq_fd.as_raw_fd(),
                changes.as_ptr(),
                changes.len() as _,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if result == -1 {
            // Failed to register
            Err(Error::last())
        } else {
            self.events.push(unsafe { std::mem::zeroed() });
            Ok(())
        }
    }

    pub fn add_signal(&mut self, signal: libc::c_int) -> Result<()> {
        // Need to disable default action for monitored signals
        unsafe {
            if libc::signal(signal, libc::SIG_IGN) == libc::SIG_ERR {
                return Err(Error::last());
            }
        }
        let event = libc::kevent {
            ident: signal as libc::uintptr_t,
            filter: libc::EVFILT_SIGNAL,
            flags: libc::EV_ADD,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        self.add_event(&event)
    }

    /// Polls the kqueue and invokes `action` for each received event.
    ///
    /// This function blocks until at least one event is available, then
    /// calls `action` once for each `kevent` returned by the kernel.
    ///
    /// # Parameters
    ///
    /// * `action` — A callback invoked for each event. The callback receives
    ///   a reference to the raw `libc::kevent` structure as reported by kqueue.
    ///
    ///   The callback should return:
    ///   * `PollAction::Continue` to continue processing subsequent events
    ///   * `PollAction::Stop` to stop processing events and return from `poll`
    ///   * `Err(e)` to abort processing and return the error immediately
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * the underlying `kevent(2)` system call fails
    /// * `action` returns an error
    pub fn poll<F>(&mut self, mut action: F) -> Result<()>
    where
        F: FnMut(&libc::kevent) -> Result<PollAction>,
    {
        let num_events = unsafe {
            libc::kevent(
                self.kq_fd.as_raw_fd(),
                std::ptr::null(),
                0,
                self.events.as_mut_ptr(),
                self.events.len() as libc::c_int,
                std::ptr::null(),
            )
        };
        if num_events == -1 {
            return Err(Error::last());
        }

        for event in &self.events[..(num_events as usize)] {
            if action(event)? == PollAction::Stop {
                break;
            }
        }
        Ok(())
    }
}

/// Returned from the callback passed to [`poll`] to indicate whether
/// polling should continue or stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollAction {
    /// Continue processing subsequent events.
    Continue,
    /// Stop processing events and return from `poll`.
    Stop,
}
