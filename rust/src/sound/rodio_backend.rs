//! Rodio-based audio backend for UQM
//!
//! This module implements the audio_Driver interface using rodio.
//! It replaces the mixer.c implementation entirely.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink, Source};

use crate::bridge_log::rust_bridge_log_msg;

// =============================================================================
// Types matching audiocore.h enum values
// =============================================================================

/// Audio object handle (source or buffer)
/// This MUST match audiocore.h's `typedef uintptr_t audio_Object`
/// On 64-bit systems (like ARM64 macOS), uintptr_t is 64 bits!
pub type AudioObject = usize;

/// Integer value type for audio properties (matches `audio_IntVal` = intptr_t)
pub type AudioIntVal = isize;

// Enum values from audiocore.h - these MUST match!
// audio_NO_ERROR = 0, audio_INVALID_NAME = 1, ... audio_DRIVER_FAILURE = 6

/// Source properties (starting at enum value 7)
pub const AUDIO_POSITION: i32 = 7; // audio_POSITION
pub const AUDIO_LOOPING: i32 = 8; // audio_LOOPING
pub const AUDIO_BUFFER: i32 = 9; // audio_BUFFER
pub const AUDIO_GAIN: i32 = 10; // audio_GAIN
pub const AUDIO_SOURCE_STATE: i32 = 11; // audio_SOURCE_STATE
pub const AUDIO_BUFFERS_QUEUED: i32 = 12; // audio_BUFFERS_QUEUED
pub const AUDIO_BUFFERS_PROCESSED: i32 = 13; // audio_BUFFERS_PROCESSED

/// Source states (starting at enum value 14)
pub const AUDIO_INITIAL: i32 = 14; // audio_INITIAL
pub const AUDIO_STOPPED: i32 = 15; // audio_STOPPED
pub const AUDIO_PLAYING: i32 = 16; // audio_PLAYING
pub const AUDIO_PAUSED: i32 = 17; // audio_PAUSED

/// Buffer properties (starting at enum value 18)
pub const AUDIO_FREQUENCY: i32 = 18; // audio_FREQUENCY
pub const AUDIO_BITS: i32 = 19; // audio_BITS
pub const AUDIO_CHANNELS: i32 = 20; // audio_CHANNELS
pub const AUDIO_SIZE: i32 = 21; // audio_SIZE

/// Buffer formats (starting at enum value 22)
pub const AUDIO_FORMAT_MONO16: u32 = 22; // audio_FORMAT_MONO16
pub const AUDIO_FORMAT_STEREO16: u32 = 23; // audio_FORMAT_STEREO16
pub const AUDIO_FORMAT_MONO8: u32 = 24; // audio_FORMAT_MONO8
pub const AUDIO_FORMAT_STEREO8: u32 = 25; // audio_FORMAT_STEREO8

// =============================================================================
// Internal Types
// =============================================================================

/// Audio command sent to the audio thread
enum AudioCmd {
    // Source operations
    GenSources(u32, Sender<Vec<AudioObject>>),
    DeleteSources(Vec<AudioObject>),
    SourceSetInt(AudioObject, i32, AudioIntVal),
    SourceSetFloat(AudioObject, i32, f32),
    SourceGetInt(AudioObject, i32, Sender<AudioIntVal>),
    SourcePlay(AudioObject),
    SourcePause(AudioObject),
    SourceStop(AudioObject),
    SourceRewind(AudioObject),
    SourceQueueBuffers(AudioObject, Vec<AudioObject>),
    SourceUnqueueBuffers(AudioObject, u32, Sender<Vec<AudioObject>>),

    // Buffer operations
    GenBuffers(u32, Sender<Vec<AudioObject>>),
    DeleteBuffers(Vec<AudioObject>),
    BufferData(AudioObject, u32, Vec<u8>, u32), // obj, format, data, freq
    BufferGetInt(AudioObject, i32, Sender<AudioIntVal>),

    // Control
    Shutdown,
}

/// Buffer data stored in the audio thread
struct BufferData {
    format: u32,
    frequency: u32,
    samples: Vec<i16>, // Converted to i16 for rodio
    channels: u16,
    size: u32, // Original byte size
}

/// Queued buffer with timing info for tracking when it's been played
struct QueuedBuffer {
    id: AudioObject,
    samples: usize, // Number of samples in this buffer
}

/// Source state
struct SourceState {
    sink: Option<Sink>,
    queued_buffers: Vec<QueuedBuffer>,
    processed_buffers: Vec<AudioObject>,
    gain: f32,
    looping: bool,
    state: i32,
    /// Total samples queued to sink
    total_samples_queued: usize,
    /// Total samples consumed (moved to processed)
    samples_consumed: usize,
    /// Samples per second for this source (set when first buffer is queued)
    sample_rate: u32,
    /// Time when playback started
    play_start_time: Option<std::time::Instant>,
    /// Total samples played before any pause
    samples_played_before_pause: usize,
}

impl SourceState {
    fn new() -> Self {
        Self {
            sink: None,
            queued_buffers: Vec::new(),
            processed_buffers: Vec::new(),
            gain: 1.0,
            looping: false,
            state: AUDIO_INITIAL,
            total_samples_queued: 0,
            samples_consumed: 0,
            sample_rate: 0,
            play_start_time: None,
            samples_played_before_pause: 0,
        }
    }

    /// Estimate how many samples have been played based on elapsed time
    fn samples_played(&self) -> usize {
        if self.sample_rate == 0 {
            return self.samples_played_before_pause;
        }

        if let Some(start) = self.play_start_time {
            let elapsed = start.elapsed();
            let samples_from_time = (elapsed.as_secs_f64() * self.sample_rate as f64) as usize;
            self.samples_played_before_pause + samples_from_time
        } else {
            self.samples_played_before_pause
        }
    }

    /// Move buffers from queued to processed based on estimated playback position
    fn update_processed_buffers(&mut self) {
        let played = self.samples_played();
        let target = played.saturating_sub(self.samples_consumed);
        let mut consumed = 0usize;
        let mut moved = 0;

        while !self.queued_buffers.is_empty() {
            let buf = &self.queued_buffers[0];
            if consumed + buf.samples <= target {
                consumed += buf.samples;
                let buf = self.queued_buffers.remove(0);
                self.processed_buffers.push(buf.id);
                moved += 1;
            } else {
                break;
            }
        }

        if moved > 0 {
            self.samples_consumed += consumed;
            self.total_samples_queued = self.total_samples_queued.saturating_sub(consumed);
        }
    }
}

// =============================================================================
// Global State
// =============================================================================

static AUDIO_SENDER: Mutex<Option<Sender<AudioCmd>>> = Mutex::new(None);
static AUDIO_THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static NEXT_OBJECT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

// =============================================================================
// Audio Thread
// =============================================================================

fn source_int_value(
    sources: &mut HashMap<AudioObject, SourceState>,
    id: AudioObject,
    prop: i32,
) -> AudioIntVal {
    let Some(src) = sources.get_mut(&id) else {
        return 0;
    };
    let value = match prop {
        AUDIO_SOURCE_STATE => {
            if let Some(ref sink) = src.sink {
                if sink.empty() {
                    src.state = AUDIO_STOPPED;
                    src.play_start_time = None;
                    AUDIO_STOPPED
                } else if sink.is_paused() {
                    AUDIO_PAUSED
                } else {
                    AUDIO_PLAYING
                }
            } else {
                src.state
            }
        }
        AUDIO_BUFFERS_QUEUED => src.queued_buffers.len() as i32,
        AUDIO_BUFFERS_PROCESSED => {
            if let Some(ref sink) = src.sink {
                if sink.empty() {
                    while let Some(buf) = src.queued_buffers.pop() {
                        src.processed_buffers.push(buf.id);
                    }
                    src.total_samples_queued = 0;
                    src.samples_consumed = 0;
                    src.play_start_time = None;
                } else {
                    src.update_processed_buffers();
                }
            } else {
                src.update_processed_buffers();
            }
            src.processed_buffers.len() as i32
        }
        AUDIO_LOOPING if src.looping => 1,
        _ => 0,
    };
    value as AudioIntVal
}

fn play_source(
    sources: &mut HashMap<AudioObject, SourceState>,
    buffers: &HashMap<AudioObject, BufferData>,
    stream_handle: &rodio::OutputStreamHandle,
    id: AudioObject,
) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    if let Some(ref sink) = src.sink {
        if sink.is_paused() {
            sink.play();
            src.state = AUDIO_PLAYING;
            if src.sample_rate != 0 {
                src.play_start_time = Some(std::time::Instant::now());
            }
            return;
        }
    }
    if src.queued_buffers.is_empty() {
        return;
    }
    let Ok(sink) = Sink::try_new(stream_handle) else {
        return;
    };
    sink.set_volume(src.gain);
    for qbuf in &src.queued_buffers {
        if let Some(buf) = buffers.get(&qbuf.id) {
            if src.sample_rate == 0 {
                src.sample_rate = buf.frequency;
            }
            let source = SamplesBuffer::new(buf.channels, buf.frequency, buf.samples.clone());
            if src.looping {
                sink.append(source.repeat_infinite());
            } else {
                sink.append(source);
            }
        }
    }
    src.sink = Some(sink);
    src.state = AUDIO_PLAYING;
    if src.sample_rate != 0 {
        src.play_start_time = Some(std::time::Instant::now());
    }
    src.samples_played_before_pause = 0;
    src.samples_consumed = 0;
}

/// Audio device state owned by the audio thread.
struct AudioThreadState {
    sources: HashMap<AudioObject, SourceState>,
    buffers: HashMap<AudioObject, BufferData>,
}

/// Whether the audio thread should keep serving commands.
#[derive(Debug, PartialEq, Eq)]
enum AudioLoop {
    Continue,
    Stop,
}

fn audio_thread_main(rx: Receiver<AudioCmd>) {
    rust_bridge_log_msg("RODIO_BACKEND: audio thread starting");

    let (stream, stream_handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RODIO_BACKEND: failed to open audio - {}", e));
            return;
        }
    };

    // Keep stream alive
    let _stream = stream;

    let mut state = AudioThreadState {
        sources: HashMap::new(),
        buffers: HashMap::new(),
    };

    rust_bridge_log_msg("RODIO_BACKEND: audio thread ready");

    loop {
        let Ok(cmd) = rx.recv() else {
            rust_bridge_log_msg("RODIO_BACKEND: channel closed");
            break;
        };
        let play = |state: &mut AudioThreadState, id: AudioObject| {
            play_source(&mut state.sources, &state.buffers, &stream_handle, id);
        };
        if handle_audio_cmd(&mut state, cmd, play) == AudioLoop::Stop {
            break;
        }
    }

    rust_bridge_log_msg("RODIO_BACKEND: audio thread exited");
}

/// Serve one command, reporting whether the thread should keep running.
///
/// Starting playback needs the output device, so the caller supplies how a
/// source is played.
fn handle_audio_cmd(
    state: &mut AudioThreadState,
    cmd: AudioCmd,
    play: impl FnOnce(&mut AudioThreadState, AudioObject),
) -> AudioLoop {
    match cmd {
        AudioCmd::GenSources(n, response) => {
            let _ = response.send(gen_sources(&mut state.sources, n));
        }
        AudioCmd::DeleteSources(ids) => delete_sources(&mut state.sources, ids),
        AudioCmd::SourceSetInt(id, prop, value) => {
            source_set_int(&mut state.sources, &state.buffers, id, prop, value);
        }
        AudioCmd::SourceSetFloat(id, prop, value) => {
            source_set_float(&mut state.sources, id, prop, value);
        }
        AudioCmd::SourceGetInt(id, prop, response) => {
            let _ = response.send(source_int_value(&mut state.sources, id, prop));
        }
        AudioCmd::SourcePlay(id) => play(state, id),
        AudioCmd::SourcePause(id) => source_pause(&mut state.sources, id),
        AudioCmd::SourceStop(id) => source_stop(&mut state.sources, id),
        AudioCmd::SourceRewind(id) => source_rewind(&mut state.sources, id),
        AudioCmd::SourceQueueBuffers(id, buf_ids) => {
            source_queue_buffers(&mut state.sources, &state.buffers, id, buf_ids);
        }
        AudioCmd::SourceUnqueueBuffers(id, n, response) => {
            let _ = response.send(source_unqueue_buffers(&mut state.sources, id, n));
        }
        AudioCmd::GenBuffers(n, response) => {
            let _ = response.send(gen_buffers(&mut state.buffers, n));
        }
        AudioCmd::DeleteBuffers(ids) => {
            for id in ids {
                state.buffers.remove(&id);
            }
        }
        AudioCmd::BufferData(id, format, data, freq) => {
            fill_buffer(&mut state.buffers, id, format, &data, freq);
        }
        AudioCmd::BufferGetInt(id, prop, response) => {
            let _ = response.send(buffer_int_value(&state.buffers, id, prop));
        }
        AudioCmd::Shutdown => {
            rust_bridge_log_msg("RODIO_BACKEND: shutting down");
            for (_, mut src) in state.sources.drain() {
                if let Some(sink) = src.sink.take() {
                    sink.stop();
                }
            }
            return AudioLoop::Stop;
        }
    }
    AudioLoop::Continue
}

/// Allocate `n` source ids.
fn gen_sources(sources: &mut HashMap<AudioObject, SourceState>, n: u32) -> Vec<AudioObject> {
    let mut ids = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed);
        sources.insert(id, SourceState::new());
        ids.push(id);
    }
    ids
}

/// Stop and forget the given sources.
fn delete_sources(sources: &mut HashMap<AudioObject, SourceState>, ids: Vec<AudioObject>) {
    for id in ids {
        if let Some(mut src) = sources.remove(&id) {
            if let Some(sink) = src.sink.take() {
                sink.stop();
            }
        }
    }
}

/// Apply an integer source property.
fn source_set_int(
    sources: &mut HashMap<AudioObject, SourceState>,
    buffers: &HashMap<AudioObject, BufferData>,
    id: AudioObject,
    prop: i32,
    value: AudioIntVal,
) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    match prop {
        AUDIO_LOOPING => src.looping = value != 0,
        AUDIO_BUFFER => attach_single_buffer(src, buffers, value),
        _ => {}
    }
}

/// Queue exactly one buffer, replacing anything already queued.
///
/// This is the non-streaming playback path.
fn attach_single_buffer(
    src: &mut SourceState,
    buffers: &HashMap<AudioObject, BufferData>,
    value: AudioIntVal,
) {
    src.queued_buffers.clear();
    src.processed_buffers.clear();
    src.total_samples_queued = 0;
    if value == 0 {
        return;
    }

    let buf_id = value as AudioObject;
    let samples = if let Some(buf) = buffers.get(&buf_id) {
        src.sample_rate = buf.frequency;
        buf.samples.len() / buf.channels as usize
    } else {
        0
    };
    src.queued_buffers.push(QueuedBuffer {
        id: buf_id,
        samples,
    });
    src.total_samples_queued = samples;
    src.samples_consumed = 0;
}

/// Apply a float source property.
fn source_set_float(
    sources: &mut HashMap<AudioObject, SourceState>,
    id: AudioObject,
    prop: i32,
    value: f32,
) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    if prop == AUDIO_GAIN {
        src.gain = value;
        if let Some(ref sink) = src.sink {
            sink.set_volume(value);
        }
    }
}

/// Pause a source, banking the samples played so far.
fn source_pause(sources: &mut HashMap<AudioObject, SourceState>, id: AudioObject) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    if let Some(ref sink) = src.sink {
        sink.pause();
    }
    src.state = AUDIO_PAUSED;
    if src.play_start_time.is_some() {
        src.samples_played_before_pause = src.samples_played();
        src.play_start_time = None;
    }
}

/// Stop a source and retire every queued buffer.
fn source_stop(sources: &mut HashMap<AudioObject, SourceState>, id: AudioObject) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    if let Some(sink) = src.sink.take() {
        sink.stop();
    }
    while let Some(buf) = src.queued_buffers.pop() {
        src.processed_buffers.push(buf.id);
    }
    src.state = AUDIO_STOPPED;
    src.play_start_time = None;
    src.samples_played_before_pause = 0;
    src.total_samples_queued = 0;
    src.samples_consumed = 0;
}

/// Return a source to its initial state.
fn source_rewind(sources: &mut HashMap<AudioObject, SourceState>, id: AudioObject) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    if let Some(sink) = src.sink.take() {
        sink.stop();
    }
    src.state = AUDIO_INITIAL;
}

/// Append buffers to a source, feeding the sink directly when already playing.
fn source_queue_buffers(
    sources: &mut HashMap<AudioObject, SourceState>,
    buffers: &HashMap<AudioObject, BufferData>,
    id: AudioObject,
    buf_ids: Vec<AudioObject>,
) {
    let Some(src) = sources.get_mut(&id) else {
        return;
    };
    for buf_id in buf_ids {
        let samples_in_buf = queue_one_buffer(src, buffers, buf_id);
        src.total_samples_queued += samples_in_buf;
        src.queued_buffers.push(QueuedBuffer {
            id: buf_id,
            samples: samples_in_buf,
        });
    }
}

/// Queue a single buffer, returning its length in samples per channel.
fn queue_one_buffer(
    src: &mut SourceState,
    buffers: &HashMap<AudioObject, BufferData>,
    buf_id: AudioObject,
) -> usize {
    let Some(buf) = buffers.get(&buf_id) else {
        return 0;
    };
    if src.sample_rate == 0 {
        src.sample_rate = buf.frequency;
    }
    if src.state == AUDIO_PLAYING {
        if let Some(ref sink) = src.sink {
            sink.append(SamplesBuffer::new(
                buf.channels,
                buf.frequency,
                buf.samples.clone(),
            ));
        }
    }
    buf.samples.len() / buf.channels as usize
}

/// Reclaim up to `n` processed buffers.
fn source_unqueue_buffers(
    sources: &mut HashMap<AudioObject, SourceState>,
    id: AudioObject,
    n: u32,
) -> Vec<AudioObject> {
    let mut unqueued = Vec::new();
    if let Some(src) = sources.get_mut(&id) {
        for _ in 0..n {
            if let Some(buf_id) = src.processed_buffers.pop() {
                unqueued.push(buf_id);
            }
        }
    }
    unqueued
}

/// Allocate `n` empty buffers.
fn gen_buffers(buffers: &mut HashMap<AudioObject, BufferData>, n: u32) -> Vec<AudioObject> {
    let mut ids = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed);
        buffers.insert(
            id,
            BufferData {
                format: 0,
                frequency: 0,
                samples: Vec::new(),
                channels: 1,
                size: 0,
            },
        );
        ids.push(id);
    }
    ids
}

/// Decode PCM bytes into a buffer's i16 samples.
fn fill_buffer(
    buffers: &mut HashMap<AudioObject, BufferData>,
    id: AudioObject,
    format: u32,
    data: &[u8],
    freq: u32,
) {
    let Some(buf) = buffers.get_mut(&id) else {
        return;
    };
    buf.format = format;
    buf.frequency = freq;
    buf.size = data.len() as u32;

    let (channels, bits) = match format {
        AUDIO_FORMAT_MONO8 => (1u16, 8u16),
        AUDIO_FORMAT_STEREO8 => (2, 8),
        AUDIO_FORMAT_MONO16 => (1, 16),
        AUDIO_FORMAT_STEREO16 => (2, 16),
        _ => (1, 16),
    };
    buf.channels = channels;

    buf.samples = if bits == 16 {
        data.as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    } else {
        // 8-bit unsigned to 16-bit signed
        data.iter().map(|&b| ((b as i16) - 128) * 256).collect()
    };
}

/// Read an integer buffer property.
fn buffer_int_value(
    buffers: &HashMap<AudioObject, BufferData>,
    id: AudioObject,
    prop: i32,
) -> AudioIntVal {
    let Some(buf) = buffers.get(&id) else {
        return 0;
    };
    let value = match prop {
        AUDIO_FREQUENCY => buf.frequency as i32,
        AUDIO_BITS => {
            if buf.format == AUDIO_FORMAT_MONO8 || buf.format == AUDIO_FORMAT_STEREO8 {
                8
            } else {
                16
            }
        }
        AUDIO_CHANNELS => buf.channels as i32,
        AUDIO_SIZE => buf.size as i32,
        _ => 0,
    };
    value as AudioIntVal
}

// =============================================================================
// Helper to send commands
// =============================================================================

fn send_cmd(cmd: AudioCmd) -> bool {
    if let Ok(guard) = AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            return sender.send(cmd).is_ok();
        }
    }
    false
}

fn send_cmd_wait<T>(cmd_fn: impl FnOnce(Sender<T>) -> AudioCmd) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    if send_cmd(cmd_fn(tx)) {
        // Use recv_timeout to avoid hanging forever
        rx.recv_timeout(std::time::Duration::from_millis(100)).ok()
    } else {
        None
    }
}

// =============================================================================
// FFI - Initialization
// =============================================================================

/// Initialize the rodio audio backend
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_backend_init(_flags: i32) -> i32 {
    rust_bridge_log_msg("RODIO_BACKEND_INIT");

    // Check if already running
    {
        let guard = match AUDIO_SENDER.lock() {
            Ok(g) => g,
            Err(_) => {
                return 0;
            }
        };
        if guard.is_some() {
            rust_bridge_log_msg("RODIO_BACKEND_INIT: already initialized");
            return 1;
        }
    }

    // Create channel
    let (tx, rx) = mpsc::channel();

    // Store sender
    {
        let mut guard = AUDIO_SENDER.lock().unwrap();
        *guard = Some(tx);
    }

    // Spawn audio thread
    let handle = thread::spawn(move || {
        audio_thread_main(rx);
    });

    // Store thread handle
    {
        let mut guard = AUDIO_THREAD.lock().unwrap();
        *guard = Some(handle);
    }

    // Give thread time to initialize
    std::thread::sleep(std::time::Duration::from_millis(100));

    rust_bridge_log_msg("RODIO_BACKEND_INIT: success");
    1
}

/// Shutdown the rodio audio backend
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_backend_uninit() {
    rust_bridge_log_msg("RODIO_BACKEND_UNINIT");

    send_cmd(AudioCmd::Shutdown);

    {
        let mut guard = AUDIO_SENDER.lock().unwrap();
        *guard = None;
    }

    {
        let mut guard = AUDIO_THREAD.lock().unwrap();
        if let Some(handle) = guard.take() {
            let _ = handle.join();
        }
    }
}

// =============================================================================
// FFI - Sources
// =============================================================================
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_gen_sources(n: u32, out: *mut AudioObject) {
    if out.is_null() {
        return;
    }

    if let Some(ids) = send_cmd_wait(|tx| AudioCmd::GenSources(n, tx)) {
        for (i, id) in ids.into_iter().enumerate() {
            unsafe {
                *out.add(i) = id;
            }
        }
    }
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_delete_sources(n: u32, ids: *const AudioObject) {
    if ids.is_null() {
        return;
    }

    let ids_vec: Vec<AudioObject> = unsafe { std::slice::from_raw_parts(ids, n as usize) }.to_vec();
    send_cmd(AudioCmd::DeleteSources(ids_vec));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_i(src: AudioObject, prop: i32, value: AudioIntVal) {
    send_cmd(AudioCmd::SourceSetInt(src, prop, value));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_f(src: AudioObject, prop: i32, value: f32) {
    send_cmd(AudioCmd::SourceSetFloat(src, prop, value));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_get_source_i(
    src: AudioObject,
    prop: i32,
    out: *mut AudioIntVal,
) {
    if out.is_null() {
        return;
    }

    if let Some(value) = send_cmd_wait(|tx| AudioCmd::SourceGetInt(src, prop, tx)) {
        unsafe {
            *out = value;
        }
    } else {
        // Return 0 on timeout
        unsafe {
            *out = 0;
        }
    }
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_play(src: AudioObject) {
    send_cmd(AudioCmd::SourcePlay(src));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_pause(src: AudioObject) {
    send_cmd(AudioCmd::SourcePause(src));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_stop(src: AudioObject) {
    send_cmd(AudioCmd::SourceStop(src));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_rewind(src: AudioObject) {
    send_cmd(AudioCmd::SourceRewind(src));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_queue_buffers(
    src: AudioObject,
    n: u32,
    bufs: *const AudioObject,
) {
    if bufs.is_null() {
        return;
    }

    let buf_ids: Vec<AudioObject> =
        unsafe { std::slice::from_raw_parts(bufs, n as usize) }.to_vec();
    send_cmd(AudioCmd::SourceQueueBuffers(src, buf_ids));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_unqueue_buffers(
    src: AudioObject,
    n: u32,
    out: *mut AudioObject,
) {
    if out.is_null() {
        return;
    }

    if let Some(ids) = send_cmd_wait(|tx| AudioCmd::SourceUnqueueBuffers(src, n, tx)) {
        for (i, id) in ids.into_iter().enumerate() {
            unsafe {
                *out.add(i) = id;
            }
        }
    }
}

// =============================================================================
// FFI - Buffers
// =============================================================================
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_gen_buffers(n: u32, out: *mut AudioObject) {
    if out.is_null() {
        return;
    }

    if let Some(ids) = send_cmd_wait(|tx| AudioCmd::GenBuffers(n, tx)) {
        for (i, id) in ids.into_iter().enumerate() {
            unsafe {
                *out.add(i) = id;
            }
        }
    }
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_delete_buffers(n: u32, ids: *const AudioObject) {
    if ids.is_null() {
        return;
    }

    let ids_vec: Vec<AudioObject> = unsafe { std::slice::from_raw_parts(ids, n as usize) }.to_vec();
    send_cmd(AudioCmd::DeleteBuffers(ids_vec));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_buffer_data(
    buf: AudioObject,
    format: u32,
    data: *const u8,
    size: u32,
    freq: u32,
) {
    if data.is_null() {
        return;
    }

    if size == 0 {
        return;
    }

    let data_vec = unsafe { std::slice::from_raw_parts(data, size as usize) }.to_vec();
    send_cmd(AudioCmd::BufferData(buf, format, data_vec, freq));
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_get_buffer_i(
    buf: AudioObject,
    prop: i32,
    out: *mut AudioIntVal,
) {
    if out.is_null() {
        return;
    }

    if let Some(value) = send_cmd_wait(|tx| AudioCmd::BufferGetInt(buf, prop, tx)) {
        unsafe {
            *out = value;
        }
    }
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_is_source(_src: AudioObject) -> i32 {
    // For now, just return 1 - we don't have a way to query without blocking
    1
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_is_buffer(_buf: AudioObject) -> i32 {
    1
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_get_error() -> i32 {
    0 // No error
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_source_fv(_src: AudioObject, _prop: i32, _values: *const f32) {
    // Position is the only fv property we care about
    // For now, ignore positioning (rodio doesn't do spatial audio easily)
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_audio_get_source_f(_src: AudioObject, _prop: i32, out: *mut f32) {
    if out.is_null() {
        return;
    }
    unsafe {
        *out = 1.0; // Default gain
    }
}

#[cfg(test)]
mod audio_command_tests {
    use super::*;

    fn buffer_map() -> HashMap<AudioObject, BufferData> {
        let mut buffers = HashMap::new();
        buffers.insert(
            1,
            BufferData {
                format: 0,
                frequency: 0,
                samples: Vec::new(),
                channels: 1,
                size: 0,
            },
        );
        buffers
    }

    #[test]
    fn sixteen_bit_data_is_decoded_little_endian() {
        let mut buffers = buffer_map();

        fill_buffer(&mut buffers, 1, AUDIO_FORMAT_STEREO16, &[1, 0, 0, 1], 22050);

        let buf = &buffers[&1];
        assert_eq!(buf.samples, [1, 256]);
        assert_eq!(buf.channels, 2);
        assert_eq!(buf.frequency, 22050);
        assert_eq!(buf.size, 4);
    }

    #[test]
    fn eight_bit_data_is_recentred_and_scaled() {
        let mut buffers = buffer_map();

        fill_buffer(&mut buffers, 1, AUDIO_FORMAT_MONO8, &[128, 129, 127], 11025);

        // 8-bit is unsigned with silence at 128.
        assert_eq!(buffers[&1].samples, [0, 256, -256]);
        assert_eq!(buffers[&1].channels, 1);
    }

    #[test]
    fn filling_an_unknown_buffer_is_ignored() {
        let mut buffers = buffer_map();

        fill_buffer(&mut buffers, 99, AUDIO_FORMAT_MONO16, &[1, 0], 8000);

        assert!(
            buffers[&1].samples.is_empty(),
            "the real buffer is untouched"
        );
        assert!(!buffers.contains_key(&99), "no buffer is invented");
    }

    #[test]
    fn buffer_properties_report_bit_depth_from_the_format() {
        let mut buffers = buffer_map();
        fill_buffer(&mut buffers, 1, AUDIO_FORMAT_STEREO8, &[128, 128], 8000);

        assert_eq!(buffer_int_value(&buffers, 1, AUDIO_BITS), 8);
        assert_eq!(buffer_int_value(&buffers, 1, AUDIO_CHANNELS), 2);
        assert_eq!(buffer_int_value(&buffers, 1, AUDIO_FREQUENCY), 8000);
        assert_eq!(buffer_int_value(&buffers, 1, AUDIO_SIZE), 2);
        assert_eq!(buffer_int_value(&buffers, 99, AUDIO_BITS), 0);
    }

    #[test]
    fn generated_source_and_buffer_ids_are_unique() {
        let mut sources = HashMap::new();
        let mut buffers = HashMap::new();

        let source_ids = gen_sources(&mut sources, 3);
        let buffer_ids = gen_buffers(&mut buffers, 3);

        let mut all: Vec<AudioObject> = source_ids
            .iter()
            .chain(buffer_ids.iter())
            .copied()
            .collect();
        all.sort_unstable();
        let unique = {
            let mut u = all.clone();
            u.dedup();
            u
        };
        assert_eq!(all, unique, "ids must never collide");
        assert_eq!(sources.len(), 3);
        assert_eq!(buffers.len(), 3);
    }

    #[test]
    fn unqueue_returns_only_what_was_processed() {
        let mut sources = HashMap::new();
        let ids = gen_sources(&mut sources, 1);
        let id = ids[0];
        sources.get_mut(&id).unwrap().processed_buffers = vec![7, 8];

        let taken = source_unqueue_buffers(&mut sources, id, 5);

        assert_eq!(taken.len(), 2, "only processed buffers come back");
        assert!(sources[&id].processed_buffers.is_empty());
        assert!(source_unqueue_buffers(&mut sources, 999, 1).is_empty());
    }
}

#[cfg(test)]
mod source_state_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn buffer(channels: u16, frames: usize, frequency: u32) -> BufferData {
        BufferData {
            format: if channels == 2 {
                AUDIO_FORMAT_STEREO16
            } else {
                AUDIO_FORMAT_MONO16
            },
            frequency,
            samples: vec![0; frames * channels as usize],
            channels,
            size: (frames * channels as usize * 2) as u32,
        }
    }

    fn queued(id: AudioObject, samples: usize) -> QueuedBuffer {
        QueuedBuffer { id, samples }
    }

    fn one_source(state: SourceState) -> (HashMap<AudioObject, SourceState>, AudioObject) {
        let mut sources = HashMap::new();
        sources.insert(77, state);
        (sources, 77)
    }

    #[test]
    fn a_new_source_is_initial_with_unity_gain_and_nothing_queued() {
        let src = SourceState::new();

        assert_eq!(src.state, AUDIO_INITIAL);
        assert_eq!(src.gain, 1.0);
        assert!(!src.looping);
        assert!(src.sink.is_none());
        assert!(src.queued_buffers.is_empty());
        assert!(src.processed_buffers.is_empty());
        assert_eq!(src.samples_played(), 0);
    }

    #[test]
    fn samples_played_without_a_sample_rate_is_only_the_banked_count() {
        let mut src = SourceState::new();
        src.samples_played_before_pause = 7;
        src.play_start_time = Some(Instant::now() - Duration::from_secs(5));

        assert_eq!(src.samples_played(), 7);
    }

    #[test]
    fn samples_played_adds_the_running_clock_to_the_banked_count() {
        let mut src = SourceState::new();
        src.sample_rate = 1000;
        src.samples_played_before_pause = 500;
        src.play_start_time = Some(Instant::now() - Duration::from_secs(2));

        assert!(src.samples_played() >= 2500);
    }

    #[test]
    fn samples_played_ignores_the_clock_while_not_running() {
        let mut src = SourceState::new();
        src.sample_rate = 1000;
        src.samples_played_before_pause = 321;

        assert_eq!(src.samples_played(), 321);
    }

    #[test]
    fn only_fully_played_buffers_move_to_processed() {
        let mut src = SourceState::new();
        src.sample_rate = 1000;
        src.queued_buffers = vec![queued(1, 100), queued(2, 100), queued(3, 100)];
        src.total_samples_queued = 300;
        src.samples_played_before_pause = 250;

        src.update_processed_buffers();

        assert_eq!(src.processed_buffers, [1, 2]);
        let remaining: Vec<AudioObject> = src.queued_buffers.iter().map(|b| b.id).collect();
        assert_eq!(remaining, [3]);
        assert_eq!(src.samples_consumed, 200);
        assert_eq!(src.total_samples_queued, 100);

        // A second look at the same clock must not move anything again.
        src.update_processed_buffers();
        assert_eq!(src.processed_buffers, [1, 2]);
        assert_eq!(src.samples_consumed, 200);
    }

    #[test]
    fn nothing_moves_before_playback_has_covered_the_first_buffer() {
        let mut src = SourceState::new();
        src.sample_rate = 1000;
        src.queued_buffers = vec![queued(1, 100)];
        src.total_samples_queued = 100;
        src.samples_played_before_pause = 99;

        src.update_processed_buffers();

        assert!(src.processed_buffers.is_empty());
        assert_eq!(src.queued_buffers.len(), 1);
        assert_eq!(src.samples_consumed, 0);
        assert_eq!(src.total_samples_queued, 100);
    }

    #[test]
    fn an_unknown_source_reads_as_zero_for_every_property() {
        let mut sources = HashMap::new();

        for prop in [
            AUDIO_SOURCE_STATE,
            AUDIO_BUFFERS_QUEUED,
            AUDIO_BUFFERS_PROCESSED,
        ] {
            assert_eq!(source_int_value(&mut sources, 5, prop), 0);
        }
    }

    #[test]
    fn a_sinkless_source_reports_its_stored_state_and_queue() {
        let mut src = SourceState::new();
        src.queued_buffers = vec![queued(1, 10), queued(2, 10)];
        src.looping = true;
        let (mut sources, id) = one_source(src);

        assert_eq!(
            source_int_value(&mut sources, id, AUDIO_SOURCE_STATE),
            AUDIO_INITIAL as AudioIntVal
        );
        assert_eq!(source_int_value(&mut sources, id, AUDIO_BUFFERS_QUEUED), 2);
        assert_eq!(source_int_value(&mut sources, id, AUDIO_LOOPING), 1);
        assert_eq!(source_int_value(&mut sources, id, AUDIO_GAIN), 0);
    }

    #[test]
    fn a_source_that_is_not_looping_reads_zero_for_looping() {
        let (mut sources, id) = one_source(SourceState::new());

        assert_eq!(source_int_value(&mut sources, id, AUDIO_LOOPING), 0);
    }

    #[test]
    fn a_drained_sink_turns_the_source_stopped() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.state = AUDIO_PLAYING;
        src.play_start_time = Some(Instant::now());
        src.sink = Some(sink);
        let (mut sources, id) = one_source(src);

        let state = source_int_value(&mut sources, id, AUDIO_SOURCE_STATE);

        assert_eq!(state, AUDIO_STOPPED as AudioIntVal);
        assert_eq!(sources[&id].state, AUDIO_STOPPED);
        assert!(sources[&id].play_start_time.is_none());
    }

    #[test]
    fn a_sink_holding_audio_reports_playing_then_paused() {
        let (sink, _output) = Sink::new_idle();
        sink.append(rodio::buffer::SamplesBuffer::new(1, 8000, vec![0i16; 16]));
        let mut src = SourceState::new();
        src.sink = Some(sink);
        let (mut sources, id) = one_source(src);

        assert_eq!(
            source_int_value(&mut sources, id, AUDIO_SOURCE_STATE),
            AUDIO_PLAYING as AudioIntVal
        );

        sources[&id].sink.as_ref().unwrap().pause();
        assert_eq!(
            source_int_value(&mut sources, id, AUDIO_SOURCE_STATE),
            AUDIO_PAUSED as AudioIntVal
        );
    }

    #[test]
    fn a_drained_sink_retires_every_queued_buffer() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        src.queued_buffers = vec![queued(1, 10), queued(2, 10), queued(3, 10)];
        src.total_samples_queued = 30;
        src.samples_consumed = 4;
        let (mut sources, id) = one_source(src);

        let processed = source_int_value(&mut sources, id, AUDIO_BUFFERS_PROCESSED);

        assert_eq!(processed, 3);
        let src = &sources[&id];
        assert!(src.queued_buffers.is_empty());
        assert_eq!(src.processed_buffers, [3, 2, 1], "retired from the back");
        assert_eq!(src.total_samples_queued, 0);
        assert_eq!(src.samples_consumed, 0);
    }

    #[test]
    fn a_live_sink_retires_buffers_by_the_playback_estimate() {
        let (sink, _output) = Sink::new_idle();
        sink.append(rodio::buffer::SamplesBuffer::new(1, 10, vec![0i16; 100]));
        let mut src = SourceState::new();
        src.sink = Some(sink);
        src.sample_rate = 10;
        src.samples_played_before_pause = 15;
        src.queued_buffers = vec![queued(1, 10), queued(2, 10)];
        src.total_samples_queued = 20;
        let (mut sources, id) = one_source(src);

        let processed = source_int_value(&mut sources, id, AUDIO_BUFFERS_PROCESSED);

        assert_eq!(processed, 1);
        assert_eq!(sources[&id].processed_buffers, [1]);
        assert_eq!(sources[&id].queued_buffers.len(), 1);
    }

    #[test]
    fn a_sinkless_source_retires_buffers_by_the_playback_estimate_too() {
        let mut src = SourceState::new();
        src.sample_rate = 10;
        src.samples_played_before_pause = 25;
        src.queued_buffers = vec![queued(1, 10), queued(2, 10), queued(3, 10)];
        src.total_samples_queued = 30;
        let (mut sources, id) = one_source(src);

        assert_eq!(
            source_int_value(&mut sources, id, AUDIO_BUFFERS_PROCESSED),
            2
        );
    }

    #[test]
    fn setting_a_buffer_replaces_the_queue_with_exactly_that_buffer() {
        let mut buffers = HashMap::new();
        buffers.insert(9, buffer(2, 50, 22050));
        let mut src = SourceState::new();
        src.queued_buffers = vec![queued(1, 10), queued(2, 10)];
        src.processed_buffers = vec![4];
        src.total_samples_queued = 20;
        src.samples_consumed = 6;
        let (mut sources, id) = one_source(src);

        source_set_int(&mut sources, &buffers, id, AUDIO_BUFFER, 9);

        let src = &sources[&id];
        let ids: Vec<AudioObject> = src.queued_buffers.iter().map(|b| b.id).collect();
        assert_eq!(ids, [9]);
        assert_eq!(
            src.queued_buffers[0].samples, 50,
            "frames, not interleaved samples"
        );
        assert!(src.processed_buffers.is_empty());
        assert_eq!(src.total_samples_queued, 50);
        assert_eq!(src.samples_consumed, 0);
        assert_eq!(src.sample_rate, 22050);
    }

    #[test]
    fn setting_buffer_zero_detaches_everything() {
        let mut src = SourceState::new();
        src.queued_buffers = vec![queued(1, 10)];
        src.processed_buffers = vec![2];
        src.total_samples_queued = 10;
        let (mut sources, id) = one_source(src);

        source_set_int(&mut sources, &HashMap::new(), id, AUDIO_BUFFER, 0);

        let src = &sources[&id];
        assert!(src.queued_buffers.is_empty());
        assert!(src.processed_buffers.is_empty());
        assert_eq!(src.total_samples_queued, 0);
    }

    #[test]
    fn setting_an_unknown_buffer_queues_a_silent_placeholder() {
        let (mut sources, id) = one_source(SourceState::new());

        source_set_int(&mut sources, &HashMap::new(), id, AUDIO_BUFFER, 404);

        let src = &sources[&id];
        assert_eq!(src.queued_buffers.len(), 1);
        assert_eq!(src.queued_buffers[0].id, 404);
        assert_eq!(src.queued_buffers[0].samples, 0);
        assert_eq!(src.sample_rate, 0, "an unknown buffer has no rate to adopt");
    }

    #[test]
    fn looping_follows_the_integer_value_and_other_properties_are_ignored() {
        let (mut sources, id) = one_source(SourceState::new());

        source_set_int(&mut sources, &HashMap::new(), id, AUDIO_LOOPING, 1);
        assert!(sources[&id].looping);
        source_set_int(&mut sources, &HashMap::new(), id, AUDIO_GAIN, 5);
        assert!(sources[&id].looping, "gain is not an integer property");
        source_set_int(&mut sources, &HashMap::new(), id, AUDIO_LOOPING, 0);
        assert!(!sources[&id].looping);

        // Unknown sources are skipped without being invented.
        source_set_int(&mut sources, &HashMap::new(), 1234, AUDIO_LOOPING, 1);
        assert!(!sources.contains_key(&1234));
    }

    #[test]
    fn gain_is_stored_and_pushed_to_a_live_sink() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        let (mut sources, id) = one_source(src);

        source_set_float(&mut sources, id, AUDIO_GAIN, 0.25);
        assert_eq!(sources[&id].gain, 0.25);
        assert_eq!(sources[&id].sink.as_ref().unwrap().volume(), 0.25);

        source_set_float(&mut sources, id, AUDIO_POSITION, 0.9);
        assert_eq!(sources[&id].gain, 0.25, "only gain is a float property");

        source_set_float(&mut sources, 1234, AUDIO_GAIN, 0.5);
        assert!(!sources.contains_key(&1234));
    }

    #[test]
    fn pausing_banks_the_samples_played_and_stops_the_clock() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        src.sample_rate = 1000;
        src.state = AUDIO_PLAYING;
        src.play_start_time = Some(Instant::now() - Duration::from_secs(1));
        let (mut sources, id) = one_source(src);

        source_pause(&mut sources, id);

        let src = &sources[&id];
        assert_eq!(src.state, AUDIO_PAUSED);
        assert!(src.sink.as_ref().unwrap().is_paused());
        assert!(src.play_start_time.is_none());
        assert!(src.samples_played_before_pause >= 1000);
    }

    #[test]
    fn pausing_a_source_that_never_started_keeps_its_banked_count() {
        let mut src = SourceState::new();
        src.samples_played_before_pause = 42;
        let (mut sources, id) = one_source(src);

        source_pause(&mut sources, id);

        assert_eq!(sources[&id].state, AUDIO_PAUSED);
        assert_eq!(sources[&id].samples_played_before_pause, 42);

        source_pause(&mut sources, 1234);
        assert!(!sources.contains_key(&1234));
    }

    #[test]
    fn stopping_retires_the_queue_and_resets_the_counters() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        src.state = AUDIO_PLAYING;
        src.play_start_time = Some(Instant::now());
        src.samples_played_before_pause = 9;
        src.total_samples_queued = 20;
        src.samples_consumed = 5;
        src.queued_buffers = vec![queued(1, 10), queued(2, 10)];
        let (mut sources, id) = one_source(src);

        source_stop(&mut sources, id);

        let src = &sources[&id];
        assert!(src.sink.is_none());
        assert_eq!(src.state, AUDIO_STOPPED);
        assert!(src.queued_buffers.is_empty());
        assert_eq!(src.processed_buffers, [2, 1]);
        assert!(src.play_start_time.is_none());
        assert_eq!(src.samples_played_before_pause, 0);
        assert_eq!(src.total_samples_queued, 0);
        assert_eq!(src.samples_consumed, 0);

        source_stop(&mut sources, 1234);
        assert!(!sources.contains_key(&1234));
    }

    #[test]
    fn rewinding_drops_the_sink_but_keeps_the_queue() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        src.state = AUDIO_PLAYING;
        src.queued_buffers = vec![queued(1, 10)];
        let (mut sources, id) = one_source(src);

        source_rewind(&mut sources, id);

        let src = &sources[&id];
        assert!(src.sink.is_none());
        assert_eq!(src.state, AUDIO_INITIAL);
        assert_eq!(src.queued_buffers.len(), 1);

        source_rewind(&mut sources, 1234);
        assert!(!sources.contains_key(&1234));
    }

    #[test]
    fn deleting_sources_forgets_them_and_ignores_strangers() {
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        let (mut sources, id) = one_source(src);
        sources.insert(78, SourceState::new());

        delete_sources(&mut sources, vec![id, 4040]);

        assert!(!sources.contains_key(&id));
        assert!(sources.contains_key(&78), "other sources survive");
    }

    #[test]
    fn queued_buffers_accumulate_frames_and_adopt_the_first_rate() {
        let mut buffers = HashMap::new();
        buffers.insert(1, buffer(1, 40, 11025));
        buffers.insert(2, buffer(2, 30, 44100));
        let (mut sources, id) = one_source(SourceState::new());

        source_queue_buffers(&mut sources, &buffers, id, vec![1, 2, 3]);

        let src = &sources[&id];
        let queue: Vec<(AudioObject, usize)> = src
            .queued_buffers
            .iter()
            .map(|b| (b.id, b.samples))
            .collect();
        assert_eq!(queue, [(1, 40), (2, 30), (3, 0)]);
        assert_eq!(src.total_samples_queued, 70);
        assert_eq!(src.sample_rate, 11025, "the first buffer sets the rate");
        assert!(
            src.sink.is_none(),
            "a source that is not playing has no sink"
        );

        source_queue_buffers(&mut sources, &buffers, 1234, vec![1]);
        assert!(!sources.contains_key(&1234));
    }

    #[test]
    fn buffers_queued_on_a_playing_source_are_fed_to_its_sink() {
        let mut buffers = HashMap::new();
        buffers.insert(1, buffer(1, 40, 11025));
        let (sink, _output) = Sink::new_idle();
        let mut src = SourceState::new();
        src.sink = Some(sink);
        src.state = AUDIO_PLAYING;
        let (mut sources, id) = one_source(src);

        source_queue_buffers(&mut sources, &buffers, id, vec![1, 1]);

        assert_eq!(sources[&id].sink.as_ref().unwrap().len(), 2);

        sources.get_mut(&id).unwrap().state = AUDIO_PAUSED;
        source_queue_buffers(&mut sources, &buffers, id, vec![1]);
        assert_eq!(
            sources[&id].sink.as_ref().unwrap().len(),
            2,
            "a paused source only records the queue"
        );
        assert_eq!(sources[&id].queued_buffers.len(), 3);
    }

    #[test]
    fn unqueue_hands_back_the_most_recently_processed_first() {
        let mut src = SourceState::new();
        src.processed_buffers = vec![1, 2, 3];
        let (mut sources, id) = one_source(src);

        assert_eq!(source_unqueue_buffers(&mut sources, id, 2), [3, 2]);
        assert_eq!(sources[&id].processed_buffers, [1]);
    }

    #[test]
    fn new_buffers_start_empty_mono_and_unknown_properties_read_zero() {
        let mut buffers = HashMap::new();

        let ids = gen_buffers(&mut buffers, 2);

        for id in ids {
            assert_eq!(buffer_int_value(&buffers, id, AUDIO_CHANNELS), 1);
            assert_eq!(buffer_int_value(&buffers, id, AUDIO_SIZE), 0);
            assert_eq!(buffer_int_value(&buffers, id, AUDIO_FREQUENCY), 0);
            assert_eq!(buffer_int_value(&buffers, id, AUDIO_GAIN), 0);
        }
    }

    #[test]
    fn an_unrecognised_format_is_treated_as_mono_sixteen_bit() {
        let mut buffers = HashMap::new();
        let ids = gen_buffers(&mut buffers, 1);

        fill_buffer(&mut buffers, ids[0], 9999, &[2, 0, 0, 1, 9], 8000);

        let buf = &buffers[&ids[0]];
        assert_eq!(buf.channels, 1);
        assert_eq!(buf.samples, [2, 256], "a trailing odd byte is dropped");
        assert_eq!(buf.size, 5, "the size is the original byte count");
    }

    #[test]
    fn handle_audio_cmd_answers_requests_through_their_reply_channels() {
        let mut state = AudioThreadState {
            sources: HashMap::new(),
            buffers: HashMap::new(),
        };
        let no_play = |_: &mut AudioThreadState, _: AudioObject| panic!("no play requested");

        let (tx, rx) = mpsc::channel();
        assert_eq!(
            handle_audio_cmd(&mut state, AudioCmd::GenSources(2, tx), no_play),
            AudioLoop::Continue
        );
        let sources = rx.recv().unwrap();
        assert_eq!(sources.len(), 2);

        let (tx, rx) = mpsc::channel();
        handle_audio_cmd(&mut state, AudioCmd::GenBuffers(1, tx), no_play);
        let buffers = rx.recv().unwrap();

        handle_audio_cmd(
            &mut state,
            AudioCmd::BufferData(buffers[0], AUDIO_FORMAT_STEREO16, vec![0; 8], 4000),
            no_play,
        );
        let (tx, rx) = mpsc::channel();
        handle_audio_cmd(
            &mut state,
            AudioCmd::BufferGetInt(buffers[0], AUDIO_SIZE, tx),
            no_play,
        );
        assert_eq!(rx.recv().unwrap(), 8);

        handle_audio_cmd(
            &mut state,
            AudioCmd::SourceSetInt(sources[0], AUDIO_BUFFER, buffers[0] as AudioIntVal),
            no_play,
        );
        handle_audio_cmd(
            &mut state,
            AudioCmd::SourceQueueBuffers(sources[1], vec![buffers[0]]),
            no_play,
        );
        for source in &sources {
            let (tx, rx) = mpsc::channel();
            handle_audio_cmd(
                &mut state,
                AudioCmd::SourceGetInt(*source, AUDIO_BUFFERS_QUEUED, tx),
                no_play,
            );
            assert_eq!(rx.recv().unwrap(), 1);
        }

        handle_audio_cmd(&mut state, AudioCmd::SourceStop(sources[0]), no_play);
        let (tx, rx) = mpsc::channel();
        handle_audio_cmd(
            &mut state,
            AudioCmd::SourceUnqueueBuffers(sources[0], 4, tx),
            no_play,
        );
        assert_eq!(rx.recv().unwrap(), [buffers[0]]);

        handle_audio_cmd(&mut state, AudioCmd::SourcePause(sources[1]), no_play);
        assert_eq!(state.sources[&sources[1]].state, AUDIO_PAUSED);
        handle_audio_cmd(&mut state, AudioCmd::SourceRewind(sources[1]), no_play);
        assert_eq!(state.sources[&sources[1]].state, AUDIO_INITIAL);
        handle_audio_cmd(
            &mut state,
            AudioCmd::SourceSetFloat(sources[1], AUDIO_GAIN, 0.5),
            no_play,
        );
        assert_eq!(state.sources[&sources[1]].gain, 0.5);

        handle_audio_cmd(
            &mut state,
            AudioCmd::DeleteSources(sources.clone()),
            no_play,
        );
        assert!(state.sources.is_empty());
        handle_audio_cmd(
            &mut state,
            AudioCmd::DeleteBuffers(buffers.clone()),
            no_play,
        );
        assert!(state.buffers.is_empty());
    }

    #[test]
    fn handle_audio_cmd_hands_play_requests_to_the_player_exactly_once() {
        let mut state = AudioThreadState {
            sources: HashMap::new(),
            buffers: HashMap::new(),
        };
        let mut played = Vec::new();

        let verdict = handle_audio_cmd(&mut state, AudioCmd::SourcePlay(31), |_, id| {
            played.push(id)
        });

        assert_eq!(verdict, AudioLoop::Continue);
        assert_eq!(played, [31]);
    }

    #[test]
    fn shutdown_stops_the_loop_and_silences_every_sink() {
        let (sink, _output) = Sink::new_idle();
        sink.append(rodio::buffer::SamplesBuffer::new(1, 8000, vec![0i16; 8]));
        let mut src = SourceState::new();
        src.sink = Some(sink);
        let mut state = AudioThreadState {
            sources: HashMap::new(),
            buffers: HashMap::new(),
        };
        state.sources.insert(1, src);

        let verdict = handle_audio_cmd(&mut state, AudioCmd::Shutdown, |_, _| {});

        assert_eq!(verdict, AudioLoop::Stop);
        assert!(state.sources.is_empty(), "every source is drained");
    }
}

/// The C-facing exports talk to the audio thread through one process-wide
/// channel, so these tests share a fake thread that serves the real command
/// handler and run one at a time.
#[cfg(test)]
mod ffi_export_tests {
    use super::*;
    use std::sync::{Arc, MutexGuard};

    static BACKEND_LOCK: Mutex<()> = Mutex::new(());

    struct FakeBackend {
        played: Arc<Mutex<Vec<AudioObject>>>,
        _exclusive: MutexGuard<'static, ()>,
    }

    impl FakeBackend {
        fn start() -> Self {
            let exclusive = BACKEND_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            assert!(
                AUDIO_SENDER.lock().unwrap().is_none(),
                "no other test may leave a backend running"
            );
            let (tx, rx) = mpsc::channel();
            *AUDIO_SENDER.lock().unwrap() = Some(tx);
            let played = Arc::new(Mutex::new(Vec::new()));
            let recorder = Arc::clone(&played);
            let thread = thread::spawn(move || {
                let mut state = AudioThreadState {
                    sources: HashMap::new(),
                    buffers: HashMap::new(),
                };
                while let Ok(cmd) = rx.recv() {
                    let record = |_: &mut AudioThreadState, id: AudioObject| {
                        recorder.lock().unwrap().push(id)
                    };
                    if handle_audio_cmd(&mut state, cmd, record) == AudioLoop::Stop {
                        break;
                    }
                }
            });
            *AUDIO_THREAD.lock().unwrap() = Some(thread);
            Self {
                played,
                _exclusive: exclusive,
            }
        }
    }

    impl Drop for FakeBackend {
        fn drop(&mut self) {
            unsafe { rust_audio_backend_uninit() };
        }
    }

    fn query_source(src: AudioObject, prop: i32) -> AudioIntVal {
        let mut out: AudioIntVal = -1;
        unsafe { rust_audio_get_source_i(src, prop, &mut out) };
        out
    }

    fn query_buffer(buf: AudioObject, prop: i32) -> AudioIntVal {
        let mut out: AudioIntVal = -1;
        unsafe { rust_audio_get_buffer_i(buf, prop, &mut out) };
        out
    }

    fn new_source() -> AudioObject {
        let mut id: AudioObject = 0;
        unsafe { rust_audio_gen_sources(1, &mut id) };
        id
    }

    fn new_buffer(bytes: &[u8], format: u32, freq: u32) -> AudioObject {
        let mut id: AudioObject = 0;
        unsafe {
            rust_audio_gen_buffers(1, &mut id);
            rust_audio_buffer_data(id, format, bytes.as_ptr(), bytes.len() as u32, freq);
        }
        id
    }

    #[test]
    fn exports_return_inert_answers_when_no_backend_is_running() {
        let _exclusive = BACKEND_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert!(AUDIO_SENDER.lock().unwrap().is_none());

        let mut id: AudioObject = 55;
        unsafe { rust_audio_gen_sources(1, &mut id) };
        assert_eq!(id, 55, "no ids are invented without a backend");

        let mut value: AudioIntVal = 99;
        unsafe { rust_audio_get_source_i(1, AUDIO_SOURCE_STATE, &mut value) };
        assert_eq!(value, 0, "a source query that times out reads zero");

        let mut value: AudioIntVal = 99;
        unsafe { rust_audio_get_buffer_i(1, AUDIO_SIZE, &mut value) };
        assert_eq!(value, 99, "a buffer query is left alone without a backend");

        assert!(!send_cmd(AudioCmd::Shutdown));
        assert!(send_cmd_wait::<Vec<AudioObject>>(|tx| AudioCmd::GenBuffers(1, tx)).is_none());
    }

    #[test]
    fn null_pointers_are_refused_without_reaching_the_backend() {
        let _backend = FakeBackend::start();

        unsafe {
            rust_audio_gen_sources(1, std::ptr::null_mut());
            rust_audio_gen_buffers(1, std::ptr::null_mut());
            rust_audio_delete_sources(1, std::ptr::null());
            rust_audio_delete_buffers(1, std::ptr::null());
            rust_audio_source_queue_buffers(1, 1, std::ptr::null());
            rust_audio_source_unqueue_buffers(1, 1, std::ptr::null_mut());
            rust_audio_buffer_data(1, AUDIO_FORMAT_MONO16, std::ptr::null(), 4, 8000);
            rust_audio_get_source_i(1, AUDIO_SOURCE_STATE, std::ptr::null_mut());
            rust_audio_get_buffer_i(1, AUDIO_SIZE, std::ptr::null_mut());
            rust_audio_get_source_f(1, AUDIO_GAIN, std::ptr::null_mut());
        }

        // The backend is still healthy afterwards.
        let src = new_source();
        assert_ne!(src, 0);
    }

    #[test]
    fn a_buffer_round_trips_its_format_through_the_backend() {
        let _backend = FakeBackend::start();

        let buf = new_buffer(&[0, 0, 1, 0, 2, 0, 3, 0], AUDIO_FORMAT_STEREO16, 22050);

        assert_eq!(query_buffer(buf, AUDIO_FREQUENCY), 22050);
        assert_eq!(query_buffer(buf, AUDIO_CHANNELS), 2);
        assert_eq!(query_buffer(buf, AUDIO_BITS), 16);
        assert_eq!(query_buffer(buf, AUDIO_SIZE), 8);
    }

    #[test]
    fn an_empty_buffer_upload_leaves_the_buffer_untouched() {
        let _backend = FakeBackend::start();
        let buf = new_buffer(&[1, 0], AUDIO_FORMAT_MONO16, 8000);
        let bytes = [9u8; 4];

        unsafe { rust_audio_buffer_data(buf, AUDIO_FORMAT_STEREO8, bytes.as_ptr(), 0, 44100) };

        assert_eq!(query_buffer(buf, AUDIO_FREQUENCY), 8000);
        assert_eq!(query_buffer(buf, AUDIO_SIZE), 2);
    }

    #[test]
    fn a_source_walks_through_queue_stop_and_unqueue() {
        let _backend = FakeBackend::start();
        let src = new_source();
        let first = new_buffer(&[0; 8], AUDIO_FORMAT_MONO16, 8000);
        let second = new_buffer(&[0; 8], AUDIO_FORMAT_MONO16, 8000);

        let queue = [first, second];
        unsafe { rust_audio_source_queue_buffers(src, 2, queue.as_ptr()) };
        assert_eq!(query_source(src, AUDIO_BUFFERS_QUEUED), 2);
        assert_eq!(
            query_source(src, AUDIO_SOURCE_STATE),
            AUDIO_INITIAL as AudioIntVal
        );

        unsafe { rust_audio_source_stop(src) };
        assert_eq!(
            query_source(src, AUDIO_SOURCE_STATE),
            AUDIO_STOPPED as AudioIntVal
        );
        assert_eq!(query_source(src, AUDIO_BUFFERS_QUEUED), 0);
        assert_eq!(query_source(src, AUDIO_BUFFERS_PROCESSED), 2);

        let mut taken = [0 as AudioObject; 2];
        unsafe { rust_audio_source_unqueue_buffers(src, 2, taken.as_mut_ptr()) };
        assert_eq!(
            taken,
            [first, second],
            "the oldest retired buffer comes back first"
        );
        assert_eq!(query_source(src, AUDIO_BUFFERS_PROCESSED), 0);
    }

    #[test]
    fn single_buffer_looping_gain_pause_and_rewind_reach_the_source() {
        let _backend = FakeBackend::start();
        let src = new_source();
        let buf = new_buffer(&[0; 16], AUDIO_FORMAT_MONO16, 8000);

        unsafe {
            rust_audio_source_i(src, AUDIO_BUFFER, buf as AudioIntVal);
            rust_audio_source_i(src, AUDIO_LOOPING, 1);
            rust_audio_source_f(src, AUDIO_GAIN, 0.5);
            rust_audio_source_fv(src, AUDIO_POSITION, [0.0f32; 3].as_ptr());
        }
        assert_eq!(query_source(src, AUDIO_BUFFERS_QUEUED), 1);
        assert_eq!(query_source(src, AUDIO_LOOPING), 1);

        unsafe { rust_audio_source_pause(src) };
        assert_eq!(
            query_source(src, AUDIO_SOURCE_STATE),
            AUDIO_PAUSED as AudioIntVal
        );

        unsafe { rust_audio_source_rewind(src) };
        assert_eq!(
            query_source(src, AUDIO_SOURCE_STATE),
            AUDIO_INITIAL as AudioIntVal
        );
        assert_eq!(
            query_source(src, AUDIO_BUFFERS_QUEUED),
            1,
            "rewind keeps the queue"
        );
    }

    #[test]
    fn deleted_sources_and_buffers_read_as_gone() {
        let _backend = FakeBackend::start();
        let src = new_source();
        let buf = new_buffer(&[0; 4], AUDIO_FORMAT_MONO16, 8000);
        unsafe { rust_audio_source_i(src, AUDIO_BUFFER, buf as AudioIntVal) };
        assert_eq!(query_source(src, AUDIO_BUFFERS_QUEUED), 1);

        unsafe {
            rust_audio_delete_sources(1, &src);
            rust_audio_delete_buffers(1, &buf);
        }

        assert_eq!(query_source(src, AUDIO_BUFFERS_QUEUED), 0);
        assert_eq!(query_buffer(buf, AUDIO_SIZE), 0);
    }

    #[test]
    fn play_requests_reach_the_backends_player() {
        let backend = FakeBackend::start();
        let src = new_source();

        unsafe { rust_audio_source_play(src) };
        // A reply makes sure the play command has been served.
        query_source(src, AUDIO_SOURCE_STATE);

        assert_eq!(*backend.played.lock().unwrap(), [src]);
    }

    #[test]
    fn init_reports_success_without_a_second_thread_when_already_running() {
        let _backend = FakeBackend::start();

        assert_eq!(unsafe { rust_audio_backend_init(0) }, 1);

        // The fake thread is still the one serving requests.
        assert_ne!(new_source(), 0);
    }

    #[test]
    fn uninit_stops_the_thread_and_clears_the_channel() {
        let backend = FakeBackend::start();
        assert_ne!(new_source(), 0);

        unsafe { rust_audio_backend_uninit() };

        assert!(AUDIO_SENDER.lock().unwrap().is_none());
        assert!(AUDIO_THREAD.lock().unwrap().is_none());
        assert!(!send_cmd(AudioCmd::Shutdown));
        drop(backend);
    }

    #[test]
    fn fixed_answers_for_object_queries_and_errors() {
        assert_eq!(unsafe { rust_audio_is_source(123) }, 1);
        assert_eq!(unsafe { rust_audio_is_buffer(123) }, 1);
        assert_eq!(unsafe { rust_audio_get_error() }, 0);

        let mut gain = 0.0f32;
        unsafe { rust_audio_get_source_f(1, AUDIO_GAIN, &mut gain) };
        assert_eq!(gain, 1.0);
    }
}
