// mix.rs - Main mixing logic

//! Main audio mixing logic for the mixer.
//!
//! This module provides the core mixing functionality that combines multiple
//! audio sources into a single output stream.

use crate::sound::mixer::types::*;
use crate::sound::mixer::source::{MixerSource, mixer_get_source};
use crate::sound::mixer::buffer::MixerBuffer;
use crate::sound::mixer::resample::{get_sample_int, put_sample_int, resample_none, resample_nearest, resample_linear, resample_cubic};
use parking_lot::Mutex;
use std::sync::Arc;

/// Global mixer state
static MIXER_STATE: Mutex<Option<MixerState>> = Mutex::new(None);

/// Mixer state structure
#[derive(Debug)]
struct MixerState {
    last_error: MixerError,
    format: MixerFormat,
    chansize: u32,
    sampsize: u32,
    freq: u32,
    channels: u32,
    quality: MixerQuality,
    flags: MixerFlags,
    active_sources: [Option<usize>; MAX_SOURCES],
}

impl MixerState {
    fn new(freq: u32, format: MixerFormat, quality: MixerQuality, flags: MixerFlags) -> Self {
        let chansize = format.bytes_per_channel();
        let channels = format.channels();
        MixerState {
            last_error: MixerError::NoError,
            format,
            chansize,
            sampsize: format.sample_size(),
            freq,
            channels,
            quality,
            flags,
            active_sources: [None; MAX_SOURCES],
        }
    }
}

/// Initialize the mixer
///
/// Sets up the mixer with the specified audio format and quality settings.
pub fn mixer_init(
    frequency: u32,
    format: MixerFormat,
    quality: MixerQuality,
    flags: MixerFlags,
) -> Result<(), MixerError> {
    let mut state = MIXER_STATE.lock();

    // Already initialized - uninit first
    if state.is_some() {
        drop(state);
        mixer_uninit()?;
        state = MIXER_STATE.lock();
    }

    *state = Some(MixerState::new(frequency, format, quality, flags));

    Ok(())
}

/// Uninitialize the mixer
///
/// Cleans up all mixer state.
pub fn mixer_uninit() -> Result<(), MixerError> {
    let mut state = MIXER_STATE.lock();

    if state.is_none() {
        return Ok(());
    }

    *state = None;

    Ok(())
}

/// Get the last error
pub fn mixer_get_error() -> MixerError {
    let mut state = MIXER_STATE.lock();
    if let Some(ref mut s) = *state {
        let error = s.last_error;
        s.last_error = MixerError::NoError;
        error
    } else {
        MixerError::InvalidOperation
    }
}

/// Check if mixer is initialized
pub fn mixer_is_initialized() -> bool {
    MIXER_STATE.lock().is_some()
}

/// Get the mixer format
pub fn mixer_get_format() -> MixerFormat {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.format).unwrap_or(MixerFormat::Stereo16)
}

/// Get the mixer frequency
pub fn mixer_get_frequency() -> u32 {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.freq).unwrap_or(44100)
}

/// Get the mixer channels
pub fn mixer_get_channels() -> u32 {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.channels).unwrap_or(2)
}

/// Get the mixer channel size
pub fn mixer_get_chansize() -> u32 {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.chansize).unwrap_or(2)
}

/// Get the mixer sample size
pub fn mixer_get_sampsize() -> u32 {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.sampsize).unwrap_or(4)
}

/// Get the mixer quality
pub fn mixer_get_quality() -> MixerQuality {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.quality).unwrap_or(MixerQuality::Medium)
}

/// Get the mixer flags
pub fn mixer_get_flags() -> MixerFlags {
    let state = MIXER_STATE.lock();
    state.as_ref().map(|s| s.flags).unwrap_or(MixerFlags::None)
}

/// Activate a source (add to active list)
pub fn activate_source(handle: usize) -> Result<(), MixerError> {
    let mut state_guard = MIXER_STATE.lock();
    let state = state_guard.as_mut().ok_or(MixerError::InvalidOperation)?;

    // Check if source is already active
    for active in &state.active_sources {
        if *active == Some(handle) {
            return Ok(());
        }
    }

    // Find empty slot
    for slot in &mut state.active_sources {
        if slot.is_none() {
            *slot = Some(handle);
            return Ok(());
        }
    }

    // No available slot
    state.last_error = MixerError::InvalidOperation;
    Err(MixerError::InvalidOperation)
}

/// Deactivate a source (remove from active list)
pub fn deactivate_source(handle: usize) {
    let mut state_guard = MIXER_STATE.lock();
    if let Some(state) = state_guard.as_mut() {
        for slot in &mut state.active_sources {
            if *slot == Some(handle) {
                *slot = None;
                break;
            }
        }
    }
}

/// Get the next sample from a source
fn get_next_sample(
    _source_handle: usize,
    source: &Arc<Mutex<MixerSource>>,
    left: bool,
) -> Option<f32> {
    let src = source.lock();
    let buf_handle = src.next_queued?;

    let sample_cache = src.sample_cache;
    let pos = src.pos;
    let _count = src.count;
    let gain = src.gain;
    let org_channels = 2; // Will be read from buffer

    drop(src); // Release lock before accessing buffer

    // Get buffer
    let buffer = crate::sound::mixer::buffer::mixer_get_buffer(buf_handle)?;
    let buf = buffer.lock();

    if buf.data.is_none() || buf.size < buf.sampsize {
        return None;
    }

    let mut src = source.lock();

    // For mono sources, duplicate left channel to right
    if !left && org_channels == 1 {
        src.sample_cache = sample_cache;
        return Some(sample_cache * gain);
    }

    // Resample based on quality
    let sample = match mixer_get_quality() {
        MixerQuality::Low => {
            let (s, new_pos) = resample_nearest(&src, &buf, pos, buf.sampsize, left);
            src.pos = new_pos;
            s
        }
        MixerQuality::High => {
            let (s, new_pos) = resample_cubic(&src, &buf, pos, buf.sampsize, left);
            src.pos = new_pos;
            s
        }
        _ => {
            // Medium or Default - use linear
            let (s, new_pos) = resample_linear(&src, &buf, pos, buf.sampsize, left);
            src.pos = new_pos;
            s
        }
    };

    drop(src); // Release lock before accessing buffer again

    // Update buffer state
    let buffer_binding = crate::sound::mixer::buffer::mixer_get_buffer(buf_handle)?;
    let mut buf = buffer_binding.lock();

    let mut src = source.lock();

    if src.pos < buf.size || (left && buf.sampsize != mixer_get_sampsize()) {
        buf.state = BufferState::Playing as u32;
    } else {
        // Buffer exhausted
        buf.state = BufferState::Processed as u32;
        src.pos = 0;
        src.prev_queued = src.next_queued;
        src.next_queued = buf.next;
        src.processed_count += 1;
    }

    src.sample_cache = sample * gain;

    Some(sample * gain)
}

/// Mix audio channels from all active sources
///
/// This is the main mixing function called by the audio callback.
pub fn mixer_mix_channels(stream: &mut [u8]) -> Result<(), MixerError> {
    let state = MIXER_STATE.lock();

    if state.is_none() {
        // Not initialized - output silence
        for byte in stream.iter_mut() {
            *byte = 0;
        }
        return Ok(());
    }

    let chansize = state.as_ref().unwrap().chansize;
    let channels = state.as_ref().unwrap().channels;

    drop(state);

    // Get all sources ONCE at the start (like C code does with its mutex)
    let sources = crate::sound::mixer::source::get_all_sources();
    
    // mixer_sampsize is the size of one complete sample (all channels)
    let mixer_sampsize = (chansize * channels) as u32;
    
    let mut stream_idx = 0;
    let mut left = true;

    while stream_idx < stream.len() {
        let mut fullsamp: f32 = 0.0;

        // Mix all playing sources
        for (_handle, source) in &sources {
            // The C mixer holds its src/buf/active locks for the whole callback.
            // Skipping a source because we failed to lock it causes periodic dropouts
            // (heard as crackle), which are much more obvious on bass.
            let mut src_guard = source.lock();

            if src_guard.state != (SourceState::Playing as u32) {
                continue;
            }

            // Get next buffer handle
            let buf_handle = match src_guard.next_queued {
                Some(h) => h,
                None => {
                    // No buffer queued - stop playback
                    src_guard.state = SourceState::Stopped as u32;
                    continue;
                }
            };

            // Get buffer
            let buffer = match crate::sound::mixer::buffer::mixer_get_buffer(buf_handle) {
                Some(b) => b,
                None => continue,
            };

            let mut buf_guard = buffer.lock();

            if buf_guard.data.is_none() || buf_guard.size == 0 {
                continue;
            }

            let buf_org_channels = buf_guard.org_channels;
            let buf_org_freq = buf_guard.org_freq;
            let buf_size = buf_guard.size;
            let buf_sampsize = buf_guard.sampsize;
            let buf_high = buf_guard.high;
            let buf_low = buf_guard.low;
            let mixer_freq = crate::sound::mixer::mix::mixer_get_frequency();

            // Debug: log first time we see music buffer (44100 Hz stereo)
            static LOGGED_MUSIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if buf_org_freq == 44100 && buf_org_channels == 2 && !LOGGED_MUSIC.load(std::sync::atomic::Ordering::Relaxed) {
                LOGGED_MUSIC.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Some(ref data) = buf_guard.data {
                    crate::bridge_log::rust_bridge_log_msg(&format!(
                        "MUSIC_BUF_DEBUG: org_freq={} org_chans={} chansize={} sampsize={} size={} data.len={} high={} low={} mixer_freq={} mixer_chans={}",
                        buf_org_freq, buf_org_channels, chansize, buf_sampsize, buf_size, data.len(), buf_high, buf_low, mixer_freq, channels
                    ));
                }
            }

            // Handle mono source on right channel - just use cached sample
            let sample = if !left && buf_org_channels == 1 {
                src_guard.sample_cache
            } else if let Some(ref data) = buf_guard.data {
                // Capture pos BEFORE any modifications
                let pos_before = src_guard.pos as usize;

                // Determine resampling mode
                let same_freq = mixer_freq == buf_org_freq;

                if same_freq {
                    // ResampleNone (C):
                    //   d0 = data + pos;
                    //   pos += mixer_chansize;
                    //   (void)left;
                    //   return sample(d0);
                    //
                    // This intentionally ignores `left` and simply walks forward by one
                    // channel each call. The outer mixer alternates left/right, so for
                    // interleaved stereo [L][R][L][R] this naturally yields L0, R0, L1, R1...
                    let read_offset = pos_before;

                    let s = if read_offset + 1 < data.len() && read_offset + 1 < buf_size as usize {
                        i16::from_le_bytes([data[read_offset], data[read_offset + 1]])
                    } else {
                        0
                    };

                    src_guard.pos += chansize as u32;

                    s as f32 * src_guard.gain
                } else {
                    // ResampleNearest: d0 = data + pos; d0 += SourceAdvance(left); return sample(d0)
                    let return_offset = if buf_org_channels == 2 && channels == 2 {
                        if left {
                            0usize
                        } else {
                            src_guard.pos += buf_high;
                            src_guard.count += buf_low;
                            if src_guard.count > 0xFFFF {
                                src_guard.count -= 0xFFFF;
                                src_guard.pos += buf_sampsize;
                            }
                            chansize as usize
                        }
                    } else {
                        src_guard.pos += buf_high;
                        src_guard.count += buf_low;
                        if src_guard.count > 0xFFFF {
                            src_guard.count -= 0xFFFF;
                            src_guard.pos += buf_sampsize;
                        }
                        0usize
                    };

                    let read_offset = pos_before + return_offset;

                    let s = if read_offset + 1 < data.len() && read_offset < buf_size as usize {
                        i16::from_le_bytes([data[read_offset], data[read_offset + 1]])
                    } else {
                        0
                    };

                    s as f32 * src_guard.gain
                }
            } else {
                0.0
            };

            // Cache sample for mono->stereo duplication
            if left || buf_org_channels != 1 {
                src_guard.sample_cache = sample;
            }

            fullsamp += sample;

            // Update buffer state / exhaustion logic (matches mixer.c 1047-1060)
            if src_guard.pos < buf_size || (left && buf_sampsize != mixer_sampsize) {
                buf_guard.state = BufferState::Playing as u32;
            } else {
                // buffer exhausted, go next
                buf_guard.state = BufferState::Processed as u32;
                src_guard.pos = 0;
                src_guard.prev_queued = src_guard.next_queued;
                src_guard.next_queued = buf_guard.next;
                src_guard.processed_count += 1;
            }
        }

        // Clip the sample
        let clipped = if chansize == 2 {
            // 16-bit clipping
            if fullsamp > SINT16_MAX {
                SINT16_MAX
            } else if fullsamp < SINT16_MIN {
                SINT16_MIN
            } else {
                fullsamp
            }
        } else {
            // 8-bit clipping
            if fullsamp > SINT8_MAX {
                SINT8_MAX
            } else if fullsamp < SINT8_MIN {
                SINT8_MIN
            } else {
                fullsamp
            }
        };

        // Write to output stream
        put_sample_int(stream, stream_idx, chansize, clipped as i32);
        stream_idx += chansize as usize;

        // Update left/right flag for stereo
        if channels == 2 {
            left = !left;
        }
    }

    Ok(())
}

/// Fake mixing - only process buffer and source states without actual mixing
///
/// This is used for timing purposes without actual audio output.
pub fn mixer_mix_fake(stream: &mut [u8]) -> Result<(), MixerError> {
    let state = MIXER_STATE.lock();

    if state.is_none() {
        return Err(MixerError::InvalidOperation);
    }

    let chansize = state.as_ref().unwrap().chansize;
    let channels = state.as_ref().unwrap().channels;

    drop(state);

    let mut stream_idx = 0;
    let mut left = true;

    // Just advance through all sources without mixing
    while stream_idx < stream.len() {
        let mut active = Vec::new();

        {
            let state_guard = MIXER_STATE.lock();
            if let Some(state) = state_guard.as_ref() {
                for &handle in &state.active_sources {
                    if let Some(h) = handle {
                        active.push(h);
                    }
                }
            }
        }

        for handle in active {
            if let Some(source) = mixer_get_source(handle) {
                // Just advance position, don't get actual sample
                let _ = get_next_sample(handle, &source, left);
            }
        }

        // Write silence
        for i in 0..chansize as usize {
            stream[stream_idx + i] = 128;
        }
        stream_idx += chansize as usize;

        if channels == 2 {
            left = !left;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Helper to ensure mixer is clean before/after each test
    fn with_mixer<F>(freq: u32, format: MixerFormat, quality: MixerQuality, flags: MixerFlags, f: F)
    where
        F: FnOnce(),
    {
        // Ensure clean state
        let _ = mixer_uninit();
        mixer_init(freq, format, quality, flags).unwrap();
        f();
        let _ = mixer_uninit();
    }

    #[test]
    #[serial]
    fn test_mixer_init() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Stereo16, MixerQuality::Medium, MixerFlags::None).unwrap();
        assert!(mixer_is_initialized());
        mixer_uninit().unwrap();
    }

    #[test]
    #[serial]
    fn test_mixer_init_uninit() {
        let _ = mixer_uninit();
        assert!(!mixer_is_initialized());

        mixer_init(48000, MixerFormat::Mono16, MixerQuality::High, MixerFlags::None).unwrap();
        assert!(mixer_is_initialized());
        assert_eq!(mixer_get_frequency(), 48000);
        assert_eq!(mixer_get_format(), MixerFormat::Mono16);
        assert_eq!(mixer_get_quality(), MixerQuality::High);

        mixer_uninit().unwrap();
        assert!(!mixer_is_initialized());
    }

    #[test]
    #[serial]
    fn test_mixer_reinit() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Stereo16, MixerQuality::Medium, MixerFlags::None).unwrap();
        mixer_init(48000, MixerFormat::Mono16, MixerQuality::High, MixerFlags::None).unwrap();

        assert_eq!(mixer_get_frequency(), 48000);
        assert_eq!(mixer_get_format(), MixerFormat::Mono16);

        mixer_uninit().unwrap();
    }

    #[test]
    #[serial]
    fn test_mixer_get_params() {
        let _ = mixer_uninit();
        mixer_init(22050, MixerFormat::Stereo8, MixerQuality::Low, MixerFlags::FakeData).unwrap();

        assert_eq!(mixer_get_frequency(), 22050);
        assert_eq!(mixer_get_format(), MixerFormat::Stereo8);
        assert_eq!(mixer_get_channels(), 2);
        assert_eq!(mixer_get_chansize(), 1);
        assert_eq!(mixer_get_sampsize(), 2);
        assert_eq!(mixer_get_quality(), MixerQuality::Low);
        assert_eq!(mixer_get_flags(), MixerFlags::FakeData);

        mixer_uninit().unwrap();
    }

    #[test]
    #[serial]
    fn test_mixer_error() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Stereo16, MixerQuality::Medium, MixerFlags::None).unwrap();

        // Mix without initialization should succeed and output silence
        mixer_uninit().unwrap();

        let mut stream = vec![0u8; 100];
        let result = mixer_mix_channels(&mut stream);
        assert!(result.is_ok());
        assert!(stream.iter().all(|&b| b == 0));
    }

    #[test]
    #[serial]
    fn test_mixer_mix_channels_empty() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Mono8, MixerQuality::Medium, MixerFlags::None).unwrap();

        let mut stream = vec![0u8; 10];
        mixer_mix_channels(&mut stream).unwrap();

        // With no active sources, output should be silent (0 for signed audio)
        // The mixing loop writes 0 when there are no sources
        for &byte in &stream {
            assert_eq!(byte, 0);
        }

        mixer_uninit().unwrap();
    }

    #[test]
    #[serial]
    fn test_mixer_mix_fake() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Mono8, MixerQuality::Medium, MixerFlags::None).unwrap();

        let mut stream = vec![0u8; 10];
        mixer_mix_fake(&mut stream).unwrap();

        // Fake mixing writes 128 (silence in unsigned 8-bit)
        for &byte in &stream {
            assert_eq!(byte, 128);
        }

        mixer_uninit().unwrap();
    }

    #[test]
    #[serial]
    fn test_mixer_mix_channels_mono16() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Mono16, MixerQuality::Medium, MixerFlags::None).unwrap();

        let mut stream = vec![0u8; 20]; // 10 samples
        mixer_mix_channels(&mut stream).unwrap();

        // Should be silent (all zeros)
        for i in (0..stream.len()).step_by(2) {
            let sample = i16::from_le_bytes([stream[i], stream[i + 1]]);
            assert_eq!(sample, 0);
        }

        mixer_uninit().unwrap();
    }

    #[test]
    #[serial]
    fn test_mixer_mix_channels_stereo16() {
        let _ = mixer_uninit();
        mixer_init(44100, MixerFormat::Stereo16, MixerQuality::Medium, MixerFlags::None).unwrap();

        let mut stream = vec![0u8; 40]; // 10 stereo samples
        mixer_mix_channels(&mut stream).unwrap();

        // Should be silent (all zeros)
        for i in (0..stream.len()).step_by(2) {
            let sample = i16::from_le_bytes([stream[i], stream[i + 1]]);
            assert_eq!(sample, 0);
        }

        mixer_uninit().unwrap();
    }
}
