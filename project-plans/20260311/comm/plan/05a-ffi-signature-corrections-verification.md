# Phase 05a: FFI Signature Corrections Verification

## Phase ID
`PLAN-20260314-COMM.P05a`

## Prerequisites
- Required: Phase 05 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ResponseEntry.response_func` type is `Option<extern "C" fn(u32)>`
- [ ] All FFI function signatures in `ffi.rs` match spec §14
- [ ] All FFI function declarations in `rust_comm.h` match Rust exports
- [ ] Thread-local subtitle buffer implemented in `rust_GetSubtitle`
- [ ] New FFI exports: `rust_SpliceMultiTrack`, seek functions, `rust_PlayingTrack`

## Semantic Verification Checklist

### Response Callback ABI
- [ ] `test_ffi_response_callback_receives_ref` — callback receives response_ref as u32 argument
- [ ] `test_ffi_response_callback_correct_ref` — multiple responses, each callback gets correct ref
- [ ] `test_ffi_response_execute_no_callback` — response without callback returns ref, no crash

### Subtitle Safety
- [ ] `test_ffi_get_subtitle_stable_pointer` — pointer remains valid after acquiring and releasing state
- [ ] `test_ffi_get_subtitle_null_when_none` — returns null when no subtitle active
- [ ] `test_ffi_get_subtitle_updates` — returns different pointer after subtitle change

### Track FFI
- [ ] `test_ffi_splice_track_with_timestamps` — timestamps parameter accepted
- [ ] `test_ffi_splice_track_with_callback` — callback stored for later dispatch
- [ ] `test_ffi_splice_multi_track` — multiple clips merged into single phrase
- [ ] `test_ffi_jump_track_no_offset` — JumpTrack takes no arguments, skips current phrase

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/ffi.rs rust/src/comm/response.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P05a.md`
