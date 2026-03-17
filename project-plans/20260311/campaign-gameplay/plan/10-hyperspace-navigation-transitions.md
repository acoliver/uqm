# Phase 10: Hyperspace & Navigation Transitions

## Phase ID
`PLAN-20260314-CAMPAIGN.P10`

## Prerequisites
- Required: Phase 09a completed
- Expected files: `loop_dispatch.rs`, campaign types, `state_bridge.rs`
- Dependency: Clock subsystem for `SetGameClockRate`

## Requirements Implemented (Expanded)

### Hyperspace-to-Encounter Transition (§7.3, from requirements.md)
**Requirement text**: When the player collides with an NPC group in hyperspace, the resulting encounter identity shall correspond to the collided NPC group, and navigation context shall be preserved for post-encounter resume.

Behavior contract:
- GIVEN: Player collides with Ilwrath group in hyperspace at coords (500, 400)
- WHEN: Encounter transition triggers
- THEN: Encounter identity is Ilwrath, pre-encounter hyperspace state saved, `start_encounter` set

### Hyperspace-to-Interplanetary Transition (§7.4)
**Requirement text**: When the player enters a solar system from hyperspace, transition to solar-system exploration targeting the destination. Special-case Arilou homeworld routes to encounter.

Behavior contract:
- GIVEN: Player enters Sol system
- WHEN: Interplanetary transition triggers
- THEN: `start_interplanetary` set, destination = Sol, orbit context cleared

- GIVEN: Player enters Arilou homeworld space
- WHEN: Transition triggers
- THEN: Routes to encounter, NOT interplanetary

### Quasispace Transition (§7.5)
**Requirement text**: Handle transitions between hyperspace and quasispace as campaign-layer navigation events.

### Clock Rate Policy (§8.4)
**Requirement text**: Hyperspace uses hyperspace pacing rate; interplanetary uses interplanetary pacing rate.

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 100-125

### Files to create

- `rust/src/campaign/transitions.rs` — Navigation transition logic
  - marker: `@plan PLAN-20260314-CAMPAIGN.P10`
  - marker: `@requirement §7.3, §7.4, §7.5, §8.4`
  - `handle_hyperspace_encounter(session: &mut CampaignSession, collided_group: &QueueEntry)` or a validated bridge-backed group representation from P03.5
    - Save hyperspace navigation context (coords, ship-state token)
    - Reorder encounter queue so collided group is first
    - Set `start_encounter` flag
    - Verify encounter identity matches collided group
  - `handle_interplanetary_transition(session: &mut CampaignSession, target_system: SystemId)`
    - Clear orbit context
    - Reset broadcaster state if campaign-owned at this seam
    - Check for Arilou homeworld: if so, route to encounter instead
    - Otherwise: set `start_interplanetary`, set destination system
  - `handle_quasispace_transition(session: &mut CampaignSession, direction: QuasispaceDirection)`
    - `QuasispaceDirection` enum: `ToQuasispace`, `ToHyperspace(portal_exit: HyperspaceCoords)`
    - ToQuasispace: save hyperspace coords, set `in_quasispace = true`
    - ToHyperspace: restore coords from portal exit, set `in_quasispace = false`
  - `restore_hyperspace_context(session: &mut CampaignSession)`
    - Called after encounter resolution to restore pre-encounter navigation
  - `set_activity_clock_rate(session: &CampaignSession)`
    - Match activity to clock rate:
    - Hyperspace/Quasispace: `HYPERSPACE_CLOCK_RATE`
    - Interplanetary: `INTERPLANETARY_CLOCK_RATE`
    - Call `SetGameClockRate()` via clock API
  - Constants: `HYPERSPACE_CLOCK_RATE`, `INTERPLANETARY_CLOCK_RATE`
  - `ARILOU_HOMEWORLD_SYSTEM: SystemId` constant for special-case detection
  - Comprehensive tests:
    - Hyperspace encounter saves navigation context correctly
    - Encounter queue reordered so collided group is first
    - Encounter identity derived from collided group
    - Interplanetary transition sets correct flags and destination
    - Arilou homeworld routes to encounter, not interplanetary
    - Quasispace to-quasispace saves coords
    - Quasispace to-hyperspace restores from portal exit
    - Clock rate set correctly for each activity type
    - Post-encounter hyperspace context correctly restored

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod transitions;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `transitions.rs` created with all transition functions
- [ ] Module wired into `campaign/mod.rs`
- [ ] No speculative type names remain without validation provenance from P01/P03.5

## Semantic Verification Checklist (Mandatory)
- [ ] Hyperspace encounter preserves navigation context for post-encounter resume
- [ ] Encounter identity matches collided NPC group
- [ ] Encounter queue correctly reordered
- [ ] Interplanetary transition targets correct system
- [ ] Arilou homeworld special case routes to encounter
- [ ] Quasispace transitions preserve/restore coordinates correctly
- [ ] Clock rate policy matches §8.4 requirements
- [ ] Post-encounter hyperspace restoration works

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/transitions.rs
```

## Success Criteria
- [ ] All transition types produce correct state changes
- [ ] Special cases handled (Arilou homeworld, quasispace portals)
- [ ] Clock rate correctly set per activity

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/transitions.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P10.md`
