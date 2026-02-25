# Rust Audio Heart — Functional & Technical Specification

**Scope:** The six C subsystems that form the "heart" of UQM's audio pipeline — streaming, track player, music, SFX, volume/sound control, and file loading — specified as Rust modules with EARS requirements. When implemented, these modules replace `stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, and `fileinst.c` entirely.

**Companion document:** `c-heart.md` — C behavior analysis and requirements (requirement IDs are shared).

---

## 1. Architecture Overview

### 1.1 Module Mapping

| C File | Rust Module | Location |
|--------|-------------|----------|
| `stream.c` | `sound::stream` | `rust/src/sound/stream.rs` |
| `trackplayer.c` | `sound::trackplayer` | `rust/src/sound/trackplayer.rs` |
| `music.c` | `sound::music` | `rust/src/sound/music.rs` |
| `sfx.c` | `sound::sfx` | `rust/src/sound/sfx.rs` |
| `sound.c` | `sound::control` | `rust/src/sound/control.rs` |
| `fileinst.c` | `sound::fileinst` | `rust/src/sound/fileinst.rs` |
| (FFI shims) | `sound::heart_ffi` | `rust/src/sound/heart_ffi.rs` |

### 1.2 Existing Rust Modules (Reused, Not Modified)

| Module | Role | Key Types |
|--------|------|-----------|
| `sound::mixer` | OpenAL-like mixer engine | `MixerSource`, `MixerBuffer`, `MixerFormat`, `SourceProp`, `BufferProp`, `SourceState` |
| `sound::mixer::ffi` | C FFI for mixer | `rust_mixer_*` functions, `MixerObject`, `MixerIntVal` |
| `sound::decoder` | Decoder trait | `SoundDecoder`, `DecodeError`, `DecodeResult` |
| `sound::formats` | Audio formats | `AudioFormat`, `DecoderFormats` |
| `sound::wav` | WAV decoder | `WavDecoder` |
| `sound::ogg` | Ogg Vorbis decoder | `OggDecoder` |
| `sound::mod_decoder` | MOD decoder | `ModDecoder` |
| `sound::dukaud` | DukAud decoder | `DukAudDecoder` |
| `sound::null` | Silent decoder | `NullDecoder` |
| `sound::rodio_backend` | Rodio backend | `AudioObject`, `AudioIntVal` |

### 1.3 Layered Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  C Game Logic (comm screens, menus, combat)                      │
│  (still C — calls through FFI)                                   │
├──────────────────────────────────────────────────────────────────┤
│  sound::heart_ffi                                                │
│  #[no_mangle] pub extern "C" fn PLRPlaySong(...)                 │
│  #[no_mangle] pub extern "C" fn PlayChannel(...)                 │
│  ... thin shims, no logic ...                                    │
├──────────┬──────────┬───────────────┬────────────────────────────┤
│ music.rs │ sfx.rs   │ trackplayer.rs│ fileinst.rs                │
│ play/    │ play     │ splice/play/  │ load_sound_file            │
│ stop/    │ channel  │ seek/subtitle │ load_music_file            │
│ pause    │          │               │                            │
├──────────┴──────────┴───────────────┴────────────────────────────┤
│                    stream.rs                                      │
│  play_stream / stop_stream / pause_stream / resume_stream         │
│  StreamDecoderTask (background thread)                            │
│  SoundSample / SoundTag / scope buffer                            │
├──────────────────────────────────────────────────────────────────┤
│                    control.rs                                     │
│  SoundSourceArray (Arc<Mutex<>> wrapped sources)                  │
│  stop_source / clean_source / volume control                      │
├──────────────────────────────────────────────────────────────────┤
│                 sound::mixer (existing)                            │
│  mixer_source_play, mixer_buffer_data, mixer_gen_buffers, etc.    │
├──────────────────────────────────────────────────────────────────┤
│              sound::decoder (existing)                            │
│  SoundDecoder trait, WavDecoder, OggDecoder, ModDecoder, etc.     │
└──────────────────────────────────────────────────────────────────┘
```

### 1.4 Key Design Decisions

1. **Direct mixer calls.** The Rust streaming code calls `mixer_source_play()`, `mixer_buffer_data()`, etc. directly — no FFI round-trip through `audio_*` / `rust_mixer_*`. The mixer module's Rust API is the canonical interface.

2. **`Arc<Mutex<>>` replaces C globals.** All mutable shared state (sound sources, fade state, track player state) is wrapped in `Arc<Mutex<>>` or `parking_lot::Mutex<>` (matching the mixer's convention of using `parking_lot`).

3. **`Box<dyn SoundDecoder>` replaces `TFB_SoundDecoder*`.** Decoder instances are heap-allocated trait objects. Ownership is explicit: chunks own their decoders, samples borrow them.

4. **`Result<T, AudioError>` replaces C return codes.** A unified `AudioError` enum covers all error conditions. FFI shims convert to C-compatible integers.

5. **Callbacks become trait objects or closures.** The C function-pointer callbacks (`TFB_SoundCallbacks`) become a Rust trait (`StreamCallbacks`) with default no-op implementations, stored as `Option<Box<dyn StreamCallbacks + Send>>`.

6. **The streaming thread uses `std::thread` + `parking_lot::Condvar`.** Replaces `AssignTask` / `ConcludeTask` / `HibernateThread`. An `AtomicBool` signals shutdown.

---

## 2. Shared Types

### 2.1 `AudioError`

```rust
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

impl std::fmt::Display for AudioError { /* ... */ }
impl std::error::Error for AudioError {}
impl From<MixerError> for AudioError { /* ... */ }
impl From<DecodeError> for AudioError { /* ... */ }

pub type AudioResult<T> = Result<T, AudioError>;
```

### 2.2 Source Index Constants

```rust
pub const NUM_SFX_CHANNELS: usize = 5;
pub const FIRST_SFX_SOURCE: usize = 0;
pub const LAST_SFX_SOURCE: usize = FIRST_SFX_SOURCE + NUM_SFX_CHANNELS - 1; // 4
pub const MUSIC_SOURCE: usize = LAST_SFX_SOURCE + 1;                         // 5
pub const SPEECH_SOURCE: usize = MUSIC_SOURCE + 1;                            // 6
pub const NUM_SOUNDSOURCES: usize = SPEECH_SOURCE + 1;                        // 7
```

### 2.3 Volume Constants

```rust
pub const MAX_VOLUME: i32 = 255;
pub const NORMAL_VOLUME: i32 = 160;
```

---

## 3. Module Specifications

### 3.1 `sound::stream` — Streaming Engine

#### 3.1.1 Public Types

```rust
/// Audio sample — owns buffers, borrows decoder.
/// Replaces TFB_SoundSample.
pub struct SoundSample {
    decoder: Option<Box<dyn SoundDecoder>>,
    length: f32,
    buffers: Vec<usize>,            // mixer buffer handles
    num_buffers: u32,
    buffer_tags: Vec<Option<SoundTag>>,
    offset: i32,                    // initial time offset
    looping: bool,                  // stored on sample (not decoder)
    data: Option<Box<dyn Any + Send>>,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
}

/// Buffer tag for subtitle synchronization.
/// Replaces TFB_SoundTag.
pub struct SoundTag {
    buf_handle: usize,              // mixer buffer handle
    data: usize,                    // opaque payload (chunk pointer equivalent)
}

/// Stream callbacks — replaces TFB_SoundCallbacks function pointers.
pub trait StreamCallbacks: Send {
    /// Called before initial buffering. Return false to abort.
    fn on_start_stream(&mut self, sample: &mut SoundSample) -> bool { true }

    /// Called when decoder hits EOF. Return true if a new decoder was set.
    fn on_end_chunk(&mut self, sample: &mut SoundSample, buffer: usize) -> bool { false }

    /// Called when all buffers played and no more data.
    fn on_end_stream(&mut self, sample: &mut SoundSample) {}

    /// Called when a tagged buffer finishes playback.
    fn on_tagged_buffer(&mut self, sample: &mut SoundSample, tag: &SoundTag) {}

    /// Called when a buffer is queued.
    fn on_queue_buffer(&mut self, sample: &mut SoundSample, buffer: usize) {}
}

/// Per-source state — replaces TFB_SoundSource.
/// Stored in the global SoundSourceArray.
pub struct SoundSource {
    sample: Option<Arc<Mutex<SoundSample>>>,
    handle: usize,                   // mixer source handle
    stream_should_be_playing: bool,
    start_time: i32,                 // playback start timestamp
    pause_time: u32,                 // 0 = not paused
    positional_object: usize,        // opaque game object pointer (as usize for FFI)
    last_q_buf: usize,              // last queued buffer handle

    // Oscilloscope ring buffer
    sbuffer: Option<Vec<u8>>,
    sbuf_size: u32,
    sbuf_tail: u32,
    sbuf_head: u32,
    sbuf_lasttime: u32,
}

/// Fade state — protected by its own mutex, separate from sources.
struct FadeState {
    start_time: u32,
    interval: u32,
    start_volume: i32,
    delta: i32,
}
```

#### 3.1.2 Internal State

```rust
/// Global streaming engine state.
struct StreamEngine {
    // Note: sources live in SoundSourceArray (control.rs), NOT here.
    // StreamEngine only manages the decoder thread and fade state.
    fade: parking_lot::Mutex<FadeState>,
    decoder_thread: Option<JoinHandle<()>>,
    shutdown: AtomicBool,
    wake: parking_lot::Condvar,  // to wake the decoder thread when a stream starts
}

lazy_static! {
    static ref ENGINE: StreamEngine = StreamEngine::new();
}
```

#### 3.1.3 Public API

```rust
// Sample lifecycle
pub fn create_sound_sample(
    decoder: Option<Box<dyn SoundDecoder>>,
    num_buffers: u32,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
) -> AudioResult<SoundSample>;

pub fn destroy_sound_sample(sample: &mut SoundSample) -> AudioResult<()>;
pub fn set_sound_sample_data(sample: &mut SoundSample, data: Box<dyn Any + Send>);
pub fn get_sound_sample_data(sample: &SoundSample) -> Option<&(dyn Any + Send)>;
pub fn set_sound_sample_callbacks(sample: &mut SoundSample, callbacks: Option<Box<dyn StreamCallbacks + Send>>);
pub fn get_sound_sample_decoder(sample: &SoundSample) -> Option<&dyn SoundDecoder>;

// Stream control (caller must hold source mutex except where noted)
pub fn play_stream(
    sample: Arc<Mutex<SoundSample>>,
    source_index: usize,
    looping: bool,
    scope: bool,
    rewind: bool,
) -> AudioResult<()>;

pub fn stop_stream(source_index: usize) -> AudioResult<()>;
pub fn pause_stream(source_index: usize) -> AudioResult<()>;
pub fn resume_stream(source_index: usize) -> AudioResult<()>;
pub fn seek_stream(source_index: usize, pos_ms: u32) -> AudioResult<()>;
pub fn playing_stream(source_index: usize) -> bool;

// Buffer tagging
pub fn find_tagged_buffer(sample: &SoundSample, buffer: usize) -> Option<&SoundTag>;
pub fn tag_buffer(sample: &mut SoundSample, buffer: usize, data: usize) -> bool;
pub fn clear_buffer_tag(tag: &mut SoundTag);

// Scope / oscilloscope
pub fn graph_foreground_stream(
    data: &mut [i32],
    width: usize,
    height: usize,
    want_speech: bool,
) -> usize;

// Fade
pub fn set_music_stream_fade(how_long: u32, end_volume: i32) -> bool;

// Lifecycle
pub fn init_stream_decoder() -> AudioResult<()>;
pub fn uninit_stream_decoder() -> AudioResult<()>;
```

#### 3.1.4 Threading Model

- **Decoder thread:** Spawned by `init_stream_decoder()`. Runs `stream_decoder_task()` in a loop. Iterates sources `MUSIC_SOURCE..NUM_SOUNDSOURCES`, locking each source's mutex individually. Wakes via `Condvar` when a stream starts; sleeps 100ms (`Duration::from_millis(100)`) when idle.
- **Main thread:** Calls `play_stream`, `stop_stream`, etc. Must hold the source mutex (the stream functions acquire it internally via `sources[index].lock()`).
- **Fade mutex:** Separate `Mutex<FadeState>`, locked by both the decoder thread (`process_music_fade()`) and the main thread (`set_music_stream_fade()`).

#### 3.1.5 Scope Buffer

The scope buffer is a cyclic ring buffer (`Vec<u8>`) inside `SoundSource`. Write pointer (`sbuf_tail`) advances when new audio is queued; read pointer (`sbuf_head`) advances when buffers are dequeued. `graph_foreground_stream()` reads from the buffer to produce oscilloscope waveform data for the comm screen.

- Prefers speech source over music when `want_speech` is true.
- Normalizes step size to 11025 Hz reference.
- Implements AGC with 16-page running average, 8-frame sub-pages, default page max 28000, and VAD with energy threshold 100.
- Handles 8-bit (unsigned→signed conversion) and 16-bit samples.

---

### 3.2 `sound::trackplayer` — Track Player State Machine

#### 3.2.1 Public Types

```rust
/// A chunk of audio with associated subtitle and callback.
/// Replaces TFB_SoundChunk. Owns its decoder.
pub struct SoundChunk {
    decoder: Box<dyn SoundDecoder>,
    start_time: f64,                 // seconds from track start (f64 for precision in long tracks)
    tag_me: bool,
    track_num: u32,
    text: Option<String>,            // subtitle text (UTF-8, not UNICODE*)
    callback: Option<Box<dyn Fn(i32) + Send>>,
    next: Option<Box<SoundChunk>>,   // linked list via Box
}

/// Opaque handle for subtitle iteration.
/// Wraps a pointer into the chunk list.
pub struct SubtitleRef {
    // internal: NonNull<SoundChunk> or index
}
```

#### 3.2.2 Internal State

```rust
struct TrackPlayerState {
    track_count: u32,
    no_page_break: bool,
    sound_sample: Option<Arc<Mutex<SoundSample>>>,
    tracks_length: AtomicU32,        // volatile in C, AtomicU32 in Rust
    chunks_head: Option<Box<SoundChunk>>,
    chunks_tail: *mut SoundChunk,    // raw pointer into the list (non-owning)
    last_sub: *mut SoundChunk,       // last chunk with subtitle text
    cur_chunk: Option<NonNull<SoundChunk>>,     // guarded by stream_mutex
    cur_sub_chunk: Option<NonNull<SoundChunk>>, // guarded by stream_mutex
    last_track_name: String,
    dec_offset: f64,                 // accumulated decoder offset in ms (f64 for precision)
}

// Safety: TrackPlayerState is accessed from the main thread only for
// mutation (SpliceTrack, StopTrack). cur_chunk and cur_sub_chunk are
// accessed from the decoder thread under stream_mutex.
unsafe impl Send for TrackPlayerState {}

lazy_static! {
    static ref TRACK_STATE: Mutex<TrackPlayerState> = Mutex::new(TrackPlayerState::new());
}
```

#### 3.2.3 Public API

```rust
// Assembly
pub fn splice_track(
    track_name: Option<&str>,
    track_text: Option<&str>,
    timestamp: Option<&str>,
    callback: Option<Box<dyn Fn(i32) + Send>>,
) -> AudioResult<()>;

pub fn splice_multi_track(
    track_names: &[&str],
    track_text: Option<&str>,
) -> AudioResult<()>;

// Playback control
pub fn play_track() -> AudioResult<()>;
pub fn stop_track() -> AudioResult<()>;
pub fn jump_track() -> AudioResult<()>;
pub fn pause_track() -> AudioResult<()>;
pub fn resume_track() -> AudioResult<()>;
pub fn playing_track() -> u32;  // 0 = not playing, else track_num+1

// Seeking
pub fn fast_reverse_smooth() -> AudioResult<()>;
pub fn fast_forward_smooth() -> AudioResult<()>;
pub fn fast_reverse_page() -> AudioResult<()>;
pub fn fast_forward_page() -> AudioResult<()>;

// Position & subtitle queries
pub fn get_track_position(in_units: u32) -> u32;
pub fn get_track_subtitle() -> Option<String>;
pub fn get_first_track_subtitle() -> Option<SubtitleRef>;
pub fn get_next_track_subtitle(last_ref: &SubtitleRef) -> Option<SubtitleRef>;
pub fn get_track_subtitle_text(sub_ref: &SubtitleRef) -> Option<&str>;
```

#### 3.2.4 Callbacks

The track player creates a `TrackCallbacks` struct implementing `StreamCallbacks`:

```rust
struct TrackCallbacks;

impl StreamCallbacks for TrackCallbacks {
    fn on_start_stream(&mut self, sample: &mut SoundSample) -> bool { /* TRACK-CALLBACK-01..03 */ }
    fn on_end_chunk(&mut self, sample: &mut SoundSample, buffer: usize) -> bool { /* TRACK-CALLBACK-04..06 */ }
    fn on_end_stream(&mut self, sample: &mut SoundSample) { /* TRACK-CALLBACK-07 */ }
    fn on_tagged_buffer(&mut self, sample: &mut SoundSample, tag: &SoundTag) { /* TRACK-CALLBACK-08 */ }
}
```

#### 3.2.5 Subtitle Page Splitting

```rust
struct SubPage {
    text: String,
    timestamp: i32,  // ms, negative = suggested minimum
}

fn split_sub_pages(text: &str) -> Vec<SubPage>;
fn get_time_stamps(timestamp_str: &str) -> Vec<u32>;
```

`TEXT_SPEED = 80` ms/char. Minimum page time: 1000ms. Continuation prefix: `..`. Mid-word break suffix: `...`.

#### 3.2.6 Threading Model

- All list mutation (`splice_track`, `splice_multi_track`, `stop_track`) happens on the main thread under `TRACK_STATE` mutex.
- `cur_chunk` and `cur_sub_chunk` are additionally guarded by the speech source's stream mutex (accessed from the decoder thread in callbacks).
- `tracks_length` is `AtomicU32` with `Ordering::Release` (writes in `play_track()`/`stop_track()`) and `Ordering::Acquire` (reads in `get_track_position()`). The Release/Acquire pair ensures visibility of track state written before the store. (FIX: spec previously said `Relaxed`, but pseudocode correctly uses `Release`/`Acquire`.)

#### 3.2.7 Seeking

`ACCEL_SCROLL_SPEED = 300` time units. Seek functions lock the speech source mutex, compute the target offset, walk the chunk list, and either seek the decoder or restart playback.

---

### 3.3 `sound::music` — Music Playback API

#### 3.3.1 Public Types

```rust
/// Opaque music reference. Wraps Arc<Mutex<SoundSample>> for shared ownership.
/// Replaces MUSIC_REF (TFB_SoundSample**).
///
/// FIX ISSUE-FFI-05: Uses Arc<Mutex<SoundSample>> instead of raw pointer.
/// This prevents double-free and use-after-free bugs inherent in the C raw
/// pointer approach. The FFI layer converts raw pointers to/from Arc at the
/// boundary using Arc::into_raw / Arc::from_raw.
pub struct MusicRef(Arc<parking_lot::Mutex<SoundSample>>);

// For FFI: C code stores an opaque *mut c_void handle.
// get_music_data: creates Arc, calls Arc::into_raw → *mut c_void to C.
// plr_play_song: receives *mut c_void, calls Arc::increment_strong_count +
//   Arc::from_raw to reconstruct without taking ownership.
// release_music_data: receives *mut c_void, calls Arc::from_raw to reclaim
//   ownership (drops the Arc, decrementing refcount).
```

#### 3.3.2 Internal State

```rust
struct MusicState {
    cur_music_ref: Option<MusicRef>,
    cur_speech_ref: Option<MusicRef>,
    music_volume: i32,               // 0..255, default NORMAL_VOLUME
    music_volume_scale: f32,         // from options
}

lazy_static! {
    static ref MUSIC_STATE: Mutex<MusicState> = Mutex::new(MusicState::new());
}
```

#### 3.3.3 Public API

```rust
// Playback
pub fn plr_play_song(music_ref: MusicRef, continuous: bool, priority: i32) -> AudioResult<()>;
pub fn plr_stop(music_ref: MusicRef) -> AudioResult<()>;
pub fn plr_playing(music_ref: MusicRef) -> bool;
pub fn plr_seek(music_ref: MusicRef, pos: u32) -> AudioResult<()>;
pub fn plr_pause(music_ref: MusicRef) -> AudioResult<()>;
pub fn plr_resume(music_ref: MusicRef) -> AudioResult<()>;

// Speech-as-music
pub fn snd_play_speech(speech_ref: MusicRef) -> AudioResult<()>;
pub fn snd_stop_speech() -> AudioResult<()>;

// Loading
pub fn get_music_data(filename: &str) -> AudioResult<MusicRef>;
pub fn release_music_data(music_ref: MusicRef) -> AudioResult<()>;
pub fn check_music_res_name(filename: &str) -> Option<String>;

// Volume
pub fn set_music_volume(volume: i32);
pub fn fade_music(end_vol: i32, time_interval: i32) -> u32;
```

#### 3.3.4 Threading Model

All `plr_*` functions lock `soundSource[MUSIC_SOURCE]`'s stream mutex internally. `snd_play_speech` / `snd_stop_speech` lock `soundSource[SPEECH_SOURCE]`'s stream mutex. `MUSIC_STATE` mutex is held briefly to read/write `cur_music_ref`.

---

### 3.4 `sound::sfx` — Sound Effects

#### 3.4.1 Public Types

```rust
/// Position for spatial audio.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SoundPosition {
    pub positional: bool,
    pub x: i32,
    pub y: i32,
}

impl SoundPosition {
    pub const NON_POSITIONAL: SoundPosition = SoundPosition {
        positional: false, x: 0, y: 0,
    };
}

/// Sound bank — a collection of pre-decoded SFX samples.
/// Replaces STRING_TABLE of TFB_SoundSample**.
pub struct SoundBank {
    samples: Vec<Option<SoundSample>>,
}

/// Handle to a specific sound effect within a bank.
/// Replaces SOUND / SOUNDPTR.
pub type SoundHandle = usize;
```

#### 3.4.2 Internal State

```rust
struct SfxState {
    opt_stereo_sfx: bool,
}

lazy_static! {
    static ref SFX_STATE: Mutex<SfxState> = Mutex::new(SfxState { opt_stereo_sfx: false });
}
```

#### 3.4.3 Public API

```rust
pub fn play_channel(
    channel: usize,
    sound_bank: &SoundBank,
    sound_index: SoundHandle,
    pos: SoundPosition,
    positional_object: usize,
    priority: i32,
) -> AudioResult<()>;

pub fn stop_channel(channel: usize, priority: i32) -> AudioResult<()>;
pub fn channel_playing(channel: usize) -> bool;
pub fn set_channel_volume(channel: usize, volume: i32, priority: i32);
pub fn check_finished_channels();

// Positional audio
pub fn update_sound_position(channel: usize, pos: SoundPosition);
pub fn get_positional_object(channel: usize) -> usize;
pub fn set_positional_object(channel: usize, obj: usize);

// Loading
pub fn get_sound_bank_data(filename: &str, data: &[u8]) -> AudioResult<SoundBank>;
pub fn release_sound_bank_data(bank: SoundBank) -> AudioResult<()>;
```

#### 3.4.4 Positional Audio

`ATTENUATION = 160.0f32`. `MIN_DISTANCE = 0.5f32`. Non-positional sounds at `(0, 0, -1)`.

#### 3.4.5 Threading Model

SFX channels are main-thread only. The decoder thread never touches sources `0..LAST_SFX_SOURCE`. No mutex required for SFX-specific state beyond the source array.

---

### 3.5 `sound::control` — Volume & Global Control

#### 3.5.1 Public Types

```rust
/// The global sound source array.
/// Replaces the C `soundSource[NUM_SOUNDSOURCES]` global.
pub struct SoundSourceArray {
    sources: [Mutex<SoundSource>; NUM_SOUNDSOURCES],
}
```

#### 3.5.2 Internal State

```rust
struct VolumeState {
    music_volume: i32,
    music_volume_scale: f32,
    sfx_volume_scale: f32,
    speech_volume_scale: f32,
}

lazy_static! {
    pub(crate) static ref SOURCES: SoundSourceArray = SoundSourceArray::new();
    static ref VOLUME: Mutex<VolumeState> = Mutex::new(VolumeState::new());
}
```

#### 3.5.3 Public API

```rust
// Source management
pub fn stop_source(source_index: usize) -> AudioResult<()>;
pub fn clean_source(source_index: usize) -> AudioResult<()>;
pub fn stop_sound();

// Volume
pub fn set_sfx_volume(volume: f32);
pub fn set_speech_volume(volume: f32);

// Queries
pub fn sound_playing() -> bool;
pub fn wait_for_sound_end(channel: Option<usize>);

// Lifecycle
pub fn init_sound() -> AudioResult<()>;
pub fn uninit_sound();
```

#### 3.5.4 Threading Model

`SOURCES` is the shared state between the main thread and the decoder thread. Each source has its own `Mutex<SoundSource>`. Volume state is main-thread-only (set from options menu / game logic).

---

### 3.6 `sound::fileinst` — File-Based Loading

#### 3.6.1 Internal State

```rust
struct FileInstState {
    cur_resfile_name: Option<String>,
}

lazy_static! {
    static ref FILE_STATE: Mutex<FileInstState> = Mutex::new(FileInstState { cur_resfile_name: None });
}
```

#### 3.6.2 Public API

```rust
pub fn load_sound_file(filename: &str) -> AudioResult<SoundBank>;
pub fn load_music_file(filename: &str) -> AudioResult<MusicRef>;
pub fn destroy_sound(bank: SoundBank) -> AudioResult<()>;
pub fn destroy_music(music_ref: MusicRef) -> AudioResult<()>;
```

#### 3.6.3 Threading Model

File loading is main-thread-only. The `cur_resfile_name` guard prevents concurrent loads (returns `AudioError::ConcurrentLoad`).

---

### 3.7 `sound::heart_ffi` — C FFI Boundary

Thin shim module. Every function is `#[no_mangle] pub extern "C" fn`. No logic beyond pointer conversion and error translation. See §5 for full listing.

---

## 4. EARS Requirements

### STREAM — Streaming Audio Engine

#### STREAM-INIT: Initialization and Shutdown

**STREAM-INIT-01:** The streaming system shall create a `Mutex<FadeState>` for music fade state protection upon initialization.

**STREAM-INIT-02:** The streaming system shall spawn a background decoder thread via `std::thread::Builder::new().name("audio stream decoder".into()).spawn()` upon initialization.

**STREAM-INIT-03:** The streaming system shall return `Err(AudioError::NotInitialized)` if the decoder thread fails to spawn.

**STREAM-INIT-04:** When the streaming system is uninitialized, the system shall set the `shutdown: AtomicBool` to `true` (with `Ordering::Release`), notify the decoder thread's `Condvar`, and call `JoinHandle::join()` to wait for completion.

**STREAM-INIT-05:** When the streaming system is uninitialized, the `FadeState` mutex shall be dropped as part of normal Rust drop semantics (no explicit destruction needed).

**STREAM-INIT-06:** When the decoder thread `JoinHandle` is `None` during uninitialization, the system shall skip thread termination without error.

**STREAM-INIT-07:** When the `FadeState` has already been dropped during uninitialization, the system shall not panic (idempotent shutdown).

#### STREAM-PLAY: Stream Playback Control

**STREAM-PLAY-01:** When `play_stream()` is called, the system shall first call `stop_stream(source_index)` on the given source.

**STREAM-PLAY-02:** When `play_stream()` is called with a `SoundSample` whose inner `Arc` strong count would drop to zero (logically empty), the system shall return `Err(AudioError::InvalidSample)`.

**STREAM-PLAY-03:** When `play_stream()` is called and the `StreamCallbacks::on_start_stream()` method exists and returns `false`, the system shall return `Err(AudioError::EndOfStream)` without starting playback.

**STREAM-PLAY-04:** When `play_stream()` is called, the system shall clear all buffer tags by setting each element of `sample.buffer_tags` to `None`.

**STREAM-PLAY-05:** When `play_stream()` is called with `rewind=true`, the system shall call `decoder.seek(0)` (or equivalent rewind) on the sample's decoder.

**STREAM-PLAY-06:** When `play_stream()` is called with `rewind=false`, the system shall compute the time offset as `sample.offset + (decoder.get_time() * ONE_SECOND as f32) as i32` where `ONE_SECOND` is the game time unit constant.

**STREAM-PLAY-07:** The system shall store the sample in `source.sample` as `Some(Arc::clone(&sample))`, set `decoder.looping` via the decoder trait, and call `mixer_source_i(handle, SourceProp::Looping, 0)` (looping always false at mixer level).

**STREAM-PLAY-08:** When `play_stream()` is called with `scope=true`, the system shall allocate `source.sbuffer = Some(vec![0u8; (num_buffers * buffer_size + PAD_SCOPE_BYTES) as usize])` where `PAD_SCOPE_BYTES = 256`.

**STREAM-PLAY-09:** The system shall pre-fill up to `num_buffers` audio buffers by calling `decoder.decode(&mut buf)`, uploading each via `mixer_buffer_data(buffer_handle, format, &data, freq, mixer_freq, mixer_format)`, and queuing each via `mixer_source_queue_buffers(source_handle, &[buffer_handle])`.

**STREAM-PLAY-10:** When a buffer is queued and `sample.callbacks` is `Some`, the system shall call `callbacks.on_queue_buffer(sample, buffer_handle)`.

**STREAM-PLAY-11:** When the decoder returns `Err(DecodeError::EndOfFile)` during pre-filling, the system shall call `callbacks.on_end_chunk(sample, buffer_handle)`. Where the callback returns `true`, the system shall continue with the updated decoder from `sample.decoder`.

**STREAM-PLAY-12:** When `decoder.decode()` returns `Ok(0)` during pre-filling, the system shall stop pre-filling and proceed to playback.

**STREAM-PLAY-13:** After pre-filling, the system shall set `source.sbuf_lasttime`, compute `source.start_time = get_time_counter() - offset`, set `source.pause_time = 0`, set `source.stream_should_be_playing = true`, and call `mixer_source_play(source.handle)`.

**STREAM-PLAY-14:** When `stop_stream()` is called, the system shall call `stop_source(source_index)`, set `source.stream_should_be_playing = false`, set `source.sample = None`, drop the scope buffer (`source.sbuffer = None`), and zero `source.sbuf_size`, `source.sbuf_tail`, `source.sbuf_head`, `source.sbuf_lasttime`, and `source.pause_time`.

**STREAM-PLAY-15:** When `pause_stream()` is called, the system shall set `source.stream_should_be_playing = false`, record `source.pause_time = get_time_counter()` (only if `source.pause_time == 0`), and call `mixer_source_pause(source.handle)`.

**STREAM-PLAY-16:** When `resume_stream()` is called and `source.pause_time != 0`, the system shall adjust `source.start_time += get_time_counter() - source.pause_time`.

**STREAM-PLAY-17:** When `resume_stream()` is called, the system shall set `source.pause_time = 0`, set `source.stream_should_be_playing = true`, and call `mixer_source_play(source.handle)`.

**STREAM-PLAY-18:** When `seek_stream()` is called, the system shall call `mixer_source_stop(source.handle)`, seek the decoder to `pos_ms` milliseconds via `decoder.seek(pcm_pos)`, and restart playback by calling `play_stream()` with the same looping and scope settings.

**STREAM-PLAY-19:** When `seek_stream()` is called and `source.sample` is `None`, the system shall return `Err(AudioError::InvalidSample)`.

**STREAM-PLAY-20:** `playing_stream(source_index)` shall acquire the source mutex, read `source.stream_should_be_playing`, release the mutex, and return the value.

#### STREAM-THREAD: Background Decoder Thread

**STREAM-THREAD-01:** While `shutdown.load(Ordering::Acquire)` is `false`, the decoder thread shall continuously loop.

**STREAM-THREAD-02:** On each iteration, the decoder thread shall call `process_music_fade()` to advance any active music fade.

**STREAM-THREAD-03:** On each iteration, the decoder thread shall iterate over source indices `MUSIC_SOURCE..NUM_SOUNDSOURCES` (inclusive of both `MUSIC_SOURCE` and `SPEECH_SOURCE`).

**STREAM-THREAD-04:** For each source, the decoder thread shall acquire `sources[index].lock()` before checking or modifying any source state, and release it after processing.

**STREAM-THREAD-05:** Where a source has `sample == None`, no decoder in the sample, `stream_should_be_playing == false`, or the decoder's last error was not recoverable, the decoder thread shall skip that source.

**STREAM-THREAD-06:** Where no active streams exist in an iteration, the decoder thread shall wait on the `Condvar` with a timeout of `Duration::from_millis(100)`.

**STREAM-THREAD-07:** Where at least one active stream exists, the decoder thread shall call `std::thread::yield_now()` rather than sleeping.

**STREAM-THREAD-08:** When the decoder thread exits its loop (shutdown signaled), it shall return normally (no explicit cleanup beyond Rust's drop semantics).

#### STREAM-PROCESS: Per-Source Stream Processing

**STREAM-PROCESS-01:** The system shall query `mixer_get_source_i(source.handle, SourceProp::BuffersProcessed)` and `mixer_get_source_i(source.handle, SourceProp::BuffersQueued)` from the mixer.

**STREAM-PROCESS-02:** When `processed == 0` and `mixer_get_source_i(handle, SourceProp::SourceState) != Ok(SourceState::Playing as i32)`: where `queued == 0` and the decoder's error state matches `DecodeError::EndOfFile`, the system shall set `source.stream_should_be_playing = false` and call `callbacks.on_end_stream(sample)` if callbacks are `Some`.

**STREAM-PROCESS-03:** When `processed == 0` and the source is not playing but `queued > 0`, the system shall log a buffer underrun warning via `log::warn!()` and call `mixer_source_play(source.handle)` to restart playback.

**STREAM-PROCESS-04:** For each processed buffer, the system shall unqueue it via `mixer_source_unqueue_buffers(source.handle, 1)`.

**STREAM-PROCESS-05:** When `mixer_source_unqueue_buffers()` returns `Err(_)`, the system shall log the error and break out of the processing loop.

**STREAM-PROCESS-06:** When callbacks are `Some` and the unqueued buffer has a matching tag found via `find_tagged_buffer(sample, buffer_handle)`, the system shall call `callbacks.on_tagged_buffer(sample, tag)`.

**STREAM-PROCESS-07:** When `source.sbuffer.is_some()`, the system shall call `remove_scope_data(source, buffer_handle)` for each unqueued buffer.

**STREAM-PROCESS-08:** When the decoder returns `Err(DecodeError::EndOfFile)` and either no callbacks exist or `callbacks.on_end_chunk()` returns `false`, the system shall set an internal `end_chunk_failed` flag and skip further EOF handling in this iteration.

**STREAM-PROCESS-09:** When the decoder returns `Err(DecodeError::EndOfFile)` and `callbacks.on_end_chunk()` returns `true`, the system shall re-read `sample.decoder` to get the new decoder reference.

**STREAM-PROCESS-10:** When the decoder has a non-EOF error (any `DecodeError` variant other than `EndOfFile`), the system shall skip that buffer without attempting to decode.

**STREAM-PROCESS-11:** The system shall decode new audio via `decoder.decode(&mut buf)` for each recycled buffer.

**STREAM-PROCESS-12:** When `decoder.decode()` returns `Err(DecodeError::DecoderError(_))`, the system shall log the error, set `source.stream_should_be_playing = false`, and skip the buffer.

**STREAM-PROCESS-13:** When `decoder.decode()` returns `Ok(0)` (zero decoded bytes), the system shall skip the buffer.

**STREAM-PROCESS-14:** The system shall upload decoded data via `mixer_buffer_data()` and queue the buffer via `mixer_source_queue_buffers()`, checking each `Result` for errors and logging failures.

**STREAM-PROCESS-15:** The system shall set `source.last_q_buf = buffer_handle` and call `callbacks.on_queue_buffer(sample, buffer_handle)` if callbacks are `Some`.

**STREAM-PROCESS-16:** When `source.sbuffer.is_some()`, the system shall call `add_scope_data(source, decoded_bytes)` after queuing.

#### STREAM-SAMPLE: Sound Sample Management

**STREAM-SAMPLE-01:** `create_sound_sample()` shall construct a `SoundSample` with the given decoder, allocate `num_buffers` mixer buffer handles via `mixer_gen_buffers(num_buffers)`, initialize `buffer_tags` as `vec![None; num_buffers as usize]`, and store callbacks if provided. It shall return `Err(AudioError::MixerError(_))` if buffer generation fails.

**STREAM-SAMPLE-02:** `destroy_sound_sample()` shall delete mixer buffers via `mixer_delete_buffers(&sample.buffers)`, clear `sample.buffers`, clear `sample.buffer_tags`, and drop callbacks. The decoder is NOT dropped (ownership belongs to the caller or chunk).

**STREAM-SAMPLE-03:** `set_sound_sample_data()` shall store user-defined data in `sample.data` as `Some(data)`, and `get_sound_sample_data()` shall return `sample.data.as_deref()`.

**STREAM-SAMPLE-04:** `set_sound_sample_callbacks()` shall replace `sample.callbacks` with the provided value (or `None`).

**STREAM-SAMPLE-05:** `get_sound_sample_decoder()` shall return `sample.decoder.as_deref()`.

#### STREAM-TAG: Buffer Tagging

**STREAM-TAG-01:** `find_tagged_buffer()` shall iterate `sample.buffer_tags` and return `Some(&tag)` for the first `Some(tag)` where `tag.buf_handle == buffer`. Where `buffer_tags` is empty or no match exists, it shall return `None`.

**STREAM-TAG-02:** `tag_buffer()` shall find the first `None` slot or a slot whose `buf_handle` matches `buffer` in `sample.buffer_tags`. Where no slot is available, it shall return `false`. Otherwise it shall set the slot to `Some(SoundTag { buf_handle: buffer, data })` and return `true`.

**STREAM-TAG-03:** `clear_buffer_tag()` shall set the tag to its cleared state (the containing `Option` in `buffer_tags` shall be set to `None`).

#### STREAM-SCOPE: Oscilloscope Ring Buffer

**STREAM-SCOPE-01:** `add_scope_data()` shall copy decoded audio bytes from the decoder buffer to the scope ring buffer at `sbuf_tail`, wrapping modulo `sbuf_size` when the tail reaches the end.

**STREAM-SCOPE-02:** `remove_scope_data()` shall advance `sbuf_head` by the byte size of the given audio buffer (queried via `mixer_get_buffer_i(buffer, BufferProp::Size)`), wrapping modulo `sbuf_size`. It shall update `sbuf_lasttime` to `get_time_counter()`.

**STREAM-SCOPE-03:** `graph_foreground_stream()` shall prefer the speech source (`SPEECH_SOURCE`) when `want_speech` is true and the speech source has a non-`None` decoder. Otherwise it shall use the music source (`MUSIC_SOURCE`).

**STREAM-SCOPE-04:** When no playable stream exists (no sample, no decoder, no scope buffer, or `sbuf_size == 0`), `graph_foreground_stream()` shall return 0.

**STREAM-SCOPE-05:** `graph_foreground_stream()` shall compute the step size normalized to 11025 Hz: step 1 for speech, step 4 for music, scaled by `decoder.frequency() as f32 / 11025.0`, minimum 1, multiplied by `bytes_per_full_sample` (format's `bytes_per_sample()`).

**STREAM-SCOPE-06:** `graph_foreground_stream()` shall compute the current read position as `sbuf_head + delta` where `delta = (get_time_counter() - sbuf_lasttime) as f32 * frequency as f32 * bytes_per_full_sample as f32 / ONE_SECOND as f32`, clamped to `[0, sbuf_size]`.

**STREAM-SCOPE-07:** `graph_foreground_stream()` shall read samples from the scope buffer, scale by `avg_amp / target_amp` (where `target_amp = height / 4`), center at `height / 2`, clamp to `[0, height - 1]`, and write results to the output `data` slice.

**STREAM-SCOPE-08:** A `read_sound_sample()` helper shall convert 8-bit unsigned samples to signed 16-bit range by `((value as i16) - 128) << 8`. For 16-bit samples it shall return the `i16` value directly.

**STREAM-SCOPE-09:** `graph_foreground_stream()` shall implement Automatic Gain Control (AGC) using a 16-element (`AGC_PAGE_COUNT = 16`) running average of 8-frame (`AGC_FRAME_COUNT = 8`) maximum amplitude pages, with a default page maximum of `DEF_PAGE_MAX = 28000`.

**STREAM-SCOPE-10:** The AGC shall include Voice Activity Detection: when the per-frame signal energy is below `VAD_MIN_ENERGY: i32 = 100`, the frame shall not contribute to the running average.

**STREAM-SCOPE-11:** When multi-channel audio is detected (`decoder.format().channels() > 1`), `graph_foreground_stream()` shall sum both channel samples for each step position.

#### STREAM-FADE: Music Fade

**STREAM-FADE-01:** `set_music_stream_fade()` shall acquire the `FadeState` mutex, set `start_time = get_time_counter()`, `interval = how_long`, `start_volume` to the current `music_volume` (read from `MUSIC_STATE`), and `delta = end_volume - start_volume`.

**STREAM-FADE-02:** When `how_long == 0` after any clamping, `set_music_stream_fade()` shall return `false` (reject the fade).

**STREAM-FADE-03:** While a fade is active (`fade.interval != 0`), `process_music_fade()` shall compute elapsed time clamped to `[0, fade.interval]`, linearly interpolate volume as `start_volume + delta * elapsed / interval` (integer arithmetic), and call `set_music_volume(interpolated_volume)`.

**STREAM-FADE-04:** When elapsed time reaches or exceeds `fade.interval`, `process_music_fade()` shall set `fade.interval = 0` to end the fade.

**STREAM-FADE-05:** When no fade is active (`fade.interval == 0`), `process_music_fade()` shall return immediately without acquiring any additional locks.

---

### TRACK — Track Player

#### TRACK-ASSEMBLE: Track Assembly

**TRACK-ASSEMBLE-01:** `splice_track()` shall split subtitle text into pages at `\r\n` boundaries using `split_sub_pages()`.

**TRACK-ASSEMBLE-02:** `split_sub_pages()` shall calculate a display timestamp for each page as `text.chars().count() as i32 * TEXT_SPEED` where `TEXT_SPEED: i32 = 80` ms/char, with a minimum of `1000` ms.

**TRACK-ASSEMBLE-03:** `split_sub_pages()` shall prepend `".."` to continuation pages and append `"..."` to pages ending at a mid-word break (the character before the break is not whitespace or punctuation).

**TRACK-ASSEMBLE-04:** When `splice_track()` is called with `track_name == None`, the system shall append subtitle text to the last existing track's subtitle. The first page shall be appended to `last_sub.text` via `String::push_str()`.

**TRACK-ASSEMBLE-05:** When `splice_track()` is called with `track_name == None` and `track_count == 0`, the system shall log a warning via `log::warn!()` and return `Ok(())`.

**TRACK-ASSEMBLE-06:** When `splice_track()` is called with `track_text == None`, the system shall return `Ok(())` immediately.

**TRACK-ASSEMBLE-07:** When `splice_track()` is called with `track_name == Some(name)`, the system shall create a new track with a decoder loaded via the `SoundDecoder` trait from the content directory. On the first call, it shall also create the `sound_sample` via `create_sound_sample()` with `num_buffers = 8` and a `TrackCallbacks` instance.

**TRACK-ASSEMBLE-08:** `splice_track()` shall load decoder segments with `buffer_size = 4096`, `start_time = dec_offset` (accumulated), and `run_time = timestamp` for each page. `dec_offset` shall accumulate as `decoder.length() * 1000.0` ms.

**TRACK-ASSEMBLE-09:** When timestamps are provided via the `timestamp` parameter, `splice_track()` shall parse them via `get_time_stamps()` (comma/CR/LF separated `u32` values, skipping zeros) and use them instead of calculated page timestamps.

**TRACK-ASSEMBLE-10:** `splice_track()` shall always make the last page's timestamp negative (by negating the `i32` value), indicating it is a suggested minimum rather than a hard cutoff.

**TRACK-ASSEMBLE-11:** When `state.no_page_break` is `true` and tracks exist, `splice_track()` shall append the first page to the last subtitle's text via `String::push_str()` instead of creating a new track.

**TRACK-ASSEMBLE-12:** Each `SoundChunk` created by `splice_track()` shall have `tag_me = true` (unless `no_page_break`), its subtitle text as `Option<String>`, callback as `Option<Box<dyn Fn(i32) + Send>>`, and appropriate `track_num`.

**TRACK-ASSEMBLE-13:** `splice_track()` shall set `state.no_page_break = false` after processing each chunk.

**TRACK-ASSEMBLE-14:** `get_time_stamps()` shall parse a `&str` of comma/CR/LF-separated unsigned integers, skip zero values, and return a `Vec<u32>`.

**TRACK-ASSEMBLE-15:** `splice_multi_track()` shall load up to `MAX_MULTI_TRACKS: usize = 20` decoders with `buffer_size = 32768` and `run_time = -3 * TEXT_SPEED`, fully pre-decode each via `decoder.decode_all()`, and append each as a new `SoundChunk` sharing the current `track_num`.

**TRACK-ASSEMBLE-16:** `splice_multi_track()` shall append `track_text` to `last_sub.text` (via `String::push_str()`) and set `state.no_page_break = true`.

**TRACK-ASSEMBLE-17:** When `splice_multi_track()` is called before any `splice_track()` (i.e., `track_count == 0`), the system shall log a warning and return `Err(AudioError::InvalidSample)`.

**TRACK-ASSEMBLE-18:** `SoundChunk::new()` shall construct a zeroed chunk with the given decoder and `start_time`, with `tag_me = false`, `track_num = 0`, `text = None`, `callback = None`, `next = None`.

**TRACK-ASSEMBLE-19:** When a `SoundChunk` linked list is dropped (via `Drop` on the head `Box<SoundChunk>`), each chunk's decoder shall be dropped (Rust ownership semantics — the `Box<dyn SoundDecoder>` field is dropped automatically), and each chunk's `text: Option<String>` shall be dropped automatically.

#### TRACK-PLAY: Track Playback Control

**TRACK-PLAY-01:** `play_track()` shall compute `tracks_length` as the end time of the last chunk (via `tracks_end_time()`), store it in the `AtomicU32` (with `Ordering::Release`), set `cur_chunk` to point to `chunks_head`, and call `play_stream(sound_sample, SPEECH_SOURCE, false, true, true)`.

**TRACK-PLAY-02:** When `play_track()` is called and `state.sound_sample` is `None`, the system shall return `Ok(())` immediately.

**TRACK-PLAY-03:** `stop_track()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()`, call `stop_stream(SPEECH_SOURCE)`, reset `track_count = 0`, `tracks_length.store(0, Ordering::Release)`, `cur_chunk = None`, `cur_sub_chunk = None`, release the source lock.

**TRACK-PLAY-04:** `stop_track()` shall drop the entire chunk list (by setting `chunks_head = None`, which triggers recursive `Drop`), set `chunks_tail` and `last_sub` to null, freeing all decoders and subtitle text.

**TRACK-PLAY-05:** `stop_track()` shall set `sound_sample.decoder = None` before calling `destroy_sound_sample()` to prevent double-free (decoders are owned by chunks, not the sample).

**TRACK-PLAY-06:** `jump_track()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()` and call `seek_track(tracks_length.load(Ordering::Acquire) + 1)` to advance past the end of all tracks.

**TRACK-PLAY-07:** When `jump_track()` is called and `state.sound_sample` is `None`, the system shall return `Ok(())` immediately.

**TRACK-PLAY-08:** `pause_track()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()` and call `pause_stream(SPEECH_SOURCE)`.

**TRACK-PLAY-09:** `resume_track()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()`, verify `cur_chunk.is_some()` and `mixer_get_source_i(handle, SourceProp::SourceState) == Ok(SourceState::Paused as i32)`, then call `resume_stream(SPEECH_SOURCE)`.

**TRACK-PLAY-10:** `playing_track()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()`, return `cur_chunk.map(|c| c.track_num + 1).unwrap_or(0)`. Where `state.sound_sample` is `None`, it shall return `0`.

#### TRACK-SEEK: Seeking and Navigation

**TRACK-SEEK-01:** `seek_track()` shall clamp the offset to `0..=tracks_length.load(Ordering::Acquire) + 1`.

**TRACK-SEEK-02:** `seek_track()` shall set `source.start_time = get_time_counter() as i32 - offset as i32` on the speech source.

**TRACK-SEEK-03:** `seek_track()` shall walk the chunk list from `chunks_head` to find the chunk whose cumulative end time exceeds `offset`, tracking the last chunk with `tag_me == true`.

**TRACK-SEEK-04:** When a chunk is found at the seek position, `seek_track()` shall seek the chunk's decoder to the correct position within the chunk (in milliseconds), set `sample.decoder = Some(/* borrowed reference */)`, and call `do_track_tag()` on the last tagged chunk.

**TRACK-SEEK-05:** When the offset exceeds all chunks, `seek_track()` shall call `stop_stream(SPEECH_SOURCE)` and set `cur_chunk = None` and `cur_sub_chunk = None`.

**TRACK-SEEK-06:** `get_current_track_pos()` shall return `(get_time_counter() as i32 - source.start_time).clamp(0, tracks_length as i32) as u32`.

**TRACK-SEEK-07:** `fast_reverse_smooth()` shall subtract `ACCEL_SCROLL_SPEED: u32 = 300` from the current position and call `seek_track()`. Where the stream was not playing (`stream_should_be_playing == false`), it shall restart playback via `play_stream()`.

**TRACK-SEEK-08:** `fast_forward_smooth()` shall add `ACCEL_SCROLL_SPEED` to the current position and call `seek_track()`.

**TRACK-SEEK-09:** `fast_reverse_page()` shall find the previous page via `find_prev_page(cur_sub_chunk)` and restart playback from that chunk via `play_stream()`. Where no previous page exists, it shall do nothing (return `Ok(())`).

**TRACK-SEEK-10:** `fast_forward_page()` shall find the next page via `find_next_page(cur_sub_chunk)` and restart playback from that chunk. Where no next page exists, it shall call `seek_track(tracks_length + 1)`.

**TRACK-SEEK-11:** `find_next_page()` shall iterate `chunk.next` until it finds a chunk with `tag_me == true`. Where the input is `None` or no tagged successor exists, it shall return `None`.

**TRACK-SEEK-12:** `find_prev_page()` shall walk from `chunks_head` to find the last chunk before `cur` with `tag_me == true`, defaulting to `chunks_head`. Where `cur == chunks_head`, it shall return a reference to `chunks_head`.

**TRACK-SEEK-13:** All seek and navigation functions shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()` before modifying shared state.

#### TRACK-CALLBACK: Stream Callbacks

**TRACK-CALLBACK-01:** `TrackCallbacks::on_start_stream()` shall acquire `TRACK_STATE.lock()`, verify the sample matches `state.sound_sample` (by `Arc::ptr_eq`) and `state.cur_chunk.is_some()`. Where either check fails, it shall return `false`.

**TRACK-CALLBACK-02:** `TrackCallbacks::on_start_stream()` shall set `sample.decoder` to `cur_chunk.decoder` (by reference/clone) and `sample.offset` to `(cur_chunk.start_time * ONE_SECOND as f32) as i32`.

**TRACK-CALLBACK-03:** When `cur_chunk.tag_me` is `true` in `on_start_stream()`, the system shall call `do_track_tag(cur_chunk)`.

**TRACK-CALLBACK-04:** `TrackCallbacks::on_end_chunk()` shall return `false` when the sample doesn't match `state.sound_sample` (by `Arc::ptr_eq`), or when `cur_chunk` is `None` or `cur_chunk.next` is `None`.

**TRACK-CALLBACK-05:** `TrackCallbacks::on_end_chunk()` shall advance `cur_chunk` to `cur_chunk.next` (via the `NonNull` pointer), set `sample.decoder` to the new chunk's decoder, and call `decoder.seek(0)` to rewind.

**TRACK-CALLBACK-06:** When the new chunk in `on_end_chunk()` has `tag_me == true`, the system shall call `tag_buffer(sample, buffer, chunk_as_usize)` where `chunk_as_usize` is the raw pointer to the chunk cast to `usize`.

**TRACK-CALLBACK-07:** `TrackCallbacks::on_end_stream()` shall set `cur_chunk = None` and `cur_sub_chunk = None` under the `TRACK_STATE` mutex.

**TRACK-CALLBACK-08:** `TrackCallbacks::on_tagged_buffer()` shall extract the `*mut SoundChunk` from `tag.data` (cast from `usize`), call `clear_buffer_tag()`, and call `do_track_tag()` on the extracted chunk.

**TRACK-CALLBACK-09:** `do_track_tag()` shall acquire the speech source mutex, call `chunk.callback.as_ref().map(|cb| cb(0))` if a callback exists, and set `cur_sub_chunk = Some(NonNull::from(chunk))`.

#### TRACK-SUBTITLE: Subtitle Access

**TRACK-SUBTITLE-01:** `get_track_subtitle()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()` and return `cur_sub_chunk.and_then(|c| c.text.clone())`. Where `state.sound_sample` is `None` or `cur_sub_chunk` is `None`, it shall return `None`.

**TRACK-SUBTITLE-02:** `get_first_track_subtitle()` shall return a `SubtitleRef` pointing to `chunks_head`, or `None` if `chunks_head` is `None`.

**TRACK-SUBTITLE-03:** `get_next_track_subtitle()` shall call `find_next_page()` on the referenced chunk and return the resulting `SubtitleRef`, or `None`.

**TRACK-SUBTITLE-04:** `get_track_subtitle_text()` shall return `sub_ref.text.as_deref()`, or `None` if the ref is invalid.

#### TRACK-POSITION: Position Tracking

**TRACK-POSITION-01:** `get_track_position(in_units)` shall return `in_units * offset / tracks_length` (integer arithmetic). Where `state.sound_sample` is `None` or `tracks_length == 0`, it shall return `0`.

**TRACK-POSITION-02:** `get_track_position()` shall load `tracks_length` from the `AtomicU32` (with `Ordering::Acquire`) into a local variable before use to avoid division-by-zero from concurrent modification.

---

### MUSIC — Music Playback API

#### MUSIC-PLAY: Playback Control

**MUSIC-PLAY-01:** `plr_play_song()` shall acquire `SOURCES.sources[MUSIC_SOURCE].lock()`, call `play_stream(sample, MUSIC_SOURCE, continuous, true, true)` (scope always true, rewind always true), release the lock, and store `music_ref` as `MUSIC_STATE.cur_music_ref`.

**MUSIC-PLAY-02:** When `plr_play_song()` is called with a null or invalid `MusicRef`, the system shall return `Err(AudioError::InvalidSample)`.

**MUSIC-PLAY-03:** The `priority` parameter of `plr_play_song()` shall be accepted but ignored.

**MUSIC-PLAY-04:** `plr_stop()` shall stop the music stream and set `MUSIC_STATE.cur_music_ref = None` when the provided `MusicRef` matches `cur_music_ref` (by pointer equality) or is the wildcard value (`MusicRef(!0usize as *mut _)`).

**MUSIC-PLAY-05:** `plr_playing()` shall return `true` when `MUSIC_STATE.cur_music_ref` is `Some`, the ref matches (or is wildcard), and `playing_stream(MUSIC_SOURCE)` returns `true`.

**MUSIC-PLAY-06:** `plr_seek()` shall call `seek_stream(MUSIC_SOURCE, pos)` under the music source mutex when the ref matches or is wildcard.

**MUSIC-PLAY-07:** `plr_pause()` shall call `pause_stream(MUSIC_SOURCE)` under the music source mutex when the ref matches or is wildcard.

**MUSIC-PLAY-08:** `plr_resume()` shall call `resume_stream(MUSIC_SOURCE)` under the music source mutex when the ref matches or is wildcard.

#### MUSIC-SPEECH: Speech-as-Music

**MUSIC-SPEECH-01:** `snd_play_speech()` shall acquire `SOURCES.sources[SPEECH_SOURCE].lock()` and call `play_stream(sample, SPEECH_SOURCE, false, false, true)` — no looping, no scope, with rewind.

**MUSIC-SPEECH-02:** `snd_stop_speech()` shall stop the speech stream and set `MUSIC_STATE.cur_speech_ref = None`. When `cur_speech_ref` is already `None`, it shall return `Ok(())` immediately.

#### MUSIC-LOAD: Music Loading

**MUSIC-LOAD-01:** `get_music_data()` shall return `Err(AudioError::NullPointer)` if the filename is empty.

**MUSIC-LOAD-02:** `get_music_data()` shall load a decoder (dispatched by file extension to the appropriate `SoundDecoder` implementation) with `buffer_size = 4096`, `start_time = 0`, `run_time = 0`, and create a sample via `create_sound_sample(Some(decoder), 64, None)`.

**MUSIC-LOAD-03:** `get_music_data()` shall return the `MusicRef` wrapping a `Box<SoundSample>` leaked to a raw pointer.

**MUSIC-LOAD-04:** When the decoder fails to load, `get_music_data()` shall return `Err(AudioError::ResourceNotFound(filename))`.

**MUSIC-LOAD-05:** When `create_sound_sample()` fails, `get_music_data()` shall drop the decoder and return the error.

**MUSIC-LOAD-06:** `check_music_res_name()` shall log a `log::warn!()` if the file does not exist in the content directory, but shall still return `Some(filename.to_string())`.

#### MUSIC-RELEASE: Music Release

**MUSIC-RELEASE-01:** `release_music_data()` shall return `Err(AudioError::NullPointer)` when passed a null `MusicRef`.

**MUSIC-RELEASE-02:** When the sample has a decoder: `release_music_data()` shall acquire `SOURCES.sources[MUSIC_SOURCE].lock()`, check if the sample is currently the music source's active sample (by `Arc::ptr_eq`), and if so, call `stop_stream(MUSIC_SOURCE)`.

**MUSIC-RELEASE-03:** `release_music_data()` shall set `sample.decoder = None`, drop the decoder, call `destroy_sound_sample()`, and reclaim the `Box<SoundSample>` from the raw pointer (via `Box::from_raw()`).

**MUSIC-RELEASE-04:** The FFI function `DestroyMusic` shall delegate to `release_music_data()`.

#### MUSIC-VOLUME: Music Volume

**MUSIC-VOLUME-01:** `set_music_volume()` shall compute gain as `(volume as f32 / 255.0) * music_volume_scale` and apply it via `mixer_source_f(SOURCES.sources[MUSIC_SOURCE].handle, SourceProp::Gain, gain)`. It shall store `volume` in `MUSIC_STATE.music_volume`.

---

### SFX — Sound Effects

#### SFX-PLAY: Effect Playback

**SFX-PLAY-01:** `play_channel()` shall call `stop_source(channel)` before starting new playback.

**SFX-PLAY-02:** `play_channel()` shall call `check_finished_channels()` to clean up all stopped SFX sources before playback.

**SFX-PLAY-03:** When the sound bank has no sample at `sound_index` (i.e., `bank.samples[sound_index]` is `None`), `play_channel()` shall return `Err(AudioError::InvalidSample)`.

**SFX-PLAY-04:** `play_channel()` shall set `source.sample` (via `Arc::new(Mutex::new(sample.clone()))` or by sharing ownership) and `source.positional_object = positional_object`.

**SFX-PLAY-05:** Where `SFX_STATE.opt_stereo_sfx` is `true`, `play_channel()` shall call `update_sound_position(channel, pos)`. Otherwise it shall call `update_sound_position(channel, SoundPosition::NON_POSITIONAL)`.

**SFX-PLAY-06:** `play_channel()` shall bind the sample's single buffer to the source via `mixer_source_i(source.handle, SourceProp::Buffer, sample.buffers[0] as i32)` and call `mixer_source_play(source.handle)`.

**SFX-PLAY-07:** `stop_channel()` shall call `stop_source(channel)`. The `priority` parameter shall be accepted but ignored.

**SFX-PLAY-08:** `check_finished_channels()` shall iterate `FIRST_SFX_SOURCE..=LAST_SFX_SOURCE` and call `clean_source(i)` on any source whose `mixer_get_source_i(handle, SourceProp::SourceState)` returns `Ok(SourceState::Stopped as i32)`.

**SFX-PLAY-09:** `channel_playing()` shall query `mixer_get_source_i(source.handle, SourceProp::SourceState)` and return `true` only when the result is `Ok(SourceState::Playing as i32)`.

#### SFX-POSITION: Positional Audio

**SFX-POSITION-01:** When `pos.positional` is `true`, `update_sound_position()` shall compute audio position as `(pos.x as f32 / 160.0, 0.0, pos.y as f32 / 160.0)` and apply via `mixer_source_fv()` (or equivalent 3D positioning API).

**SFX-POSITION-02:** When the computed distance from origin is less than `MIN_DISTANCE: f32 = 0.5`, `update_sound_position()` shall normalize the position vector and scale it to exactly `MIN_DISTANCE`.

**SFX-POSITION-03:** When `pos.positional` is `false`, `update_sound_position()` shall set the audio position to `(0.0, 0.0, -1.0)`.

**SFX-POSITION-04:** `get_positional_object()` shall acquire `SOURCES.sources[channel].lock()` and return `source.positional_object`.

**SFX-POSITION-05:** `set_positional_object()` shall acquire `SOURCES.sources[channel].lock()` and set `source.positional_object = obj`.

#### SFX-VOLUME: Channel Volume

**SFX-VOLUME-01:** `set_channel_volume()` shall compute gain as `(volume as f32 / MAX_VOLUME as f32) * sfx_volume_scale` and apply via `mixer_source_f(source.handle, SourceProp::Gain, gain)`. The `priority` parameter shall be accepted but ignored.

#### SFX-LOAD: Sound Bank Loading

**SFX-LOAD-01:** `get_sound_bank_data()` shall extract the directory prefix from the filename by finding the last `/` or `\` separator (using `std::path::Path::parent()`).

**SFX-LOAD-02:** `get_sound_bank_data()` shall read lines from the data, parsing each as a filename (up to `MAX_FX: usize = 256` sound effects).

**SFX-LOAD-03:** For each sound effect, the system shall load a decoder with `buffer_size = 4096`, `start_time = 0`, `run_time = 0`, create a sample with `num_buffers = 1` and no callbacks, fully pre-decode via `decoder.decode_all()` (or equivalent loop calling `decode()` until EOF), upload the decoded data to the sample's single buffer via `mixer_buffer_data()`, and drop the decoder.

**SFX-LOAD-04:** When no sound effects are successfully decoded, `get_sound_bank_data()` shall return `Err(AudioError::ResourceNotFound(_))`.

**SFX-LOAD-05:** `get_sound_bank_data()` shall return a `SoundBank` with `samples: Vec<Option<SoundSample>>` populated with each loaded sample.

**SFX-LOAD-06:** When `SoundBank` construction fails mid-loading, any already-created samples shall be cleaned up by dropping the partially-populated `Vec` (Rust's drop semantics handle this).

**SFX-LOAD-07:** Sound effect lookup from a `SoundBank` shall use direct indexing: `bank.samples[index]`.

#### SFX-RELEASE: Sound Bank Release

**SFX-RELEASE-01:** `release_sound_bank_data()` moves the `SoundBank` by value; passing an empty/invalid bank is a no-op.

**SFX-RELEASE-02:** For each `Some(sample)` in the sound bank, `release_sound_bank_data()` shall check all `NUM_SOUNDSOURCES` sources to see if the sample is currently active (by `Arc::ptr_eq` comparison). If so, it shall call `stop_source(i)` and set `source.sample = None`.

**SFX-RELEASE-03:** `release_sound_bank_data()` shall call `destroy_sound_sample()` on each sample and drop the `SoundBank` (Rust drop semantics handle memory).

**SFX-RELEASE-04:** The FFI function `DestroySound` shall delegate to `release_sound_bank_data()`.

---

### VOLUME — Volume & Global Control

#### VOLUME-INIT: Initialization

**VOLUME-INIT-01:** The system shall declare a `SoundSourceArray` containing `NUM_SOUNDSOURCES` (7) `Mutex<SoundSource>` entries, each initialized with a mixer source handle obtained via `mixer_gen_sources(NUM_SOUNDSOURCES as u32)`.

**VOLUME-INIT-02:** The system shall initialize `music_volume` to `NORMAL_VOLUME` (160).

**VOLUME-INIT-03:** The system shall declare volume scale fields: `music_volume_scale: f32`, `sfx_volume_scale: f32`, `speech_volume_scale: f32`, all defaulting to `1.0`.

**VOLUME-INIT-04:** `init_sound()` shall be callable and return `Ok(())`. Arguments are not needed (Rust uses builder pattern or config struct if needed in the future).

**VOLUME-INIT-05:** `uninit_sound()` shall be a no-op (resource cleanup is handled by Rust's `Drop` on program exit or explicit `uninit_stream_decoder()`).

#### VOLUME-CONTROL: Volume Control

**VOLUME-CONTROL-01:** `set_sfx_volume()` shall iterate `FIRST_SFX_SOURCE..=LAST_SFX_SOURCE` and call `mixer_source_f(source.handle, SourceProp::Gain, volume)` on each.

**VOLUME-CONTROL-02:** `set_speech_volume()` shall call `mixer_source_f(SOURCES.sources[SPEECH_SOURCE].handle, SourceProp::Gain, volume)`.

**VOLUME-CONTROL-03:** `fade_music()` shall clamp `time_interval` to `0` when `quit_posted()` returns `true` or `time_interval < 0`.

**VOLUME-CONTROL-04:** `fade_music()` shall call `set_music_stream_fade(time_interval as u32, end_vol)`. When the fade is rejected (returns `false`), it shall call `set_music_volume(end_vol)` immediately and return `get_time_counter()`.

**VOLUME-CONTROL-05:** When `fade_music()`'s fade is accepted, it shall return `get_time_counter() + time_interval as u32 + 1`.

#### VOLUME-SOURCE: Source Management

**VOLUME-SOURCE-01:** `stop_source()` shall call `mixer_source_stop(source.handle)` followed by `clean_source(source_index)`.

**VOLUME-SOURCE-02:** `clean_source()` shall set `source.positional_object = 0`, query `mixer_get_source_i(handle, SourceProp::BuffersProcessed)`, unqueue all processed buffers via `mixer_source_unqueue_buffers(handle, count)`, and call `mixer_source_rewind(handle)`.

**VOLUME-SOURCE-03:** Buffer handles from `mixer_source_unqueue_buffers()` are returned as a `Vec<usize>` — no stack-vs-heap decision needed (Rust `Vec` handles allocation).

**VOLUME-SOURCE-04:** `stop_sound()` shall iterate `FIRST_SFX_SOURCE..=LAST_SFX_SOURCE` and call `stop_source(i)` on each.

#### VOLUME-QUERY: Playback Queries

**VOLUME-QUERY-01:** `sound_playing()` shall iterate all `NUM_SOUNDSOURCES` sources. For sources with `sample.is_some()` and a decoder, it shall call `playing_stream(i)` under the source mutex. For sources without a decoder, it shall query `mixer_get_source_i(handle, SourceProp::SourceState)` and check for `SourceState::Playing`. It shall return `true` if any source is playing.

**VOLUME-QUERY-02:** `wait_for_sound_end()` shall poll in a loop, sleeping `Duration::from_millis(50)` per iteration. When `channel` is `None`, it shall wait for `sound_playing()` to return `false`; when `channel` is `Some(ch)`, it shall wait for `channel_playing(ch)` to return `false`.

**VOLUME-QUERY-03:** `wait_for_sound_end()` shall break immediately when `quit_posted()` returns `true`, to avoid blocking during application shutdown.

---

### FILEINST — File-Based Loading

#### FILEINST-LOAD: File Loading

**FILEINST-LOAD-01:** `load_sound_file()` shall acquire `FILE_STATE.lock()` and check that `cur_resfile_name` is `None`. Where it is `Some(_)`, the function shall return `Err(AudioError::ConcurrentLoad)`.

**FILEINST-LOAD-02:** `load_sound_file()` shall set `cur_resfile_name = Some(filename.to_string())`, call `get_sound_bank_data(filename, &data)`, set `cur_resfile_name = None`, and return the result. The `cur_resfile_name` shall be cleared in a `finally`-equivalent (`defer`/RAII guard pattern) to ensure cleanup on error.

**FILEINST-LOAD-03:** When the resource file fails to read, `load_sound_file()` shall return `Err(AudioError::IoError(_))`.

**FILEINST-LOAD-04:** `load_music_file()` shall acquire `FILE_STATE.lock()` and check that `cur_resfile_name` is `None`. Where it is `Some(_)`, the function shall return `Err(AudioError::ConcurrentLoad)`.

**FILEINST-LOAD-05:** `load_music_file()` shall validate the filename via `check_music_res_name()`, set `cur_resfile_name`, call `get_music_data(filename)`, clear `cur_resfile_name` (via RAII guard), and return the result.

**FILEINST-LOAD-06:** When the resource file fails to read, `load_music_file()` shall return `Err(AudioError::IoError(_))`.

**FILEINST-LOAD-07:** Both `load_sound_file()` and `load_music_file()` shall use an RAII guard (a `Drop`-implementing struct) to ensure `cur_resfile_name` is reset to `None` on all exit paths (success, error, panic).

---

### CROSS — Cross-Cutting Requirements

#### CROSS-THREAD: Thread Safety

**CROSS-THREAD-01:** All streaming functions that modify `SoundSource` fields shall be called with the appropriate per-source `Mutex` held by the caller (or acquire it internally).

**CROSS-THREAD-02:** The decoder thread shall lock each source's `Mutex<SoundSource>` individually before processing and release it after.

**CROSS-THREAD-03:** The fade system shall use a dedicated `Mutex<FadeState>` to protect all four fade state variables (`start_time`, `interval`, `start_volume`, `delta`).

**CROSS-THREAD-04:** The `FILE_STATE.cur_resfile_name` field shall serve as a mutual-exclusion guard for resource loading. `load_sound_file()` and `load_music_file()` shall return `Err(AudioError::ConcurrentLoad)` when `cur_resfile_name` is already `Some(_)`.

#### CROSS-MEMORY: Memory Management

**CROSS-MEMORY-01:** All heap allocations shall use standard Rust allocator (`Box`, `Vec`, `String`). No manual `HCalloc`/`HMalloc`/`HFree` — Rust's ownership system handles allocation and deallocation.

**CROSS-MEMORY-02:** `SoundSample` ownership: the sample owns its `buffers: Vec<usize>` and `buffer_tags: Vec<Option<SoundTag>>`. The decoder is NOT owned by the sample when used with the track player (the `SoundChunk` owns it). For music samples, the sample owns its decoder.

**CROSS-MEMORY-03:** `SoundChunk` ownership: each chunk owns its `decoder: Box<dyn SoundDecoder>` and its `text: Option<String>`. The linked list is owned by `TrackPlayerState.chunks_head: Option<Box<SoundChunk>>`. Dropping the head recursively drops all chunks.

**CROSS-MEMORY-04:** Mixer buffer handles (`usize`) shall be created with `mixer_gen_buffers()` and freed with `mixer_delete_buffers()`.

#### CROSS-CONST: Constants

**CROSS-CONST-01:** The system shall define `MAX_VOLUME: i32 = 255`.

**CROSS-CONST-02:** The system shall define `NORMAL_VOLUME: i32 = 160`.

**CROSS-CONST-03:** The system shall define `NUM_SFX_CHANNELS: usize = 5`.

**CROSS-CONST-04:** The system shall define source indices: `FIRST_SFX_SOURCE: usize = 0`, `LAST_SFX_SOURCE: usize = 4`, `MUSIC_SOURCE: usize = 5`, `SPEECH_SOURCE: usize = 6`, `NUM_SOUNDSOURCES: usize = 7`.

**CROSS-CONST-05:** The system shall define `PAD_SCOPE_BYTES: u32 = 256`.

**CROSS-CONST-06:** The system shall define `ACCEL_SCROLL_SPEED: u32 = 300`.

**CROSS-CONST-07:** The system shall define `TEXT_SPEED: i32 = 80` (ms per character for subtitle timing).

**CROSS-CONST-08:** The system shall define `ONE_SECOND: u32 = 840` (game time units per second).

#### CROSS-FFI: Foreign Function Interface

**CROSS-FFI-01:** All public API functions listed in §5 shall be exposed as `extern "C"` with `#[no_mangle]` for C callers.

**CROSS-FFI-02:** Type mappings shall preserve C ABI compatibility: `MusicRef` as `*mut c_void` (opaque pointer), `SoundBank` as `*mut c_void`, `SoundPosition` as `#[repr(C)]` struct, `SoundTag` as `#[repr(C)]` struct, mixer handles as `usize`.

**CROSS-FFI-03:** The Rust implementation shall call the Rust mixer API directly (`mixer_source_play()`, `mixer_buffer_data()`, etc.) rather than through C FFI (`rust_mixer_SourcePlay`, etc.), eliminating the `audiocore.h` indirection layer.

**CROSS-FFI-04:** The Rust implementation shall call Rust decoder methods directly via the `SoundDecoder` trait (`decoder.decode()`, `decoder.seek()`, etc.) rather than through C vtable dispatch.

#### CROSS-ERROR: Error Handling

**CROSS-ERROR-01:** Where mixer operations fail (return `Err(MixerError)`), the system shall log a warning via `log::warn!()` and continue operating without panicking.

**CROSS-ERROR-02:** Where a decoder returns `Err(DecodeError::DecoderError(_))`, the system shall log the error and either skip the current operation or halt the stream, but shall not panic.

**CROSS-ERROR-03:** Where resource loading functions fail (decoder not found, file not found), the system shall log a warning and return `Err(AudioError::ResourceNotFound(_))` to the caller.

#### CROSS-GENERAL: General

**CROSS-GENERAL-01:** All `Mutex` acquisitions shall use `parking_lot::Mutex` (consistent with `sound::mixer`), which does not return `Result` and does not panic on poison.

**CROSS-GENERAL-02:** Logging shall use the `log` crate macros (`log::warn!()`, `log::error!()`, `log::debug!()`) rather than `crate::bridge_log::rust_bridge_log_msg()` for new modules. The bridge log is for FFI shims only.

**CROSS-GENERAL-03:** All FFI functions shall be `unsafe` only at the boundary (the `extern "C" fn` itself). Internal Rust code shall use safe abstractions and never require `unsafe` blocks except for raw pointer conversions at the FFI layer.

**CROSS-GENERAL-04:** The `SoundDecoder` trait objects shall be `Send` (already required by the trait bound `pub trait SoundDecoder: Send`). `SoundSample` shall be `Send + Sync` when wrapped in `Arc<Mutex<>>`.

**CROSS-GENERAL-05:** Time functions (`get_time_counter()`, `ONE_SECOND`) shall be obtained via FFI from the C game engine's timing system until the timing subsystem is ported. Declarations: `extern "C" { fn GetTimeCounter() -> u32; }` and `const ONE_SECOND: u32 = 840;`.

**CROSS-GENERAL-06:** Content directory access for decoder loading shall use the existing `uio_*` FFI functions (`uio_open`, `uio_read`, `uio_close`, `uio_fstat`) until the I/O subsystem is ported, consistent with the pattern in `sound::ffi` and `sound::wav_ffi`.

**CROSS-GENERAL-07:** All new modules shall be added to `sound::mod.rs` as `pub mod` declarations and re-export key types.

**CROSS-GENERAL-08:** Error handling at the FFI boundary shall follow this convention: functions returning `bool` return `1` for success and `0` for failure. Functions returning counts return `0` for failure. Functions returning pointers return `null` for failure. Internal `Result` errors shall be logged before conversion.

---

## 5. FFI Boundary — Exported C Functions

Every function below is `#[no_mangle] pub extern "C" fn` in `sound::heart_ffi`. Signatures use C-compatible types. Pointer parameters are documented as nullable or non-null.

### 5.1 Stream Functions

```rust
/// Initialize the streaming decoder thread.
/// Returns: 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn InitStreamDecoder() -> c_int;

/// Shut down the streaming decoder thread.
#[no_mangle]
pub extern "C" fn UninitStreamDecoder();

/// Create a sound sample with num_buffers audio buffers.
/// decoder: nullable (can be NULL for SFX samples).
/// callbacks: nullable.
/// Returns: opaque pointer (Arc<Mutex<SoundSample>>::into_raw), or null on failure.
///
/// ALL-ARC STRATEGY: Every SoundSample pointer at the FFI boundary is
/// Arc<Mutex<SoundSample>>::into_raw. Functions that borrow use
/// Arc::increment_strong_count + Arc::from_raw. Destroy uses Arc::from_raw
/// (consuming). This eliminates Box/Arc confusion at the boundary.
#[no_mangle]
pub extern "C" fn TFB_CreateSoundSample(
    decoder: *mut c_void,       // *mut dyn SoundDecoder (opaque)
    num_buffers: u32,
    callbacks: *const TFB_SoundCallbacks_C,  // nullable
) -> *mut c_void;  // Arc<Mutex<SoundSample>>::into_raw

/// Destroy a sound sample. Decrements Arc refcount; frees if last reference.
/// Does NOT free the decoder.
#[no_mangle]
pub extern "C" fn TFB_DestroySoundSample(sample: *mut c_void);  // Arc ptr

/// Set user data on a sample.
#[no_mangle]
pub extern "C" fn TFB_SetSoundSampleData(sample: *mut SoundSample, data: *mut c_void);

/// Get user data from a sample.
#[no_mangle]
pub extern "C" fn TFB_GetSoundSampleData(sample: *const SoundSample) -> *mut c_void;

/// Set callbacks on a sample. callbacks may be null to clear.
#[no_mangle]
pub extern "C" fn TFB_SetSoundSampleCallbacks(
    sample: *mut SoundSample,
    callbacks: *const TFB_SoundCallbacks_C,
);

/// Get the decoder from a sample.
#[no_mangle]
pub extern "C" fn TFB_GetSoundSampleDecoder(sample: *const SoundSample) -> *mut c_void;

/// Start streaming playback of sample on source.
#[no_mangle]
pub extern "C" fn PlayStream(
    sample: *mut SoundSample,
    source: u32,
    looping: c_int,   // bool
    scope: c_int,      // bool
    rewind: c_int,     // bool
);

/// Stop streaming on source.
#[no_mangle]
pub extern "C" fn StopStream(source: u32);

/// Pause streaming on source.
#[no_mangle]
pub extern "C" fn PauseStream(source: u32);

/// Resume streaming on source.
#[no_mangle]
pub extern "C" fn ResumeStream(source: u32);

/// Seek stream to pos milliseconds.
#[no_mangle]
pub extern "C" fn SeekStream(source: u32, pos: u32);

/// Returns 1 if source is streaming, 0 otherwise.
#[no_mangle]
pub extern "C" fn PlayingStream(source: u32) -> c_int;

/// Find a tagged buffer. Returns pointer to tag or null.
#[no_mangle]
pub extern "C" fn TFB_FindTaggedBuffer(
    sample: *mut SoundSample,
    buffer: usize,
) -> *mut SoundTag;

/// Tag a buffer with data. Returns 1 on success, 0 on failure.
#[no_mangle]
pub extern "C" fn TFB_TagBuffer(
    sample: *mut SoundSample,
    buffer: usize,
    data: isize,
) -> c_int;

/// Clear a buffer tag.
#[no_mangle]
pub extern "C" fn TFB_ClearBufferTag(tag: *mut SoundTag);

/// Set music stream fade parameters.
/// Returns 1 if fade accepted, 0 if rejected.
#[no_mangle]
pub extern "C" fn SetMusicStreamFade(how_long: i32, end_volume: i32) -> c_int;

/// Read oscilloscope data into buffer.
/// Returns number of samples written.
#[no_mangle]
pub extern "C" fn GraphForegroundStream(
    data: *mut i32,
    width: i32,
    height: i32,
    want_speech: c_int,
) -> i32;
```

### 5.2 Track Player Functions

```rust
/// Add a track/subtitle to the speech queue.
/// track_name: nullable (NULL = subtitle-only append).
/// track_text: nullable.
/// timestamp: nullable.
/// callback: nullable function pointer.
#[no_mangle]
pub extern "C" fn SpliceTrack(
    track_name: *const c_char,
    track_text: *const c_char,   // UNICODE* in C = UTF-16; convert to UTF-8
    timestamp: *const c_char,
    callback: Option<extern "C" fn(c_int)>,
);

/// Add multiple tracks as a single combined track.
/// track_names: array of C strings, NULL-terminated.
/// track_text: nullable.
#[no_mangle]
pub extern "C" fn SpliceMultiTrack(
    track_names: *const *const c_char,
    track_text: *const c_char,
);

/// Begin speech playback.
#[no_mangle]
pub extern "C" fn PlayTrack();

/// Stop speech playback and free all track data.
#[no_mangle]
pub extern "C" fn StopTrack();

/// Jump past end of track (skip remaining).
#[no_mangle]
pub extern "C" fn JumpTrack();

/// Pause speech playback.
#[no_mangle]
pub extern "C" fn PauseTrack();

/// Resume speech playback.
#[no_mangle]
pub extern "C" fn ResumeTrack();

/// Returns track_num+1 if playing, 0 if not.
#[no_mangle]
pub extern "C" fn PlayingTrack() -> c_int;

/// Get current track position scaled to in_units.
#[no_mangle]
pub extern "C" fn GetTrackPosition(in_units: i32) -> i32;

/// Get current subtitle text. Returns pointer to C string or null.
/// Returned pointer is valid until next track operation.
#[no_mangle]
pub extern "C" fn GetTrackSubtitle() -> *const c_char;

/// Get first subtitle reference for iteration.
#[no_mangle]
pub extern "C" fn GetFirstTrackSubtitle() -> *mut c_void;

/// Get next subtitle reference after last_ref.
#[no_mangle]
pub extern "C" fn GetNextTrackSubtitle(last_ref: *mut c_void) -> *mut c_void;

/// Get text from a subtitle reference.
#[no_mangle]
pub extern "C" fn GetTrackSubtitleText(sub_ref: *mut c_void) -> *const c_char;

/// Seek/scroll operations.
#[no_mangle]
pub extern "C" fn FastReverse_Smooth();
#[no_mangle]
pub extern "C" fn FastForward_Smooth();
#[no_mangle]
pub extern "C" fn FastReverse_Page();
#[no_mangle]
pub extern "C" fn FastForward_Page();
```

### 5.3 Music Functions

```rust
/// Play a music track.
/// music_ref: opaque handle from LoadMusicFile.
/// continuous: 1 for looping, 0 for one-shot.
/// priority: ignored.
#[no_mangle]
pub extern "C" fn PLRPlaySong(
    music_ref: *mut c_void,  // MUSIC_REF
    continuous: c_int,
    priority: c_int,
);

/// Stop a music track. Pass ~0 for wildcard (stop any).
#[no_mangle]
pub extern "C" fn PLRStop(music_ref: *mut c_void);

/// Check if music is playing. Returns 1/0.
#[no_mangle]
pub extern "C" fn PLRPlaying(music_ref: *mut c_void) -> c_int;

/// Seek music to pos milliseconds.
#[no_mangle]
pub extern "C" fn PLRSeek(music_ref: *mut c_void, pos: u32);

/// Pause music playback.
#[no_mangle]
pub extern "C" fn PLRPause(music_ref: *mut c_void);

/// Resume music playback.
#[no_mangle]
pub extern "C" fn PLRResume(music_ref: *mut c_void);

/// Play speech from a music ref (speech-as-music).
#[no_mangle]
pub extern "C" fn snd_PlaySpeech(speech_ref: *mut c_void);

/// Stop speech playback.
#[no_mangle]
pub extern "C" fn snd_StopSpeech();

/// Set music volume (0-255).
#[no_mangle]
pub extern "C" fn SetMusicVolume(volume: c_int);

/// Fade music to end_vol over time_interval.
/// Returns expected completion time, or current time if immediate.
#[no_mangle]
pub extern "C" fn FadeMusic(end_vol: c_int, time_interval: c_int) -> u32;

/// Destroy a music instance.
#[no_mangle]
pub extern "C" fn DestroyMusic(music_ref: *mut c_void);
```

### 5.4 SFX Functions

```rust
/// Play a sound effect on a channel.
/// channel: 0-4 (SFX channel index).
/// snd: opaque SOUND handle (from resource system).
/// pos: positional audio data.
/// positional_object: opaque game object pointer.
/// priority: ignored.
#[no_mangle]
pub extern "C" fn PlayChannel(
    channel: c_int,
    snd: *mut c_void,     // SOUND handle
    pos: SoundPosition,
    positional_object: *mut c_void,
    priority: c_int,
);

/// Stop a channel. priority: ignored.
#[no_mangle]
pub extern "C" fn StopChannel(channel: c_int, priority: c_int);

/// Check if a channel is playing. Returns 1/0.
#[no_mangle]
pub extern "C" fn ChannelPlaying(channel: c_int) -> c_int;

/// Set volume on a specific channel. priority: ignored.
#[no_mangle]
pub extern "C" fn SetChannelVolume(channel: c_int, volume: c_int, priority: c_int);

/// Update 3D audio position for a channel.
#[no_mangle]
pub extern "C" fn UpdateSoundPosition(channel: c_int, pos: SoundPosition);

/// Get the positional game object for a channel.
#[no_mangle]
pub extern "C" fn GetPositionalObject(channel: c_int) -> *mut c_void;

/// Set the positional game object for a channel.
#[no_mangle]
pub extern "C" fn SetPositionalObject(channel: c_int, obj: *mut c_void);

/// Destroy a sound bank.
#[no_mangle]
pub extern "C" fn DestroySound(snd: *mut c_void);
```

### 5.5 Sound Control Functions

```rust
/// Stop all SFX channels.
#[no_mangle]
pub extern "C" fn StopSound();

/// Check if any source is playing. Returns 1/0.
#[no_mangle]
pub extern "C" fn SoundPlaying() -> c_int;

/// Block until channel (or all) finishes.
/// channel: channel index, or TFBSOUND_WAIT_ALL (-1) for all.
#[no_mangle]
pub extern "C" fn WaitForSoundEnd(channel: c_int);

/// Set volume for all SFX channels.
#[no_mangle]
pub extern "C" fn SetSFXVolume(volume: f32);

/// Set volume for speech channel.
#[no_mangle]
pub extern "C" fn SetSpeechVolume(volume: f32);

/// Initialize sound system. Returns 1 on success.
#[no_mangle]
pub extern "C" fn InitSound(argc: c_int, argv: *const *const c_char) -> c_int;

/// Uninitialize sound system.
#[no_mangle]
pub extern "C" fn UninitSound();
```

### 5.6 File Loading Functions

```rust
/// Load a sound bank from a file.
/// Returns opaque SOUND handle, or null on failure.
#[no_mangle]
pub extern "C" fn LoadSoundFile(filename: *const c_char) -> *mut c_void;

/// Load a music file.
/// Returns opaque MUSIC_REF handle, or null on failure.
#[no_mangle]
pub extern "C" fn LoadMusicFile(filename: *const c_char) -> *mut c_void;
```

### 5.7 C Callback Wrapper Types

```rust
/// C-compatible callback structure matching TFB_SoundCallbacks.
/// Used only at the FFI boundary for C callers that provide callbacks.
#[repr(C)]
pub struct TFB_SoundCallbacks_C {
    pub on_start_stream: Option<unsafe extern "C" fn(*mut SoundSample) -> c_int>,
    pub on_end_chunk: Option<unsafe extern "C" fn(*mut SoundSample, usize) -> c_int>,
    pub on_end_stream: Option<unsafe extern "C" fn(*mut SoundSample)>,
    pub on_tagged_buffer: Option<unsafe extern "C" fn(*mut SoundSample, *mut SoundTag)>,
    pub on_queue_buffer: Option<unsafe extern "C" fn(*mut SoundSample, usize)>,
}
```

The FFI layer converts `TFB_SoundCallbacks_C` into a `Box<dyn StreamCallbacks + Send>` wrapper that calls the C function pointers via `unsafe`. This wrapper is only needed for C code that directly creates samples with callbacks (i.e., if any C code other than the track player creates samples). The track player's callbacks are pure Rust.

---

## 6. Integration Notes

### 6.1 Decoder Loading

The new modules need a decoder factory function. This dispatches by file extension:

```rust
pub fn load_decoder(
    dir: *mut uio_DirHandle,
    filename: &str,
    buffer_size: u32,
    start_sample: u32,
    end_sample: u32,
) -> AudioResult<Box<dyn SoundDecoder>>;
```

This wraps the existing `uio_*` FFI + decoder selection logic from the C `SoundDecoder_Load()`. It reads the file via `uio_open`/`uio_read`/`uio_close`, detects format by extension (`.ogg` → `OggDecoder`, `.wav` → `WavDecoder`, `.mod`/`.s3m`/`.xm`/`.it` → `ModDecoder`, `.duk` → `DukAudDecoder`), calls `open_from_bytes()`, and returns the boxed decoder.

### 6.2 Time Functions

Until the timing subsystem is ported:

```rust
extern "C" {
    fn GetTimeCounter() -> u32;
}

fn get_time_counter() -> u32 {
    unsafe { GetTimeCounter() }
}

const ONE_SECOND: u32 = 840;
```

### 6.3 Quit Detection

```rust
extern "C" {
    fn QuitPosted() -> c_int;
}

fn quit_posted() -> bool {
    unsafe { QuitPosted() != 0 }
}
```

### 6.4 Resource System Integration

`LoadSoundFile` and `LoadMusicFile` interface with the C resource system (`reslib`). The FFI shims accept the same opaque handles that C game code uses. The Rust modules own the `SoundSample` and `SoundBank` data; the C code holds opaque pointers.

### 6.5 Build Integration

New modules are added to `rust/src/sound/mod.rs`:

```rust
pub mod control;
pub mod fileinst;
pub mod heart_ffi;
pub mod music;
pub mod sfx;
pub mod stream;
pub mod trackplayer;
```

The `heart_ffi` module's `#[no_mangle]` functions are linked into the C build via the existing Rust static library. C header declarations go in a new `rust_audio_heart.h`.

### 6.6 Migration Path

1. Build the Rust modules and verify they compile alongside existing code.
2. Create `rust_audio_heart.h` with all FFI declarations.
3. Swap C files for Rust: `#ifdef USE_RUST_AUDIO_HEART` in the build system routes calls through the new header instead of compiling `stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c`.
4. The existing C `audiocore.h` → `rust_mixer.h` redirection remains unchanged — the Rust streaming code calls the mixer directly, and any remaining C code still goes through the mixer FFI.

## Review Notes

*Reviewed 2026-02-24 by cross-referencing c-heart.md (226 EARS requirements), rust-heart.md (234 EARS requirements), and existing Rust source in `rust/src/sound/` (mod.rs, decoder.rs, mixer/mod.rs, mixer/types.rs, mixer/source.rs, rodio_backend.rs, formats.rs).*

### 1. Requirement Coverage — C→Rust Mapping

**All 226 unique C EARS requirement IDs from c-heart.md are present in the Rust spec with matching IDs.** Verified via automated `grep | sort -u | comm -23` comparison — zero missing. The Rust spec adds 8 Rust-specific `CROSS-GENERAL-*` requirements (CROSS-GENERAL-01 through CROSS-GENERAL-08) covering parking_lot consistency, logging conventions, unsafe boundaries, Send+Sync bounds, time FFI, content I/O, module registration, and FFI error conventions.

**Note:** The C spec footer (line 1427) claims "Total requirements: 137" but actual unique ID count is 226. The footer count appears to be from an earlier draft — the discrepancy is cosmetic and does not affect coverage.

**Category-level breakdown (all matched):**

| Category | C count | Rust count | Status |
|----------|---------|------------|--------|
| STREAM-INIT | 7 | 7 | [OK] |
| STREAM-PLAY | 20 | 20 | [OK] |
| STREAM-THREAD | 8 | 8 | [OK] |
| STREAM-PROCESS | 16 | 16 | [OK] |
| STREAM-SAMPLE | 5 | 5 | [OK] |
| STREAM-TAG | 3 | 3 | [OK] |
| STREAM-SCOPE | 11 | 11 | [OK] |
| STREAM-FADE | 5 | 5 | [OK] |
| TRACK-ASSEMBLE | 19 | 19 | [OK] |
| TRACK-PLAY | 10 | 10 | [OK] |
| TRACK-SEEK | 13 | 13 | [OK] |
| TRACK-CALLBACK | 9 | 9 | [OK] |
| TRACK-SUBTITLE | 4 | 4 | [OK] |
| TRACK-POSITION | 2 | 2 | [OK] |
| MUSIC-PLAY | 8 | 8 | [OK] |
| MUSIC-SPEECH | 2 | 2 | [OK] |
| MUSIC-LOAD | 6 | 6 | [OK] |
| MUSIC-RELEASE | 4 | 4 | [OK] |
| MUSIC-VOLUME | 1 | 1 | [OK] |
| SFX-PLAY | 9 | 9 | [OK] |
| SFX-POSITION | 5 | 5 | [OK] |
| SFX-VOLUME | 1 | 1 | [OK] |
| SFX-LOAD | 7 | 7 | [OK] |
| SFX-RELEASE | 4 | 4 | [OK] |
| VOLUME-INIT | 5 | 5 | [OK] |
| VOLUME-CONTROL | 5 | 5 | [OK] |
| VOLUME-SOURCE | 4 | 4 | [OK] |
| VOLUME-QUERY | 3 | 3 | [OK] |
| FILEINST-LOAD | 7 | 7 | [OK] |
| CROSS-THREAD | 4 | 4 | [OK] |
| CROSS-MEMORY | 4 | 4 | [OK] |
| CROSS-CONST | 8 | 8 | [OK] |
| CROSS-FFI | 4 | 4 | [OK] |
| CROSS-ERROR | 3 | 3 | [OK] |
| CROSS-GENERAL | — | 8 | [OK] (Rust-only) |

### 2. Technical Soundness

Verified against actual code in `rust/src/sound/`.

#### 2.1 Confirmed — Spec Matches Existing Code

- **`SoundDecoder` trait** (decoder.rs): Trait exists with `decode(&mut [u8]) -> DecodeResult<usize>`, `seek(u32) -> DecodeResult<u32>`, `frequency() -> u32`, `format() -> AudioFormat`, `length() -> f32`. The trait requires `Send`. The spec's `Box<dyn SoundDecoder>` pattern is correct.
- **`AudioFormat`** (formats.rs): Has `bytes_per_sample()` and `channels()` methods. Spec references like `decoder.format().channels()` in STREAM-SCOPE-11 are valid.
- **Mixer API** (mixer/mod.rs, mixer/source.rs): All functions referenced in the spec exist: `mixer_source_play`, `mixer_source_stop`, `mixer_source_pause`, `mixer_source_rewind`, `mixer_source_queue_buffers`, `mixer_source_unqueue_buffers`, `mixer_buffer_data`, `mixer_gen_buffers`, `mixer_delete_buffers`, `mixer_get_source_i`, `mixer_get_buffer_i`, `mixer_source_i`, `mixer_source_f`, `mixer_gen_sources`, `mixer_delete_sources`. Signatures match: e.g., `mixer_source_i(handle: usize, prop: SourceProp, value: i32) -> Result<(), MixerError>`.
- **`SourceProp` / `BufferProp` enums** (mixer/types.rs): All variants referenced exist: `SourceProp::Gain`, `SourceProp::Looping`, `SourceProp::Buffer`, `SourceProp::SourceState`, `SourceProp::BuffersQueued`, `SourceProp::BuffersProcessed`, `SourceProp::Position`. `BufferProp::Size` exists for scope data.
- **`SourceState` enum**: `Playing`, `Stopped`, `Paused`, `Initial` — all exist (mixer/types.rs).
- **`MixerError` enum** (mixer/types.rs line 38): Exists, spec's `From<MixerError> for AudioError` is feasible.
- **`DecodeError` enum** (decoder.rs): Has `EndOfFile`, `DecoderError(String)`, `IoError(String)` — all variants referenced in spec exist.
- **`parking_lot::Mutex`**: Already used in mixer/buffer.rs, mixer/mix.rs, mixer/source.rs. CROSS-GENERAL-01's requirement to use `parking_lot::Mutex` for consistency is correct.
- **Module structure** (mod.rs): Currently declares `decoder`, `formats`, `mixer`, `ogg`, `wav`, `mod_decoder`, `dukaud`, `null`, `rodio_audio`, `rodio_backend`, plus FFI modules. Spec's §6.5 correctly adds `control`, `fileinst`, `heart_ffi`, `music`, `sfx`, `stream`, `trackplayer`.

#### 2.2 Issues — Spec References Non-Existent APIs

1. **`decoder.looping` / `set_looping()`** — STREAM-PLAY-07 says "set `decoder.looping` via the decoder trait." The `SoundDecoder` trait (decoder.rs) has NO `looping` field or `set_looping()` method. In the C code, `looping` is a field on `TFB_SoundDecoder` set directly. **Action needed:** Either add a `set_looping(&mut self, looping: bool)` method to the `SoundDecoder` trait, or store the looping flag on `SoundSample` instead of the decoder (simpler — the decoder doesn't actually use this flag internally, it's used by the stream processing loop to decide whether to rewind on EOF).

2. **`decoder.decode_all()`** — Referenced in TRACK-ASSEMBLE-15, SFX-LOAD-03 and the spec's §3.4.3 API. The `SoundDecoder` trait has no `decode_all()` method. In C, `SoundDecoder_DecodeAll()` is a standalone function that loops `decode()` calls. **Action needed:** Either add `decode_all()` as a default method on the trait, or note it as a free function (e.g., `fn decode_all(decoder: &mut dyn SoundDecoder) -> DecodeResult<Vec<u8>>`) in the spec. A default trait method is the cleaner Rust pattern.

3. **`decoder.get_time()`** — Referenced in STREAM-PLAY-06. The `SoundDecoder` trait has `get_frame()` but no `get_time()` method. In C, `SoundDecoder_GetTime()` computes `pcm_pos / frequency`. **Action needed:** Add a default `get_time(&self) -> f32` method to the trait (or a free function). It can be derived from `get_frame() / frequency()`.

4. **`mixer_source_fv()` / 3D position API** — SFX-POSITION-01 references "`mixer_source_fv()` (or equivalent 3D positioning API)." The mixer module (source.rs) exports `mixer_source_f` but NOT `mixer_source_fv` (no vector variant). The `SourceProp::Position` enum variant exists but `mixer_source_f` only takes a single `f32` value, not a float vector. The rodio_backend has `rust_audio_source_fv` (rodio_backend.rs:861) but that's the old API. **Action needed:** Either add `mixer_source_fv(handle, prop, &[f32; 3])` to the mixer, or use three separate `mixer_source_f` calls to set X/Y/Z position components. The spec should be more specific about which approach to use. The mixer may need a vector-valued setter added.

5. **`mixer_source_rewind()`** — Referenced in VOLUME-SOURCE-02. Exists in mixer (re-exported from source.rs). Confirmed OK.

#### 2.3 Observations — Technically Sound But Worth Noting

1. **`TrackPlayerState` uses raw pointers** (§3.2.2): `chunks_tail: *mut SoundChunk` and `last_sub: *mut SoundChunk` are raw non-owning pointers. The `unsafe impl Send` is necessary but introduces a correctness burden. The spec correctly documents when these are valid (only accessed from main thread under `TRACK_STATE` mutex). This is an acceptable tradeoff for matching the C linked-list pattern — a safe alternative would require arena allocation or index-based addressing, which would be a significant departure.

2. **`StreamEngine` global via `lazy_static!`** (§3.1.2): The `lazy_static!` block initializing `ENGINE` will call `StreamEngine::new()` on first access. If mixer sources haven't been generated yet (mixer not initialized), the `sources` array initialization could fail. The spec should note that `init_stream_decoder()` must be called after `mixer_init()`.

3. **`SoundSample` shared via `Arc<Mutex<SoundSample>>`**: The spec shows `play_stream()` taking `Arc<Mutex<SoundSample>>`. This means the decoder thread locks the sample mutex on every buffer processing iteration. Since the source mutex is already held, this creates a two-lock-deep nesting pattern (source mutex → sample mutex). Lock ordering must be consistent: always source-then-sample to avoid deadlocks. The spec doesn't explicitly state this lock ordering rule — it should.

4. **Iterative `Drop` on linked list (FIX: ISSUE-MISC-02)**: TRACK-PLAY-04 / TRACK-ASSEMBLE-19 use an iterative `Drop` implementation for `SoundChunk` to avoid stack overflow from deep recursion on long linked lists. The `Drop` impl loops through `next` pointers, taking each Box and dropping it in a loop rather than relying on recursive Drop. This is safe for any list length.

5. **`MusicRef` as raw pointer wrapper** (§3.3.1): `MusicRef(*mut SoundSample)` wrapping a leaked `Box` is the standard pattern for FFI opaque handles. The `Box::from_raw()` in `release_music_data()` correctly reclaims ownership. This is sound.

6. **`std::sync::Mutex` vs `parking_lot::Mutex`**: §1.4 says `parking_lot::Mutex<>` matching the mixer, and CROSS-GENERAL-01 mandates it, but `StreamEngine` (§3.1.2) shows `Mutex<SoundSource>` without qualification. The spec should consistently say `parking_lot::Mutex` everywhere to avoid confusion with `std::sync::Mutex`.

### 3. Scope Completeness

**When fully implemented, all six C files will be replaceable.** Every public function from each C file has a corresponding Rust function in the spec:

| C File | Rust Module | C Functions | Rust Coverage |
|--------|-------------|-------------|---------------|
| `stream.c` | `stream.rs` | `PlayStream`, `StopStream`, `PauseStream`, `ResumeStream`, `SeekStream`, `PlayingStream`, `TFB_CreateSoundSample`, `TFB_DestroySoundSample`, `TFB_SetSoundSampleData`, `TFB_GetSoundSampleData`, `TFB_SetSoundSampleCallbacks`, `TFB_GetSoundSampleDecoder`, `TFB_FindTaggedBuffer`, `TFB_TagBuffer`, `TFB_ClearBufferTag`, `SetMusicStreamFade`, `GraphForegroundStream`, `InitStreamDecoder`, `UninitStreamDecoder` | All covered (§3.1, §5.1) |
| `trackplayer.c` | `trackplayer.rs` | `SpliceTrack`, `SpliceMultiTrack`, `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`, `PlayingTrack`, `FastReverse_Smooth`, `FastForward_Smooth`, `FastReverse_Page`, `FastForward_Page`, `GetTrackPosition`, `GetTrackSubtitle`, `GetFirstTrackSubtitle`, `GetNextTrackSubtitle`, `GetTrackSubtitleText` | All covered (§3.2, §5.2) |
| `music.c` | `music.rs` | `PLRPlaySong`, `PLRStop`, `PLRPlaying`, `PLRSeek`, `PLRPause`, `PLRResume`, `snd_PlaySpeech`, `snd_StopSpeech`, `SetMusicVolume`, `FadeMusic`, `_GetMusicData`, `_ReleaseMusicData`, `CheckMusicResName`, `DestroyMusic` | All covered (§3.3, §5.3) |
| `sfx.c` | `sfx.rs` | `PlayChannel`, `StopChannel`, `ChannelPlaying`, `SetChannelVolume`, `CheckFinishedChannels`, `UpdateSoundPosition`, `GetPositionalObject`, `SetPositionalObject`, `_GetSoundBankData`, `_ReleaseSoundBankData`, `DestroySound` | All covered (§3.4, §5.4) |
| `sound.c` | `control.rs` | `StopSource`, `CleanSource`, `StopSound`, `SoundPlaying`, `WaitForSoundEnd`, `SetSFXVolume`, `SetSpeechVolume`, `InitSound`, `UninitSound` | All covered (§3.5, §5.5) |
| `fileinst.c` | `fileinst.rs` | `LoadSoundFile`, `LoadMusicFile` | All covered (§3.6, §5.6) |

**The FFI boundary (§5) provides `#[no_mangle] pub extern "C"` shims for all 60+ functions** called from C game code.

**One gap to note:** The C spec documents `GetSoundAddress` / `GetStringAddress` (SFX-LOAD-07, sfx.c:304–308) as the function that resolves a `SOUND` handle to a `SOUNDPTR`. The Rust spec's `play_channel()` takes a `&SoundBank` + `SoundHandle` (index) instead, which is a cleaner API. However, the FFI `PlayChannel` in §5.4 takes `snd: *mut c_void` (opaque SOUND handle). The spec doesn't document how this opaque handle is resolved to a `SoundBank` + index. This is a minor gap — the FFI shim will need to look up the sound bank from the resource system, but the mechanism isn't specified.

### 4. Summary of Action Items

| # | Severity | Issue | Location |
|---|----------|-------|----------|
| 1 | **Must fix** | `SoundDecoder` trait lacks `set_looping()` — needed by STREAM-PLAY-07. Add to trait or move looping flag to `SoundSample`. | decoder.rs / §3.1 |
| 2 | **Must fix** | `SoundDecoder` trait lacks `decode_all()` — needed by TRACK-ASSEMBLE-15, SFX-LOAD-03. Add as default trait method or free function. | decoder.rs / §3.2, §3.4 |
| 3 | **Must fix** | `SoundDecoder` trait lacks `get_time()` — needed by STREAM-PLAY-06. Add as default method: `fn get_time(&self) -> f32 { self.get_frame() as f32 / self.frequency() as f32 }`. | decoder.rs / §3.1 |
| 4 | **Must fix** | Mixer lacks `mixer_source_fv()` vector setter for 3D position — needed by SFX-POSITION-01. Add to mixer/source.rs or specify alternative approach. | mixer/source.rs / §3.4 |
| 5 | **Should fix** | Specify lock ordering rule: source mutex must always be acquired before sample mutex. Avoids deadlocks in decoder thread. | §3.1.4 |
| 6 | **Should fix** | Document that `init_stream_decoder()` must be called after `mixer_init()` (lazy_static ENGINE depends on mixer being ready). | §3.1.4 / STREAM-INIT-01 |
| 7 | **Should fix** | Consistently write `parking_lot::Mutex` in all code examples (some show unqualified `Mutex`). | §3.1.2, §3.2.2, etc. |
| 8 | **Minor** | Document how FFI `PlayChannel(snd: *mut c_void)` resolves the opaque SOUND handle to a `SoundBank` + index. | §5.4 |
| 9 | **Minor** | C spec footer says "137 requirements" but actual count is 226. Cosmetic only. | c-heart.md line 1427 |
| 10 | **Minor** | Consider iterative `Drop` for `SoundChunk` linked list to avoid stack overflow on very long chains. | §3.2.1 |

