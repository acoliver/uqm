//! Navigation-world state, encounters and movement.

#[allow(
    clippy::all,
    dead_code,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clashing_extern_declarations,
    reason = "header-generated C ABI declarations, including opaque engine resources"
)]
pub mod abi {
    include!(concat!(env!("OUT_DIR"), "/navigation_abi.rs"));
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod globals;
pub mod initial_state;

