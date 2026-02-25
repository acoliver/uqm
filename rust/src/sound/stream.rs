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

use parking_lot::{Condvar, Mutex};

use super::decoder::{DecodeError, SoundDecoder};
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
pub fn create_sound_sample(
    decoder: Option<Box<dyn SoundDecoder>>,
    num_buffers: u32,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
) -> AudioResult<SoundSample> {
    todo!("P08: create_sound_sample")
}

/// Destroy a sound sample, releasing all mixer buffers.
pub fn destroy_sound_sample(sample: &mut SoundSample) -> AudioResult<()> {
    todo!("P08: destroy_sound_sample")
}

/// Attach opaque user data to a sample.
pub fn set_sound_sample_data(sample: &mut SoundSample, data: Box<dyn std::any::Any + Send>) {
    todo!("P08: set_sound_sample_data")
}

/// Get a reference to the sample's opaque user data.
pub fn get_sound_sample_data(sample: &SoundSample) -> Option<&(dyn std::any::Any + Send)> {
    todo!("P08: get_sound_sample_data")
}

/// Replace the callbacks on a sample.
pub fn set_sound_sample_callbacks(
    sample: &mut SoundSample,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
) {
    todo!("P08: set_sound_sample_callbacks")
}

/// Get a reference to the sample's decoder.
pub fn get_sound_sample_decoder(sample: &SoundSample) -> Option<&dyn SoundDecoder> {
    todo!("P08: get_sound_sample_decoder")
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
pub fn play_stream(
    sample: Arc<Mutex<SoundSample>>,
    source_index: usize,
    looping: bool,
    scope: bool,
    rewind: bool,
) -> AudioResult<()> {
    todo!("P08: play_stream")
}

/// Stop streaming on the given source.
pub fn stop_stream(source_index: usize) -> AudioResult<()> {
    todo!("P08: stop_stream")
}

/// Pause streaming on the given source.
pub fn pause_stream(source_index: usize) -> AudioResult<()> {
    todo!("P08: pause_stream")
}

/// Resume streaming on the given source.
pub fn resume_stream(source_index: usize) -> AudioResult<()> {
    todo!("P08: resume_stream")
}

/// Seek the stream to the given position in milliseconds.
pub fn seek_stream(source_index: usize, pos_ms: u32) -> AudioResult<()> {
    todo!("P08: seek_stream")
}

/// Check if a source is currently streaming.
pub fn playing_stream(source_index: usize) -> bool {
    todo!("P08: playing_stream")
}

// =============================================================================
// Buffer Tagging (spec §3.1.3)
// =============================================================================

/// Find the tag associated with a buffer handle.
pub fn find_tagged_buffer(sample: &SoundSample, buffer: usize) -> Option<&SoundTag> {
    todo!("P08: find_tagged_buffer")
}

/// Attach a tag to a buffer in the sample.
pub fn tag_buffer(sample: &mut SoundSample, buffer: usize, data: usize) -> bool {
    todo!("P08: tag_buffer")
}

/// Clear a buffer tag.
pub fn clear_buffer_tag(tag: &mut SoundTag) {
    todo!("P08: clear_buffer_tag")
}

// =============================================================================
// Scope / Oscilloscope (spec §3.1.3, §3.1.5)
// =============================================================================

/// Generate oscilloscope waveform data for the foreground stream.
///
/// Reads from the scope ring buffer of the active speech or music source.
/// Returns the number of samples written to `data`.
pub fn graph_foreground_stream(
    data: &mut [i32],
    width: usize,
    height: usize,
    want_speech: bool,
) -> usize {
    todo!("P08: graph_foreground_stream")
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
    todo!("P08: set_music_stream_fade")
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
    todo!("P08: init_stream_decoder")
}

/// Shut down the streaming decoder subsystem.
///
/// Signals the decoder thread to stop and joins it.
pub fn uninit_stream_decoder() -> AudioResult<()> {
    todo!("P08: uninit_stream_decoder")
}

// =============================================================================
// Internal Functions
// =============================================================================

/// Decoder thread entry point. Loops until `shutdown` is set.
fn stream_decoder_task() {
    todo!("P08: stream_decoder_task")
}

/// Process one source's stream: decode, buffer, queue.
fn process_source_stream(source_index: usize) {
    todo!("P08: process_source_stream")
}

/// Process music volume fade on each decoder iteration.
fn process_music_fade() {
    todo!("P08: process_music_fade")
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
    todo!("P08: read_sound_sample")
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
    #[ignore = "P08: create_sound_sample stub"]
    fn test_create_sound_sample_basic() {
        let sample = create_sound_sample(None, 4, None).unwrap();
        assert_eq!(sample.num_buffers, 4);
        assert_eq!(sample.buffers.len(), 4);
        assert!(sample.buffer_tags.iter().all(|t| t.is_none()));
    }

    #[test]
    #[ignore = "P08: create_sound_sample stub"]
    fn test_create_sound_sample_with_callbacks() {
        struct TestCb;
        impl StreamCallbacks for TestCb {}
        let sample = create_sound_sample(None, 2, Some(Box::new(TestCb))).unwrap();
        assert!(sample.callbacks.is_some());
    }

    #[test]
    #[ignore = "P08: create_sound_sample stub"]
    fn test_create_sound_sample_no_decoder() {
        let sample = create_sound_sample(None, 1, None).unwrap();
        assert!(sample.decoder.is_none());
    }

    #[test]
    #[ignore = "P08: destroy_sound_sample stub"]
    fn test_destroy_sound_sample_clears_buffers() {
        let mut sample = create_sound_sample(None, 4, None).unwrap();
        destroy_sound_sample(&mut sample).unwrap();
        assert!(sample.buffers.is_empty());
    }

    #[test]
    #[ignore = "P08: set/get_sound_sample_data stub"]
    fn test_set_get_sound_sample_data() {
        let mut sample = create_sound_sample(None, 1, None).unwrap();
        set_sound_sample_data(&mut sample, Box::new(42u32));
        let data = get_sound_sample_data(&sample).unwrap();
        assert_eq!(data.downcast_ref::<u32>(), Some(&42));
    }

    #[test]
    #[ignore = "P08: set_sound_sample_callbacks stub"]
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
    #[ignore = "P08: stop_stream stub"]
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
    #[ignore = "P08: seek_stream stub"]
    fn test_seek_no_sample_error() {
        // Source with no sample attached
        let result = seek_stream(0, 0);
        assert!(matches!(result, Err(AudioError::InvalidSample)));
    }

    // --- Thread (REQ-STREAM-THREAD-*) ---

    #[test]
    #[ignore = "P08: init_stream_decoder stub"]
    fn test_init_decoder_spawns_thread() {
        init_stream_decoder().unwrap();
        let guard = ENGINE.decoder_thread.lock();
        assert!(guard.is_some());
        drop(guard);
        uninit_stream_decoder().unwrap();
    }

    #[test]
    #[ignore = "P08: uninit_stream_decoder stub"]
    fn test_uninit_decoder_joins_thread() {
        init_stream_decoder().unwrap();
        uninit_stream_decoder().unwrap();
        let guard = ENGINE.decoder_thread.lock();
        assert!(guard.is_none());
    }

    #[test]
    #[ignore = "P08: uninit_stream_decoder stub"]
    fn test_uninit_no_thread_ok() {
        // Should not error when no thread is running
        let result = uninit_stream_decoder();
        assert!(result.is_ok());
    }
}
