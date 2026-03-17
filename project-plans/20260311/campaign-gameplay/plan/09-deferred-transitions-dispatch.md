# Phase 09: Deferred Transitions & Activity Dispatch

## Phase ID
`PLAN-20260314-CAMPAIGN.P09`

## Prerequisites
- Required: Phase 08a completed
- Expected files: All campaign types, events, save/load modules
- Dependency: Clock subsystem for `InitGameClock`, `UninitGameClock`
- Dependency: Game init for `InitKernel`, `FreeKernel`
- Dependency: validated callable seam inventory and ownership notes from P01/P03.5 for start flow, kernel lifecycle, loop dispatch, and solar-system entry handoff

## Requirements Implemented (Expanded)

### Main Campaign Loop (from requirements.md — Campaign loop and activity dispatch)
**Requirement text**: When a deferred transition is pending, dispatch to the target activity. When encounter/starbase requested, dispatch to that flow. When interplanetary requested, dispatch to solar-system. Otherwise enter hyperspace.

Behavior contract:
- GIVEN: Campaign loop running with no pending transitions or requests
- WHEN: Loop iteration executes
- THEN: Enters hyperspace navigation runtime

- GIVEN: `start_encounter` flag set
- WHEN: Loop iteration executes
- THEN: Dispatches to encounter or starbase (based on starbase context)

### Start Flow (from requirements.md — New-game, load-game entry)
**Requirement text**: Present new-game/load-game decision and loop until valid campaign start.

### Deferred Transitions (§5.3)
**Requirement text**: Sub-activity can designate target activity; main loop processes on next cycle; no save-slot mutation.

Behavior contract:
- GIVEN: Starbase requests deferred transition to Interplanetary
- WHEN: Next loop iteration
- THEN: Interplanetary is entered with same initialization as direct selection; no save mutation occurs

### Terminal Conditions (§5.2)
**Requirement text**: Loop exits on victory, defeat, or restart/abort. Clock shutdown and teardown on exit.

### Restart Behavior (§4.4)
**Requirement text**: No stale campaign state carried into subsequent session.

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 001-031, 040-073, 080-090

### Files to create

- `rust/src/campaign/loop_dispatch.rs` — Main campaign loop and dispatch
  - marker: `@plan PLAN-20260314-CAMPAIGN.P09`
  - marker: `@requirement §4.1, §4.2, §4.4, §5.1, §5.2, §5.3`
  - Must consume the validated seam inventory and ownership notes from P01/P03.5 rather than assuming direct replacement of legacy top-level entrypoints whose callable shape is still C-owned
  - `campaign_run(session: &mut CampaignSession) -> CampaignResult`
    - Entry: call `start_game(session)`, return if should_exit
    - Init: `init_campaign_kernel()`, `init_game_clock()`, `add_initial_game_events()`
    - Loop body per §5.1 priority order:
      1. Adopt deferred transition if pending
      2. Encounter/starbase dispatch if `start_encounter`
      3. Interplanetary dispatch if `start_interplanetary`
      4. Default: hyperspace
    - Terminal checks per §5.2: victory, defeat, restart
    - Cleanup: `uninit_game_clock()`, `free_campaign_kernel()`
  - `CampaignResult` enum: `Victory`, `Defeat`, `Restart`, `Quit`
  - `start_game(session: &mut CampaignSession) -> StartResult`
    - Loop: `try_start_game()` until valid start
    - NewGame: `init_new_campaign()`, optional intro, set Interplanetary/Sol
    - LoadGame: call `load_game()`, retry on failure
    - SuperMelee/Quit: set should_exit
  - `init_new_campaign(session: &mut CampaignSession)`
    - Clear all state via `session.clear()`
    - Set start date, Sol location, Interplanetary mode
    - Initialize escort queue with starting ship
    - Initialize game-state bitfield defaults
  - `adopt_deferred_transition(session: &mut CampaignSession)`
    - Take pending transition, set activity and flags
    - Verify: no save-slot mutation occurs (test this)
  - `request_deferred_transition(session: &mut CampaignSession, target: CampaignActivity, flags: TransitionFlags)`
  - Comprehensive tests:
    - Loop dispatch priority: deferred > encounter > interplanetary > hyperspace
    - New-game produces correct initial state (Sol, Interplanetary, start date)
    - Deferred transition consumed exactly once (does not repeat)
    - Deferred transition does not mutate save slots using persistence-boundary fixture diff or equivalent artifact observation, not only a mock
    - Terminal conditions exit loop correctly
    - Restart clears all state (no carry-over)
    - Load-game resumes in correct mode
    - Loop handles all 5 activity modes

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod loop_dispatch;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `loop_dispatch.rs` created with campaign loop and start flow
- [ ] Module wired into `campaign/mod.rs`
- [ ] All callable seams used by this phase are traceable to validated P01/P03.5 inventory rows

## Semantic Verification Checklist (Mandatory)
- [ ] Dispatch priority order matches §5.1: deferred > encounter/starbase > interplanetary > hyperspace
- [ ] Deferred transition consumed exactly once, not repeated
- [ ] Deferred transition produces no save-slot mutation (§5.3) based on persistence-boundary evidence, not only in-memory assertions
- [ ] New-game initializes at Sol, Interplanetary, correct start date (§4.2)
- [ ] Start flow loops until valid start or exit (§4.1)
- [ ] Restart tears down all state, no carry-over (§4.4)
- [ ] Victory/defeat/restart exit loop correctly (§5.2)
- [ ] Clock init/uninit called at correct lifecycle points

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/loop_dispatch.rs
```

## Success Criteria
- [ ] Campaign loop dispatches correctly for all activity modes
- [ ] Start flow handles new-game and load-game
- [ ] Deferred transitions work without side effects
- [ ] Terminal conditions produce correct results

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/loop_dispatch.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P09.md`
