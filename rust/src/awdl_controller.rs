use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};

use crate::error::{DaemonError, Error, PosixError};

const RTM_IFINFO: libc::c_uchar = libc::RTM_IFINFO as libc::c_uchar;

// These two are not defined in libc
const SIOCGIFFLAGS: libc::c_ulong = 0xc020_6911;
const SIOCSIFFLAGS: libc::c_ulong = 0x8020_6910;

pub struct AWDLController {
    fd: OwnedFd,
    ifreq: libc::ifreq,
    index: libc::c_uint,
}

impl AWDLController {
    pub fn new() -> Result<Self, Error> {
        let fd = {
            let raw_fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            if raw_fd == -1 {
                return Err(Error::last(DaemonError::IoctlFDCreation));
            }
            unsafe { OwnedFd::from_raw_fd(raw_fd) }
        };

        let ifreq = libc::ifreq {
            ifr_name: [
                'a' as i8, 'w' as i8, 'd' as i8, 'l' as i8, '0' as i8, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0,
            ],
            ..{ unsafe { std::mem::zeroed() } }
        };

        // `ifreq.ifr_name` is set to c"awdl0"
        let index = unsafe { libc::if_nametoindex(ifreq.ifr_name.as_ptr()) };
        if index == 0 {
            return Err(Error::last(DaemonError::IndexLookup));
        }

        Ok(AWDLController { fd, ifreq, index })
    }

    fn get_flags(&self) -> libc::c_short {
        unsafe { self.ifreq.ifr_ifru.ifru_flags }
    }

    fn set_flags(&mut self, new_flags: libc::c_short) {
        self.ifreq.ifr_ifru.ifru_flags = new_flags
    }

    fn ioctl(&mut self, request: libc::c_ulong) -> Result<(), PosixError> {
        unsafe {
            let result = libc::ioctl(
                self.fd.as_fd().as_raw_fd(),
                request,
                &raw mut self.ifreq as *mut libc::c_void,
            );
            if result < 0 {
                return Err(PosixError::last());
            }
        }
        Ok(())
    }

    pub fn current_status(&mut self) -> Result<(bool, libc::c_short), PosixError> {
        self.ioctl(SIOCGIFFLAGS)?;
        let current_flags = self.get_flags();
        let is_up = (current_flags & libc::IFF_UP as i16) != 0;
        Ok((is_up, current_flags))
    }

    pub fn disable(&mut self, flags: libc::c_short) -> Result<(), PosixError> {
        self.set_flags(flags & !libc::IFF_UP as i16);
        self.ioctl(SIOCSIFFLAGS)
    }

    pub fn enable(&mut self, flags: libc::c_short) -> Result<(), PosixError> {
        self.set_flags(flags | libc::IFF_UP as i16);
        self.ioctl(SIOCSIFFLAGS)
    }

    pub fn is_awdl(&self, header: &libc::if_msghdr) -> bool {
        // RTM_IFINFO - reports a change in interface status
        header.ifm_type == RTM_IFINFO && header.ifm_index as libc::c_uint == self.index
    }
}
