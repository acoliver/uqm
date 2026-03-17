# Phase 03: Types, Enums, Domain Model

## Phase ID
`PLAN-20260314-CAMPAIGN.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Verify previous phase markers/artifacts exist

## Requirements Implemented (Expanded)

### Campaign Activity Vocabulary (§3.1)
**Requirement text**: The subsystem shall maintain campaign state sufficient to distinguish hyperspace navigation, interplanetary/solar-system, encounter, starbase visit, and last battle for dispatch, resume, and verification.

Behavior contract:
- GIVEN: Any point during campaign execution
- WHEN: The campaign activity is queried
- THEN: Exactly one of the five observable modes is active and distinguishable

### Campaign Runtime State (§3.2)
**Requirement text**: The subsystem shall maintain runtime state sufficient to represent the recoverable campaign mode, pending transitions, clock reference, navigation position, queues, bitfield, and ship state.

Behavior contract:
- GIVEN: A new campaign session
- WHEN: State is initialized
- THEN: All campaign-owned fields have valid default or starting values, and all legacy-owned state dependencies are represented through explicit bridge/access placeholders rather than speculative copies

### Transition Flags (§3.1)
**Requirement text**: The subsystem shall maintain transition flags for encounter/starbase start, interplanetary start, and load/restart/abort control.

## Implementation Tasks

### Files to create

- `rust/src/campaign/mod.rs` — Module root
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03`
  - Re-exports all public campaign types
  - Submodule declarations

- `rust/src/campaign/activity.rs` — Campaign activity enum and transition model
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03`
  - marker: `@requirement §3.1`
  - `CampaignActivity` enum: `HyperspaceNavigation`, `Interplanetary`, `Encounter`, `StarbaseVisit`, `LastBattle`
  - `TerminalOutcome` enum: `Victory`, `Defeat`
  - `TransitionFlags` struct with bitfield-style flags: `start_encounter`, `start_interplanetary`, `check_load`, `check_restart`, `check_abort`
  - `DeferredTransition` struct: `target_activity: CampaignActivity`, `flags: TransitionFlags`
  - Conversion traits or helper functions for legacy activity/flag tokens only after validated values from `globdata.h`
  - Constants for validated legacy C flag values: `IN_LAST_BATTLE`, `IN_ENCOUNTER`, `IN_HYPERSPACE`, `IN_INTERPLANETARY`, `START_ENCOUNTER`, `START_INTERPLANETARY`, `CHECK_LOAD`, `CHECK_RESTART`, `CHECK_ABORT`
  - Unit tests for enum conversions and flag operations

- `rust/src/campaign/types.rs` — Shared campaign types
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03`
  - `HyperspaceCoords` struct: `x: i32, y: i32`
  - `SystemId` struct: `x: i32, y: i32` (baseline system coordinates)
  - `ShipIdentity` struct: `race_id: u16, ship_type_id: u16`
  - `QueueEntry` struct for persisted or bridge-normalized escort/NPC/encounter queue items
  - `EncounterIdentity` enum with closed baseline vocabulary: `Arilou`, `BlackUrquan`, `Chmmr`, `Druuge`, `Human`, `Ilwrath`, `Mycon`, `Orz`, `Pkunk`, `Shofixti`, `Slylandro`, `Spathi`, `Supox`, `Thraddash`, `Umgah`, `Urquan`, `Utwig`, `Vux`, `Yehat`, `Zoqfotpik`, `Melnorme`, `TalkingPet`, `Samatra`, `Starbase`, `SamatraHomeworld`
  - `FactionId` enum with closed baseline vocabulary matching §10.1
  - `AllianceStatus` enum: `Allied`, `Hostile`, `Neutral`, `Dead`
  - `ShipStateToken` struct or equivalent validated boundary representation for persisted ship-state fields until concrete source types are proven
  - Serialization support via `serde::Serialize` / `serde::Deserialize` derives
  - `impl Display` for canonical string representations (lowercase_with_underscores)
  - Unit tests for all type conversions and serialization

- `rust/src/campaign/session.rs` — Campaign session state container
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03`
  - marker: `@requirement §3.2`
  - `CampaignSession` struct:
    - `current_activity: CampaignActivity`
    - `pending_transition: Option<DeferredTransition>`
    - `transition_flags: TransitionFlags`
    - `navigation_position: HyperspaceCoords`
    - `destination_system: Option<SystemId>`
    - `in_quasispace: bool`
    - `starbase_context: bool`
    - `autopilot_target: Option<HyperspaceCoords>`
    - `ship_state: ShipStateToken`
    - `orbit_flags: u32`
    - `escort_queue_view: Vec<QueueEntry>` or validated opaque bridge-backed queue view type
    - `npc_ship_queue_view: Vec<QueueEntry>` or validated opaque bridge-backed queue view type
    - `encounter_queue_view: Vec<QueueEntry>` or validated opaque bridge-backed queue view type
    - `battle_segue: bool`
    - `should_exit: bool`
  - `CampaignSession::new()` — default initialization
  - `CampaignSession::clear()` — reset all campaign-owned state (for restart/new-game)
  - Helper methods: `is_in_hyperspace()`, `is_in_interplanetary()`, `is_encounter()`, `is_starbase()`, `is_last_battle()`
  - Helper methods: `has_start_encounter()`, `has_start_interplanetary()`, `has_check_load()`, `has_restart_requested()`
  - Explicit docs noting which fields are source-of-truth owned here vs mirrored/derived from bridge state
  - Unit tests for state initialization, clearing, and helper predicates

### Files to modify

- `rust/src/lib.rs`
  - Add `pub mod campaign;`
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03`

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/campaign/mod.rs` created with submodule declarations
- [ ] `rust/src/campaign/activity.rs` created with `CampaignActivity` enum
- [ ] `rust/src/campaign/types.rs` created with shared types
- [ ] `rust/src/campaign/session.rs` created with `CampaignSession`
- [ ] `rust/src/lib.rs` updated with `pub mod campaign`
- [ ] Plan/requirement traceability markers present in all new files

## Semantic Verification Checklist (Mandatory)
- [ ] All 5 campaign modes from §3.1 represented in `CampaignActivity`
- [ ] All transition flags from §3.1 represented in `TransitionFlags`
- [ ] `CampaignSession` contains all campaign-owned runtime state fields from §3.2
- [ ] Queue/state fields that remain legacy-owned are represented as validated bridge views/placeholders, not assumed Rust-owned source-of-truth copies
- [ ] `EncounterIdentity` enum matches closed vocabulary from §10.1
- [ ] `FactionId` enum matches closed vocabulary from §10.1
- [ ] C-compatible flag constants match values in `globdata.h`
- [ ] Tests verify enum round-trip conversions
- [ ] Tests verify session initialization and clear behavior

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/
```

## Success Criteria
- [ ] All types compile and tests pass
- [ ] Enum conversions to/from C flag values are correct
- [ ] Session state can be created, inspected, and cleared without prematurely fixing unresolved ownership seams

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/ rust/src/lib.rs`
- blocking issues: type conflicts with existing Rust modules, missing serde dependency

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P03.md`
