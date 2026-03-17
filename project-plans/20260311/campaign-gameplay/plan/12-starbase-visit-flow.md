# Phase 12: Starbase Visit Flow

## Phase ID
`PLAN-20260314-CAMPAIGN.P12`

## Prerequisites
- Required: Phase 11a completed
- Expected files: encounter, transitions, loop dispatch modules
- Dependency: Comm subsystem for Commander conversation (via FFI)
- Dependency: Clock subsystem for `MoveGameClockDays(14)`
- Dependency: validated callable seam inventory and ownership notes from P01/P03.5 for starbase routing, conversation entry, and departure handoff

## Requirements Implemented (Expanded)

### Starbase Visit Flow (§6.4, from requirements.md)
**Requirement text**: When the player visits the allied starbase, enter starbase visit mode with special-case handling for mandatory special-sequence categories: bomb-transport and pre-alliance Ilwrath-response.

### Bomb-Transport Special Sequence (§6.4)
Behavior contract:
- GIVEN: Player arrives at starbase with bomb device
- WHEN: `visit_starbase()` called
- THEN: Bomb-transport sequence gates subsequent flow until completed

### Pre-Alliance Ilwrath-Response Sequence (§6.4)
Behavior contract:
- GIVEN: Starbase not yet allied, Ilwrath response conditions met
- WHEN: `visit_starbase()` called
- THEN: Commander conversation, conditional Ilwrath battle staged, return to conversation after battle

### Forced Conversations (§6.4)
Behavior contract:
- GIVEN: First starbase availability or post-bomb-installation
- WHEN: Starbase entered
- THEN: 14-day time skip and forced Commander conversation before normal menu

### Starbase Menu (from requirements.md)
**Requirement text**: Menu supports Commander, outfit, shipyard. Exits on load, abort, or departure.

### Starbase Departure (§6.5)
**Requirement text**: On departure, resume campaign navigation in interplanetary via deferred transition.

### Starbase Save/Load Resume (§6.6, from requirements.md)
**Requirement text**: Save preserves starbase progression point. Load resumes at closed progression-point contract with mandatory-next-action rule.

Behavior contract:
- GIVEN: Save during starbase with forced conversation pending
- WHEN: Loaded
- THEN: Forced conversation surfaces before any non-mandatory interaction

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 180-215

### Files to create

- `rust/src/campaign/starbase.rs` — Starbase visit flow
  - marker: `@plan PLAN-20260314-CAMPAIGN.P12`
  - marker: `@requirement §6.4, §6.5, §6.6`
  - Any FFI routing or runtime ownership assumptions in this phase must follow the validated callable seam inventory from P01/P03.5 rather than inventing a new starbase ownership split here
  - `StarbaseProgressionPoint` struct:
    - `forced_conversation_pending: bool` (first-availability or post-bomb-installation)
    - `forced_conversation_completed: bool`
    - `bomb_transport_status: BombTransportStatus`
    - `pre_alliance_ilwrath_status: IlwrathResponseStatus`
    - `normal_menu_accessible: bool`
    - `departure_available: bool`
    - `mandatory_next_action: MandatoryAction`
  - `BombTransportStatus` enum: `NotApplicable`, `Pending`, `InProgress`, `Completed`
  - `IlwrathResponseStatus` enum: `NotApplicable`, `Pending`, `BattleActive`, `Completed`
  - `MandatoryAction` enum: `ForcedConversation`, `BombTransport`, `IlwrathBattle`, `NormalMenu`, `None`
  - `visit_starbase(session: &mut CampaignSession) -> Result<(), CampaignError>`
    - Set starbase_context marker
    - Check bomb-transport case: handle specially if applicable
    - If not allied:
      - Run Commander conversation via validated FFI seam
      - If Ilwrath response conditions: stage Ilwrath ship, run battle, return to conversation
      - Return
    - If first-availability or post-bomb-installation:
      - `MoveGameClockDays(14)` via clock API
      - Run forced Commander conversation
    - Enter `do_starbase_menu()`
  - `do_starbase_menu(session: &mut CampaignSession) -> Result<(), CampaignError>`
    - Loop:
      - Check load/abort -> break
      - Present menu choices: Commander, Outfit, Shipyard, Depart
      - Dispatch to appropriate sub-screen via validated FFI seam
      - On Depart: break
    - Clear `STARBASE_VISITED` flag
    - Request deferred transition to Interplanetary with START_INTERPLANETARY
  - `derive_starbase_progression_point(session: &CampaignSession) -> StarbaseProgressionPoint`
    - Examine game-state bits to determine:
      - Whether forced conversation is pending/completed
      - Bomb-transport status
      - Ilwrath-response status
      - Whether normal menu is next available
    - Apply mandatory-next-action rule: which mandatory route surfaces first under zero optional input
  - `resume_starbase_from_save(session: &mut CampaignSession) -> Result<(), CampaignError>`
    - Derive progression point from loaded state
    - Route to correct entry point:
      - If forced conversation pending: run it first
      - If bomb-transport pending: run that sequence
      - If Ilwrath battle pending: stage and run
      - Otherwise: normal menu
    - Guarantee: no completed mandatory action replays
    - Guarantee: no pending mandatory action skipped
    - Guarantee: no non-mandatory interaction before pending mandatory action
  - `cleanup_after_starbase(session: &mut CampaignSession)`
    - Clear starbase context marker
    - Set deferred transition to interplanetary
  - Comprehensive tests:
    - Normal allied starbase visit: menu accessible, departure works
    - First availability: 14-day skip, forced conversation before menu
    - Post-bomb-installation: 14-day skip, forced conversation before menu
    - Bomb-transport: special sequence gates normal flow
    - Pre-alliance: Commander conversation, conditional Ilwrath battle
    - Ilwrath battle: return to conversation after resolution
    - Departure: deferred transition to interplanetary
    - Load/abort: clean exit from menu
    - Save/load resume: correct progression point restored using scenario or persistence-boundary evidence at the closed comparison object, not only internal unit assertions
    - No completed actions replay after load
    - No pending actions skipped after load
    - No non-mandatory before mandatory after load
    - Mandatory-next-action rule: correct priority when multiple mandatory routes latent

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod starbase;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `starbase.rs` created with all starbase functions
- [ ] Module wired into `campaign/mod.rs`
- [ ] Progression-point types defined
- [ ] All callable seams used by this phase are traceable to validated P01/P03.5 inventory rows

## Semantic Verification Checklist (Mandatory)
- [ ] Bomb-transport special sequence correctly gates flow
- [ ] Pre-alliance Ilwrath response correctly staged
- [ ] Forced conversation fires before normal menu when applicable
- [ ] 14-day time skip occurs at correct points
- [ ] Menu supports Commander/outfit/shipyard/depart
- [ ] Departure uses deferred transition (no direct activity entry)
- [ ] Save/load resume matches closed progression-point contract based on player-visible or persistence-boundary evidence at the required observation point
- [ ] No mandatory action replayed after load
- [ ] No mandatory action skipped after load
- [ ] Mandatory-next-action rule applied correctly
- [ ] No non-mandatory interaction available before pending mandatory

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/starbase.rs
```

## Success Criteria
- [ ] All starbase special sequences work correctly
- [ ] Save/load resume at correct progression point
- [ ] Departure resumes campaign correctly

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/starbase.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P12.md`
