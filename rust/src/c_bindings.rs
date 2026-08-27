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
    include!(concat!(env!("OUT_DIR"), "/controller_input_abi.rs"));
}

extern "C" {
    static mut PlayerControls:
        [controller_input::CONTROL_TEMPLATE; crate::battle::battle_controls::NUM_PLAYERS];
}

/// Read one control-template selector without creating a reference to the C global.
///
/// # Safety
///
/// The caller must run on the game thread at a point where C cannot mutate
/// `PlayerControls` concurrently.
pub unsafe fn player_control_template(player: usize) -> Option<controller_input::CONTROL_TEMPLATE> {
    if player >= crate::battle::battle_controls::NUM_PLAYERS {
        return None;
    }
    let controls = std::ptr::addr_of!(PlayerControls).cast::<controller_input::CONTROL_TEMPLATE>();
    Some(unsafe { controls.add(player).read_volatile() })
}
