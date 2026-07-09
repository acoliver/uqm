//! Restart menu subsystem — types, navigation logic, and FFI bridge.
//!
//! This module ports the C restart menu (`sc2/src/uqm/restart.c`) to Rust,
//! covering `DoRestart`, `RestartMenu`, `TryStartGame`, and `StartGame`.
//!
//! # Module Structure
//!
//! - [`types`] — `RestartMenuItem`, `SelectionResult`, `MenuInputState`
//! - [`menu_logic`] — pure navigation/selection/timeout functions
//! - [`c_extern`] — raw FFI extern declarations and C constants
//! - [`restart_ops`] — `RestartMenuOps` trait and `CffiOps` production impl
//! - [`do_restart`] — per-frame DoRestart callback logic
//!
//! @plan PLAN-20260707-RESTARTMENU.P02
//! @requirement REQ-RM-001

pub mod c_extern;
pub mod do_restart;
pub mod menu_logic;
pub mod orchestration;
pub mod restart_ops;
pub mod types;
