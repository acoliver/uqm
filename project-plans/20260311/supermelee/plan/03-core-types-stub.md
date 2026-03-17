# Phase 03: Core Types & Error — Stub

## Phase ID
`PLAN-20260314-SUPERMELEE.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed and passed
- Expected artifacts from previous phases: analysis, pseudocode

## Requirements Implemented (Expanded)

### Team/Fleet Model
**Requirement text**: The subsystem shall maintain editable team state independently for each side. Each team's state shall include a team name and a fleet of ship slots. The subsystem shall represent empty fleet slots distinctly from occupied fleet slots.

Behavior contract:
- GIVEN: A SuperMelee session is active
- WHEN: Any team operation occurs
- THEN: Team state is independently maintained per side with typed ship slots and bounded names

### Fleet Value
**Requirement text**: The subsystem shall maintain a fleet value or equivalent derived team-strength summary consistent with the team's current ship contents.

Behavior contract:
- GIVEN: A team with ships assigned
- WHEN: Fleet value is queried
- THEN: The returned value equals the sum of costs of all non-empty ship slots

### Error Handling
**Requirement text**: Failure in one team file, built-in team entry, or setup artifact shall not corrupt unrelated valid team state already present in memory.

Behavior contract:
- GIVEN: Any subsystem operation
- WHEN: An error occurs
- THEN: The error is represented as a typed Result, not a panic or silent corruption

## Implementation Tasks

### Files to create

- `rust/src/supermelee/mod.rs` — Module root with sub-module declarations
  - marker: `@plan PLAN-20260314-SUPERMELEE.P03`

- `rust/src/supermelee/types.rs` — Core types: `MeleeShip` enum, constants, control types
  - marker: `@plan PLAN-20260314-SUPERMELEE.P03`
  - marker: `@requirement team/fleet model`
  - Contents:
    - `MeleeShip` enum with all melee ships plus explicit empty/unset sentinels as verified against `meleeship.h`
    - `MELEE_FLEET_SIZE = 14` constant
    - `MAX_TEAM_CHARS` constant
    - `NUM_SIDES = 2` constant
    - `PlayerControl` flags/newtype matching the audited setup-side control representation
    - `BattleReadyCombatant` placeholder wrapper type or trait alias marked as a **design placeholder** until Phase P08 audits the exact battle-facing type
    - `SelectionCommit` struct for internal selection-state bookkeeping

- `rust/src/supermelee/error.rs` — Error types
  - marker: `@plan PLAN-20260314-SUPERMELEE.P03`
  - marker: `@requirement error handling`
  - Contents:
    - `SuperMeleeError` enum with variants such as:
      - `InvalidShipId(u8)`
      - `InvalidTeamData`
      - `PersistenceError`
      - `ConfigError`
      - `SelectionError`
      - `BattleHandoffError`
      - `NetplayValidationError`
      - `CompatibilityAuditPending`

- `rust/src/supermelee/setup/mod.rs` — Setup sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-SUPERMELEE.P03`

- `rust/src/supermelee/setup/team.rs` — Team data model stubs
  - marker: `@plan PLAN-20260314-SUPERMELEE.P03`
  - marker: `@requirement team/fleet model`
  - Contents:
    - `MeleeTeam` struct with `ships: [MeleeShip; MELEE_FLEET_SIZE]`, bounded name storage
    - `MeleeSetup` struct with `teams`, `fleet_value`, and setup-owned control-mode state as verified in analysis
    - `impl MeleeTeam` with `new()`, `Default`
    - `impl MeleeSetup` with `new()`, `Default`
    - Stub methods: `set_ship()`, `set_team_name()`, `replace_team()`, `get_fleet_value()`, `serial_size()`

### Files to modify

- `rust/src/lib.rs` — Add `pub mod supermelee;`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P03`

### Pseudocode traceability
- Uses pseudocode lines: 001–027 (Team Model)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/supermelee/mod.rs` exists and declares sub-modules
- [ ] `rust/src/supermelee/types.rs` exists with `MeleeShip` enum
- [ ] `rust/src/supermelee/error.rs` exists with `SuperMeleeError` enum
- [ ] `rust/src/supermelee/setup/mod.rs` exists
- [ ] `rust/src/supermelee/setup/team.rs` exists with struct definitions
- [ ] `rust/src/lib.rs` includes `pub mod supermelee`
- [ ] All files compile without errors

## Semantic Verification Checklist (Mandatory)
- [ ] `MeleeShip` values are explicitly marked as verified against C ordering before later phases depend on them
- [ ] `PlayerControl` representation is limited to setup-owned meaning; netplay transport concerns are not pulled into this phase
- [ ] Any battle-facing combatant type in this phase is clearly labeled as a placeholder until Phase P08 audits the real contract
- [ ] `MeleeTeam` and `MeleeSetup` contain only SuperMelee-owned state
- [ ] Error enum covers persistence, setup, handoff, and netplay-boundary failures without implying ownership of battle simulation internals

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase: todo!() is allowed in method bodies only.
# Verify no fake semantic-success gates are being used in place of real implementation.
grep -RIn "todo!()\|unimplemented!()" rust/src/supermelee/ | grep -v test
```

## Success Criteria
- [ ] All new files compile
- [ ] Module structure matches the scoped plan
- [ ] Types are defined with correct SuperMelee-owned field intent
- [ ] Placeholder vs verified boundary types are clearly distinguished
- [ ] `cargo test` passes (compilation gate)

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/ rust/src/lib.rs`
- blocking issues: type mismatches with audited C/setup contracts, missing required dependencies

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P03.md`
