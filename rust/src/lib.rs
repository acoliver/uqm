// UQM Rust library
// Phase 1: Core Systems Modernization
// Phase 2: Graphics Modernization (drawable/context systems)
// Phase 6: Input/VControl system
// Phase 7: Alien Communication system

pub use uqm_engine::callback;
pub use uqm_modes::battle;

pub use uqm_modes::supermelee;

pub mod automation;
pub use uqm_engine::bridge_log;
pub mod cli;
pub use uqm_engine::collections;
pub use uqm_modes::comm;
pub mod config;
pub use uqm_engine as engine;
pub use uqm_modes::front_end;
pub mod game_init;
pub use uqm_engine::graphics;
pub use uqm_engine::input_port as input;
pub use uqm_engine::io;
pub use uqm_engine::log_abi;
pub use uqm_engine::logging;
pub mod mainloop;
pub use uqm_engine::math;
pub use uqm_engine::memory;
pub use uqm_engine::network;
pub use uqm_engine::platform;
pub use uqm_engine::resource;
pub use uqm_modes::pickship;
pub use uqm_modes::planet_side;
pub use uqm_modes::planets;
pub mod savegame;
pub use uqm_content::ships;
pub use uqm_engine::sound;
pub use uqm_engine::state;
pub use uqm_engine::strings;
pub use uqm_engine::threading;
pub use uqm_modes::starbase;
pub mod time;
pub use uqm_engine::video;

pub use bridge_log::rust_bridge_log_msg;
pub use cli::Cli;
pub use comm::Comm;
pub use config::Options;
pub use input::VControl;
pub use logging::LogLevel;

pub use uqm_modes::navigation;

pub use uqm_modes::ui;

pub use uqm_engine::{log_debug, log_error, log_fatal, log_info, log_warning};

pub mod app;

#[cfg(test)]
extern crate self as uqm_rust;
