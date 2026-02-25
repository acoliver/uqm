# Phase 08a: PCM Decode Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P08a`

## Prerequisites
- Required: Phase 08 completed
- Expected: `decode_pcm()` fully implemented

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# All tests pass
cargo test --lib --all-features -- aiff

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred impl check
grep -n "todo!()" src/sound/aiff.rs
# Should show: decode_sdx2, seek (not decode_pcm)

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder" src/sound/aiff.rs || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] No `todo!()` in `decode_pcm()`
- [ ] `todo!()` remains in `decode_sdx2()` and `seek()`
- [ ] All tests pass: `cargo test --lib --all-features -- aiff`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes
- [ ] No `FIXME`/`HACK`/`placeholder` in decode code

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Mono 16-bit decode: output bytes match input data exactly (no conversion for 16-bit)
- [ ] Stereo 16-bit decode: interleaved channels preserved correctly
- [ ] Mono 8-bit decode: signed→unsigned conversion (0x80→0x00, 0x00→0x80, 0x7F→0xFF)
- [ ] Partial buffer: correct number of frames decoded (floor division of buf.len() by block_align)
- [ ] EOF: returns `Err(DecodeError::EndOfFile)` after all frames consumed, not `Ok(0)`
- [ ] Position tracking: `cur_pcm` and `data_pos` consistent after each decode call

### Subjective Checks
- [ ] Does the implementation avoid allocations during decode (just slice copying and in-place byte modification)?
- [ ] Does the 8-bit conversion correctly apply `wrapping_add(128)` to every byte in the output, not the source data?
- [ ] If the buffer is smaller than one frame (e.g., 1 byte for stereo 16-bit with block_align=4), does decode correctly return Ok(0) without advancing position?
- [ ] Is the relationship between `file_block` (bytes read from data) and `block_align` (bytes written to output) maintained correctly for PCM where they should be equal?

## Deferred Implementation Detection

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: decode_sdx2, seek (not decode_pcm)
grep -RIn "FIXME\|HACK\|placeholder" src/sound/aiff.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All parser + PCM decode tests pass
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] No `todo!()` in `decode_pcm()`
- [ ] `decode_sdx2()` and `seek()` still stubbed

## Failure Recovery
- Return to Phase 08 and fix failing tests
- If 8-bit conversion doesn't match expected values, verify test data uses AIFF signed encoding
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P08a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P08a
- timestamp
- verification result: PASS/FAIL
- test results summary
- gate decision: proceed to P09 or return to P08
