//! `errno` values and their `strerror` text.

use std::ffi::{c_char, c_int, CStr};

/// An `errno` value a UIO call left behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Errno(pub c_int);

impl Errno {
    /// The calling thread's `errno`. Read it before any other call.
    pub fn last() -> Self {
        Self(
            std::io::Error::last_os_error()
                .raw_os_error()
                .expect("last_os_error carries the OS error code"),
        )
    }

    /// `strerror(errno)` as bytes.
    pub fn message(self) -> Vec<u8> {
        let mut buffer = [0 as c_char; 256];
        // SAFETY: the buffer is writable for its length and `strerror_r`
        // NUL-terminates it (the XSI variant the libc crate binds).
        let status = unsafe { libc::strerror_r(self.0, buffer.as_mut_ptr(), buffer.len()) };
        if status != 0 && status != libc::ERANGE {
            return format!("Unknown error {}", self.0).into_bytes();
        }
        // SAFETY: the buffer is NUL terminated by `strerror_r`, or by its
        // zero initialisation when the message was cut for ERANGE.
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_bytes()
            .to_vec()
    }
}
