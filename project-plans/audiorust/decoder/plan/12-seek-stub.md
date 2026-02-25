# Phase 12: Seek Stub

## Phase ID
`PLAN-20260225-AIFF-DECODER.P12`

## Prerequisites
- Required: Phase 11 completed (SDX2 decode working)
- Expected files: `rust/src/sound/aiff.rs` with working PCM + SDX2 decode

## Requirements Implemented (Expanded)

### REQ-SK-1 (stub confirmation): Seek Stub
**Requirement text**: Confirm `seek()` stub exists with correct signature and is reachable via SoundDecoder trait.

Behavior contract:
- GIVEN: `seek()` has a `todo!()` stub from Phase 03
- WHEN: `cargo check` is run
- THEN: The code compiles with signature `fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32>`
- AND: The `todo!()` body is preserved (will be replaced in Phase 14)

### Stub Verification
Behavior contract:
- GIVEN: The `seek()` method is defined in `impl SoundDecoder for AiffDecoder`
- WHEN: A caller invokes `decoder.seek(position)` through the trait
- THEN: The dispatch reaches the `seek()` stub (which panics with `todo!()` in this phase)

Why it matters:
- Seek already has a stub from Phase 03
- This phase confirms it's ready for TDD

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P12`
  - marker: `@requirement REQ-SK-1`
  - Verify: `seek()` stub exists with correct signature
  - No changes needed if stub is already correct

### Pseudocode traceability
- Confirms stub for pseudocode lines: 300–312

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo check --all-features
cargo test --lib --all-features -- aiff
grep "fn seek" src/sound/aiff.rs
```

## Structural Verification Checklist
- [ ] `seek()` method exists with `todo!()` body
- [ ] Correct signature in trait impl
- [ ] All existing tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] `seek()` is reachable from `SoundDecoder` trait
- [ ] No fake seek behavior

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only
```

## Success Criteria
- [ ] `seek()` stub confirmed
- [ ] All tests pass

## Failure Recovery
- rollback steps: N/A
- blocking issues: None expected

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P12.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P12
- timestamp
- files changed: `rust/src/sound/aiff.rs` (verified)
- tests added/updated: None
- verification outputs
- semantic verification summary
