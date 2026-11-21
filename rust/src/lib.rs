// UQM Rust library
// Phase 1: Core Systems Modernization
// Phase 2: Graphics Modernization (drawable/context systems)
// Phase 6: Input/VControl system
// Phase 7: Alien Communication system
#![forbid(unsafe_code)]

pub mod automation;
pub mod cli;
pub mod config;
pub mod game_init;
pub mod mainloop;
pub mod savegame;
pub mod time;

pub use cli::Cli;
pub use config::Options;

pub mod app;

#[cfg(test)]
extern crate self as uqm_rust;
