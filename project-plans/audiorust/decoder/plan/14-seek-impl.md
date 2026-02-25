# Phase 14: Seek Implementation

## Phase ID
`PLAN-20260225-AIFF-DECODER.P14`

## Prerequisites
- Required: Phase 13 completed (seek tests exist)
- Expected files: `rust/src/sound/aiff.rs` with failing seek tests

## Requirements Implemented (Expanded)

### REQ-SK-1: Seek Position Clamping
**Requirement text**: `pcm_pos.min(self.max_pcm)`.

### REQ-SK-2: Seek Position Update
**Requirement text**: `self.cur_pcm = pcm_pos; self.data_pos = pcm_pos * file_block`.

### REQ-SK-3: SDX2 Predictor Reset
**Requirement text**: `self.prev_val = [0i32; MAX_CHANNELS]`.

### REQ-SK-4: Seek Return Value
**Requirement text**: `Ok(pcm_pos)`.

Why it matters:
- GREEN phase — making all seek tests pass
- Completes all pure Rust decoder functionality (parser + PCM + SDX2 + seek)
- After this phase, `aiff.rs` has NO `todo!()` remaining

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P14`
  - marker: `@requirement REQ-SK-1, REQ-SK-2, REQ-SK-3, REQ-SK-4`
  - Implement: `seek()` — remove `todo!()`, implement per pseudocode lines 300–312
  - Steps:
    1. Clamp: `let pcm_pos = pcm_pos.min(self.max_pcm);`
    2. Update: `self.cur_pcm = pcm_pos;`
    3. Update: `self.data_pos = pcm_pos as usize * self.file_block as usize;`
    4. Reset: `self.prev_val = [0i32; MAX_CHANNELS];`
    5. Return: `Ok(pcm_pos)`

### Pseudocode traceability
- `seek`: pseudocode lines 300–312

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# ALL tests pass (GREEN) — no more todo!() in aiff.rs
cargo test --lib --all-features -- aiff

# Quality gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `seek()` no longer contains `todo!()`
- [ ] **No `todo!()` remaining in `aiff.rs`** — all decoder methods implemented
- [ ] All parser + PCM + SDX2 + seek tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Seek past end clamped to max_pcm
- [ ] Position correctly updated (both cur_pcm and data_pos)
- [ ] Predictor reset verified (prev_val all zeros after seek)
- [ ] Decode after seek works correctly (PCM continues from correct position)
- [ ] Decode after seek works correctly (SDX2 starts with fresh predictor)
- [ ] Return value matches clamped position

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()\|unimplemented!()" src/sound/aiff.rs
# Should return NO results — all methods implemented
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All tests pass (parser + PCM + SDX2 + seek)
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] **ZERO `todo!()` remaining in `aiff.rs`**
- [ ] `aiff.rs` is feature-complete for the pure Rust decoder

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: None expected (seek is simple)

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P14.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P14
- timestamp
- files changed: `rust/src/sound/aiff.rs`
- tests added/updated: None (GREEN phase)
- verification outputs
- semantic verification summary
- **MILESTONE**: `aiff.rs` feature-complete — all pure Rust decoder methods implemented
