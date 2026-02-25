# Phase 03a: Parser Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P03a`

## Prerequisites
- Required: Phase 03 completed
- Expected files: `rust/src/sound/aiff.rs`, updated `rust/src/sound/mod.rs`

## Verification Checklist

### Structural
- [ ] `rust/src/sound/aiff.rs` exists
- [ ] `pub mod aiff;` present in `rust/src/sound/mod.rs`
- [ ] `cargo check --all-features` succeeds
- [ ] `cargo fmt --all --check` passes
- [ ] Constants: FORM_ID, FORM_TYPE_AIFF, FORM_TYPE_AIFC, COMMON_ID, SOUND_DATA_ID, SDX2_COMPRESSION defined
- [ ] Types: CompressionType, CommonChunk, SoundDataHeader, ChunkHeader defined
- [ ] AiffDecoder struct has all fields from spec

### Semantic
- [ ] `AiffDecoder::new()` compiles and initializes all fields
- [ ] `impl SoundDecoder for AiffDecoder` compiles
- [ ] Trivial accessors (name, frequency, format, length, is_null, needs_swap, get_frame) are implemented, not stubbed
- [ ] `get_error()` implements get-and-clear pattern
- [ ] `close()` clears data/positions/predictor
- [ ] `init()` accesses formats for need_swap
- [ ] Complex methods (open, open_from_bytes, decode, seek) use `todo!()` stubs

### No Fraud
- [ ] No `FIXME`/`HACK`/`placeholder` markers in implemented methods
- [ ] `todo!()` only in functions genuinely not yet implemented
- [ ] No fake return values masquerading as implementations

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# Format check
cargo fmt --all --check

# Verify module registration
grep "pub mod aiff;" src/sound/mod.rs

# Verify trait impl
grep "impl SoundDecoder for AiffDecoder" src/sound/aiff.rs

# Verify constants
grep "FORM_ID" src/sound/aiff.rs
grep "SDX2_COMPRESSION" src/sound/aiff.rs

# Check for forbidden patterns in non-stub code
grep -n "todo!()" src/sound/aiff.rs | head -20
```

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: return to Phase 03 and fix issues
