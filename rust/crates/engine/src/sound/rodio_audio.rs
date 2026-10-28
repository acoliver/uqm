//! Rodio-based audio system for UQM
//!
//! This replaces the hand-rolled OpenAL-style mixer with the rodio library,
//! which handles mixing, resampling, and audio output correctly.
//!
//! The audio runs on a dedicated thread since rodio's OutputStream is not Send.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use rodio::{OutputStream, OutputStreamHandle, Sink, Source};

use crate::bridge_log::rust_bridge_log_msg;

/// Commands sent to the audio thread
pub(super) enum AudioCommand {
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
#[derive(Default)]
struct AudioWorker {
    thread: Option<JoinHandle<()>>,
    replay: Option<super::replay_audio::ReplayAudio>,
}

static AUDIO_THREAD: Mutex<AudioWorker> = Mutex::new(AudioWorker {
    thread: None,
    replay: None,
});

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
                    let handle = play_decoded(
                        &stream_handle,
                        &data,
                        category,
                        looping,
                        &volumes,
                        &mut sinks,
                        "WAV",
                    );
                    let _ = response.send(handle);
                }
                AudioCommand::PlayOgg(data, category, looping, response) => {
                    let handle = play_decoded(
                        &stream_handle,
                        &data,
                        category,
                        looping,
                        &volumes,
                        &mut sinks,
                        "OGG",
                    );
                    let _ = response.send(handle);
                }
                AudioCommand::PlayRaw(
                    data,
                    sample_rate,
                    channels,
                    bits,
                    category,
                    looping,
                    response,
                ) => {
                    let input = RawPlayback {
                        data: &data,
                        sample_rate,
                        channels,
                        bits,
                        category,
                        looping,
                    };
                    let handle = play_raw(&stream_handle, &input, &volumes, &mut sinks);
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

struct RawPlayback<'a> {
    data: &'a [u8],
    sample_rate: u32,
    channels: u16,
    bits: u16,
    category: SoundCategory,
    looping: bool,
}

fn pcm_samples(data: &[u8], bits: u16) -> Option<Vec<i16>> {
    if bits == 16 {
        Some(
            data.as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect(),
        )
    } else if bits == 8 {
        Some(data.iter().map(|&b| ((b as i16) - 128) * 256).collect())
    } else {
        rust_bridge_log_msg(&format!("RUST_AUDIO_PLAY_RAW: unsupported bits={}", bits));
        None
    }
}

fn play_raw(
    stream_handle: &OutputStreamHandle,
    input: &RawPlayback<'_>,
    volumes: &VolumeState,
    sinks: &mut HashMap<u32, (Sink, SoundCategory)>,
) -> u32 {
    let Some(samples) = pcm_samples(input.data, input.bits) else {
        return 0;
    };

    let source = rodio::buffer::SamplesBuffer::new(input.channels, input.sample_rate, samples);

    let sink = match Sink::try_new(stream_handle) {
        Ok(s) => s,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_AUDIO_PLAY_RAW: sink error - {}", e));
            return 0;
        }
    };

    sink.set_volume(volumes.get_volume(input.category));

    if input.looping {
        sink.append(source.repeat_infinite());
    } else {
        sink.append(source);
    }

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    sinks.insert(handle, (sink, input.category));

    rust_bridge_log_msg(&format!(
        "RUST_AUDIO_PLAY_RAW: handle={} rate={} ch={} bits={} cat={:?}",
        handle, input.sample_rate, input.channels, input.bits, input.category
    ));

    handle
}

// =============================================================================
// FFI Functions
// =============================================================================

fn send_command(cmd: AudioCommand) -> bool {
    if crate::time::source::is_replay() {
        let mut worker = AUDIO_THREAD.lock().expect("audio worker poisoned");
        if let Some(replay) = &mut worker.replay {
            replay.command(cmd, crate::time::source::now_ticks());
            return true;
        }
        return false;
    }
    if let Ok(guard) = AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            return sender.send(cmd).is_ok();
        }
    }
    false
}

/// Initialize the audio system
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_init() -> i32 {
    if crate::time::source::is_replay() {
        AUDIO_THREAD
            .lock()
            .expect("audio worker poisoned")
            .replay
            .get_or_insert_with(super::replay_audio::ReplayAudio::default);
        return 1;
    }
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
        guard.thread = Some(handle);
    }

    // Give the thread a moment to initialize
    std::thread::sleep(std::time::Duration::from_millis(50));

    rust_bridge_log_msg("RUST_AUDIO_INIT: success");
    1
}

/// Shutdown the audio system
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_uninit() {
    rust_bridge_log_msg("RUST_AUDIO_UNINIT");

    // Send shutdown command
    send_command(AudioCommand::Shutdown);
    AUDIO_THREAD.lock().expect("audio worker poisoned").replay = None;

    // Clear sender (this will also cause the thread to exit if it's waiting)
    {
        let mut guard = AUDIO_SENDER.lock().unwrap();
        *guard = None;
    }

    // Wait for thread to finish
    {
        let mut guard = AUDIO_THREAD.lock().unwrap();
        if let Some(handle) = guard.thread.take() {
            let _ = handle.join();
        }
    }
}

/// Play a WAV sound from raw bytes
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_play_wav(
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_play_ogg(
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_play_raw(
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

/// Play raw PCM audio directly from Rust (no FFI pointer dance).
///
/// Returns a handle > 0 on success, 0 on failure.
pub fn play_raw_pcm(
    pcm_data: Vec<u8>,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    category: SoundCategory,
    looping: bool,
) -> u32 {
    let (tx, rx) = mpsc::channel();
    if send_command(AudioCommand::PlayRaw(
        pcm_data,
        sample_rate,
        channels,
        bits_per_sample,
        category,
        looping,
        tx,
    )) {
        rx.recv().unwrap_or(0)
    } else {
        rust_bridge_log_msg(
            "RUST_AUDIO: play_raw_pcm send_command failed (audio thread not running?)",
        );
        0
    }
}

/// Stop a playing sound by handle (Rust-callable).
pub fn stop_sound(handle: u32) {
    send_command(AudioCommand::Stop(handle));
}

/// Stop a playing sound
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_stop(handle: u32) {
    send_command(AudioCommand::Stop(handle));
}

/// Pause a playing sound
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_pause(handle: u32) {
    send_command(AudioCommand::Pause(handle));
}

/// Resume a paused sound
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_resume(handle: u32) {
    send_command(AudioCommand::Resume(handle));
}

/// Set volume for a specific sound (0.0 - 1.0)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_set_volume(handle: u32, volume: f32) {
    send_command(AudioCommand::SetVolume(handle, volume));
}

/// Set master volume (0.0 - 1.0)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_set_master_volume(volume: f32) {
    send_command(AudioCommand::SetMasterVolume(volume));
}

/// Set music volume (0.0 - 1.0)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_set_music_volume(volume: f32) {
    send_command(AudioCommand::SetMusicVolume(volume));
}

/// Set SFX volume (0.0 - 1.0)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_set_sfx_volume(volume: f32) {
    send_command(AudioCommand::SetSfxVolume(volume));
}

/// Set speech volume (0.0 - 1.0)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_set_speech_volume(volume: f32) {
    send_command(AudioCommand::SetSpeechVolume(volume));
}

/// Check if a sound is still playing
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_is_playing(handle: u32) -> i32 {
    let (tx, rx) = mpsc::channel();
    if send_command(AudioCommand::IsPlaying(handle, tx)) {
        if rx.recv().unwrap_or(false) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// Stop all sounds
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_stop_all() {
    send_command(AudioCommand::StopAll);
}

/// Cleanup finished sounds
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe extern "C" fn rust_audio_cleanup() {
    send_command(AudioCommand::Cleanup);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_conversion_preserves_signed_limits_centers_unsigned_audio_and_ignores_partial_frames() {
        assert_eq!(
            pcm_samples(&[0, 128, 255, 255, 0, 0, 255, 127, 42], 16),
            Some(vec![i16::MIN, -1, 0, i16::MAX])
        );
        assert_eq!(
            pcm_samples(&[0, 127, 128, 255], 8),
            Some(vec![-32768, -256, 0, 32512])
        );
        assert_eq!(pcm_samples(&[], 16), Some(Vec::new()));
        assert_eq!(pcm_samples(&[42], 16), Some(Vec::new()));
        assert_eq!(pcm_samples(&[0; 3], 24), None);
    }

    #[test]
    fn test_volume_state() {
        let v = VolumeState::new();
        assert_eq!(v.get_volume(SoundCategory::Music), 1.0);
        assert_eq!(v.get_volume(SoundCategory::Sfx), 1.0);
        assert_eq!(v.get_volume(SoundCategory::Speech), 1.0);
    }

    #[test]
    fn category_volume_scales_with_the_master_volume() {
        let mut v = VolumeState::new();
        v.master = 0.5;
        v.music = 0.8;
        v.sfx = 0.25;
        v.speech = 0.0;

        assert!((v.get_volume(SoundCategory::Music) - 0.4).abs() < 1e-6);
        assert!((v.get_volume(SoundCategory::Sfx) - 0.125).abs() < 1e-6);
        assert_eq!(v.get_volume(SoundCategory::Speech), 0.0);
    }
}

/// The exports talk to the audio thread over one process-wide channel. These
/// tests stand in a thread that records every command it receives and answers
/// the requests that expect a reply, and run one at a time.
#[cfg(test)]
mod command_export_tests {
    use super::*;
    use crate::sound::test_support::Probe;
    use std::sync::{Arc, MutexGuard};

    static EXPORT_LOCK: Mutex<()> = Mutex::new(());

    #[derive(Debug, Clone, PartialEq)]
    enum Seen {
        Wav(Vec<u8>, SoundCategory, bool),
        Ogg(Vec<u8>, SoundCategory, bool),
        Raw(Vec<u8>, u32, u16, u16, SoundCategory, bool),
        Stop(u32),
        Pause(u32),
        Resume(u32),
        SetVolume(u32, f32),
        IsPlaying(u32),
        StopAll,
        Master(f32),
        Music(f32),
        Sfx(f32),
        Speech(f32),
        Cleanup,
        Shutdown,
    }

    const PLAY_HANDLE: u32 = 4242;

    struct Recorder {
        seen: Arc<Mutex<Vec<Seen>>>,
        _exclusive: MutexGuard<'static, ()>,
    }

    impl Recorder {
        fn start(is_playing: bool) -> Self {
            let exclusive = EXPORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            assert!(
                AUDIO_SENDER.lock().unwrap().is_none(),
                "no other test may leave an audio thread running"
            );
            let (tx, rx) = mpsc::channel();
            *AUDIO_SENDER.lock().unwrap() = Some(tx);
            let seen = Arc::new(Mutex::new(Vec::new()));
            let log = Arc::clone(&seen);
            let thread = thread::spawn(move || {
                // A reply releases the caller to read the log, so the command
                // is logged before it is answered.
                let record = |entry: Seen| log.lock().unwrap().push(entry);
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        AudioCommand::PlayWav(data, cat, looping, reply) => {
                            record(Seen::Wav(data, cat, looping));
                            let _ = reply.send(PLAY_HANDLE);
                        }
                        AudioCommand::PlayOgg(data, cat, looping, reply) => {
                            record(Seen::Ogg(data, cat, looping));
                            let _ = reply.send(PLAY_HANDLE + 1);
                        }
                        AudioCommand::PlayRaw(data, rate, channels, bits, cat, looping, reply) => {
                            record(Seen::Raw(data, rate, channels, bits, cat, looping));
                            let _ = reply.send(PLAY_HANDLE + 2);
                        }
                        AudioCommand::Stop(h) => record(Seen::Stop(h)),
                        AudioCommand::Pause(h) => record(Seen::Pause(h)),
                        AudioCommand::Resume(h) => record(Seen::Resume(h)),
                        AudioCommand::SetVolume(h, v) => record(Seen::SetVolume(h, v)),
                        AudioCommand::IsPlaying(h, reply) => {
                            record(Seen::IsPlaying(h));
                            let _ = reply.send(is_playing);
                        }
                        AudioCommand::StopAll => record(Seen::StopAll),
                        AudioCommand::SetMasterVolume(v) => record(Seen::Master(v)),
                        AudioCommand::SetMusicVolume(v) => record(Seen::Music(v)),
                        AudioCommand::SetSfxVolume(v) => record(Seen::Sfx(v)),
                        AudioCommand::SetSpeechVolume(v) => record(Seen::Speech(v)),
                        AudioCommand::Cleanup => record(Seen::Cleanup),
                        AudioCommand::Shutdown => {
                            record(Seen::Shutdown);
                            break;
                        }
                    }
                }
            });
            AUDIO_THREAD.lock().unwrap().thread = Some(thread);
            Self {
                seen,
                _exclusive: exclusive,
            }
        }

        /// Everything recorded so far; waits for in-flight commands by round
        /// tripping a reply-bearing request first.
        fn seen(&self) -> Vec<Seen> {
            let _ = unsafe { rust_audio_is_playing(0) };
            self.seen.lock().unwrap().clone()
        }
    }

    impl Drop for Recorder {
        fn drop(&mut self) {
            unsafe { rust_audio_uninit() };
        }
    }

    /// `seen()` trusts that a command answered through a reply has already
    /// been logged; holding the log shows whether the reply waits for it.
    #[test]
    fn a_reply_is_sent_only_after_the_recorder_has_logged_the_command() {
        let recorder = Recorder::start(false);
        let held_log = recorder.seen.lock().unwrap();
        let asking = Probe::spawn(|| {
            let _ = unsafe { rust_audio_is_playing(0) };
        });

        let answered_before_logging = asking.finishes_within_grace();
        drop(held_log);
        let outcome = asking.join();

        assert!(
            !answered_before_logging,
            "the reply was sent while the command was not yet in the log"
        );
        assert!(outcome.is_ok());
        assert_eq!(*recorder.seen.lock().unwrap(), [Seen::IsPlaying(0)]);
    }

    #[test]
    fn plays_report_no_handle_when_the_audio_thread_is_not_running() {
        let _exclusive = EXPORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert!(AUDIO_SENDER.lock().unwrap().is_none());
        let bytes = [1u8, 2, 3];

        assert_eq!(unsafe { rust_audio_play_wav(bytes.as_ptr(), 3, 0, 0) }, 0);
        assert_eq!(unsafe { rust_audio_play_ogg(bytes.as_ptr(), 3, 0, 0) }, 0);
        assert_eq!(
            unsafe { rust_audio_play_raw(bytes.as_ptr(), 3, 8000, 1, 8, 0, 0) },
            0
        );
        assert_eq!(
            play_raw_pcm(vec![1, 2], 8000, 1, 16, SoundCategory::Sfx, false),
            0
        );
        assert_eq!(unsafe { rust_audio_is_playing(5) }, 0);
        assert!(!send_command(AudioCommand::StopAll));
    }

    #[test]
    fn empty_or_missing_sound_data_is_refused_before_it_is_sent() {
        let recorder = Recorder::start(false);
        let bytes = [1u8];

        assert_eq!(unsafe { rust_audio_play_wav(std::ptr::null(), 4, 0, 0) }, 0);
        assert_eq!(unsafe { rust_audio_play_wav(bytes.as_ptr(), 0, 0, 0) }, 0);
        assert_eq!(unsafe { rust_audio_play_ogg(std::ptr::null(), 4, 0, 0) }, 0);
        assert_eq!(unsafe { rust_audio_play_ogg(bytes.as_ptr(), 0, 0, 0) }, 0);
        assert_eq!(
            unsafe { rust_audio_play_raw(std::ptr::null(), 4, 8000, 1, 8, 0, 0) },
            0
        );
        assert_eq!(
            unsafe { rust_audio_play_raw(bytes.as_ptr(), 0, 8000, 1, 8, 0, 0) },
            0
        );

        assert_eq!(recorder.seen(), [Seen::IsPlaying(0)]);
    }

    #[test]
    fn wav_and_ogg_plays_carry_their_bytes_category_and_loop_flag() {
        let recorder = Recorder::start(false);
        let wav = [10u8, 11, 12];
        let ogg = [20u8, 21];

        let wav_handle = unsafe { rust_audio_play_wav(wav.as_ptr(), 3, 0, 1) };
        let ogg_handle = unsafe { rust_audio_play_ogg(ogg.as_ptr(), 2, 1, 0) };
        let speech_handle = unsafe { rust_audio_play_wav(wav.as_ptr(), 3, 7, 5) };

        assert_eq!(wav_handle, PLAY_HANDLE);
        assert_eq!(ogg_handle, PLAY_HANDLE + 1);
        assert_eq!(speech_handle, PLAY_HANDLE);
        assert_eq!(
            recorder.seen(),
            [
                Seen::Wav(vec![10, 11, 12], SoundCategory::Music, true),
                Seen::Ogg(vec![20, 21], SoundCategory::Sfx, false),
                Seen::Wav(vec![10, 11, 12], SoundCategory::Speech, true),
                Seen::IsPlaying(0),
            ],
            "unknown categories are treated as speech and any non-zero loop flag loops"
        );
    }

    #[test]
    fn raw_plays_carry_the_pcm_format_through_both_entry_points() {
        let recorder = Recorder::start(false);
        let pcm = [0u8, 1, 2, 3];

        let from_c = unsafe { rust_audio_play_raw(pcm.as_ptr(), 4, 22050, 2, 16, 2, 0) };
        let from_rust = play_raw_pcm(vec![9, 8], 11025, 1, 8, SoundCategory::Music, true);

        assert_eq!(from_c, PLAY_HANDLE + 2);
        assert_eq!(from_rust, PLAY_HANDLE + 2);
        assert_eq!(
            recorder.seen(),
            [
                Seen::Raw(vec![0, 1, 2, 3], 22050, 2, 16, SoundCategory::Speech, false),
                Seen::Raw(vec![9, 8], 11025, 1, 8, SoundCategory::Music, true),
                Seen::IsPlaying(0),
            ]
        );
    }

    #[test]
    fn transport_and_volume_controls_arrive_in_order() {
        let recorder = Recorder::start(false);

        unsafe {
            rust_audio_pause(3);
            rust_audio_resume(3);
            rust_audio_set_volume(3, 0.5);
            rust_audio_set_master_volume(0.9);
            rust_audio_set_music_volume(0.8);
            rust_audio_set_sfx_volume(0.7);
            rust_audio_set_speech_volume(0.6);
            rust_audio_cleanup();
            rust_audio_stop(3);
            stop_sound(4);
            rust_audio_stop_all();
        }

        assert_eq!(
            recorder.seen(),
            [
                Seen::Pause(3),
                Seen::Resume(3),
                Seen::SetVolume(3, 0.5),
                Seen::Master(0.9),
                Seen::Music(0.8),
                Seen::Sfx(0.7),
                Seen::Speech(0.6),
                Seen::Cleanup,
                Seen::Stop(3),
                Seen::Stop(4),
                Seen::StopAll,
                Seen::IsPlaying(0),
            ]
        );
    }

    #[test]
    fn is_playing_reports_what_the_audio_thread_answers() {
        let playing = Recorder::start(true);
        assert_eq!(unsafe { rust_audio_is_playing(9) }, 1);
        drop(playing);

        let idle = Recorder::start(false);
        assert_eq!(unsafe { rust_audio_is_playing(9) }, 0);
        drop(idle);
    }

    #[test]
    fn init_keeps_a_running_audio_thread_and_uninit_shuts_it_down() {
        let recorder = Recorder::start(false);

        assert_eq!(unsafe { rust_audio_init() }, 1, "already initialised");
        assert_eq!(
            unsafe { rust_audio_is_playing(1) },
            0,
            "same thread answers"
        );

        unsafe { rust_audio_uninit() };

        assert_eq!(
            *recorder.seen.lock().unwrap(),
            [Seen::IsPlaying(1), Seen::Shutdown]
        );
        assert!(AUDIO_SENDER.lock().unwrap().is_none());
        assert!(AUDIO_THREAD.lock().unwrap().thread.is_none());
    }
}
