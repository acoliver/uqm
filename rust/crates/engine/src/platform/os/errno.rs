//! The C runtime's per-thread `errno` cell.

use std::ffi::c_int;

#[cfg(windows)]
unsafe extern "C" {
    fn _errno() -> *mut c_int;
}

/// The calling thread's current `errno`.
pub fn errno() -> c_int {
    // SAFETY: each runtime returns the calling thread's errno cell.
    unsafe { *errno_cell() }
}

/// Set the calling thread's `errno`.
pub fn set_errno(error: c_int) {
    // SAFETY: each runtime returns the calling thread's errno cell.
    unsafe { *errno_cell() = error };
}

fn errno_cell() -> *mut c_int {
    // SAFETY: each runtime's accessor takes no arguments and returns the
    // calling thread's errno cell.
    #[cfg(windows)]
    {
        unsafe { _errno() }
    }
    // SAFETY: as above.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe { libc::__errno_location() }
    }
    // SAFETY: as above.
    #[cfg(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        unsafe { libc::__error() }
    }
}

#[cfg(test)]
mod tests {
    use super::{errno, set_errno};

    #[test]
    fn set_errno_is_visible_to_errno_on_the_same_thread() {
        set_errno(libc::ERANGE);
        assert_eq!(errno(), libc::ERANGE);
        set_errno(0);
        assert_eq!(errno(), 0);
    }

    #[test]
    fn errno_is_per_thread() {
        set_errno(libc::EINVAL);
        std::thread::spawn(|| {
            set_errno(libc::EPERM);
            assert_eq!(errno(), libc::EPERM);
        })
        .join()
        .unwrap();
        assert_eq!(errno(), libc::EINVAL);
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_system_call_reports_through_the_cell() {
        set_errno(0);
        let error = std::fs::File::open("/nonexistent/uqm-errno-probe").unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));
        assert_eq!(errno(), libc::ENOENT);
    }
}
