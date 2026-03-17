# Phase 06: Team Model & Persistence

## Phase ID
`PLAN-20260314-SUPERMELEE.P06`

## Prerequisites
- Required: Phase 05a completed and passed
- Core types and errors available from `types.rs`, `error.rs`, and `setup/team.rs`

## Purpose

Implement the first substantive SuperMelee-owned behavior slice after core types:
- editable team state and fleet-value consistency,
- built-in team catalog initialization and unified browser enumeration,
- legacy `.mle` load interoperability and save semantics,
- `melee.cfg` load/save and startup sanitization,
- clean failure behavior for malformed persisted artifacts.

This phase replaces the stale battle-engine work previously occupying P06 and closes the plan gap around persistence and built-in-team requirements.

## Implementation Tasks

### Files to create or complete

- `rust/src/supermelee/setup/team.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P06`
  - Implement:
    - `MeleeTeam`
    - `MeleeSetup`
    - bounded team-name storage/normalization helpers
    - slot mutation helpers: `set_ship`, `clear_slot`, `replace_team`
    - derived helpers: `fleet_value`, `recompute_value`, `is_playable`

- `rust/src/supermelee/setup/persistence.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P06`
  - Implement:
    - built-in team catalog initialization
    - unified team browser enumeration across built-ins and saved `.mle` files
    - valid built-in team load into active side
    - valid saved `.mle` load into active side
    - malformed/unreadable team-file failure without corrupting active setup state
    - safe save flow with partial-write cleanup semantics
    - semantic reloadability of newly written team files

- `rust/src/supermelee/setup/config.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P06`
  - Implement:
    - `melee.cfg` load
    - `melee.cfg` write
    - startup fallback result classification for missing/invalid config
    - sanitization/downgrade of transient network-only control modes when restoring local startup state

### Tests to create

- `rust/src/supermelee/setup/team_tests.rs`
  - `test_set_ship_updates_active_slot_and_fleet_value`
  - `test_clear_slot_marks_slot_empty_and_updates_fleet_value`
  - `test_replace_team_updates_name_slots_and_fleet_value`
  - `test_team_name_is_bounded_and_valid`
  - `test_is_playable_requires_at_least_one_valid_ship`

- `rust/src/supermelee/setup/persistence_tests.rs`
  - `test_builtin_catalog_initializes_valid_teams`
  - `test_team_browser_exposes_builtin_and_saved_entries`
  - `test_valid_builtin_team_loads_into_active_side`
  - `test_valid_saved_team_loads_into_active_side`
  - `test_invalid_saved_team_fails_without_corrupting_active_state`
  - `test_legacy_mle_load_preserves_slots_and_name`
  - `test_save_failure_cleans_partial_artifact`
  - `test_saved_team_roundtrip_preserves_semantic_payload`

- `rust/src/supermelee/setup/config_tests.rs`
  - `test_missing_config_requests_builtin_fallback`
  - `test_invalid_config_requests_builtin_fallback`
  - `test_valid_config_restores_setup_and_controls`
  - `test_transient_network_control_is_sanitized_for_startup`
  - `test_config_write_roundtrip_restores_team_state`

### Files to modify

- `rust/src/supermelee/setup/mod.rs`
  - ensure `team`, `persistence`, and `config` modules are declared and test modules are wired per project conventions

### Pseudocode traceability
- Uses pseudocode lines: 001–072

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features team_tests persistence_tests config_tests
```

## Structural Verification Checklist
- [ ] `team.rs`, `persistence.rs`, and `config.rs` exist under `setup/`
- [ ] Team, persistence, and config tests exist
- [ ] Built-in catalog and saved-team enumeration are planned in the persistence module rather than deferred to E2E only
- [ ] `melee.cfg` handling is planned as first-class implementation work in this phase

## Semantic Verification Checklist (Mandatory)
- [ ] Empty fleet slots remain distinct from occupied slots
- [ ] Fleet value remains consistent after set/clear/replace operations
- [ ] Built-in team loading is specified separately from saved-team loading
- [ ] Malformed saved-team files fail without mutating the active side
- [ ] Save-failure cleanup semantics prevent an apparently successful corrupted artifact
- [ ] Restored startup state sanitizes transient network-only control modes when required
- [ ] The plan preserves semantic legacy `.mle` load interoperability without prematurely claiming byte-for-byte save parity

## Success Criteria
- [ ] Team model behavior, `.mle` load/save, built-in catalog browsing, and `melee.cfg` persistence all have concrete implementation tasks
- [ ] Phase-local tests exist for substantive persistence and sanitization behavior
- [ ] This phase provides real backing for the persistence and built-in-team requirements referenced by the overview/tracker

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/setup/team.rs rust/src/supermelee/setup/persistence.rs rust/src/supermelee/setup/config.rs`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P06.md`
