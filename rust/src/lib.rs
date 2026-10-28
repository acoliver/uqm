// UQM Rust library
// Phase 1: Core Systems Modernization
// Phase 2: Graphics Modernization (drawable/context systems)
// Phase 6: Input/VControl system
// Phase 7: Alien Communication system

pub mod battle;
pub use uqm_engine::callback;

pub mod supermelee;

pub mod automation;
pub use uqm_engine::bridge_log;
pub mod c_bindings;
pub mod cli;
pub use uqm_engine::collections;
pub mod comm;
pub mod config;
pub mod engine;
pub mod front_end;
pub mod game_init;
pub use uqm_engine::graphics;
pub use uqm_engine::input;
pub use uqm_engine::io;
pub use uqm_engine::log_abi;
pub use uqm_engine::logging;
pub mod mainloop;
pub use uqm_engine::math;
pub use uqm_engine::memory;
pub use uqm_engine::network;
pub mod pickship;
pub mod planet_side;
pub mod planets;
pub use uqm_engine::platform;
pub use uqm_engine::resource;
pub mod savegame;
pub mod ships;
pub use uqm_engine::sound;
pub mod starbase;
pub mod state;
pub use uqm_engine::strings;
pub use uqm_engine::threading;
pub mod time;
pub use uqm_engine::video;

pub use bridge_log::rust_bridge_log_msg;
pub use cli::Cli;
pub use comm::{CommState, COMM_STATE};
pub use config::Options;
pub use input::{VControl, VCONTROL};
pub use logging::LogLevel;
pub use sound::rust_ova_DecoderVtbl;
pub use sound::rust_wav_DecoderVtbl;

pub mod navigation;

pub mod ui;

pub use uqm_engine::{log_debug, log_error, log_fatal, log_info, log_warning};

pub mod app;
