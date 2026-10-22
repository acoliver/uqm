//! Layout-checked declarations for the remaining engine graphics callers.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all,
    clashing_extern_declarations,
    reason = "bindgen-generated C ABI declarations"
)]
include!(concat!(env!("OUT_DIR"), "/graphics_abi.rs"));
