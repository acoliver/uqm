// Resource Module
// Handles loading and caching of game resources

#[cfg(test)]
pub(crate) mod abort_child;
pub mod direct;
pub mod dispatch;
pub mod ffi_bridge;
pub mod ffi_types;
#[cfg(test)]
mod fixture_tests;
pub mod propfile;
pub mod raw_resource_type;
pub mod resource_type;
pub mod string_bank;
pub mod string_table;
pub mod type_registry;

pub use propfile::*;
pub use resource_type::*;
