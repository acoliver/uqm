# Phase 12: Battle Lifecycle

## Phase ID
`PLAN-20260320-BATTLEPT2.P12`

## Prerequisites
- Required: Phase 11a (AI Dispatch Verification) completed with PASS
- Expected files: All battle logic modules complete (process_loop.rs, ship_runtime.rs, tactical.rs, ai.rs)
- Expected artifacts: All 64 ported functions implemented across modules

## Requirements Implemented (Expanded)

### REQ: Battle entry/exit (battle/requirements.md §Battle entry/exit)
**Requirement text**: Battle() is the top-level entry point. Seeds RNG, loads battle song, initializes ships, runs DoInput loop with DoBattle callback, cleans up on exit.

Behavior contract:
- GIVEN: A battle encounter (Super Melee or story encounter)
- WHEN: Battle() is called (battle.c:396-516)
- THEN: Full sequence matching battle.c:396-516:
  1. RNG seed: TFB_SeedRandom(GetTimeCounter()) unless SUPER_MELEE (already seeded) or DEMO_MODE (deterministic BattleSeed) (lines 402-412)
  2. BattleSong(FALSE) — loads music resource but does NOT play (line 414)
  3. num_ships = InitShips() (line 416)
  4. instantVictory check: if set, num_ships=0, battle_counter=[1,0], clear flag (lines 418-424)
  5. if num_ships > 0: set IN_BATTLE (line 430), battle_counter[0/1] = CountLinks (lines 431-432), SetGraphicScaleMode if continuous zoom (lines 434-435), setupBattleInputOrder (line 437), NETPLAY init: buffers/checksums/framecount=0/ResetWinnerStarShip/setBattleStateConnections (lines 438-446)
  6. selectAllShips(num_ships) — if fails: CHECK_ABORT → goto AbortBattle (lines 449-452)
  7. BattleSong(TRUE) — NOW plays music (line 455)
  8. NETPLAY: initBattleStateDataConnections + negotiateReadyConnections → if fails: CHECK_ABORT → goto AbortBattle (lines 457-466)
  9. DoInput(&bs, FALSE) — main battle loop with DoBattle callback (line 472)
  10. AbortBattle label (line 475): SUPER_MELEE abort handling (clear CHECK_ABORT, or MeleeGameOver) (lines 476-496), NETPLAY cleanup (uninitBattleInputBuffers, uninitChecksumBuffers, setBattleStateConnections(NULL)) (lines 498-504)
  11. StopDitty + StopMusic + StopSound (lines 506-508) — inside the `if (num_ships)` block
  12. UninitShips() — always called regardless of num_ships (line 511)
  13. FreeBattleSong() — always called (line 512)
  14. return (BOOLEAN)(num_ships < 0) — returns TRUE only for negative num_ships (line 515)

### REQ: Ship initialization (battle/requirements.md §Ship initialization)
**Requirement text**: InitShips initializes the battle arena: space background, status/space contexts, display list, galaxy background, player queues.

Behavior contract:
- GIVEN: Battle is starting
- WHEN: InitShips is called
- THEN: InitSpace → SetContext(StatusContext) → InitDisplayList → InitGalaxy → queue setup for both players

### REQ: Ship deinitialization (battle/requirements.md §Ship deinitialization)
**Requirement text**: UninitShips tears down: stops sound, calls UninitSpace, counts floating crew, iterates display list to find ship elements, recovers crew for surviving ship, frees each ship's resources, clears IN_BATTLE, handles post-battle crew updates and queue cleanup.

Behavior contract:
- GIVEN: Battle is ending
- WHEN: UninitShips is called (init.c:277-361)
- THEN:
  1. StopSound() — stops all battle sounds
  2. UninitSpace() — releases shared assets
  3. CountCrewElements() — counts CREW_OBJECT elements floating in space
  4. Iterate entire display list: for each element that is PLAYER_SHIP or has death_func == new_ship, get its StarShipPtr, recover crew (surviving ship gets floating crew capped at max_crew), record final crew_level, track SPtr[playerNr], free_ship(RaceDescPtr, TRUE, TRUE), clear RaceDescPtr
  5. Clear IN_BATTLE from CurrentActivity
  6. If IN_ENCOUNTER and not CHECK_ABORT: iterate SPtr[] backwards, call UpdateShipFragCrew for non-infinite fleets
  7. If not IN_ENCOUNTER: ReinitQueue both race_q's, FreeHyperspace if inHQSpace

### REQ: Space initialization (battle/requirements.md §Space initialization)
**Requirement text**: InitSpace loads shared assets (explosion, blast, asteroid frames) with reference counting. UninitSpace releases with correct refcount semantics.

Behavior contract:
- GIVEN: Battle space is being initialized
- WHEN: InitSpace is called (possibly re-entrant from multiple battles)
- THEN: Shared assets loaded with reference count increment; second call increments count without re-loading

### REQ: Input processing (battle/requirements.md §Input processing)
**Requirement text**: ProcessInput maps abstract battle inputs (BATTLE_LEFT/RIGHT/THRUST/WEAPON/SPECIAL/ESCAPE) to ship_input_state. BATTLE_ESCAPE triggers DoRunAway.

Behavior contract:
- GIVEN: Player input state available
- WHEN: ProcessInput is called
- THEN: Each player's abstract inputs mapped to ship input flags; BATTLE_ESCAPE → DoRunAway if allowed

### REQ: Crew counting (battle/requirements.md §Crew counting)
**Requirement text**: CountCrewElements scans the display list for CREW_OBJECT elements to count all visible crew globally.

Behavior contract:
- GIVEN: A display list with crew objects
- WHEN: CountCrewElements is called (init.c:253-274, static function, takes no arguments)
- THEN: Returns total count of ALL CREW_OBJECT elements in the display list (not per-player — counts globally)

### REQ: Flee allowance (battle/requirements.md §Flee allowance)
**Requirement text**: RunAwayAllowed returns whether the current game state allows fleeing (battle.c:63-70).

Behavior contract:
- GIVEN: Any battle context
- WHEN: RunAwayAllowed is checked
- THEN: Returns the conjunction of three activity/game-state predicates (battle.c:63-70): `(LOBYTE(CurrentActivity) == IN_ENCOUNTER || LOBYTE(CurrentActivity) == IN_LAST_BATTLE) && GET_GAME_STATE(STARBASE_AVAILABLE) && !GET_GAME_STATE(BOMB_CARRIER)`. SUPER_MELEE is excluded implicitly because LOBYTE(CurrentActivity) == SUPER_MELEE fails the first predicate; it is not special-cased symbolically.

### REQ: Battle input order (battle/requirements.md §Battle input order)
**Requirement text**: setupBattleInputOrder configures which player gets priority input processing.

Behavior contract:
- GIVEN: A battle with two players
- WHEN: setupBattleInputOrder is called
- THEN: Input processing order set based on player types (human/AI/network)

### REQ: Battle music (battle/requirements.md §Battle music)
**Requirement text**: BattleSong loads context-appropriate music (hyperspace, quasispace, or battle); FreeBattleSong releases it.

Behavior contract:
- GIVEN: A battle in hyperspace
- WHEN: BattleSong is called
- THEN: Hyperspace battle music loaded; normal battle music NOT loaded

### REQ: Ship selection (battle/requirements.md §Ship selection)
**Requirement text**: selectAllShips handles single-ship (hyperspace) vs multi-ship (encounter) selection.

Behavior contract:
- GIVEN: A hyperspace encounter with 1 ship
- WHEN: selectAllShips is called
- THEN: GetNextStarShip(NULL, 0) called for single-ship spawn

### REQ: Player order (battle/requirements.md §Player order)
**Requirement text**: GetPlayerOrder returns which player acts first based on battle configuration.

Behavior contract:
- GIVEN: A battle configuration
- WHEN: GetPlayerOrder is called
- THEN: Returns correct player ordering

## Implementation Tasks

### Files to modify

- `rust/src/battle/lifecycle.rs` — Add lifecycle logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P12`
  - marker: `@requirement REQ-BATTLE-ENTRY, REQ-SHIP-INIT, REQ-SPACE-INIT, REQ-INPUT-PROCESSING`
  - Contents to add:
    - `pub fn battle()` — Top-level entry matching battle.c Battle():396-516. Full sequence with abort/cleanup paths:
      (1) RNG seed: TFB_SeedRandom(GetTimeCounter()) unless SUPER_MELEE or DEMO_MODE (lines 402-412)
      (2) BattleSong(FALSE) — load music, do NOT play (line 414)
      (3) num_ships = InitShips() (line 416)
      (4) instantVictory: if set → num_ships=0, battle_counter=[1,0], clear flag (lines 418-424)
      (5) if num_ships > 0: IN_BATTLE set, battle_counter from CountLinks, SetGraphicScaleMode, setupBattleInputOrder, NETPLAY init (lines 426-446)
      (6) selectAllShips(num_ships) — if fails: CHECK_ABORT, goto AbortBattle (lines 449-452)
      (7) BattleSong(TRUE) — play music (line 455)
      (8) NETPLAY: initBattleStateDataConnections + negotiateReady — if fails: CHECK_ABORT, goto AbortBattle (lines 457-466)
      (9) DoInput(&bs, FALSE) — main battle loop (line 472)
      (10) AbortBattle (line 475): SUPER_MELEE: CHECK_ABORT → clear abort flag (+ NETPLAY waitResetConnections), else → MeleeGameOver() (lines 476-496). NETPLAY cleanup: uninitBattleInputBuffers, uninitChecksumBuffers, setBattleStateConnections(NULL) (lines 498-504)
      (11) StopDitty + StopMusic + StopSound (lines 506-508) — inside num_ships>0 block
      (12) UninitShips() — always called (line 511)
      (13) FreeBattleSong() — always called (line 512)
      (14) return (BOOLEAN)(num_ships < 0) (line 515)
    - `pub fn init_ships() -> i32` — Initialization matching init.c InitShips:182-250. InitSpace → SetContext(StatusContext) → SetContext(SpaceContext) → InitDisplayList → InitGalaxy → branch on inHQSpace: (HQ: ReinitQueue both race_q's, BuildSIS, LoadHyperspace, num_ships=1) or (encounter: SetContextFGFrame, SetContextClipRect, SetContextBackGroundColor, ClearDrawable, spawn asteroids/planets or free_gravity_well for IN_LAST_BATTLE, num_ships=NUM_SIDES) → return num_ships.
    - `pub fn uninit_ships()` — Teardown matching init.c UninitShips:277-361. StopSound → UninitSpace → CountCrewElements (floating crew count) → iterate display list: for each PLAYER_SHIP or death_func==new_ship element, recover crew (surviving ship gets floating crew capped at max_crew), record crew_level, free_ship per ship → clear IN_BATTLE → if IN_ENCOUNTER && !CHECK_ABORT: UpdateShipFragCrew per player → if !IN_ENCOUNTER: ReinitQueue both race_q's, FreeHyperspace if inHQSpace.
    - `pub fn init_space()` — Asset loading matching init.c InitSpace:230-280. Reference-counted loading of explosion[3], blast[3], asteroid[3] frame arrays and stars_in_space.
    - `pub fn uninit_space()` — Asset release matching init.c UninitSpace:282-310. Reference-counted release.
    - `pub fn process_input()` — Input mapping matching battle.c ProcessInput:130-210. For each player: read abstract inputs → map to SHIP_LEFT/SHIP_RIGHT/THRUST/WEAPON/SPECIAL → BATTLE_ESCAPE → DoRunAway if RunAwayAllowed. Netplay flush.
    - `fn count_crew_elements() -> u32` — Crew scan matching init.c:253-274 CountCrewElements. Static/private function, takes no arguments. Iterates entire display list counting ALL elements with CREW_OBJECT flag set (global count, not per-player).
    - `pub fn run_away_allowed() -> bool` — Flee check matching battle.c:63-70 RunAwayAllowed. Pure activity/game-state predicate: `(LOBYTE(CurrentActivity) == IN_ENCOUNTER || LOBYTE(CurrentActivity) == IN_LAST_BATTLE) && GET_GAME_STATE(STARBASE_AVAILABLE) && !GET_GAME_STATE(BOMB_CARRIER)`. SUPER_MELEE is excluded implicitly (its activity byte doesn't match either predicate), not by name.
    - `pub fn setup_battle_input_order()` — Input order matching battle.c setupBattleInputOrder.
    - `pub fn battle_song(do_play: bool)` — Music matching battle.c BattleSong(BOOLEAN DoPlay):234-249. Loads the appropriate music resource if not yet loaded (inHyperSpace → HYPERSPACE_MUSIC, inQuasiSpace → QUASISPACE_MUSIC, else → BATTLE_MUSIC). If do_play is true, calls PlayMusic(BattleRef, TRUE, 1) to start playback.
    - `pub fn free_battle_song()` — Release matching battle.c FreeBattleSong:472-490.
    - `pub fn select_all_ships()` — Selection matching battle.c selectAllShips:142-175. 1 ship → HyperSpace (GetNextStarShip(NULL, 0)), else → GetInitialStarShips.
    - `pub fn get_player_order() -> u8` — Order matching battle.c GetPlayerOrder.

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `Battle()` | battle.c | :396-516 | `battle()` | `lifecycle.rs` |
| `DoBattle()` | battle.c | :258-354 | `do_battle()` | `lifecycle.rs` |
| `InitShips()` | init.c | :181-249 | `init_ships()` | `lifecycle.rs` |
| `UninitShips()` | init.c | :277-361 | `uninit_ships()` | `lifecycle.rs` |
| `InitSpace()` | init.c | :117-148 | `init_space()` | `lifecycle.rs` |
| `UninitSpace()` | init.c | :150-162 | `uninit_space()` | `lifecycle.rs` |
| `ProcessInput()` | battle.c | :144-226 | `process_input()` | `lifecycle.rs` |
| `CountCrewElements()` | init.c | :253-274 | `count_crew_elements()` | `lifecycle.rs` |
| `RunAwayAllowed()` | battle.c | :63-70 | `run_away_allowed()` | `lifecycle.rs` |
| `setupBattleInputOrder()` | battle.c | :107-135 | `setup_battle_input_order()` | `lifecycle.rs` |
| `BattleSong()` | battle.c | :234-249 | `battle_song()` | `lifecycle.rs` |
| `FreeBattleSong()` | battle.c | :251-256 | `free_battle_song()` | `lifecycle.rs` |
| `selectAllShips()` | battle.c | :375-394 | `select_all_ships()` | `lifecycle.rs` |
| `GetPlayerOrder()` | battle.c | :357-372 | `get_player_order()` | `lifecycle.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `SUPER_MELEE` | Battle(), selectAllShips, RunAwayAllowed, BattleSong | SuperMelee: different init, ship selection, flee always allowed, specific music |
| `CHECK_ABORT` | Battle() exit check | Abort-game cleanup path |
| `CHECK_LOAD` | Battle() exit check | Load-game cleanup path |
| `IN_ENCOUNTER` | RunAwayAllowed, InitShips | Encounter-specific init and flee rules |
| `IN_LAST_BATTLE` | RunAwayAllowed | Final battle: flee blocked |
| `inHyperSpace()` | BattleSong, selectAllShips | HyperSpace music, single-ship spawn |
| `inQuasiSpace()` | BattleSong | QuasiSpace music |
| `NETPLAY` | ProcessInput, Battle() | Netplay input flush, frame sync |
| `DEMO_MODE` / `CREATE_JOURNAL` | Battle() RNG seed | Demo: deterministic seed |
| `USE_RUST_SHIPS` | init.c:38-41 (extern declarations), init.c:184-186 (InitShips guard → `rust_ships_init()`), init.c:279-281 (UninitShips guard → `rust_ships_uninit()`); ship.c:38-43 (extern declarations), ship.c:158-160 (ship_preprocess → `rust_ships_preprocess()`), ship.c:295-297 (ship_postprocess → `rust_ships_postprocess()`), ship.c:396-397 (spawn_ship → `rust_ships_spawn()`) | When enabled, delegates InitShips→rust_ships_init(), UninitShips→rust_ships_uninit(), ship_preprocess→rust_ships_preprocess(), ship_postprocess→rust_ships_postprocess(), spawn_ship→rust_ships_spawn(). Rust side: these are the actual implementations, so this branch IS the Rust path. Note: `USE_RUST_SHIPS` is an EXISTING guard separate from `USE_RUST_BATTLE_LOOP` (P13). Both may coexist. |

### Integration points
- P05 `process_loop.rs`: redraw_queue, init_display_list, init_kernel
- P08 `ship_runtime.rs`: get_next_starship, get_initial_starships
- P10 `tactical.rs`: do_run_away, reset_winner_starship
- P09 `tactical.rs`: stop_ditty, stop_all_battle_music
- P06 `c_bridge.rs`: TFB_SeedRandom, SetContext, DoInput, StopSound, StopMusic, etc.
- Phase 1 `lifecycle.rs` (types): BattleState, frame rate constants
- Phase 1 `display_list.rs`: push_back for queue setup

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/battle-lifecycle.md`: Battle, InitShips, UninitShips, ProcessInput, BattleSong sections

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 13 functions implemented in `lifecycle.rs`
- [ ] Phase 1 type definitions preserved
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Battle(): full sequence: RNG seed → BattleSong(FALSE) → InitShips → instantVictory → IN_BATTLE setup → selectAllShips (fail→AbortBattle) → BattleSong(TRUE) → NETPLAY negotiate (fail→AbortBattle) → DoInput(DoBattle) → AbortBattle: SUPER_MELEE abort/MeleeGameOver, NETPLAY cleanup → StopDitty+StopMusic+StopSound → UninitShips (always) → FreeBattleSong (always) → return (num_ships<0)
- [ ] InitShips: InitSpace → SetContext(StatusContext) → SetContext(SpaceContext) → InitDisplayList → InitGalaxy → HQ branch (ReinitQueue, BuildSIS, LoadHyperspace) or encounter branch (clip rect, asteroids/planets or free_gravity_well)
- [ ] UninitShips: StopSound → UninitSpace → CountCrewElements → iterate display list for ship elements → crew recovery (floating crew to survivor, capped at max) → free_ship per ship → clear IN_BATTLE → encounter: UpdateShipFragCrew → non-encounter: ReinitQueue + FreeHyperspace
- [ ] InitSpace/UninitSpace: reference counting correct (increment on init, decrement on uninit, free at zero)
- [ ] ProcessInput: all 5 abstract inputs mapped correctly; BATTLE_ESCAPE → DoRunAway
- [ ] CountCrewElements: correct CREW_OBJECT scan
- [ ] RunAwayAllowed: pure activity/game-state predicate — (activity==IN_ENCOUNTER || activity==IN_LAST_BATTLE) && STARBASE_AVAILABLE && !BOMB_CARRIER; SUPER_MELEE excluded implicitly (not symbolically special-cased)
- [ ] BattleSong: hyperspace/quasispace/battle music selection correct
- [ ] selectAllShips: 1 ship → HyperSpace path; >1 → GetInitialStarShips
- [ ] All cleanup sequences release all resources
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/lifecycle.rs
```

## Success Criteria
- [ ] All 13 functions implemented and tested
- [ ] Battle lifecycle end-to-end sequence correct
- [ ] Reference counting correct (no leaks, no double-free)
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/lifecycle.rs`
- blocking issues: Reference counting, DoInput bridge complexity, music loading

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P12.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P12
- timestamp
- files changed: lifecycle.rs
- tests added/updated
- verification outputs
- semantic verification summary
