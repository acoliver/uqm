//! Descriptor duplication and file opening.

use std::ffi::{c_int, CStr};
use std::io;
use std::os::fd::{AsRawFd as _, BorrowedFd, FromRawFd as _, OwnedFd};

pub use uqm_process_support::os::fd::{
    is_close_on_exec, read_exact_until, set_close_on_exec, set_inheritable, write_all_until,
};

/// A new close-on-exec descriptor for the same open file description as `fd`.
///
/// # Errors
///
/// Returns the OS error from `fcntl`, for example `EMFILE`.
pub fn duplicate_close_on_exec(fd: BorrowedFd<'_>) -> io::Result<OwnedFd> {
    // SAFETY: F_DUPFD_CLOEXEC takes an integer and never dereferences memory.
    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fcntl returned a new descriptor that nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

/// Open `path`; the flags decide access, `O_DIRECTORY` and symlink handling.
///
/// # Errors
///
/// Returns the OS error from `open`.
pub fn open(path: &CStr, flags: c_int) -> io::Result<OwnedFd> {
    // SAFETY: path is NUL-terminated and outlives the call.
    let fd = unsafe { libc::open(path.as_ptr(), flags) };
    owned_descriptor(fd)
}

/// Open `name` relative to the directory `directory`.
///
/// `mode` applies only when `flags` contains `O_CREAT`.
///
/// # Errors
///
/// Returns the OS error from `openat`, for example `ELOOP` when `O_NOFOLLOW`
/// meets a symlink.
pub fn open_at(
    directory: BorrowedFd<'_>,
    name: &CStr,
    flags: c_int,
    mode: libc::mode_t,
) -> io::Result<OwnedFd> {
    // SAFETY: directory is open for the call and name is NUL-terminated. The
    // variadic mode is promoted to c_uint as the C calling convention requires.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags,
            libc::c_uint::from(mode),
        )
    };
    owned_descriptor(fd)
}

fn owned_descriptor(fd: c_int) -> io::Result<OwnedFd> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: open and openat returned a new descriptor that nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::{Read as _, Write as _};
    use std::os::fd::AsFd as _;

    #[test]
    fn a_duplicate_is_close_on_exec_and_shares_the_file() {
        let (mut reader, mut writer) = std::io::pipe().unwrap();
        let duplicate = duplicate_close_on_exec(reader.as_fd()).unwrap();
        assert!(is_close_on_exec(duplicate.as_fd()).unwrap());
        writer.write_all(b"z").unwrap();
        let mut file = std::fs::File::from(duplicate);
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte).unwrap();
        assert_eq!(&byte, b"z");
        let _ = &mut reader;
    }

    #[test]
    fn open_reports_missing_files_and_refuses_symlinks_when_asked() {
        let root = tempfile::tempdir().unwrap();
        let missing = CString::new(root.path().join("absent").to_str().unwrap()).unwrap();
        let error = open(&missing, libc::O_RDONLY).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));

        let target = root.path().join("target");
        std::fs::write(&target, b"x").unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let link = CString::new(link.to_str().unwrap()).unwrap();
        assert!(open(&link, libc::O_RDONLY).is_ok());
        let error = open(&link, libc::O_RDONLY | libc::O_NOFOLLOW).unwrap_err();
        assert!(matches!(
            error.raw_os_error(),
            Some(libc::ELOOP) | Some(libc::EMLINK)
        ));
    }

    #[test]
    fn open_at_creates_with_the_requested_mode_and_rejects_duplicates() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = tempfile::tempdir().unwrap();
        let directory = std::fs::File::open(root.path()).unwrap();
        let name = CString::new("created").unwrap();
        let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC;
        let created = open_at(directory.as_fd(), &name, flags, 0o600).unwrap();
        let mode = std::fs::File::from(created)
            .metadata()
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let error = open_at(directory.as_fd(), &name, flags, 0o600).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EEXIST));
    }

    #[test]
    fn open_at_below_a_regular_file_is_not_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let file_path = root.path().join("plain");
        std::fs::write(&file_path, b"x").unwrap();
        let file = std::fs::File::open(&file_path).unwrap();
        let name = CString::new("child").unwrap();
        let error = open_at(file.as_fd(), &name, libc::O_RDONLY, 0).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTDIR));
    }
}
