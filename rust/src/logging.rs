use std::ffi::CString;

/// Log levels matching the C enum
#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Nothing = 0,
    User = 1,
    Error = 2,
    Warning = 3,
    Info = 4,
    Debug = 5,
    All = 6,
}

impl LogLevel {
    /// Create a LogLevel from an integer
    #[allow(dead_code)]
    pub fn from_i32(level: i32) -> Self {
        match level {
            0 => LogLevel::Nothing,
            1 => LogLevel::User,
            2 => LogLevel::Error,
            3 => LogLevel::Warning,
            4 => LogLevel::Info,
            5 => LogLevel::Debug,
            6 => LogLevel::All,
            _ => LogLevel::Info,
        }
    }

    /// Get the integer representation for the C interface
    #[allow(dead_code)]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Add a log entry by calling the C log_add function via FFI
///
/// # Safety
/// This function calls into C code via FFI
pub unsafe fn log_add(level: LogLevel, message: &str) {
    // Convert the Rust string to a C string
    let _c_msg = match CString::new(message) {
        Ok(s) => s,
        Err(_) => {
            // If the string contains a null byte, we can't convert it
            // Just return silently or handle however is appropriate
            return;
        }
    };

    // Call the C log_add function
    // Note: In Phase 0, this is a stub - the actual C function may not be available
    // For now, we'll just print to stderr
    eprintln!("[{:?}] {}", level, message);
}

/// Initialize the logging system
///
/// # Safety
/// This function calls into C code via FFI
pub unsafe fn log_init(max_lines: i32) {
    // Call the C log_init function
    // For Phase 0, this is a stub
    eprintln!("Logging initialized with max_lines = {}", max_lines);
}

/// Convenience macro for fatal errors
#[macro_export]
macro_rules! log_fatal {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            unsafe {
                $crate::logging::log_add($crate::logging::LogLevel::User, &msg)
            }
        }
    };
}

/// Convenience macro for errors
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            unsafe {
                $crate::logging::log_add($crate::logging::LogLevel::Error, &msg)
            }
        }
    };
}

/// Convenience macro for warnings
#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            unsafe {
                $crate::logging::log_add($crate::logging::LogLevel::Warning, &msg)
            }
        }
    };
}

/// Convenience macro for info messages
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            unsafe {
                $crate::logging::log_add($crate::logging::LogLevel::Info, &msg)
            }
        }
    };
}

/// Convenience macro for debug messages
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            unsafe {
                $crate::logging::log_add($crate::logging::LogLevel::Debug, &msg)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_i32() {
        assert_eq!(LogLevel::from_i32(0), LogLevel::Nothing);
        assert_eq!(LogLevel::from_i32(1), LogLevel::User);
        assert_eq!(LogLevel::from_i32(2), LogLevel::Error);
        assert_eq!(LogLevel::from_i32(3), LogLevel::Warning);
        assert_eq!(LogLevel::from_i32(4), LogLevel::Info);
        assert_eq!(LogLevel::from_i32(5), LogLevel::Debug);
        assert_eq!(LogLevel::from_i32(6), LogLevel::All);
    }

    #[test]
    fn test_log_level_as_i32() {
        assert_eq!(LogLevel::Nothing.as_i32(), 0);
        assert_eq!(LogLevel::User.as_i32(), 1);
        assert_eq!(LogLevel::Error.as_i32(), 2);
        assert_eq!(LogLevel::Warning.as_i32(), 3);
        assert_eq!(LogLevel::Info.as_i32(), 4);
        assert_eq!(LogLevel::Debug.as_i32(), 5);
        assert_eq!(LogLevel::All.as_i32(), 6);
    }

    #[test]
    fn test_log_level_invalid() {
        // Invalid values should default to Info
        assert_eq!(LogLevel::from_i32(100), LogLevel::Info);
        assert_eq!(LogLevel::from_i32(-1), LogLevel::Info);
    }

    #[test]
    fn test_log_add() {
        unsafe {
            log_add(LogLevel::Info, "Test message");
        }
    }

    #[test]
    fn test_log_init() {
        unsafe {
            log_init(100);
        }
    }

    #[test]
    fn test_log_macros() {
        log_info!("Info message: {}", 42);
        log_error!("Error code: {}", -1);
        log_warning!("Warning!");
        log_debug!("Debug info");
        log_fatal!("Fatal error");
    }
}
