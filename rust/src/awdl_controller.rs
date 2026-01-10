use std::{
    ffi::CStr,
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
};

use crate::error::{Error, Result};

const AWDL_IFREQ_NAME: &CStr = c"awdl0";
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
    pub fn new() -> Result<Self> {
        let fd = {
            let raw_fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            if raw_fd == -1 {
                return Err(Error::last());
            }
            unsafe { OwnedFd::from_raw_fd(raw_fd) }
        };

        let ifreq = new_ifreq(AWDL_IFREQ_NAME)?;

        // Just an optimization. Uses `ifreq.ifr_name` instead of `AWDL_IFREQ_NAME` to prevent the
        // latter from being present at runtime. b"awdl0" should only be present in one part of the
        // resulting binary from doing this.
        let index = unsafe { libc::if_nametoindex(ifreq.ifr_name.as_ptr()) };
        if index == 0 {
            return Err(Error::last());
        }

        Ok(AWDLController { fd, ifreq, index })
    }

    fn get_flags(&self) -> libc::c_short {
        unsafe { self.ifreq.ifr_ifru.ifru_flags }
    }

    fn set_flags(&mut self, new_flags: libc::c_short) {
        self.ifreq.ifr_ifru.ifru_flags = new_flags
    }

    fn ioctl(&mut self, request: libc::c_ulong) -> Result<()> {
        unsafe {
            let result = libc::ioctl(
                self.fd.as_fd().as_raw_fd(),
                request,
                &raw mut self.ifreq as *mut libc::c_void,
            );
            if result < 0 {
                return Err(Error::last());
            }
        }
        Ok(())
    }

    pub fn handle_if_up<F>(&mut self, mut action: F) -> Result<()>
    where
        F: FnMut(bool, &mut AWDLController, libc::c_int) -> Result<()>,
    {
        self.ioctl(SIOCGIFFLAGS)?;
        let current_flags = self.get_flags() as libc::c_int;

        let is_up = (current_flags & libc::IFF_UP) != 0;
        action(is_up, self, current_flags)
    }

    pub fn disable(&mut self, flags: libc::c_int) -> Result<()> {
        self.set_flags((flags & !libc::IFF_UP) as _);
        self.ioctl(SIOCSIFFLAGS)
    }

    pub fn enable(&mut self, flags: libc::c_int) -> Result<()> {
        self.set_flags((flags | libc::IFF_UP) as _);
        self.ioctl(SIOCSIFFLAGS)
    }

    pub fn is_awdl(&self, header: &libc::if_msghdr) -> bool {
        // RTM_IFINFO - reports a change in interface status
        header.ifm_type == RTM_IFINFO && header.ifm_index as libc::c_uint == self.index
    }
}

// Compile-time copy name to `ifreq`
const fn copy_ifreq_name(ifreq: &mut libc::ifreq, name: &CStr) -> Result<()> {
    let bytes_with_nul = name.to_bytes_with_nul();

    if bytes_with_nul.len() > ifreq.ifr_name.len() {
        Err(Error::from_raw(libc::ERANGE))
    } else {
        // Unchecked const version of `ifreq.ifr_name[..bytes_with_nul.len()]`
        let target: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(
                ifreq.ifr_name.as_mut_ptr() as *mut u8,
                bytes_with_nul.len(),
            )
        };

        // Copy `name` to `ifr_name` including its nul terminator
        target.copy_from_slice(bytes_with_nul);
        Ok(())
    }
}

// Compile-time create a `libc::ifreq`
const fn new_ifreq(name: &CStr) -> Result<libc::ifreq> {
    let mut ifreq: libc::ifreq = unsafe { std::mem::zeroed() };
    match copy_ifreq_name(&mut ifreq, name) {
        Ok(()) => Ok(ifreq),
        Err(e) => Err(e),
    }
}

#[test]
fn test_name_copy() {
    let mut ifreq: libc::ifreq = unsafe { std::mem::zeroed() };
    assert!(copy_ifreq_name(&mut ifreq, c"awdl0").is_ok());

    ifreq = unsafe { std::mem::zeroed() };
    assert!(copy_ifreq_name(&mut ifreq, c"abcdefghijlmnop").is_ok());

    ifreq = unsafe { std::mem::zeroed() };
    assert!(copy_ifreq_name(&mut ifreq, c"abcdefghijlmnopq").is_err());
}
