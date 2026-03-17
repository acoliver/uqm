# Phase 08: Battle-Facing Combatant Selection Contract

## Phase ID
`PLAN-20260314-SUPERMELEE.P08`

## Prerequisites
- Required: Phase 07a completed and passed
- Setup/menu flow can start local battle from valid fleets

## Purpose

Implement the scoped SuperMelee-owned combatant-selection contract for:
- initial combatant selection,
- next combatant selection after a loss,
- local prompt/auto-selection policy over fleet state,
- commit of selection state,
- preservation of a battle-ready handoff object rather than weakening the interface to bare ship IDs or slot indexes.

## Implementation Tasks

### Files to create or complete

- `rust/src/supermelee/setup/pick_melee.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P08`
  - Implement:
    - initial combatant selection request handling
    - next combatant selection request handling after prior combatant loss
    - local human prompt path where required
    - local auto-selection path where required
    - consumed-slot tracking and commit of selected slots
    - creation/receipt of the battle-ready combatant object via the audited ship/battle boundary
    - explicit preservation of the stronger battle-facing contract

### Tests to create

- `rust/src/supermelee/setup/pick_melee_tests.rs`
  - `test_initial_combatants_return_battle_ready_entries_for_both_sides`
  - `test_next_combatant_returns_battle_ready_entry_after_loss`
  - `test_consumed_slot_is_not_reselected`
  - `test_no_valid_slot_returns_none_without_corrupting_selection_state`
  - `test_local_prompt_selection_commits_selected_slot`
  - `test_auto_selection_path_commits_selected_slot`
  - `test_handoff_contract_is_not_weakened_to_bare_ship_id_or_slot`

### Files to modify

- `rust/src/supermelee/setup/mod.rs`
  - ensure `pick_melee` module is declared and tests are wired according to project conventions

### Pseudocode traceability
- Uses pseudocode lines: 124–147

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features pick_melee_tests
```

## Structural Verification Checklist
- [ ] `pick_melee.rs` exists under `setup/`
- [ ] Dedicated selection-contract tests exist
- [ ] The phase targets setup-owned selection behavior, not battle simulation internals

## Semantic Verification Checklist (Mandatory)
- [ ] Initial and next combatant selection are both implemented and verified
- [ ] Selection state is tracked so consumed/eliminated slots are not reselected
- [ ] The handoff contract remains battle-ready and is not reduced to bare identities or slot numbers
- [ ] No per-ship combat behavior is introduced into this phase
- [ ] Failure to produce a valid next combatant does not corrupt existing setup/selection state

## Success Criteria
- [ ] A concrete implementation phase now exists for the combatant-selection requirements referenced by the overview/tracker
- [ ] The battle-facing contract is preserved at the plan level with explicit local tests

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/setup/pick_melee.rs`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P08.md`
