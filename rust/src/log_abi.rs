//! Rust-owned game logging.

use std::ffi::{c_char, c_int};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use libc::FILE;

pub const LOG_WARNING: i32 = 3;
const LOG_DEBUG: i32 = 5;
const MAX_LOG_ENTRY_SIZE: usize = 256;

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
pub extern "C" fn log_exit(code: c_int) -> c_int {
    SHOW_BOX.store(false, Ordering::Relaxed);
    code
}

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

/// Logs a formatted message at the game log threshold. Messages are dropped
/// until `log_init` has set the stream.
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
