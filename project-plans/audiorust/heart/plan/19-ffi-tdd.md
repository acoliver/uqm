# Phase 19: FFI TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P19`

## Prerequisites
- Required: Phase P18a (FFI Stub Verification) passed
- Expected: heart_ffi.rs compiling with all stubs

## Requirements Implemented (Expanded)

### REQ-CROSS-FFI-01, REQ-CROSS-FFI-02, REQ-CROSS-FFI-03, REQ-CROSS-FFI-04: FFI Correctness
### REQ-CROSS-GENERAL-03: Unsafe Containment
### REQ-CROSS-GENERAL-08: Error Translation

## Implementation Tasks

### Files to modify
- `rust/src/sound/heart_ffi.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P19`

### Tests to write

**Null Safety (REQ-CROSS-FFI-02)**
1. `test_create_sound_sample_null_decoder` — Null decoder ptr accepted, returns valid ptr
2. `test_destroy_sound_sample_null_ptr` — Null ptr is no-op
3. `test_set_sound_sample_data_null_ptr` — Null ptr is no-op
4. `test_get_sound_sample_data_null_ptr` — Returns null
5. `test_splice_track_null_name` — Null name ptr handled
6. `test_play_channel_null_bank` — Null bank is no-op

**Error Translation (REQ-CROSS-FFI-03)**
7. `test_init_stream_decoder_return_code` — Returns 0 on success
8. `test_playing_stream_returns_int` — Returns 0 or 1
9. `test_playing_track_returns_int` — Returns 0 or track_num+1
10. `test_sound_playing_returns_int` — Returns 0 or 1
11. `test_load_sound_file_null_returns_null` — Null filename → null result

**String Conversion (REQ-CROSS-FFI-04)**
12. `test_c_str_to_option_null` — Null → None
13. `test_c_str_to_option_empty` — Empty string → Some("")
14. `test_c_str_to_option_valid` — Valid CStr → Some(str)

**Callback Wrapping (REQ-CROSS-GENERAL-08)**
15. `test_convert_c_callbacks_null` — Null → None
16. `test_callback_wrapper_default_on_start` — Returns true by default
17. `test_callback_wrapper_default_on_end_chunk` — Returns false by default

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::heart_ffi::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test module added
- [ ] 17+ test functions
- [ ] Tests reference REQ-CROSS-FFI-* requirements

## Semantic Verification Checklist (Mandatory)
- [ ] All null pointer paths tested
- [ ] Error code mapping verified (Result → c_int)
- [ ] String conversion edge cases covered
- [ ] Callback wrapper behavior matches trait defaults
- [ ] Tests use actual FFI function signatures (not just internal helpers)

## Deferred Implementation Detection (Mandatory)
N/A — TDD phase

## Success Criteria
- [ ] 17+ tests written and compiling

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/heart_ffi.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P19.md`
