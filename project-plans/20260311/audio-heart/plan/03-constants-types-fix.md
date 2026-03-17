# Phase 03: Constants & Types Fix

## Phase ID
`PLAN-20260314-AUDIO-HEART.P03`

## Prerequisites
- Required: Phase P02a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `plan/02-pseudocode.md`, `plan/02a-pseudocode-verification.md`

## Requirements Implemented (Expanded)

### NORMAL_VOLUME Consistency (Spec §6)
**Requirement text**: There must be exactly one definition of `NORMAL_VOLUME` (160) used throughout the subsystem. Any conflicting local redefinitions must be eliminated.

Behavior contract:
- GIVEN: `types.rs` defines `NORMAL_VOLUME = 160` (canonical)
- WHEN: `control.rs` also defines `NORMAL_VOLUME = MAX_VOLUME` (255)
- THEN: Remove the conflicting definition, update all references to use `types::NORMAL_VOLUME`

Why it matters:
- Music starts at wrong default volume if `VolumeState::new()` uses 255 instead of 160
- Inconsistency between modules creates confusion and potential behavioral divergence

## Implementation Tasks

### Files to modify

#### `rust/src/sound/control.rs`
- **Remove** line 29: `pub const NORMAL_VOLUME: i32 = MAX_VOLUME;`
- **Modify** `VolumeState::new()` (line 48-55): change `music_volume: NORMAL_VOLUME` to `music_volume: types::NORMAL_VOLUME` (or just `NORMAL_VOLUME` since types::* is already imported)
- **Fix** test `test_normal_volume_is_max` (line 276-279): rename to `test_normal_volume_is_canonical`, assert `types::NORMAL_VOLUME == 160`
- **Fix** test `test_volume_state_new` (line 268-274): verify `music_volume` is initialized to 160, not 255
- marker: `@plan PLAN-20260314-AUDIO-HEART.P03`

### Pseudocode traceability
- Uses pseudocode PC-06 lines 01-05

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Specific verification
grep -n 'NORMAL_VOLUME' rust/src/sound/control.rs rust/src/sound/types.rs
# Expected: only types.rs defines it; control.rs imports it
```

## Structural Verification Checklist
- [ ] `control.rs` no longer defines `NORMAL_VOLUME`
- [ ] `types.rs` still defines `NORMAL_VOLUME = 160`
- [ ] `VolumeState::new()` initializes `music_volume` to 160
- [ ] No other file redefines `NORMAL_VOLUME`

## Semantic Verification Checklist
- [ ] `test_volume_state_new` passes with `music_volume == 160`
- [ ] No behavioral change to running code (the types.rs constant was always 160)
- [ ] Volume initialization is consistent across all paths

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/control.rs
```

## Success Criteria
- [ ] Single canonical `NORMAL_VOLUME = 160`
- [ ] All tests pass
- [ ] No conflicting definitions anywhere in `rust/src/sound/`

## Failure Recovery
- rollback: `git restore rust/src/sound/control.rs`
- blocking issues: None expected

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P03.md`
