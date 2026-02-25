# Phase 20: FFI Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P20`

## Prerequisites
- Required: Phase P19a (FFI TDD Verification) passed
- Expected: 17+ tests in heart_ffi.rs

## Requirements Implemented (Expanded)

All REQ-CROSS-FFI-* (4) and REQ-CROSS-GENERAL-03,08 requirements fully implemented.

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
  - marker: `@requirement REQ-CROSS-FFI-01..04, REQ-CROSS-GENERAL-03,08`

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
- `GetFirstTrackSubtitle`/`GetNextTrackSubtitle`: Box::into_raw for SubtitleRef
- `LoadSoundFile`/`LoadMusicFile`: Box::into_raw for return values
- `CCallbackWrapper`: stores raw C function pointers, calls them via `unsafe`
- All `unsafe` blocks must be documented with safety invariant comments

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
