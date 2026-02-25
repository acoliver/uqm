# Audio Heart — Specification

Plan ID: `PLAN-20260225-AUDIO-HEART`
Source: `project-plans/audiorust/rust-heart.md`
Date: 2026-02-25

---

## 1. Purpose / Problem Statement

The UQM audio pipeline currently uses six C source files (`stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c`) that implement the "heart" of the audio subsystem — streaming playback, track/subtitle management, music/SFX APIs, volume control, and file-based loading. These C modules use global mutable state, function-pointer callbacks, manual memory management, and ad-hoc thread synchronization.

This specification defines equivalent Rust modules that replace all six C files entirely. The Rust implementation uses `Arc<parking_lot::Mutex<>>`, trait objects, typed errors, and explicit ownership to provide the same behavior with stronger safety guarantees. The existing Rust mixer (`sound::mixer`) and decoder (`sound::decoder`) modules are reused without modification; the new modules sit above them in the architecture.

---

## 2. Architectural Boundaries

### 2.1 Module Mapping

| C File | Rust Module | Location |
|--------|-------------|----------|
| `stream.c` | `sound::stream` | `rust/src/sound/stream.rs` |
| `trackplayer.c` | `sound::trackplayer` | `rust/src/sound/trackplayer.rs` |
| `music.c` | `sound::music` | `rust/src/sound/music.rs` |
| `sfx.c` | `sound::sfx` | `rust/src/sound/sfx.rs` |
| `sound.c` | `sound::control` | `rust/src/sound/control.rs` |
| `fileinst.c` | `sound::fileinst` | `rust/src/sound/fileinst.rs` |
| (FFI shims) | `sound::heart_ffi` | `rust/src/sound/heart_ffi.rs` |

### 2.2 Existing Modules (Reused, Not Modified)

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

### 2.3 Layered Architecture

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
│  SoundSourceArray (parking_lot::Mutex<> wrapped sources)           │
│  stop_source / clean_source / volume control                      │
├──────────────────────────────────────────────────────────────────┤
│                 sound::mixer (existing)                            │
│  mixer_source_play, mixer_buffer_data, mixer_gen_buffers, etc.    │
├──────────────────────────────────────────────────────────────────┤
│              sound::decoder (existing)                            │
│  SoundDecoder trait, WavDecoder, OggDecoder, ModDecoder, etc.     │
└──────────────────────────────────────────────────────────────────┘
```

### 2.4 Key Design Decisions

1. **Direct mixer calls** — Rust streaming code calls `mixer_source_play()` etc. directly (no FFI round-trip through `audio_*` / `rust_mixer_*`).
2. **`Arc<Mutex<>>` replaces C globals** — All mutable shared state wrapped in `parking_lot::Mutex<>` (matching mixer conventions).
3. **`Box<dyn SoundDecoder>` replaces `TFB_SoundDecoder*`** — Heap-allocated trait objects with explicit ownership.
4. **`Result<T, AudioError>` replaces C return codes** — Unified error enum; FFI shims convert to C-compatible integers.
5. **Callbacks become trait objects** — `StreamCallbacks` trait with default no-op implementations.
6. **`std::thread` + `parking_lot::Condvar`** — Replaces `AssignTask`/`ConcludeTask`/`HibernateThread`.
7. **Lock ordering** — Source mutex must always be acquired before sample mutex (avoids deadlocks).
8. **Initialization order** — `init_stream_decoder()` must be called after `mixer_init()`.

---

## 3. Data Contracts and Invariants

### 3.1 `AudioError` — Unified Error Enum

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
```

Invariants:
- Implements `Display`, `Error`, `From<MixerError>`, `From<DecodeError>`.
- `AudioResult<T> = Result<T, AudioError>` is the canonical return type.

### 3.2 Source Index Constants

```rust
pub const NUM_SFX_CHANNELS: usize = 5;
pub const FIRST_SFX_SOURCE: usize = 0;
pub const LAST_SFX_SOURCE: usize = 4;
pub const MUSIC_SOURCE: usize = 5;
pub const SPEECH_SOURCE: usize = 6;
pub const NUM_SOUNDSOURCES: usize = 7;
pub const MAX_VOLUME: i32 = 255;
pub const NORMAL_VOLUME: i32 = 160;
pub const PAD_SCOPE_BYTES: u32 = 256;
pub const ACCEL_SCROLL_SPEED: u32 = 300;
pub const TEXT_SPEED: i32 = 80;
pub const ONE_SECOND: u32 = 840;
```

### 3.3 Core Data Structures

- **`SoundSample`** — Owns buffers, borrows decoder. Fields: `decoder`, `length`, `buffers`, `num_buffers`, `buffer_tags`, `offset`, `data`, `callbacks`.
- **`SoundTag`** — Buffer-to-subtitle link. Fields: `buf_handle`, `data`.
- **`StreamCallbacks`** — Trait with `on_start_stream`, `on_end_chunk`, `on_end_stream`, `on_tagged_buffer`, `on_queue_buffer`.
- **`SoundSource`** — Per-source state: `sample`, `handle`, `stream_should_be_playing`, `start_time`, `pause_time`, `positional_object`, `last_q_buf`, scope buffer ring.
- **`FadeState`** — `start_time`, `interval`, `start_volume`, `delta`.
- **`StreamEngine`** — Global: `sources`, `fade`, `decoder_thread`, `shutdown`, `wake`.
- **`SoundChunk`** — Track chunk: `decoder`, `start_time`, `tag_me`, `track_num`, `text`, `callback`, `next`.
- **`TrackPlayerState`** — Track list head/tail, cur_chunk pointers, track_count, dec_offset.
- **`MusicRef`** — `#[repr(transparent)]` wrapper around `*mut SoundSample`.
- **`SoundPosition`** — `#[repr(C)]` struct: `positional`, `x`, `y`.
- **`SoundBank`** — Collection of pre-decoded SFX samples.
- **`SoundSourceArray`** — `[parking_lot::Mutex<SoundSource>; NUM_SOUNDSOURCES]`.
- **`VolumeState`** — `music_volume`, `music_volume_scale`, `sfx_volume_scale`, `speech_volume_scale`.
- **`FileInstState`** — `cur_resfile_name` concurrency guard.

---

## 4. Integration Points with Existing Modules

### 4.1 Mixer API (direct Rust calls)

All `mixer_*` functions from `sound::mixer` are called directly:
- `mixer_gen_sources`, `mixer_delete_sources`
- `mixer_gen_buffers`, `mixer_delete_buffers`
- `mixer_source_play`, `mixer_source_stop`, `mixer_source_pause`, `mixer_source_rewind`
- `mixer_source_queue_buffers`, `mixer_source_unqueue_buffers`
- `mixer_buffer_data`
- `mixer_source_i`, `mixer_source_f`, `mixer_get_source_i`, `mixer_get_source_f`, `mixer_get_buffer_i`

### 4.2 Decoder API (direct Rust calls)

`SoundDecoder` trait methods: `decode()`, `seek()`, `frequency()`, `format()`, `length()`, `get_frame()`, `is_null()`, `open_from_bytes()`, `close()`.

**Gaps requiring additions to `SoundDecoder` trait:**
- `set_looping()` or looping flag on `SoundSample`
- `decode_all()` default method
- `get_time()` default method

### 4.3 C Game Engine (FFI)

- `GetTimeCounter() -> u32` — Time function
- `QuitPosted() -> c_int` — Shutdown detection
- `uio_open` / `uio_read` / `uio_close` / `uio_fstat` — Content directory I/O

### 4.4 C Build System

- New header: `rust_audio_heart.h` with all FFI declarations
- `USE_RUST_AUDIO_HEART` flag in `config_unix.h`
- Conditional compilation excludes C files when flag is set

---

## 5. Functional Requirements

### STREAM — Streaming Audio Engine

| ID | Title | Summary |
|----|-------|---------|
| REQ-STREAM-INIT-01 | Fade mutex init | Create `parking_lot::Mutex<FadeState>` on init |
| REQ-STREAM-INIT-02 | Decoder thread spawn | Spawn background decoder thread |
| REQ-STREAM-INIT-03 | Thread spawn failure | Return `Err(AudioError::NotInitialized)` on spawn failure |
| REQ-STREAM-INIT-04 | Shutdown sequence | Set `shutdown` atomic, notify condvar, join thread |
| REQ-STREAM-INIT-05 | FadeState drop | Normal Rust drop semantics |
| REQ-STREAM-INIT-06 | No thread at uninit | Skip thread termination when handle is None |
| REQ-STREAM-INIT-07 | Idempotent shutdown | No panic if FadeState already dropped |
| REQ-STREAM-PLAY-01 | Stop before play | Call `stop_stream()` first |
| REQ-STREAM-PLAY-02 | Empty sample check | Return `Err(InvalidSample)` for empty Arc |
| REQ-STREAM-PLAY-03 | Callback abort | Return `Err(EndOfStream)` if `on_start_stream` returns false |
| REQ-STREAM-PLAY-04 | Clear tags | Clear all buffer tags on play |
| REQ-STREAM-PLAY-05 | Rewind | Seek decoder to 0 when `rewind=true` |
| REQ-STREAM-PLAY-06 | Time offset calc | Compute offset from decoder time when not rewinding |
| REQ-STREAM-PLAY-07 | Source setup | Store sample, set looping, configure mixer |
| REQ-STREAM-PLAY-08 | Scope alloc | Allocate scope buffer when `scope=true` |
| REQ-STREAM-PLAY-09 | Pre-fill buffers | Decode and queue up to `num_buffers` initial buffers |
| REQ-STREAM-PLAY-10 | Queue callback | Call `on_queue_buffer` for each queued buffer |
| REQ-STREAM-PLAY-11 | EOF during pre-fill | Call `on_end_chunk`; continue if returns true |
| REQ-STREAM-PLAY-12 | Zero decode | Stop pre-filling on `Ok(0)` |
| REQ-STREAM-PLAY-13 | Start playback | Set timing, flags, call `mixer_source_play` |
| REQ-STREAM-PLAY-14 | Stop stream | Stop source, clear sample and scope |
| REQ-STREAM-PLAY-15 | Pause stream | Record pause time, pause mixer source |
| REQ-STREAM-PLAY-16 | Resume time adjust | Adjust start_time for pause duration |
| REQ-STREAM-PLAY-17 | Resume stream | Clear pause, restart mixer source |
| REQ-STREAM-PLAY-18 | Seek stream | Stop, seek decoder, restart |
| REQ-STREAM-PLAY-19 | Seek no sample | Return `Err(InvalidSample)` if no sample |
| REQ-STREAM-PLAY-20 | Playing query | Return `stream_should_be_playing` under mutex |
| REQ-STREAM-THREAD-01 | Main loop | Loop while `shutdown` is false |
| REQ-STREAM-THREAD-02 | Process fade | Call `process_music_fade()` each iteration |
| REQ-STREAM-THREAD-03 | Source iteration | Iterate `MUSIC_SOURCE..NUM_SOUNDSOURCES` |
| REQ-STREAM-THREAD-04 | Per-source locking | Lock each source individually |
| REQ-STREAM-THREAD-05 | Skip inactive | Skip sources with no sample/decoder/not playing |
| REQ-STREAM-THREAD-06 | Idle sleep | Wait on condvar 100ms when no active streams |
| REQ-STREAM-THREAD-07 | Active yield | Yield instead of sleep when streams active |
| REQ-STREAM-THREAD-08 | Clean exit | Return normally on shutdown |
| REQ-STREAM-PROCESS-01 | Query processed/queued | Query mixer buffer counts |
| REQ-STREAM-PROCESS-02 | End of stream detect | Detect end when processed=0, not playing, queued=0, EOF |
| REQ-STREAM-PROCESS-03 | Buffer underrun | Restart playback on underrun |
| REQ-STREAM-PROCESS-04 | Unqueue buffers | Unqueue processed buffers |
| REQ-STREAM-PROCESS-05 | Unqueue error | Log and break on unqueue error |
| REQ-STREAM-PROCESS-06 | Tagged buffer callback | Call on_tagged_buffer for tagged unqueued buffers |
| REQ-STREAM-PROCESS-07 | Scope remove | Remove scope data for unqueued buffers |
| REQ-STREAM-PROCESS-08 | EOF no callback | Set end_chunk_failed flag |
| REQ-STREAM-PROCESS-09 | EOF with callback | Re-read decoder on successful callback |
| REQ-STREAM-PROCESS-10 | Non-EOF error | Skip buffer on non-EOF error |
| REQ-STREAM-PROCESS-11 | Decode new audio | Decode into recycled buffer |
| REQ-STREAM-PROCESS-12 | Decode error | Log, stop stream on decode error |
| REQ-STREAM-PROCESS-13 | Zero decode skip | Skip buffer on zero bytes decoded |
| REQ-STREAM-PROCESS-14 | Upload and queue | Upload via mixer_buffer_data, queue |
| REQ-STREAM-PROCESS-15 | Last queued buffer | Store last_q_buf, call on_queue_buffer |
| REQ-STREAM-PROCESS-16 | Scope add | Add scope data after queuing |
| REQ-STREAM-SAMPLE-01 | Create sample | Allocate buffers, tags, store callbacks |
| REQ-STREAM-SAMPLE-02 | Destroy sample | Delete mixer buffers, clear tags, drop callbacks |
| REQ-STREAM-SAMPLE-03 | User data | Store/retrieve opaque data |
| REQ-STREAM-SAMPLE-04 | Set callbacks | Replace callbacks |
| REQ-STREAM-SAMPLE-05 | Get decoder | Return decoder reference |
| REQ-STREAM-TAG-01 | Find tag | Linear search buffer_tags |
| REQ-STREAM-TAG-02 | Set tag | Find/create tag slot |
| REQ-STREAM-TAG-03 | Clear tag | Set containing Option to None |
| REQ-STREAM-SCOPE-01 | Add scope data | Copy to ring buffer at tail |
| REQ-STREAM-SCOPE-02 | Remove scope data | Advance head, update lasttime |
| REQ-STREAM-SCOPE-03 | Source preference | Prefer speech when want_speech=true |
| REQ-STREAM-SCOPE-04 | No stream | Return 0 when no playable stream |
| REQ-STREAM-SCOPE-05 | Step size | Normalize to 11025Hz reference |
| REQ-STREAM-SCOPE-06 | Read position | Compute from head + delta |
| REQ-STREAM-SCOPE-07 | Render | Scale, center, clamp, write |
| REQ-STREAM-SCOPE-08 | Sample conversion | 8-bit unsigned to signed 16-bit |
| REQ-STREAM-SCOPE-09 | AGC | 16-page running average, 8-frame pages |
| REQ-STREAM-SCOPE-10 | VAD | Energy threshold 100 |
| REQ-STREAM-SCOPE-11 | Multi-channel | Sum both channels |
| REQ-STREAM-FADE-01 | Start fade | Store fade params |
| REQ-STREAM-FADE-02 | Reject zero | Return false for zero-duration fade |
| REQ-STREAM-FADE-03 | Interpolate | Linear volume interpolation |
| REQ-STREAM-FADE-04 | End fade | Set interval=0 when elapsed >= interval |
| REQ-STREAM-FADE-05 | No-op inactive | Return immediately when interval=0 |

### TRACK — Track Player

| ID | Title | Summary |
|----|-------|---------|
| REQ-TRACK-ASSEMBLE-01 | Split pages | Split subtitle at `\r\n` |
| REQ-TRACK-ASSEMBLE-02 | Page timing | TEXT_SPEED * char count, min 1000ms |
| REQ-TRACK-ASSEMBLE-03 | Continuation marks | `..` prefix, `...` suffix |
| REQ-TRACK-ASSEMBLE-04 | Append to last | Append when track_name is None |
| REQ-TRACK-ASSEMBLE-05 | No tracks warning | Warn when track_count=0 and no name |
| REQ-TRACK-ASSEMBLE-06 | No text early return | Return Ok when track_text is None |
| REQ-TRACK-ASSEMBLE-07 | New track | Create decoder, sample on first call |
| REQ-TRACK-ASSEMBLE-08 | Decoder config | buffer_size=4096, accumulate dec_offset |
| REQ-TRACK-ASSEMBLE-09 | Explicit timestamps | Parse comma/CR/LF-separated timestamps |
| REQ-TRACK-ASSEMBLE-10 | Last page negative | Negate last page timestamp |
| REQ-TRACK-ASSEMBLE-11 | No page break | Append to last subtitle when flag set |
| REQ-TRACK-ASSEMBLE-12 | Chunk fields | tag_me, text, callback, track_num |
| REQ-TRACK-ASSEMBLE-13 | Reset page break | Set no_page_break=false after processing |
| REQ-TRACK-ASSEMBLE-14 | Timestamp parsing | Parse unsigned ints, skip zeros |
| REQ-TRACK-ASSEMBLE-15 | Multi-track | Up to 20 decoders, buffer_size=32768, pre-decode all |
| REQ-TRACK-ASSEMBLE-16 | Multi-track text | Append text, set no_page_break=true |
| REQ-TRACK-ASSEMBLE-17 | Multi-track precondition | Error when track_count=0 |
| REQ-TRACK-ASSEMBLE-18 | Chunk constructor | Zeroed chunk with decoder and start_time |
| REQ-TRACK-ASSEMBLE-19 | Chunk drop | Recursive Drop for linked list |
| REQ-TRACK-PLAY-01 | Play | Compute tracks_length, set cur_chunk, play_stream |
| REQ-TRACK-PLAY-02 | Play no sample | Return Ok when no sample |
| REQ-TRACK-PLAY-03 | Stop | Stop stream, reset state, drop chunks |
| REQ-TRACK-PLAY-04 | Stop cleanup | Drop chunk list via chunks_head=None |
| REQ-TRACK-PLAY-05 | Stop decoder | Set sample.decoder=None before destroy |
| REQ-TRACK-PLAY-06 | Jump | Seek past end |
| REQ-TRACK-PLAY-07 | Jump no sample | Return Ok when no sample |
| REQ-TRACK-PLAY-08 | Pause | Pause speech stream |
| REQ-TRACK-PLAY-09 | Resume | Verify paused, resume speech stream |
| REQ-TRACK-PLAY-10 | Playing query | Return cur_chunk.track_num+1 |
| REQ-TRACK-SEEK-01 | Clamp offset | Clamp to 0..=tracks_length+1 |
| REQ-TRACK-SEEK-02 | Set start_time | time_counter - offset |
| REQ-TRACK-SEEK-03 | Walk chunks | Find chunk at seek position |
| REQ-TRACK-SEEK-04 | Seek decoder | Seek within chunk, call do_track_tag |
| REQ-TRACK-SEEK-05 | Past end | Stop stream, clear pointers |
| REQ-TRACK-SEEK-06 | Current position | get_time_counter - start_time, clamped |
| REQ-TRACK-SEEK-07 | Reverse smooth | Subtract ACCEL_SCROLL_SPEED |
| REQ-TRACK-SEEK-08 | Forward smooth | Add ACCEL_SCROLL_SPEED |
| REQ-TRACK-SEEK-09 | Reverse page | Find prev page, restart |
| REQ-TRACK-SEEK-10 | Forward page | Find next page, restart |
| REQ-TRACK-SEEK-11 | Find next page | Iterate until tag_me=true |
| REQ-TRACK-SEEK-12 | Find prev page | Walk from head, find last tagged before cur |
| REQ-TRACK-SEEK-13 | Mutex acquisition | Lock SPEECH_SOURCE before modification |
| REQ-TRACK-CALLBACK-01 | Start verify | Verify sample match and cur_chunk |
| REQ-TRACK-CALLBACK-02 | Start setup | Set decoder and offset from cur_chunk |
| REQ-TRACK-CALLBACK-03 | Start tag | Call do_track_tag if tag_me |
| REQ-TRACK-CALLBACK-04 | End chunk fail | Return false on mismatch |
| REQ-TRACK-CALLBACK-05 | End chunk advance | Advance cur_chunk, set decoder, rewind |
| REQ-TRACK-CALLBACK-06 | End chunk tag | Tag buffer for tagged chunks |
| REQ-TRACK-CALLBACK-07 | End stream | Clear cur_chunk and cur_sub_chunk |
| REQ-TRACK-CALLBACK-08 | Tagged buffer | Extract chunk, clear tag, call do_track_tag |
| REQ-TRACK-CALLBACK-09 | Do track tag | Call callback, set cur_sub_chunk |
| REQ-TRACK-SUBTITLE-01 | Get subtitle | Return cur_sub_chunk.text |
| REQ-TRACK-SUBTITLE-02 | First subtitle | Return ref to chunks_head |
| REQ-TRACK-SUBTITLE-03 | Next subtitle | Call find_next_page |
| REQ-TRACK-SUBTITLE-04 | Subtitle text | Return sub_ref.text |
| REQ-TRACK-POSITION-01 | Position calc | in_units * offset / tracks_length |
| REQ-TRACK-POSITION-02 | Atomic load | Load tracks_length with Acquire ordering |

### MUSIC — Music Playback API

| ID | Title | Summary |
|----|-------|---------|
| REQ-MUSIC-PLAY-01 | Play song | Lock source, play_stream, store ref |
| REQ-MUSIC-PLAY-02 | Invalid ref | Return Err(InvalidSample) |
| REQ-MUSIC-PLAY-03 | Priority ignored | Accept but ignore priority |
| REQ-MUSIC-PLAY-04 | Stop | Stop stream, clear ref |
| REQ-MUSIC-PLAY-05 | Playing query | Check ref match and playing_stream |
| REQ-MUSIC-PLAY-06 | Seek | seek_stream under music source mutex |
| REQ-MUSIC-PLAY-07 | Pause | pause_stream under music source mutex |
| REQ-MUSIC-PLAY-08 | Resume | resume_stream under music source mutex |
| REQ-MUSIC-SPEECH-01 | Play speech | play_stream on SPEECH_SOURCE |
| REQ-MUSIC-SPEECH-02 | Stop speech | Stop stream, clear ref |
| REQ-MUSIC-LOAD-01 | Empty filename | Return Err(NullPointer) |
| REQ-MUSIC-LOAD-02 | Load decoder | Dispatch by extension, create sample |
| REQ-MUSIC-LOAD-03 | Return ref | Leak Box to raw pointer |
| REQ-MUSIC-LOAD-04 | Decoder fail | Return Err(ResourceNotFound) |
| REQ-MUSIC-LOAD-05 | Sample fail | Drop decoder, return error |
| REQ-MUSIC-LOAD-06 | Check res name | Warn on missing, return Some |
| REQ-MUSIC-RELEASE-01 | Null check | Return Err(NullPointer) |
| REQ-MUSIC-RELEASE-02 | Active stop | Stop if currently active |
| REQ-MUSIC-RELEASE-03 | Cleanup | Clear decoder, destroy sample, reclaim Box |
| REQ-MUSIC-RELEASE-04 | FFI delegate | DestroyMusic delegates to release |
| REQ-MUSIC-VOLUME-01 | Set volume | Compute gain, apply via mixer |

### SFX — Sound Effects

| ID | Title | Summary |
|----|-------|---------|
| REQ-SFX-PLAY-01 | Stop before play | Call stop_source first |
| REQ-SFX-PLAY-02 | Check finished | Call check_finished_channels |
| REQ-SFX-PLAY-03 | Missing sample | Return Err(InvalidSample) |
| REQ-SFX-PLAY-04 | Set source | Store sample and positional_object |
| REQ-SFX-PLAY-05 | Stereo position | Apply position if opt_stereo_sfx |
| REQ-SFX-PLAY-06 | Bind and play | Bind buffer, play source |
| REQ-SFX-PLAY-07 | Stop channel | Stop source, ignore priority |
| REQ-SFX-PLAY-08 | Check finished | Clean stopped SFX sources |
| REQ-SFX-PLAY-09 | Channel playing | Query SourceState::Playing |
| REQ-SFX-POSITION-01 | Compute position | x/160.0, y/160.0 for positional |
| REQ-SFX-POSITION-02 | Min distance | Normalize to MIN_DISTANCE=0.5 |
| REQ-SFX-POSITION-03 | Non-positional | Set to (0, 0, -1) |
| REQ-SFX-POSITION-04 | Get object | Return positional_object |
| REQ-SFX-POSITION-05 | Set object | Set positional_object |
| REQ-SFX-VOLUME-01 | Set volume | Compute gain with sfx_volume_scale |
| REQ-SFX-LOAD-01 | Dir prefix | Extract directory from filename |
| REQ-SFX-LOAD-02 | Parse lines | Read filenames, max 256 |
| REQ-SFX-LOAD-03 | Pre-decode | Load, decode_all, upload to buffer |
| REQ-SFX-LOAD-04 | Empty bank error | Err(ResourceNotFound) if none decoded |
| REQ-SFX-LOAD-05 | Return bank | Vec<Option<SoundSample>> |
| REQ-SFX-LOAD-06 | Partial cleanup | Rust drop handles partial Vec |
| REQ-SFX-LOAD-07 | Direct index | bank.samples[index] |
| REQ-SFX-RELEASE-01 | Empty no-op | Empty/invalid bank is no-op |
| REQ-SFX-RELEASE-02 | Active check | Stop active sources using sample |
| REQ-SFX-RELEASE-03 | Destroy samples | destroy_sound_sample on each |
| REQ-SFX-RELEASE-04 | FFI delegate | DestroySound delegates |

### VOLUME — Volume & Global Control

| ID | Title | Summary |
|----|-------|---------|
| REQ-VOLUME-INIT-01 | Source array | NUM_SOUNDSOURCES mutex entries with mixer handles |
| REQ-VOLUME-INIT-02 | Default volume | music_volume = NORMAL_VOLUME |
| REQ-VOLUME-INIT-03 | Volume scales | All 1.0 by default |
| REQ-VOLUME-INIT-04 | Init callable | Return Ok(()) |
| REQ-VOLUME-INIT-05 | Uninit no-op | Resource cleanup via Drop |
| REQ-VOLUME-CONTROL-01 | SFX volume | Apply gain to all SFX sources |
| REQ-VOLUME-CONTROL-02 | Speech volume | Apply gain to SPEECH_SOURCE |
| REQ-VOLUME-CONTROL-03 | Fade clamp | Clamp time_interval on quit |
| REQ-VOLUME-CONTROL-04 | Fade call | set_music_stream_fade or immediate |
| REQ-VOLUME-CONTROL-05 | Fade return | Return completion time |
| REQ-VOLUME-SOURCE-01 | Stop source | mixer_source_stop + clean_source |
| REQ-VOLUME-SOURCE-02 | Clean source | Reset, unqueue, rewind |
| REQ-VOLUME-SOURCE-03 | Buffer Vec | Rust Vec handles allocation |
| REQ-VOLUME-SOURCE-04 | Stop sound | Stop all SFX sources |
| REQ-VOLUME-QUERY-01 | Sound playing | Any source playing |
| REQ-VOLUME-QUERY-02 | Wait for end | Poll loop with 50ms sleep |
| REQ-VOLUME-QUERY-03 | Quit break | Break on quit_posted |

### FILEINST — File-Based Loading

| ID | Title | Summary |
|----|-------|---------|
| REQ-FILEINST-LOAD-01 | Concurrent guard | Check cur_resfile_name is None |
| REQ-FILEINST-LOAD-02 | Sound file | Set name, call get_sound_bank_data, clear |
| REQ-FILEINST-LOAD-03 | Sound read fail | Return Err(IoError) |
| REQ-FILEINST-LOAD-04 | Music guard | Check cur_resfile_name is None |
| REQ-FILEINST-LOAD-05 | Music file | Validate, set name, call get_music_data, clear |
| REQ-FILEINST-LOAD-06 | Music read fail | Return Err(IoError) |
| REQ-FILEINST-LOAD-07 | RAII guard | Drop-based cleanup for cur_resfile_name |

### CROSS — Cross-Cutting Requirements

| ID | Title | Summary |
|----|-------|---------|
| REQ-CROSS-THREAD-01 | Source mutex | All source modifications under mutex |
| REQ-CROSS-THREAD-02 | Decoder thread locking | Individual per-source locks |
| REQ-CROSS-THREAD-03 | Fade mutex | Dedicated FadeState mutex |
| REQ-CROSS-THREAD-04 | File mutex | cur_resfile_name as exclusion guard |
| REQ-CROSS-MEMORY-01 | Rust allocator | Standard Box/Vec/String |
| REQ-CROSS-MEMORY-02 | Sample ownership | Sample owns buffers; decoder ownership varies |
| REQ-CROSS-MEMORY-03 | Chunk ownership | Chunk owns decoder and text |
| REQ-CROSS-MEMORY-04 | Mixer handles | gen_buffers/delete_buffers lifecycle |
| REQ-CROSS-CONST-01 | MAX_VOLUME | 255 |
| REQ-CROSS-CONST-02 | NORMAL_VOLUME | 160 |
| REQ-CROSS-CONST-03 | NUM_SFX_CHANNELS | 5 |
| REQ-CROSS-CONST-04 | Source indices | 0-6 mapping |
| REQ-CROSS-CONST-05 | PAD_SCOPE_BYTES | 256 |
| REQ-CROSS-CONST-06 | ACCEL_SCROLL_SPEED | 300 |
| REQ-CROSS-CONST-07 | TEXT_SPEED | 80 |
| REQ-CROSS-CONST-08 | ONE_SECOND | 840 |
| REQ-CROSS-FFI-01 | extern C exports | All public API as #[no_mangle] |
| REQ-CROSS-FFI-02 | ABI compat | C-compatible type mappings |
| REQ-CROSS-FFI-03 | Direct mixer | No C FFI round-trip to mixer |
| REQ-CROSS-FFI-04 | Direct decoder | No C vtable dispatch |
| REQ-CROSS-ERROR-01 | Mixer error | Log and continue, no panic |
| REQ-CROSS-ERROR-02 | Decoder error | Log and skip/halt, no panic |
| REQ-CROSS-ERROR-03 | Resource error | Log and return Err(ResourceNotFound) |
| REQ-CROSS-GENERAL-01 | parking_lot | Use parking_lot::Mutex consistently |
| REQ-CROSS-GENERAL-02 | log crate | Use log macros for new modules |
| REQ-CROSS-GENERAL-03 | Unsafe boundary | unsafe only at FFI boundary |
| REQ-CROSS-GENERAL-04 | Send+Sync | SoundDecoder is Send; sample is Send+Sync in Arc |
| REQ-CROSS-GENERAL-05 | Time FFI | GetTimeCounter via FFI, ONE_SECOND=840 |
| REQ-CROSS-GENERAL-06 | Content I/O | uio_* FFI for file access |
| REQ-CROSS-GENERAL-07 | Module registration | Add to sound::mod.rs |
| REQ-CROSS-GENERAL-08 | FFI error convention | bool→1/0, count→0, pointer→null |

---

## 6. Error and Edge Cases

1. **Thread spawn failure** — `init_stream_decoder()` returns error, entire system remains uninitialized.
2. **Double init** — `AlreadyInitialized` error.
3. **Operations before init** — `NotInitialized` error.
4. **Concurrent file loading** — `ConcurrentLoad` error via `cur_resfile_name` guard.
5. **Null/invalid MusicRef** — `NullPointer` or `InvalidSample` error at FFI boundary.
6. **Decoder EOF during pre-fill** — Callback-driven chunk advancement or graceful end.
7. **Buffer underrun** — Log warning, restart mixer source.
8. **Recursive Drop on long chunk list** — Risk of stack overflow for very long lists (>50 chunks unlikely in practice).
9. **Lock ordering violation** — Source mutex must always precede sample mutex.
10. **Mixer not initialized** — Lazy static ENGINE initialization depends on mixer readiness.

---

## 7. Non-Functional Requirements

1. **Thread safety** — All shared state protected by `parking_lot::Mutex` or atomics.
2. **No panics** — Production paths use `Result`, never `unwrap()`/`expect()`.
3. **No unsafe in internal code** — `unsafe` only in FFI boundary functions.
4. **Memory safety** — Rust ownership prevents use-after-free, double-free.
5. **Performance** — Decoder thread yields (not sleeps) when active; direct mixer calls eliminate FFI overhead.
6. **Compatibility** — All 60+ C FFI functions maintain identical signatures and behavior.

---

## 8. Testability Requirements

1. **Unit tests** per module — test state transitions, error paths, edge cases.
2. **Mock mixer** — Tests use mock mixer functions or inject test doubles.
3. **Mock decoder** — `NullDecoder` or custom test decoders for streaming tests.
4. **Integration tests** — Verify module interactions (stream ↔ control, trackplayer ↔ stream).
5. **FFI boundary tests** — Verify C-compatible types, null pointer handling, error code translation.
6. **Thread safety tests** — Concurrent access patterns, shutdown ordering.
7. **Verification baseline:**
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features`
   - `cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm`
