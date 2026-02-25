# Phase 16a: FFI TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P16a`

## Prerequisites
- Required: Phase 16 completed
- Expected: FFI tests in `aiff_ffi.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile-only check
cargo test --lib --all-features -- aiff_ffi --no-run

# Count FFI test functions
grep -c "#\[test\]" src/sound/aiff_ffi.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `#[cfg(test)] mod tests` block exists in `aiff_ffi.rs`
- [ ] Tests compile: `cargo test --lib --all-features -- aiff_ffi --no-run`
- [ ] At least 9 test functions defined (including Open null decoder test)
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Vtable static exists test (REQ-FF-2)
- [ ] GetName returns "Rust AIFF" string (REQ-FF-12)
- [ ] GetStructSize returns value >= size_of TFB_SoundDecoder (REQ-FF-11)
- [ ] Null pointer handling tested for all functions (REQ-FF-10)
- [ ] InitModule/TermModule lifecycle tested (REQ-FF-3)
- [ ] Init/Term allocation lifecycle tested — Init creates Box, Term drops it (REQ-FF-4, REQ-FF-5)

### Subjective Checks
- [ ] Do the null pointer tests verify that each function returns a safe default (0, null, etc.) when passed a null decoder?
- [ ] Does the InitModule test verify that the Mutex actually stores the formats?
- [ ] Does the Init test verify that the rust_decoder pointer is non-null after Init?
- [ ] Does the Term test verify that the rust_decoder pointer is null after Term?
- [ ] Are the tests testing FFI boundary behavior (C-visible semantics), not internal Rust implementation details?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff_ffi.rs
# Open and Decode should still have todo!()
```

## Success Criteria
- [ ] All FFI test functions compile
- [ ] Tests would fail when run (RED phase for Open/Decode)
- [ ] Vtable, null pointer, and lifecycle tests present
- [ ] Tests verify C-visible behavior

## Failure Recovery
- Return to Phase 16 and add missing test cases
- If FFI types don't match, check ffi.rs for correct struct definitions
- rollback: `git checkout -- rust/src/sound/aiff_ffi.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P16a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P16a
- timestamp
- verification result: PASS/FAIL
- test count
- gate decision: proceed to P17 or return to P16
