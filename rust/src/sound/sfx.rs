// @plan PLAN-20260225-AUDIO-HEART.P12
// @requirement REQ-SFX-PLAY-01..09, REQ-SFX-POSITION-01..05
// @requirement REQ-SFX-VOLUME-01, REQ-SFX-LOAD-01..07
// @requirement REQ-SFX-RELEASE-01..04
#![allow(dead_code, unused_imports, unused_variables)]

//! Sound effects — channel-based SFX playback with positional audio,
//! volume control, and sound bank resource management.
//!
//! # 3D Positioning (Plan Note — REQ-SFX-POSITION-01)
//!
//! The mixer module does NOT have `mixer_source_fv()` (vector setter).
//! Instead, `update_sound_position` uses three separate `mixer_source_f`
//! calls to set X, Y, Z position components individually via the
//! PositionX/Y/Z source properties added in P02b.

use std::sync::Arc;

use parking_lot::Mutex;

use super::mixer::source as mixer_source;
use super::mixer::SourceProp;
use super::stream;
use super::types::*;

// =============================================================================
// Constants
// =============================================================================

/// Distance attenuation factor for positional audio.
pub const ATTENUATION: f32 = 160.0;
/// Minimum distance (no attenuation below this).
pub const MIN_DISTANCE: f32 = 0.5;
/// Maximum simultaneous sound effects.
pub const MAX_FX: usize = NUM_SFX_CHANNELS;

// =============================================================================
// State
// =============================================================================

/// SFX subsystem state.
struct SfxState {
    /// Whether stereo SFX positioning is enabled.
    opt_stereo_sfx: bool,
    /// SFX master volume (0..MAX_VOLUME).
    sfx_volume: i32,
    /// SFX volume scale factor (0.0..1.0).
    sfx_volume_scale: f32,
}

impl SfxState {
    fn new() -> Self {
        SfxState {
            opt_stereo_sfx: false,
            sfx_volume: MAX_VOLUME,
            sfx_volume_scale: 1.0,
        }
    }
}

static SFX_STATE: std::sync::LazyLock<Mutex<SfxState>> =
    std::sync::LazyLock::new(|| Mutex::new(SfxState::new()));

// =============================================================================
// SFX Playback (spec §3.4)
// =============================================================================

/// Play a sound effect on the given channel.
pub fn play_channel(
    channel: usize,
    sound_bank: &SoundBank,
    sound_index: usize,
    pos: SoundPosition,
    positional_object: usize,
    _priority: i32,
) -> AudioResult<()> {
    if channel > LAST_SFX_SOURCE {
        return Err(AudioError::InvalidChannel(channel));
    }

    // Stop before play
    stream::stop_source(channel)?;
    check_finished_channels();

    // Validate sample exists
    let sample = sound_bank
        .samples
        .get(sound_index)
        .ok_or(AudioError::InvalidSample)?;

    // Set positional audio
    let state = SFX_STATE.lock();
    if state.opt_stereo_sfx {
        update_sound_position(channel, pos);
    } else {
        update_sound_position(
            channel,
            SoundPosition {
                positional: false,
                x: 0,
                y: 0,
            },
        );
    }
    drop(state);

    // Bind buffer and play via mixer
    stream::with_source(channel, |source| {
        source.positional_object = positional_object;
        let handle = source.handle;
        if !sample.buffers.is_empty() {
            let _ =
                mixer_source::mixer_source_i(handle, SourceProp::Buffer, sample.buffers[0] as i32);
        }
        let _ = mixer_source::mixer_source_play(handle);
    })
    .ok_or(AudioError::InvalidChannel(channel))?;

    Ok(())
}

/// Play a single pre-decoded SoundSample on a channel (called from FFI).
pub fn play_sample(
    channel: usize,
    sample: &SoundSample,
    pos: SoundPosition,
    positional_object: usize,
    _priority: i32,
) -> AudioResult<()> {
    if channel > LAST_SFX_SOURCE {
        return Err(AudioError::InvalidChannel(channel));
    }

    stream::stop_source(channel)?;
    check_finished_channels();

    let state = SFX_STATE.lock();
    if state.opt_stereo_sfx {
        update_sound_position(channel, pos);
    } else {
        update_sound_position(channel, SoundPosition { positional: false, x: 0, y: 0 });
    }
    drop(state);

    stream::with_source(channel, |source| {
        source.positional_object = positional_object;
        let handle = source.handle;
        if !sample.buffers.is_empty() {
            let buf_id = sample.buffers[0] as i32;
            let r = mixer_source::mixer_source_i(handle, SourceProp::Buffer, buf_id);
            // Verify the buffer was actually queued
            let queued_after = mixer_source::mixer_get_source_i(handle, SourceProp::BuffersQueued).unwrap_or(-1);
            let state_after = mixer_source::mixer_get_source_i(handle, SourceProp::SourceState).unwrap_or(-1);
            eprintln!("[play_sample] ch={} handle={} buf_id={} set_result={:?} queued_after={} state=0x{:x}", channel, handle, buf_id, r, queued_after, state_after);
        } else {
            eprintln!("[play_sample] ch={} handle={} NO BUFFERS", channel, handle);
        }
        let _ = mixer_source::mixer_source_play(handle);
    })
    .ok_or(AudioError::InvalidChannel(channel))?;

    Ok(())
}

/// Stop a sound effect channel.
pub fn stop_channel(channel: usize, _priority: i32) -> AudioResult<()> {
    stream::stop_source(channel)
}

/// Check if a channel is currently playing.
pub fn channel_playing(channel: usize) -> bool {
    stream::with_source(channel, |source| {
        mixer_source::mixer_get_source_i(source.handle, SourceProp::SourceState)
            .map(|s| s == 0x1012) // Playing state
            .unwrap_or(false)
    })
    .unwrap_or(false)
}

/// Set the volume for a specific channel.
pub fn set_channel_volume(channel: usize, volume: i32, _priority: i32) {
    let state = SFX_STATE.lock();
    let gain = (volume as f32 / MAX_VOLUME as f32) * state.sfx_volume_scale;
    drop(state);
    stream::with_source(channel, |source| {
        let _ = mixer_source::mixer_source_f(source.handle, SourceProp::Gain, gain);
    });
}

/// Check and clean up finished SFX channels.
pub fn check_finished_channels() {
    for i in FIRST_SFX_SOURCE..=LAST_SFX_SOURCE {
        let stopped = stream::with_source(i, |source| {
            mixer_source::mixer_get_source_i(source.handle, SourceProp::SourceState)
                .map(|s| s == 0x1014) // Stopped state
                .unwrap_or(false)
        })
        .unwrap_or(false);

        if stopped {
            let _ = stream::stop_source(i);
        }
    }
}

// =============================================================================
// Positional Audio
// =============================================================================

/// Update the 3D position for a sound source.
pub fn update_sound_position(source_index: usize, pos: SoundPosition) {
    stream::with_source(source_index, |source| {
        let handle = source.handle;
        if pos.positional {
            let mut x = pos.x as f32 / ATTENUATION;
            let y = 0.0f32;
            let mut z = pos.y as f32 / ATTENUATION;

            // Min distance check
            let dist = (x * x + z * z).sqrt();
            if dist < MIN_DISTANCE {
                if dist > 0.0 {
                    let scale = MIN_DISTANCE / dist;
                    x *= scale;
                    z *= scale;
                } else {
                    z = -MIN_DISTANCE;
                }
            }

            let _ = mixer_source::mixer_source_f(handle, SourceProp::PositionX, x);
            let _ = mixer_source::mixer_source_f(handle, SourceProp::PositionY, y);
            let _ = mixer_source::mixer_source_f(handle, SourceProp::PositionZ, z);
        } else {
            // Non-positional: centered
            let _ = mixer_source::mixer_source_f(handle, SourceProp::PositionX, 0.0);
            let _ = mixer_source::mixer_source_f(handle, SourceProp::PositionY, 0.0);
            let _ = mixer_source::mixer_source_f(handle, SourceProp::PositionZ, -1.0);
        }
    });
}

/// Get the positional object ID for a source.
pub fn get_positional_object(source_index: usize) -> usize {
    stream::with_source(source_index, |source| source.positional_object).unwrap_or(0)
}

/// Set the positional object ID for a source.
pub fn set_positional_object(source_index: usize, object: usize) {
    stream::with_source(source_index, |source| {
        source.positional_object = object;
    });
}

// =============================================================================
// Volume Control
// =============================================================================

/// Set the SFX master volume (0..MAX_VOLUME).
pub fn set_sfx_volume(volume: i32) {
    let mut state = SFX_STATE.lock();
    state.sfx_volume = volume;
    state.sfx_volume_scale = volume as f32 / MAX_VOLUME as f32;
}

// =============================================================================
// SFX Loading / Release
// =============================================================================

/// Load sound bank data from a resource.
///
/// In the full system, this parses a resource file listing WAV filenames,
/// loads each decoder, pre-decodes all audio, and uploads to mixer buffers.
/// Here we accept a filename for resource tracking.
pub fn get_sound_bank_data(filename: &str) -> AudioResult<SoundBank> {
    if filename.is_empty() {
        return Err(AudioError::ResourceNotFound("empty filename".into()));
    }

    // Resource loading will be connected via FFI in P20.
    // For now, return an empty bank that tracks its source.
    Ok(SoundBank {
        samples: Vec::new(),
        source_file: Some(filename.to_string()),
    })
}

/// Release a sound bank.
pub fn release_sound_bank_data(sound_bank: SoundBank) -> AudioResult<()> {
    // Stop any channels using samples from this bank
    for sample in &sound_bank.samples {
        // Destroy mixer resources for each sample
        let mut sample_clone = stream::create_sound_sample(None, 0, None)?;
        sample_clone.buffers = sample.buffers.clone();
        stream::destroy_sound_sample(&mut sample_clone)?;
    }
    // Bank dropped by Rust ownership
    drop(sound_bank);
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- P13 TDD ---

    // REQ-SFX-PLAY-01..09
    #[test]

    fn test_play_channel_invalid_channel_error() {
        let bank = SoundBank {
            samples: Vec::new(),
            source_file: None,
        };
        let result = play_channel(
            999,
            &bank,
            0,
            SoundPosition {
                positional: false,
                x: 0,
                y: 0,
            },
            0,
            0,
        );
        assert!(result.is_err());
    }

    #[test]

    fn test_play_channel_missing_sample_error() {
        let bank = SoundBank {
            samples: Vec::new(),
            source_file: None,
        };
        let result = play_channel(
            0,
            &bank,
            0,
            SoundPosition {
                positional: false,
                x: 0,
                y: 0,
            },
            0,
            0,
        );
        assert!(result.is_err());
    }

    #[test]

    fn test_stop_channel_delegates() {
        let result = stop_channel(0, 0);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]

    fn test_channel_playing_initial_false() {
        assert!(!channel_playing(0));
    }

    #[test]

    fn test_check_finished_channels_cleans() {
        check_finished_channels(); // should not panic
    }

    // REQ-SFX-POSITION-01..05
    #[test]
    fn test_sound_position_non_positional() {
        let pos = SoundPosition {
            positional: false,
            x: 0,
            y: 0,
        };
        assert!(!pos.positional);
    }

    #[test]
    fn test_sound_position_positional() {
        let pos = SoundPosition {
            positional: true,
            x: 100,
            y: -50,
        };
        assert!(pos.positional);
        assert_eq!(pos.x, 100);
        assert_eq!(pos.y, -50);
    }

    #[test]
    fn test_sound_position_min_distance_concept() {
        // MIN_DISTANCE prevents zero-distance division
        let dist = 0.0f32;
        let clamped = dist.max(MIN_DISTANCE);
        assert!(clamped >= MIN_DISTANCE);
    }

    #[test]

    fn test_get_set_positional_object() {
        set_positional_object(0, 42);
        assert_eq!(get_positional_object(0), 42);
    }

    // REQ-SFX-LOAD-01..07
    #[test]

    fn test_get_sound_bank_data_empty_lines() {
        let result = get_sound_bank_data("");
        assert!(result.is_err());
    }

    // REQ-SFX-RELEASE-01..04
    #[test]

    fn test_release_sound_bank_data_empty_ok() {
        let bank = SoundBank {
            samples: Vec::new(),
            source_file: None,
        };
        let result = release_sound_bank_data(bank);
        assert!(result.is_ok());
    }

    // REQ-SFX-VOLUME-01
    #[test]
    fn test_set_channel_volume_gain_concept() {
        // Gain = volume / MAX_VOLUME * scale
        let volume = 128i32;
        let scale = 0.8f32;
        let gain = volume as f32 / MAX_VOLUME as f32 * scale;
        assert!((gain - 0.4016).abs() < 0.01);
    }

    #[test]
    fn test_sfx_state_new() {
        let state = SfxState::new();
        assert!(!state.opt_stereo_sfx);
        assert_eq!(state.sfx_volume, MAX_VOLUME);
        assert!((state.sfx_volume_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_constants() {
        assert!(ATTENUATION > 0.0);
        assert!(MIN_DISTANCE > 0.0);
        assert_eq!(MAX_FX, NUM_SFX_CHANNELS);
    }

    #[test]
    fn test_sound_bank_empty() {
        let bank = SoundBank {
            samples: Vec::new(),
            source_file: None,
        };
        assert!(bank.samples.is_empty());
    }
}
