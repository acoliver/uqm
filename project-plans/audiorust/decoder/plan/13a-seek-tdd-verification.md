# Phase 13a: Seek TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P13a`

## Prerequisites
- Required: Phase 13 completed
- Expected: Seek tests in `aiff.rs`

## Verification Checklist

### Structural
- [ ] At least 10 test functions with `test_seek` in name
- [ ] Tests compile: `cargo test --lib --all-features -- test_seek --no-run`

### Semantic
- [ ] Position clamping tested
- [ ] data_pos = pcm_pos * file_block verified
- [ ] Predictor reset tested for SDX2
- [ ] Decode-after-seek tested
- [ ] Return value verified

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features -- test_seek --no-run
grep -c "test_seek" src/sound/aiff.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Gate Decision
- [ ] PASS: proceed to Phase 14
- [ ] FAIL: return to Phase 13
