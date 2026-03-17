# Phase 07a: Animation Engine Verification

## Phase ID
`PLAN-20260314-COMM.P07a`

## Prerequisites
- Required: Phase 07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `CommAnimDesc` has all 8 fields matching C `ANIMATION_DESC`
- [ ] `AnimSequence` has alarm, frame, direction, active, change_pending
- [ ] `CommAnimState` has sequences array, active_mask, talk/transit indices
- [ ] All AnimFlags constants defined and correct
- [ ] CommState.animations type changed to CommAnimState
- [ ] FFI exports for all animation control functions present

## Semantic Verification Checklist

### Frame Advancement
- [ ] `test_circular_anim_wraps` — frame 0→1→2→0 cycling
- [ ] `test_random_anim_different` — random frame != current frame
- [ ] `test_yoyo_anim_bounces` — forward then backward
- [ ] `test_colorxform_advances_colormap` — colormap index instead of sprite frame

### Conflict Resolution
- [ ] `test_blockmask_prevents_conflict` — two animations with overlapping BlockMask don't both run
- [ ] `test_blockmask_allows_nonconflict` — non-overlapping BlockMask allows concurrent

### Talking Integration
- [ ] `test_wait_talking_settles` — ambient settles to neutral when talking
- [ ] `test_wait_talking_resumes` — ambient resumes when talking stops
- [ ] `test_talk_anim_activates` — talk sequence starts on talking state
- [ ] `test_talk_anim_deactivates` — talk sequence stops when talking ends
- [ ] `test_transit_anim_on_transition` — transit plays during state changes

### One-Shot
- [ ] `test_one_shot_completes` — animation disables after last frame
- [ ] `test_one_shot_no_restart` — doesn't loop after completion

### Timing
- [ ] `test_frame_rate_in_range` — random delay between base and base+random
- [ ] `test_restart_rate_in_range` — random restart between base and base+random

### Lifecycle
- [ ] `test_init_from_commdata` — populates from CommData descriptors
- [ ] `test_clear_resets_all` — all sequences stopped, mask cleared
- [ ] `test_max_sequences_22` — 20 ambient + talk + transit

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/animation.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P07a.md`
