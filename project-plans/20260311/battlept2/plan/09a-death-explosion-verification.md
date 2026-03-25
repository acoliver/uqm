# Phase 09a: Death + Explosion Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P09a`

## Prerequisites
- Required: Phase 09 (Death + Explosion) completed
- Expected artifacts: 17 functions in `tactical.rs`

## Structural Verification Checklist
- [ ] All 17 functions present in `tactical.rs`
- [ ] Phase 1 type definitions preserved
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory — Most Important)

### Death callback chain (4-phase sequence)
- [ ] **Phase 1 — ship_death → StartShipExplosion**: preprocess_func set to explosion_preprocess; death_func set to cleanup_dead_ship; life_span = EXPLOSION_LIFE (NUM_EXPLOSION_FRAMES * 3); RecordShipDeath called; checkOtherShipLifeSpan called; explosion sound started
- [ ] **Phase 2 — explosion_preprocess (per-frame)**: spawns 1-3 fragments per frame; fragment count = random 1-3; each fragment has random angle (TFB_Random % FULL_CIRCLE), random distance from ship center, random velocity; fragments use animation_preprocess from P07; CHANGING flag set on ship element
- [ ] **Phase 3 — cleanup_dead_ship**: crew_level written back to starship.crew_level from RaceDescPtr.ship_info.crew_level; iterates ALL elements in display list; elements owned by dead ship (GetElementStarShip == DeadStarShipPtr): SetElementStarShip(0), mark for deletion EXCEPT crew objects with crew_preprocess; victory ditty played if winner has PLAY_VICTORY_DITTY; death_func set to new_ship; preprocess_func set to preprocess_dead_ship; life_span = MIN_DITTY_FRAME_COUNT; DISAPPEARING cleared
- [ ] **Phase 4 — new_ship**: readyForBattleEnd() check → if not ready, extend life_span and return; StopDitty, StopMusic, StopSound; free ship image; GetNextStarShip for replacement; if replacement found: spawn_ship, update battle_counter; if no replacement: IN_BATTLE cleared; BattleSong restart; netplay negotiation
- [ ] **Chain integrity**: each phase correctly sets up the next phase's callbacks and life_span

### Explosion fragment details (tactrans.c explosion_preprocess)
- [ ] Fragment count per frame: 1 + (TFB_Random % 3) = 1 to 3
- [ ] Total explosion frames: NUM_EXPLOSION_FRAMES * 3 (life_span controls)
- [ ] Fragment position: ship center + random offset based on angle and distance
- [ ] Fragment velocity: random, based on distance from center
- [ ] Fragment uses animation_preprocess as preprocess_func
- [ ] Fragment has FINITE_LIFE flag

### Simultaneous death (checkOtherShipLifeSpan)
- [ ] Iterates display list for other PLAYER_SHIP elements
- [ ] If winner found (mass_points ≤ MAX_SHIP_MASS + 1, crew_level > 0): winner gets +1 life_span
- [ ] If tie (both dying, no winner): keep other dead ship alive (life_span extended)
- [ ] Pkunk reincarnation: mass_points == MAX_SHIP_MASS + 1 treated as alive
- [ ] SetWinnerStarShip called for winner: PLAY_VICTORY_DITTY set, first winner preserved

### cleanup_dead_ship element cleanup
- [ ] EVERY element with GetElementStarShip == DeadStarShipPtr: SetElementStarShip(0)
- [ ] Non-crew elements: PrimType = NO_PRIM, life_span = 0, state_flags = NONSOLID | DISAPPEARING | FINITE_LIFE, all callbacks nulled
- [ ] Crew objects with crew_preprocess: KEPT ALIVE (not marked for deletion)
- [ ] Victory ditty: played for starship with PLAY_VICTORY_DITTY flag, flag then cleared

### preprocess_dead_ship
- [ ] Only action: ProcessSound(~0, NULL) — flush pending sounds
- [ ] Element parameter unused

### readyForBattleEnd (3-mode behavior)
- [ ] Non-netplay non-demo: return !DittyPlaying()
- [ ] DEMO_MODE: always return true (journal replay accuracy)
- [ ] NETPLAY: !DittyPlaying() AND all players' battleEndReady handlers return true

### RecordShipDeath
- [ ] Decrements battle_counter[playerNr]
- [ ] Skip condition: mass_points > MAX_SHIP_MASS (ship is running away, not dead)

### Ion trail
- [ ] spawn_ion_trail: allocates element, positions behind ship at facing offset
- [ ] cycle_ion_trail: frame advance, fade effect, CHANGING flag, finite life

### Ditty functions
- [ ] PlayDitty: loads race-specific victory music, starts playback
- [ ] StopDitty: stops victory music playback
- [ ] DittyPlaying: returns true if victory music still playing
- [ ] StopAllBattleMusic: stops all active battle-related music

## Branch-Parity Verification
- [ ] `NETPLAY`: readyForBattleEnd all-player check
- [ ] `DEMO_MODE`: readyForBattleEnd always true
- [ ] `SUPER_MELEE`: RecordShipDeath per-player tracking, new_ship crew recovery

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/tactical.rs
```

## Pass/Fail Gate Criteria
- **PASS:** Full 4-phase death chain verified. Explosion fragments spawned correctly per frame. Simultaneous death handling (winner +1, tie keep-alive) verified. Cleanup preserves crew objects. readyForBattleEnd all 3 modes. No TODO/FIXME/HACK.
- **FAIL:** Death chain doesn't progress through all 4 phases. Explosion fragments wrong count/position/velocity. Simultaneous death incorrect. Cleanup destroys crew objects. readyForBattleEnd wrong for any mode.
