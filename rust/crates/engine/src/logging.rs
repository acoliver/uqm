use std::ffi::CString;

/// Game log levels, most severe first. A message is shown when its level is
/// at or below the output threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    User,
    Error,
    Warning,
    Info,
    Debug,
}

impl LogLevel {
    /// Semantic alias for `User` matching C's `log_Fatal = log_User`.
    /// OOM handlers and other fatal-error paths should use this for clarity.
    /// @plan PLAN-20260224-MEM-SWAP.P03
    /// @requirement REQ-MEM-005
    pub const FATAL: LogLevel = LogLevel::User;
}

/// Add a log entry by calling the C log_add function via FFI
pub fn log_add(level: LogLevel, message: &str) {
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

/// Convenience macro for fatal errors
#[macro_export]
macro_rules! log_fatal {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            $crate::logging::log_add($crate::logging::LogLevel::User, &msg)
        }
    };
}

/// Convenience macro for errors
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            $crate::logging::log_add($crate::logging::LogLevel::Error, &msg)
        }
    };
}

/// Convenience macro for warnings
#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            $crate::logging::log_add($crate::logging::LogLevel::Warning, &msg)
        }
    };
}

/// Convenience macro for info messages
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            $crate::logging::log_add($crate::logging::LogLevel::Info, &msg)
        }
    };
}

/// Convenience macro for debug messages
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            $crate::logging::log_add($crate::logging::LogLevel::Debug, &msg)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_order_from_most_to_least_severe() {
        assert!(LogLevel::User < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
    }

    #[test]
    fn test_log_add() {
        log_add(LogLevel::Info, "Test message");
    }

    #[test]
    fn test_log_macros() {
        log_info!("Info message: {}", 42);
        log_error!("Error code: {}", -1);
        log_warning!("Warning!");
        log_debug!("Debug info");
        log_fatal!("Fatal error");
    }

    /// @plan PLAN-20260224-MEM-SWAP.P04
    /// @requirement REQ-MEM-005
    #[test]
    fn test_fatal_alias() {
        assert_eq!(LogLevel::FATAL, LogLevel::User);
    }
}
