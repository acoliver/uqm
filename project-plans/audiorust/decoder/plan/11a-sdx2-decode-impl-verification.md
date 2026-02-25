# Phase 11a: SDX2 Decode Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P11a`

## Prerequisites
- Required: Phase 11 completed
- Expected: `decode_sdx2()` fully implemented

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# All tests pass
cargo test --lib --all-features -- aiff

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred impl check
grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder" src/sound/aiff.rs || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] No `todo!()` in `decode_sdx2()`
- [ ] `todo!()` only in `seek()`
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes
- [ ] No `FIXME`/`HACK`/`placeholder` in decode code

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Even byte decode produces v = (s * |s|) << 1 (e.g., byte=16 → 512)
- [ ] Odd byte decode adds predictor value (delta mode verified with exact values)
- [ ] Negative byte produces negative output (e.g., byte=-16 → -512)
- [ ] Stereo: channel 0 and channel 1 predictors independent (modifying one doesn't affect other)
- [ ] Clamping: values exceeding 32767 saturated to 32767
- [ ] Clamping: values below -32768 saturated to -32768
- [ ] EOF: `Err(EndOfFile)` after all frames consumed
- [ ] Position update: cur_pcm and data_pos correct after decode

### Subjective Checks
- [ ] Does the SDX2 decoder produce correct output for known test vectors that can be verified against the C implementation?
- [ ] Does the byte swap (need_swap) work correctly for SDX2 output — writing i16 values in the expected byte order?
- [ ] Is the compressed data reading correct — one byte per channel per frame, with frame-major interleaving?
- [ ] After decoding multiple frames, does the predictor state correctly accumulate, producing different output than if predictor was reset each frame?
- [ ] Does the endianness handling (`formats.big_endian != formats.want_big_endian`) produce the correct need_swap value? (Uses runtime `formats.big_endian`, NOT compile-time `cfg!(target_endian = "big")`)

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only
grep -RIn "FIXME\|HACK\|placeholder" src/sound/aiff.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All parser + PCM + SDX2 tests pass
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] No `todo!()` in `decode_sdx2()`
- [ ] Only `seek()` still stubbed

## Failure Recovery
- Return to Phase 11 and fix failing tests
- If SDX2 output doesn't match expected values, compare with C implementation step by step
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P11a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P11a
- timestamp
- verification result: PASS/FAIL
- test results summary
- gate decision: proceed to P12 or return to P11
