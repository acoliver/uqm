# Phase 04: Core Types & Error — TDD

## Phase ID
`PLAN-20260314-SUPERMELEE.P04`

## Prerequisites
- Required: Phase 03a completed and passed
- Expected files from P03: `types.rs`, `error.rs`, `team.rs` stubs

## Requirements Implemented (Expanded)

### SM-REQ: Team/Fleet Model
Behavior contracts tested:
- GIVEN: A default MeleeTeam → WHEN: inspected → THEN: all slots are MELEE_NONE, name is empty
- GIVEN: A MeleeSetup → WHEN: set_ship is called → THEN: fleet_value updates correctly
- GIVEN: A MeleeSetup → WHEN: set_team_name is called with oversized name → THEN: name is truncated

### SM-REQ: Invalid Ship IDs in Persisted Data
Behavior contracts tested:
- GIVEN: A byte value > LAST_MELEE_ID → WHEN: MeleeShip::from_u8 → THEN: returns MELEE_NONE
- GIVEN: A valid byte value → WHEN: MeleeShip::from_u8 → THEN: returns correct variant

### SM-REQ: Fleet Value Consistency
Behavior contracts tested:
- GIVEN: An empty team → WHEN: fleet_value queried → THEN: returns 0
- GIVEN: A team with 3 ships → WHEN: one ship removed → THEN: fleet_value decreases by that ship's cost
- GIVEN: A team → WHEN: ship replaced → THEN: fleet_value = old - removed_cost + added_cost

## Implementation Tasks

### Files to create

- `rust/src/supermelee/types_tests.rs` — Tests for MeleeShip enum and constants
  - marker: `@plan PLAN-20260314-SUPERMELEE.P04`
  - Tests:
    - `test_melee_ship_from_u8_valid` — all 25 valid ship IDs round-trip
    - `test_melee_ship_from_u8_invalid` — out-of-range values → MELEE_NONE
    - `test_melee_ship_from_u8_none` — MELEE_NONE byte round-trips
    - `test_melee_ship_from_u8_unset` — MELEE_UNSET byte round-trips
    - `test_melee_ship_is_valid` — valid ships return true, NONE/UNSET return false
    - `test_species_id_matches_c_values` — enum discriminants match C header values
    - `test_player_control_flags` — flag combinations work correctly
    - `test_constants_match_c` — MELEE_FLEET_SIZE, NUM_SIDES, BATTLE_FRAME_RATE

- `rust/src/supermelee/setup/team_tests.rs` — Tests for team model
  - marker: `@plan PLAN-20260314-SUPERMELEE.P04`
  - Tests:
    - `test_melee_team_default_all_none` — default team has all MELEE_NONE slots
    - `test_melee_team_default_empty_name` — default team name is empty/zeroed
    - `test_melee_team_new_with_name` — constructor sets name correctly
    - `test_melee_setup_default_zero_value` — default setup has zero fleet values
    - `test_melee_setup_set_ship_updates_value` — setting a ship updates cached value
    - `test_melee_setup_set_ship_replace_updates_value` — replacing ship adjusts value correctly
    - `test_melee_setup_set_ship_same_noop` — setting same ship is a no-op
    - `test_melee_setup_set_ship_to_none_decreases_value` — clearing slot decreases value
    - `test_melee_setup_set_team_name_bounded` — oversized name is truncated
    - `test_melee_setup_set_team_name_null_terminated` — name always null-terminated
    - `test_melee_setup_replace_team` — full team replacement updates all state
    - `test_melee_team_serialize_size` — serial size is FLEET_SIZE + name_size
    - `test_melee_team_serialize_roundtrip` — serialize then deserialize preserves state
    - `test_melee_team_deserialize_invalid_ship_ids` — invalid IDs clamped to NONE
    - `test_melee_team_deserialize_short_buffer` — short buffer returns error

### Pseudocode traceability
- Uses pseudocode lines: 245–269

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Tests should FAIL at this point (stubs return todo!())
# This is expected — confirms tests are testing real behavior
```

## Structural Verification Checklist
- [ ] Test files exist
- [ ] Tests compile (even if they panic on `todo!()`)
- [ ] At least 15 test functions defined
- [ ] Tests reference plan/requirement markers in comments

## Semantic Verification Checklist (Mandatory)
- [ ] Tests cover all behavior contracts listed above
- [ ] Tests verify behavior, not implementation details
- [ ] Error path tests exist (invalid data, short buffer)
- [ ] Round-trip serialization test exists
- [ ] Fleet value consistency test covers add/remove/replace

## Success Criteria
- [ ] All tests compile
- [ ] Tests fail with `todo!()` panics (proving they test real behavior)
- [ ] No tests that would pass with trivial stub implementations

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/`
- blocking issues: test infrastructure problems

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P04.md`
