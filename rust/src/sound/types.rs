// @plan PLAN-20260225-AUDIO-HEART.P03
// @requirement REQ-CROSS-CONST-01..08, REQ-CROSS-ERROR-01..03, REQ-CROSS-GENERAL-01,04,05,07,08
#![allow(dead_code, unused_imports)]

//! Shared types for the Audio Heart streaming pipeline.
//!
//! Defines error types, constants, core structs, and callback traits
//! used across all audio heart modules (stream, trackplayer, music, sfx,
//! control, fileinst, heart_ffi).

use std::any::Any;
use std::sync::Arc;

use parking_lot::Mutex;

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::mixer::types::MixerError;

// =============================================================================
// Error Types (REQ-CROSS-ERROR-01..03)
// =============================================================================

/// Unified error type for the audio heart subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    NotInitialized,
    AlreadyInitialized,
    InvalidSource(usize),
    InvalidChannel(usize),
    InvalidSample,
    InvalidDecoder,
    DecoderError(String),
    MixerError(MixerError),
    IoError(String),
    NullPointer,
    ConcurrentLoad,
    ResourceNotFound(String),
    EndOfStream,
    BufferUnderrun,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NotInitialized => write!(f, "audio not initialized"),
            AudioError::AlreadyInitialized => write!(f, "audio already initialized"),
            AudioError::InvalidSource(idx) => write!(f, "invalid source index {}", idx),
            AudioError::InvalidChannel(ch) => write!(f, "invalid channel {}", ch),
            AudioError::InvalidSample => write!(f, "invalid sample"),
            AudioError::InvalidDecoder => write!(f, "invalid decoder"),
            AudioError::DecoderError(e) => write!(f, "decoder error: {}", e),
            AudioError::MixerError(e) => write!(f, "mixer error: {:?}", e),
            AudioError::IoError(e) => write!(f, "I/O error: {}", e),
            AudioError::NullPointer => write!(f, "null pointer"),
            AudioError::ConcurrentLoad => write!(f, "concurrent load in progress"),
            AudioError::ResourceNotFound(name) => write!(f, "resource not found: {}", name),
            AudioError::EndOfStream => write!(f, "end of stream"),
            AudioError::BufferUnderrun => write!(f, "buffer underrun"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<MixerError> for AudioError {
    fn from(e: MixerError) -> Self {
        AudioError::MixerError(e)
    }
}

impl From<DecodeError> for AudioError {
    fn from(e: DecodeError) -> Self {
        match e {
            DecodeError::EndOfFile => AudioError::EndOfStream,
            DecodeError::NotInitialized => AudioError::NotInitialized,
            DecodeError::NotFound(s) => AudioError::ResourceNotFound(s),
            other => AudioError::DecoderError(format!("{:?}", other)),
        }
    }
}

/// Convenience Result alias for audio operations.
pub type AudioResult<T> = Result<T, AudioError>;

// =============================================================================
// Constants (REQ-CROSS-CONST-01..08)
// =============================================================================

/// Number of SFX channels available for simultaneous sound effects.
pub const NUM_SFX_CHANNELS: usize = 5;

/// First SFX source index.
pub const FIRST_SFX_SOURCE: usize = 0;

/// Last SFX source index.
pub const LAST_SFX_SOURCE: usize = FIRST_SFX_SOURCE + NUM_SFX_CHANNELS - 1;

/// Music source index (follows SFX sources).
pub const MUSIC_SOURCE: usize = LAST_SFX_SOURCE + 1;

/// Speech source index (follows music).
pub const SPEECH_SOURCE: usize = MUSIC_SOURCE + 1;

/// Total number of sound sources.
pub const NUM_SOUNDSOURCES: usize = SPEECH_SOURCE + 1;

/// Maximum volume level (used by C code).
pub const MAX_VOLUME: i32 = 255;

/// Normal (default) volume level.
pub const NORMAL_VOLUME: i32 = 160;

/// Number of scope buffer bytes for oscilloscope display.
pub const PAD_SCOPE_BYTES: usize = 256;

/// Accelerated scroll speed for comm screen.
pub const ACCEL_SCROLL_SPEED: u32 = 300;

/// Text display speed for subtitles.
pub const TEXT_SPEED: u32 = 80;

/// One second in GetTimeCounter ticks.
pub const ONE_SECOND: u32 = 840;

/// Number of mixer buffers per streaming source.
pub const NUM_BUFFERS_PER_SOURCE: u32 = 8;

/// Size of each mixer buffer (in bytes).
pub const BUFFER_SIZE: usize = 16384;

// =============================================================================
// Time FFI (REQ-CROSS-GENERAL-05)
// =============================================================================

extern "C" {
    fn GetTimeCounter() -> u32;
    fn QuitPosted() -> i32;
}

/// Safe wrapper around the C `GetTimeCounter()` function.
pub fn get_time_counter() -> u32 {
    unsafe { GetTimeCounter() }
}

/// Safe wrapper around the C `QuitPosted()` function.
pub fn quit_posted() -> bool {
    unsafe { QuitPosted() != 0 }
}

// =============================================================================
// Core Structs
// =============================================================================

/// Audio sample — owns buffers, borrows decoder.
/// Replaces TFB_SoundSample from C.
pub struct SoundSample {
    /// Decoder for this sample (None if no source data).
    pub decoder: Option<Box<dyn SoundDecoder>>,
    /// Total length in seconds.
    pub length: f32,
    /// Mixer buffer handles.
    pub buffers: Vec<usize>,
    /// Number of active buffers.
    pub num_buffers: u32,
    /// Per-buffer tags for subtitle synchronization.
    pub buffer_tags: Vec<Option<SoundTag>>,
    /// Initial time offset (for track positioning).
    pub offset: i32,
    /// Whether this sample should loop (stored here, not on decoder).
    pub looping: bool,
    /// Opaque user data (game-specific).
    pub data: Option<Box<dyn Any + Send>>,
    /// Stream callbacks.
    pub callbacks: Option<Box<dyn StreamCallbacks + Send>>,
}

impl SoundSample {
    /// Create a new empty sound sample.
    pub fn new() -> Self {
        SoundSample {
            decoder: None,
            length: 0.0,
            buffers: Vec::new(),
            num_buffers: 0,
            buffer_tags: Vec::new(),
            offset: 0,
            looping: false,
            data: None,
            callbacks: None,
        }
    }
}

/// Buffer tag for subtitle synchronization.
/// Replaces TFB_SoundTag from C.
pub struct SoundTag {
    /// Mixer buffer handle this tag is attached to.
    pub buf_handle: usize,
    /// Opaque payload (chunk pointer equivalent).
    pub data: usize,
}

/// Stream callbacks — replaces TFB_SoundCallbacks function pointers.
pub trait StreamCallbacks: Send {
    /// Called before initial buffering. Return false to abort.
    fn on_start_stream(&mut self, _sample: &mut SoundSample) -> bool {
        true
    }

    /// Called when decoder hits EOF. Return true if a new decoder was set.
    fn on_end_chunk(&mut self, _sample: &mut SoundSample, _buffer: usize) -> bool {
        false
    }

    /// Called when all buffers played and no more data.
    fn on_end_stream(&mut self, _sample: &mut SoundSample) {}

    /// Called when a tagged buffer finishes playback.
    fn on_tagged_buffer(&mut self, _sample: &mut SoundSample, _tag: &SoundTag) {}

    /// Called when a buffer is queued.
    fn on_queue_buffer(&mut self, _sample: &mut SoundSample, _buffer: usize) {}
}

/// Per-source state — replaces TFB_SoundSource.
pub struct SoundSource {
    /// The sample currently attached to this source.
    pub sample: Option<Arc<Mutex<SoundSample>>>,
    /// Mixer source handle.
    pub handle: usize,
    /// Whether this source should currently be streaming audio.
    pub stream_should_be_playing: bool,
    /// Playback start timestamp (GetTimeCounter ticks).
    pub start_time: i32,
    /// Pause timestamp (0 = not paused).
    pub pause_time: u32,
    /// Opaque game object pointer for positional audio.
    pub positional_object: usize,
    /// Last queued buffer handle.
    pub last_q_buf: usize,
    /// Oscilloscope ring buffer.
    pub sbuffer: Option<Vec<u8>>,
    /// Scope buffer capacity.
    pub sbuf_size: u32,
    /// Scope write pointer.
    pub sbuf_tail: u32,
    /// Scope read pointer.
    pub sbuf_head: u32,
    /// Last scope sample time.
    pub sbuf_lasttime: u32,
}

impl SoundSource {
    /// Create a new inactive sound source.
    pub fn new() -> Self {
        SoundSource {
            sample: None,
            handle: 0,
            stream_should_be_playing: false,
            start_time: 0,
            pause_time: 0,
            positional_object: 0,
            last_q_buf: 0,
            sbuffer: None,
            sbuf_size: 0,
            sbuf_tail: 0,
            sbuf_head: 0,
            sbuf_lasttime: 0,
        }
    }
}

/// Fade state — protected by its own mutex, separate from sources.
pub struct FadeState {
    /// Fade start time (GetTimeCounter ticks).
    pub start_time: u32,
    /// Fade interval in ticks (0 = inactive).
    pub interval: u32,
    /// Volume at fade start.
    pub start_volume: i32,
    /// Volume delta (end_volume - start_volume).
    pub delta: i32,
}

impl FadeState {
    /// Create a new inactive fade state.
    pub fn new() -> Self {
        FadeState {
            start_time: 0,
            interval: 0,
            start_volume: 0,
            delta: 0,
        }
    }
}

/// 3D position for sound effects.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundPosition {
    /// Whether positional audio is enabled.
    pub positional: bool,
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

impl SoundPosition {
    /// Non-positional (default).
    pub fn non_positional() -> Self {
        SoundPosition {
            positional: false,
            x: 0,
            y: 0,
        }
    }
}

/// Track chunk — linked list node for assembled tracks.
pub struct SoundChunk {
    /// Sound sample for this chunk.
    pub sample: Option<Arc<Mutex<SoundSample>>>,
    /// Start timestamp for this chunk (GetTimeCounter ticks).
    pub start_time: u32,
    /// Tag data for subtitle matching.
    pub tag_me: bool,
    /// Tag data value.
    pub tag_data: usize,
    /// Callback to call at start of this chunk.
    pub callback: Option<Box<dyn FnOnce() + Send>>,
    /// Next chunk in linked list.
    pub next: Option<Box<SoundChunk>>,
}

/// Music reference — shared ownership wrapper for loaded music samples.
#[repr(transparent)]
pub struct MusicRef(pub Arc<Mutex<SoundSample>>);

impl MusicRef {
    /// Create a new MusicRef wrapping a sample.
    pub fn new(sample: SoundSample) -> Self {
        MusicRef(Arc::new(Mutex::new(sample)))
    }
}

impl Clone for MusicRef {
    fn clone(&self) -> Self {
        MusicRef(Arc::clone(&self.0))
    }
}

/// Sound bank — collection of SFX samples loaded from a resource file.
pub struct SoundBank {
    /// Samples in this bank.
    pub samples: Vec<SoundSample>,
    /// Resource file name this bank was loaded from.
    pub source_file: Option<String>,
}

impl SoundBank {
    /// Create a new empty sound bank.
    pub fn new() -> Self {
        SoundBank {
            samples: Vec::new(),
            source_file: None,
        }
    }
}

/// Subtitle reference — pointer into chunk list for subtitle display.
pub struct SubtitleRef {
    /// Tag data value (opaque pointer equivalent).
    pub data: usize,
}

// =============================================================================
// Free Functions (Decoder Trait Gap Resolution)
// =============================================================================

/// Decode all remaining data from a decoder into a Vec<u8>.
/// Loops `decoder.decode()` until EOF, collecting bytes.
pub fn decode_all(decoder: &mut dyn SoundDecoder) -> DecodeResult<Vec<u8>> {
    todo!("P05: decode_all implementation")
}

/// Compute the current playback time of a decoder in seconds.
pub fn get_decoder_time(decoder: &dyn SoundDecoder) -> f32 {
    todo!("P05: get_decoder_time implementation")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_error_display() {
        assert_eq!(
            AudioError::InvalidSource(5).to_string(),
            "invalid source index 5"
        );
        assert_eq!(AudioError::EndOfStream.to_string(), "end of stream");
        assert_eq!(
            AudioError::ResourceNotFound("foo.ogg".into()).to_string(),
            "resource not found: foo.ogg"
        );
    }

    #[test]
    fn test_audio_error_from_mixer_error() {
        let ae: AudioError = MixerError::InvalidName.into();
        assert_eq!(ae, AudioError::MixerError(MixerError::InvalidName));
    }

    #[test]
    fn test_audio_error_from_decode_error() {
        let ae: AudioError = DecodeError::EndOfFile.into();
        assert_eq!(ae, AudioError::EndOfStream);

        let ae2: AudioError = DecodeError::NotInitialized.into();
        assert_eq!(ae2, AudioError::NotInitialized);

        let ae3: AudioError = DecodeError::NotFound("x".into()).into();
        assert_eq!(ae3, AudioError::ResourceNotFound("x".into()));
    }

    #[test]
    fn test_constants() {
        assert_eq!(NUM_SFX_CHANNELS, 5);
        assert_eq!(FIRST_SFX_SOURCE, 0);
        assert_eq!(LAST_SFX_SOURCE, 4);
        assert_eq!(MUSIC_SOURCE, 5);
        assert_eq!(SPEECH_SOURCE, 6);
        assert_eq!(NUM_SOUNDSOURCES, 7);
        assert_eq!(MAX_VOLUME, 255);
        assert_eq!(NORMAL_VOLUME, 160);
        assert_eq!(ONE_SECOND, 840);
    }

    #[test]
    fn test_sound_sample_new() {
        let sample = SoundSample::new();
        assert!(sample.decoder.is_none());
        assert_eq!(sample.length, 0.0);
        assert!(!sample.looping);
        assert!(sample.buffers.is_empty());
    }

    #[test]
    fn test_sound_source_new() {
        let source = SoundSource::new();
        assert!(source.sample.is_none());
        assert_eq!(source.handle, 0);
        assert!(!source.stream_should_be_playing);
        assert_eq!(source.pause_time, 0);
    }

    #[test]
    fn test_fade_state_new() {
        let fade = FadeState::new();
        assert_eq!(fade.interval, 0);
        assert_eq!(fade.delta, 0);
    }

    #[test]
    fn test_sound_position() {
        let pos = SoundPosition::non_positional();
        assert!(!pos.positional);
        assert_eq!(pos.x, 0);
        assert_eq!(pos.y, 0);
    }

    #[test]
    fn test_music_ref_clone() {
        let mref = MusicRef::new(SoundSample::new());
        let mref2 = mref.clone();
        assert!(Arc::ptr_eq(&mref.0, &mref2.0));
    }

    #[test]
    fn test_sound_bank_new() {
        let bank = SoundBank::new();
        assert!(bank.samples.is_empty());
        assert!(bank.source_file.is_none());
    }

    #[test]
    fn test_audio_error_14_variants() {
        // Verify all 14 variants exist and are constructible
        let variants: Vec<AudioError> = vec![
            AudioError::NotInitialized,
            AudioError::AlreadyInitialized,
            AudioError::InvalidSource(0),
            AudioError::InvalidChannel(0),
            AudioError::InvalidSample,
            AudioError::InvalidDecoder,
            AudioError::DecoderError(String::new()),
            AudioError::MixerError(MixerError::NoError),
            AudioError::IoError(String::new()),
            AudioError::NullPointer,
            AudioError::ConcurrentLoad,
            AudioError::ResourceNotFound(String::new()),
            AudioError::EndOfStream,
            AudioError::BufferUnderrun,
        ];
        assert_eq!(variants.len(), 14);
    }

    #[test]
    fn test_stream_callbacks_default() {
        struct TestCallbacks;
        impl StreamCallbacks for TestCallbacks {}
        let mut cb = TestCallbacks;
        let mut sample = SoundSample::new();
        assert!(cb.on_start_stream(&mut sample));
        assert!(!cb.on_end_chunk(&mut sample, 0));
    }
}
