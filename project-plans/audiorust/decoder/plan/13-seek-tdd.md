# Phase 13: Seek TDD

## Phase ID
`PLAN-20260225-AIFF-DECODER.P13`

## Prerequisites
- Required: Phase 12 completed (seek stub confirmed)
- Expected files: `rust/src/sound/aiff.rs` with `seek()` stub

## Requirements Implemented (Expanded)

### REQ-SK-1: Seek Position Clamping
**Requirement text**: Clamp requested position to `max_pcm`.

Behavior contract:
- GIVEN: A decoder with max_pcm=100
- WHEN: `seek(200)` is called
- THEN: Returns `Ok(100)` (clamped to max_pcm)

### REQ-SK-2: Seek Position Update
**Requirement text**: Set `cur_pcm` and `data_pos = pcm_pos * file_block`.

Behavior contract:
- GIVEN: A decoder with file_block=4, seek to position 50
- WHEN: `seek(50)` is called
- THEN: `cur_pcm=50`, `data_pos=200`

### REQ-SK-3: SDX2 Predictor Reset on Seek
**Requirement text**: Zero all `prev_val` entries on seek.

Behavior contract:
- GIVEN: A decoder with SDX2 data, after decoding some frames (prev_val has non-zero values)
- WHEN: `seek(0)` is called
- THEN: All `prev_val` entries are 0, subsequent decode starts fresh

### REQ-SK-4: Seek Return Value
**Requirement text**: Return `Ok(clamped pcm_pos)`.

Behavior contract:
- GIVEN: seek(50) with max_pcm=100
- WHEN: seek completes
- THEN: Returns `Ok(50)`

Why it matters:
- Seeking is critical for audio scrubbing and looping
- Predictor reset on seek ensures SDX2 recovery from any position

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs` — Add seek tests
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P13`
  - marker: `@requirement REQ-SK-1, REQ-SK-2, REQ-SK-3, REQ-SK-4`

### Test Cases to Write

1. `test_seek_to_beginning` — Seek to 0, verify cur_pcm=0, data_pos=0
2. `test_seek_to_middle` — Seek to max_pcm/2, verify position
3. `test_seek_past_end_clamped` — Seek beyond max_pcm, verify clamped
4. `test_seek_returns_clamped_position` — Return value = clamped position
5. `test_seek_data_pos_sync` — data_pos == pcm_pos * file_block after seek
6. `test_seek_resets_sdx2_predictor` — After SDX2 decode, seek resets prev_val to 0
7. `test_seek_then_decode_pcm` — Seek to position, then decode → correct data
8. `test_seek_then_decode_sdx2` — Seek to 0, then decode SDX2 → fresh predictor state
9. `test_seek_to_max_pcm` — Seek to exact end, next decode returns EndOfFile
10. `test_seek_zero_is_noop` — Seek to 0 when already at 0

### Pseudocode traceability
- Tests cover pseudocode lines: 300–312 (seek)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Tests compile but fail (RED)
cargo test --lib --all-features -- aiff::tests::test_seek --no-run

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] At least 10 seek test functions defined
- [ ] Tests compile (`--no-run`)
- [ ] Tests cover: clamping, position update, predictor reset, decode-after-seek

## Semantic Verification Checklist (Mandatory)
- [ ] Position clamping tested with value > max_pcm
- [ ] data_pos sync tested (pcm_pos * file_block)
- [ ] Predictor reset tested for SDX2 mode
- [ ] Decode after seek tested for both PCM and SDX2
- [ ] Return value tested (must be clamped value)

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: seek only
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase)
- [ ] All REQ-SK-* requirements covered

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: None expected

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P13.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P13
- timestamp
- files changed: `rust/src/sound/aiff.rs` (tests added)
- tests added: ~10 seek tests
- verification outputs
- semantic verification summary
