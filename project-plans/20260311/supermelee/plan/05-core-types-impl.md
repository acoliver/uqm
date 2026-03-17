# Phase 05: Core Types & Error — Implementation

## Phase ID
`PLAN-20260314-SUPERMELEE.P05`

## Prerequisites
- Required: Phase 04a completed and passed
- Expected: failing tests from P04 that need implementations to pass

## Requirements Implemented (Expanded)

### SM-REQ: Team/Fleet Model — Full Implementation
**Requirement text**: The subsystem shall maintain editable team state independently for each side.

Behavior contract:
- GIVEN: Stub types from P03 with failing tests from P04
- WHEN: Implementation is completed
- THEN: All P04 tests pass — team model is fully functional

### SM-REQ: Invalid Ship IDs
**Requirement text**: When invalid ship identifiers are encountered in persisted team data, the subsystem shall fail cleanly or normalize consistently.

Behavior contract:
- GIVEN: Byte value outside valid MeleeShip range
- WHEN: `MeleeShip::from_u8()` is called
- THEN: Returns `MeleeShip::MeleeNone` (clamped)

## Implementation Tasks

### Files to modify

- `rust/src/supermelee/types.rs` — Implement all type methods
  - marker: `@plan PLAN-20260314-SUPERMELEE.P05`
  - Changes:
    - `MeleeShip::from_u8()` — validated conversion with MELEE_NONE fallback
    - `MeleeShip::to_u8()` — reverse conversion
    - `MeleeShip::is_valid()` — true for actual ships, false for NONE/UNSET
    - `MeleeShip::cost()` — returns ship cost (delegates to ship catalog or uses embedded table)
    - `SpeciesId::from_melee_ship()` — conversion from melee ship to species ID
    - `impl Display for MeleeShip` — human-readable ship names

- `rust/src/supermelee/setup/team.rs` — Implement all team methods
  - marker: `@plan PLAN-20260314-SUPERMELEE.P05`
  - marker: `@requirement SM-REQ: team/fleet model`
  - Changes:
    - `MeleeTeam::new(name, ships)` — constructor
    - `MeleeTeam::default()` — all NONE, empty name
    - `MeleeTeam::serial_size()` — FLEET_SIZE + name buffer size
    - `MeleeTeam::serialize(&self, buf: &mut [u8])` — write ship bytes + name
    - `MeleeTeam::deserialize(buf: &[u8]) -> Result<Self>` — read with validation
    - `MeleeTeam::get_value(&self) -> u16` — sum of ship costs
    - `MeleeTeam::name_str(&self) -> &str` — safe name accessor
    - `MeleeSetup::new()` — default two-side setup
    - `MeleeSetup::set_ship(side, slot, ship)` — with fleet_value cache update
    - `MeleeSetup::set_team_name(side, name)` — bounded, null-terminated
    - `MeleeSetup::get_fleet_value(side) -> u16` — cached value
    - `MeleeSetup::replace_team(side, team)` — full replacement with value recompute
    - `MeleeSetup::serialize_team(side, buf)` — delegate to MeleeTeam
    - `MeleeSetup::deserialize_team(side, buf)` — delegate + update cache
    - `MeleeSetup::has_playable_fleet(side) -> bool` — at least one valid ship

### Pseudocode traceability
- Uses pseudocode lines: 245–269

## Verification Commands

```bash
# All gates must pass
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] No `todo!()` remains in `types.rs` or `team.rs`
- [ ] All P04 tests pass
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `MeleeShip` round-trip: `from_u8(ship.to_u8()) == ship` for all valid ships
- [ ] Invalid bytes clamp to MELEE_NONE
- [ ] Fleet value is consistent after any sequence of set_ship calls
- [ ] Team serialization round-trips: `deserialize(serialize(team)) == team`
- [ ] Name truncation works for oversized input
- [ ] `has_playable_fleet` returns false for empty team, true for team with ships

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|todo!()" rust/src/supermelee/types.rs rust/src/supermelee/setup/team.rs
# Should return ZERO matches
```

## Success Criteria
- [ ] All tests pass
- [ ] No placeholder code remains
- [ ] Core types are production-ready

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/types.rs rust/src/supermelee/setup/team.rs`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P05.md`
