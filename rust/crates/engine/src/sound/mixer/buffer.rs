// buffer.rs - Audio buffer management

//! Audio buffer management for the mixer.
//!
//! Buffers hold audio data that can be queued to sources for playback.
//! This module provides functions to create, delete, and manipulate buffers.

use crate::sound::mixer::types::*;
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio buffer containing PCM data
#[derive(Debug)]
pub struct MixerBuffer {
    /// Magic number for validation (MIXB)
    pub magic: u32,
    /// Whether the buffer is locked (being modified)
    pub locked: bool,
    /// Current buffer state
    pub state: u32,
    /// Internal audio data (converted format)
    pub data: Option<Vec<u8>>,
    /// Size of data in bytes
    pub size: u32,
    /// Size of one sample in internal format
    pub sampsize: u32,
    /// High part of resampling ratio
    pub high: u32,
    /// Low part of resampling ratio (fractional)
    pub low: u32,

    // Original buffer values for OpenAL compatibility
    /// Original data pointer (for FFI compatibility)
    pub org_data: Option<Vec<u8>>,
    /// Original frequency
    pub org_freq: u32,
    /// Original size
    pub org_size: u32,
    /// Original number of channels
    pub org_channels: u32,
    /// Original bytes per channel
    pub org_chansize: u32,

    /// Next buffer in queue
    pub next: Option<usize>,
}

impl MixerBuffer {
    /// Create a new uninitialized buffer
    pub fn new() -> Self {
        MixerBuffer {
            magic: MIXER_BUF_MAGIC,
            locked: false,
            state: BufferState::Initial as u32,
            data: None,
            size: 0,
            sampsize: 0,
            high: 0,
            low: 0,
            org_data: None,
            org_freq: 0,
            org_size: 0,
            org_channels: 0,
            org_chansize: 0,
            next: None,
        }
    }

    /// Validate that this is a valid buffer
    pub fn is_valid(&self) -> bool {
        self.magic == MIXER_BUF_MAGIC
    }

    /// Check if buffer is in a valid state for operations
    pub fn check_state(&self) -> Result<(), MixerError> {
        if self.magic != MIXER_BUF_MAGIC {
            return Err(MixerError::InvalidName);
        }
        if self.locked {
            return Err(MixerError::InvalidOperation);
        }
        Ok(())
    }
}

impl Default for MixerBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global buffer storage
///
/// Uses Arc<Mutex<>> for thread-safe access to the buffer pool.
/// Buffers are stored by index in the vector.
static BUFFER_POOL: Mutex<Vec<Option<Arc<Mutex<MixerBuffer>>>>> = Mutex::new(Vec::new());

/// Generate new buffer objects
///
/// Returns handles to newly created buffers.
/// Handles are 1-based (handle 0 = "no buffer" sentinel, matching OpenAL convention).
pub fn mixer_gen_buffers(n: u32) -> Result<Vec<usize>, MixerError> {
    if n == 0 {
        return Ok(Vec::new());
    }

    let mut pool = BUFFER_POOL.lock();

    // Reserve index 0 as "no buffer" sentinel on first allocation
    if pool.is_empty() {
        pool.push(None);
    }

    let mut handles = Vec::with_capacity(n as usize);

    for _ in 0..n {
        let buffer = Arc::new(Mutex::new(MixerBuffer::new()));
        let index = pool.len();
        pool.push(Some(buffer));
        handles.push(index);
    }

    Ok(handles)
}

/// Delete buffer objects
///
/// Removes buffers from the pool. All buffers must be valid and not in use.
pub fn mixer_delete_buffers(handles: &[usize]) -> Result<(), MixerError> {
    if handles.is_empty() {
        return Ok(());
    }

    let mut pool = BUFFER_POOL.lock();

    // First pass: validate all buffers can be deleted
    for &handle in handles {
        if handle >= pool.len() {
            return Err(MixerError::InvalidName);
        }

        let buffer = pool.get(handle).and_then(|b| b.as_ref());
        match buffer {
            None => return Err(MixerError::InvalidName),
            Some(buf) => {
                let buf_guard = buf.lock();
                if !buf_guard.is_valid() {
                    return Err(MixerError::InvalidName);
                }
                if buf_guard.locked {
                    return Err(MixerError::InvalidOperation);
                }
                if buf_guard.state >= (BufferState::Queued as u32) {
                    return Err(MixerError::InvalidOperation);
                }
            }
        }
    }

    // Second pass: delete all buffers
    for &handle in handles {
        pool[handle] = None;
    }

    Ok(())
}

/// Check if a handle is a valid buffer
pub fn mixer_is_buffer(handle: usize) -> bool {
    let pool = BUFFER_POOL.lock();

    if handle >= pool.len() {
        return false;
    }

    match &pool[handle] {
        None => false,
        Some(buf) => buf.lock().is_valid(),
    }
}

/// Get a reference to a buffer by handle
pub fn mixer_get_buffer(handle: usize) -> Option<Arc<Mutex<MixerBuffer>>> {
    let pool = BUFFER_POOL.lock();

    if handle >= pool.len() {
        return None;
    }

    pool[handle].as_ref().map(Arc::clone)
}

/// Load audio data into a buffer
///
/// Converts the audio data to the mixer's internal format if necessary.
pub fn mixer_buffer_data(
    handle: usize,
    format: u32,
    data: &[u8],
    freq: u32,
    mixer_freq: u32,
    mixer_format: MixerFormat,
) -> Result<(), MixerError> {
    if data.is_empty() {
        return Err(MixerError::InvalidValue);
    }

    let buffer = mixer_get_buffer(handle).ok_or(MixerError::InvalidName)?;
    let mut buf = buffer.lock();

    buf.check_state()?;

    if buf.state > BufferState::Filled as u32 {
        return Err(MixerError::InvalidOperation);
    }

    // The format coming from C is MIX_FORMAT_MAKE encoded:
    // MIX_FORMAT_DUMMYID (0x00170000) | bytes_per_channel | (channels << 8)
    // For MIX_FORMAT_MONO16: 0x170102 = bpc=2, chans=1
    // For MIX_FORMAT_STEREO16: 0x170202 = bpc=2, chans=2
    let src_bpc = format & 0xFF;
    let src_chans = (format >> 8) & 0xFF;

    // Validate the format - if either is 0, something is wrong
    if src_bpc == 0 || src_chans == 0 {
        // Log the issue and return error
        crate::bridge_log::rust_bridge_log_msg(&format!(
            "RUST_MIXER_BUFFER_DATA: Invalid format 0x{:x} (bpc={}, chans={})",
            format, src_bpc, src_chans
        ));
        return Err(MixerError::InvalidValue);
    }

    // Store original buffer values for OpenAL compatibility, once the
    // upload is known to be acceptable.
    buf.org_data = Some(data.to_vec());
    buf.org_freq = freq;
    buf.org_size = data.len() as u32;
    buf.org_channels = src_chans;
    buf.org_chansize = src_bpc;

    // Calculate conversion parameters
    let dst_bpc = mixer_format.bytes_per_channel();
    let dst_chans_raw = mixer_format.channels();

    let src_sampsize = src_bpc * src_chans;
    let num_samples = data.len() / src_sampsize as usize;

    // Calculate destination sample size - keep source channels if less than mixer channels
    let dst_chans = if src_chans < dst_chans_raw {
        src_chans
    } else {
        dst_chans_raw
    };

    buf.sampsize = dst_bpc * dst_chans;

    // Check if format is compatible (same bpc, same or fewer channels)
    let format_compatible = src_bpc == dst_bpc && src_chans <= dst_chans_raw;

    let (converted_data, final_size) = if format_compatible {
        // Just copy the data - it's already in the right format
        let mut d = data.to_vec();
        // Convert 8-bit unsigned to signed if needed
        if src_bpc == 1 {
            for byte in &mut d {
                *byte ^= 0x80;
            }
        }
        let size = d.len();
        (d, size as u32)
    } else {
        let dst_size = num_samples * buf.sampsize as usize;
        // Need to convert
        let mut converted = vec![0u8; dst_size];

        let needs_size_up = src_bpc < dst_bpc; // 8-bit to 16-bit
        let needs_size_down = src_bpc > dst_bpc; // 16-bit to 8-bit
        let needs_stereo_down = src_chans > dst_chans; // stereo to mono

        let mut src_idx = 0usize;
        let mut dst_idx = 0usize;

        for _ in 0..num_samples {
            // For each output channel
            for _ch in 0..dst_chans {
                // Read sample from source
                let mut samp: i32 = if src_bpc == 2 {
                    // 16-bit source
                    if src_idx + 1 < data.len() {
                        i16::from_le_bytes([data[src_idx], data[src_idx + 1]]) as i32
                    } else {
                        0
                    }
                } else {
                    // 8-bit source (unsigned, convert to signed)
                    if src_idx < data.len() {
                        (data[src_idx] as i32) - 128
                    } else {
                        0
                    }
                };
                src_idx += src_bpc as usize;

                // Handle stereo downmix
                if needs_stereo_down {
                    let samp2: i32 = if src_bpc == 2 {
                        if src_idx + 1 < data.len() {
                            i16::from_le_bytes([data[src_idx], data[src_idx + 1]]) as i32
                        } else {
                            0
                        }
                    } else if src_idx < data.len() {
                        (data[src_idx] as i32) - 128
                    } else {
                        0
                    };
                    src_idx += src_bpc as usize;
                    samp = (samp + samp2) / 2;
                }

                // Convert sample size
                if needs_size_up {
                    // 8-bit to 16-bit: shift left 8 bits
                    samp <<= 8;
                } else if needs_size_down {
                    // 16-bit to 8-bit: shift right 8 bits
                    samp >>= 8;
                }

                // Write to destination
                if dst_bpc == 2 {
                    let s = (samp as i16).to_le_bytes();
                    if dst_idx + 1 < converted.len() {
                        converted[dst_idx] = s[0];
                        converted[dst_idx + 1] = s[1];
                    }
                } else if dst_idx < converted.len() {
                    converted[dst_idx] = samp as i8 as u8;
                }
                dst_idx += dst_bpc as usize;
            }

            // Skip extra source channels if not downmixing
            if !needs_stereo_down && src_chans > dst_chans {
                src_idx += (src_chans - dst_chans) as usize * src_bpc as usize;
            }
        }
        (converted, dst_size as u32)
    };

    // Diagnostic: check if buffer data is non-zero
    static BUF_DIAG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let bd = BUF_DIAG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if bd < 5 {
        let nz = converted_data.iter().position(|&b| b != 0);
        let first_bytes: Vec<u8> = converted_data.iter().take(16).copied().collect();
        eprintln!(
            "[BUF_DATA#{}] handle={} src_fmt=0x{:x} freq={} mixer_freq={} data_len={} nonzero={:?} first16={:?}",
            bd, handle, format, freq, mixer_freq, converted_data.len(), nz, first_bytes
        );
    }

    buf.data = Some(converted_data);
    buf.size = final_size;
    buf.state = BufferState::Filled as u32;

    // Calculate resampling parameters
    if mixer_freq == freq {
        buf.high = buf.sampsize;
        buf.low = 0;
    } else {
        buf.high = (freq / mixer_freq) * buf.sampsize;
        buf.low = ((freq % mixer_freq) << 16) / mixer_freq;
    }

    Ok(())
}

/// Get a buffer property
pub fn mixer_get_buffer_i(handle: usize, prop: BufferProp) -> Result<i32, MixerError> {
    let buffer = mixer_get_buffer(handle).ok_or(MixerError::InvalidName)?;
    let buf = buffer.lock();

    if buf.locked {
        return Err(MixerError::InvalidOperation);
    }

    if !buf.is_valid() {
        return Err(MixerError::InvalidName);
    }

    match prop {
        BufferProp::Frequency => Ok(buf.org_freq as i32),
        BufferProp::Bits => Ok((buf.org_chansize << 3) as i32),
        BufferProp::Channels => Ok(buf.org_channels as i32),
        BufferProp::Size => Ok(buf.org_size as i32),
        BufferProp::Data => Ok(0), // Return pointer to data - not implemented yet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_buffers() {
        let handles = mixer_gen_buffers(3).unwrap();
        assert_eq!(handles.len(), 3);
        assert!(mixer_is_buffer(handles[0]));
        assert!(mixer_is_buffer(handles[1]));
        assert!(mixer_is_buffer(handles[2]));
    }

    #[test]
    fn test_delete_buffers() {
        let handles = mixer_gen_buffers(2).unwrap();
        assert!(mixer_is_buffer(handles[0]));
        assert!(mixer_is_buffer(handles[1]));
        mixer_delete_buffers(&handles).unwrap();
        assert!(!mixer_is_buffer(handles[0]));
        assert!(!mixer_is_buffer(handles[1]));
    }

    #[test]
    fn test_is_buffer() {
        let handles = mixer_gen_buffers(1).unwrap();
        assert!(mixer_is_buffer(handles[0]));
        assert!(!mixer_is_buffer(999));
    }

    #[test]
    fn test_buffer_data_mono8() {
        let handles = mixer_gen_buffers(1).unwrap();
        let data = vec![128u8; 100]; // Silent mono 8-bit
        mixer_buffer_data(
            handles[0],
            MixerFormat::Mono8 as u32,
            &data,
            44100,
            44100,
            MixerFormat::Mono8,
        )
        .unwrap();
        let buffer = mixer_get_buffer(handles[0]).unwrap();
        let buf = buffer.lock();
        assert_eq!(buf.state, BufferState::Filled as u32);
        assert_eq!(buf.org_freq, 44100);
        assert_eq!(buf.org_size, 100);
    }

    #[test]
    fn test_buffer_data_stereo16() {
        let handles = mixer_gen_buffers(1).unwrap();
        let data = vec![0u8; 400]; // Silent stereo 16-bit (100 samples * 2 channels * 2 bytes)
        mixer_buffer_data(
            handles[0],
            MixerFormat::Stereo16 as u32,
            &data,
            48000,
            48000,
            MixerFormat::Stereo16,
        )
        .unwrap();
        let buffer = mixer_get_buffer(handles[0]).unwrap();
        let buf = buffer.lock();
        assert_eq!(buf.state, BufferState::Filled as u32);
        assert_eq!(buf.org_freq, 48000);
        assert_eq!(buf.org_size, 400);
    }

    #[test]
    fn test_get_buffer_i() {
        let handles = mixer_gen_buffers(1).unwrap();
        let data = vec![0u8; 100];
        mixer_buffer_data(
            handles[0],
            MixerFormat::Mono8 as u32,
            &data,
            22050,
            22050,
            MixerFormat::Mono8,
        )
        .unwrap();
        let freq = mixer_get_buffer_i(handles[0], BufferProp::Frequency).unwrap();
        assert_eq!(freq, 22050);
        let size = mixer_get_buffer_i(handles[0], BufferProp::Size).unwrap();
        assert_eq!(size, 100);
        let channels = mixer_get_buffer_i(handles[0], BufferProp::Channels).unwrap();
        assert_eq!(channels, 1);
        let bits = mixer_get_buffer_i(handles[0], BufferProp::Bits).unwrap();
        assert_eq!(bits, 8);
    }

    #[test]
    fn test_delete_invalid_buffer() {
        let result = mixer_delete_buffers(&[999]);
        assert_eq!(result, Err(MixerError::InvalidName));
    }

    #[test]
    fn test_buffer_data_empty() {
        let handles = mixer_gen_buffers(1).unwrap();
        let data = vec![];
        let result = mixer_buffer_data(
            handles[0],
            MixerFormat::Mono8 as u32,
            &data,
            44100,
            44100,
            MixerFormat::Mono8,
        );
        assert_eq!(result, Err(MixerError::InvalidValue));
    }

    #[test]
    fn test_buffer_new() {
        let buf = MixerBuffer::new();
        assert_eq!(buf.magic, MIXER_BUF_MAGIC);
        assert_eq!(buf.state, BufferState::Initial as u32);
        assert!(!buf.locked);
        assert!(buf.data.is_none());
    }

    #[test]
    fn test_buffer_default() {
        let buf = MixerBuffer::default();
        assert_eq!(buf.magic, MIXER_BUF_MAGIC);
        assert_eq!(buf.state, BufferState::Initial as u32);
    }

    #[test]
    fn test_buffer_validation() {
        let mut buf = MixerBuffer::new();
        assert!(buf.is_valid());
        assert!(buf.check_state().is_ok());
        buf.magic = 0xDEADBEEF;
        assert!(!buf.is_valid());
        assert_eq!(buf.check_state(), Err(MixerError::InvalidName));
        let mut buf2 = MixerBuffer::new();
        buf2.locked = true;
        assert_eq!(buf2.check_state(), Err(MixerError::InvalidOperation));
    }

    // The format word is MIX_FORMAT_MAKE: DUMMYID | bytes-per-channel | channels << 8.
    const MONO8: u32 = 0x0017_0101;
    const MONO16: u32 = 0x0017_0102;
    const STEREO8: u32 = 0x0017_0201;
    const STEREO16: u32 = 0x0017_0202;

    fn le16(samples: &[i16]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }

    /// What the mixer will play from a loaded buffer.
    struct Loaded {
        data: Option<Vec<u8>>,
        size: u32,
        sampsize: u32,
        high: u32,
        low: u32,
        org_channels: u32,
        org_chansize: u32,
        state: u32,
    }

    /// Loads `data` into a fresh buffer and reports what it holds afterwards.
    fn load(
        format: u32,
        data: &[u8],
        freq: u32,
        mixer_freq: u32,
        mixer_format: MixerFormat,
    ) -> Result<Loaded, MixerError> {
        let handle = mixer_gen_buffers(1).unwrap()[0];
        let loaded = mixer_buffer_data(handle, format, data, freq, mixer_freq, mixer_format);
        let buffer = mixer_get_buffer(handle).unwrap();
        let snapshot = {
            let guard = buffer.lock();
            Loaded {
                data: guard.data.clone(),
                size: guard.size,
                sampsize: guard.sampsize,
                high: guard.high,
                low: guard.low,
                org_channels: guard.org_channels,
                org_chansize: guard.org_chansize,
                state: guard.state,
            }
        };
        mixer_delete_buffers(&[handle]).unwrap();
        loaded.map(|()| snapshot)
    }

    #[test]
    fn eight_bit_mono_is_widened_to_signed_sixteen_bit() {
        // 8-bit PCM is unsigned with 128 as silence; the mixer plays signed 16-bit.
        let buffer = load(MONO8, &[128, 255, 0], 44100, 44100, MixerFormat::Stereo16).unwrap();
        assert_eq!(
            buffer.data.as_deref(),
            Some(&le16(&[0, 127 << 8, -128 << 8])[..])
        );
        assert_eq!(buffer.size, 6);
        // Mono sources stay mono even when the mixer output is stereo.
        assert_eq!(buffer.sampsize, 2);
        assert_eq!(buffer.org_channels, 1);
        assert_eq!(buffer.org_chansize, 1);
        assert_eq!(buffer.state, BufferState::Filled as u32);
    }

    #[test]
    fn stereo_sixteen_bit_is_averaged_when_the_mixer_is_mono() {
        let source = le16(&[1000, 3000, -1000, -3001, i16::MAX, i16::MAX]);
        let buffer = load(STEREO16, &source, 44100, 44100, MixerFormat::Mono16).unwrap();
        // Averages use integer division, so -4001 / 2 truncates toward zero.
        assert_eq!(
            buffer.data.as_deref(),
            Some(&le16(&[2000, -2000, i16::MAX])[..])
        );
        assert_eq!(buffer.sampsize, 2);
        assert_eq!(buffer.size, 6);
        assert_eq!(buffer.org_channels, 2);
    }

    #[test]
    fn sixteen_bit_sources_are_narrowed_for_an_eight_bit_mixer() {
        let buffer = load(
            MONO16,
            &le16(&[0x1234, -0x1234, 0x7fff]),
            44100,
            44100,
            MixerFormat::Mono8,
        )
        .unwrap();
        // The arithmetic shift keeps the sign: -0x1234 >> 8 is -19.
        assert_eq!(
            buffer.data.as_deref(),
            Some(&[0x12_u8, (-19_i8) as u8, 0x7f][..])
        );
        assert_eq!(buffer.sampsize, 1);
    }

    #[test]
    fn stereo_eight_bit_is_downmixed_and_widened_in_one_pass() {
        // Unsigned 8-bit pairs (130,134) and (100,100) become signed (2,6) and (-28,-28).
        let buffer = load(
            STEREO8,
            &[130, 134, 100, 100],
            44100,
            44100,
            MixerFormat::Mono16,
        )
        .unwrap();
        assert_eq!(buffer.data.as_deref(), Some(&le16(&[4 << 8, -28 << 8])[..]));
    }

    #[test]
    fn matching_sixteen_bit_formats_are_copied_unchanged() {
        let samples = le16(&[1, -2, 300, -400]);
        for (format, mixer_format) in [
            (MONO16, MixerFormat::Mono16),
            (MONO16, MixerFormat::Stereo16),
            (STEREO16, MixerFormat::Stereo16),
        ] {
            let buffer = load(format, &samples, 44100, 44100, mixer_format).unwrap();
            assert_eq!(buffer.data.as_deref(), Some(&samples[..]), "{format:#x}");
            assert_eq!(buffer.size as usize, samples.len());
        }
    }

    #[test]
    fn matching_eight_bit_formats_only_flip_the_sign_bit() {
        let buffer = load(
            STEREO8,
            &[0, 128, 255, 127],
            44100,
            44100,
            MixerFormat::Stereo8,
        )
        .unwrap();
        assert_eq!(
            buffer.data.as_deref(),
            Some(&[0x80_u8, 0x00, 0x7f, 0xff][..])
        );
    }

    #[test]
    fn a_trailing_partial_sample_is_dropped_by_conversion() {
        // Six bytes hold one and a half 16-bit stereo frames.
        let buffer = load(
            STEREO16,
            &[1, 0, 3, 0, 9, 9],
            44100,
            44100,
            MixerFormat::Mono16,
        )
        .unwrap();
        assert_eq!(buffer.size, 2);
        assert_eq!(buffer.data.as_deref(), Some(&le16(&[2])[..]));
    }

    #[test]
    fn the_original_buffer_description_is_kept_for_property_queries() {
        let source = le16(&[5, 6, 7, 8]);
        let handle = mixer_gen_buffers(1).unwrap()[0];
        mixer_buffer_data(
            handle,
            STEREO16,
            &source,
            22050,
            44100,
            MixerFormat::Stereo16,
        )
        .unwrap();
        assert_eq!(mixer_get_buffer_i(handle, BufferProp::Frequency), Ok(22050));
        assert_eq!(mixer_get_buffer_i(handle, BufferProp::Bits), Ok(16));
        assert_eq!(mixer_get_buffer_i(handle, BufferProp::Channels), Ok(2));
        assert_eq!(mixer_get_buffer_i(handle, BufferProp::Size), Ok(8));
        assert_eq!(mixer_get_buffer_i(handle, BufferProp::Data), Ok(0));
        let buffer = mixer_get_buffer(handle).unwrap();
        assert_eq!(buffer.lock().org_data.as_deref(), Some(&source[..]));

        buffer.lock().locked = true;
        assert_eq!(
            mixer_get_buffer_i(handle, BufferProp::Size),
            Err(MixerError::InvalidOperation)
        );
        buffer.lock().locked = false;
        buffer.lock().magic = 0;
        assert_eq!(
            mixer_get_buffer_i(handle, BufferProp::Size),
            Err(MixerError::InvalidName)
        );
        assert_eq!(
            mixer_get_buffer_i(usize::MAX, BufferProp::Size),
            Err(MixerError::InvalidName)
        );
        buffer.lock().magic = MIXER_BUF_MAGIC;
        mixer_delete_buffers(&[handle]).unwrap();
    }

    #[test]
    fn resampling_step_follows_the_ratio_of_source_to_mixer_frequency() {
        let data = le16(&[1, 2, 3, 4]);
        for (freq, mixer_freq, high, low) in [
            // Same rate: advance exactly one frame, no fraction.
            (44100, 44100, 2, 0),
            // Half rate: no whole frame per output sample, half a frame as a 16.16 fraction.
            (22050, 44100, 0, 32768),
            // 48 kHz into 44.1 kHz: one whole frame plus 3900/44100 of another.
            (48000, 44100, 2, ((48000 % 44100) << 16) / 44100),
            // Double rate: two whole frames, no fraction.
            (88200, 44100, 4, 0),
        ] {
            let buffer = load(MONO16, &data, freq, mixer_freq, MixerFormat::Mono16).unwrap();
            assert_eq!(
                (buffer.high, buffer.low),
                (high, low),
                "{freq} -> {mixer_freq}"
            );
        }
    }

    #[test]
    fn unusable_format_words_and_buffer_states_are_refused() {
        let handle = mixer_gen_buffers(1).unwrap()[0];
        let data = le16(&[1, 2]);
        // Either field of the format word being zero leaves no sample size to decode.
        for format in [0, 0x0017_0100, 0x0017_0002, 0x0017_0000] {
            assert_eq!(
                mixer_buffer_data(handle, format, &data, 44100, 44100, MixerFormat::Mono16),
                Err(MixerError::InvalidValue),
                "{format:#x}"
            );
        }
        assert_eq!(
            mixer_buffer_data(usize::MAX, MONO16, &data, 44100, 44100, MixerFormat::Mono16),
            Err(MixerError::InvalidName)
        );

        let buffer = mixer_get_buffer(handle).unwrap();
        buffer.lock().state = BufferState::Queued as u32;
        assert_eq!(
            mixer_buffer_data(handle, MONO16, &data, 44100, 44100, MixerFormat::Mono16),
            Err(MixerError::InvalidOperation),
            "a buffer a source is playing from cannot be refilled"
        );
        buffer.lock().state = BufferState::Filled as u32;
        buffer.lock().locked = true;
        assert_eq!(
            mixer_buffer_data(handle, MONO16, &data, 44100, 44100, MixerFormat::Mono16),
            Err(MixerError::InvalidOperation)
        );
        buffer.lock().locked = false;
        // A filled buffer that is not yet queued may be refilled.
        mixer_buffer_data(handle, MONO16, &data, 44100, 44100, MixerFormat::Mono16).unwrap();
        mixer_delete_buffers(&[handle]).unwrap();
    }
}
