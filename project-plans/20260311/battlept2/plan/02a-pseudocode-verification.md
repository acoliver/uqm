# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed
- Expected artifacts: 6 pseudocode files in `project-plans/20260311/battlept2/analysis/pseudocode/`

## Structural Verification Checklist
- [ ] All 6 pseudocode files exist:
  - `process-loop.md` (14 functions)
  - `process-collisions.md` (1 function)
  - `zoom-camera.md` (2 functions)
  - `ship-runtime.md` (8 functions)
  - `tactical-transitions.md` (25 functions)
  - `battle-lifecycle.md` (13 functions + computer_intelligence)
- [ ] Every ported function (64 total) has numbered pseudocode
- [ ] Line numbers are sequential and non-overlapping within each file
- [ ] FFI calls explicitly marked with `FFI:` prefix
- [ ] Phase 1 type references marked with `Phase1:` prefix
- [ ] Branch-family conditionals marked with `BRANCH:` prefix

## Semantic Verification Checklist (Mandatory — Most Important)

### Process Loop (process-loop.md)
- [ ] **PreProcess** matches process.c:180-270 behavior exactly:
  - life_span==0 → Untarget + DISAPPEARING + death_func
  - APPEARING + PLAYER_SHIP → clear APPEARING in local copy only, still call preprocess_func
  - APPEARING + non-PLAYER_SHIP → skip preprocess_func, only init intersect
  - CHANGING + collidable → reinit intersect frame
  - !IGNORE_VELOCITY → apply velocity via Phase1: get_next_components()
  - Collidable → init intersect end point
  - FINITE_LIFE → decrement life_span
  - Set PRE_PROCESS, clear POST_PROCESS + COLLISION
- [ ] **PostProcess** matches process.c:274-285 behavior:
  - call postprocess_func
  - copy next → current (Phase1: commit_state)
  - reinit intersect points
- [ ] **AllocElement/FreeElement** match process.c:143-177
- [ ] **SetUpElement** matches process.c:156-177
- [ ] **Untarget** matches process.c:93-115 (clear hTarget refs)
- [ ] **RemoveElement** matches process.c:118-140 (remove sound + Untarget + remove from queue)

### ProcessCollisions (process-collisions.md)
- [ ] Recursive structure matches process.c:362-628 exactly
- [ ] Dispatch ordering: PLAYER_SHIP test element → test first, else current first
- [ ] Stuck overlap: max time + same frame → APPEARING killed, else revert position
- [ ] Position snapping: next.location = collision point
- [ ] Post-bounce: elastic_collide → re-call ProcessCollisions from head for both elements
- [ ] Recursive earlier-time: check both elements against earlier list before dispatching
- [ ] COLLISION flag as re-entry guard correctly used
- [ ] PreProcess called on unprocessed elements during successor walk

### Zoom/Camera (zoom-camera.md)
- [ ] CalcReduction step mode: 3 discrete levels with hysteresis thresholds
- [ ] CalcReduction continuous mode: smooth interpolation with MAX_ZOOM_OUT clamping
- [ ] CalcView: midpoint camera, ORG_JUMP clamping, VIEW_STABLE/VIEW_SCROLL/VIEW_CHANGE

### Ship Runtime (ship-runtime.md)
- [ ] ship_preprocess 7-stage pipeline: input → APPEARING → energy → race preprocess → turn → thrust → status
- [ ] APPEARING first-frame: suppress inputs, init crew, init status, race preprocess, ship_transition, early return
- [ ] inertial_thrust: inertialess (instant max), normal, gravity well, at-max-speed half-thrust
- [ ] ship_postprocess: weapon firing sequence (counter → energy → init_weapon → bind → sound → wait)
- [ ] spawn_ship: element init, Sa-Matra center, random position avoiding gravity, element reuse
- [ ] animation_preprocess: turn_wait decrement, frame advance, CHANGING flag

### Tactical Transitions (tactical-transitions.md)
- [ ] ship_death→StartShipExplosion→explosion_preprocess→cleanup_dead_ship→new_ship chain
- [ ] explosion_preprocess: 36 frames, 1-3 debris per frame, frame 15 hide, frame 25 clear preprocess
- [ ] cleanup_dead_ship: crew preservation (CREW_OBJECT), ditty, death_func=new_ship, life_span=MIN_DITTY_FRAME_COUNT
- [ ] new_ship: readyForBattleEnd(), StopDitty, free_ship, GetNextStarShip, BattleSong
- [ ] readyForBattleEnd: NETPLAY vs DEMO_MODE vs default branches
- [ ] checkOtherShipLifeSpan: winner stays alive one frame longer
- [ ] Ion trail: 12-color table, POINT_PRIM, head-insert, PRE_PROCESS set, life_span pre-decremented
- [ ] ship_transition: 15-frame ghost images, warp-in materialization, warp-out departure
- [ ] flee_preprocess: 20-color pulse, accelerating timing, warp-out trigger at midpoint
- [ ] FindAliveStarShip: display-list iteration, Pkunk mass=11 reincarnation
- [ ] SetWinnerStarShip: first-winner-only recording, PLAY_VICTORY_DITTY always set
- [ ] OpponentAlive: 3 return cases (false if found dead, true otherwise)

### Battle Lifecycle (battle-lifecycle.md)
- [ ] Battle(): seed → BattleSong → InitShips → instantVictory → selectAllShips → DoInput → cleanup
- [ ] InitShips: InitSpace → contexts → InitDisplayList → environment spawning
- [ ] UninitShips: StopSound → UninitSpace → CountCrew → iterate → survivor → crew add → free_ship
- [ ] ProcessInput: bit mapping, BATTLE_ESCAPE → DoRunAway()
- [ ] computer_intelligence: 4 paths (IN_LAST_BATTLE→0, CYBORG→tactical+escape, PSYTRON→sleep+weapon, non-cyborg→human)
- [ ] Branch families present: NETPLAY, DEMO_MODE, SUPER_MELEE, CHECK_ABORT, IN_ENCOUNTER, IN_LAST_BATTLE, inHyperSpace, max-speed

## Branch-Parity Verification
- [ ] `NETPLAY` branches present in: readyForBattleEnd, new_ship, Battle, DoBattle
- [ ] `DEMO_MODE` branches present in: readyForBattleEnd
- [ ] `SUPER_MELEE` branches present in: RecordShipDeath, GetInitialStarShips, Battle cleanup
- [ ] `CHECK_ABORT/CHECK_LOAD` branches present in: Battle cleanup, UninitShips
- [ ] `IN_ENCOUNTER/IN_LAST_BATTLE` branches present in: spawn_ship, ship_preprocess, FindAliveStarShip, InitShips
- [ ] `inHyperSpace/inQuasiSpace` branches present in: spawn_ship, BattleSong, InitShips
- [ ] Max-speed branches present in: DoBattle/RedrawQueue rendering skip

## Verification Commands

```bash
# Phase 1 still passes
cargo test --workspace --all-features
```

## Pass/Fail Gate Criteria
- **PASS:** All 64 functions have accurate pseudocode. All semantic checklist items verified. All branch families present in the correct functions. Pseudocode line numbers are traceable.
- **FAIL:** Any function missing pseudocode, any behavioral discrepancy with C reference, any branch family missing from a function that contains it in C.
