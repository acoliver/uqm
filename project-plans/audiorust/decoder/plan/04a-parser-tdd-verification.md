# Phase 04a: Parser TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P04a`

## Prerequisites
- Required: Phase 04 completed
- Expected files: `rust/src/sound/aiff.rs` with test module

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile-only check
cargo test --lib --all-features -- aiff --no-run

# Count test functions
grep -c "#\[test\]" src/sound/aiff.rs

# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify f80 edge case tests exist
grep -q "denormalized\|denorm" src/sound/aiff.rs && echo "PASS: f80 denorm test" || echo "FAIL: f80 denorm test"
grep -q "infinity\|nan\|NaN\|0x7FFF" src/sound/aiff.rs && echo "PASS: f80 inf/nan test" || echo "FAIL: f80 inf/nan test"
```

## Structural Verification Checklist
- [ ] `#[cfg(test)] mod tests` block exists in `aiff.rs`
- [ ] Tests compile: `cargo test --lib --all-features -- aiff --no-run` succeeds
- [ ] At least 23 test functions defined (including truncated file test)
- [ ] `build_aiff_file()` helper exists
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Valid AIFF tests cover: mono8, mono16, stereo8, stereo16
- [ ] Valid AIFC test covers SDX2 compression detection
- [ ] Error tests cover all validation paths: REQ-FP-2, REQ-FP-3, REQ-FP-9, REQ-SV-2, REQ-SV-3, REQ-SV-4, REQ-SV-5, REQ-SV-6, REQ-CH-2, REQ-CH-4, REQ-CH-5, REQ-CH-6
- [ ] f80 tests cover at least 6 known sample rates (8000, 11025, 22050, 44100, 48000, 96000)
- [ ] f80 denormalized test (exponent==0) verifies Ok(0)
- [ ] f80 infinity/NaN test (exponent==0x7FFF) verifies Err(InvalidData)
- [ ] Edge case tests: odd chunk padding, unknown chunk skip, duplicate COMM
- [ ] Tests check specific `DecodeError` variants, not just `is_err()`

### Subjective Checks
- [ ] Does the `build_aiff_file()` helper correctly construct byte-accurate AIFF headers that match the AIFF specification (big-endian, proper chunk sizes)?
- [ ] Do the validation tests actually trigger the specific error path they intend to test, or could they pass due to an earlier validation check?
- [ ] Are the f80 test vectors using the correct IEEE 754 80-bit encoding for each sample rate (e.g., 44100 = 0x400E AC44 0000 0000 0000)?
- [ ] Would any of these tests pass with a trivial fake implementation (e.g., always returning Ok(()))?
- [ ] Does the test for duplicate COMM chunks verify that the later chunk's values actually take effect?

## Deferred Implementation Detection

```bash
# TDD phase: implementation is still todo!(), which is expected
# Verify no fake implementations snuck in:
cd /Users/acoliver/projects/uqm/rust && grep -c "todo!()" src/sound/aiff.rs
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase — stubs panic)
- [ ] All requirement categories covered by at least one test
- [ ] f80 edge case tests present (denormalized, infinity/NaN)
- [ ] Test helper produces valid synthetic AIFF byte arrays

## Failure Recovery
- Return to Phase 04 and add missing test cases
- If synthetic byte array construction is wrong, fix the helper using known AIFF file hex dumps
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P04a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P04a
- timestamp
- verification result: PASS/FAIL
- test count
- gate decision: proceed to P05 or return to P04
