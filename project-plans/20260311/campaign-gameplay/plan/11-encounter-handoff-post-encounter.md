# Phase 11: Encounter Handoff & Post-Encounter Processing

## Phase ID
`PLAN-20260314-CAMPAIGN.P11`

## Prerequisites
- Required: Phase 10a completed
- Expected files: transitions, loop dispatch, campaign types
- Dependency: Comm subsystem for encounter dialogue dispatch (via FFI)
- Dependency: Battle subsystem for `Battle()` invocation (via FFI)
- Dependency: validated callable seam inventory and ownership notes from P01/P03.5 for comm dispatch, battle invocation, and encounter-state cleanup boundaries

## Requirements Implemented (Expanded)

### Encounter Dispatch (§6.1, from requirements.md)
**Requirement text**: When an encounter is triggered, identify the encountered race from the current encounter context and dispatch to the appropriate communication/dialogue entry point.

### Battle Segue (§6.2, from requirements.md)
**Requirement text**: When an encounter leads to combat, provide the battle subsystem with ship identities, fleet composition, and backdrop, then invoke battle.

Behavior contract:
- GIVEN: Encounter with Ilwrath, battle segue set
- WHEN: `encounter_battle()` is called
- THEN: Battle invoked with Ilwrath ships in NPC queue, hyperspace backdrop, SIS flagship injected

### Post-Encounter Processing (§6.3, from requirements.md)
**Requirement text**: After normal encounter resolution, determine outcome, identify race, apply consequences (salvage, flag updates, queue cleanup).

Behavior contract:
- GIVEN: Player wins encounter battle against 3 Ilwrath ships
- WHEN: `uninit_encounter()` is called
- THEN: Defeated Ilwrath ships removed from encounter queue, progression flags updated, salvage awarded

### Suppress Processing (§6.3)
**Requirement text**: Suppress post-encounter processing when exit is due to abort, load, death, or final-battle.

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 130-170

### Files to create

- `rust/src/campaign/encounter.rs` — Encounter handoff and post-encounter processing
  - marker: `@plan PLAN-20260314-CAMPAIGN.P11`
  - marker: `@requirement §6.1, §6.2, §6.3`
  - Battle-facing setup/result structures in this phase remain constrained by the validated seam inventory from P01/P03.5; exact FFI payload layout must follow the proved callable contract rather than forcing a speculative replacement shape
  - `BattleSetup` struct:
    - `npc_ships: Vec<QueueEntry>`
    - `player_ships: Vec<QueueEntry>`
    - `backdrop: BattleBackdrop`
    - `is_last_battle: bool`
  - `BattleBackdrop` enum: `SaMatra`, `Hyperspace`, `Planetary`, `Default`
  - `BattleResult` struct:
    - `player_ships_lost: u16`
    - `npc_ships_destroyed: u16`
    - `player_retreated: bool`
    - `player_dead: bool`
  - `build_battle(session: &CampaignSession) -> BattleSetup`
    - Convert NPC ship queue fragments to race queue entries
    - Select backdrop: LastBattle -> SaMatra, Hyperspace -> Hyperspace, else Planetary
    - Inject SIS flagship into player queue
  - `encounter_battle(session: &mut CampaignSession) -> Result<BattleResult, CampaignError>`
    - Save previous activity
    - Set battle_segue, switch to InEncounter (or InLastBattle)
    - Zero battle counters
    - Invoke battle via seam-inventory-backed FFI path
    - Restore previous activity
    - Return result
  - `identify_encounter_race(session: &CampaignSession) -> Option<EncounterIdentity>`
    - Scan NPC ship queue for race identity
    - Map to EncounterIdentity enum
  - `uninit_encounter(session: &mut CampaignSession, result: &BattleResult)`
    - Check suppress conditions: abort, load_requested, defeat, is_last_battle -> return early
    - Clear battle_segue flag
    - Clear bomb_carrier flag
    - Determine victory state from battle counters + story flags
    - Identify encountered race
    - Apply consequences based on outcome:
      - Victory: remove defeated NPC ships from encounter queue, award salvage, update flags
      - Escape: minimal cleanup
      - Defeat: remove destroyed escort ships from escort queue
    - Clean up encounter state for navigation resume
  - `dispatch_encounter(session: &mut CampaignSession) -> Result<(), CampaignError>`
    - Check if starbase context -> route to starbase (delegates to Phase 12)
    - Otherwise: identify race, call communication/dialogue via validated seam-inventory-backed FFI path
    - If battle_segue after dialogue: call encounter_battle
    - Call uninit_encounter with result
  - Comprehensive tests:
    - build_battle produces correct ship queues for various encounters
    - Backdrop selection matches activity type
    - SIS flagship injected into player queue
    - encounter_battle saves/restores previous activity
    - Battle counters correctly tracked
    - uninit_encounter suppresses on abort/load/death/last-battle
    - Victory: defeated NPC ships removed, salvage awarded
    - Escape: minimal cleanup, encounter state cleaned
    - Race identification from NPC queue
    - Starbase context routed separately from normal encounters
    - FFI smoke/integration coverage proving the chosen comm and battle seams are callable with the validated ownership contract

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod encounter;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `encounter.rs` created with all encounter functions
- [ ] Module wired into `campaign/mod.rs`
- [ ] Battle and comm FFI call paths are defined only through validated callable seams

## Semantic Verification Checklist (Mandatory)
- [ ] build_battle produces correct NPC ships, backdrop, and player flagship
- [ ] encounter_battle saves/restores activity correctly
- [ ] uninit_encounter suppresses for abort, load, death, last-battle
- [ ] Victory processing removes defeated ships and awards salvage
- [ ] Encounter identity correctly identified from NPC queue
- [ ] Starbase context routes differently from normal encounters
- [ ] Clean encounter state after normal resolution enables navigation resume
- [ ] Error handling for FFI failures
- [ ] Integration coverage demonstrates the validated comm/battle seam contract actually works, not just unit-level local logic

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/encounter.rs
```

## Success Criteria
- [ ] Complete encounter lifecycle: dispatch -> dialogue -> battle -> consequences -> cleanup
- [ ] All outcome paths tested (victory, escape, defeat, suppress)
- [ ] Campaign resumes correctly after encounter

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/encounter.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P11.md`
