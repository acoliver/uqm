// State Management Module
// Handles game state with bitfield-based storage for save/load compatibility

pub mod ffi;
pub mod game_state;
pub mod planet_info;
pub mod state_file;

pub use game_state::*;
pub use planet_info::*;
pub use state_file::*;
