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

#[cfg(not(test))]
pub mod macro_abi;

#[cfg(not(test))]
pub mod cleanup;

pub mod space_resources;

#[cfg(not(test))]
pub mod kernel_abi;
