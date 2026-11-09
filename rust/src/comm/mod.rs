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
pub(crate) mod comm_abi;
mod commanim_abi;
mod commglue_abi;
pub(crate) mod descriptors;
pub mod dispatch;
pub mod encounter;
pub mod ffi;
#[cfg(test)]
mod ffi_behavior_tests;
pub mod glue;
pub mod hail;
mod input;
pub(crate) mod live;
pub mod locdata;
mod oscill_abi;
pub mod oscilloscope;
pub mod phrase_state;
pub mod player_font;
mod presentation;
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
pub(crate) mod text_layout;
pub mod track;
pub mod types;

pub use animation::{AnimContext, CommAnimState};
pub use oscilloscope::Oscilloscope;
pub use response::{ResponseEntry, ResponseSystem};
pub use state::Comm;
pub use subtitle::{SubtitleChunk, SubtitleTracker};
pub use track::{SoundChunk, TrackManager};
pub use types::{
    AnimationDescData, CommData, CommError, CommIntroMode, CommResult, MAX_ANIMATIONS,
};

#[cfg(test)]
pub(crate) fn with_test_context<R>(
    call: impl FnOnce(&mut crate::engine::battle::Cx<'_>) -> R,
) -> R {
    let mut game = crate::engine::Game::new();
    let mut battle = crate::engine::battle::Battle::new();
    call(&mut crate::engine::battle::Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    ))
}

#[derive(Clone)]
pub struct CommOptions {
    pub oscilloscope: bool,
    pub slider: bool,
    pub window: Option<std::rc::Rc<std::cell::Cell<locdata::CRect>>>,
    pub intro: (u32, u32),
}
impl Default for CommOptions {
    fn default() -> Self {
        Self {
            oscilloscope: true,
            slider: true,
            window: None,
            intro: (0, 0),
        }
    }
}
