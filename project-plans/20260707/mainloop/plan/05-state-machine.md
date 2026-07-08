# Phase 05: Activity State Machine

## Phase ID
`PLAN-20260707-MAINLOOP.P05`

## Prerequisites
- Phase 04 complete (init infrastructure + FFI bridge pattern established)

## Requirements Implemented

### REQ-ML-004: Activity State Machine in Rust
The activity dispatch logic from `Starcon2Main` (starcon.c:210-290) is
implemented in Rust as a typed state machine.

---

## Implementation Tasks

### Stub
**Files to create:**
- `rust/src/mainloop/state_machine.rs` — `ActivityStateMachine` struct,
  `ActivityDecision` enum, `evaluate()` and `post_dispatch()` methods
  (initially `todo!()`)
  - marker: `@plan PLAN-20260707-MAINLOOP.P05 @requirement REQ-ML-004`

**Files to modify:**
- `rust/src/mainloop/mod.rs` — add `pub mod state_machine;`

### TDD
**Tests in `rust/src/mainloop/state_machine.rs`:**

```rust
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_dispatch_encounter_with_bomb_and_starbase() {
    // GIVEN: START_ENCOUNTER + CHMMR_BOMB_STATE == 2 + STARBASE_AVAILABLE
    uqm_set_chmmr_bomb_state(2);
    // starbase available (mock or real accessor)
    set_mock_game_state("STARBASE_AVAILABLE", 1);
    let activity = ActivityValue(0x0403); // START_ENCOUNTER | IN_ENCOUNTER
    let decision = ActivityStateMachine::evaluate(activity);
    assert_eq!(decision, ActivityDecision::VisitStarBase);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_dispatch_encounter_bomb_no_starbase_installs_bomb() {
    // GIVEN: bomb==2 but NOT starbase_available
    uqm_set_chmmr_bomb_state(2);
    set_mock_game_state("STARBASE_AVAILABLE", 0);
    let activity = ActivityValue(0x0402);
    let decision = ActivityStateMachine::evaluate(activity);
    assert_eq!(decision, ActivityDecision::InstallBombAtEarth);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_dispatch_encounter_global_flags_all_set() {
    // GIVEN: GLOBAL_FLAGS_AND_DATA == 0xFF (all flags set)
    set_mock_game_state("GLOBAL_FLAGS_AND_DATA", 0xFF);
    set_mock_game_state("CHMMR_BOMB_STATE", 0);
    let activity = ActivityValue(0x0402); // START_ENCOUNTER
    let decision = ActivityStateMachine::evaluate(activity);
    assert_eq!(decision, ActivityDecision::VisitStarBase);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_dispatch_encounter_race_comm() {
    set_mock_game_state("CHMMR_BOMB_STATE", 0);
    set_mock_game_state("GLOBAL_FLAGS_AND_DATA", 0);
    set_mock_game_state("STARBASE_AVAILABLE", 0);
    let activity = ActivityValue(0x0402);
    let decision = ActivityStateMachine::evaluate(activity);
    assert_eq!(decision, ActivityDecision::RaceCommunication);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_dispatch_interplanetary() {
    let activity = ActivityValue(0x0804);
    let decision = ActivityStateMachine::evaluate(activity);
    assert_eq!(decision, ActivityDecision::ExploreSolarSystem);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_dispatch_default_battle() {
    let activity = ActivityValue(0x0003);
    let decision = ActivityStateMachine::evaluate(activity);
    assert_eq!(decision, ActivityDecision::Battle);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_post_dispatch_clears_encounter_flag_for_encounter_branch() {
    // ENCOUNTER BRANCH ONLY: starcon.c:263-268
    let mut activity = ActivityValue(0x0404); // START_ENCOUNTER | IN_INTERPLANETARY
    let result = ActivityStateMachine::post_dispatch(activity, true /* was_encounter */);
    assert!(!result.has_flag(ActivityFlags::START_ENCOUNTER));
    assert!(result.has_flag(ActivityFlags::START_INTERPLANETARY));
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_post_dispatch_no_mutation_for_non_encounter_branch() {
    // Non-encounter branches (ExploreSolarSystem, Battle) do NOT run
    // the START_ENCOUNTER clearing / START_INTERPLANETARY setting block.
    let activity = ActivityValue(0x0404);
    let result = ActivityStateMachine::post_dispatch(activity, false /* not encounter */);
    // No mutation for non-encounter branch
    assert_eq!(result.0, 0x0404);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_post_dispatch_preserves_flags_on_abort_or_load() {
    // CRITICAL: must NOT clear flags when CHECK_ABORT or CHECK_LOAD is set
    let activity = ActivityValue(0x4404); // CHECK_ABORT | START_ENCOUNTER | IN_INTERPLANETARY
    let result = ActivityStateMachine::post_dispatch(activity, true /* was_encounter */);
    // Flags should be preserved (no mutation)
    assert!(result.has_flag(ActivityFlags::START_ENCOUNTER));
    assert!(result.has_flag(ActivityFlags::CHECK_ABORT));
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_should_stop_on_won_last_battle() {
    // No CHECK_ABORT/CHECK_LOAD — WON_LAST_BATTLE triggers stop
    let activity = ActivityValue(0x0005); // WON_LAST_BATTLE
    assert!(should_stop_loop(activity));
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_should_not_stop_on_won_with_check_abort() {
    // CHECK_ABORT | WON_LAST_BATTLE: should_stop_loop returns false
    // (the loop exits via CHECK_ABORT condition, not should_stop_loop)
    let activity = ActivityValue(0x4005); // CHECK_ABORT | WON_LAST_BATTLE
    assert!(!should_stop_loop(activity));
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_should_stop_on_player_death() {
    // CrewEnlisted == 0xFFFF means player died
    set_mock_crew_enlisted(0xFFFF);
    let activity = ActivityValue(0x0003); // normal hyperspace
    assert!(should_stop_loop(activity));
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[test]
fn test_kohr_ah_triggers_blackurq_communication() {
    // WON_LAST_BATTLE + KOHR_AH_KILLED_ALL → InitCommunication(BLACKURQ)
    set_mock_game_state("KOHR_AH_KILLED_ALL", 1);
    let activity = ActivityValue(0x0005); // WON_LAST_BATTLE (no CHECK_ABORT)
    should_stop_loop(activity); // should call InitCommunication internally
    verify_mock_called("InitCommunication", BLACKURQ_CONVERSATION);
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
/// Load path: NextActivity with CHECK_LOAD suppresses ZeroVelocityComponents
#[test]
fn test_load_path_next_activity_check_load_suppresses_zero_velocity() {
    // starcon.c:237: if NOT ((CurrentActivity | NextActivity) & CHECK_LOAD)
    // CurrentActivity = 0, NextActivity = CHECK_LOAD
    set_mock_activity(0x0000);
    set_mock_next_activity(0x1000); // CHECK_LOAD = MAKE_WORD(0, 1<<4)
    let should_zero = should_zero_velocity();
    assert!(!should_zero); // suppressed because NextActivity has CHECK_LOAD
}

/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
/// Load path: CurrentActivity with CHECK_LOAD replaces with NextActivity
#[test]
fn test_load_path_current_activity_check_load_replaces() {
    // starcon.c:240-241: if CurrentActivity & CHECK_LOAD, CurrentActivity = NextActivity
    set_mock_activity(0x1003); // CHECK_LOAD | IN_HYPERSPACE
    set_mock_next_activity(0x0004); // IN_INTERPLANETARY
    let new_activity = resolve_load_activity();
    assert_eq!(new_activity.0, 0x0004); // CurrentActivity replaced by NextActivity
}
```

### Impl
- Implement `evaluate()` — pseudocode lines 70-96
- Implement `post_dispatch(activity, was_encounter)` — pseudocode lines 48-54
  (encounter-only mutation)
- Implement `should_zero_velocity()` — pseudocode lines 31-32
  (checks CurrentActivity | NextActivity & CHECK_LOAD)
- Implement `resolve_load_activity()` — pseudocode lines 33-36
  (CurrentActivity & CHECK_LOAD → CurrentActivity = NextActivity)
- Add FFI accessors for crew_enlisted(), game state reads in c_extern.rs

### Pseudocode traceability
- Uses pseudocode lines: 31-36 (load path), 48-54 (encounter post-dispatch), 70-96 (evaluate), 120-134 (should_stop)

---

## Verification Commands
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Semantic Verification Checklist
- [ ] All 4 dispatch branches tested (encounter/bomb, encounter/normal, interplanetary, battle)
- [ ] Post-dispatch flag clearing matches starcon.c:262-268
- [ ] Decision enum is exhaustive

## Deferred Implementation Detection
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/mainloop/state_machine.rs
```

## Success Criteria
- [ ] REQ-ML-004: state machine evaluates all branches correctly
- [ ] All dispatch tests pass

## Failure Recovery
- `git restore rust/src/mainloop/state_machine.rs`

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P05.md`
