# Phase 06a: PCM Decode Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P06a`

## Prerequisites
- Required: Phase 06 completed
- Expected: `decode_pcm()` and `decode_sdx2()` stubs exist

## Verification Checklist

### Structural
- [ ] `decode()` dispatches via match on `self.comp_type`
- [ ] `fn decode_pcm(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` exists
- [ ] `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` exists
- [ ] Both decode methods contain `todo!()`
- [ ] `cargo check --all-features` succeeds

### Semantic
- [ ] Parser tests still pass
- [ ] No decode logic implemented yet (only dispatch + stubs)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo check --all-features
cargo test --lib --all-features -- aiff::tests::test_parse
grep "fn decode_pcm" src/sound/aiff.rs
grep "fn decode_sdx2" src/sound/aiff.rs
```

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: return to Phase 06
