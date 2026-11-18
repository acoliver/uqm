/* FFI Bindings for Phase 0 */

// Re-export libc types for convenience
pub use libc::{c_char, c_int};

#[allow(
    clippy::all,
    dead_code,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
pub mod controller_input {
    include!("abi/controller_input_abi.rs");
}
