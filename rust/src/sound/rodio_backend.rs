//! Rodio-based audio backend for UQM
//!
//! This module implements the audio_Driver interface using rodio.
//! It replaces the mixer.c implementation entirely.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

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
pub const AUDIO_POSITION: i32 = 7;          // audio_POSITION
pub const AUDIO_LOOPING: i32 = 8;           // audio_LOOPING
pub const AUDIO_BUFFER: i32 = 9;            // audio_BUFFER
pub const AUDIO_GAIN: i32 = 10;             // audio_GAIN
pub const AUDIO_SOURCE_STATE: i32 = 11;     // audio_SOURCE_STATE
pub const AUDIO_BUFFERS_QUEUED: i32 = 12;   // audio_BUFFERS_QUEUED
pub const AUDIO_BUFFERS_PROCESSED: i32 = 13; // audio_BUFFERS_PROCESSED

/// Source states (starting at enum value 14)
pub const AUDIO_INITIAL: i32 = 14;          // audio_INITIAL
pub const AUDIO_STOPPED: i32 = 15;          // audio_STOPPED
pub const AUDIO_PLAYING: i32 = 16;          // audio_PLAYING
pub const AUDIO_PAUSED: i32 = 17;           // audio_PAUSED

/// Buffer properties (starting at enum value 18)
pub const AUDIO_FREQUENCY: i32 = 18;        // audio_FREQUENCY
pub const AUDIO_BITS: i32 = 19;             // audio_BITS
pub const AUDIO_CHANNELS: i32 = 20;         // audio_CHANNELS
pub const AUDIO_SIZE: i32 = 21;             // audio_SIZE

/// Buffer formats (starting at enum value 22)
pub const AUDIO_FORMAT_MONO16: u32 = 22;    // audio_FORMAT_MONO16
pub const AUDIO_FORMAT_STEREO16: u32 = 23;  // audio_FORMAT_STEREO16
pub const AUDIO_FORMAT_MONO8: u32 = 24;     // audio_FORMAT_MONO8
pub const AUDIO_FORMAT_STEREO8: u32 = 25;   // audio_FORMAT_STEREO8

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
    position: [f32; 3],
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
            position: [0.0, 0.0, 0.0],
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

    /// Remaining samples queued to sink (estimate)
    fn samples_remaining(&self) -> usize {
        self.total_samples_queued.saturating_sub(self.samples_consumed)
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

fn audio_thread_main(rx: Receiver<AudioCmd>) {
    rust_bridge_log_msg("RODIO_BACKEND: audio thread starting");

    // Initialize audio output
    let (stream, stream_handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RODIO_BACKEND: failed to open audio - {}", e));
            return;
        }
    };

    // Keep stream alive
    let _stream = stream;

    let mut sources: HashMap<AudioObject, SourceState> = HashMap::new();
    let mut buffers: HashMap<AudioObject, BufferData> = HashMap::new();

    rust_bridge_log_msg("RODIO_BACKEND: audio thread ready");

    loop {
        match rx.recv() {
            Ok(cmd) => match cmd {
                AudioCmd::GenSources(n, response) => {
                    let mut ids = Vec::with_capacity(n as usize);
                    for _ in 0..n {
                        let id = NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed);
                        sources.insert(id, SourceState::new());
                        ids.push(id);
                    }
                    let _ = response.send(ids);
                }

                AudioCmd::DeleteSources(ids) => {
                    for id in ids {
                        if let Some(mut src) = sources.remove(&id) {
                            if let Some(sink) = src.sink.take() {
                                sink.stop();
                            }
                        }
                    }
                }

                AudioCmd::SourceSetInt(id, prop, value) => {
                    if let Some(src) = sources.get_mut(&id) {
                        match prop {
                            AUDIO_LOOPING => src.looping = value != 0,
                            AUDIO_BUFFER => {
                                // Queue a single buffer (for non-streaming playback)
                                src.queued_buffers.clear();
                                src.processed_buffers.clear();
                                src.total_samples_queued = 0;
                                if value != 0 {
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
                            }
                            _ => {}
                        }
                    }
                }

                AudioCmd::SourceSetFloat(id, prop, value) => {
                    if let Some(src) = sources.get_mut(&id) {
                        match prop {
                            AUDIO_GAIN => {
                                src.gain = value;
                                if let Some(ref sink) = src.sink {
                                    sink.set_volume(value);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                AudioCmd::SourceGetInt(id, prop, response) => {
                    let value = if let Some(src) = sources.get_mut(&id) {
                        match prop {
                            AUDIO_SOURCE_STATE => {
                                // Check if sink is empty
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
                                        // Update processed buffers based on timing
                                        src.update_processed_buffers();
                                    }
                                } else {
                                    // Update processed buffers based on timing
                                    src.update_processed_buffers();
                                }
                                src.processed_buffers.len() as i32
                            }
                            AUDIO_LOOPING => if src.looping { 1 } else { 0 },
                            _ => 0,
                        }
                    } else {
                        0
                    };
                    let _ = response.send(value as AudioIntVal);
                }

                AudioCmd::SourcePlay(id) => {
                    if let Some(src) = sources.get_mut(&id) {
                        // If we have a paused sink, resume it
                        if let Some(ref sink) = src.sink {
                            if sink.is_paused() {
                                sink.play();
                                src.state = AUDIO_PLAYING;
                                if src.sample_rate != 0 {
                                    src.play_start_time = Some(std::time::Instant::now());
                                }
                                continue;
                            }
                        }

                        // Otherwise, create a new sink and play queued buffers
                        if !src.queued_buffers.is_empty() {
                            if let Ok(sink) = Sink::try_new(&stream_handle) {
                                sink.set_volume(src.gain);

                                // Play all queued buffers
                                for qbuf in &src.queued_buffers {
                                    if let Some(buf) = buffers.get(&qbuf.id) {
                                        // Set sample rate from first buffer
                                        if src.sample_rate == 0 {
                                            src.sample_rate = buf.frequency;
                                        }
                                        let source = SamplesBuffer::new(
                                            buf.channels,
                                            buf.frequency,
                                            buf.samples.clone(),
                                        );
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
                        }
                    }
                }

                AudioCmd::SourcePause(id) => {
                    if let Some(src) = sources.get_mut(&id) {
                        if let Some(ref sink) = src.sink {
                            sink.pause();
                        }
                        src.state = AUDIO_PAUSED;
                        if src.play_start_time.is_some() {
                            // Accumulate played samples up to pause point
                            src.samples_played_before_pause = src.samples_played();
                            src.play_start_time = None;
                        }
                    }
                }

                AudioCmd::SourceStop(id) => {
                    if let Some(src) = sources.get_mut(&id) {
                        if let Some(sink) = src.sink.take() {
                            sink.stop();
                        }
                        // Move queued to processed
                        while let Some(buf) = src.queued_buffers.pop() {
                            src.processed_buffers.push(buf.id);
                        }
                        src.state = AUDIO_STOPPED;
                        src.play_start_time = None;
                        src.samples_played_before_pause = 0;
                        src.total_samples_queued = 0;
                        src.samples_consumed = 0;
                    }
                }

                AudioCmd::SourceRewind(id) => {
                    if let Some(src) = sources.get_mut(&id) {
                        if let Some(sink) = src.sink.take() {
                            sink.stop();
                        }
                        src.state = AUDIO_INITIAL;
                    }
                }

                AudioCmd::SourceQueueBuffers(id, buf_ids) => {
                    if let Some(src) = sources.get_mut(&id) {
                        for buf_id in buf_ids {
                            let samples_in_buf = if let Some(buf) = buffers.get(&buf_id) {
                                // Set sample rate from first buffer
                                if src.sample_rate == 0 {
                                    src.sample_rate = buf.frequency;
                                }
                                
                                // If source is playing, append to sink
                                if src.state == AUDIO_PLAYING {
                                    if let Some(ref sink) = src.sink {
                                        let source = SamplesBuffer::new(
                                            buf.channels,
                                            buf.frequency,
                                            buf.samples.clone(),
                                        );
                                        sink.append(source);
                                    }
                                }
                                // samples per channel
                                buf.samples.len() / buf.channels as usize
                            } else {
                                0
                            };
                            
                            src.total_samples_queued += samples_in_buf;
                            src.queued_buffers.push(QueuedBuffer {
                                id: buf_id,
                                samples: samples_in_buf,
                            });
                        }
                    }
                }

                AudioCmd::SourceUnqueueBuffers(id, n, response) => {
                    let mut unqueued = Vec::new();
                    if let Some(src) = sources.get_mut(&id) {
                        for _ in 0..n {
                            if let Some(buf_id) = src.processed_buffers.pop() {
                                unqueued.push(buf_id);
                            }
                        }
                    }
                    let _ = response.send(unqueued);
                }

                AudioCmd::GenBuffers(n, response) => {
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
                    let _ = response.send(ids);
                }

                AudioCmd::DeleteBuffers(ids) => {
                    for id in ids {
                        buffers.remove(&id);
                    }
                }

                AudioCmd::BufferData(id, format, data, freq) => {
                    if let Some(buf) = buffers.get_mut(&id) {
                        buf.format = format;
                        buf.frequency = freq;
                        buf.size = data.len() as u32;

                        // Determine channels and convert to i16
                        let (channels, bits) = match format {
                            AUDIO_FORMAT_MONO8 => (1u16, 8u16),
                            AUDIO_FORMAT_STEREO8 => (2, 8),
                            AUDIO_FORMAT_MONO16 => (1, 16),
                            AUDIO_FORMAT_STEREO16 => (2, 16),
                            _ => (1, 16), // Default
                        };
                        buf.channels = channels;

                        // Convert to i16 samples
                        buf.samples = if bits == 16 {
                            data.chunks_exact(2)
                                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                                .collect()
                        } else {
                            // 8-bit unsigned to 16-bit signed
                            data.iter().map(|&b| ((b as i16) - 128) * 256).collect()
                        };
                    }
                }

                AudioCmd::BufferGetInt(id, prop, response) => {
                    let value = if let Some(buf) = buffers.get(&id) {
                        match prop {
                            AUDIO_FREQUENCY => buf.frequency as i32,
                            AUDIO_BITS => {
                                if buf.format == AUDIO_FORMAT_MONO8
                                    || buf.format == AUDIO_FORMAT_STEREO8
                                {
                                    8
                                } else {
                                    16
                                }
                            }
                            AUDIO_CHANNELS => buf.channels as i32,
                            AUDIO_SIZE => buf.size as i32,
                            _ => 0,
                        }
                    } else {
                        0
                    };
                    let _ = response.send(value as AudioIntVal);
                }

                AudioCmd::Shutdown => {
                    rust_bridge_log_msg("RODIO_BACKEND: shutting down");
                    // Stop all sources
                    for (_, mut src) in sources.drain() {
                        if let Some(sink) = src.sink.take() {
                            sink.stop();
                        }
                    }
                    break;
                }
            },
            Err(_) => {
                rust_bridge_log_msg("RODIO_BACKEND: channel closed");
                break;
            }
        }
    }

    rust_bridge_log_msg("RODIO_BACKEND: audio thread exited");
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
#[no_mangle]
pub extern "C" fn rust_audio_backend_init(_flags: i32) -> i32 {
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
#[no_mangle]
pub extern "C" fn rust_audio_backend_uninit() {
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

#[no_mangle]
pub extern "C" fn rust_audio_gen_sources(n: u32, out: *mut AudioObject) {
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

#[no_mangle]
pub extern "C" fn rust_audio_delete_sources(n: u32, ids: *const AudioObject) {
    if ids.is_null() {
        return;
    }

    let ids_vec: Vec<AudioObject> =
        unsafe { std::slice::from_raw_parts(ids, n as usize) }.to_vec();
    send_cmd(AudioCmd::DeleteSources(ids_vec));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_i(src: AudioObject, prop: i32, value: AudioIntVal) {
    send_cmd(AudioCmd::SourceSetInt(src, prop, value));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_f(src: AudioObject, prop: i32, value: f32) {
    send_cmd(AudioCmd::SourceSetFloat(src, prop, value));
}

#[no_mangle]
pub extern "C" fn rust_audio_get_source_i(src: AudioObject, prop: i32, out: *mut AudioIntVal) {
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

#[no_mangle]
pub extern "C" fn rust_audio_source_play(src: AudioObject) {
    send_cmd(AudioCmd::SourcePlay(src));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_pause(src: AudioObject) {
    send_cmd(AudioCmd::SourcePause(src));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_stop(src: AudioObject) {
    send_cmd(AudioCmd::SourceStop(src));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_rewind(src: AudioObject) {
    send_cmd(AudioCmd::SourceRewind(src));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_queue_buffers(src: AudioObject, n: u32, bufs: *const AudioObject) {
    if bufs.is_null() {
        return;
    }

    let buf_ids: Vec<AudioObject> =
        unsafe { std::slice::from_raw_parts(bufs, n as usize) }.to_vec();
    send_cmd(AudioCmd::SourceQueueBuffers(src, buf_ids));
}

#[no_mangle]
pub extern "C" fn rust_audio_source_unqueue_buffers(
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

#[no_mangle]
pub extern "C" fn rust_audio_gen_buffers(n: u32, out: *mut AudioObject) {
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

#[no_mangle]
pub extern "C" fn rust_audio_delete_buffers(n: u32, ids: *const AudioObject) {
    if ids.is_null() {
        return;
    }

    let ids_vec: Vec<AudioObject> =
        unsafe { std::slice::from_raw_parts(ids, n as usize) }.to_vec();
    send_cmd(AudioCmd::DeleteBuffers(ids_vec));
}

#[no_mangle]
pub extern "C" fn rust_audio_buffer_data(
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

#[no_mangle]
pub extern "C" fn rust_audio_get_buffer_i(buf: AudioObject, prop: i32, out: *mut AudioIntVal) {
    if out.is_null() {
        return;
    }

    if let Some(value) = send_cmd_wait(|tx| AudioCmd::BufferGetInt(buf, prop, tx)) {
        unsafe {
            *out = value;
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_audio_is_source(src: AudioObject) -> i32 {
    // For now, just return 1 - we don't have a way to query without blocking
    1
}

#[no_mangle]
pub extern "C" fn rust_audio_is_buffer(buf: AudioObject) -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn rust_audio_get_error() -> i32 {
    0 // No error
}

#[no_mangle]
pub extern "C" fn rust_audio_source_fv(src: AudioObject, prop: i32, values: *const f32) {
    // Position is the only fv property we care about
    // For now, ignore positioning (rodio doesn't do spatial audio easily)
}

#[no_mangle]
pub extern "C" fn rust_audio_get_source_f(src: AudioObject, prop: i32, out: *mut f32) {
    if out.is_null() {
        return;
    }
    unsafe {
        *out = 1.0; // Default gain
    }
}
