# Phase 11: SDX2 Decode Implementation

## Phase ID
`PLAN-20260225-AIFF-DECODER.P11`

## Prerequisites
- Required: Phase 10 completed (SDX2 decode tests exist)
- Expected files: `rust/src/sound/aiff.rs` with failing SDX2 tests

## Requirements Implemented (Expanded)

### REQ-DS-1: SDX2 Frame Count
**Requirement text**: `min(buf.len() / block_align, max_pcm - cur_pcm)`.

### REQ-DS-2: SDX2 Data Read
**Requirement text**: Read `dec_pcm * file_block` compressed bytes from `self.data[self.data_pos..]`.

### REQ-DS-3: SDX2 Position Update
**Requirement text**: Advance `cur_pcm += dec_pcm` and `data_pos += dec_pcm * file_block`.

### REQ-DS-4: SDX2 Decode Algorithm
**Requirement text**: For each compressed byte per channel:
1. Cast to i8 then i32
2. `v = (sample * sample.abs()) << 1`
3. If odd: `v += prev_val[ch]`
4. Clamp to [-32768, 32767]
5. Store `prev_val[ch] = v`
6. Write v as i16 to output (with potential byte swap)

### REQ-DS-5: SDX2 Channel Iteration
**Requirement text**: Iterate channels 0..channels per frame, reading one byte per channel.

### REQ-DS-6: SDX2 Return Value
**Requirement text**: Return `Ok(dec_pcm * block_align)`.

### REQ-DS-8: SDX2 EOF
**Requirement text**: Return `Err(EndOfFile)` when `cur_pcm >= max_pcm`.

Why it matters:
- GREEN phase — making all SDX2 decode tests pass
- Implements the complex ADPCM decode path with predictor state

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P11`
  - marker: `@requirement REQ-DS-1, REQ-DS-2, REQ-DS-3, REQ-DS-4, REQ-DS-5, REQ-DS-6, REQ-DS-8`
  - Implement: `decode_sdx2()` — remove `todo!()`, implement per pseudocode lines 250–295
  - Steps:
    1. Check EOF (cur_pcm >= max_pcm → EndOfFile)
    2. Calculate dec_pcm
    3. Slice compressed data from self.data
    4. For each frame, for each channel:
       a. Read compressed byte as i8
       b. Apply square-with-sign: `v = (sample * sample.abs()) << 1`
       c. If odd byte: add prev_val[ch] (delta mode)
       d. Clamp to i16 range
       e. Store predictor
       f. Write i16 to output buffer (with endian handling)
    5. Update cur_pcm and data_pos
    6. Return byte count

### Pseudocode traceability
- `decode_sdx2`: pseudocode lines 250–295

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# All tests pass (GREEN)
cargo test --lib --all-features -- aiff

# Quality gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `decode_sdx2()` no longer contains `todo!()`
- [ ] `seek()` still contains `todo!()` (not yet implemented)
- [ ] All parser + PCM + SDX2 tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] SDX2 even byte produces correct v = (s * |s|) << 1
- [ ] SDX2 odd byte adds predictor value
- [ ] Negative samples produce negative outputs (sign preservation via square-with-sign)
- [ ] Predictor accumulates across frames
- [ ] Clamping works at both bounds
- [ ] Stereo channel predictors are independent
- [ ] EOF correctly returned
- [ ] Endianness handled (need_swap for SDX2 uses cfg! target_endian)

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only
grep -RIn "FIXME\|HACK\|placeholder" src/sound/aiff.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All parser + PCM + SDX2 tests pass
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] No `todo!()` in `decode_sdx2()`
- [ ] Only `seek()` still stubbed

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: If SDX2 output doesn't match expected values, hand-verify the algorithm with test data

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P11.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P11
- timestamp
- files changed: `rust/src/sound/aiff.rs`
- tests added/updated: None (GREEN phase)
- verification outputs
- semantic verification summary
