//! Navigation-world state, encounters and movement.

#[allow(
    clippy::all,
    dead_code,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clashing_extern_declarations,
    reason = "Rust-owned engine ABI declarations, including opaque resources"
)]
pub mod abi {
    include!("../abi/navigation_abi.rs");
}

pub mod events;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod globals;
pub mod groups;
pub mod hyper_data;
pub mod hyper_encounters;
pub mod hyper_menu;
pub mod hyper_radar;
pub mod hyper_transition;
pub mod hyper_units;
pub mod hyperspace;
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
