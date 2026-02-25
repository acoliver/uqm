# Audio Heart — Domain Model Analysis

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. Entity Inventory

### 1.1 Core Entities

| Entity | Rust Type | Ownership | Lifetime |
|--------|-----------|-----------|----------|
| SoundSample | `struct SoundSample` | Owned by caller (music: Box via MusicRef, track: Arc via TrackPlayerState, sfx: in SoundBank) | Created on load/splice, destroyed on release/stop |
| SoundTag | `struct SoundTag` | Owned by SoundSample (in buffer_tags Vec) | Created on tag_buffer, cleared on clear_buffer_tag |
| SoundSource | `struct SoundSource` | Owned by StreamEngine (static array) | Program lifetime |
| SoundChunk | `struct SoundChunk` | Owned by TrackPlayerState (linked list via Box) | Created on splice_track, dropped on stop_track |
| FadeState | `struct FadeState` | Owned by StreamEngine | Program lifetime |
| StreamEngine | `struct StreamEngine` | Global static (lazy_static) | Program lifetime |
| TrackPlayerState | `struct TrackPlayerState` | Global static (lazy_static + Mutex) | Program lifetime |
| MusicState | `struct MusicState` | Global static (lazy_static + Mutex) | Program lifetime |
| SfxState | `struct SfxState` | Global static (lazy_static + Mutex) | Program lifetime |
| VolumeState | `struct VolumeState` | Global static (lazy_static + Mutex) | Program lifetime |
| FileInstState | `struct FileInstState` | Global static (lazy_static + Mutex) | Program lifetime |
| SoundSourceArray | `struct SoundSourceArray` | Global static (lazy_static) | Program lifetime |
| MusicRef | `#[repr(transparent)] struct MusicRef(*mut SoundSample)` | Raw pointer wrapper (C owns handle, Rust owns data) | Created on load, destroyed on release |
| SoundBank | `struct SoundBank` | Owned by caller (C game code via opaque pointer) | Created on load_sound_file, destroyed on release |
| SoundPosition | `#[repr(C)] struct SoundPosition` | Value type (Copy) | Stack lifetime |
| SubtitleRef | `struct SubtitleRef` | Wrapper around pointer into chunk list | Valid while track state is held |

### 1.2 Trait Entities

| Trait | Implementors | Notes |
|-------|-------------|-------|
| StreamCallbacks | TrackCallbacks, CCallbackWrapper (FFI) | Default no-op implementations |
| SoundDecoder | WavDecoder, OggDecoder, ModDecoder, DukAudDecoder, NullDecoder | Send required |

---

## 2. State Transition Diagrams

### 2.1 SoundSource States

```
                ┌────────────┐
                │  Inactive   │ sample=None, stream_should_be_playing=false
                └─────┬──────┘
                      │ play_stream()
                      ▼
                ┌────────────┐
      ┌────────│  Playing    │ stream_should_be_playing=true, pause_time=0
      │        └─────┬──────┘
      │              │ pause_stream()
      │              ▼
      │        ┌────────────┐
      │        │  Paused     │ stream_should_be_playing=false, pause_time>0
      │        └─────┬──────┘
      │              │ resume_stream()
      │              ▼
      │        ┌────────────┐
      │        │  Playing    │ (back to playing)
      │        └─────┬──────┘
      │              │
      ├──────────────┤ stop_stream() / end_of_stream
      │              │
      │              ▼
      │        ┌────────────┐
      └───────▶│  Stopped    │ stream_should_be_playing=false, sample=None
               └────────────┘
```

### 2.2 FadeState States

```
     ┌──────────────┐
     │  Inactive     │ interval=0
     └──────┬───────┘
            │ set_music_stream_fade()
            ▼
     ┌──────────────┐
     │  Active       │ interval>0, start_time set
     └──────┬───────┘
            │ elapsed >= interval
            ▼
     ┌──────────────┐
     │  Completed    │ interval=0 (→ Inactive)
     └──────────────┘
```

### 2.3 TrackPlayerState Lifecycle

```
     ┌──────────────┐
     │  Empty        │ track_count=0, chunks_head=None
     └──────┬───────┘
            │ splice_track() (first call)
            ▼
     ┌──────────────┐
     │  Assembling   │ track_count>0, chunks building
     └──────┬───────┘
            │ play_track()
            ▼
     ┌──────────────┐
     │  Playing      │ cur_chunk=Some, sound_sample active
     └──────┬───────┘
            │ stop_track() / jump_track() / end_of_stream
            ▼
     ┌──────────────┐
     │  Stopped      │ track_count=0, chunks dropped
     └──────────────┘
```

### 2.4 FileInstState Guard

```
     ┌──────────────┐
     │  Idle         │ cur_resfile_name=None
     └──────┬───────┘
            │ load_sound_file() or load_music_file()
            ▼
     ┌──────────────┐
     │  Loading      │ cur_resfile_name=Some(name)
     └──────┬───────┘
            │ load completes (success or error, via RAII guard)
            ▼
     ┌──────────────┐
     │  Idle         │ cur_resfile_name=None (guaranteed by Drop)
     └──────────────┘
```

---

## 3. Edge/Error Handling Map

### 3.1 Error Propagation Paths

| Error Condition | Source | Handler | Result |
|----------------|--------|---------|--------|
| Mixer not initialized | mixer_* calls | stream/control | Log + continue or propagate AudioError::MixerError |
| Decoder EOF | decoder.decode() | stream processing | Call on_end_chunk callback, advance or end stream |
| Decoder error | decoder.decode() | stream processing | Log, stop stream (REQ-STREAM-PROCESS-12) |
| Buffer underrun | mixer reports 0 processed + not playing + queued>0 | stream thread | Log warning, restart playback (REQ-STREAM-PROCESS-03) |
| Null pointer at FFI | C passes null | heart_ffi | Return error code, log warning |
| Concurrent load | two load calls | fileinst | Return AudioError::ConcurrentLoad |
| Invalid source index | out-of-bounds index | stream/control | Return AudioError::InvalidSource |
| Invalid channel | channel >= NUM_SFX_CHANNELS | sfx | Return AudioError::InvalidChannel |
| Thread spawn failure | std::thread::Builder | init_stream_decoder | Return AudioError::NotInitialized |
| Resource not found | decoder loading | music/sfx/fileinst | Return AudioError::ResourceNotFound |

### 3.2 Panic-Free Guarantee

All paths use `Result<T, AudioError>`. No `unwrap()` or `expect()` in production code. `parking_lot::Mutex` does not poison on panic (unlike `std::sync::Mutex`), so mutex acquisition cannot fail.

---

## 4. Integration Touchpoints

### 4.1 Module → Module Dependencies

```
heart_ffi → music, sfx, trackplayer, stream, control, fileinst
music     → stream, control
sfx       → stream, control
trackplayer → stream, control
stream    → control (for SoundSourceArray), mixer
control   → mixer
fileinst  → music (get_music_data), sfx (get_sound_bank_data)
```

### 4.2 Module → Existing Code Dependencies

```
stream      → mixer::{source, buffer, types}
control     → mixer::{gen_sources, delete_sources, source_*, get_source_*}
music       → mixer::{source_f, gen_buffers}, decoder (via load_decoder)
sfx         → mixer::{source_i, source_play, buffer_data}, decoder
fileinst    → decoder (via load_decoder), uio_* FFI
heart_ffi   → std::ffi::{c_char, c_int, c_void}
all modules → extern "C" { GetTimeCounter(), QuitPosted() }
```

### 4.3 C Code → Rust FFI Callers

| C Caller | FFI Function(s) |
|----------|----------------|
| `comm.c` (comm screen) | SpliceTrack, PlayTrack, StopTrack, GetTrackSubtitle, GraphForegroundStream |
| `melee.c`, `combat.c` | PlayChannel, StopChannel, StopSound |
| `setup.c` / `starcon2.c` | InitStreamDecoder, InitSound, UninitSound, UninitStreamDecoder |
| `gameopt.c` | SetMusicVolume, SetSFXVolume, SetSpeechVolume, FadeMusic |
| `hyper.c`, `solarsys.c` | PLRPlaySong, PLRStop, PLRPlaying |
| `reslib.c` | LoadSoundFile, LoadMusicFile |
| `instfile.c` / `restypes.c` | DestroySound, DestroyMusic |

---

## 5. Old Code to Replace/Remove

When `USE_RUST_AUDIO_HEART` is enabled:

| C File | Action |
|--------|--------|
| `sc2/src/libs/sound/stream.c` | Exclude from build |
| `sc2/src/libs/sound/trackplayer.c` | Exclude from build |
| `sc2/src/libs/sound/music.c` | Exclude from build |
| `sc2/src/libs/sound/sfx.c` | Exclude from build |
| `sc2/src/libs/sound/sound.c` | Exclude from build |
| `sc2/src/libs/sound/fileinst.c` | Exclude from build |
| C headers (sndintrn.h, audiocore.h parts) | Keep but conditionally exclude implementations |

New files added:
- `sc2/src/libs/sound/rust_audio_heart.h` — FFI declarations
- `rust/src/sound/stream.rs`
- `rust/src/sound/trackplayer.rs`
- `rust/src/sound/music.rs`
- `rust/src/sound/sfx.rs`
- `rust/src/sound/control.rs`
- `rust/src/sound/fileinst.rs`
- `rust/src/sound/heart_ffi.rs`

---

## 6. Threading Model Summary

| Thread | Accesses | Locks Required |
|--------|----------|----------------|
| Main thread | All API calls, track assembly, SFX playback, volume | Per-source mutex, TRACK_STATE, MUSIC_STATE, FILE_STATE |
| Decoder thread | Stream processing for MUSIC_SOURCE and SPEECH_SOURCE | Per-source mutex (individual), FadeState mutex |

Lock ordering (must be respected to avoid deadlock):
1. `TRACK_STATE` mutex (outermost)
2. Source mutex (per-source, from `SOURCES.sources[i]`)
3. Sample mutex (per-sample, from `Arc<Mutex<SoundSample>>`)
4. `FadeState` mutex (innermost)

Never hold a higher-numbered lock while acquiring a lower-numbered one.

---

## 7. Decoder Trait Gaps (Action Items from Spec Review)

These must be resolved in early implementation phases:

1. **`set_looping()`** — Store looping flag on `SoundSample` (not decoder). Simpler and doesn't modify existing trait.
2. **`decode_all()`** — Add as free function: `fn decode_all(decoder: &mut dyn SoundDecoder) -> DecodeResult<Vec<u8>>`. Loops `decode()` until EOF.
3. **`get_time()`** — Add as free function: `fn get_decoder_time(decoder: &dyn SoundDecoder) -> f32 { decoder.get_frame() as f32 / decoder.frequency() as f32 }`.
4. **`mixer_source_fv()`** — Either add vector setter to mixer or use three separate `mixer_source_f` calls for 3D position components. Decision: add `mixer_source_fv()` to mixer module.
