# Phase 14a: Seek Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P14a`

## Prerequisites
- Required: Phase 14 completed
- Expected: `seek()` fully implemented, zero `todo!()` in `aiff.rs`


## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.
## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Full test suite
cargo test --lib --all-features -- aiff

# Zero todo check
grep -c "todo!()" src/sound/aiff.rs
# Expected: 0

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] **ZERO `todo!()` in `aiff.rs`**
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] No `FIXME`/`HACK`/`placeholder` markers

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Seek clamping works (value > max_pcm → returns clamped value)
- [ ] Position sync: data_pos == cur_pcm * file_block after seek
- [ ] Predictor reset: all prev_val entries 0 after seek
- [ ] Decode-after-seek: PCM produces correct data from new position
- [ ] Decode-after-seek: SDX2 produces correct data with reset predictor
- [ ] `aiff.rs` implements all 15+ SoundDecoder trait methods
- [ ] All helper functions implemented (byte readers, f80, chunk parsers)

### Subjective Checks
- [ ] Does seeking to position 0 on an SDX2 file, then decoding, produce identical output to the initial decode-from-open sequence?
- [ ] After seeking to a middle position and decoding PCM, do the output bytes match the expected audio data at that position?
- [ ] Is the seek implementation simple enough that it's obviously correct (clamp, two assignments, array reset, return)?
- [ ] Does the full test suite (parser + PCM + SDX2 + seek) demonstrate that aiff.rs is a complete, working decoder?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()\|unimplemented!()" src/sound/aiff.rs
# Should return NO results — all methods implemented
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All tests pass (parser + PCM + SDX2 + seek)
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] **ZERO `todo!()` remaining in `aiff.rs`**
- [ ] `aiff.rs` is feature-complete for the pure Rust decoder
- [ ] **MILESTONE: aiff.rs complete**

## Failure Recovery
- Return to Phase 14 and fix the seek implementation
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P14a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P14a
- timestamp
- verification result: PASS/FAIL
- test results summary
- gate decision: proceed to P15 (FFI stub) or return to P14
- **MILESTONE**: `aiff.rs` feature-complete
