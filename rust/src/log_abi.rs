//! Game logging with Rust-owned entry points and printf-compatible formatting.

use std::ffi::{c_char, c_int, CStr};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use libc::FILE;

pub const LOG_WARNING: i32 = 3;
const LOG_DEBUG: i32 = 5;
pub(crate) const MAX_LOG_ENTRY_SIZE: usize = 256;

extern "C" {
    fn atexit(callback: extern "C" fn()) -> c_int;
    #[cfg(target_os = "macos")]
    #[link_name = "__stderrp"]
    static C_STDERR: *mut FILE;
    #[cfg(not(target_os = "macos"))]
    #[link_name = "stderr"]
    static C_STDERR: *mut FILE;
}

static STREAM_OUT: AtomicPtr<FILE> = AtomicPtr::new(std::ptr::null_mut());
static SHOW_BOX: AtomicBool = AtomicBool::new(true);
static ERROR_BOX: AtomicBool = AtomicBool::new(true);

/// Level at or below which messages are echoed to the output stream.
const MAX_STREAM_LEVEL: i32 = LOG_DEBUG;

#[no_mangle]
pub extern "C" fn log_init(_max_lines: c_int) {
    // SAFETY: C_STDERR is initialised by the C runtime before main.
    STREAM_OUT.store(unsafe { C_STDERR }, Ordering::Relaxed);
    // SAFETY: exit_callback is a plain extern "C" fn.
    let status = unsafe { atexit(exit_callback) };
    assert_eq!(status, 0, "atexit registration failed");
}

#[no_mangle]
pub extern "C" fn log_initThreads() {}

#[no_mangle]
pub extern "C" fn log_exit(code: c_int) -> c_int {
    SHOW_BOX.store(false, Ordering::Relaxed);
    code
}

#[no_mangle]
pub extern "C" fn log_setLevel(_level: c_int) {}

/// Sets the new output stream and returns the previous one.
#[no_mangle]
pub extern "C" fn log_setOutput(out: *mut FILE) -> *mut FILE {
    STREAM_OUT.swap(out, Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn log_showBox(show: bool, err: bool) {
    SHOW_BOX.store(show, Ordering::Relaxed);
    ERROR_BOX.store(err, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn log_captureLines(_num: c_int) {}

// Keep printf's byte formatting and 254-byte payload limit without a C
// variadic forwarding function or a va_list crossing the Rust boundary.
macro_rules! game_log {
    ($level:expr, $format:expr $(, $argument:expr)* $(,)?) => {{
        let level = $level as i32;
        let mut message = [0 as ::std::ffi::c_char; $crate::log_abi::MAX_LOG_ENTRY_SIZE];
        ::libc::snprintf(
            message.as_mut_ptr(),
            $crate::log_abi::MAX_LOG_ENTRY_SIZE - 1,
            $format,
            $($argument),*
        );
        $crate::log_abi::log_cstr(level, ::std::ffi::CStr::from_ptr(message.as_ptr()));
    }};
}
pub(crate) use game_log;

pub(crate) fn log_cstr(level: i32, message: &CStr) {
    echo(level, message.as_ptr());
}

fn echo(level: c_int, message: *const c_char) {
    if level <= MAX_STREAM_LEVEL {
        // SAFETY: STREAM_OUT is a live FILE set by log_init/log_setOutput and
        // `message` is NUL-terminated.
        unsafe {
            libc::fprintf(
                STREAM_OUT.load(Ordering::Relaxed),
                c"%s\n".as_ptr(),
                message,
            );
        }
    }
}

/// Logs a message already formatted by Rust code, with the same filtering as
/// the printf entry points. Messages are dropped until `log_init` has set the stream.
pub fn log_message(level: c_int, message: &str) {
    if STREAM_OUT.load(Ordering::Relaxed).is_null() {
        return;
    }
    let mut text = [0 as c_char; MAX_LOG_ENTRY_SIZE];
    for (slot, byte) in text[..MAX_LOG_ENTRY_SIZE - 1]
        .iter_mut()
        .zip(message.bytes())
    {
        *slot = byte as c_char;
    }
    echo(level, text.as_ptr());
}

extern "C" fn exit_callback() {
    if SHOW_BOX.load(Ordering::Relaxed) {
        display_log(ERROR_BOX.load(Ordering::Relaxed));
    }
    log_exit(0);
}

/// There is no message box on this platform; tell the user so.
fn display_log(is_error: bool) {
    let kind = if is_error { c"an error" } else { c"a" };
    // SAFETY: STREAM_OUT is a live FILE set by log_init/log_setOutput.
    unsafe {
        libc::fprintf(
            STREAM_OUT.load(Ordering::Relaxed),
            c"Do not know how to display %s box\n".as_ptr(),
            kind.as_ptr(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_log_calls_preserve_printf_bytes_limit_and_level_filter() {
        unsafe {
            let stream = libc::tmpfile();
            assert!(!stream.is_null());
            let previous = log_setOutput(stream);
            crate::log_abi::game_log!(3, c"value=%d text=%s %%".as_ptr(), -17, c"raw\xff".as_ptr());
            crate::log_abi::game_log!(6, c"filtered".as_ptr());
            let long = std::ffi::CString::new(vec![b'x'; 300]).unwrap();
            crate::log_abi::game_log!(5, c"%s".as_ptr(), long.as_ptr());
            log_setOutput(previous);
            libc::rewind(stream);
            let mut bytes = [0; 512];
            let count = libc::fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), stream);
            assert_eq!(libc::fclose(stream), 0);
            let mut expected = b"value=-17 text=raw\xff %\n".to_vec();
            expected.extend([b'x'; 254]);
            expected.push(b'\n');
            assert_eq!(&bytes[..count], expected);
        }
    }
}
