# Phase 16: End-to-End Integration & Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P16`

## Prerequisites
- Required: Phase 15a completed
- Required: All previous phases completed and verified
- Required: Game builds and links with `USE_RUST_CAMPAIGN=1`

## Purpose

This is the final integration verification phase. It confirms that the entire campaign-gameplay subsystem works end-to-end with the Rust implementation active, covering gameplay flows, save/load round-trips, claim-family-specific verification surfaces, adjunct-sensitive pass/fail logic, cross-build evidence, and verifier-facing reporting obligations.

## Verification Matrix Requirement

Before scenario execution, produce a matrix at `project-plans/20260311/campaign-gameplay/verification/matrix.md` covering:
- each covered context from `../specification.md` §9.7,
- each claim/context family being evaluated,
- chosen inspection surface (`raw_save`, `canonical_export`, or `legacy_starbase_observation_exception` where specifically allowed),
- whether adjunct artifacts are required for overall covered-context conformance,
- the closed comparison object / equivalence scope used,
- expected claim-local result field and overall covered-context result field in the verifier report.

The verifier shall not mix raw-save and export facts within a single claim/context family, but different claim/context families from the same save may use different surfaces where allowed.

## Evidence Artifacts and Harness Inputs

Before end-to-end scenario execution, ensure these concrete evidence inputs/outputs exist and are populated:
- `rust/tests/fixtures/campaign/legacy/` — curated valid legacy save corpus for each covered legacy context under `../specification.md` §9.7
- `rust/tests/fixtures/campaign/corrupt/` — malformed/truncated/unknown-selector save corpus for mandatory rejection and safe-failure checks
- `rust/tests/fixtures/campaign/adjunct/` — adjunct-missing and adjunct-invalid artifact sets for context-indexed §9.4.0b failure coverage
- `project-plans/20260311/campaign-gameplay/verification/exports/` — canonical export snapshots captured during P16
- `project-plans/20260311/campaign-gameplay/verification/reports/` — machine-readable verifier reports emitted from P14/P16 evaluation
- `project-plans/20260311/campaign-gameplay/verification/evidence/manifest.md` — scenario-by-scenario artifact manifest recording save path, adjunct set, chosen surface, and comparison outcome

## End-to-End Verification Scenarios

### Scenario 1: New Game Start Flow
1. Launch game with `USE_RUST_CAMPAIGN=1`
2. Select "New Game"
3. Verify: introductory sequence plays (if defined)
4. Verify: campaign starts at Sol, Interplanetary mode
5. Verify: game clock begins at campaign start date (Feb 17 start year)
6. Verify: initial 4 events scheduled (HYPERSPACE_ENCOUNTER, ARILOU_ENTRANCE, KOHR_AH_VICTORIOUS, SLYLANDRO_RAMP_UP)

### Scenario 2: Hyperspace Navigation & Encounter
1. Navigate to hyperspace
2. Verify: clock rate changes to hyperspace pacing
3. Travel until encounter triggers
4. Verify: encounter identity matches collided group
5. Verify: communication/dialogue launches for correct race
6. If battle: verify ship queues correct, backdrop correct
7. After encounter: verify return to hyperspace at pre-encounter position

### Scenario 3: Interplanetary Entry
1. Navigate to a star system from hyperspace
2. Verify: transition to solar-system exploration
3. Verify: clock rate changes to interplanetary pacing
4. Verify: correct destination system selected

### Scenario 4: Starbase Visit
1. Return to Sol/starbase
2. Verify: starbase visit mode entered
3. Verify: Commander conversation, outfit, shipyard accessible
4. Verify: departure resumes interplanetary

### Scenario 5: Save/Load Round-Trip (Hyperspace)
1. Navigate to hyperspace coordinates (300, 400)
2. Save game to slot 1
3. Capture chosen inspection-surface evidence for the relevant claim families
4. Navigate elsewhere
5. Load slot 1
6. Verify: resume at hyperspace (300, 400)
7. Verify: fleet roster, progression flags match save-time state

### Scenario 6: Save/Load Round-Trip (Interplanetary)
1. Enter a star system
2. Save game
3. Capture primary save artifact and any required adjunct artifact set
4. Load game
5. Verify: resume into correct system
6. Verify: campaign-boundary interplanetary entry observables match

### Scenario 7: Save/Load Round-Trip (Starbase)
1. Visit starbase
2. Save game during starbase
3. Load game
4. Verify: resume at starbase at correct progression point
5. Verify: mandatory-next-action rule honored
6. Verify: no completed actions replayed, no pending actions skipped
7. If legacy raw save is insufficient for the closed comparison object, use the legacy-starbase observational exception path only for that claim family

### Scenario 8: Legacy Save Compatibility
1. Load curated legacy C-produced save fixtures from `rust/tests/fixtures/campaign/legacy/` for each covered context
2. Load each legacy save with Rust campaign active
3. Verify: each resumes in correct mode with semantically equivalent state
4. Verify: scheduled events fire at correct campaign dates

### Scenario 9: Campaign Event Progression
1. Start new game, advance time
2. Verify: ARILOU_ENTRANCE_EVENT opens portal at correct date
3. Verify: HYPERSPACE_ENCOUNTER_EVENT fires daily
4. Verify: SLYLANDRO_RAMP_UP increments probe multiplier
5. Verify any row-specific §8.6 normalization/checkpoint obligations via the chosen inspection surface

### Scenario 10: Deferred Transition Verification
1. Visit starbase, depart
2. Verify: deferred transition to interplanetary (no save mutation)
3. Verify: interplanetary entered on next loop iteration

### Scenario 11: Error Handling / Safe Failure
1. Attempt to load a corrupt save file from `rust/tests/fixtures/campaign/corrupt/`
2. Verify: load fails safely, no partial state
3. Verify: if from start flow, returns to start flow
4. Verify: if from in-session, pre-load session preserved
5. Verify: primary save artifact and documented adjunct artifacts are unchanged after failed load

### Scenario 12: Canonical Export / Inspection Surface
1. Save game in each covered context that requires export-based inspection
2. Run canonical export for each save
3. Write export snapshots under `project-plans/20260311/campaign-gameplay/verification/exports/`
4. Verify: valid JSON with all 8 sections
5. Verify: deterministic (run twice, compare output)
6. Verify: successful export is not reported as overall covered-context pass when adjunct-sensitive overall conformance still fails

### Scenario 13: Cross-Build Evidence
1. Build and run the C-only path with `USE_RUST_CAMPAIGN` disabled
2. Build and run the Rust path with `USE_RUST_CAMPAIGN=1`
3. Compare covered-context outcomes at the chosen inspection surfaces and player-visible checkpoints
4. Record any intentional end-state normalization differences versus regressions

## Verifier Report Output

For each evaluated save/context and claim/context family, record at minimum:
- claim/context family evaluated
- chosen inspection surface
- covered context
- comparison object / equivalence scope used
- claim-local result
- overall covered-context result
- whether adjunct dependency changed the overall result
- artifact set examined (primary save, adjunct files, export output, or controlled observation)
- notes on any legacy-starbase exception use

Write machine-readable reports to `project-plans/20260311/campaign-gameplay/verification/reports/` and cross-reference them from `project-plans/20260311/campaign-gameplay/verification/evidence/manifest.md`.

## Verification Commands

```bash
# Full quality gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Build verification
cd sc2 && USE_RUST_CAMPAIGN=1 ./build.sh uqm
cd sc2 && ./build.sh uqm

# Deferred implementation detection across entire campaign module
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/
```

## Structural Verification Checklist
- [ ] All campaign module files present and compiling
- [ ] No TODO/FIXME/HACK markers in any campaign source file
- [ ] All phase completion markers present (P00.5 through P15a)
- [ ] `lib.rs` exports `campaign` module
- [ ] Build succeeds with `USE_RUST_CAMPAIGN=1`
- [ ] Build succeeds with `USE_RUST_CAMPAIGN` disabled
- [ ] No `cargo clippy` warnings
- [ ] `cargo fmt` passes
- [ ] Context × claim-family × surface verification matrix produced at `project-plans/20260311/campaign-gameplay/verification/matrix.md`
- [ ] Verifier report artifacts produced under `project-plans/20260311/campaign-gameplay/verification/reports/`
- [ ] Evidence manifest produced at `project-plans/20260311/campaign-gameplay/verification/evidence/manifest.md`
- [ ] Fixture corpora for legacy, corrupt, and adjunct-sensitive scenarios are present and referenced by the manifest

## Semantic Verification Checklist (Mandatory)
- [ ] New game starts correctly with all initial state
- [ ] Campaign loop dispatches correctly for all activity modes
- [ ] Hyperspace encounters produce correct encounter identity
- [ ] Interplanetary transitions target correct systems
- [ ] Starbase visit flow handles all special sequences
- [ ] Save/load round-trips for all covered contexts
- [ ] Legacy saves load correctly
- [ ] Event progression fires at correct dates with correct effects
- [ ] Deferred transitions work without save-slot mutation
- [ ] Clock rate policy enforced per activity
- [ ] Error handling: corrupt saves rejected safely
- [ ] No-mutation checks cover primary save artifact and documented adjunct artifact set using artifact diffs or equivalent persistence-boundary evidence
- [ ] Canonical export produces valid documents where export is the chosen/required surface
- [ ] Claim-family inspection-surface selection follows the no-mixing rule
- [ ] Successful export is distinguished from overall covered-context conformance when adjunct artifacts matter
- [ ] Legacy-starbase observational exception used only in its allowed insufficiency case
- [ ] Load/export outcome classes are reported according to `../specification.md` §10.1 examples/rules
- [ ] No gameplay regressions vs C-only build

## Cross-Subsystem Integration Checklist
- [ ] Clock subsystem: init/uninit/rate-set/day-advance work via Rust campaign
- [ ] State subsystem: game-state bits read/write correctly
- [ ] Comm subsystem: dialogue dispatch works from Rust encounter flow
- [ ] Battle subsystem: Battle() invocation and result retrieval work
- [ ] IO subsystem: save/load file operations work
- [ ] Game init: kernel init/free work from Rust start flow
- [ ] Planet/solar-system: persistence infrastructure lifecycle correct
- [ ] Cross-boundary restore failures produce safe campaign-load failure rather than partial application

## Definition of Done (from 00-overview.md)
- [ ] All `cargo test --workspace --all-features` pass
- [ ] All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
- [ ] `cargo fmt --all --check` passes
- [ ] Game boots with `USE_RUST_CAMPAIGN=1` and campaign gameplay works
- [ ] New game starts at Sol/Interplanetary with correct date and initial events
- [ ] Save/load round-trips correctly for all covered contexts (§9.7)
- [ ] Legacy C saves load with semantic equivalence
- [ ] All 18 campaign event handlers produce correct observable effects
- [ ] Starbase save/load resume matches closed progression-point contract
- [ ] Deferred transitions work without save-slot mutation
- [ ] No placeholder stubs or TODO markers remain
- [ ] Build succeeds with toggle both ON and OFF
- [ ] Campaign Canonical Export Document produces valid JSON where export is required
- [ ] Claim-local and overall covered-context results are both reported where required

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P16.md`

Contents:
- phase ID: PLAN-20260314-CAMPAIGN.P16
- timestamp
- all verification command outputs
- context/claim-family/surface matrix
- verifier report outputs
- all scenario results
- overall PASS/FAIL decision
