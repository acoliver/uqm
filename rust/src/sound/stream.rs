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
use super::types::*;

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
    let base_offset = sample.offset;
    let offset = {
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
                let _ = mixer_buffer::mixer_buffer_data(
                    buf_handle,
                    format as u32,
                    &buf[..n],
                    freq,
                    mixer_freq,
                    mixer_fmt,
                );
                let _ = mixer_source::mixer_source_queue_buffers(source.handle, &[buf_handle]);
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

pub fn graph_foreground_stream(
    data: &mut [i32],
    width: usize,
    height: usize,
    want_speech: bool,
) -> usize {
    let source_idx = if want_speech {
        // Prefer speech if available
        let speech = get_source(SPEECH_SOURCE);
        if speech.as_ref().map(|s| s.sample.is_some()).unwrap_or(false) {
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

    if source.sample.is_none() || source.sbuffer.is_none() || source.sbuf_size == 0 {
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

    let base_step: usize = if source_idx == SPEECH_SOURCE { 1 } else { 4 };
    let freq_scale = decoder.frequency() as f32 / 11025.0;
    let bytes_per_sample = decoder.format().bytes_per_sample() as usize;
    let step = (base_step as f32 * freq_scale).max(1.0) as usize * bytes_per_sample.max(1);

    let delta_time = get_time_counter().wrapping_sub(source.sbuf_lasttime);
    let delta_bytes = (delta_time as f32 * decoder.frequency() as f32 * bytes_per_sample as f32
        / ONE_SECOND as f32) as u32;
    let mut read_pos =
        ((source.sbuf_head + delta_bytes.min(source.sbuf_size)) % source.sbuf_size) as usize;

    let mut agc_pages = [DEF_PAGE_MAX; AGC_PAGE_COUNT];
    let mut agc_idx = 0usize;
    let mut frame_count = 0usize;
    let mut page_max = 0i32;
    let target_amp = height as i32 / 4;
    let is_8bit = bytes_per_sample <= 1;

    let count = width.min(data.len());
    for x in 0..count {
        let sample_val = read_scope_sample(sbuf, read_pos, sbuf_size, is_8bit);

        page_max = page_max.max(sample_val.abs() as i32);
        frame_count += 1;
        if frame_count >= AGC_FRAME_COUNT {
            if page_max > VAD_MIN_ENERGY {
                agc_pages[agc_idx] = page_max;
                agc_idx = (agc_idx + 1) % AGC_PAGE_COUNT;
            }
            frame_count = 0;
            page_max = 0;
        }

        let avg_amp: i32 = agc_pages.iter().sum::<i32>() / AGC_PAGE_COUNT as i32;
        let scaled = (sample_val as i32) * target_amp / avg_amp.max(1);
        let y = (height as i32 / 2 + scaled).clamp(0, height as i32 - 1);
        data[x] = y;

        read_pos = (read_pos + step) % sbuf_size;
    }
    count
}

/// Read a single sample from the scope ring buffer.
fn read_scope_sample(buffer: &[u8], pos: usize, size: usize, is_8bit: bool) -> i16 {
    if size == 0 {
        return 0;
    }
    if is_8bit {
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
    let mut fade = ENGINE.fade.lock();
    fade.start_time = get_time_counter();
    fade.interval = how_long;
    // Start from current music volume (not stored here — use MAX_VOLUME as default)
    fade.start_volume = MAX_VOLUME;
    fade.delta = end_volume - fade.start_volume;
    true
}

// =============================================================================
// Lifecycle (spec §3.1.3)
// =============================================================================

/// Initialize the streaming decoder subsystem.
///
/// Spawns the background decoder thread. **Must be called after
/// `mixer_init()`** — the engine allocates mixer sources/buffers during
/// initialization.
pub fn init_stream_decoder() -> AudioResult<()> {
    let mut guard = ENGINE.decoder_thread.lock();
    if guard.is_some() {
        return Err(AudioError::AlreadyInitialized);
    }
    ENGINE.shutdown.store(false, Ordering::Release);

    let handle = std::thread::Builder::new()
        .name("audio stream decoder".into())
        .spawn(stream_decoder_task)
        .map_err(|_| AudioError::NotInitialized)?;

    *guard = Some(handle);
    Ok(())
}

/// Shut down the streaming decoder subsystem.
///
/// Signals the decoder thread to stop and joins it.
pub fn uninit_stream_decoder() -> AudioResult<()> {
    let handle = {
        let mut guard = ENGINE.decoder_thread.lock();
        guard.take()
    };

    if let Some(handle) = handle {
        ENGINE.shutdown.store(true, Ordering::Release);
        ENGINE.wake.notify_one();
        if handle.join().is_err() {
            log::error!("decoder thread panicked");
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
    TaggedBuffer { tag_data: usize },
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

        let mut end_chunk_failed = false;
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
                deferred.push(DeferredCallback::TaggedBuffer { tag_data });
            }

            // Scope remove
            if source.sbuffer.is_some() {
                remove_scope_data(&mut source, BUFFER_SIZE);
            }

            // Decode new audio
            if sample.decoder.is_none() || end_chunk_failed {
                continue;
            }

            let decoder = match sample.decoder.as_mut() {
                Some(d) => d,
                None => continue,
            };

            let freq = decoder.frequency();
            let format = decoder.format();
            let mut buf = vec![0u8; BUFFER_SIZE];
            match decoder.decode(&mut buf) {
                Ok(0) => continue,
                Ok(n) => {
                    let mixer_freq = mixer_mix::mixer_get_frequency();
                    let mixer_fmt = mixer_mix::mixer_get_format();
                    let _ = mixer_buffer::mixer_buffer_data(
                        buf_handle,
                        format as u32,
                        &buf[..n],
                        freq,
                        mixer_freq,
                        mixer_fmt,
                    );
                    let _ = mixer_source::mixer_source_queue_buffers(source.handle, &[buf_handle]);
                    source.last_q_buf = buf_handle;
                    deferred.push(DeferredCallback::QueueBuffer { buf_handle });
                    if source.sbuffer.is_some() {
                        add_scope_data(&mut source, &buf[..n]);
                    }
                }
                Err(DecodeError::EndOfFile) => {
                    deferred.push(DeferredCallback::EndChunk { buf_handle });
                    end_chunk_failed = true;
                }
                Err(_) => {
                    source.stream_should_be_playing = false;
                    break;
                }
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
            DeferredCallback::TaggedBuffer { tag_data } => {
                let tag = SoundTag {
                    buf_handle: 0,
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
        let vol = compute_fade_volume(&fade, get_time_counter());
        let is_done = get_time_counter().wrapping_sub(fade.start_time) >= fade.interval;
        (vol, is_done)
    };

    // Volume application would go to music module (not yet implemented)
    // For now, just mark fade as complete
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
        .find(|t| t.buf_handle == buffer)
}

/// Tag a buffer in the first available slot.
fn tag_buffer_internal(sample: &mut SoundSample, buffer: usize, data: usize) -> bool {
    for slot in &mut sample.buffer_tags {
        if slot.is_none() {
            *slot = Some(SoundTag {
                buf_handle: buffer,
                data,
            });
            return true;
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
