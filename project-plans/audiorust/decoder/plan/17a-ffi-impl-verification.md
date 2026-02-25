# Phase 17a: FFI Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P17a`

## Prerequisites
- Required: Phase 17 completed
- Expected: All FFI functions implemented, zero `todo!()` in `aiff_ffi.rs`


## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.
## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Full test suite
cargo test --lib --all-features -- aiff

# FFI tests specifically
cargo test --lib --all-features -- aiff_ffi

# Zero todo check
grep -c "todo!()" src/sound/aiff_ffi.rs
# Expected: 0

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff_ffi.rs || echo "CLEAN"

# Verify symbol export
grep "no_mangle" src/sound/aiff_ffi.rs
grep "rust_aifa_DecoderVtbl" src/sound/aiff_ffi.rs
```

## Structural Verification Checklist
- [ ] **ZERO `todo!()` in `aiff_ffi.rs`**
- [ ] All 12 vtable functions implemented
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes
- [ ] No `FIXME`/`HACK`/`placeholder` markers
- [ ] `#[no_mangle]` on vtable static

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Init allocates + propagates formats via init_module()/init() + stores + sets need_swap=false (matching wav_ffi.rs)
- [ ] Open reads file via UIO, calls open_from_bytes(), updates base struct fields (frequency, format, length, is_null, need_swap) (REQ-FF-6, REQ-FF-7)
- [ ] Open failure returns 0, logs error (REQ-FF-8)
- [ ] Decode maps: Ok(n)→n, EndOfFile→0, Err→0 (REQ-FF-9, never returns negative)
- [ ] All functions handle null decoder and null rust_decoder pointers (REQ-FF-10)
- [ ] Seek calls dec.seek(), returns Ok value or pcm_pos on error (REQ-FF-13)
- [ ] Close calls dec.close() if pointer non-null (REQ-FF-15)
- [ ] Term drops the Box and nulls out the pointer (REQ-FF-5)

### Subjective Checks
- [ ] Does the FFI vtable Init function match the wav_ffi.rs Init pattern (allocate, propagate formats, store)?
- [ ] Does Open correctly handle the case where read_uio_file returns None (returns 0, not panic)?
- [ ] Does the Decode function correctly prevent negative return values (C caller expects non-negative)?
- [ ] Is there any path where the Box could be leaked (created but never freed)?
- [ ] Is there any path where a use-after-free could occur (accessing rust_decoder after Term)?
- [ ] Does the format mapping in Open correctly convert all 4 AudioFormat variants to the corresponding C format codes?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()\|unimplemented!()" src/sound/aiff_ffi.rs
# Should return NO results
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff_ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All tests pass (aiff.rs + aiff_ffi.rs)
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] **ZERO `todo!()` remaining in `aiff_ffi.rs`**
- [ ] `aiff_ffi.rs` is feature-complete
- [ ] **MILESTONE: Pure Rust decoder + FFI bridge complete**

## Failure Recovery
- Return to Phase 17 and fix failing tests or compilation errors
- If Open's format mapping is wrong, cross-reference with dukaud_ffi.rs Open implementation
- If Box lifecycle is incorrect, trace Init/Term/Open paths carefully
- rollback: `git checkout -- rust/src/sound/aiff_ffi.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P17a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P17a
- timestamp
- verification result: PASS/FAIL
- test results summary
- gate decision: proceed to P18 (C integration) or return to P17
- **MILESTONE**: aiff_ffi.rs feature-complete
