//! Platform-owned paths and the C runtime's errno vocabulary.

use std::ffi::c_int;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub mod macos;
pub mod os;
pub mod sys;

/// Translate OS failures to the C runtime errno vocabulary used by the UIO API.
pub fn io_errno(error: &std::io::Error) -> c_int {
    #[cfg(unix)]
    if let Some(errno) = error.raw_os_error() {
        return errno;
    }
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::NotFound => libc::ENOENT,
        ErrorKind::PermissionDenied => libc::EACCES,
        ErrorKind::AlreadyExists => libc::EEXIST,
        ErrorKind::InvalidInput | ErrorKind::InvalidData => libc::EINVAL,
        ErrorKind::NotADirectory => libc::ENOTDIR,
        ErrorKind::IsADirectory => libc::EISDIR,
        ErrorKind::DirectoryNotEmpty => libc::ENOTEMPTY,
        ErrorKind::Interrupted => libc::EINTR,
        ErrorKind::WouldBlock => libc::EAGAIN,
        ErrorKind::OutOfMemory => libc::ENOMEM,
        ErrorKind::StorageFull => libc::ENOSPC,
        _ => libc::EIO,
    }
}

#[cfg(unix)]
pub type FileMode = libc::mode_t;
#[cfg(windows)]
pub type FileMode = u16;

/// Keep upstream's per-user location; packaged content never changes this path.
pub fn default_config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|home| PathBuf::from(home).join("AppData/Roaming"))
            })
            .expect("Windows must provide APPDATA or USERPROFILE")
            .join("uqm")
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let home = crate::io::legacy_dirs::get_home_dir();
        assert!(!home.is_null(), "OS home directory unavailable");
        // SAFETY: get_home_dir returns a libc-owned NUL-terminated environment/passwd path.
        let home = unsafe { std::ffi::CStr::from_ptr(home) };
        PathBuf::from(std::ffi::OsStr::from_bytes(home.to_bytes())).join(".uqm/")
    }
}

#[cfg(test)]
mod tests {
    use super::io_errno;
    use std::io::{Error, ErrorKind};

    #[test]
    fn virtual_file_errors_keep_their_errno_meaning() {
        for (kind, expected) in [
            (ErrorKind::NotFound, libc::ENOENT),
            (ErrorKind::PermissionDenied, libc::EACCES),
            (ErrorKind::InvalidInput, libc::EINVAL),
            (ErrorKind::Other, libc::EIO),
        ] {
            assert_eq!(io_errno(&Error::from(kind)), expected, "{kind:?}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn native_file_errors_keep_the_original_errno() {
        assert_eq!(
            io_errno(&Error::from_raw_os_error(libc::EROFS)),
            libc::EROFS
        );
    }
}
