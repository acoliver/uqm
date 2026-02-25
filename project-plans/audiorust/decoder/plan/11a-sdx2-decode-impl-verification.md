# Phase 11a: SDX2 Decode Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P11a`

## Prerequisites
- Required: Phase 11 completed
- Expected: `decode_sdx2()` fully implemented

## Verification Checklist

### Structural
- [ ] No `todo!()` in `decode_sdx2()`
- [ ] `todo!()` only in `seek()`
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`

### Semantic
- [ ] Even byte decode produces v = (s * |s|) << 1 (e.g., byte=16 → 512)
- [ ] Odd byte decode adds predictor (delta mode verified)
- [ ] Negative byte produces negative output
- [ ] Stereo: channel 0 and channel 1 predictors independent
- [ ] Clamping: values exceeding ±32767/32768 are saturated
- [ ] EOF: `Err(EndOfFile)` after all frames consumed

### Quality
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] No `FIXME`/`HACK`/`placeholder` in decode code

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features -- aiff
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
grep -n "todo!()" src/sound/aiff.rs
```

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: return to Phase 11
