# Phase 05a: Parser Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P05a`

## Prerequisites
- Required: Phase 05 completed
- Expected files: `rust/src/sound/aiff.rs` with implemented parsing

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

## Structural Verification Checklist
- [ ] No `todo!()` in: `read_be_u16`, `read_be_u32`, `read_be_i16`, `read_be_f80`, `read_chunk_header`, `read_common_chunk`, `read_sound_data_header`, `open_from_bytes`, `open`
- [ ] `todo!()` only in: `decode()`, `seek()` (deferred to later phases)
- [ ] `use std::io::{Cursor, Read, Seek, SeekFrom}` present
- [ ] All tests pass
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Valid AIFF mono16 parses: frequency=44100, format=Mono16, max_pcm matches sample_frames
- [ ] Valid AIFF stereo8 parses: format=Stereo8
- [ ] Valid AIFC SDX2 parses: comp_type=Sdx2, bits_per_sample=16
- [ ] f80(44100) → 44100, f80(22050) → 22050, f80(8000) → 8000
- [ ] f80 denormalized (exponent==0) → returns 0
- [ ] f80 infinity/NaN (exponent==0x7FFF) → returns Err(InvalidData)
- [ ] Error paths trigger with correct DecodeError variant
- [ ] `last_error` is -2 after parsing failures
- [ ] `close()` called before every error return in `open_from_bytes()`
- [ ] Data extraction: `self.data.len() == sample_frames * file_block`

### Subjective Checks
- [ ] Does the parser correctly reject a truncated COMM chunk (less than 18 bytes) with the right error message?
- [ ] Does the parser correctly skip unknown chunk types without corrupting the remaining byte position?
- [ ] Does odd-size chunk alignment padding work correctly — if a chunk has size 17, does it skip 18 bytes total (17 + 1 padding)?
- [ ] Does the parser handle the boundary between AIFF (form_type=AIFF, no compression) and AIFC (form_type=AIFC, must have compression) correctly?
- [ ] After a successful parse, is the extracted audio data slice exactly the right region of the input (data_start to data_start + sample_frames * file_block)?
- [ ] Does the 80-bit float parser produce correct sample rates for real AIFF files (not just synthetic test data)? **Recommended**: extract the 10-byte f80 sample rate field from an actual game `.aif` file (e.g., via `xxd -s OFFSET -l 10 path/to/file.aif`) and add it as an additional test vector to validate against real-world data, not just mathematically derived test values.

## Deferred Implementation Detection

```bash
# Implementation phase: NO todo!/placeholder allowed in parsing functions
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()\|unimplemented!()" src/sound/aiff.rs
# Remaining todo!() should ONLY be in decode() and seek() (not yet implemented)
```

## Success Criteria
- [ ] All parser tests pass
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes
- [ ] No `todo!()` in parsing functions
- [ ] `todo!()` only remains in `decode()` and `seek()`
- [ ] No `FIXME`/`HACK`/`placeholder` in parsing code

## Failure Recovery
- Return to Phase 05 and fix failing tests
- If f80 algorithm produces wrong values, debug against known sample rate encodings (44100 Hz = `[0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`)
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P05a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P05a
- timestamp
- verification result: PASS/FAIL
- test results summary
- gate decision: proceed to P06 or return to P05
