# Phase 09a: SDX2 Decode Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P09a`

## Prerequisites
- Required: Phase 09 completed
- Expected: `decode_sdx2()` stub confirmed

## Verification Checklist

### Structural
- [ ] `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` exists
- [ ] Contains `todo!()`
- [ ] `cargo check --all-features` passes
- [ ] All existing tests pass

### Semantic
- [ ] SDX2 path reachable via `decode()` dispatch for `CompressionType::Sdx2`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo check --all-features
cargo test --lib --all-features -- aiff
grep "fn decode_sdx2" src/sound/aiff.rs
```

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: return to Phase 09
