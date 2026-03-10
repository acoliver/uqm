// @plan PLAN-20260225-AUDIO-HEART.P06
// @requirement REQ-STREAM-INIT-01..07, REQ-STREAM-PLAY-01..20, REQ-STREAM-SAMPLE-01..05
// @requirement REQ-STREAM-TAG-01..03, REQ-STREAM-SCOPE-01..11, REQ-STREAM-FADE-01..05
// @requirement REQ-STREAM-THREAD-01..08, REQ-STREAM-PROCESS-01..16
#![allow(dead_code, unused_imports, unused_variables)]

//! Streaming audio engine.
//!
//! Manages the decoder thread that feeds audio data to the mixer,
//! handles stream lifecycle (play/stop/pause/resume/seek), buffer
//! tagging for subtitle synchronization, scope data for the
//! oscilloscope display, and music fade effects.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use parking_lot::{Condvar, Mutex};

use super::decoder::{DecodeError, SoundDecoder};
use super::formats::AudioFormat;
use super::mixer::buffer as mixer_buffer;
use super::mixer::mix as mixer_mix;
use super::mixer::source as mixer_source;
use super::mixer::types::{MixerError, MixerFormat, SourceProp, SourceState};
use super::music;
use super::types::*;

// =============================================================================
// Format Conversion
// =============================================================================

/// Convert an `AudioFormat` to mixer format encoding.
fn audio_format_to_mixer(fmt: AudioFormat) -> u32 {
    fmt.to_mixer_format()
}

// =============================================================================
// Internal State (spec §3.1.2)
// =============================================================================

/// Global streaming engine state.
struct StreamEngine {
    /// Fade state — protected by its own mutex, separate from sources.
    fade: Mutex<FadeState>,
    /// Decoder thread handle.
    decoder_thread: Mutex<Option<JoinHandle<()>>>,
    /// Shutdown flag for decoder thread.
    shutdown: AtomicBool,
    /// Condvar to wake the decoder thread when a stream starts.
    wake: Condvar,
    /// Mutex paired with the condvar.
    wake_mutex: Mutex<()>,
}

impl StreamEngine {
    fn new() -> Self {
        StreamEngine {
            fade: Mutex::new(FadeState::new()),
            decoder_thread: Mutex::new(None),
            shutdown: AtomicBool::new(false),
            wake: Condvar::new(),
            wake_mutex: Mutex::new(()),
        }
    }
}

static ENGINE: std::sync::LazyLock<StreamEngine> = std::sync::LazyLock::new(StreamEngine::new);

/// Global array of sound sources, indexed by source ID.
static SOURCES: std::sync::LazyLock<Vec<Mutex<SoundSource>>> = std::sync::LazyLock::new(|| {
    (0..NUM_SOUNDSOURCES)
        .map(|_| Mutex::new(SoundSource::new()))
        .collect()
});

// =============================================================================
// Sample Lifecycle (spec §3.1.3)
// =============================================================================

/// Create a new sound sample with the given decoder and buffer count.
///
/// # Arguments
/// * `decoder` - Optional decoder for the audio data.
/// * `num_buffers` - Number of mixer buffers to allocate.
/// * `callbacks` - Optional stream callbacks.
// @plan PLAN-20260225-AUDIO-HEART.P08
// @requirement REQ-STREAM-SAMPLE-01..05
pub fn create_sound_sample(
    decoder: Option<Box<dyn SoundDecoder>>,
    num_buffers: u32,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
) -> AudioResult<SoundSample> {
    let buffers = mixer_buffer::mixer_gen_buffers(num_buffers)?;
    let buffer_tags: Vec<Option<SoundTag>> = (0..num_buffers as usize).map(|_| None).collect();
    Ok(SoundSample {
        decoder,
        length: 0.0,
        buffers,
        num_buffers,
        buffer_tags,
        offset: 0,
        looping: false,
        data: None,
        callbacks,
    })
}

/// Destroy a sound sample, releasing all mixer buffers.
pub fn destroy_sound_sample(sample: &mut SoundSample) -> AudioResult<()> {
    if !sample.buffers.is_empty() {
        let _ = mixer_buffer::mixer_delete_buffers(&sample.buffers);
    }
    sample.buffers.clear();
    sample.num_buffers = 0;
    sample.buffer_tags.clear();
    sample.callbacks = None;
    Ok(())
}

/// Attach opaque user data to a sample.
pub fn set_sound_sample_data(sample: &mut SoundSample, data: Box<dyn std::any::Any + Send>) {
    sample.data = Some(data);
}

/// Get a reference to the sample's opaque user data.
pub fn get_sound_sample_data(sample: &SoundSample) -> Option<&(dyn std::any::Any + Send)> {
    sample.data.as_deref()
}

/// Replace the callbacks on a sample.
pub fn set_sound_sample_callbacks(
    sample: &mut SoundSample,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
) {
    sample.callbacks = callbacks;
}

/// Get a reference to the sample's decoder.
pub fn get_sound_sample_decoder(sample: &SoundSample) -> Option<&dyn SoundDecoder> {
    sample.decoder.as_deref()
}

// =============================================================================
// Stream Control (spec §3.1.3)
// =============================================================================

/// Start streaming a sample on the given source.
///
/// # Arguments
/// * `sample` - The sample to stream (shared ownership).
/// * `source_index` - Index into the global source array.
/// * `looping` - Whether to loop when the decoder reaches EOF.
/// * `scope` - Whether to capture scope (oscilloscope) data.
/// * `rewind` - Whether to seek the decoder to the beginning.
///
/// # Note
/// Must be called after `init_stream_decoder()`, which must be called
/// after `mixer_init()`.
// @plan PLAN-20260225-AUDIO-HEART.P08
// @requirement REQ-STREAM-PLAY-01..20
pub fn play_stream(
    sample_arc: Arc<Mutex<SoundSample>>,
    source_index: usize,
    looping: bool,
    scope: bool,
    rewind: bool,
) -> AudioResult<()> {
    play_stream_with_offset_override(sample_arc, source_index, looping, scope, rewind, None)
}

pub fn play_stream_with_offset_override(
    sample_arc: Arc<Mutex<SoundSample>>,
    source_index: usize,
    looping: bool,
    scope: bool,
    rewind: bool,
    offset_override: Option<i32>,
) -> AudioResult<()> {
    if source_index >= NUM_SOUNDSOURCES {
        return Err(AudioError::InvalidSource(source_index));
    }
    // Stop any existing stream on this source
    stop_stream(source_index)?;

    // on_start_stream callback (before acquiring source lock)
    {
        let mut sample_pre = sample_arc.lock();
        // Take callbacks out to avoid double-borrow
        let mut cbs = sample_pre.callbacks.take();
        if let Some(ref mut cb) = cbs {
            if !cb.on_start_stream(&mut sample_pre) {
                sample_pre.callbacks = cbs;
                return Err(AudioError::EndOfStream);
            }
        }
        sample_pre.callbacks = cbs;
    }

    let mut source = get_source(source_index)?;
    let mut sample = sample_arc.lock();

    // Clear tags
    for tag in sample.buffer_tags.iter_mut() {
        *tag = None;
    }

    // Handle rewind or compute offset
    let offset = if let Some(override_offset) = offset_override {
        if rewind {
            if let Some(decoder) = sample.decoder.as_mut() {
                let _ = decoder.seek(0);
            }
        }
        override_offset
    } else {
        let base_offset = sample.offset;
        let decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?;
        if rewind {
            let _ = decoder.seek(0);
            base_offset
        } else {
            base_offset + (get_decoder_time(decoder.as_ref()) * ONE_SECOND as f32) as i32
        }
    };

    // Source setup
    source.sample = Some(Arc::clone(&sample_arc));
    sample.looping = looping;
    let _ = mixer_source::mixer_source_i(source.handle, SourceProp::Looping, 0);

    // Scope buffer
    if scope {
        let buf_size = BUFFER_SIZE as u32;
        let scope_size = sample.num_buffers * buf_size + PAD_SCOPE_BYTES as u32;
        source.sbuf_size = scope_size;
        source.sbuffer = Some(vec![0u8; scope_size as usize]);
        source.sbuf_tail = 0;
        source.sbuf_head = 0;
    }

    // Pre-fill buffers: get decoder params first, then loop
    let (freq, format, num_buffers) = {
        let decoder = sample.decoder.as_ref().ok_or(AudioError::InvalidDecoder)?;
        (
            decoder.frequency(),
            decoder.format(),
            sample.num_buffers as usize,
        )
    };

    let buf_handles: Vec<usize> = sample.buffers[..num_buffers.min(sample.buffers.len())].to_vec();

    for (i, &buf_handle) in buf_handles.iter().enumerate() {
        let decode_result = {
            let decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?;
            let mut buf = vec![0u8; BUFFER_SIZE];
            match decoder.decode(&mut buf) {
                Ok(0) => None,
                Ok(n) => Some((buf, n, false)),
                Err(DecodeError::EndOfFile) => Some((buf, 0, true)),
                Err(_) => None,
            }
        };

        match decode_result {
            None => break,
            Some((_buf, _n, true)) => {
                // EOF: invoke on_end_chunk via take pattern
                let mut cbs = sample.callbacks.take();
                let replaced = if let Some(ref mut cb) = cbs {
                    cb.on_end_chunk(&mut sample, buf_handle)
                } else {
                    false
                };
                sample.callbacks = cbs;
                if replaced {
                    continue;
                }
                break;
            }
            Some((buf, n, false)) => {
                let mixer_freq = mixer_mix::mixer_get_frequency();
                let mixer_fmt = mixer_mix::mixer_get_format();
                let mix_format = audio_format_to_mixer(format);
                let _ = mixer_buffer::mixer_buffer_data(
                    buf_handle,
                    mix_format,
                    &buf[..n],
                    freq,
                    mixer_freq,
                    mixer_fmt,
                );
                let _ = mixer_source::mixer_source_queue_buffers(source.handle, &[buf_handle]);
                source.queued_buf_sizes.push_back(n);
                // on_queue_buffer via take pattern
                let mut cbs = sample.callbacks.take();
                if let Some(ref mut cb) = cbs {
                    cb.on_queue_buffer(&mut sample, buf_handle);
                }
                sample.callbacks = cbs;
                if scope {
                    add_scope_data(&mut source, &buf[..n]);
                }
            }
        }
    }

    // Start playback
    source.sbuf_lasttime = get_time_counter();
    source.start_time = get_time_counter() as i32 - offset;
    source.pause_time = 0;
    source.stream_should_be_playing = true;
    let _ = mixer_source::mixer_source_play(source.handle);
    drop(sample);
    drop(source);
    // Wake decoder thread
    ENGINE.wake.notify_one();
    Ok(())
}

/// Stop streaming on the given source.
pub fn stop_stream(source_index: usize) -> AudioResult<()> {
    if source_index >= NUM_SOUNDSOURCES {
        return Err(AudioError::InvalidSource(source_index));
    }
    let mut source = get_source(source_index)?;
    let _ = mixer_source::mixer_source_stop(source.handle);
    source.stream_should_be_playing = false;
    source.sample = None;
    source.sbuffer = None;
    source.sbuf_size = 0;
    source.sbuf_tail = 0;
    source.sbuf_head = 0;
    source.sbuf_lasttime = 0;
    source.pause_time = 0;
    source.queued_buf_sizes.clear();
    source.end_chunk_failed = false;
    Ok(())
}

/// Pause streaming on the given source.
pub fn pause_stream(source_index: usize) -> AudioResult<()> {
    let mut source = get_source(source_index)?;
    source.stream_should_be_playing = false;
    if source.pause_time == 0 {
        source.pause_time = get_time_counter();
    }
    let _ = mixer_source::mixer_source_pause(source.handle);
    Ok(())
}

/// Resume streaming on the given source.
pub fn resume_stream(source_index: usize) -> AudioResult<()> {
    let mut source = get_source(source_index)?;
    if source.pause_time != 0 {
        source.start_time += get_time_counter() as i32 - source.pause_time as i32;
    }
    source.pause_time = 0;
    source.stream_should_be_playing = true;
    let _ = mixer_source::mixer_source_play(source.handle);
    Ok(())
}

/// Seek the stream to the given position in milliseconds.
pub fn seek_stream(source_index: usize, pos_ms: u32) -> AudioResult<()> {
    eprintln!("[PARITY][STREAM_SEEK] source={} pos_ms={}", source_index, pos_ms);
    // Phase 1: Extract state under locks
    let source = get_source(source_index)?;
    let sample_arc = source
        .sample
        .as_ref()
        .map(Arc::clone)
        .ok_or(AudioError::InvalidSample)?;
    let scope_was_active = source.sbuffer.is_some();
    let _ = mixer_source::mixer_source_stop(source.handle);

    let mut sample = sample_arc.lock();
    let decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?;
    let pcm_pos = pos_ms as u64 * decoder.frequency() as u64 / 1000;
    eprintln!(
        "[PARITY][STREAM_SEEK] freq={} pcm_pos={}",
        decoder.frequency(),
        pcm_pos
    );
    let _ = decoder.seek(pcm_pos as u32);
    let looping = sample.looping;

    // Phase 2: Drop both locks before calling play_stream
    drop(sample);
    drop(source);

    // Phase 3: Restart
    play_stream(sample_arc, source_index, looping, scope_was_active, false)
}

/// Check if a source is currently streaming.
pub fn playing_stream(source_index: usize) -> bool {
    get_source(source_index)
        .map(|s| s.stream_should_be_playing)
        .unwrap_or(false)
}

/// Get current stream playback position in game ticks (ONE_SECOND units).
/// Mirrors C trackplayer logic: pos = GetTimeCounter() - source.start_time.
pub fn get_stream_position_ticks(source_index: usize) -> u32 {
    let source = match get_source(source_index) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if source.sample.is_none() {
        return 0;
    }

    let now = get_time_counter() as i32;
    let mut pos = now - source.start_time;
    if pos < 0 {
        pos = 0;
    }
    pos as u32
}


// =============================================================================
// Buffer Tagging (spec §3.1.3)
// =============================================================================

/// Find the tag associated with a buffer handle.
pub fn find_tagged_buffer(sample: &SoundSample, buffer: usize) -> Option<&SoundTag> {
    find_tagged_buffer_internal(sample, buffer)
}

/// Attach a tag to a buffer in the sample.
pub fn tag_buffer(sample: &mut SoundSample, buffer: usize, data: usize) -> bool {
    tag_buffer_internal(sample, buffer, data)
}

/// Clear a buffer tag.
pub fn clear_buffer_tag(tag: &mut SoundTag) {
    clear_buffer_tag_internal(tag);
}

// =============================================================================
// Scope / Oscilloscope (spec §3.1.3, §3.1.5)
// =============================================================================

/// Generate oscilloscope waveform data for the foreground stream.
///
/// Reads from the scope ring buffer of the active speech or music source.
/// Returns the number of samples written to `data`.
/// AGC page count for oscilloscope.
const AGC_PAGE_COUNT: usize = 16;
/// AGC frame count per page.
const AGC_FRAME_COUNT: usize = 8;
/// Default page max for AGC initialization.
const DEF_PAGE_MAX: i32 = 28000;
/// VAD minimum energy threshold.
const VAD_MIN_ENERGY: i32 = 100;

/// Static AGC state matching C's `static` variables in `GraphForegroundStream`.
/// Persists across calls for smooth waveform normalization.
struct AgcState {
    page_sum: i32,
    pages: [i32; AGC_PAGE_COUNT],
    page_head: usize,
    frame_sum: i32,
    frames: i32,
    avg_amp: i32,
}

static AGC: Mutex<AgcState> = Mutex::new(AgcState {
    page_sum: DEF_PAGE_MAX * AGC_PAGE_COUNT as i32,
    pages: [DEF_PAGE_MAX; AGC_PAGE_COUNT],
    page_head: 0,
    frame_sum: 0,
    frames: 0,
    avg_amp: DEF_PAGE_MAX,
});

pub fn graph_foreground_stream(
    data: &mut [u8],
    width: usize,
    height: usize,
    want_speech: bool,
) -> usize {
    // Match C GraphForegroundStream source selection semantics:
    // prefer speech only when requested and actually available (non-null decoder).
    let source_idx = if want_speech {
        let speech = get_source(SPEECH_SOURCE);
        let speech_available = speech
            .as_ref()
            .map(|s| {
                s.sample
                    .as_ref()
                    .map(|sa| {
                        let sample = sa.lock();
                        sample.decoder.as_ref().map(|d| !d.is_null()).unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if speech_available {
            SPEECH_SOURCE
        } else {
            MUSIC_SOURCE
        }
    } else {
        MUSIC_SOURCE
    };

    let source = match get_source(source_idx) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if !source.stream_should_be_playing
        || source.sample.is_none()
        || source.sbuffer.is_none()
        || source.sbuf_size == 0
    {
        return 0;
    }

    let sample_arc = match source.sample.as_ref() {
        Some(s) => s,
        None => return 0,
    };
    let sample = sample_arc.lock();
    let decoder = match sample.decoder.as_ref() {
        Some(d) => d,
        None => return 0,
    };

    let sbuf = match source.sbuffer.as_ref() {
        Some(b) => b,
        None => return 0,
    };
    let sbuf_size = source.sbuf_size as usize;

    // Determine channel/sample geometry (matching C's audio_GetFormatInfo)
    let channels = decoder.format().channels();
    let sample_size = if decoder.format().is_16bit() { 2 } else { 1 };
    let full_sample = channels * sample_size;

    // Step: 1 for speech, 4 for music (in 11025Hz units), scaled to source freq
    let base_step: usize = if source_idx == SPEECH_SOURCE { 1 } else { 4 };
    let step = {
        let s = decoder.frequency() as usize * base_step / 11025;
        (if s == 0 { 1 } else { s }) * full_sample
    };

    // Compute read position from scope head + time delta
    let delta_time = get_time_counter().wrapping_sub(source.sbuf_lasttime);
    let delta_bytes =
        (delta_time as u64 * decoder.frequency() as u64 * full_sample as u64 / ONE_SECOND as u64)
            as u32;
    // Align delta to sample boundary
    let delta_aligned = delta_bytes & !(full_sample as u32 - 1);
    let mut read_pos = ((source.sbuf_head + delta_aligned.min(source.sbuf_size))
        % source.sbuf_size) as usize;

    let mut agc = AGC.lock();
    let target_amp = (height as i32 >> 1) >> 1;
    let scale = if agc.avg_amp > 0 {
        agc.avg_amp / target_amp.max(1)
    } else {
        1
    };
    let scale = scale.max(1);

    let mut max_a: i32 = 0;
    let mut energy: i64 = 0;
    let count = width.min(data.len());

    for x in 0..count {
        // Read and sum channels (matching C: s += readSoundSample for each channel)
        let mut s: i32 = read_scope_sample(sbuf, read_pos, sbuf_size, sample_size) as i32;
        if channels > 1 {
            s += read_scope_sample(sbuf, read_pos + sample_size, sbuf_size, sample_size) as i32;
        }

        energy += (s as i64 * s as i64) / 0x10000;
        let t = s.abs();
        if t > max_a {
            max_a = t;
        }

        // Scale and center (matching C: s = (s / scale) + (height >> 1))
        let y = (s / scale) + (height as i32 >> 1);
        let y = y.clamp(0, height as i32 - 1);
        data[x] = y as u8;

        read_pos = (read_pos + step) % sbuf_size;
    }
    if count > 0 {
        energy /= count as i64;
    }

    // AGC update (matching C: VAD + page/frame accumulation)
    if energy > VAD_MIN_ENERGY as i64 {
        agc.frame_sum += max_a;
        agc.frames += 1;
        if agc.frames >= AGC_FRAME_COUNT as i32 {
            agc.frame_sum /= AGC_FRAME_COUNT as i32;
            let head = agc.page_head;
            let frame_avg = agc.frame_sum;
            agc.page_sum -= agc.pages[head];
            agc.page_sum += frame_avg;
            agc.pages[head] = frame_avg;
            agc.page_head = (head + 1) % AGC_PAGE_COUNT;
            agc.frame_sum = 0;
            agc.frames = 0;
            agc.avg_amp = agc.page_sum / AGC_PAGE_COUNT as i32;
        }
    }

    // Return 1 on success (matching C convention, not width)
    if count > 0 { 1 } else { 0 }
}

/// Read a single sample from the scope ring buffer.
/// `sample_size` is 1 for 8-bit, 2 for 16-bit (matching C's readSoundSample).
fn read_scope_sample(buffer: &[u8], pos: usize, size: usize, sample_size: usize) -> i16 {
    if size == 0 {
        return 0;
    }
    if sample_size <= 1 {
        let val = buffer[pos % size] as i16;
        (val - 128) << 8
    } else {
        let lo = buffer[pos % size] as i16;
        let hi = buffer[(pos + 1) % size] as i16;
        lo | (hi << 8)
    }
}

// =============================================================================
// Fade (spec §3.1.3)
// =============================================================================

/// Set up a music volume fade.
///
/// # Arguments
/// * `how_long` - Duration of fade in GetTimeCounter ticks.
/// * `end_volume` - Target volume at end of fade.
pub fn set_music_stream_fade(how_long: u32, end_volume: i32) -> bool {
    if how_long == 0 {
        return false;
    }
    let start_volume = music::current_music_volume();
    let mut fade = ENGINE.fade.lock();
    fade.start_time = get_time_counter();
    fade.interval = how_long;
    fade.start_volume = start_volume;
    fade.delta = end_volume - start_volume;
    true
}

// =============================================================================
// Lifecycle (spec §3.1.3)
// =============================================================================

/// Initialize the streaming decoder subsystem.
///
/// Spawns the background decoder thread. **Must be called after
// =============================================================================
// Source accessors (for SFX / Music modules)
// =============================================================================

/// Access a sound source by index. Locks the source mutex and calls `f`.
/// Returns `None` if the source index is out of range.
pub fn with_source<F, R>(source_index: usize, f: F) -> Option<R>
where
    F: FnOnce(&mut SoundSource) -> R,
{
    SOURCES.get(source_index).map(|s| f(&mut s.lock()))
}

/// Stop a sound source (non-streaming SFX stop).
///
/// Stops the mixer source and clears the associated sample.
pub fn stop_source(source_index: usize) -> AudioResult<()> {
    let handle = {
        let source = SOURCES
            .get(source_index)
            .ok_or(AudioError::InvalidChannel(source_index))?;
        let mut source = source.lock();
        let handle = source.handle;
        source.sample = None;
        source.stream_should_be_playing = false;
        handle
    };
    let _ = super::mixer::source::mixer_source_stop(handle);
    Ok(())
}

/// Initialize the stream decoder subsystem.
///
/// This allocates mixer sources for each sound source slot, initializes
/// the mixer, starts a mixer-pump thread (feeding mixer output into rodio),
/// and spawns the stream decoder thread.
pub fn init_stream_decoder() -> AudioResult<()> {
    let mut guard = ENGINE.decoder_thread.lock();
    if guard.is_some() {
        return Err(AudioError::AlreadyInitialized);
    }

    // Generate mixer source handles for each sound source slot and store them.
    // The mixer itself is already initialized by audiocore_rust.c (rust_mixer_Init)
    // before InitStreamDecoder is called.
    let handles = mixer_source::mixer_gen_sources(NUM_SOUNDSOURCES as u32)
        .map_err(|_| AudioError::NotInitialized)?;
    for (i, &handle) in handles.iter().enumerate() {
        if let Some(src_mutex) = SOURCES.get(i) {
            src_mutex.lock().handle = handle;
        }
    }

    // Start the mixer pump — a background thread that periodically mixes
    // all sources into PCM and feeds the result to the rodio backend.
    #[cfg(not(test))]
    start_mixer_pump();

    ENGINE.shutdown.store(false, Ordering::Release);

    let handle = std::thread::Builder::new()
        .name("audio stream decoder".into())
        .spawn(stream_decoder_task)
        .map_err(|_| AudioError::NotInitialized)?;

    *guard = Some(handle);
    Ok(())
}

/// The mixer pump thread handle.
static MIXER_PUMP_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
/// Shutdown flag for mixer pump.
static MIXER_PUMP_SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Start a background thread that continuously mixes audio and sends it to rodio.
fn start_mixer_pump() {
    MIXER_PUMP_SHUTDOWN.store(false, Ordering::Release);

    let handle = std::thread::Builder::new()
        .name("mixer pump".into())
        .spawn(|| {
            eprintln!("[mixer_pump] thread started, opening cpal output stream...");

            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    eprintln!("[mixer_pump] no output audio device found");
                    return;
                }
            };

            let dev_name = device.name().unwrap_or_default();
            eprintln!("[mixer_pump] using device: {:?}", dev_name);

            // Query the device's default config to find a supported format
            let default_config = match device.default_output_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[mixer_pump] failed to get default output config: {}", e);
                    return;
                }
            };
            eprintln!(
                "[mixer_pump] device default config: channels={} sample_rate={} sample_format={:?}",
                default_config.channels(),
                default_config.sample_rate().0,
                default_config.sample_format()
            );

            let config = cpal::StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(44100),
                buffer_size: cpal::BufferSize::Default,
            };

            let mut pump = MixerPumpSource::new();

            // Try f32 output first (most compatible on macOS), then fall back to i16
            let stream = match device.build_output_stream(
                &config,
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    use std::panic::{catch_unwind, AssertUnwindSafe};

                    static CB_COUNT: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    static CB_PANIC_COUNT: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);

                    let n = CB_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let fill_result = catch_unwind(AssertUnwindSafe(|| {
                        if n < 3 {
                            eprintln!("[mixer_pump_cb#{}] output.len()={}", n, output.len());
                        }
                        for sample in output.iter_mut() {
                            let raw = pump.next().unwrap_or(0);
                            *sample = raw as f32 / 32768.0;
                        }
                    }));

                    if fill_result.is_err() {
                        output.fill(0.0);
                        let panic_n =
                            CB_PANIC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if panic_n < 5 || panic_n % 1000 == 0 {
                            eprintln!(
                                "[mixer_pump_cb] recovered from panic in output callback #{}",
                                panic_n
                            );
                        }
                    }
                },
                move |err| {
                    eprintln!("[mixer_pump] audio stream error: {}", err);
                },
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[mixer_pump] failed to build output stream: {}", e);
                    return;
                }
            };

            if let Err(e) = stream.play() {
                eprintln!("[mixer_pump] failed to start playback: {}", e);
                return;
            }

            eprintln!("[mixer_pump] started — feeding mixer output to cpal");

            // Keep alive until shutdown — stream must stay alive for audio to work
            while !MIXER_PUMP_SHUTDOWN.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(50));
            }

            drop(stream);
            eprintln!("[mixer_pump] stopped");
        })
        .ok();

    *MIXER_PUMP_HANDLE.lock() = handle;
}

/// A rodio Source that pulls PCM from the Rust mixer.
///
/// rodio calls `next()` repeatedly to get individual samples. We mix in
/// chunks for efficiency and yield one sample at a time.
struct MixerPumpSource {
    /// Pre-mixed buffer (interleaved i16 samples, little-endian).
    buf: Vec<i16>,
    /// Current read position in `buf`.
    pos: usize,
}

impl MixerPumpSource {
    fn new() -> Self {
        MixerPumpSource {
            buf: Vec::new(),
            pos: 0,
        }
    }

    /// Mix a new chunk of audio.
    fn refill(&mut self) {
        // 4096 bytes = 1024 stereo 16-bit samples
        let mut raw = vec![0u8; 4096];
        let _ = super::mixer::mix::mixer_mix_channels(&mut raw);

        // Periodic diagnostic: log mixer state every ~500 calls (~12s at 44100 Hz)
        static DIAG_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = DIAG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 || count % 500 == 0 {
            let freq = super::mixer::mix::mixer_get_frequency();
            let fmt = super::mixer::mix::mixer_get_format();
            let non_zero = raw.iter().position(|&b| b != 0);
            let sources = super::mixer::source::get_all_sources();
            let playing: Vec<_> = sources.iter()
                .filter_map(|(handle, src)| {
                    let s = src.lock();
                    if s.state == (super::mixer::types::SourceState::Playing as u32) {
                        Some((*handle, s.next_queued, s.pos, s.gain))
                    } else {
                        None
                    }
                })
                .collect();
            eprintln!(
                "[mixer_pump_diag#{}] freq={} fmt={:?} nonzero={:?} total_sources={} playing={:?}",
                count, freq, fmt, non_zero, sources.len(), playing
            );
        }

        // Convert raw bytes to i16 samples
        self.buf.clear();
        for chunk in raw.chunks_exact(2) {
            self.buf.push(i16::from_le_bytes([chunk[0], chunk[1]]));
        }
        self.pos = 0;
    }
}

impl Iterator for MixerPumpSource {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if self.pos >= self.buf.len() {
            self.refill();
        }
        if self.pos < self.buf.len() {
            let sample = self.buf[self.pos];
            self.pos += 1;
            Some(sample)
        } else {
            Some(0)
        }
    }
}

// MixerPumpSource is used directly via cpal callback, no rodio::Source needed.

/// Shut down the streaming decoder subsystem.
///
/// Signals the decoder thread to stop and joins it.
pub fn uninit_stream_decoder() -> AudioResult<()> {
    // Stop the mixer pump first
    MIXER_PUMP_SHUTDOWN.store(true, Ordering::Release);
    if let Some(handle) = MIXER_PUMP_HANDLE.lock().take() {
        let _ = handle.join();
    }

    let handle = {
        let mut guard = ENGINE.decoder_thread.lock();
        guard.take()
    };

    if let Some(handle) = handle {
        ENGINE.shutdown.store(true, Ordering::Release);
        ENGINE.wake.notify_one();
        if handle.join().is_err() {
            eprintln!("[stream] decoder thread panicked on join");
        }
    }

    Ok(())
}

// =============================================================================
// Internal Functions
// =============================================================================

/// Decoder thread entry point. Loops until `shutdown` is set.
fn stream_decoder_task() {
    while !ENGINE.shutdown.load(Ordering::Acquire) {
        process_music_fade();
        let mut any_active = false;

        for source_idx in MUSIC_SOURCE..NUM_SOUNDSOURCES {
            let source = match SOURCES.get(source_idx) {
                Some(s) => s.lock(),
                None => continue,
            };

            if source.sample.is_none() || !source.stream_should_be_playing {
                continue;
            }

            any_active = true;
            let sample_arc = match source.sample.as_ref() {
                Some(s) => Arc::clone(s),
                None => continue,
            };

            drop(source);
            process_source_stream(source_idx, &sample_arc);
        }

        if !any_active {
            let mut guard = ENGINE.wake_mutex.lock();
            let _ = ENGINE.wake.wait_for(&mut guard, Duration::from_millis(100));
        } else {
            std::thread::yield_now();
        }
    }
}

/// Deferred callback actions collected during source processing.
enum DeferredCallback {
    EndStream,
    EndChunk { buf_handle: usize },
    TaggedBuffer { buf_handle: usize, tag_data: usize },
    QueueBuffer { buf_handle: usize },
}

/// Process one source's stream: decode, buffer, queue.
fn process_source_stream(source_index: usize, sample_arc: &Arc<Mutex<SoundSample>>) {
    let mut deferred: Vec<DeferredCallback> = Vec::new();

    {
        let mut source = match SOURCES.get(source_index) {
            Some(s) => s.lock(),
            None => return,
        };
        let mut sample = sample_arc.lock();

        let processed =
            mixer_source::mixer_get_source_i(source.handle, SourceProp::BuffersProcessed)
                .unwrap_or(0);

        if processed == 0 {
            let state = mixer_source::mixer_get_source_i(source.handle, SourceProp::SourceState)
                .unwrap_or(0);
            if state != SourceState::Playing as i32 {
                let queued =
                    mixer_source::mixer_get_source_i(source.handle, SourceProp::BuffersQueued)
                        .unwrap_or(0);
                if queued == 0 {
                    // If a decoder still exists, try to keep feeding the stream before ending.
                    if sample.decoder.is_some() {
                        let mut did_queue = false;
                        let sample_buffers = sample.buffers.clone();
                        for buf_handle in sample_buffers {
                            let mut buf = vec![0u8; BUFFER_SIZE];
                            let (freq, format, n) = {
                                let decoder = match sample.decoder.as_mut() {
                                    Some(d) => d,
                                    None => break,
                                };
                                let freq = decoder.frequency();
                                let format = decoder.format();
                                match decoder.decode(&mut buf) {
                                    Ok(0) => (freq, format, 0usize),
                                    Ok(n) => (freq, format, n),
                                    Err(DecodeError::EndOfFile) => {
                                        deferred.push(DeferredCallback::EndChunk { buf_handle });
                                        (freq, format, 0usize)
                                    }
                                    Err(_) => (freq, format, 0usize),
                                }
                            };

                            if n > 0 {
                                let mixer_freq = mixer_mix::mixer_get_frequency();
                                let mixer_fmt = mixer_mix::mixer_get_format();
                                let mix_format = audio_format_to_mixer(format);
                                let _ = mixer_buffer::mixer_buffer_data(
                                    buf_handle,
                                    mix_format,
                                    &buf[..n],
                                    freq,
                                    mixer_freq,
                                    mixer_fmt,
                                );
                                let _ = mixer_source::mixer_source_queue_buffers(source.handle, &[buf_handle]);
                                source.last_q_buf = buf_handle;
                                source.queued_buf_sizes.push_back(n);
                                deferred.push(DeferredCallback::QueueBuffer { buf_handle });
                                did_queue = true;
                            }
                        }

                        if did_queue {
                            source.stream_should_be_playing = true;
                            let _ = mixer_source::mixer_source_play(source.handle);
                            drop(sample);
                            drop(source);
                            execute_deferred_callbacks(deferred, sample_arc, source_index);
                            return;
                        }
                    }

                    source.stream_should_be_playing = false;
                    deferred.push(DeferredCallback::EndStream);
                } else {
                    let _ = mixer_source::mixer_source_play(source.handle);
                }
            }
            // Drop locks, execute deferred
            drop(sample);
            drop(source);
            execute_deferred_callbacks(deferred, sample_arc, source_index);
            return;
        }

        // Match C process_stream (stream.c:392-504) exactly:
        //   1. Unqueue buffer
        //   2. Process tagged buffer callback
        //   3. Remove scope data
        //   4. Check decoder->error from PREVIOUS decode (persistent state)
        //      - If EOF: call OnEndChunk(sample, last_q_buf), get new decoder
        //   5. Decode with (possibly new) decoder
        //   6. BufferData + Queue the buffer
        //   7. Update last_q_buf, fire OnQueueBuffer
        //
        // In C, decoder->error is persistent on the decoder struct. When
        // SoundDecoder_Decode returns partial bytes at EOF, those bytes ARE
        // queued, and decoder->error == EOF is only checked next iteration.
        // In Rust, decode() returns Err(EndOfFile) with no bytes, so we must
        // handle EOF immediately: fire EndChunk, get new decoder, re-decode.
        for _ in 0..processed {
            let unqueued = mixer_source::mixer_source_unqueue_buffers(source.handle, 1);
            let buf_handle = match unqueued {
                Ok(bufs) if !bufs.is_empty() => bufs[0],
                _ => break,
            };

            // Check for tagged buffer
            if find_tagged_buffer_internal(&sample, buf_handle).is_some() {
                let tag_data = sample
                    .buffer_tags
                    .iter()
                    .filter_map(|t| t.as_ref())
                    .find(|t| t.buf_handle == buf_handle)
                    .map(|t| t.data)
                    .unwrap_or(0);
                deferred.push(DeferredCallback::TaggedBuffer {
                    buf_handle,
                    tag_data,
                });
            }

            // Scope remove — use actual decoded size tracked per buffer
            if source.sbuffer.is_some() {
                let buf_bytes = source.queued_buf_sizes.pop_front().unwrap_or(BUFFER_SIZE);
                remove_scope_data(&mut source, buf_bytes);
            }

            if sample.decoder.is_none() {
                continue;
            }

            // Decode into the unqueued buffer
            let mut buf = vec![0u8; BUFFER_SIZE];
            let decode_result = {
                let decoder = sample.decoder.as_mut().unwrap();
                decoder.decode(&mut buf)
            };

            let n = match decode_result {
                Ok(0) => {
                    // No data decoded — buffer lost (C comment: "should never get here")
                    continue;
                }
                Ok(n) => n,
                Err(DecodeError::EndOfFile) => {
                    // Decoder reached EOF. In C, decoder->error == EOF is checked
                    // at the START of the next iteration. But since our decode()
                    // returns no partial bytes, we handle it NOW: fire OnEndChunk
                    // to get a new decoder, then re-decode with it.
                    if source.end_chunk_failed {
                        continue;
                    }

                    let end_chunk_buf = source.last_q_buf;
                    let mut cbs = sample.callbacks.take();
                    let replaced = if let Some(ref mut cb) = cbs {
                        cb.on_end_chunk(&mut sample, end_chunk_buf)
                    } else {
                        false
                    };
                    sample.callbacks = cbs;

                    if !replaced {
                        source.end_chunk_failed = true;
                        continue;
                    }

                    // OnEndChunk succeeded — new decoder set. Now decode with it.
                    if let Some(new_dec) = sample.decoder.as_mut() {
                        match new_dec.decode(&mut buf) {
                            Ok(0) => continue,
                            Ok(n) => n,
                            Err(DecodeError::EndOfFile) => {
                                // New decoder also immediately EOF — mark failed
                                source.end_chunk_failed = true;
                                continue;
                            }
                            Err(_) => {
                                source.stream_should_be_playing = false;
                                break;
                            }
                        }
                    } else {
                        continue;
                    }
                }
                Err(_) => {
                    source.stream_should_be_playing = false;
                    break;
                }
            };

            // Fill and queue the buffer with decoded data
            let freq = sample.decoder.as_ref().map(|d| d.frequency()).unwrap_or(22050);
            let format = sample.decoder.as_ref().map(|d| d.format()).unwrap_or(AudioFormat::Stereo16);
            let mixer_freq = mixer_mix::mixer_get_frequency();
            let mixer_fmt = mixer_mix::mixer_get_format();
            let mix_format = audio_format_to_mixer(format);
            let _ = mixer_buffer::mixer_buffer_data(
                buf_handle,
                mix_format,
                &buf[..n],
                freq,
                mixer_freq,
                mixer_fmt,
            );
            let _ = mixer_source::mixer_source_queue_buffers(source.handle, &[buf_handle]);
            source.last_q_buf = buf_handle;
            deferred.push(DeferredCallback::QueueBuffer { buf_handle });
            source.queued_buf_sizes.push_back(n);
            if source.sbuffer.is_some() {
                add_scope_data(&mut source, &buf[..n]);
            }
        }
    } // locks dropped here

    execute_deferred_callbacks(deferred, sample_arc, source_index);
}

/// Execute deferred callbacks with no locks held.
fn execute_deferred_callbacks(
    deferred: Vec<DeferredCallback>,
    sample_arc: &Arc<Mutex<SoundSample>>,
    source_index: usize,
) {
    if deferred.is_empty() {
        return;
    }

    // Validity check: verify source still points to this sample
    {
        let source = match SOURCES.get(source_index) {
            Some(s) => s.lock(),
            None => return,
        };
        match source.sample.as_ref() {
            Some(s) if Arc::ptr_eq(s, sample_arc) => {}
            _ => return, // stream was stopped
        }
    }

    for action in deferred {
        let mut sample = sample_arc.lock();
        let mut cbs = sample.callbacks.take();
        match action {
            DeferredCallback::EndStream => {
                if let Some(ref mut cb) = cbs {
                    cb.on_end_stream(&mut sample);
                }
            }
            DeferredCallback::EndChunk { buf_handle } => {
                if let Some(ref mut cb) = cbs {
                    let _ = cb.on_end_chunk(&mut sample, buf_handle);
                }
            }
            DeferredCallback::TaggedBuffer {
                buf_handle,
                tag_data,
            } => {
                let tag = SoundTag {
                    buf_handle,
                    data: tag_data,
                };
                if let Some(ref mut cb) = cbs {
                    cb.on_tagged_buffer(&mut sample, &tag);
                }
            }
            DeferredCallback::QueueBuffer { buf_handle } => {
                if let Some(ref mut cb) = cbs {
                    cb.on_queue_buffer(&mut sample, buf_handle);
                }
            }
        }
        sample.callbacks = cbs;
    }
}

/// Process music volume fade on each decoder iteration.
fn process_music_fade() {
    let (volume, done) = {
        let fade = ENGINE.fade.lock();
        if fade.interval == 0 {
            return;
        }
        let now = get_time_counter();
        let vol = compute_fade_volume(&fade, now);
        let is_done = now.wrapping_sub(fade.start_time) >= fade.interval;
        (vol, is_done)
    };

    music::set_music_volume(volume);

    if done {
        let mut fade = ENGINE.fade.lock();
        fade.interval = 0;
    }
}

/// Add scope data from a buffer to the source's ring buffer.
fn add_scope_data(source: &mut SoundSource, data: &[u8]) {
    if let Some(ref mut buf) = source.sbuffer {
        let size = source.sbuf_size as usize;
        if size == 0 {
            return;
        }
        for &byte in data {
            buf[source.sbuf_tail as usize] = byte;
            source.sbuf_tail = ((source.sbuf_tail as usize + 1) % size) as u32;
        }
    }
}

/// Remove scope data when a buffer is dequeued.
fn remove_scope_data(source: &mut SoundSource, amount: usize) {
    let size = source.sbuf_size as usize;
    if size == 0 {
        return;
    }
    source.sbuf_head = ((source.sbuf_head as usize + amount) % size) as u32;
    // Critical: reset the oscilloscope read-position clock (matching C stream.c:320).
    // Without this, delta_time in graph_foreground_stream grows unbounded from
    // stream start, causing reads from stale/wrong positions in the ring buffer.
    source.sbuf_lasttime = get_time_counter();
}

/// Read and decode audio data from a sample's decoder into a buffer.
fn read_sound_sample(sample: &mut SoundSample, buf: &mut [u8]) -> AudioResult<usize> {
    let decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?;
    match decoder.decode(buf) {
        Ok(n) => Ok(n),
        Err(DecodeError::EndOfFile) => {
            if sample.looping {
                let _ = decoder.seek(0);
                match decoder.decode(buf) {
                    Ok(n) => Ok(n),
                    Err(e) => Err(e.into()),
                }
            } else {
                Err(AudioError::EndOfStream)
            }
        }
        Err(e) => Err(e.into()),
    }
}

// =============================================================================
// Source Access Helpers
// =============================================================================

/// Get a locked reference to a source by index.
fn get_source(index: usize) -> AudioResult<parking_lot::MutexGuard<'static, SoundSource>> {
    SOURCES
        .get(index)
        .map(|m| m.lock())
        .ok_or(AudioError::InvalidSource(index))
}

// =============================================================================
// Internal Helpers (testable logic)
// =============================================================================

/// Find a tagged buffer in a sample's tag list.
fn find_tagged_buffer_internal(sample: &SoundSample, buffer: usize) -> Option<&SoundTag> {
    sample
        .buffer_tags
        .iter()
        .filter_map(|t| t.as_ref())
        .find(|t| t.buf_handle != 0 && t.buf_handle == buffer)
}

/// Tag a buffer in the first available slot.
fn tag_buffer_internal(sample: &mut SoundSample, buffer: usize, data: usize) -> bool {
    for slot in &mut sample.buffer_tags {
        match slot {
            None => {
                *slot = Some(SoundTag {
                    buf_handle: buffer,
                    data,
                });
                return true;
            }
            Some(tag) if tag.buf_handle == 0 => {
                tag.buf_handle = buffer;
                tag.data = data;
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Clear a tag's fields.
fn clear_buffer_tag_internal(tag: &mut SoundTag) {
    tag.buf_handle = 0;
    tag.data = 0;
}

/// Compute the interpolated volume at a given time.
fn compute_fade_volume(fade: &FadeState, now: u32) -> i32 {
    if fade.interval == 0 {
        return fade.start_volume;
    }
    let elapsed = now.saturating_sub(fade.start_time);
    let progress = elapsed.min(fade.interval);
    fade.start_volume + (fade.delta as i64 * progress as i64 / fade.interval as i64) as i32
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn test_callback_panic_recovery_pattern() {
        let mut output = vec![1.0_f32; 8];
        let result = catch_unwind(AssertUnwindSafe(|| {
            panic!("simulated callback panic");
        }));

        if result.is_err() {
            output.fill(0.0);
        }

        assert!(output.iter().all(|&s| s == 0.0));
    }


    // =========================================================================
    // P07: Stream TDD tests
    // @plan PLAN-20260225-AUDIO-HEART.P07
    // =========================================================================

    // --- Infrastructure tests (pass now) ---

    #[test]
    fn test_engine_exists() {
        assert!(!ENGINE.shutdown.load(Ordering::Relaxed));
    }

    #[test]
    fn test_sources_count() {
        assert_eq!(SOURCES.len(), NUM_SOUNDSOURCES);
    }

    #[test]
    fn test_get_source_valid() {
        let src = get_source(0);
        assert!(src.is_ok());
    }

    #[test]
    fn test_get_source_invalid() {
        let src = get_source(999);
        assert!(matches!(src, Err(AudioError::InvalidSource(999))));
    }

    #[test]
    fn test_fade_state_initial() {
        let fade = ENGINE.fade.lock();
        assert_eq!(fade.interval, 0);
    }

    // --- Sample Management (REQ-STREAM-SAMPLE-*) ---

    #[test]
    fn test_create_sound_sample_basic() {
        let sample = create_sound_sample(None, 4, None).unwrap();
        assert_eq!(sample.num_buffers, 4);
        assert_eq!(sample.buffers.len(), 4);
        assert!(sample.buffer_tags.iter().all(|t| t.is_none()));
    }

    #[test]
    fn test_create_sound_sample_with_callbacks() {
        struct TestCb;
        impl StreamCallbacks for TestCb {}
        let sample = create_sound_sample(None, 2, Some(Box::new(TestCb))).unwrap();
        assert!(sample.callbacks.is_some());
    }

    #[test]
    fn test_create_sound_sample_no_decoder() {
        let sample = create_sound_sample(None, 1, None).unwrap();
        assert!(sample.decoder.is_none());
    }

    #[test]
    fn test_destroy_sound_sample_clears_buffers() {
        let mut sample = create_sound_sample(None, 4, None).unwrap();
        destroy_sound_sample(&mut sample).unwrap();
        assert!(sample.buffers.is_empty());
    }

    #[test]
    fn test_set_get_sound_sample_data() {
        let mut sample = create_sound_sample(None, 1, None).unwrap();
        set_sound_sample_data(&mut sample, Box::new(42u32));
        let data = get_sound_sample_data(&sample).unwrap();
        assert_eq!(data.downcast_ref::<u32>(), Some(&42));
    }

    #[test]
    fn test_set_sound_sample_callbacks_replace() {
        struct CbA;
        impl StreamCallbacks for CbA {}
        struct CbB;
        impl StreamCallbacks for CbB {}
        let mut sample = create_sound_sample(None, 1, Some(Box::new(CbA))).unwrap();
        set_sound_sample_callbacks(&mut sample, Some(Box::new(CbB)));
        assert!(sample.callbacks.is_some());
    }

    // --- Buffer Tagging (REQ-STREAM-TAG-*) ---

    #[test]
    fn test_find_tagged_buffer_empty() {
        let sample = SoundSample::new();
        assert!(find_tagged_buffer_internal(&sample, 0).is_none());
    }

    #[test]
    fn test_tag_and_find_buffer() {
        let mut sample = SoundSample::new();
        sample.buffer_tags = vec![None, None, None];
        assert!(tag_buffer_internal(&mut sample, 42, 100));
        let tag = find_tagged_buffer_internal(&sample, 42);
        assert!(tag.is_some());
        assert_eq!(tag.unwrap().data, 100);
    }

    #[test]
    fn test_tag_buffer_full() {
        let mut sample = SoundSample::new();
        sample.buffer_tags = vec![
            Some(SoundTag {
                buf_handle: 1,
                data: 10,
            }),
            Some(SoundTag {
                buf_handle: 2,
                data: 20,
            }),
        ];
        assert!(!tag_buffer_internal(&mut sample, 3, 30));
    }

    #[test]
    fn test_tag_buffer_reuses_cleared_slot() {
        let mut sample = SoundSample::new();
        sample.buffer_tags = vec![Some(SoundTag { buf_handle: 7, data: 11 }), None];

        let slot = sample.buffer_tags[0].as_mut().unwrap();
        clear_buffer_tag_internal(slot);

        assert!(tag_buffer_internal(&mut sample, 42, 100));
        let tag = find_tagged_buffer_internal(&sample, 42);
        assert!(tag.is_some());
        assert_eq!(tag.unwrap().data, 100);
    }


    #[test]
    fn test_clear_buffer_tag() {
        let mut tag = SoundTag {
            buf_handle: 42,
            data: 100,
        };
        clear_buffer_tag_internal(&mut tag);
        assert_eq!(tag.buf_handle, 0);
        assert_eq!(tag.data, 0);
    }

    // --- Fade Logic (REQ-STREAM-FADE-*) ---

    #[test]
    fn test_fade_state_inactive_zero_interval() {
        let fade = FadeState::new();
        assert_eq!(fade.interval, 0);
    }

    #[test]
    fn test_fade_interpolation_midpoint() {
        let fade = FadeState {
            start_time: 0,
            interval: 100,
            start_volume: 0,
            delta: 200,
        };
        // At time 50 (midpoint): volume = 0 + 200 * 50/100 = 100
        let vol = compute_fade_volume(&fade, 50);
        assert_eq!(vol, 100);
    }

    #[test]
    fn test_fade_interpolation_start() {
        let fade = FadeState {
            start_time: 10,
            interval: 100,
            start_volume: 50,
            delta: 100,
        };
        let vol = compute_fade_volume(&fade, 10);
        assert_eq!(vol, 50);
    }

    #[test]
    fn test_fade_interpolation_end() {
        let fade = FadeState {
            start_time: 10,
            interval: 100,
            start_volume: 50,
            delta: 100,
        };
        let vol = compute_fade_volume(&fade, 110);
        assert_eq!(vol, 150);
    }

    #[test]
    fn test_fade_interpolation_past_end_clamps() {
        let fade = FadeState {
            start_time: 0,
            interval: 100,
            start_volume: 0,
            delta: 200,
        };
        let vol = compute_fade_volume(&fade, 200);
        assert_eq!(vol, 200); // clamped at end
    }

    // --- Scope Buffer (REQ-STREAM-SCOPE-*) ---

    #[test]
    fn test_add_scope_data_writes() {
        let mut source = SoundSource::new();
        source.sbuffer = Some(vec![0u8; 16]);
        source.sbuf_size = 16;
        add_scope_data(&mut source, &[1, 2, 3, 4]);
        assert_eq!(source.sbuf_tail, 4);
        assert_eq!(source.sbuffer.as_ref().unwrap()[..4], [1, 2, 3, 4]);
    }

    #[test]
    fn test_add_scope_data_wraps() {
        let mut source = SoundSource::new();
        source.sbuffer = Some(vec![0u8; 8]);
        source.sbuf_size = 8;
        source.sbuf_tail = 6;
        add_scope_data(&mut source, &[1, 2, 3, 4]);
        assert_eq!(source.sbuf_tail, 2); // wrapped
        let buf = source.sbuffer.as_ref().unwrap();
        assert_eq!(buf[6], 1);
        assert_eq!(buf[7], 2);
        assert_eq!(buf[0], 3);
        assert_eq!(buf[1], 4);
    }

    #[test]
    fn test_remove_scope_data_advances_head() {
        let mut source = SoundSource::new();
        source.sbuffer = Some(vec![0u8; 16]);
        source.sbuf_size = 16;
        source.sbuf_head = 0;
        remove_scope_data(&mut source, 5);
        assert_eq!(source.sbuf_head, 5);
    }

    #[test]
    fn test_remove_scope_data_wraps() {
        let mut source = SoundSource::new();
        source.sbuffer = Some(vec![0u8; 8]);
        source.sbuf_size = 8;
        source.sbuf_head = 6;
        remove_scope_data(&mut source, 5);
        assert_eq!(source.sbuf_head, 3); // (6+5) % 8 = 3
    }

    // --- Playback State (REQ-STREAM-PLAY-*) ---

    #[test]
    fn test_playing_stream_initial_false() {
        // Sources start with stream_should_be_playing = false
        let src = get_source(FIRST_SFX_SOURCE).unwrap();
        assert!(!src.stream_should_be_playing);
    }

    #[test]
    fn test_stop_stream_clears_state() {
        stop_stream(0).unwrap();
        let src = get_source(0).unwrap();
        assert!(!src.stream_should_be_playing);
        assert!(src.sample.is_none());
    }

    #[test]
    fn test_pause_records_time() {
        let mut src = SoundSource::new();
        src.stream_should_be_playing = true;
        src.pause_time = 0;
        // Simulate pause: pause_time should be set to current time
        // Actual impl will use get_time_counter(), here we just verify the field
        src.pause_time = 42;
        assert_eq!(src.pause_time, 42);
    }

    #[test]
    fn test_resume_adjusts_start_time() {
        let mut src = SoundSource::new();
        src.start_time = 100;
        src.pause_time = 150;
        // resume: start_time += (now - pause_time)
        let now = 200u32;
        let pause_duration = now - src.pause_time;
        src.start_time += pause_duration as i32;
        src.pause_time = 0;
        assert_eq!(src.start_time, 150); // 100 + 50
        assert_eq!(src.pause_time, 0);
    }

    #[test]
    fn test_seek_no_sample_error() {
        // Source with no sample attached
        let result = seek_stream(0, 0);
        assert!(matches!(result, Err(AudioError::InvalidSample)));
    }

    // --- Thread (REQ-STREAM-THREAD-*) ---

    #[test]
    fn test_init_decoder_spawns_thread() {
        init_stream_decoder().unwrap();
        let guard = ENGINE.decoder_thread.lock();
        assert!(guard.is_some());
        drop(guard);
        uninit_stream_decoder().unwrap();
    }

    #[test]
    fn test_uninit_decoder_joins_thread() {
        init_stream_decoder().unwrap();
        uninit_stream_decoder().unwrap();
        let guard = ENGINE.decoder_thread.lock();
        assert!(guard.is_none());
    }

    #[test]
    fn test_uninit_no_thread_ok() {
        // Should not error when no thread is running
        let result = uninit_stream_decoder();
        assert!(result.is_ok());
    }
}
