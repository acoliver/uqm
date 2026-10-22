//! C ABI for the game log (`libs/log/uqmlog.h`).
//!
//! `log_add` and `log_add_nothread` are C variadics, which stable Rust cannot
//! define; they remain as thin wrappers in `sc2/src/libs/log/uqmlog.c` that
//! forward their `va_list` to [`log_addV`] and [`log_add_nothreadV`] here.
//!
//! The C implementation also kept a ring buffer of recent messages that was
//! only ever read to fill the fatal-error message box. This platform has no
//! message box (`msgbox_stub.c` printed a notice and discarded the text), so
//! the buffer is not kept; `log_setLevel` (which only gated that buffer), `log_captureLines` and `log_initThreads` remain
//! only because C callers still invoke them.

use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use libc::FILE;

const LOG_DEBUG: i32 = 5;
const MAX_LOG_ENTRY_SIZE: usize = 256;

extern "C" {
    // `va_list` is passed through opaquely; it is pointer-sized on every
    // supported target.
    fn vsnprintf(buf: *mut c_char, size: usize, fmt: *const c_char, args: *mut c_void) -> c_int;
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

/// Formats `fmt` with `args` and echoes it to the output stream when `level`
/// is within the stream threshold.
///
/// # Safety
/// `fmt` must be a valid printf format and `args` the matching `va_list`.
#[no_mangle]
pub unsafe extern "C" fn log_addV(level: c_int, fmt: *const c_char, args: *mut c_void) {
    let mut message = [0 as c_char; MAX_LOG_ENTRY_SIZE];
    vsnprintf(message.as_mut_ptr(), MAX_LOG_ENTRY_SIZE - 1, fmt, args);
    if level <= MAX_STREAM_LEVEL {
        libc::fprintf(
            STREAM_OUT.load(Ordering::Relaxed),
            c"%s\n".as_ptr(),
            message.as_ptr(),
        );
    }
}

/// Same as [`log_addV`]; the C version differed only in how it queued the
/// message for the (absent) message box.
///
/// # Safety
/// As for [`log_addV`].
#[no_mangle]
pub unsafe extern "C" fn log_add_nothreadV(level: c_int, fmt: *const c_char, args: *mut c_void) {
    log_addV(level, fmt, args);
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
