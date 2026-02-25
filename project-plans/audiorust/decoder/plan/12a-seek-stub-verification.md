# Phase 12a: Seek Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P12a`

## Prerequisites
- Required: Phase 12 completed
- Expected: `seek()` stub confirmed

## Verification Checklist

### Structural
- [ ] `fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32>` exists in trait impl
- [ ] Contains `todo!()`
- [ ] `cargo check --all-features` passes
- [ ] All existing tests pass

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo check --all-features
cargo test --lib --all-features -- aiff
grep "fn seek" src/sound/aiff.rs
```

## Gate Decision
- [ ] PASS: proceed to Phase 13
- [ ] FAIL: return to Phase 12
