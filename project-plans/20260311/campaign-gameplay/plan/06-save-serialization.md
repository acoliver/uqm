# Phase 06: Save Serialization

## Phase ID
`PLAN-20260314-CAMPAIGN.P06`

## Prerequisites
- Required: Phase 05a completed
- Expected files: `save/summary.rs`, `save/export.rs`, campaign types
- Dependency: `rust/src/io/` — file I/O primitives
- Dependency: `rust/src/state/state_file.rs` — state-file helpers for battle-group persistence

## Requirements Implemented (Expanded)

### Campaign Save (from requirements.md)
**Requirement text**: When a save is requested, the subsystem shall persist enough campaign state to support a full round-trip resume.

Behavior contract:
- GIVEN: An active campaign session in any covered context
- WHEN: `save_game(slot)` is called
- THEN: Summary, game state, queues, and battle-group state files are persisted; load of the resulting save restores equivalent state

### Save-time Adjustments (§9.3)
**Requirement text**: When a save is requested from a special activity context, the subsystem shall apply campaign-specific save-time adjustments.

Behavior contract:
- GIVEN: Campaign at homeworld encounter screen
- WHEN: Save is requested
- THEN: Save-time adjustment normalizes for correct resume

### Battle-group State Files (§9.6)
**Requirement text**: When active NPC battle-group state files exist for visited systems, the subsystem shall persist those state files.

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 270-310

### Files to create

- `rust/src/campaign/save/serialize.rs` — Campaign save serialization
  - marker: `@plan PLAN-20260314-CAMPAIGN.P06`
  - marker: `@requirement §9.1, §9.3`
  - `save_game(session: &CampaignSession, slot: u8) -> Result<(), SaveError>`
  - `serialize_game_state(session: &CampaignSession) -> Result<Vec<u8>, SaveError>`
  - `write_summary(writer: &mut impl Write, summary: &SaveSummary) -> Result<(), SaveError>`
  - `write_game_state(writer: &mut impl Write, state: &[u8]) -> Result<(), SaveError>`
  - `write_queue_data(writer: &mut impl Write, queue: &[QueueEntry]) -> Result<(), SaveError>`
  - `save_battle_group_files(session: &CampaignSession) -> Result<(), SaveError>`
  - Save-time adjustment logic:
    - Homeworld encounter screen: normalize `START_INTERPLANETARY` handling
    - Interplanetary re-entry: preserve entry state for correct resume
    - Starbase context: ensure `GLOBAL_FLAGS_AND_DATA` marker persisted
  - `SaveError` enum: `IoError`, `SerializationError`, `StateFileError`
  - Comprehensive tests:
    - Round-trip: serialize then deserialize produces equivalent state
    - Summary written correctly for all covered contexts
    - Queue data preserved
    - Save-time adjustments applied for special contexts
    - Error handling: I/O failures produce SaveError, not panic

### Files to modify

- `rust/src/campaign/save/mod.rs`
  - Add `pub mod serialize;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `save/serialize.rs` created with all serialization functions
- [ ] Module wired into `save/mod.rs`
- [ ] Error type defined

## Semantic Verification Checklist (Mandatory)
- [ ] All §9.1 fields are serialized: activity, clock, autopilot, location, ship state, orbit flags, bitfield
- [ ] Escort, NPC, and encounter queues serialized correctly
- [ ] Battle-group state files persisted for visited systems
- [ ] Save-time adjustments correctly handle homeworld encounter, interplanetary re-entry, starbase
- [ ] Round-trip test: save then load produces equivalent campaign-boundary observables
- [ ] Failed save does not report success or leave inconsistent state
- [ ] Tests verify behavior, not only internals

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/save/serialize.rs
```

## Success Criteria
- [ ] Save produces complete, loadable save files
- [ ] All covered contexts produce correct saves
- [ ] Error handling is comprehensive

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/save/serialize.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P06.md`
