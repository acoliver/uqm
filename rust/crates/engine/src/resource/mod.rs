// Resource Module
// Handles loading and caching of game resources

pub mod cache;
pub mod direct;
pub mod dispatch;
pub mod ffi_bridge;
pub mod ffi_types;
pub mod propfile;
pub mod resource_type;
pub mod settings;
pub mod stringbank;
pub mod type_registry;

/// A resource name as game records hold it (C: `RESOURCE`).
pub type ResourceKey = *const std::ffi::c_char;

pub use propfile::*;
pub use resource_type::*;
