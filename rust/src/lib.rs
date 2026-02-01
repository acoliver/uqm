// UQM Rust library
// Phase 1: Core Systems Modernization
// Phase 2: Graphics Modernization (drawable/context systems)
// Phase 6: Input/VControl system
// Phase 7: Alien Communication system

pub mod bridge_log;
pub mod c_bindings;
pub mod cli;
pub mod comm;
pub mod config;
pub mod game_init;
pub mod graphics;
pub mod input;
pub mod io;
pub mod logging;
pub mod memory;
pub mod resource;
pub mod sound;
pub mod state;
pub mod threading;
pub mod time;
pub mod video;

pub use bridge_log::rust_bridge_log_msg;
pub use cli::Cli;
pub use comm::{CommState, COMM_STATE};
pub use config::Options;
pub use input::{VControl, VCONTROL};
pub use logging::LogLevel;
pub use sound::rust_ova_DecoderVtbl;
pub use sound::rust_wav_DecoderVtbl;
