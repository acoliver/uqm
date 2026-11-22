//! Rust-owned game logging.

use std::ffi::c_int;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::logging::LogLevel;

const MAX_LOG_ENTRY_SIZE: usize = 256;

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static SHOW_BOX: AtomicBool = AtomicBool::new(true);
static ERROR_BOX: AtomicBool = AtomicBool::new(true);

/// Level at or below which messages are echoed to the output stream.
const MAX_STREAM_LEVEL: LogLevel = LogLevel::Debug;

pub fn log_init(_max_lines: c_int) {
    INITIALIZED.store(true, Ordering::Relaxed);
    crate::host::at_exit(flush_log_at_exit);
}

pub fn log_exit(code: c_int) -> c_int {
    SHOW_BOX.store(false, Ordering::Relaxed);
    code
}

pub fn log_show_box(show: bool, err: bool) {
    SHOW_BOX.store(show, Ordering::Relaxed);
    ERROR_BOX.store(err, Ordering::Relaxed);
}

/// Logs a formatted message at the game log threshold. Messages are dropped
/// until `log_init` has initialized the output.
pub fn log_message(level: LogLevel, message: &str) {
    if !INITIALIZED.load(Ordering::Relaxed) || level > MAX_STREAM_LEVEL {
        return;
    }
    let bytes = message.as_bytes();
    let length = bytes.len().min(MAX_LOG_ENTRY_SIZE - 1);
    let length = bytes[..length]
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(length);
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(&bytes[..length]);
    let _ = stderr.write_all(b"\n");
}

fn flush_log_at_exit() {
    if SHOW_BOX.load(Ordering::Relaxed) {
        display_log(ERROR_BOX.load(Ordering::Relaxed));
    }
    log_exit(0);
}

/// There is no message box on this platform; tell the user so.
fn display_log(is_error: bool) {
    let kind = if is_error { "an error" } else { "a" };
    let _ = writeln!(
        std::io::stderr().lock(),
        "Do not know how to display {kind} box"
    );
}
