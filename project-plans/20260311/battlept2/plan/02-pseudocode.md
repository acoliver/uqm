# Phase 02: Pseudocode

## Phase ID
`PLAN-20260320-BATTLEPT2.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed with PASS
- Expected artifact: `project-plans/20260311/battlept2/analysis/domain-model.md`

## Requirements Implemented (Expanded)

This phase produces no code. It produces algorithmic pseudocode for all 64 ported functions, organized into 6 component files. Each pseudocode line is numbered for traceability from implementation phases.

### REQ: Pseudocode completeness
**Requirement text**: Every ported C function must have algorithmic pseudocode that captures validation points, error handling, ordering constraints, integration boundaries, and side effects.

Behavior contract:
- GIVEN: The 64 ported functions from spec §12.1
- WHEN: The pseudocode files are produced
- THEN: Every function has numbered pseudocode matching the C reference behavior

### REQ: Pseudocode accuracy
**Requirement text**: Pseudocode must match C reference behavior exactly, including branch-family handling, flag transitions, and callback ordering.

Behavior contract:
- GIVEN: The C source files (process.c, battle.c, tactrans.c, intel.c, ship.c, init.c)
- WHEN: Pseudocode is verified against C
- THEN: Every conditional branch, loop, callback dispatch, and state transition in the pseudocode corresponds to the C reference

## Implementation Tasks

### Files to create

- `project-plans/20260311/battlept2/analysis/pseudocode/process-loop.md` — Process loop functions
  - marker: `@plan PLAN-20260320-BATTLEPT2.P02`
  - Functions: PreProcess, PostProcess, AllocElement, FreeElement, SetUpElement, InsertPrim, Untarget, RemoveElement, PreProcessQueue, PostProcessQueue, RedrawQueue, InitDisplayList, CalcDisplayCoord, InitKernel
  - Must include: flag transition tables, velocity stepping algorithm, APPEARING special cases, DEFY_PHYSICS asymmetric clearing, newly-added element cascading

- `project-plans/20260311/battlept2/analysis/pseudocode/process-collisions.md` — ProcessCollisions
  - marker: `@plan PLAN-20260320-BATTLEPT2.P02`
  - Functions: ProcessCollisions (recursive)
  - Must include: recursive structure, dispatch ordering (PLAYER_SHIP priority), stuck-overlap handling, position snapping, post-bounce rescans, COLLISION flag as re-entry guard

- `project-plans/20260311/battlept2/analysis/pseudocode/zoom-camera.md` — Zoom and camera
  - marker: `@plan PLAN-20260320-BATTLEPT2.P02`
  - Functions: CalcReduction (step + continuous), CalcView (CALC_ZOOM_STUFF)
  - Must include: 3-level hysteresis for step mode, smooth interpolation for continuous mode, camera midpoint, single-ship clamping, VIEW_STABLE/VIEW_SCROLL/VIEW_CHANGE states

- `project-plans/20260311/battlept2/analysis/pseudocode/ship-runtime.md` — Ship runtime
  - marker: `@plan PLAN-20260320-BATTLEPT2.P02`
  - Functions: ship_preprocess (7-stage pipeline), ship_postprocess, inertial_thrust, spawn_ship, GetNextStarShip, GetInitialStarShips, animation_preprocess, collision(ship)
  - Must include: pipeline stage ordering, APPEARING first-frame handling, energy regen, turn/thrust mechanics, inertial physics (inertialess/normal/gravity/at-max-speed), weapon firing sequence, Sa-Matra center placement

- `project-plans/20260311/battlept2/analysis/pseudocode/tactical-transitions.md` — Tactical transitions
  - marker: `@plan PLAN-20260320-BATTLEPT2.P02`
  - Functions: All 25 ported tactrans.c functions (17 P09 + 8 P10)
  - Must include: ship_death→explosion→cleanup→new_ship callback chain, 36-frame explosion, debris spawning, cleanup crew preservation, victory ditty, ion trail 12-color fade, flee 20-color pulse, warp-in/out 15-frame ghost images, winner determination display-list order dependency, Pkunk reincarnation, OpponentAlive 3-return-case semantics

- `project-plans/20260311/battlept2/analysis/pseudocode/battle-lifecycle.md` — Battle lifecycle + AI
  - marker: `@plan PLAN-20260320-BATTLEPT2.P02`
  - Functions: Battle(), InitShips, UninitShips, InitSpace/UninitSpace, ProcessInput, CountCrewElements, RunAwayAllowed, setupBattleInputOrder, BattleSong, FreeBattleSong, selectAllShips, GetPlayerOrder, computer_intelligence
  - Must include: Battle() full flow (seed→song→init→loop→cleanup), reference-counted space assets, crew writeback, input bit mapping, escape detection, 4-path AI dispatch, SUPER_MELEE/encounter/final-battle branches

### Pseudocode format requirements
Each function's pseudocode must be:
- Numbered line-by-line for traceability
- Algorithmic (not prose)
- Include validation points, error handling, ordering constraints
- Mark FFI calls explicitly (e.g., `FFI: DrawablesIntersect(...)`)
- Reference Phase 1 types/functions by name (e.g., `Phase1: elastic_collide(...)`)
- Mark branch-family conditionals (e.g., `BRANCH: NETPLAY { ... }`)

### Pseudocode traceability
- N/A (this IS the pseudocode phase)

## Verification Commands

```bash
# No code changes — verify Phase 1 still passes
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 6 pseudocode files created
- [ ] All 64 ported functions have pseudocode
- [ ] All pseudocode lines are numbered
- [ ] FFI calls are marked
- [ ] Phase 1 type references are marked
- [ ] Branch-family conditionals are marked

## Semantic Verification Checklist (Mandatory)
- [ ] Pseudocode matches C reference behavior for every function
- [ ] ProcessCollisions recursion correctly captured with all edge cases
- [ ] Flag transitions match spec §4.2 / requirements §Element lifecycle flag transitions
- [ ] Callback replacement chains correctly sequenced (ship_death→explosion→cleanup→new_ship)
- [ ] explosion_preprocess notes animation_preprocess dependency (tactrans.c:606 → ship.c:46)
- [ ] All branch-family conditionals present in functions that have them
- [ ] Integration points (FFI calls) are explicitly marked

## Deferred Implementation Detection (Mandatory)

```bash
# No implementation code in this phase
git diff --name-only HEAD | grep -v 'project-plans/'
# Should produce no output
```

## Success Criteria
- [ ] All 6 pseudocode files complete
- [ ] All 64 functions covered
- [ ] Phase 1 tests still pass

## Failure Recovery
- rollback: `git checkout -- project-plans/20260311/battlept2/analysis/pseudocode/`
- blocking issues: C behavior ambiguity requiring clarification

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P02.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P02
- timestamp
- files created: 6 pseudocode files
- verification outputs
- semantic verification summary
