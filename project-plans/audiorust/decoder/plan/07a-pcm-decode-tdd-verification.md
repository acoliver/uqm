# Phase 07a: PCM Decode TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P07a`

## Prerequisites
- Required: Phase 07 completed
- Expected: PCM decode tests in `aiff.rs`

## Verification Checklist

### Structural
- [ ] At least 10 test functions with `test_decode_pcm` in name
- [ ] Tests compile: `cargo test --lib --all-features -- test_decode_pcm --no-run`
- [ ] Tests use synthetic AIFF data (not external files)

### Semantic
- [ ] Mono8, Mono16, Stereo8, Stereo16 all have at least one test
- [ ] 8-bit signed→unsigned conversion explicitly tested
- [ ] EOF condition tested
- [ ] Partial decode tested (buffer smaller than total data)
- [ ] Sequential decode tested (multiple calls)
- [ ] Return value (byte count) explicitly checked

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features -- test_decode_pcm --no-run
grep -c "test_decode_pcm" src/sound/aiff.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: return to Phase 07
