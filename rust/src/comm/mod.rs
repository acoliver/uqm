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
pub mod ffi;
pub mod oscilloscope;
pub mod response;
pub mod state;
pub mod subtitle;
pub mod track;
pub mod types;

pub use animation::{AnimContext, AnimDesc, AnimState};
pub use oscilloscope::Oscilloscope;
pub use response::{ResponseEntry, ResponseSystem};
pub use state::{CommState, COMM_STATE};
pub use subtitle::{SubtitleChunk, SubtitleTracker};
pub use track::{SoundChunk, TrackManager};
pub use types::{CommData, CommError, CommIntroMode, CommResult};
