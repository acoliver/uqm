# Phase 12a: Battle Lifecycle Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P12a`

## Prerequisites
- Required: Phase 12 (Battle Lifecycle) completed
- Expected artifacts: 13 functions in `lifecycle.rs`

## Structural Verification Checklist
- [ ] All 13 functions present: battle, init_ships, uninit_ships, init_space, uninit_space, process_input, count_crew_elements, run_away_allowed, setup_battle_input_order, battle_song, free_battle_song, select_all_ships, get_player_order
- [ ] Phase 1 type definitions preserved
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory — Most Important)

### Battle() top-level sequence (battle.c:258-354)
- [ ] **Step 1**: TFB_SeedRandom(GetTimeCounter()) — deterministic seed for DEMO_MODE
- [ ] **Step 2**: BattleSong(FALSE) — load battle music
- [ ] **Step 3**: InitShips → returns num_ships
- [ ] **Step 4**: instantVictory check: if battle_counter[0]==0 or [1]==0, skip to cleanup
- [ ] **Step 5**: battle_counter initialization from player fleets
- [ ] **Step 6**: selectAllShips — initial ship selection/spawn
- [ ] **Step 7**: DoInput(BATTLE_STATE, DoBattle) — main frame loop
- [ ] **Step 8**: Cleanup: StopDitty → StopMusic → StopSound → UninitShips → FreeBattleSong
- [ ] **Exit conditions**: IN_BATTLE cleared, or CHECK_ABORT, or CHECK_LOAD

### InitShips (init.c:117-165)
- [ ] Calls InitSpace for shared assets
- [ ] SetContext(StatusContext) → ClearDrawable
- [ ] SetContext(SpaceContext) → ClearDrawable
- [ ] InitDisplayList → empty display list with free chain
- [ ] InitGalaxy → background star field
- [ ] Queue setup for both players
- [ ] Returns number of ships available

### UninitShips (init.c:183-228)
- [ ] StopSound(BATTLE_CHANNEL) first
- [ ] UninitSpace for shared asset release
- [ ] Crew recovery loop: for each player, crew_level = min(crew_level, max_crew)
- [ ] **Crew cap**: crew never exceeds max_crew (prevents overflow from crew pickups)
- [ ] free_ship called for both players' active ships
- [ ] IN_BATTLE flag cleared

### InitSpace / UninitSpace reference counting (init.c:230-310)
- [ ] **First call**: loads explosion[3], blast[3], asteroid[3], stars_in_space; refcount = 1
- [ ] **Second call**: increments refcount only; no re-loading
- [ ] **UninitSpace first call**: decrements refcount; if > 0, nothing freed
- [ ] **UninitSpace to zero**: frees all shared assets
- [ ] **Test**: init → init → uninit → uninit: verify correct load once, free once

### ProcessInput (battle.c:130-210)
- [ ] Maps BATTLE_LEFT → SHIP_LEFT
- [ ] Maps BATTLE_RIGHT → SHIP_RIGHT
- [ ] Maps BATTLE_THRUST → THRUST
- [ ] Maps BATTLE_WEAPON → WEAPON
- [ ] Maps BATTLE_SPECIAL → SPECIAL
- [ ] Maps BATTLE_ESCAPE → calls DoRunAway if RunAwayAllowed()
- [ ] Per-player processing (both players)
- [ ] NETPLAY: flush network input after mapping

### CountCrewElements (init.c:316-340)
- [ ] Iterates display list head to tail
- [ ] Counts elements with CREW_OBJECT flag and matching player
- [ ] Returns integer count

### RunAwayAllowed
- [ ] SuperMelee: always true
- [ ] Story encounter: true unless IN_LAST_BATTLE
- [ ] IN_LAST_BATTLE: false (flee blocked in final battle)

### BattleSong / FreeBattleSong (battle.c:415-490)
- [ ] inHyperSpace → hyperspace music loaded
- [ ] inQuasiSpace → quasispace music loaded
- [ ] Normal battle → battle music loaded
- [ ] cleanup_only=TRUE → music released without loading new
- [ ] FreeBattleSong: releases loaded music

### selectAllShips (battle.c:142-175)
- [ ] num_ships == 1 → GetNextStarShip(NULL, 0) for both sides (HyperSpace single-ship)
- [ ] num_ships > 1 → GetInitialStarShips for both sides

## Branch-Parity Verification
- [ ] `SUPER_MELEE`: different init, flee always allowed, specific music
- [ ] `CHECK_ABORT`: Battle() exit check after DoInput returns
- [ ] `CHECK_LOAD`: Battle() exit check after DoInput returns
- [ ] `IN_ENCOUNTER` / `IN_LAST_BATTLE`: flee rules, init differences
- [ ] `inHyperSpace()` / `inQuasiSpace()`: music selection, single-ship spawn
- [ ] `NETPLAY`: ProcessInput flush, Battle() frame sync
- [ ] `DEMO_MODE`: deterministic RNG seed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/lifecycle.rs
```

## Pass/Fail Gate Criteria
- **PASS:** Battle() full sequence verified. InitShips/UninitShips sequence correct. Reference counting for InitSpace/UninitSpace verified (load once, free once). ProcessInput maps all 6 inputs correctly. Music selection per context correct. All 7+ branch families handled. No TODO/FIXME/HACK.
- **FAIL:** Battle() sequence out of order. Crew recovery not capped. Reference count leak or double-free. Any input mapping wrong. Music wrong for hyperspace/quasispace. CHECK_ABORT/CHECK_LOAD not handled.
