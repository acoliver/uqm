// Game Initialization Module
// Handles initialization of game structures, ships, and contexts

pub mod ffi;
pub mod init;
pub mod master;
pub mod setup;

pub use init::*;
pub use master::*;
pub use setup::*;

pub mod world_resources;

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod macro_abi;

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod cleanup;

pub mod space_resources;
