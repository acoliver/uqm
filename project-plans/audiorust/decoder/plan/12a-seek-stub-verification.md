# Phase 12a: Seek Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P12a`

## Prerequisites
- Required: Phase 12 completed
- Expected: `seek()` stub confirmed


## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.
## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# All existing tests pass
cargo test --lib --all-features -- aiff

# Verify seek stub exists
grep "fn seek" src/sound/aiff.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred implementation check
grep -n "todo!()" src/sound/aiff.rs
```

## Structural Verification Checklist
- [ ] `fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32>` exists in trait impl
- [ ] Contains `todo!()`
- [ ] `cargo check --all-features` passes
- [ ] All existing tests pass (parser + PCM + SDX2)
- [ ] `cargo fmt --all --check` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] `seek()` is in the `impl SoundDecoder for AiffDecoder` block (reachable via trait)
- [ ] `todo!()` appears in seek only (decode_pcm and decode_sdx2 are implemented)
- [ ] All parser tests pass (no regression)
- [ ] All PCM decode tests pass (no regression)
- [ ] All SDX2 decode tests pass (no regression)

### Subjective Checks
- [ ] Is the seek() signature correct per the SoundDecoder trait (takes u32, returns DecodeResult<u32>)?
- [ ] After this phase, is the seek function the ONLY remaining `todo!()` in aiff.rs?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only
```

## Success Criteria
- [ ] `seek()` stub confirmed with correct signature
- [ ] All existing tests pass (no regression)
- [ ] seek is the only remaining `todo!()`

## Failure Recovery
- Return to Phase 12 if stub is missing or has wrong signature
- rollback: N/A (minimal changes expected)

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P12a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P12a
- timestamp
- verification result: PASS/FAIL
- gate decision: proceed to P13 or return to P12
