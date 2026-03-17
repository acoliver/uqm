# Phase 13: End-to-End Local Integration Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P13`

## Prerequisites
- Required: Phase 12a completed and passed
- Scoped SuperMelee implementation, FFI wiring, compatibility audit decisions, and requirement matrix available

## Requirements Verified (Local / Non-Netplay)

This phase verifies all local-only SuperMelee requirements and the local side of shared requirements:
- entry, initialization, fallback, and teardown
- per-side team editing, bounded names, and fleet value consistency
- built-in team browsing and file-backed team browsing
- valid built-in team load into the active side
- valid saved-team load into the active side
- team save round-trip and save-failure cleanup
- fleet-edit ship picker confirm/cancel behavior
- match-start validation for playable fleets
- battle-facing initial and next combatant selection
- battle handoff/return behavior
- local-only behavior when netplay support is unavailable or disabled

## Verification Tasks

### Integration Test Suite

Create `rust/tests/supermelee_local_integration.rs`:
- `test_full_melee_lifecycle_local` — init -> edit teams -> start battle -> battle returns -> menu continues -> exit
- `test_config_roundtrip_integration` — save config, restart melee, verify config restored
- `test_builtin_team_loads_into_active_side` — select a valid built-in team and verify active side replaced
- `test_saved_team_file_loads_into_active_side` — load a valid saved team file and verify active side replaced
- `test_legacy_mle_file_loading_semantic_interop` — load actual `.mle` fixtures from the legacy subsystem
- `test_fleet_edit_confirm_applies_selection` — picker confirm updates active team slot
- `test_fleet_edit_cancel_leaves_team_unchanged` — picker cancel preserves state
- `test_match_start_blocked_for_unplayable_fleet` — invalid setup remains in menu flow
- `test_initial_combatants_handoff_preserves_battle_ready_contract` — verifies returned object/handle shape, not bare IDs
- `test_next_combatant_handoff_preserves_battle_ready_contract` — replacement selection preserves contract
- `test_local_mode_requires_no_network_state` — local setup and handoff proceed with netplay disabled

### Manual Verification Checklist (Human Tester)
- [ ] Launch game/build with Rust SuperMelee integration enabled
- [ ] Enter SuperMelee from main menu
- [ ] Verify fallback defaults are usable when no persisted setup exists
- [ ] Load a built-in team into side 0 and verify active side contents change
- [ ] Load a saved team file into side 1 and verify active side contents change
- [ ] Edit fleet composition and verify fleet value updates consistently
- [ ] Edit team name and verify bounded rendering/persistence behavior
- [ ] Open ship picker, cancel, and verify no slot change
- [ ] Open ship picker, confirm, and verify slot changes
- [ ] Attempt to start with an empty/unplayable fleet and verify start is blocked
- [ ] Start a valid local battle and verify battle returns to SuperMelee menu state
- [ ] Exit SuperMelee and re-enter to verify setup persistence restore/sanitization behavior

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --test supermelee_local_integration
```

## Structural Verification Checklist
- [ ] Local integration test file exists
- [ ] Local integration tests cover setup, persistence, picker flows, and battle handoff
- [ ] Requirement matrix rows referenced by this phase point to concrete local tests/checklists

## Semantic Verification Checklist (Mandatory)
- [ ] A valid built-in team load is verified separately from saved-team-file load
- [ ] Fleet-edit confirm and cancel semantics are verified separately
- [ ] Local-only behavior is explicitly verified with netplay disabled/unsupported
- [ ] Battle-facing handoff verification checks for battle-ready objects/handles rather than mere ship IDs or slot indexes
- [ ] Setup fallback, persistence, and battle return produce a usable local menu state
- [ ] Compatibility-sensitive exactness checks only assert outcomes required by the compatibility audit

## Success Criteria
- [ ] All local integration tests pass
- [ ] Manual verification confirms local SuperMelee usability end-to-end
- [ ] Requirement-matrix local rows have concrete verification evidence

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P13.md`
