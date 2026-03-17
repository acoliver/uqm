# Phase 07: Load Deserialization & Validation

## Phase ID
`PLAN-20260314-CAMPAIGN.P07`

## Prerequisites
- Required: Phase 06a completed
- Expected files: `save/serialize.rs`, campaign types, events module, `state_bridge.rs`
- Dependency: Clock subsystem for schedule restoration
- Dependency: Phase 05 frozen export/report contract in `save/export.rs` for malformed-save error/result shape, claim-family inspection-surface vocabulary, and verifier-report entry schema

## Requirements Implemented (Expanded)

### Campaign Load (from requirements.md)
**Requirement text**: When a save is loaded, the subsystem shall resume the campaign in the mode recoverable from the save, with fleet roster, active encounters, campaign progression flags, and navigation/location state equivalent within the applicable closed equivalence scope.

### Scheduled-Event Semantic Validation (§9.4.1)
**Requirement text**: Load shall fail if restored scheduled-event state contains an unknown event selector or structurally invalid event metadata.

Behavior contract:
- GIVEN: A save with event selector index 99 (not in catalog)
- WHEN: Load is attempted
- THEN: Load fails, campaign does not resume, safe-failure guarantees hold

### General Load-Failure Contract (§9.4.0b)
**Requirement text**: All campaign-owned load-state failures are subject to safe-failure guarantees: no partial state application, no persisted mutation, clean return to start flow or pre-load session.

Behavior contract:
- GIVEN: A corrupt save is loaded from start flow
- WHEN: Load fails at validation
- THEN: Control returns to start/load flow, no resumed campaign state from rejected save

- GIVEN: A corrupt save is loaded from within running campaign (hyperspace menu)
- WHEN: Load fails
- THEN: Pre-load campaign session remains active, no state from rejected save applied

### Adjunct Artifact Dependencies (§9.4.0b)
**Requirement text**: When a covered resume context requires a battle-group or per-system state file and that artifact is missing/invalid, the load shall fail.

### Starbase Resume (§6.6)
**Requirement text**: When a save representing starbase context is loaded, resume into starbase visit flow at the closed progression-point contract.

### Interplanetary Resume (§9.4)
**Requirement text**: When loaded state represents interplanetary without starbase, resume into solar-system exploration targeting the saved destination system.

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 320-385

### Files to create

- `rust/src/campaign/save/deserialize.rs` — Campaign load deserialization
  - marker: `@plan PLAN-20260314-CAMPAIGN.P07`
  - marker: `@requirement §9.4, §9.4.0b, §9.4.1`
  - `load_game(session: &mut CampaignSession, slot: u8) -> Result<(), LoadError>`
  - `deserialize_game_state(data: &[u8]) -> Result<GameStateBlob, LoadError>`
  - `read_summary(reader: &mut impl Read) -> Result<SaveSummary, LoadError>`
  - `read_queue_data(reader: &mut impl Read) -> Result<Vec<QueueEntry>, LoadError>`
  - `derive_resume_mode(session: &mut CampaignSession)` — determine campaign mode from loaded state
  - `normalize_interplanetary_resume(session: &mut CampaignSession)` — ensure START_INTERPLANETARY set
  - `normalize_starbase_resume(session: &mut CampaignSession)` — detect starbase context, setup resume
  - Pre-commit snapshot: save session state before attempting load for safe rollback using P03.5 bridge snapshot support
  - Commit-point logic: all validation must pass before session state or bridge-backed state is mutated
  - `LoadError` enum: `IoError`, `ParseError`, `StructuralCorruption`, `UnknownEventSelector(u8)`, `InvalidEventMetadata`, `MissingAdjunctArtifact(String)`, `AdjunctArtifactInvalid(String)`
  - Error/result classification for malformed saves must reuse the frozen Phase 05 export/error contract instead of inventing a later incompatible shape

- `rust/src/campaign/save/validation.rs` — Semantic validation for loaded state
  - marker: `@plan PLAN-20260314-CAMPAIGN.P07`
  - marker: `@requirement §9.4.0b, §9.4.1`
  - `validate_scheduled_events(events: &[ScheduledEvent]) -> Result<(), LoadError>`
    - Check 1: every selector is in the campaign event catalog (0-17)
    - Check 2: every event entry has valid date encoding (no out-of-range month/day/year)
    - Check 3: every event entry has parseable metadata
  - `validate_adjunct_artifacts(context: &ResumeContext, classifier: &AdjunctDependencyClassifier) -> Result<(), LoadError>`
    - Use the context-indexed rule set from `../specification.md` / `../requirements.md` rather than broad activity heuristics
    - Hyperspace/quasispace: no adjunct required
    - Interplanetary: per-system state files if context-indexed rule says required
    - Starbase: no adjunct required for progression-point
    - Encounter-entry: battle-group state files if context-indexed rule says required
    - Post-encounter: battle-group and/or per-system if context-indexed rule says required
    - Final battle: no adjunct required
  - `safe_failure_handler(session: &mut CampaignSession, snapshot: CampaignStateSnapshot, from_start_flow: bool)`
    - Restore session/bridge-backed state to pre-load snapshot if in-session load
    - Return to start flow if from start flow
    - Guarantee: no save-slot mutation, no persisted state mutation, no adjunct mutation
  - Comprehensive tests:
    - Unknown selector rejection
    - Invalid date encoding rejection
    - Missing adjunct artifact rejection
    - Safe-failure: session state unchanged after failed in-session load
    - Safe-failure: no partial state application
    - Correct resume mode derivation for all covered contexts
    - Starbase resume detects starbase-context marker
    - Interplanetary resume sets START_INTERPLANETARY correctly
    - No mutation of primary save artifact or documented adjunct artifacts on failed load using fixture-based persistence diff evidence rather than in-memory assumptions alone
    - Malformed-save rejection evidence remains compatible with the Phase 05 export/report contract so later verifier/report code does not need to reinterpret load failures

### Files to modify

- `rust/src/campaign/save/mod.rs`
  - Add `pub mod deserialize;`
  - Add `pub mod validation;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `save/deserialize.rs` and `save/validation.rs` created
- [ ] LoadError enum covers all failure modes
- [ ] Validation functions exist for both §9.4.1 rejection cases and §9.4.0b adjunct-sensitive safe-failure handling
- [ ] Fixture-based persistence diff or equivalent artifact-observation coverage exists for failed-load no-mutation checks
- [ ] Phase 07 explicitly consumes the frozen Phase 05 malformed-save/export/report contract rather than redefining it

## Semantic Verification Checklist (Mandatory)
- [ ] Unknown event selector (index 18+) causes mandatory load rejection
- [ ] Malformed date encoding causes mandatory load rejection
- [ ] Rejected load leaves no partial state from the save
- [ ] In-session load failure preserves pre-load session
- [ ] Start-flow load failure returns to start flow
- [ ] No save-slot mutation on failed load based on persistence-boundary evidence
- [ ] No documented adjunct artifact mutation on failed load based on persistence-boundary evidence
- [ ] Interplanetary resume correctly targets saved destination system
- [ ] Starbase resume detects starbase context from any valid encoding
- [ ] Adjunct artifact failures cause load rejection per the context-indexed §9.4.0b rule set
- [ ] All covered contexts from §9.7 produce correct resume modes
- [ ] Malformed-save/load-failure classification is compatible with the earlier frozen export/report contract

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/save/deserialize.rs rust/src/campaign/save/validation.rs
```

## Success Criteria
- [ ] Load correctly restores all campaign-boundary observables
- [ ] Validation rejects invalid saves per §9.4.1
- [ ] Safe-failure guarantees hold in all error paths
- [ ] Resume mode correctly derived for all covered contexts
- [ ] No Phase 07 behavior requires later redefinition of export/report/verifier contract

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/save/deserialize.rs rust/src/campaign/save/validation.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P07.md`
