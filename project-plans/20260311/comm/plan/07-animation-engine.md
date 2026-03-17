# Phase 07: Animation Engine

## Phase ID
`PLAN-20260314-COMM.P07`

## Prerequisites
- Required: Phase 06a completed
- Expected: CommData has AnimationDescData fields from P03

## Requirements Implemented (Expanded)

### AO-REQ-001: Animation initialization
**Requirement text**: When a dialogue session begins, the communication subsystem shall initialize communication animation state for the active encounter configuration.

### AO-REQ-002: Three animation categories
**Requirement text**: Support ambient (up to 20), talking (1), and transition (1) animation sequences.

### AO-REQ-005: Timing-based frame advancement
**Requirement text**: Active animations advance per configured timing, frame progression mode, and restart behavior.

### AO-REQ-006: Block mask mutual exclusion
**Requirement text**: When animation descriptors define mutual exclusion, conflicting animations shall not run concurrently.

### AO-REQ-007: WAIT_TALKING behavior
**Requirement text**: Ambient animations configured to pause during talking shall settle to neutral frame while talking is active.

### AO-REQ-009: Colormap transformation
**Requirement text**: Animations affecting color transformation use colormap index advance instead of sprite-frame substitution.

### AO-REQ-016: Randomized timing
**Requirement text**: Random frame/restart delays within configured ranges.

## Implementation Tasks

### Files to modify

- `rust/src/comm/animation.rs` — Major rewrite to match C commanim.c model
  - **Replace** generic `AnimDesc`/`Animation`/`AnimContext` with ANIMATION_DESC-based engine
  - marker: `@plan PLAN-20260314-COMM.P07`
  - marker: `@requirement AO-REQ-001 through AO-REQ-010, AO-REQ-016`

### New types

```rust
/// Animation flags matching C ANIMATION_DESC.AnimFlags
pub mod AnimFlags {
    pub const CIRCULAR_ANIM: u32 = 1 << 0;
    pub const RANDOM_ANIM: u32 = 1 << 1;
    pub const YOYO_ANIM: u32 = 1 << 2;
    pub const COLORXFORM_ANIM: u32 = 1 << 3;
    pub const WAIT_TALKING: u32 = 1 << 4;
    pub const ONE_SHOT_ANIM: u32 = 1 << 5;
    pub const ANIM_DISABLED: u32 = 1 << 6;
    pub const PAUSE_TALKING: u32 = 1 << 7;
}

/// Matches C ANIMATION_DESC from commanim.h
#[derive(Debug, Clone, Default)]
pub struct CommAnimDesc {
    pub start_index: u32,
    pub num_frames: u32,
    pub anim_flags: u32,
    pub base_frame_rate: u32,
    pub random_frame_rate: u32,
    pub base_restart_rate: u32,
    pub random_restart_rate: u32,
    pub block_mask: u32,
}

/// A running animation sequence (matches C SEQUENCE)
#[derive(Debug)]
pub struct AnimSequence {
    pub desc: CommAnimDesc,
    pub alarm: u32,           // ticks until next frame advance
    pub current_frame: u32,   // current frame within sequence
    pub direction: i32,       // 1=forward, -1=backward (yoyo)
    pub active: bool,
    pub change_pending: bool, // frame changed this tick
    pub frames_remaining: i32, // for one-shot tracking (-1 = infinite)
}

/// Communication animation state (replaces generic AnimContext)
pub struct CommAnimState {
    sequences: Vec<AnimSequence>,  // MAX_ANIMATIONS + 2
    active_mask: u32,
    talk_index: usize,
    transit_index: usize,
    first_ambient: usize,
    total_sequences: usize,
    last_time: u32,
    talking: bool,
    running_intro_anim: bool,
    running_talking_anim: bool,
}
```

### CommAnimState methods

- `init(comm_data: &CommData)` — populate sequences from CommData ambient array, talk desc, transit desc
- `process(delta_ticks: u32, talking: bool)` — per-frame animation tick
  - For each active sequence:
    - Decrement alarm by delta_ticks
    - If alarm expired: advance frame per anim type
    - Check BlockMask conflicts against `active_mask`
    - Check WAIT_TALKING: if talking, settle to neutral frame
    - Check ONE_SHOT: disable after last frame
    - Apply COLORXFORM: advance colormap index instead of sprite frame
    - Reset alarm: `base_frame_rate + random(0..random_frame_rate)`
    - Set `change_pending` flag
  - Handle restart delays for disabled-then-restarted sequences
- `start_talking_anim()` — activate talk sequence, start transit sequence
- `stop_talking_anim()` — deactivate talk, start transit back to silent
- `set_intro_anim(running: bool)`
- `want_talking_anim() -> bool` — check if talk anim is defined
- `have_talking_anim() -> bool` — check if talk anim is active
- `get_frame(index: usize) -> u32` — get current frame for rendering
- `clear()` — reset all state

### Random number generation

Use the existing C `TFB_Random()` through FFI for consistent behavior, or use a simple LCG matching the C implementation:

```rust
fn random_frame_rate(desc: &CommAnimDesc) -> u32 {
    desc.base_frame_rate + (tfb_random() % (desc.random_frame_rate + 1))
}

fn random_restart_rate(desc: &CommAnimDesc) -> u32 {
    desc.base_restart_rate + (tfb_random() % (desc.random_restart_rate + 1))
}
```

### Files to modify (continued)

- `rust/src/comm/state.rs`
  - Replace `animations: AnimContext` with `animations: CommAnimState`
  - Update `clear()`, `update()` methods
  - Add `init_animations()` method called from encounter setup
  - marker: `@plan PLAN-20260314-COMM.P07`

- `rust/src/comm/ffi.rs`
  - Update animation FFI exports to use new CommAnimState
  - Add `rust_ProcessCommAnimations(delta_ticks: c_uint)` export
  - Add `rust_InitCommAnimations()` export
  - Add `rust_WantTalkingAnim() -> c_int`, `rust_HaveTalkingAnim() -> c_int`
  - Add `rust_SetRunIntroAnim(run: c_int)`, `rust_SetRunTalkingAnim(run: c_int)`
  - Add `rust_RunningIntroAnim() -> c_int`, `rust_RunningTalkingAnim() -> c_int`
  - Add `rust_SetStopTalkingAnim()`
  - marker: `@plan PLAN-20260314-COMM.P07`

### Pseudocode traceability
- Uses pseudocode lines: 135-167 (animation engine)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `CommAnimDesc` matches C `ANIMATION_DESC` fields
- [ ] `AnimSequence` matches C `SEQUENCE` behavior
- [ ] `CommAnimState` replaces generic `AnimContext`
- [ ] All AnimFlags constants defined
- [ ] BlockMask conflict checking implemented
- [ ] Talk/transit sequence management implemented
- [ ] FFI exports for animation control present

## Semantic Verification Checklist (Mandatory)
- [ ] Test: CIRCULAR_ANIM wraps frames sequentially
- [ ] Test: RANDOM_ANIM selects random frame != current
- [ ] Test: YOYO_ANIM bounces forward/backward
- [ ] Test: COLORXFORM_ANIM advances colormap index
- [ ] Test: BlockMask prevents conflicting animations from running simultaneously
- [ ] Test: WAIT_TALKING settles ambient to neutral when talking
- [ ] Test: ONE_SHOT_ANIM disables after completing
- [ ] Test: frame rate randomization stays within configured range
- [ ] Test: restart rate randomization stays within configured range
- [ ] Test: talk anim activates/deactivates on talking state change
- [ ] Test: transit anim plays during state transitions
- [ ] Test: init populates sequences from CommData descriptors
- [ ] Test: clear resets all animation state
- [ ] Test: 20 ambient + talk + transit = 22 sequences maximum

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/animation.rs
```

## Success Criteria
- [ ] Animation engine matches C commanim.c behavior
- [ ] All AO-REQ tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git restore rust/src/comm/animation.rs rust/src/comm/state.rs`
- blocking: Random number source must be determined (C TFB_Random FFI or local LCG)

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P07.md`
