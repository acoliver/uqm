# Phase 18: FFI Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P18`

## Prerequisites
- Required: Phase P17a (Control + FileInst Implementation Verification) passed
- Expected: All 6 Rust modules (types, stream, trackplayer, music, sfx, control, fileinst) fully implemented

## Requirements Implemented (Expanded)

### REQ-CROSS-FFI-01: #[no_mangle] + extern "C"
**Requirement text**: All FFI functions use `#[no_mangle] pub extern "C" fn`.

Behavior contract:
- GIVEN: All Rust API functions are implemented
- WHEN: FFI shims are created
- THEN: Every C-callable function has correct `#[no_mangle] pub extern "C" fn` signature

### REQ-CROSS-FFI-02: Pointer Validation
**Requirement text**: All incoming pointers checked for null before dereference.

Behavior contract:
- GIVEN: C code passes potentially null pointers
- WHEN: FFI shim receives a pointer
- THEN: Null check is performed before any dereference; null returns early with safe default

### REQ-CROSS-FFI-03: Error Translation
**Requirement text**: Result<T, AudioError> translated to C-compatible return values.

Behavior contract:
- GIVEN: Rust API returns Result
- WHEN: FFI shim wraps the call
- THEN: Ok → success value (0, 1, or pointer), Err → logged + failure value (0, -1, or null)

### REQ-CROSS-FFI-04: String Conversion
**Requirement text**: C strings (`*const c_char`) converted to `&str` safely; Rust strings returned as leaked CString or thread-local cache.

### REQ-CROSS-GENERAL-03: Unsafe Containment
**Requirement text**: All unsafe code confined to heart_ffi.rs (and types.rs FFI wrappers).

### REQ-CROSS-GENERAL-08: C Callback Wrapping
**Requirement text**: C function pointer callbacks wrapped in StreamCallbacks trait objects.

## Implementation Tasks

### Files to create
- `rust/src/sound/heart_ffi.rs` — All 60+ `#[no_mangle] pub extern "C" fn` shims
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P18`
  - marker: `@requirement REQ-CROSS-FFI-01, REQ-CROSS-FFI-02, REQ-CROSS-FFI-03, REQ-CROSS-FFI-04, REQ-CROSS-GENERAL-03, REQ-CROSS-GENERAL-08`

### Files to modify
- `rust/src/sound/mod.rs` — Add `pub mod heart_ffi;`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P18`

### Stub contents (all `todo!()` or minimal safe passthrough)

**Stream FFI (18 functions)**
- `InitStreamDecoder`, `UninitStreamDecoder`
- `TFB_CreateSoundSample`, `TFB_DestroySoundSample`
- `TFB_SetSoundSampleData`, `TFB_GetSoundSampleData`
- `TFB_SetSoundSampleCallbacks`, `TFB_GetSoundSampleDecoder`
- `PlayStream`, `StopStream`, `PauseStream`, `ResumeStream`, `SeekStream`, `PlayingStream`
- `TFB_FindTaggedBuffer`, `TFB_TagBuffer`, `TFB_ClearBufferTag`
- `SetMusicStreamFade`, `GraphForegroundStream`

**Track Player FFI (14 functions)**
- `SpliceTrack`, `SpliceMultiTrack`
- `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`, `PlayingTrack`
- `GetTrackPosition`, `GetTrackSubtitle`
- `GetFirstTrackSubtitle`, `GetNextTrackSubtitle`, `GetTrackSubtitleText`
- `FastReverse_Smooth`, `FastForward_Smooth`, `FastReverse_Page`, `FastForward_Page`

**Music FFI (12 functions)**
- `PLRPlaySong`, `PLRStop`, `PLRPlaying`, `PLRSeek`, `PLRPause`, `PLRResume`
- `snd_PlaySpeech`, `snd_StopSpeech`
- `SetMusicVolume`, `FadeMusic`
- `DestroyMusic`

**SFX FFI (8 functions)**
- `PlayChannel`, `StopChannel`, `ChannelPlaying`
- `SetChannelVolume`, `UpdateSoundPosition`
- `GetPositionalObject`, `SetPositionalObject`
- `DestroySound`

**Control FFI (6 functions)**
- `StopSound`, `SoundPlaying`, `WaitForSoundEnd`
- `SetSFXVolume`, `SetSpeechVolume`
- `InitSound`, `UninitSound`

**File Loading FFI (2 functions)**
- `LoadSoundFile`, `LoadMusicFile`

**C Callback Wrapper**
- `CCallbackWrapper` struct implementing StreamCallbacks
- `convert_c_callbacks` helper

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `heart_ffi.rs` created with 60+ functions
- [ ] All functions have `#[no_mangle] pub extern "C" fn`
- [ ] `mod.rs` updated
- [ ] `cargo check` passes

## Semantic Verification Checklist (Mandatory)
- [ ] All C function names match spec §5
- [ ] Parameter types use `c_int`, `c_uint`, `*mut c_void`, `*const c_char`
- [ ] Return types are C-compatible (c_int, *mut, etc.)
- [ ] CCallbackWrapper implements StreamCallbacks

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "todo!()" rust/src/sound/heart_ffi.rs | wc -l  # Expected > 0 (stubs)
```

## Success Criteria
- [ ] All FFI signatures compile
- [ ] Module registered
- [ ] Symbol names visible to linker

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and remove heart_ffi.rs

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P18.md`
