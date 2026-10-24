//! Platform-owned paths and C runtime error state used by the remaining ABI.

use std::ffi::c_int;
use std::path::PathBuf;

pub fn redirect_stderr(path: &std::path::Path) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        // SAFETY: file and stderr are valid CRT descriptors.
        if unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE};
        // SAFETY: transfer the file handle to the CRT, duplicate it onto stderr,
        // then point Rust/Win32 stderr at that same live CRT-owned handle.
        unsafe {
            let fd = libc::open_osfhandle(
                file.as_raw_handle() as isize,
                libc::O_WRONLY | libc::O_BINARY,
            );
            if fd == -1 {
                return Err(std::io::Error::from_raw_os_error(errno()));
            }
            std::mem::forget(file);
            let result = libc::dup2(fd, 2);
            libc::close(fd);
            if result == -1 {
                return Err(std::io::Error::from_raw_os_error(errno()));
            }
            if SetStdHandle(STD_ERROR_HANDLE, libc::get_osfhandle(2) as _) == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

/// Translate OS failures to the C runtime errno vocabulary used by the UIO API.
pub fn io_errno(error: &std::io::Error) -> c_int {
    #[cfg(unix)]
    {
        error.raw_os_error().unwrap_or(libc::EIO)
    }
    #[cfg(windows)]
    {
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
}

#[cfg(windows)]
unsafe extern "C" {
    fn _errno() -> *mut c_int;
}

pub fn errno() -> c_int {
    // SAFETY: each runtime returns the calling thread's errno cell.
    unsafe { *errno_cell() }
}

pub fn set_errno(error: c_int) {
    // SAFETY: each runtime returns the calling thread's errno cell.
    unsafe { *errno_cell() = error };
}

unsafe fn errno_cell() -> *mut c_int {
    #[cfg(windows)]
    {
        _errno()
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        libc::__errno_location()
    }
    #[cfg(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        libc::__error()
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
        PathBuf::from(std::env::var_os("HOME").expect("HOME must be set")).join(".uqm/")
    }
}
