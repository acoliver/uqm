# Phase 14a: Seek Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P14a`

## Prerequisites
- Required: Phase 14 completed
- Expected: `seek()` fully implemented, zero `todo!()` in `aiff.rs`

## Verification Checklist

### Structural
- [ ] **ZERO `todo!()` in `aiff.rs`**
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes

### Semantic
- [ ] Seek clamping works (value > max_pcm → clamped)
- [ ] Position sync: data_pos == cur_pcm * file_block after seek
- [ ] Predictor reset: all prev_val entries 0 after seek
- [ ] Decode-after-seek: PCM produces correct data from new position
- [ ] Decode-after-seek: SDX2 produces correct data with reset predictor

### Completeness
- [ ] `aiff.rs` implements all 15 SoundDecoder trait methods
- [ ] All helper functions implemented (byte readers, f80, chunk parsers)
- [ ] No deferred implementation markers anywhere in `aiff.rs`

### Quality
- [ ] No `FIXME`/`HACK`/`placeholder` markers
- [ ] All error handling uses proper DecodeError variants
- [ ] close() called before error returns in open_from_bytes()

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Full test suite
cargo test --lib --all-features -- aiff

# Zero todo check
grep -c "todo!()" src/sound/aiff.rs
# Expected: 0

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Gate Decision
- [ ] PASS: proceed to Phase 15 (FFI stub) — **MILESTONE: aiff.rs complete**
- [ ] FAIL: return to Phase 14
