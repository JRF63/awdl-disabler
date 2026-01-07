pub type Result<T> = std::result::Result<T, Error>;

#[repr(transparent)]
pub struct Error {
    inner: libc::c_int,
}

impl Error {
    pub const fn from_raw(code: libc::c_int) -> Self {
        Error { inner: code }
    }

    pub fn raw_error(&self) -> libc::c_int {
        self.inner
    }

    pub fn last() -> Self {
        let inner = unsafe { *libc::__error() };
        Error { inner }
    }
}
