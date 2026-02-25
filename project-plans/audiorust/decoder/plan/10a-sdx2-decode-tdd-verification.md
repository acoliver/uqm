# Phase 10a: SDX2 Decode TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P10a`

## Prerequisites
- Required: Phase 10 completed
- Expected: SDX2 decode tests in `aiff.rs`

## Verification Checklist

### Structural
- [ ] At least 12 test functions with `test_decode_sdx2` in name
- [ ] Tests compile: `cargo test --lib --all-features -- test_decode_sdx2 --no-run`
- [ ] `build_aifc_sdx2_file()` helper exists

### Semantic
- [ ] Even byte (no delta) tested with exact expected value
- [ ] Odd byte (delta mode) tested with exact expected value
- [ ] Negative compressed byte tested
- [ ] Stereo interleaving tested (per-channel predictor)
- [ ] Clamping tested (positive and negative overflow)
- [ ] EOF tested
- [ ] Position update tested

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features -- test_decode_sdx2 --no-run
grep -c "test_decode_sdx2" src/sound/aiff.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Gate Decision
- [ ] PASS: proceed to Phase 11
- [ ] FAIL: return to Phase 10
