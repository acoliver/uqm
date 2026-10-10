//! The real process environment behind [`Environment`].

use std::ffi::{c_char, CStr, OsStr};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::ptr;

use super::expand::Environment;

/// The environment of the running process: `getenv`, the passwd database and
/// the working directory.
pub struct ProcessEnvironment;

impl Environment for ProcessEnvironment {
    fn var(&self, name: &[u8]) -> Option<Vec<u8>> {
        std::env::var_os(OsStr::from_bytes(name)).map(OsStringExt::into_vec)
    }

    fn home_dir(&self) -> Option<Vec<u8>> {
        // SAFETY: the pointer is read and copied before anything can change
        // the environment or call `getpwuid` again on this thread.
        let home = unsafe { home_dir_pointer() };
        if home.is_null() {
            return None;
        }
        // SAFETY: `home_dir_pointer` returns null or a NUL-terminated string.
        Some(unsafe { CStr::from_ptr(home) }.to_bytes().to_vec())
    }

    fn current_dir(&self) -> io::Result<Vec<u8>> {
        std::env::current_dir().map(|dir| dir.into_os_string().into_vec())
    }
}

/// The C `getHomeDir`: the `HOME` variable if set (even when empty), else the
/// home directory of the real user from the passwd database, else null.
///
/// The returned pointer is borrowed, never freed by the caller: it points into
/// the process environment block (`HOME`) or into the buffer that `getpwuid`
/// owns. It stays valid until the environment is modified or `getpwuid` is
/// called again, exactly as for the C function. It is deliberately not wrapped
/// in a `'static` reference, because it is not one.
///
/// # Safety
/// No other thread may modify the environment or call `getpw*` while this runs
/// or while the result is in use; the same rule the C function has.
pub unsafe fn home_dir_pointer() -> *const c_char {
    // SAFETY: the name is a valid NUL-terminated string; the caller upholds the
    // environment-access rule above.
    let home = unsafe { libc::getenv(c"HOME".as_ptr()) };
    if !home.is_null() {
        return home;
    }
    // SAFETY: `getuid` has no preconditions; `getpwuid` result is checked.
    let entry = unsafe { libc::getpwuid(libc::getuid()) };
    if entry.is_null() {
        return ptr::null();
    }
    // SAFETY: non-null `getpwuid` results point to a valid `passwd`.
    unsafe { (*entry).pw_dir }
}
