# Phase 19a: FFI TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P19a`

## Prerequisites
- Required: Phase P19 completed
- Expected: heart_ffi.rs test module with 15+ tests

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::heart_ffi::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] Test module exists in `heart_ffi.rs`
- [ ] At least 15 test functions present
- [ ] All tests compile
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] Test count >= 15: `cargo test --lib --all-features -- sound::heart_ffi::tests 2>&1 | grep "test result"`
- [ ] Specific tests exist for: `test_null_pointer_handling`, `test_error_code_translation`, `test_music_ref_roundtrip`, `test_sound_bank_roundtrip`, `test_callback_wrapper`

### Subjective checks
- [ ] Null pointer tests verify every function that accepts a pointer handles NULL gracefully — does it return an error code, not crash?
- [ ] Error code translation tests verify AudioResult → C integer mapping — does `Ok(())` become 1 (TRUE)? Does `Err(...)` become 0 (FALSE)?
- [ ] MusicRef roundtrip tests verify Box::into_raw → use → Box::from_raw lifecycle — does the pointer survive the roundtrip?
- [ ] SoundBank roundtrip tests verify the opaque SOUND handle lifecycle — can a bank be created, used in PlayChannel, and destroyed?
- [ ] CCallbackWrapper tests verify C function pointer wrapping — can a Rust closure be wrapped and called through the C interface?
- [ ] UTF-16 conversion tests for SpliceTrack — does `*const u16` text correctly convert to Rust &str?
- [ ] Tests use C-compatible types (c_int, *mut c_void) not Rust-native types

## Deferred Implementation Detection
N/A — TDD phase, stubs still have `todo!()`.

## Success Criteria
- [ ] 15+ tests written and compiling
- [ ] FFI boundary behavior tested: null pointers, error codes, type conversions, lifecycle
- [ ] Tests reflect real C caller patterns

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/heart_ffi.rs`
- blocking issues: If C type definitions need clarification, reference actual C headers

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P19a.md`
