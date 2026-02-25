# Phase 13a: Seek TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P13a`

## Prerequisites
- Required: Phase 13 completed
- Expected: Seek tests in `aiff.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile-only check
cargo test --lib --all-features -- test_seek --no-run

# Count seek test functions
grep -c "test_seek" src/sound/aiff.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify test variety
for test in beginning middle past_end clamped data_pos predictor then_decode; do
  grep -q "test_seek.*$test\|test_seek_$test" src/sound/aiff.rs && echo "PASS: $test" || echo "FAIL: $test"
done
```

## Structural Verification Checklist
- [ ] At least 10 test functions with `test_seek` in name
- [ ] Tests compile: `cargo test --lib --all-features -- test_seek --no-run`
- [ ] Tests cover: clamping, position update, predictor reset, decode-after-seek
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Position clamping tested with value > max_pcm (verifies REQ-SK-1)
- [ ] data_pos = pcm_pos * file_block verified after seek (REQ-SK-2)
- [ ] Predictor reset tested for SDX2 mode — prev_val all zeros after seek (REQ-SK-3)
- [ ] Return value tested — must be the clamped value (REQ-SK-4)
- [ ] Decode after seek tested for PCM mode (correct data from new position)
- [ ] Decode after seek tested for SDX2 mode (fresh predictor state)

### Subjective Checks
- [ ] Does seeking to position 0 reset the SDX2 predictor state, so that re-decoding from the beginning produces the same output as the first decode?
- [ ] Does seeking to max_pcm cause the next decode() to return Err(EndOfFile)?
- [ ] Does the data_pos calculation correctly account for file_block (which differs between PCM and SDX2)?
- [ ] After seek + decode in SDX2 mode, are the predictor values starting from zero (not carrying over from before the seek)?
- [ ] Does seeking to the same position as cur_pcm still reset the predictor (even though position doesn't change)?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase — seek stub panics)
- [ ] All REQ-SK-1..4 requirements covered by tests
- [ ] Decode-after-seek tests present for both PCM and SDX2

## Failure Recovery
- Return to Phase 13 and add missing test cases
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P13a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P13a
- timestamp
- verification result: PASS/FAIL
- test count
- gate decision: proceed to P14 or return to P13
