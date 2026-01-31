# Phase 4: Audio Mixer Port to Rust

## Overview
Port `sc2/src/libs/sound/mixer/mixer.c` (1760 lines) to Rust. The mixer is an OpenAL-like audio mixing engine that combines multiple audio sources into a single output stream.

## C Source Files
- `sc2/src/libs/sound/mixer/mixer.c` - Main implementation
- `sc2/src/libs/sound/mixer/mixer.h` - Public interface
- `sc2/src/libs/sound/mixer/mixerint.h` - Internal declarations

## Key Data Structures

### mixer_Buffer
```rust
pub struct MixerBuffer {
    pub magic: u32,           // 0x4258494D ("MIXB")
    pub locked: bool,
    pub state: BufferState,
    pub data: Vec<u8>,
    pub size: u32,
    pub sampsize: u32,
    pub high: u32,
    pub low: u32,
    // Original buffer values for compatibility
    pub org_data: Option<Vec<u8>>,
    pub org_freq: u32,
    pub org_size: u32,
    pub org_channels: u32,
    pub org_chansize: u32,
    // Linked list (use indices or Rc)
    pub next: Option<usize>,
}
```

### mixer_Source
```rust
pub struct MixerSource {
    pub magic: u32,           // 0x5358494D ("MIXS")
    pub locked: bool,
    pub state: SourceState,
    pub looping: bool,
    pub gain: f32,
    pub queued_count: u32,
    pub processed_count: u32,
    pub first_queued: Option<usize>,
    pub next_queued: Option<usize>,
    pub prev_queued: Option<usize>,
    pub last_queued: Option<usize>,
    pub pos: u32,
    pub count: u32,
    pub sample_cache: f32,
}
```

### Enums
```rust
pub enum MixerError {
    NoError = 0,
    InvalidName = 0xA001,
    InvalidEnum = 0xA002,
    InvalidValue = 0xA003,
    InvalidOperation = 0xA004,
    OutOfMemory = 0xA005,
    DriverFailure = 0xA101,
}

pub enum SourceState {
    Initial = 0,
    Stopped,
    Playing,
    Paused,
}

pub enum BufferState {
    Initial = 0,
    Filled,
    Queued,
    Playing,
    Processed,
}

pub enum MixerQuality {
    Low = 0,
    Medium,
    High,
}
```

## Core Functions to Implement

### Initialization
- `mixer_Init(frequency, format, quality, flags) -> bool`
- `mixer_Uninit()`
- `mixer_GetError() -> u32`

### Source Management
- `mixer_GenSources(n, sources)`
- `mixer_DeleteSources(n, sources)`
- `mixer_IsSource(src) -> bool`
- `mixer_Sourcei(src, prop, value)`
- `mixer_Sourcef(src, prop, value)`
- `mixer_GetSourcei(src, prop) -> value`
- `mixer_GetSourcef(src, prop) -> value`
- `mixer_SourcePlay(src)`
- `mixer_SourcePause(src)`
- `mixer_SourceStop(src)`
- `mixer_SourceRewind(src)`
- `mixer_SourceQueueBuffers(src, n, buffers)`
- `mixer_SourceUnqueueBuffers(src, n, buffers)`

### Buffer Management
- `mixer_GenBuffers(n, buffers)`
- `mixer_DeleteBuffers(n, buffers)`
- `mixer_IsBuffer(buf) -> bool`
- `mixer_BufferData(buf, format, data, size, freq)`
- `mixer_GetBufferi(buf, prop) -> value`

### Mixing
- `mixer_MixChannels(userdata, stream, len)` - Main mixing callback
- `mixer_MixFake(userdata, stream, len)` - Fake mixing for timing

### Resampling (internal)
- `mixer_ResampleNone(src, left) -> f32`
- `mixer_ResampleNearest(src, left) -> f32`
- `mixer_UpsampleLinear(src, left) -> f32`
- `mixer_UpsampleCubic(src, left) -> f32`

## Audio Format
```rust
// Format: bits 0-7 = bytes per channel, bits 8-15 = channels
pub const MIX_FORMAT_MONO8: u32 = 0x00170101;
pub const MIX_FORMAT_STEREO8: u32 = 0x00170201;
pub const MIX_FORMAT_MONO16: u32 = 0x00170102;
pub const MIX_FORMAT_STEREO16: u32 = 0x00170202;
```

## Thread Safety
The C code uses three mutexes:
- `src_mutex` - Protects source operations
- `buf_mutex` - Protects buffer operations  
- `act_mutex` - Protects active sources list

Use `parking_lot::RwLock` or `Mutex` for Rust implementation.

## Test Plan (TDD)

### Unit Tests
1. `test_mixer_init_uninit` - Initialize and uninitialize
2. `test_mixer_gen_delete_sources` - Create and delete sources
3. `test_mixer_gen_delete_buffers` - Create and delete buffers
4. `test_mixer_buffer_data` - Load audio data into buffer
5. `test_mixer_source_properties` - Get/set source properties
6. `test_mixer_buffer_properties` - Get buffer properties
7. `test_mixer_source_queue` - Queue buffers to source
8. `test_mixer_source_unqueue` - Unqueue processed buffers
9. `test_mixer_source_play_stop` - Play and stop sources
10. `test_mixer_source_pause_resume` - Pause and resume
11. `test_mixer_source_looping` - Looping playback
12. `test_mixer_gain` - Volume control
13. `test_mixer_resample_none` - No resampling
14. `test_mixer_resample_linear` - Linear interpolation
15. `test_mixer_resample_cubic` - Cubic interpolation
16. `test_mixer_mix_mono8` - Mix mono 8-bit
17. `test_mixer_mix_stereo16` - Mix stereo 16-bit
18. `test_mixer_mix_multiple_sources` - Mix multiple sources

### Integration Tests
1. `test_mixer_with_ogg_decoder` - Load Ogg, queue, play
2. `test_mixer_with_wav_decoder` - Load WAV, queue, play
3. `test_mixer_crossfade` - Fade between sources

## File Structure
```
rust/src/sound/
├── mod.rs              (add mixer module)
├── mixer/
│   ├── mod.rs          (public exports)
│   ├── types.rs        (enums, structs)
│   ├── buffer.rs       (buffer management)
│   ├── source.rs       (source management)
│   ├── resample.rs     (resampling algorithms)
│   ├── mix.rs          (mixing logic)
│   └── ffi.rs          (C FFI bindings)
```

## FFI Functions to Export
```rust
#[no_mangle]
pub extern "C" fn rust_mixer_Init(...) -> c_int;
#[no_mangle]
pub extern "C" fn rust_mixer_Uninit();
#[no_mangle]
pub extern "C" fn rust_mixer_MixChannels(...);
// ... etc
```

## Dependencies
- `parking_lot` - For mutex/rwlock
- `crossbeam` - For lock-free data structures (optional)

## Acceptance Criteria
1. All unit tests pass
2. Can mix audio from Ogg decoder
3. Resampling quality matches C implementation
4. Thread-safe operation
5. FFI bindings work with C code
6. No memory leaks
