// @plan PLAN-20260225-AUDIO-HEART.P15
// @requirement REQ-VOLUME-INIT-01..05, REQ-VOLUME-CONTROL-01..05
// @requirement REQ-VOLUME-SOURCE-01..04, REQ-VOLUME-QUERY-01..03
#![allow(dead_code, unused_imports, unused_variables)]

//! Audio control — initialization, volume management, source lifecycle,
//! and global playback queries.
//!
//! Provides `init_sound`/`uninit_sound` for system lifecycle, per-category
//! volume control (SFX, speech), source stop/clean operations, and
//! `sound_playing`/`wait_for_sound_end` for synchronization.

use std::thread;
use std::time::Duration;

use parking_lot::Mutex;

use super::mixer::source as mixer_source;
use super::mixer::SourceProp;
use super::sfx;
use super::stream;
use super::types::*;

// =============================================================================
// Constants
// =============================================================================

/// Default volume for all categories.
pub const NORMAL_VOLUME: i32 = MAX_VOLUME;

// =============================================================================
// State
// =============================================================================

/// Per-category volume state.
pub struct VolumeState {
    /// Music volume (0..MAX_VOLUME).
    pub music_volume: i32,
    /// Music gain scale (0.0..1.0).
    pub music_volume_scale: f32,
    /// SFX gain scale (0.0..1.0).
    pub sfx_volume_scale: f32,
    /// Speech gain scale (0.0..1.0).
    pub speech_volume_scale: f32,
}

impl VolumeState {
    fn new() -> Self {
        VolumeState {
            music_volume: NORMAL_VOLUME,
            music_volume_scale: 1.0,
            sfx_volume_scale: 1.0,
            speech_volume_scale: 1.0,
        }
    }
}

static VOLUME: std::sync::LazyLock<Mutex<VolumeState>> =
    std::sync::LazyLock::new(|| Mutex::new(VolumeState::new()));

// =============================================================================
// Initialization (spec §3.5)
// =============================================================================

/// Initialize the sound system.
pub fn init_sound() -> AudioResult<()> {
    todo!("P17: init_sound")
}

/// Uninitialize the sound system.
pub fn uninit_sound() {
    todo!("P17: uninit_sound")
}

// =============================================================================
// Source Management
// =============================================================================

/// Stop a sound source by index.
pub fn stop_source(source_index: usize) -> AudioResult<()> {
    todo!("P17: stop_source")
}

/// Clean up a stopped source (unqueue buffers, rewind).
pub fn clean_source(source_index: usize) -> AudioResult<()> {
    todo!("P17: clean_source")
}

/// Stop all SFX sources.
pub fn stop_sound() {
    todo!("P17: stop_sound")
}

// =============================================================================
// Volume Control
// =============================================================================

/// Set the SFX volume and apply gain to all SFX sources.
pub fn set_sfx_volume(volume: i32) {
    todo!("P17: set_sfx_volume")
}

/// Set the speech volume and apply gain to the speech source.
pub fn set_speech_volume(volume: i32) {
    todo!("P17: set_speech_volume")
}

// =============================================================================
// Queries
// =============================================================================

/// Check if any sound is currently playing.
pub fn sound_playing() -> bool {
    todo!("P17: sound_playing")
}

/// Block until sound playback finishes.
///
/// `channel`: `None` = wait for all sources, `Some(ch)` = specific channel.
pub fn wait_for_sound_end(channel: Option<usize>) {
    todo!("P17: wait_for_sound_end")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- P16 TDD ---

    #[test]
    #[ignore = "P17: init_sound stub"]
    fn test_init_sound_ok() {
        let result = init_sound();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "P17: stop_source stub"]
    fn test_stop_source_invalid_index_error() {
        let result = stop_source(999);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "P17: clean_source stub"]
    fn test_clean_source_invalid_index_error() {
        let result = clean_source(999);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "P17: stop_sound stub"]
    fn test_stop_sound_all_sfx_channels() {
        stop_sound(); // should not panic
    }

    #[test]
    #[ignore = "P17: set_sfx_volume stub"]
    fn test_set_sfx_volume_all_channels() {
        set_sfx_volume(128); // should not panic
        let state = VOLUME.lock();
        assert!((state.sfx_volume_scale - (128.0 / 255.0)).abs() < 0.01);
    }

    #[test]
    #[ignore = "P17: set_speech_volume stub"]
    fn test_set_speech_volume_speech_source() {
        set_speech_volume(200); // should not panic
        let state = VOLUME.lock();
        assert!((state.speech_volume_scale - (200.0 / 255.0)).abs() < 0.01);
    }

    #[test]
    #[ignore = "P17: sound_playing stub"]
    fn test_sound_playing_false_when_idle() {
        assert!(!sound_playing());
    }

    #[test]
    #[ignore = "P17: wait_for_sound_end stub"]
    fn test_wait_for_sound_end_returns_when_not_playing() {
        wait_for_sound_end(None); // should return immediately
    }

    #[test]
    fn test_volume_state_new() {
        let state = VolumeState::new();
        assert_eq!(state.music_volume, NORMAL_VOLUME);
        assert!((state.music_volume_scale - 1.0).abs() < f32::EPSILON);
        assert!((state.sfx_volume_scale - 1.0).abs() < f32::EPSILON);
        assert!((state.speech_volume_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normal_volume_is_max() {
        assert_eq!(NORMAL_VOLUME, MAX_VOLUME);
    }
}
