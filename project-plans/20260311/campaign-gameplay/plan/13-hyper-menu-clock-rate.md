# Phase 13: Hyperspace Menu & Clock Rate Policy

## Phase ID
`PLAN-20260314-CAMPAIGN.P13`

## Prerequisites
- Required: Phase 12a completed
- Expected files: transitions, encounter, starbase, save/load modules
- Dependency: Clock subsystem for rate-setting API
- Dependency: State subsystem for game-state device usage
- Dependency: Resource subsystem for starmap
- Dependency: validated callable seam inventory and ownership notes from P01/P03.5 for hyperspace runtime callbacks, menu entry/exit, and sub-activity load handoff

## Requirements Implemented (Expanded)

### Hyperspace Menu (§7.6, from requirements.md)
**Requirement text**: The hyperspace menu shall support device usage, cargo display, roster display, save/load, starmap access, and return to navigation. When a menu action triggers a campaign transition, the menu shall exit and allow the campaign loop to process the resulting transition.

Behavior contract:
- GIVEN: Player opens hyperspace menu
- WHEN: Player selects "Devices" and uses a device that triggers encounter
- THEN: Menu exits, `start_encounter` set, campaign loop dispatches to encounter

### Clock Rate Policy (§8.4, from requirements.md)
**Requirement text**: Hyperspace uses hyperspace pacing rate; interplanetary uses interplanetary rate; starbase uses discrete day advancement.

### Day Advancement (§8.5)
**Requirement text**: Support requesting day advancement by specified number of days for story-driven time skips.

### Sub-Activity Save/Load (§9.5)
**Requirement text**: When load occurs during sub-activity, sub-activity exits cleanly and main loop processes loaded state.

Behavior contract:
- GIVEN: Player opens hyperspace menu, selects Load, loads a save
- WHEN: Load succeeds
- THEN: Menu exits, main campaign loop dispatches to loaded save's resume mode

## Implementation Tasks

### Files to create

- `rust/src/campaign/hyper_menu.rs` — Hyperspace menu campaign orchestration
  - marker: `@plan PLAN-20260314-CAMPAIGN.P13`
  - marker: `@requirement §7.6, §9.5`
  - Runtime callback shape, menu FFI dispatch, and load-exit behavior in this phase must follow the validated callable seam inventory from P01/P03.5 instead of assuming Rust fully owns the surrounding legacy loop shell
  - `HyperMenuChoice` enum: `Devices`, `Cargo`, `Roster`, `SaveGame`, `LoadGame`, `Starmap`, `Navigate`, `Cancel`
  - `do_hyperspace_menu(session: &mut CampaignSession) -> Result<HyperMenuResult, CampaignError>`
    - Loop menu choices
    - Devices: call device handler, if triggers encounter -> exit with `TransitionRequested`
    - Cargo: display cargo screen (via FFI)
    - Roster: display roster screen (via FFI)
    - SaveGame: call `save_game(session, slot)`, handle errors
    - LoadGame: call `load_game(session, slot)`, if success -> exit with `LoadCompleted`
    - Starmap: open starmap (via FFI)
    - Navigate/Cancel: exit with `Resume`
    - Check after each action: if `start_encounter` or other transition flag set -> exit immediately
  - `HyperMenuResult` enum: `Resume`, `TransitionRequested`, `LoadCompleted`
  - `cleanup_hyperspace_menu(session: &mut CampaignSession, result: &HyperMenuResult)`
    - If not still in battle state: clear hyperspace menu graphics context
  - `enter_hyperspace_runtime(session: &mut CampaignSession) -> Result<(), CampaignError>`
    - Set `IN_HYPERSPACE` flag
    - `set_activity_clock_rate()` -> hyperspace rate
    - Invoke hyperspace battle loop via seam-inventory-backed FFI callback path
    - On exit: check transition flags for encounter/interplanetary/quasispace
  - `on_battle_frame(session: &mut CampaignSession)` — per-frame hyperspace callback
    - Process events (NPC movement, encounter checks)
    - Handle NPC collision -> trigger encounter transition
  - Comprehensive tests:
    - Menu choice dispatch for all options
    - Device usage triggering encounter exits menu
    - Save from menu works correctly
    - Load from menu exits and resumes loaded state
    - Transition flags cause immediate menu exit
    - Clock rate set to hyperspace on entering hyperspace runtime
    - Menu cleanup runs correctly
    - FFI smoke/integration coverage proving sub-activity load exits through the validated top-level handoff contract

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod hyper_menu;`

- `rust/src/campaign/loop_dispatch.rs`
  - Wire hyperspace branch to call `enter_hyperspace_runtime()`
  - Wire encounter dispatch check after hyperspace return
  - Call `set_activity_clock_rate()` before interplanetary and hyperspace entries

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `hyper_menu.rs` created with all menu functions
- [ ] Module wired into `campaign/mod.rs`
- [ ] Loop dispatch updated for clock rate and hyperspace entry
- [ ] All callable seams used by this phase are traceable to validated P01/P03.5 inventory rows

## Semantic Verification Checklist (Mandatory)
- [ ] All menu choices dispatch correctly
- [ ] Device usage triggering encounter causes menu exit
- [ ] Save from menu persists correctly
- [ ] Load from menu resumes loaded state, old session replaced
- [ ] Transition flags cause immediate menu exit (§7.6)
- [ ] Clock rate set to hyperspace before hyperspace entry
- [ ] Clock rate set to interplanetary before interplanetary entry
- [ ] Sub-activity save/load (§9.5) works correctly with integration evidence for the validated top-level handoff, not only unit-local assertions

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/hyper_menu.rs
```

## Success Criteria
- [ ] Hyperspace menu fully functional
- [ ] Clock rate policy enforced
- [ ] Sub-activity save/load works

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/hyper_menu.rs rust/src/campaign/loop_dispatch.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P13.md`
