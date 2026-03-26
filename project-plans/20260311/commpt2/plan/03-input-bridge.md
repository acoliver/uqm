# Phase 03: Input Bridge

## Phase ID
`PLAN-20260326-COMMPT2.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Pseudocode A (lines A01–A45) and B (lines B01–B05) verified
- `c_GetPulsedMenuKey(int)` exists in rust_comm.c (line 749)
- All existing comm tests pass

## Requirements Implemented (Expanded)

### REQ-IP-001: check_select_input polls real input
**Requirement text**: `check_select_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_SELECT]` via the C bridge, not hardcoded false.

Behavior contract:
- GIVEN: The game is in an active encounter with `USE_RUST_COMM=on`
- WHEN: `check_select_input` is called in the `#[cfg(not(test))]` path
- THEN: It returns `c_GetPulsedMenuKey(9) != 0`, reflecting the actual key state

Why it matters:
- Without this, the player cannot confirm response selections — the dialogue is non-interactive

### REQ-IP-002: check_cancel_input polls real input
**Requirement text**: `check_cancel_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_CANCEL]` via the C bridge.

Behavior contract:
- GIVEN: The game is in an active encounter
- WHEN: `check_cancel_input` is called in production mode
- THEN: It returns `c_GetPulsedMenuKey(10) != 0`

Why it matters:
- Without this, the player cannot access the conversation summary or skip dialogue

### REQ-IP-003: check_up_input polls real input
**Requirement text**: `check_up_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_UP]` via the C bridge.

Behavior contract:
- GIVEN: The game is in an active encounter
- WHEN: `check_up_input` is called in production mode
- THEN: It returns `c_GetPulsedMenuKey(5) != 0`

Why it matters:
- Without this, the player cannot navigate up through response options

### REQ-IP-004: check_down_input polls real input
**Requirement text**: `check_down_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_DOWN]` via the C bridge.

Behavior contract:
- GIVEN: The game is in an active encounter
- WHEN: `check_down_input` is called in production mode
- THEN: It returns `c_GetPulsedMenuKey(6) != 0`

Why it matters:
- Without this, the player cannot navigate down through response options

### REQ-IP-005: check_left_input polls real input
**Requirement text**: `check_left_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_LEFT]` via the C bridge.

Behavior contract:
- GIVEN: The game is in an active encounter
- WHEN: `check_left_input` is called in production mode
- THEN: It returns `c_GetPulsedMenuKey(7) != 0`

Why it matters:
- Without this, the player cannot replay/reverse speech playback

### REQ-IP-006: check_right_input polls real input
**Requirement text**: `check_right_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_RIGHT]` via the C bridge.

Behavior contract:
- GIVEN: The game is in an active encounter
- WHEN: `check_right_input` is called in production mode
- THEN: It returns `c_GetPulsedMenuKey(8) != 0`

Why it matters:
- Without this, the player cannot fast-forward speech playback

### REQ-IP-007: Input bridge uses c_GetPulsedMenuKey
**Requirement text**: Input bridge functions SHALL be called from the Rust encounter loop via the existing `c_GetPulsedMenuKey(key_index)` C wrapper already present in rust_comm.c.

Behavior contract:
- GIVEN: `c_GetPulsedMenuKey` is declared in `rust_comm.c` (line 749)
- WHEN: Any `check_*_input` function is called in production mode
- THEN: It delegates to `c_GetPulsedMenuKey` with the correct key index

Why it matters:
- Maintains the single source of truth for input state (C's PulsedInputState)

### REQ-IP-008: Test mode uses simulated input
**Requirement text**: Test mode (`#[cfg(test)]`) SHALL continue to use test-driven input simulation, not the C bridge.

Behavior contract:
- GIVEN: Code is compiled with `#[cfg(test)]`
- WHEN: Any `check_*_input` function is called
- THEN: It reads from CommState test fields, not from C bridge

Why it matters:
- Tests must remain deterministic and not require C runtime

### REQ-AT-001: has_transition_anim checks LOCDATA
**Requirement text**: `has_transition_anim` SHALL check the actual LOCDATA transition descriptor (NumFrames > 0), not return hardcoded false.

Behavior contract:
- GIVEN: An encounter is active with loaded LOCDATA
- WHEN: `has_transition_anim` is called in production mode
- THEN: It returns whether the LOCDATA has a transition animation (via C bridge)

Why it matters:
- Without this, intro transitions never play, breaking the visual presentation

## Implementation Tasks

### Files to modify

#### `rust/src/comm/talk_segue.rs`
- **Add `c_GetPulsedMenuKey` to the `c_bridge` extern block** (line ~46)
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-007`

- **Add `c_HasTransitionAnim` to the `c_bridge` extern block**
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-AT-001`

- **Add key constant definitions** near the c_bridge module (values from `controls.h` second enum, starting after KEY_PAUSE=0..KEY_FULLSCREEN=4)
  - `KEY_MENU_UP: c_int = 5`
  - `KEY_MENU_DOWN: c_int = 6`
  - `KEY_MENU_LEFT: c_int = 7`
  - `KEY_MENU_RIGHT: c_int = 8`
  - `KEY_MENU_SELECT: c_int = 9`
  - `KEY_MENU_CANCEL: c_int = 10`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`

- **Replace `check_select_input`** (lines 504–515)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_SELECT) != 0 }`
  - Keep `#[cfg(test)]` body unchanged (or update to use test input fields)
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-001`

- **Replace `check_cancel_input`** (lines 487–502)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_CANCEL) != 0 }`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-002`

- **Replace `check_up_input`** (lines 543–554)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_UP) != 0 }`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-003`

- **Replace `check_down_input`** (lines 556–567)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_DOWN) != 0 }`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-004`

- **Replace `check_left_input`** (lines 517–528)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_LEFT) != 0 }`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-005`

- **Replace `check_right_input`** (lines 530–541)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_RIGHT) != 0 }`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-IP-006`

- **Replace `has_transition_anim`** (lines 828–839)
  - Change `#[cfg(not(test))]` body from `false` to `unsafe { c_bridge::c_HasTransitionAnim() != 0 }`
  - Keep `#[cfg(test)]` body: `state.animations().has_transition_anim()`
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-AT-001`

#### `sc2/src/uqm/rust_comm.c` (if `c_HasTransitionAnim` doesn't exist)
- **Add `c_HasTransitionAnim` bridge wrapper**
  - Calls C's `haveTransitionAnim()` or checks CommData transit descriptor directly
  - marker: `@plan PLAN-20260326-COMMPT2.P03`
  - marker: `@requirement REQ-AT-001`

#### `sc2/src/uqm/rust_comm.h` (if needed)
- **Add declaration for `c_HasTransitionAnim`**

### Pseudocode traceability
- Uses pseudocode lines: A01–A45 (input bridge wiring)
- Uses pseudocode lines: B01–B05 (transition animation check)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify input bridge is wired (not hardcoded false)
grep -n "c_GetPulsedMenuKey" rust/src/comm/talk_segue.rs

# Verify has_transition_anim is wired
grep -n "c_HasTransitionAnim" rust/src/comm/talk_segue.rs

# Verify no remaining hardcoded false in non-test input functions
grep -A3 "cfg(not(test))" rust/src/comm/talk_segue.rs | grep "false"
# Should only match won_last_battle test path, not input functions

# C build verification
# (project-specific USE_RUST_COMM=on build)
```

## Structural Verification Checklist
- [ ] `c_GetPulsedMenuKey` declared in `c_bridge` extern block
- [ ] `c_HasTransitionAnim` declared in `c_bridge` extern block (or added to rust_comm.c)
- [ ] Key constants defined (UP=5, DOWN=6, LEFT=7, RIGHT=8, SELECT=9, CANCEL=10)
- [ ] All 6 `check_*_input` functions updated in `#[cfg(not(test))]` path
- [ ] `has_transition_anim` updated in `#[cfg(not(test))]` path
- [ ] `#[cfg(test)]` paths unchanged or updated to use proper test fields
- [ ] `@plan` and `@requirement` markers present
- [ ] All existing tests compile and pass

## Semantic Verification Checklist (Mandatory)
- [ ] Each `check_*_input` calls `c_GetPulsedMenuKey` with the correct key index
- [ ] Key indices match C `controls.h` constants exactly
- [ ] `has_transition_anim` delegates to C bridge for actual LOCDATA check
- [ ] No production code path returns hardcoded `false` for input
- [ ] Test mode continues to work without C runtime
- [ ] Feature behavior is reachable: input functions are called from `player_response_input` and `do_talk_segue`
- [ ] No placeholder/deferred patterns remain in modified functions

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in the modified functions:
grep -n "false.*production\|false.*hardcoded\|P11: Stub" rust/src/comm/talk_segue.rs
# Should return zero matches in the check_*_input and has_transition_anim functions
```

## Success Criteria
- [ ] All 6 input functions call `c_GetPulsedMenuKey` with correct key indices
- [ ] `has_transition_anim` calls `c_HasTransitionAnim` (or equivalent)
- [ ] All existing 267+ comm tests pass
- [ ] Both build modes compile
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] No hardcoded `false` remains in production input functions

## Failure Recovery
- Rollback: `git checkout -- rust/src/comm/talk_segue.rs`
- If `c_HasTransitionAnim` doesn't exist in C: add bridge wrapper to rust_comm.c first
- If key constants don't match: verify against `sc2/src/uqm/controls.h` before proceeding
- Blocking: must verify key index values match C before any implementation

## Phase Completion Marker
Create: `project-plans/20260311/commpt2/.completed/P03.md`

Contents:
- Phase ID: `PLAN-20260326-COMMPT2.P03`
- Timestamp
- Files changed: `rust/src/comm/talk_segue.rs`, optionally `rust_comm.c`, `rust_comm.h`
- Tests added/updated: (list)
- Verification outputs: cargo fmt/clippy/test results
- Semantic verification summary
