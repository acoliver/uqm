# Video, Threading & Resource System Port to Rust

## Overview

This plan covers porting three interconnected C subsystems to Rust:
1. **Video Playback** - FMV/cutscene decoding and playback
2. **Threading/Tasks** - Thread management and task scheduling  
3. **Resource System** - Resource loading, caching, and management

All work follows **test-first development** with subagents handling individual components.

---

## Phase 1: Threading & Task System

**Goal:** Replace C pthreads/SDL threads with Rust's native threading model.

### C Files to Port
- `libs/threads/thrcommon.c` / `thrcommon.h` - Common thread utilities
- `libs/threads/sdl/sdlthread.c` - SDL thread wrapper
- `libs/threads/pthread/posixthread.c` - POSIX thread wrapper
- `libs/task/tasklib.c` - Task scheduling system

### Rust Implementation Plan

#### 1.1 Core Threading (`rust/src/threading/mod.rs`)
```
rust/src/threading/
├── mod.rs           # Module exports
├── thread.rs        # Thread wrapper (std::thread)
├── mutex.rs         # Mutex/RwLock wrappers
├── condvar.rs       # Condition variables
├── semaphore.rs     # Counting semaphore
└── ffi.rs           # C FFI bindings
```

**Key Types:**
- `Thread` - Wrapper around `std::thread::JoinHandle`
- `Mutex<T>` - Re-export or wrap `std::sync::Mutex`
- `CondVar` - Condition variable for signaling
- `Semaphore` - Counting semaphore (use `tokio::sync::Semaphore` or manual impl)

#### 1.2 Task System (`rust/src/threading/task.rs`)
- `Task` struct matching C `Task` 
- `TaskFunc` callback type
- `Task_SetState()`, `Task_GetState()` equivalents
- Thread-safe task queue

### Tests (Write First!)

```rust
// tests/threading_tests.rs

#[test]
fn test_thread_spawn_join() { }

#[test]
fn test_mutex_lock_unlock() { }

#[test]
fn test_condvar_wait_signal() { }

#[test]
fn test_semaphore_acquire_release() { }

#[test]
fn test_task_create_destroy() { }

#[test]
fn test_task_state_transitions() { }

#[test]
fn test_concurrent_task_execution() { }
```

### Subagent Tasks

| Task ID | Description | Subagent | Dependencies |
|---------|-------------|----------|--------------|
| T1.1 | Write threading unit tests | `coder` | None |
| T1.2 | Implement Thread wrapper | `coder` | T1.1 |
| T1.3 | Implement Mutex/RwLock | `coder` | T1.1 |
| T1.4 | Implement CondVar | `coder` | T1.3 |
| T1.5 | Implement Semaphore | `coder` | T1.3 |
| T1.6 | Write task system tests | `coder` | T1.2-T1.5 |
| T1.7 | Implement Task system | `coder` | T1.6 |
| T1.8 | Create FFI bindings | `coder` | T1.7 |
| T1.9 | Wire into C code | `coder` | T1.8 |

---

## Phase 2: Resource System Completion

**Goal:** Complete the partially-implemented Rust resource system.

### C Files to Port
- `libs/resource/resinit.c` - Resource initialization
- `libs/resource/reslib.c` - Core resource library
- `libs/resource/getres.c` - Resource retrieval
- `libs/resource/loadres.c` - Resource loading
- `libs/resource/propfile.c` - Property file parsing
- `libs/resource/stringbank.c` - String table management
- `libs/resource/index.c` - Resource index handling

### Existing Rust Code
```
rust/src/resource/
├── mod.rs              # [OK] Exists
├── propfile.rs         # [OK] Exists (property files)
├── stringbank.rs       # [OK] Exists (string tables)
├── resource_system.rs  # [OK] Exists (partial)
└── ffi.rs              # [OK] Exists (partial)
```

### Rust Implementation Plan

#### 2.1 Complete Resource System (`rust/src/resource/`)
```
rust/src/resource/
├── mod.rs              # Module exports
├── propfile.rs         # [OK] Complete property file parser
├── stringbank.rs       # [OK] Complete string bank
├── resource_system.rs  # Complete core resource system
├── index.rs            # NEW: Resource index (.rmp files)
├── loader.rs           # NEW: Resource loading
├── cache.rs            # NEW: Resource caching (LRU)
├── types.rs            # NEW: Resource type definitions
└── ffi.rs              # Complete FFI bindings
```

**Key Types:**
- `ResourceIndex` - Parsed .rmp index file
- `ResourceHandle` - Handle to loaded resource
- `ResourceCache` - LRU cache for loaded resources
- `ResourceLoader` - Async resource loading

### Tests (Write First!)

```rust
// tests/resource_tests.rs

#[test]
fn test_propfile_parse() { }

#[test]
fn test_propfile_get_value() { }

#[test]
fn test_stringbank_load() { }

#[test]
fn test_stringbank_get_string() { }

#[test]
fn test_resource_index_parse() { }

#[test]
fn test_resource_index_lookup() { }

#[test]
fn test_resource_load_from_file() { }

#[test]
fn test_resource_cache_lru() { }

#[test]
fn test_resource_handle_refcount() { }
```

### Subagent Tasks

| Task ID | Description | Subagent | Dependencies |
|---------|-------------|----------|--------------|
| R2.1 | Audit existing resource code | `coder` | None |
| R2.2 | Write resource index tests | `coder` | R2.1 |
| R2.3 | Implement ResourceIndex | `coder` | R2.2 |
| R2.4 | Write resource loader tests | `coder` | R2.3 |
| R2.5 | Implement ResourceLoader | `coder` | R2.4 |
| R2.6 | Write cache tests | `coder` | R2.5 |
| R2.7 | Implement ResourceCache | `coder` | R2.6 |
| R2.8 | Complete FFI bindings | `coder` | R2.7 |
| R2.9 | Wire into C code | `coder` | R2.8 |

---

## Phase 3: Video Playback System

**Goal:** Replace C video decoder with Rust using modern video crates.

### C Files to Port
- `libs/video/video.c` / `video.h` - Core video system
- `libs/video/videodec.c` / `videodec.h` - Video decoder interface
- `libs/video/vidplayer.c` / `vidplayer.h` - Video player
- `libs/video/dukvid.c` / `dukvid.h` - DukVid format decoder
- `libs/video/legacyplayer.c` - Legacy video player
- `libs/video/vfileins.c` - Video file instance
- `libs/video/vresins.c` - Video resource instance

### Video Format Analysis

UQM uses **DukVid** format (`.duk` files) - a custom format with:
- Frame-based video (likely RLE or similar compression)
- Audio track (likely PCM or ADPCM)
- Subtitle/timing data

### Rust Implementation Plan

#### 3.1 Video System (`rust/src/video/`)
```
rust/src/video/
├── mod.rs           # Module exports
├── types.rs         # VideoFrame, AudioFrame, etc.
├── decoder.rs       # VideoDecoder trait
├── dukvid.rs        # DukVid format decoder
├── player.rs        # VideoPlayer (coordinates decode + render)
├── audio_sync.rs    # Audio/video synchronization
└── ffi.rs           # C FFI bindings
```

**Rust Crates to Consider:**
- `image` - Frame manipulation (already in project)
- `rodio` - Audio playback (already in project)
- For DukVid: Custom decoder (format is UQM-specific)

**Key Types:**
```rust
pub trait VideoDecoder: Send {
    fn open(&mut self, path: &Path) -> Result<VideoInfo>;
    fn decode_frame(&mut self) -> Result<Option<VideoFrame>>;
    fn decode_audio(&mut self) -> Result<Option<AudioChunk>>;
    fn seek(&mut self, time_ms: u32) -> Result<()>;
    fn close(&mut self);
}

pub struct VideoPlayer {
    decoder: Box<dyn VideoDecoder>,
    audio_queue: AudioQueue,
    current_frame: Option<VideoFrame>,
    state: PlayState,
}
```

### Tests (Write First!)

```rust
// tests/video_tests.rs

#[test]
fn test_dukvid_open() { }

#[test]
fn test_dukvid_read_header() { }

#[test]
fn test_dukvid_decode_frame() { }

#[test]
fn test_dukvid_decode_audio() { }

#[test]
fn test_video_player_create() { }

#[test]
fn test_video_player_play_pause() { }

#[test]
fn test_video_player_seek() { }

#[test]
fn test_audio_video_sync() { }
```

### Subagent Tasks

| Task ID | Description | Subagent | Dependencies |
|---------|-------------|----------|--------------|
| V3.1 | Analyze DukVid format | `coder` | None |
| V3.2 | Write video types tests | `coder` | V3.1 |
| V3.3 | Implement VideoFrame/AudioChunk | `coder` | V3.2 |
| V3.4 | Write DukVid decoder tests | `coder` | V3.3 |
| V3.5 | Implement DukVid decoder | `coder` | V3.4 |
| V3.6 | Write VideoPlayer tests | `coder` | V3.5 |
| V3.7 | Implement VideoPlayer | `coder` | V3.6 |
| V3.8 | Implement audio sync | `coder` | V3.7 |
| V3.9 | Create FFI bindings | `coder` | V3.8 |
| V3.10 | Wire into C code | `coder` | V3.9 |

---

## Execution Order

```
Phase 1: Threading (Foundation)
    ├── T1.1-T1.5: Core threading primitives
    ├── T1.6-T1.7: Task system
    └── T1.8-T1.9: FFI + C integration

Phase 2: Resources (Uses Threading)
    ├── R2.1-R2.3: Resource index
    ├── R2.4-R2.5: Resource loader
    ├── R2.6-R2.7: Cache system
    └── R2.8-R2.9: FFI + C integration

Phase 3: Video (Uses Threading + Resources)
    ├── V3.1-V3.3: Video types
    ├── V3.4-V3.5: DukVid decoder
    ├── V3.6-V3.8: VideoPlayer + sync
    └── V3.9-V3.10: FFI + C integration
```

---

## Definition of Done

### Phase 1 Complete When:
- [ ] All threading tests pass
- [ ] Task system tests pass
- [ ] C code can create/join Rust threads via FFI
- [ ] Existing game functionality unchanged
- [ ] `USE_RUST_THREADING` flag controls usage

### Phase 2 Complete When:
- [ ] All resource tests pass
- [ ] .rmp index files parse correctly
- [ ] Resources load from content packs
- [ ] Cache prevents redundant loads
- [ ] `USE_RUST_RESOURCE` flag controls usage

### Phase 3 Complete When:
- [ ] All video tests pass
- [ ] DukVid files decode correctly
- [ ] Cutscenes play with audio sync
- [ ] Intro/ending videos work
- [ ] `USE_RUST_VIDEO` flag controls usage

---

## Risk Mitigation

1. **DukVid Format Unknown**: Analyze C decoder thoroughly before implementing
2. **Threading Deadlocks**: Use Rust's ownership to prevent; add deadlock detection tests
3. **Resource Compatibility**: Test with all content packs
4. **Performance**: Benchmark against C implementation

---

## File Checklist

### New Rust Files to Create:
```
rust/src/threading/
├── mod.rs
├── thread.rs
├── mutex.rs
├── condvar.rs
├── semaphore.rs
├── task.rs
└── ffi.rs

rust/src/resource/
├── index.rs        (NEW)
├── loader.rs       (NEW)
├── cache.rs        (NEW)
└── types.rs        (NEW)

rust/src/video/
├── mod.rs
├── types.rs
├── decoder.rs
├── dukvid.rs
├── player.rs
├── audio_sync.rs
└── ffi.rs

rust/tests/
├── threading_tests.rs
├── resource_tests.rs
└── video_tests.rs
```

### C Files to Modify:
```
sc2/src/config_unix.h          # Add USE_RUST_* flags
sc2/src/libs/threads/          # Conditional Rust calls
sc2/src/libs/task/tasklib.c    # Conditional Rust calls
sc2/src/libs/resource/         # Conditional Rust calls
sc2/src/libs/video/            # Conditional Rust calls
```

### C Headers to Create:
```
sc2/src/libs/threads/rust_threading.h
sc2/src/libs/resource/rust_resource.h
sc2/src/libs/video/rust_video.h
```
