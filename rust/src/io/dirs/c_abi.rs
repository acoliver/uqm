//! C-callable entry points, one per function `dirs.c` defines for
//! `sc2/src/libs/file.h`, with the header's exact signatures.
//!
//! These carry no export attribute while the C `dirs.c.o` provider is still
//! the selected one: exporting the same symbols from both would be a
//! duplicate-provider link. Activation gives each function its C symbol with
//! `#[export_name = "..."]` in the same change that retires the C object; the
//! symbol each one takes over is named in its documentation.
//!
//! Failure follows the C contract: `-1` with `errno` set. A null pointer
//! argument is a caller bug that C answers with a fault or undefined
//! behaviour; here it is logged and the process aborts, and nothing unwinds
//! into C. Bytes are never validated as UTF-8.

use std::ffi::{c_char, c_int, CStr};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use super::expand::expand_path as expand;
use super::mkdirhier::{create_directory as create, mkdirhier as make_path};
use super::process_env::{home_dir_pointer, ProcessEnvironment};
use crate::logging::{log_add, LogLevel};

/// Takes over `createDirectory`: `mkdir(dir, mode)`.
///
/// # Safety
/// `dir` is a NUL-terminated string.
pub unsafe extern "C" fn create_directory(dir: *const c_char, mode: c_int) -> c_int {
    let dir = unsafe { required_string("createDirectory", "dir", dir) };
    // `mode` narrows to `mode_t` as C's implicit conversion does.
    let result = create(
        Path::new(std::ffi::OsStr::from_bytes(dir.to_bytes())),
        mode as u32,
    );
    status(result)
}

/// Takes over `mkdirhier`.
///
/// # Safety
/// `path` is a NUL-terminated string.
pub unsafe extern "C" fn mkdirhier(path: *const c_char) -> c_int {
    let path = unsafe { required_string("mkdirhier", "path", path) };
    status(make_path(path.to_bytes()))
}

/// Takes over `getHomeDir`. The result is not owned by the caller and must not
/// be freed: it is the `HOME` entry of the environment or the `pw_dir` of the
/// passwd record, valid until the environment changes or `getpwuid` is called
/// again. Null when neither exists.
///
/// # Safety
/// No other thread modifies the environment or calls `getpw*` concurrently
/// with this call or with the use of the result.
pub unsafe extern "C" fn get_home_dir() -> *const c_char {
    unsafe { home_dir_pointer() }
}

/// Takes over `expandPath`. Writes the expanded, NUL-terminated path into
/// `dest[0..len]` and returns 0, or returns -1 with `errno` set and leaves
/// `dest` untouched. Never writes at or beyond `dest + len`.
///
/// # Safety
/// `dest` points to at least `len` writable bytes and `src` is a
/// NUL-terminated string.
pub unsafe extern "C" fn expand_path(
    dest: *mut c_char,
    len: usize,
    src: *const c_char,
    what: c_int,
) -> c_int {
    const FUNCTION: &str = "expandPath";
    let src = unsafe { required_string(FUNCTION, "src", src) };
    if dest.is_null() {
        contract_violation(FUNCTION, "dest is null");
    }
    match expand(&ProcessEnvironment, src.to_bytes(), len, what) {
        Ok(expanded) => {
            assert!(
                expanded.len() < len,
                "expand_path returned {} bytes for a {len}-byte buffer",
                expanded.len()
            );
            // SAFETY: the caller provides `len` bytes and the assertion above
            // leaves room for the NUL; source and destination are distinct.
            unsafe {
                ptr::copy_nonoverlapping(expanded.as_ptr(), dest.cast::<u8>(), expanded.len());
                dest.add(expanded.len()).write(0);
            }
            0
        }
        Err(error) => status::<()>(Err(error)),
    }
}

/// `0` for success; `-1` with `errno` set for an OS error.
fn status<T>(result: io::Result<T>) -> c_int {
    match result {
        Ok(_) => 0,
        Err(error) => {
            set_errno(
                error
                    .raw_os_error()
                    .expect("directory errors carry an errno"),
            );
            -1
        }
    }
}

/// # Safety
/// `value` is null or a NUL-terminated string that outlives `'a`.
unsafe fn required_string<'a>(function: &str, argument: &str, value: *const c_char) -> &'a CStr {
    if value.is_null() {
        contract_violation(function, &format!("{argument} is null"));
    }
    // SAFETY: non-null and NUL-terminated per the contract above.
    unsafe { CStr::from_ptr(value) }
}

fn contract_violation(function: &str, reason: &str) -> ! {
    // SAFETY: `log_add` has no preconditions beyond a valid message.
    unsafe {
        log_add(
            LogLevel::Fatal,
            &format!("{function}() FATAL: caller bug: {reason}."),
        );
    }
    std::process::abort()
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
fn set_errno(code: c_int) {
    // SAFETY: `__error` returns this thread's errno location.
    unsafe { *libc::__error() = code };
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set_errno(code: c_int) {
    // SAFETY: `__errno_location` returns this thread's errno location.
    unsafe { *libc::__errno_location() = code };
}
