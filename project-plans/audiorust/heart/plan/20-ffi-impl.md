# Phase 20: FFI Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P20`

## Prerequisites
- Required: Phase P19a (FFI TDD Verification) passed
- Expected: 17+ tests in heart_ffi.rs

## Requirements Implemented (Expanded)

### REQ-CROSS-FFI-01: C-Compatible Function Signatures
- GIVEN: 60+ audio API functions
- WHEN: C code calls any audio function
- THEN: The `#[no_mangle] pub extern "C" fn` shim converts C types to Rust types, calls the internal Rust API, and converts results back to C types

### REQ-CROSS-FFI-02: Null Pointer Safety
- GIVEN: Any FFI function receiving pointer parameters
- WHEN: A null pointer is passed
- THEN: The function returns a safe error value (0, null, or no-op) without panicking

### REQ-CROSS-FFI-03: No C FFI Round-Trip to Mixer
- GIVEN: Rust audio heart calling mixer functions
- WHEN: Mixer API is used
- THEN: Calls go directly to Rust mixer module, never through C FFI shims

### REQ-CROSS-FFI-04: Symbol Export
- GIVEN: A built Rust static library
- WHEN: Linked into the C binary
- THEN: All 60+ audio symbols are visible to the linker via `#[no_mangle]`

### REQ-CROSS-GENERAL-03: Unsafe Confinement
- GIVEN: Unsafe operations (pointer derefs, FFI calls)
- WHEN: Code requires unsafe
- THEN: All unsafe is confined to heart_ffi.rs with `// SAFETY:` comments

### REQ-CROSS-GENERAL-08: Error Convention
- GIVEN: Internal Rust Result<T, AudioError> returns
- WHEN: Crossing FFI boundary
- THEN: bool→c_int (1/0), counts→0, pointers→null; errors logged before conversion

### Pseudocode traceability
- Stream FFI: pseudocode `heart_ffi.md` lines 1-97
- Track Player FFI: pseudocode `heart_ffi.md` lines 100-175
- Music FFI: pseudocode `heart_ffi.md` lines 180-222
- SFX FFI: pseudocode `heart_ffi.md` lines 230-257
- Control FFI: pseudocode `heart_ffi.md` lines 260-283
- File Loading FFI: pseudocode `heart_ffi.md` lines 290-304
- Callback Wrapper: pseudocode `heart_ffi.md` lines 310-340

## Implementation Tasks

### Files to modify
- `rust/src/sound/heart_ffi.rs` — Replace all `todo!()` with FFI shim implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P20`
  - marker: `@requirement REQ-CROSS-FFI-01, REQ-CROSS-FFI-02, REQ-CROSS-FFI-03, REQ-CROSS-FFI-04, REQ-CROSS-GENERAL-03, REQ-CROSS-GENERAL-08`

### Implementation details for each function category

**Pattern: every function follows**
```
1. Null-check all pointer parameters → return safe default if null
2. Convert C types to Rust types (CStr→&str, *mut→&mut, etc.)
3. Call corresponding Rust API function
4. Convert Result to C return value (log errors)
5. Return
```

**Specific considerations**
- `SpliceTrack`: UNICODE* (`*const u16`) text requires UTF-16→UTF-8 conversion
- `GetTrackSubtitle`: Must return `*const c_char` — use thread-local `RefCell<CString>` cache
- `GetFirstTrackSubtitle`/`GetNextTrackSubtitle`: Return raw `NonNull<SoundChunk>` pointers cast to `*mut c_void` (zero allocation, matches C behavior — borrowed pointers valid while track state unchanged)
- `LoadSoundFile`: `Box::into_raw(Box::new(SoundBank))` for return values (SoundBank is NOT Arc-wrapped — it's single-owner)
- `LoadMusicFile`: `Arc::into_raw(Arc::new(Mutex::new(sample)))` per all-Arc strategy
- `CCallbackWrapper`: stores raw C function pointers, calls them via `unsafe`
- All `unsafe` blocks must be documented with safety invariant comments

**All-Arc SoundSample Pointer Strategy**

Every `SoundSample` pointer at the FFI boundary uses `Arc<Mutex<SoundSample>>`:
- `TFB_CreateSoundSample` → `Arc::new(Mutex::new(sample))` → `Arc::into_raw()` as `*mut c_void`
- `PlayStream`, `StopStream`, accessors → `Arc::increment_strong_count` + `Arc::from_raw` (borrow without ownership change)
- `TFB_DestroySoundSample` → `Arc::from_raw` (consuming — decrements refcount, frees if last ref)
- `LoadMusicFile` → same pattern: `Arc::into_raw()`, `DestroyMusic` → `Arc::from_raw`

Note: `SoundBank` (from `LoadSoundFile`) uses `Box`, not `Arc`, because banks are single-owner (no sharing).

**PlayChannel FFI Handle Resolution (Technical Review Issue #6)**

The C API `PlayChannel(snd, index, notsfx, priority, positional)` receives `snd` as `*mut c_void` (opaque SOUND handle). Resolution:

1. The SOUND handle is a `*mut SoundBank` — it was created by `LoadSoundFile` → `get_sound_bank_data` → `Box::into_raw`.
2. In `PlayChannel` FFI shim: cast `snd` back to `&SoundBank` via `Box::from_raw` (or just `&*snd.cast::<SoundBank>()` with null check).
3. Use `index` to look up the specific sample: `bank.samples[index as usize]`.
4. If `index` is out of bounds or `samples[index]` is `None`, return error (no-op in C convention).
5. Pass the resolved sample to `sfx::play_channel()`.

The SOUND handle lifecycle:
- Created: `LoadSoundFile` → `Box::into_raw(Box::new(SoundBank))` → returns `*mut c_void`
- Used: `PlayChannel` → `&*(snd as *mut SoundBank)` → `bank.samples[index]`
- Destroyed: `DestroySound` → `Box::from_raw(snd as *mut SoundBank)` → drops

Document this pattern with `// SAFETY:` comments in the FFI shim.

### GraphForegroundStream scope buffer FFI rendering path

The C graphics code needs to read the scope (oscilloscope) data to render the waveform on screen. The data flow is:

1. **Rust produces scope data**: The `graph_foreground_stream()` function (pseudocode `stream.md` §14, lines 400-460) reads the ring buffer, applies AGC/VAD, and writes scaled waveform amplitudes into an output array `data[0..width]` where each element is a Y-coordinate.

2. **FFI function signature**: The FFI shim exposes this as:
   ```c
   // C declaration in audio_heart_rust.h:
   uint32_t GraphForegroundStream(int32_t* data, uint32_t width, uint32_t height, bool want_speech);
   ```
   - `data`: Caller-allocated `int32_t` array of at least `width` elements. Rust writes the computed Y-coordinates into this buffer.
   - `width`: Number of horizontal pixels (= number of samples to compute).
   - `height`: Vertical pixel range for the waveform.
   - `want_speech`: If true and speech is active, use speech source; otherwise use music source.
   - Returns: Number of samples written (0 if no active stream, otherwise `width`).

3. **FFI shim implementation**:
   ```rust
   #[no_mangle]
   pub extern "C" fn GraphForegroundStream(
       data: *mut i32, width: u32, height: u32, want_speech: c_int,
   ) -> u32 {
       if data.is_null() || width == 0 || height == 0 { return 0; }
       // SAFETY: Caller guarantees data points to at least width i32 elements
       let slice = unsafe { std::slice::from_raw_parts_mut(data, width as usize) };
       stream::graph_foreground_stream(slice, width as usize, height as usize, want_speech != 0)
   }
   ```

4. **C graphics rendering**: The C code in the comm screen renderer:
   - Allocates a local `int32_t data[MAX_WIDTH]` array on the stack
   - Calls `GraphForegroundStream(data, width, height, want_speech)`
   - Iterates `data[0..returned_count]`, drawing line segments between consecutive Y-coordinates to produce the oscilloscope waveform

5. **No shared ring buffer pointer**: The C code does **not** get a raw pointer to the Rust ring buffer. The scope ring buffer is internal to Rust. The `GraphForegroundStream` function performs all ring buffer reading, AGC, and scaling internally and writes pre-computed Y-coordinates into the caller's buffer. This avoids unsafe shared memory and synchronization issues — the only cross-language data transfer is the output array.

### Safety documentation requirements
Every `unsafe` block in heart_ffi.rs must have a `// SAFETY:` comment explaining:
- What invariant the caller guarantees
- Why the operation is sound

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::heart_ffi::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Verify all FFI symbols exported
cd /Users/acoliver/projects/uqm/rust && cargo build --lib --all-features
nm target/debug/libuqm_rust.a 2>/dev/null | grep " T " | grep -c "InitStreamDecoder\|PLRPlaySong\|PlayChannel\|SpliceTrack\|LoadSoundFile\|StopSound"
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from heart_ffi.rs
- [ ] All tests pass
- [ ] 60+ FFI functions implemented
- [ ] All `unsafe` blocks have `// SAFETY:` comments
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] Every FFI function delegates to the correct Rust API
- [ ] Null pointers handled safely in every function
- [ ] Error codes match C expectations
- [ ] String conversion handles edge cases (null, empty, invalid UTF-8)
- [ ] CCallbackWrapper correctly wraps C function pointers
- [ ] Symbols exported in static library

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/heart_ffi.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 17+ tests pass
- [ ] Zero deferred implementations
- [ ] All 60+ FFI symbols exported
- [ ] Complete C↔Rust boundary operational

## Failure Recovery
- rollback: `git stash`
- blocking issues: If C function signature doesn't match, update both sides

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P20.md`
