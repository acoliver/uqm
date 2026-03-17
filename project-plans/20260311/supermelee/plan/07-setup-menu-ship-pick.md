# Phase 07: Setup Menu & Fleet Ship Pick

## Phase ID
`PLAN-20260314-SUPERMELEE.P07`

## Prerequisites
- Required: Phase 06a completed and passed
- Team model, persistence, and config behavior available

## Purpose

Implement the SuperMelee-owned interactive setup flow:
- `Melee()` entry and teardown,
- fallback initialization through built-in teams when persisted setup is unavailable,
- main menu loop and transient subviews,
- fleet-edit ship picker confirm/cancel behavior,
- start gating for local playable fleets,
- battle handoff invocation and post-battle menu restoration.

## Implementation Tasks

### Files to create or complete

- `rust/src/supermelee/setup/melee.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P07`
  - Implement:
    - `Melee()` entry path
    - runtime state initialization and teardown
    - persisted-setup restore or built-in fallback initialization
    - setup/menu loop and subview dispatch
    - start-button flow and invalid-start rejection
    - setup-to-battle transition hooks
    - post-battle restoration to a valid menu state

- `rust/src/supermelee/setup/build_pick.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P07`
  - Implement:
    - fleet-edit ship picker navigation
    - confirm behavior that commits selected ship into the active slot
    - cancel behavior that leaves team state unchanged
    - picker state/result types used by the menu flow

### Tests to create

- `rust/src/supermelee/setup/melee_tests.rs`
  - `test_melee_entry_initializes_runtime_state`
  - `test_invalid_or_missing_config_uses_builtin_fallback`
  - `test_menu_cancelled_subview_returns_to_valid_menu_state`
  - `test_match_start_blocked_when_either_side_unplayable`
  - `test_match_start_allowed_when_both_sides_playable_and_local`
  - `test_battle_return_restores_valid_post_battle_menu_state`
  - `test_exit_path_persists_setup_state_and_releases_resources`

- `rust/src/supermelee/setup/build_pick_tests.rs`
  - `test_picker_navigation_changes_highlighted_ship`
  - `test_picker_confirm_applies_selection_to_active_slot`
  - `test_picker_cancel_leaves_team_state_unchanged`
  - `test_cancelled_team_load_or_save_subview_leaves_state_unchanged`

### Files to modify

- `rust/src/supermelee/setup/mod.rs`
  - ensure `melee` and `build_pick` modules are declared and tests are wired per local conventions

### Pseudocode traceability
- Uses pseudocode lines: 073–123

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features melee_tests build_pick_tests
```

## Structural Verification Checklist
- [ ] `melee.rs` and `build_pick.rs` exist under `setup/`
- [ ] Tests exist for setup/menu flow and picker behavior
- [ ] Start validation is implemented in setup/menu code rather than deferred to battle internals

## Semantic Verification Checklist (Mandatory)
- [ ] Entry initializes a usable setup/menu state
- [ ] Missing or invalid persisted setup falls back through the built-in team offering
- [ ] Cancelled transient subviews return to a valid menu state without committing edits
- [ ] Fleet-edit confirm and cancel semantics are verified separately
- [ ] Invalid setup cannot enter battle
- [ ] Valid local setup can hand off to battle and restore a valid post-battle menu state after return

## Success Criteria
- [ ] The plan now includes a real implementation phase for setup/menu orchestration and fleet ship-pick behavior
- [ ] Core setup/menu requirements are covered by phase-local tasks and tests rather than only later E2E phases

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/setup/melee.rs rust/src/supermelee/setup/build_pick.rs`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P07.md`
