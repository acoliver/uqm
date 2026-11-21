//! File-based runtime diagnostics.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use crate::host::FileStream;

// Global log file handle protected by a mutex for thread safety
static LOG_FILE: Mutex<Option<FileStream>> = Mutex::new(None);

/// Created in the working directory.
const LOG_PATH: &str = "rust-bridge.log";

/// Initialize the Rust bridge logging system.
/// This function creates or truncates the log file and writes the Phase 0 marker.
/// Returns 0 on success, -1 on failure.
pub fn rust_bridge_init() -> std::ffi::c_int {
    let log_path = Path::new(LOG_PATH);
    match crate::host::platform().disk().open_write(log_path) {
        Ok(mut file) => {
            if writeln!(file, "RUST_BRIDGE_PHASE0_OK").is_err()
                || writeln!(file, "rust_bridge_init called at: {log_path:?}").is_err()
            {
                eprintln!("rust_bridge_init: Failed to write marker to {log_path:?}");
                return -1;
            }
            *crate::sync::lock(&LOG_FILE) = Some(file);
            0
        }
        Err(e) => {
            eprintln!("rust_bridge_init: Failed to create log file {log_path:?}: {e}");
            -1
        }
    }
}

/// Log a message to the Rust bridge log file (internal Rust API).
///
/// This is a convenience function for Rust code to write to the log file
/// without going through the C FFI.
pub fn rust_bridge_log_msg(message: &str) {
    let mut guard = crate::sync::lock(&LOG_FILE);
    if let Some(ref mut file) = *guard {
        let _ = writeln!(file, "{}", message);
        let _ = file.flush();
    }
}
