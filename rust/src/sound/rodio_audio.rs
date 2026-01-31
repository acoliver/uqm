//! Rodio-based audio system for UQM
//!
//! This replaces the hand-rolled OpenAL-style mixer with the rodio library,
//! which handles mixing, resampling, and audio output correctly.
//!
//! The audio runs on a dedicated thread since rodio's OutputStream is not Send.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use rodio::{OutputStream, OutputStreamHandle, Sink, Source};

use crate::bridge_log::rust_bridge_log_msg;

/// Commands sent to the audio thread
enum AudioCommand {
    /// Play WAV data: (data, category, looping) -> handle via response channel
    PlayWav(Vec<u8>, SoundCategory, bool, Sender<u32>),
    /// Play OGG data
    PlayOgg(Vec<u8>, SoundCategory, bool, Sender<u32>),
    /// Play raw PCM: (data, sample_rate, channels, bits, category, looping)
    PlayRaw(Vec<u8>, u32, u16, u16, SoundCategory, bool, Sender<u32>),
    /// Stop a sound by handle
    Stop(u32),
    /// Pause a sound
    Pause(u32),
    /// Resume a sound
    Resume(u32),
    /// Set volume for a sound
    SetVolume(u32, f32),
    /// Check if playing (responds via channel)
    IsPlaying(u32, Sender<bool>),
    /// Stop all sounds
    StopAll,
    /// Set master volume
    SetMasterVolume(f32),
    /// Set music volume
    SetMusicVolume(f32),
    /// Set sfx volume
    SetSfxVolume(f32),
    /// Set speech volume
    SetSpeechVolume(f32),
    /// Cleanup finished sounds
    Cleanup,
    /// Shutdown the audio thread
    Shutdown,
}

/// Global sender to audio thread
static AUDIO_SENDER: Mutex<Option<Sender<AudioCommand>>> = Mutex::new(None);
static AUDIO_THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Handle counter for sources
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

/// Sound category for volume control
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCategory {
    Music = 0,
    Sfx = 1,
    Speech = 2,
}

/// Volume state
struct VolumeState {
    master: f32,
    music: f32,
    sfx: f32,
    speech: f32,
}

impl VolumeState {
    fn new() -> Self {
        Self {
            master: 1.0,
            music: 1.0,
            sfx: 1.0,
            speech: 1.0,
        }
    }

    fn get_volume(&self, category: SoundCategory) -> f32 {
        self.master
            * match category {
                SoundCategory::Music => self.music,
                SoundCategory::Sfx => self.sfx,
                SoundCategory::Speech => self.speech,
            }
    }
}

/// Audio thread main function
fn audio_thread_main(rx: Receiver<AudioCommand>) {
    rust_bridge_log_msg("RUST_AUDIO_THREAD: starting");

    // Initialize audio output on this thread
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_AUDIO_THREAD: failed to open output - {}", e));
            return;
        }
    };

    rust_bridge_log_msg("RUST_AUDIO_THREAD: output stream opened");

    let mut sinks: HashMap<u32, (Sink, SoundCategory)> = HashMap::new();
    let mut volumes = VolumeState::new();

    loop {
        match rx.recv() {
            Ok(cmd) => match cmd {
                AudioCommand::PlayWav(data, category, looping, response) => {
                    let handle = play_decoded(&stream_handle, &data, category, looping, &volumes, &mut sinks, "WAV");
                    let _ = response.send(handle);
                }
                AudioCommand::PlayOgg(data, category, looping, response) => {
                    let handle = play_decoded(&stream_handle, &data, category, looping, &volumes, &mut sinks, "OGG");
                    let _ = response.send(handle);
                }
                AudioCommand::PlayRaw(data, sample_rate, channels, bits, category, looping, response) => {
                    let handle = play_raw(&stream_handle, &data, sample_rate, channels, bits, category, looping, &volumes, &mut sinks);
                    let _ = response.send(handle);
                }
                AudioCommand::Stop(handle) => {
                    if let Some((sink, _)) = sinks.remove(&handle) {
                        sink.stop();
                    }
                }
                AudioCommand::Pause(handle) => {
                    if let Some((sink, _)) = sinks.get(&handle) {
                        sink.pause();
                    }
                }
                AudioCommand::Resume(handle) => {
                    if let Some((sink, _)) = sinks.get(&handle) {
                        sink.play();
                    }
                }
                AudioCommand::SetVolume(handle, volume) => {
                    if let Some((sink, _)) = sinks.get(&handle) {
                        sink.set_volume(volume.clamp(0.0, 1.0));
                    }
                }
                AudioCommand::IsPlaying(handle, response) => {
                    let playing = sinks.get(&handle).is_some_and(|(sink, _)| !sink.empty());
                    let _ = response.send(playing);
                }
                AudioCommand::StopAll => {
                    for (_, (sink, _)) in sinks.drain() {
                        sink.stop();
                    }
                }
                AudioCommand::SetMasterVolume(v) => volumes.master = v.clamp(0.0, 1.0),
                AudioCommand::SetMusicVolume(v) => volumes.music = v.clamp(0.0, 1.0),
                AudioCommand::SetSfxVolume(v) => volumes.sfx = v.clamp(0.0, 1.0),
                AudioCommand::SetSpeechVolume(v) => volumes.speech = v.clamp(0.0, 1.0),
                AudioCommand::Cleanup => {
                    sinks.retain(|_, (sink, _)| !sink.empty());
                }
                AudioCommand::Shutdown => {
                    rust_bridge_log_msg("RUST_AUDIO_THREAD: shutting down");
                    for (_, (sink, _)) in sinks.drain() {
                        sink.stop();
                    }
                    break;
                }
            },
            Err(_) => {
                // Channel closed
                rust_bridge_log_msg("RUST_AUDIO_THREAD: channel closed");
                break;
            }
        }
    }

    rust_bridge_log_msg("RUST_AUDIO_THREAD: exited");
}

fn play_decoded(
    stream_handle: &OutputStreamHandle,
    data: &[u8],
    category: SoundCategory,
    looping: bool,
    volumes: &VolumeState,
    sinks: &mut HashMap<u32, (Sink, SoundCategory)>,
    format: &str,
) -> u32 {
    let cursor = Cursor::new(data.to_vec());

    let source = match rodio::Decoder::new(cursor) {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_AUDIO_PLAY_{}: decode error - {}", format, e));
            return 0;
        }
    };

    let sink = match Sink::try_new(stream_handle) {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_AUDIO_PLAY_{}: sink error - {}", format, e));
            return 0;
        }
    };

    sink.set_volume(volumes.get_volume(category));

    if looping {
        sink.append(source.repeat_infinite());
    } else {
        sink.append(source);
    }

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    sinks.insert(handle, (sink, category));

    rust_bridge_log_msg(&format!(
        "RUST_AUDIO_PLAY_{}: handle={} cat={:?} loop={}",
        format, handle, category, looping
    ));

    handle
}

fn play_raw(
    stream_handle: &OutputStreamHandle,
    data: &[u8],
    sample_rate: u32,
    channels: u16,
    bits: u16,
    category: SoundCategory,
    looping: bool,
    volumes: &VolumeState,
    sinks: &mut HashMap<u32, (Sink, SoundCategory)>,
) -> u32 {
    // Convert bytes to i16 samples
    let samples: Vec<i16> = if bits == 16 {
        data.chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    } else if bits == 8 {
        data.iter().map(|&b| ((b as i16) - 128) * 256).collect()
    } else {
        rust_bridge_log_msg(&format!("RUST_AUDIO_PLAY_RAW: unsupported bits={}", bits));
        return 0;
    };

    let source = rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples);

    let sink = match Sink::try_new(stream_handle) {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_AUDIO_PLAY_RAW: sink error - {}", e));
            return 0;
        }
    };

    sink.set_volume(volumes.get_volume(category));

    if looping {
        sink.append(source.repeat_infinite());
    } else {
        sink.append(source);
    }

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    sinks.insert(handle, (sink, category));

    rust_bridge_log_msg(&format!(
        "RUST_AUDIO_PLAY_RAW: handle={} rate={} ch={} bits={} cat={:?}",
        handle, sample_rate, channels, bits, category
    ));

    handle
}

// =============================================================================
// FFI Functions
// =============================================================================

fn send_command(cmd: AudioCommand) -> bool {
    if let Ok(guard) = AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            return sender.send(cmd).is_ok();
        }
    }
    false
}

/// Initialize the audio system
#[no_mangle]
pub extern "C" fn rust_audio_init() -> i32 {
    rust_bridge_log_msg("RUST_AUDIO_INIT: starting rodio audio system");

    // Check if already running
    {
        let guard = AUDIO_SENDER.lock().unwrap();
        if guard.is_some() {
            rust_bridge_log_msg("RUST_AUDIO_INIT: already initialized");
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

    // Give the thread a moment to initialize
    std::thread::sleep(std::time::Duration::from_millis(50));

    rust_bridge_log_msg("RUST_AUDIO_INIT: success");
    1
}

/// Shutdown the audio system
#[no_mangle]
pub extern "C" fn rust_audio_uninit() {
    rust_bridge_log_msg("RUST_AUDIO_UNINIT");

    // Send shutdown command
    send_command(AudioCommand::Shutdown);

    // Clear sender (this will also cause the thread to exit if it's waiting)
    {
        let mut guard = AUDIO_SENDER.lock().unwrap();
        *guard = None;
    }

    // Wait for thread to finish
    {
        let mut guard = AUDIO_THREAD.lock().unwrap();
        if let Some(handle) = guard.take() {
            let _ = handle.join();
        }
    }
}

/// Play a WAV sound from raw bytes
#[no_mangle]
pub extern "C" fn rust_audio_play_wav(
    data: *const u8,
    len: usize,
    category: i32,
    looping: i32,
) -> u32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    let cat = match category {
        0 => SoundCategory::Music,
        1 => SoundCategory::Sfx,
        _ => SoundCategory::Speech,
    };

    let (tx, rx) = mpsc::channel();
    if send_command(AudioCommand::PlayWav(bytes, cat, looping != 0, tx)) {
        rx.recv().unwrap_or(0)
    } else {
        0
    }
}

/// Play an OGG sound from raw bytes
#[no_mangle]
pub extern "C" fn rust_audio_play_ogg(
    data: *const u8,
    len: usize,
    category: i32,
    looping: i32,
) -> u32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    let cat = match category {
        0 => SoundCategory::Music,
        1 => SoundCategory::Sfx,
        _ => SoundCategory::Speech,
    };

    let (tx, rx) = mpsc::channel();
    if send_command(AudioCommand::PlayOgg(bytes, cat, looping != 0, tx)) {
        rx.recv().unwrap_or(0)
    } else {
        0
    }
}

/// Play raw PCM audio data
#[no_mangle]
pub extern "C" fn rust_audio_play_raw(
    data: *const u8,
    len: usize,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    category: i32,
    looping: i32,
) -> u32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    let cat = match category {
        0 => SoundCategory::Music,
        1 => SoundCategory::Sfx,
        _ => SoundCategory::Speech,
    };

    let (tx, rx) = mpsc::channel();
    if send_command(AudioCommand::PlayRaw(
        bytes,
        sample_rate,
        channels,
        bits_per_sample,
        cat,
        looping != 0,
        tx,
    )) {
        rx.recv().unwrap_or(0)
    } else {
        0
    }
}

/// Stop a playing sound
#[no_mangle]
pub extern "C" fn rust_audio_stop(handle: u32) {
    send_command(AudioCommand::Stop(handle));
}

/// Pause a playing sound
#[no_mangle]
pub extern "C" fn rust_audio_pause(handle: u32) {
    send_command(AudioCommand::Pause(handle));
}

/// Resume a paused sound
#[no_mangle]
pub extern "C" fn rust_audio_resume(handle: u32) {
    send_command(AudioCommand::Resume(handle));
}

/// Set volume for a specific sound (0.0 - 1.0)
#[no_mangle]
pub extern "C" fn rust_audio_set_volume(handle: u32, volume: f32) {
    send_command(AudioCommand::SetVolume(handle, volume));
}

/// Set master volume (0.0 - 1.0)
#[no_mangle]
pub extern "C" fn rust_audio_set_master_volume(volume: f32) {
    send_command(AudioCommand::SetMasterVolume(volume));
}

/// Set music volume (0.0 - 1.0)
#[no_mangle]
pub extern "C" fn rust_audio_set_music_volume(volume: f32) {
    send_command(AudioCommand::SetMusicVolume(volume));
}

/// Set SFX volume (0.0 - 1.0)
#[no_mangle]
pub extern "C" fn rust_audio_set_sfx_volume(volume: f32) {
    send_command(AudioCommand::SetSfxVolume(volume));
}

/// Set speech volume (0.0 - 1.0)
#[no_mangle]
pub extern "C" fn rust_audio_set_speech_volume(volume: f32) {
    send_command(AudioCommand::SetSpeechVolume(volume));
}

/// Check if a sound is still playing
#[no_mangle]
pub extern "C" fn rust_audio_is_playing(handle: u32) -> i32 {
    let (tx, rx) = mpsc::channel();
    if send_command(AudioCommand::IsPlaying(handle, tx)) {
        if rx.recv().unwrap_or(false) { 1 } else { 0 }
    } else {
        0
    }
}

/// Stop all sounds
#[no_mangle]
pub extern "C" fn rust_audio_stop_all() {
    send_command(AudioCommand::StopAll);
}

/// Cleanup finished sounds
#[no_mangle]
pub extern "C" fn rust_audio_cleanup() {
    send_command(AudioCommand::Cleanup);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_state() {
        let v = VolumeState::new();
        assert_eq!(v.get_volume(SoundCategory::Music), 1.0);
        assert_eq!(v.get_volume(SoundCategory::Sfx), 1.0);
        assert_eq!(v.get_volume(SoundCategory::Speech), 1.0);
    }
}
