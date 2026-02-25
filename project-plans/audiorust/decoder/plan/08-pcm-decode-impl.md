# Phase 08: PCM Decode Implementation

## Phase ID
`PLAN-20260225-AIFF-DECODER.P08`

## Prerequisites
- Required: Phase 07 completed (PCM decode tests exist)
- Expected files: `rust/src/sound/aiff.rs` with failing PCM decode tests

## Requirements Implemented (Expanded)

### REQ-DP-1: PCM Frame Count
**Requirement text**: Calculate `min(buf.len() as u32 / block_align, max_pcm - cur_pcm)`.

### REQ-DP-2: PCM Data Read
**Requirement text**: Copy `dec_pcm * file_block` bytes from `self.data[self.data_pos..]` into output buffer.

### REQ-DP-3: PCM Position Update
**Requirement text**: Advance `self.cur_pcm += dec_pcm` and `self.data_pos += dec_pcm * file_block`.

### REQ-DP-4: PCM Return Value
**Requirement text**: Return `Ok(dec_pcm as usize * block_align as usize)`.

### REQ-DP-5: 8-bit Signed-to-Unsigned
**Requirement text**: When `bits_per_sample == 8`, apply `byte.wrapping_add(128)` to all output bytes.

### REQ-DP-6: PCM EOF
**Requirement text**: When `cur_pcm >= max_pcm`, return `Err(DecodeError::EndOfFile)`.

Why it matters:
- GREEN phase — making all PCM decode tests pass
- Implements the simpler of the two decode paths first

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P08`
  - marker: `@requirement REQ-DP-1, REQ-DP-2, REQ-DP-3, REQ-DP-4, REQ-DP-5, REQ-DP-6`
  - Implement: `decode_pcm()` — remove `todo!()`, implement per pseudocode lines 226–249
  - Steps:
    1. Check EOF (cur_pcm >= max_pcm → EndOfFile)
    2. Calculate dec_pcm = min(buf.len()/block_align, max_pcm - cur_pcm)
    3. Copy data from self.data[data_pos..] to buf
    4. Apply 8-bit conversion if bits_per_sample == 8
    5. Update cur_pcm and data_pos
    6. Return byte count

### Pseudocode traceability
- `decode_pcm`: pseudocode lines 226–249

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
- [ ] `decode_pcm()` no longer contains `todo!()`
- [ ] `decode_sdx2()` still contains `todo!()` (not yet implemented)
- [ ] All parser tests still pass
- [ ] All PCM decode tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] PCM data copied correctly for all bit depths and channel counts
- [ ] 8-bit signed→unsigned conversion verified (known input → known output)
- [ ] Position tracking correct across multiple decode calls
- [ ] EOF returns `Err(DecodeError::EndOfFile)`, not `Ok(0)`
- [ ] Partial buffer decode returns correct frame count
- [ ] No allocation during decode (just slice copying)

## Deferred Implementation Detection (Mandatory)

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
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: If 8-bit conversion doesn't match expected values, verify test data encoding

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P08.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P08
- timestamp
- files changed: `rust/src/sound/aiff.rs`
- tests added/updated: None (GREEN phase)
- verification outputs
- semantic verification summary
