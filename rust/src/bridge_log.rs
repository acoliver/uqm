//! Rust bridge logging channel
//!
//! This module provides C-ABI functions for logging from the C codebase into Rust.
//! It establishes the baseline infrastructure for Rust integration.

use libc::c_char;
use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

// Global log file handle protected by a mutex for thread safety
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

const LOG_PATH: &str = "rust-bridge.log";

/// Initialize the Rust bridge logging system.
///
/// This function creates or truncates the log file and writes the Phase 0 marker.
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn rust_bridge_init() -> libc::c_int {
    // Use absolute path to ensure log file is created in project root
    let log_path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(LOG_PATH);

    match File::create(&log_path) {
        Ok(mut file) => {
            // Write the Phase 0 marker with timestamp
            if let Err(_) = writeln!(file, "RUST_BRIDGE_PHASE0_OK") {
                eprintln!("rust_bridge_init: Failed to write marker to {:?}", log_path);
                return -1;
            }

            // Write initialization timestamp
            if let Err(_) = writeln!(file, "rust_bridge_init called at: {:?}", log_path) {
                eprintln!(
                    "rust_bridge_init: Failed to write timestamp to {:?}",
                    log_path
                );
                return -1;
            }

            // Store the file handle in the global mutex
            let mut guard = LOG_FILE.lock().unwrap();
            *guard = Some(file);

            0
        }
        Err(e) => {
            eprintln!(
                "rust_bridge_init: Failed to create log file {:?}: {}",
                log_path, e
            );
            -1
        }
    }
}

/// Log a message to the Rust bridge log file (internal Rust API).
///
/// This is a convenience function for Rust code to write to the log file
/// without going through the C FFI.
pub fn rust_bridge_log_msg(message: &str) {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *guard {
        let _ = writeln!(file, "{}", message);
        let _ = file.flush();
    }
}

/// Log a message to the Rust bridge log file.
///
/// # Safety
/// The message pointer must be a valid null-terminated C string.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn rust_bridge_log(message: *const c_char) -> libc::c_int {
    if message.is_null() {
        return -1;
    }

    // Convert C string to Rust string
    let c_str = unsafe { CStr::from_ptr(message) };
    let message_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // Get the log file handle from the global mutex
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *guard {
        if let Err(_) = writeln!(file, "{}", message_str) {
            return -1;
        }
        // Flush immediately to ensure logs are written
        if let Err(_) = file.flush() {
            return -1;
        }
        0
    } else {
        -1
    }
}
