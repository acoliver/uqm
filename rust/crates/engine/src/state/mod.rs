// State Management Module
// Handles game state with bitfield-based storage for save/load compatibility

pub mod ffi;
pub mod game_state;
pub mod game_state_keys;

pub use game_state::*;
