# Phase 07a: PCM Decode TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P07a`

## Prerequisites
- Required: Phase 07 completed
- Expected: PCM decode tests in `aiff.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile-only check (tests should compile but fail at runtime)
cargo test --lib --all-features -- test_decode_pcm --no-run

# Count PCM decode test functions
grep -c "test_decode_pcm" src/sound/aiff.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify specific test names exist
for test in mono16 stereo16 mono8 stereo8 partial_buffer multiple_calls eof exact_fit returns_byte_count position_update; do
  grep -q "test_decode_pcm.*$test\|test_decode_pcm_$test" src/sound/aiff.rs && echo "PASS: $test" || echo "FAIL: $test"
done
```

## Structural Verification Checklist
- [ ] At least 11 test functions with `test_decode_pcm` in name
- [ ] Tests compile: `cargo test --lib --all-features -- test_decode_pcm --no-run`
- [ ] Tests use synthetic AIFF data (not external files)
- [ ] Tests verify output buffer contents, not just return values
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Mono8, Mono16, Stereo8, Stereo16 all have at least one test
- [ ] 8-bit signed→unsigned conversion test checks specific byte values (e.g., 0x80→0x00, 0x00→0x80, 0x7F→0xFF)
- [ ] EOF condition test verifies exact `DecodeError::EndOfFile` variant
- [ ] Partial decode test verifies correct frame count calculation
- [ ] Sequential decode test (multiple calls) verifies continuation from previous position
- [ ] Return value (byte count) explicitly checked against expected dec_pcm * block_align

### Subjective Checks
- [ ] Do the tests construct AIFF data with known PCM sample values that are verifiable in the output?
- [ ] Does the 8-bit conversion test verify the wrapping_add(128) behavior for boundary values (-128, -1, 0, 127)?
- [ ] Does the partial buffer test use a buffer size that isn't a multiple of block_align to verify truncation?
- [ ] Does the multi-call test verify that data_pos advances correctly between calls?
- [ ] Would any of these tests pass with decode_pcm() still as `todo!()`? (They should not)

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# decode_pcm and decode_sdx2 should still have todo!()
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase — stubs panic)
- [ ] All REQ-DP-1..6 requirements covered by tests
- [ ] Tests check output buffer contents, not just Ok/Err

## Failure Recovery
- Return to Phase 07 and add missing test cases
- If synthetic AIFF construction is complex, simplify to minimal valid files
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P07a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P07a
- timestamp
- verification result: PASS/FAIL
- test count
- gate decision: proceed to P08 or return to P07
