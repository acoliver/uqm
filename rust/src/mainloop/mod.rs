//! UQM Main Loop subsystem — Activity types and FFI accessors.
//!
//! This module provides the Rust-side types and FFI bridge for the UQM
//! main loop's activity state machine. Phase P03 covers the type
//! definitions and FFI accessor wrappers; the actual game loop logic
//! and state machine are implemented in later phases (P05/P06).
//!
//! # Module Structure
//!
//! - [`types`] — `CBoolean`, `ActivityValue`, `ActivityKind`, activity flags
//! - [`c_extern`] — raw `extern "C"` declarations (unsafe)
//! - [`ffi`] — safe Rust wrappers around the externs
//!
//! # C ABI Reference
//!
//! All type mappings are verified against `sc2/src/libs/compiler.h`:
//!
//! | C type     | Rust type | Size  |
//! |------------|-----------|-------|
//! | `BOOLEAN`  | `c_int`   | 4 B   |
//! | `ACTIVITY` | `u16`     | 2 B   |
//! | `UWORD`    | `u16`     | 2 B   |
//! | `COUNT`    | `u16`     | 2 B   |
//! | `BYTE`     | `u8`      | 1 B   |
//!
//! @plan PLAN-20260707-MAINLOOP.P03

pub mod c_extern;
pub mod ffi;
pub mod game_loop;
pub mod init_reference;
pub mod state_machine;
pub mod types;

pub use ffi::{
    battle_with_frame_callback, get_chmmr_bomb_state, get_crew_enlisted, get_current_activity,
    get_global_flags_and_data, get_kohr_ah_killed_all, get_last_activity, get_next_activity,
    get_starbase_available, load_kernel, set_chmmr_bomb_state, set_current_activity,
    set_flash_rect_null, set_last_activity, set_main_exited, set_next_activity,
    set_player_input_all_or_explode, splash_with_bg_init_kernel, zero_global_velocity,
};
pub use types::{activity_flags, ActivityKind, ActivityValue, CBoolean};
pub use state_machine::{
    ActivityDecision, BreakAction, GameStateInfo, check_break, evaluate,
    post_encounter_clear, pre_dispatch_mutate, resolve_load_activity,
    should_continue, should_zero_velocity,
};

/// Errors that can occur in the main loop FFI bridge.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MainLoopError {
    /// An FFI call returned an unexpected value.
    #[error("main loop FFI error: {0}")]
    Ffi(String),
    /// LoadKernel returned FALSE — kernel initialization failed.
    #[error("LoadKernel failed — kernel initialization error")]
    LoadKernelFailed,
}
