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

pub mod content;
pub mod dir_prep;
pub mod input_frame;
pub mod logging;
pub mod options;
pub mod options_abi;
pub use uqm_modes::mainloop::restart_menu;
pub mod startup;
pub mod state_machine;
pub mod teardown;
pub mod types;

pub use state_machine::{
    check_break, evaluate, post_encounter_clear, pre_dispatch_mutate, resolve_load_activity,
    should_continue, should_zero_velocity, ActivityDecision, BreakAction, GameStateInfo,
};
pub use types::{activity_flags, ActivityKind, ActivityValue, CBoolean};

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
    /// A Starbase or Outfit route ended with an error.
    #[error("starbase: {0}")]
    Starbase(#[from] uqm_modes::starbase::StarbaseError),
    #[error(transparent)]
    Navigation(#[from] uqm_engine::kernel::NavigationLoadError),
}

pub mod input_abi;
pub mod lifecycle_abi;
