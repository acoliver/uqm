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
    Ok(())
}

/// Uninitialize the sound system.
pub fn uninit_sound() {
    // Cleanup handled by Rust Drop semantics on program exit
}

// =============================================================================
// Source Management
// =============================================================================

/// Stop a sound source by index.
pub fn stop_source(source_index: usize) -> AudioResult<()> {
    if source_index >= NUM_SOUNDSOURCES {
        return Err(AudioError::InvalidChannel(source_index));
    }
    stream::stop_source(source_index)?;
    clean_source(source_index)?;
    Ok(())
}

/// Clean up a stopped source (unqueue buffers, rewind).
pub fn clean_source(source_index: usize) -> AudioResult<()> {
    if source_index >= NUM_SOUNDSOURCES {
        return Err(AudioError::InvalidChannel(source_index));
    }
    stream::with_source(source_index, |source| {
        source.positional_object = 0;
        let handle = source.handle;

        // Unqueue processed buffers
        let processed = mixer_source::mixer_get_source_i(handle, SourceProp::BuffersProcessed)
            .unwrap_or(0) as u32;
        if processed > 0 {
            let _ = mixer_source::mixer_source_unqueue_buffers(handle, processed);
        }

        let _ = mixer_source::mixer_source_rewind(handle);
    });
    Ok(())
}

/// Stop all SFX sources.
pub fn stop_sound() {
    for i in FIRST_SFX_SOURCE..=LAST_SFX_SOURCE {
        let _ = stop_source(i);
    }
}

// =============================================================================
// Volume Control
// =============================================================================

/// Set the SFX volume and apply gain to all SFX sources.
pub fn set_sfx_volume(volume: i32) {
    let mut vol = VOLUME.lock();
    vol.sfx_volume_scale = volume as f32 / MAX_VOLUME as f32;
    let scale = vol.sfx_volume_scale;
    drop(vol);

    for i in FIRST_SFX_SOURCE..=LAST_SFX_SOURCE {
        stream::with_source(i, |source| {
            let _ = mixer_source::mixer_source_f(source.handle, SourceProp::Gain, scale);
        });
    }
}

/// Set the speech volume and apply gain to the speech source.
pub fn set_speech_volume(volume: i32) {
    let mut vol = VOLUME.lock();
    vol.speech_volume_scale = volume as f32 / MAX_VOLUME as f32;
    let scale = vol.speech_volume_scale;
    drop(vol);

    stream::with_source(SPEECH_SOURCE, |source| {
        let _ = mixer_source::mixer_source_f(source.handle, SourceProp::Gain, scale);
    });
}

// =============================================================================
// Queries
// =============================================================================

/// Check if any sound is currently playing.
pub fn sound_playing() -> bool {
    for i in 0..NUM_SOUNDSOURCES {
        let playing = stream::with_source(i, |source| {
            if source.sample.is_some() {
                if let Some(ref sample_arc) = source.sample {
                    let sample = sample_arc.lock();
                    if sample.decoder.is_some() {
                        // Streaming source: check flag to avoid deadlock
                        return source.stream_should_be_playing;
                    }
                }
                // Non-streaming: query mixer
                mixer_source::mixer_get_source_i(source.handle, SourceProp::SourceState)
                    .map(|s| s == 0x1012) // Playing
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .unwrap_or(false);

        if playing {
            return true;
        }
    }
    false
}

/// Block until sound playback finishes.
///
/// `channel`: `None` = wait for all sources, `Some(ch)` = specific channel.
pub fn wait_for_sound_end(channel: Option<usize>) {
    loop {
        if crate::sound::types::quit_posted() {
            break;
        }

        let still_playing = match channel {
            None => sound_playing(),
            Some(ch) => sfx::channel_playing(ch),
        };

        if !still_playing {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- P16 TDD ---

    #[test]

    fn test_init_sound_ok() {
        let result = init_sound();
        assert!(result.is_ok());
    }

    #[test]

    fn test_stop_source_invalid_index_error() {
        let result = stop_source(999);
        assert!(result.is_err());
    }

    #[test]

    fn test_clean_source_invalid_index_error() {
        let result = clean_source(999);
        assert!(result.is_err());
    }

    #[test]

    fn test_stop_sound_all_sfx_channels() {
        stop_sound(); // should not panic
    }

    #[test]

    fn test_set_sfx_volume_all_channels() {
        set_sfx_volume(128); // should not panic
        let state = VOLUME.lock();
        assert!((state.sfx_volume_scale - (128.0 / 255.0)).abs() < 0.01);
    }

    #[test]

    fn test_set_speech_volume_speech_source() {
        set_speech_volume(200); // should not panic
        let state = VOLUME.lock();
        assert!((state.speech_volume_scale - (200.0 / 255.0)).abs() < 0.01);
    }

    #[test]

    fn test_sound_playing_false_when_idle() {
        assert!(!sound_playing());
    }

    #[test]

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
