# Phase 10a: SDX2 Decode TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P10a`

## Prerequisites
- Required: Phase 10 completed
- Expected: SDX2 decode tests in `aiff.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile-only check
cargo test --lib --all-features -- test_decode_sdx2 --no-run

# Count SDX2 decode test functions
grep -c "test_decode_sdx2" src/sound/aiff.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify test variety
for test in even odd negative stereo clamp predictor eof zero; do
  grep -q "test_decode_sdx2.*$test\|test_decode_sdx2_.*$test" src/sound/aiff.rs && echo "PASS: $test" || echo "FAIL: $test"
done
```

## Structural Verification Checklist
- [ ] At least 12 test functions with `test_decode_sdx2` in name
- [ ] Tests compile: `cargo test --lib --all-features -- test_decode_sdx2 --no-run`
- [ ] `build_aifc_sdx2_file()` helper exists
- [ ] Tests verify exact output values (not just success/failure)
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Even byte (no delta) tested with exact expected value (e.g., byte=16 → v=(16*16)<<1=512)
- [ ] Odd byte (delta mode) tested with exact expected value including predictor addition
- [ ] Negative compressed byte tested (e.g., byte=-16 → v=(-16*16)<<1=-512)
- [ ] Stereo interleaving tested with per-channel predictor independence
- [ ] Clamping tested for positive overflow (value > 32767 → 32767)
- [ ] Clamping tested for negative overflow (value < -32768 → -32768)
- [ ] EOF tested with `DecodeError::EndOfFile`
- [ ] Position update (cur_pcm, data_pos) tested

### Subjective Checks
- [ ] Does the SDX2 decode algorithm test verify the exact mathematical formula `v = (sample * abs(sample)) << 1`?
- [ ] Does the delta mode test verify that the predictor value from a previous frame is correctly added when the LSB is 1?
- [ ] Does the stereo test verify that channel 0 predictor is independent from channel 1 predictor (modifying ch0 doesn't affect ch1)?
- [ ] Are the clamping tests using values that would actually exceed the i16 range after the algorithm computation?
- [ ] Does the predictor accumulation test verify that predictor state builds up correctly across multiple frames in delta mode?
- [ ] Would any of these tests pass with a trivial `todo!()` or fake implementation?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# decode_sdx2 and seek should still have todo!()
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase — stubs panic)
- [ ] All REQ-DS-1..8 requirements covered by tests
- [ ] Test values are hand-calculated and verifiable

## Failure Recovery
- Return to Phase 10 and add missing test cases
- If SDX2 expected values seem wrong, hand-calculate from algorithm step by step
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P10a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P10a
- timestamp
- verification result: PASS/FAIL
- test count
- gate decision: proceed to P11 or return to P10
