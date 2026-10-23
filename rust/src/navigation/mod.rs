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

pub mod events;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod globals;
pub mod groups;
pub mod initial_state;
pub mod interplanetary;

#[cfg(any(not(test), feature = "linked_c_archive"))]
fn get(name: &str) -> u8 {
    let (start, end) = crate::state::game_state::lookup_game_state_bits(name)
        .expect("navigation state key exists");
    unsafe { crate::state::ffi::rust_get_game_state_bits(start as i32, end as i32) }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
fn set(name: &str, value: u8) {
    let (start, end) = crate::state::game_state::lookup_game_state_bits(name)
        .expect("navigation state key exists");
    unsafe { crate::state::ffi::rust_set_game_state_bits(start as i32, end as i32, value) }
}
