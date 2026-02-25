# Phase 08a: PCM Decode Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P08a`

## Prerequisites
- Required: Phase 08 completed
- Expected: `decode_pcm()` fully implemented

## Verification Checklist

### Structural
- [ ] No `todo!()` in `decode_pcm()`
- [ ] `todo!()` remains in `decode_sdx2()` and `seek()`
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`

### Semantic
- [ ] Mono 16-bit decode: output bytes match input data exactly
- [ ] Stereo 16-bit decode: interleaved channels preserved
- [ ] Mono 8-bit decode: signed→unsigned (e.g., 0x80→0x00, 0x00→0x80, 0x7F→0xFF)
- [ ] Partial buffer: correct number of frames decoded
- [ ] EOF: returns `Err(DecodeError::EndOfFile)` after all frames consumed
- [ ] Position tracking: `cur_pcm` and `data_pos` consistent

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
grep -RIn "FIXME\|HACK\|placeholder" src/sound/aiff.rs || echo "CLEAN"
```

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: return to Phase 08
