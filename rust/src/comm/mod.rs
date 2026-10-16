//! Alien Communication System
//!
//! This module handles alien dialogue, response selection, animation
//! synchronization, and speech/subtitle playback.
//!
//! # Architecture
//!
//! The communication system consists of:
//! - Track management for speech playback
//! - Subtitle timing and display
//! - Response selection system
//! - Animation synchronization
//! - Oscilloscope display for audio waveform
//!
//! # Thread Safety
//!
//! The comm system is mostly single-threaded but stream callbacks
//! come from the audio thread. Shared state is protected by Mutex.

pub mod animation;
pub mod dispatch;
pub mod encounter;
pub mod ffi;
#[cfg(test)]
mod ffi_behavior_tests;
pub mod glue;
pub mod hail;
pub mod locdata;
pub mod oscilloscope;
pub mod phrase_state;
pub mod player_font;
pub mod races;
pub mod response;
pub mod response_ui;
pub mod segue;
pub mod sis_graphics;
#[cfg(test)]
mod sis_graphics_font_tests;
pub mod speech_graphics;
pub mod state;
pub mod subtitle;
pub mod subtitle_display;
pub mod summary;
pub mod talk_segue;
pub mod track;
pub mod types;

pub use animation::{AnimContext, CommAnimState};
pub use oscilloscope::Oscilloscope;
pub use response::{ResponseEntry, ResponseSystem};
pub use state::{CommState, COMM_STATE};
pub use subtitle::{SubtitleChunk, SubtitleTracker};
pub use track::{SoundChunk, TrackManager};
pub use types::{
    AnimationDescData, CommData, CommError, CommIntroMode, CommResult, MAX_ANIMATIONS,
};
