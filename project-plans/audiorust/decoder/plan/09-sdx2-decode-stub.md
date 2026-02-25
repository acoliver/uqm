# Phase 09: SDX2 Decode Stub

## Phase ID
`PLAN-20260225-AIFF-DECODER.P09`

## Prerequisites
- Required: Phase 08 completed (PCM decode working)
- Expected files: `rust/src/sound/aiff.rs` with working PCM decode

## Requirements Implemented (Expanded)

### REQ-DS-1 (dispatch confirmation): SDX2 Decode Dispatch
**Requirement text**: Confirm `decode_sdx2()` stub exists and is reachable from `decode()` dispatch.

Behavior contract:
- GIVEN: `decode_sdx2()` has a `todo!()` stub from Phase 06
- WHEN: `decode()` is called on a decoder opened with `CompressionType::Sdx2`
- THEN: The dispatch match reaches `decode_sdx2()` (which panics with `todo!()` in this phase)

### Stub Verification
Behavior contract:
- GIVEN: The `decode_sdx2()` stub was created in Phase 06
- WHEN: `cargo check` is run
- THEN: The code compiles with the correct signature `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>`
- AND: The `todo!()` body is preserved (will be replaced in Phase 11)

Why it matters:
- Confirms the SDX2 decode path is reachable via the dispatch in decode()
- Stub already exists from Phase 06; this phase verifies it's properly wired

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P09`
  - marker: `@requirement REQ-DS-1`
  - Verify: `decode_sdx2()` stub exists with correct signature
  - Verify: `decode()` dispatch reaches `decode_sdx2()` for `CompressionType::Sdx2`
  - No implementation changes needed if stub is already correct from Phase 06

### Pseudocode traceability
- Confirms stub for pseudocode lines: 270–315

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# All existing tests pass
cargo test --lib --all-features -- aiff

# Verify SDX2 stub exists
grep "fn decode_sdx2" src/sound/aiff.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `decode_sdx2()` method exists with `todo!()` body
- [ ] Correct signature: `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>`
- [ ] `decode()` dispatch matches on `CompressionType::Sdx2` → `self.decode_sdx2(buf)`
- [ ] All existing tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] SDX2 decode path reachable from `decode()` dispatch
- [ ] No fake SDX2 behavior (only `todo!()`)
- [ ] PCM decode unaffected

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: decode_sdx2, seek
```

## Success Criteria
- [ ] `decode_sdx2()` stub confirmed
- [ ] All tests pass
- [ ] Dispatch to SDX2 path confirmed

## Failure Recovery
- rollback steps: N/A (minimal changes)
- blocking issues: None expected

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P09.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P09
- timestamp
- files changed: `rust/src/sound/aiff.rs` (verified/minor)
- tests added/updated: None
- verification outputs
- semantic verification summary
