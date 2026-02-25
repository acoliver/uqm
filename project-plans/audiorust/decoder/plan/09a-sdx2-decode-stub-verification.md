# Phase 09a: SDX2 Decode Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P09a`

## Prerequisites
- Required: Phase 09 completed
- Expected: `decode_sdx2()` stub confirmed, dispatch wired

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# All existing tests pass
cargo test --lib --all-features -- aiff

# Verify stub exists
grep "fn decode_sdx2" src/sound/aiff.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred implementation check
grep -n "todo!()" src/sound/aiff.rs
```

## Structural Verification Checklist
- [ ] `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` exists
- [ ] Contains `todo!()`
- [ ] `cargo check --all-features` passes
- [ ] All existing tests pass (parser + PCM decode)
- [ ] `cargo fmt --all --check` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] SDX2 path reachable via `decode()` dispatch for `CompressionType::Sdx2`
- [ ] `todo!()` present in decode_sdx2 and seek only (decode_pcm is implemented)
- [ ] All parser tests still pass (no regression)
- [ ] All PCM decode tests still pass (no regression)

### Subjective Checks
- [ ] Is the decode_sdx2 stub properly wired into the decode() dispatch for CompressionType::Sdx2?
- [ ] Does the decode_sdx2 signature match what the TDD phase expects to test against?
- [ ] Are there any side effects from Phase 09 that could break existing PCM decode behavior?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: decode_sdx2, seek
```

## Success Criteria
- [ ] `decode_sdx2()` stub confirmed with correct signature
- [ ] All existing tests pass
- [ ] Dispatch to SDX2 path confirmed

## Failure Recovery
- Return to Phase 09 if stub is missing or signature is wrong
- rollback: N/A (minimal changes expected)

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P09a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P09a
- timestamp
- verification result: PASS/FAIL
- gate decision: proceed to P10 or return to P09
