# Phase 05a: Parser Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P05a`

## Prerequisites
- Required: Phase 05 completed
- Expected files: `rust/src/sound/aiff.rs` with implemented parsing

## Verification Checklist

### Structural
- [ ] No `todo!()` in: `read_be_u16`, `read_be_u32`, `read_be_i16`, `read_be_f80`, `read_chunk_header`, `read_common_chunk`, `read_sound_data_header`, `open_from_bytes`, `open`
- [ ] `todo!()` only in: `decode()`, `seek()` (deferred to later phases)
- [ ] `use std::io::{Cursor, Read, Seek, SeekFrom}` present
- [ ] All tests pass

### Semantic
- [ ] Valid AIFF mono16 parses: frequency=44100, format=Mono16, max_pcm matches sample_frames
- [ ] Valid AIFF stereo8 parses: format=Stereo8
- [ ] Valid AIFC SDX2 parses: comp_type=Sdx2, bits_per_sample=16
- [ ] f80(44100) → 44100, f80(22050) → 22050, f80(8000) → 8000
- [ ] Error paths all trigger correctly with right DecodeError variant
- [ ] `last_error` is -2 after parsing failures
- [ ] `close()` called before every error return in `open_from_bytes()`
- [ ] Data extraction: `self.data.len() == sample_frames * file_block`

### Quality
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] No `FIXME`/`HACK`/`placeholder` markers in parsing code

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# All parser tests pass
cargo test --lib --all-features -- aiff

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred impl check
grep -n "todo!()" src/sound/aiff.rs
# Should show only decode() and seek()

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: return to Phase 05 and fix implementation
