
/* FFI Bindings for Phase 0 */

// Re-export libc types for convenience
pub use libc::{c_int, c_char, c_void};

extern "C" {
    /// Entry point that Rust calls to start the C code
    /// This is defined in mem_wrapper.c
    pub fn c_entry_point(argc: c_int, argv: *mut *mut c_char) -> c_int;
}
