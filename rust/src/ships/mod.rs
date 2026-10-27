// Ship Subsystem - Phase 2 Ships Modernization
// @plan PLAN-20260314-SHIPS.P03

pub mod battle_bridge;
pub mod c_bridge;
pub mod catalog;
pub(crate) mod code_resources;
pub mod ffi_contract;
pub mod lifecycle;
pub mod loader;
pub(crate) mod loader_native;
pub mod master_native;
pub mod native;
pub mod queue;
pub mod races;
pub mod registry;
pub mod types;
pub mod writeback;

pub mod ffi;

pub mod native_adapters;

pub mod hook_registry;
