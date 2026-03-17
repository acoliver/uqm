# Phase 08a: Encounter Lifecycle Verification

## Phase ID
`PLAN-20260314-COMM.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `encounter.rs` exists and is registered in `mod.rs`
- [ ] `race_communication`, `init_communication`, `hail_alien`, `destroy_encounter_resources` implemented
- [ ] Lock release/reacquire around all C callbacks
- [ ] Encounter callback tracking (init_called, post_called, uninit_called)
- [ ] Resource loading and destruction functions present
- [ ] Encounter flow FFI declarations present
- [ ] Saved-game SIS update hook present in the `RaceCommunication()` path

## Semantic Verification Checklist

### Entry-Point Routing
- [ ] `test_race_communication_context_resolution` — `RaceCommunication()` chooses the expected `CONVERSATION` for representative contexts
- [ ] `test_race_communication_saved_game_sis_update` — SIS update occurs before `InitCommunication()` setup when a save was just loaded
- [ ] `test_init_communication_direct_path` — direct `InitCommunication(which_comm)` still works for already-resolved callers

### Lifecycle Callback Ordering
- [ ] `test_normal_exit_callback_order` — init→dialogue→post→uninit
- [ ] `test_abort_exit_callback_order` — init→abort→uninit (no post)
- [ ] `test_attack_no_hail_callback_order` — post→uninit (no init, no dialogue)
- [ ] `test_callback_exactly_once` — verify counters show exactly one call each
- [ ] `test_no_double_teardown` — encounter teardown doesn't re-invoke callbacks (CV-REQ-017)

### Resource Lifecycle
- [ ] `test_resources_loaded_before_scripts` — all handles non-zero after load
- [ ] `test_resources_destroyed_on_teardown` — all handles zeroed after destroy
- [ ] `test_resources_destroyed_reverse_order` — reverse of creation order
- [ ] `test_teardown_after_abort` — resources still destroyed on abort
- [ ] `test_no_leak_repeated_encounters` — multiple encounters don't accumulate resources (CV-REQ-001)

### State Validity
- [ ] `test_reuse_after_teardown` — new encounter can init after previous teardown
- [ ] `test_encounter_active_flag` — true during encounter, false after
- [ ] `test_phrase_state_reset_on_new_encounter` — no carryover from previous encounter

### Hail/Attack Decision
- [ ] `test_battle_segue_zero_enters_dialogue` — BATTLE_SEGUE=0 goes to hail_alien
- [ ] `test_battle_segue_nonzero_presents_choice` — BATTLE_SEGUE!=0 shows hail/attack
- [ ] `test_attack_sets_battle_segue` — choosing attack sets BATTLE_SEGUE=1

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/encounter.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P08a.md`
