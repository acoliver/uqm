# Phase 09: Tactical Transitions — Death + Explosion

## Phase ID
`PLAN-20260320-BATTLEPT2.P09`

## Prerequisites
- Required: Phase 08a (Ship Spawn Verification) completed with PASS
- Expected files: `ship_runtime.rs` with P07+P08 functions
- Expected artifacts: spawn_ship, get_next_starship verified

## Requirements Implemented (Expanded)

### REQ: Death callback chain (battle/requirements.md §Ship death)
**Requirement text**: ship_death is the element's death_func. It orchestrates the death entry sequence, then the four-phase death lifecycle: (1) StartShipExplosion, (2) explosion_preprocess (spawns fragments over 36 frames), (3) cleanup_dead_ship (final disposal), (4) new_ship (replacement selection).

Behavior contract:
- GIVEN: A ship element with life_span == 0 (crew depleted)
- WHEN: PreProcess triggers death_func = ship_death
- THEN: ship_death executes in this exact order (tactrans.c:729-749, verified line-by-line):
  1. GetElementStarShip(ShipPtr, &StarShipPtr) — (line 735)
  2. StopAllBattleMusic() — stops ditty + battle music (line 737)
  3. Clear PLAY_VICTORY_DITTY from cur_status_flags — prevents ditty if winner dies before loser finishes exploding (line 742)
  4. StartShipExplosion(ShipPtr, true) — initializes explosion state (line 744)
  5. winner = FindAliveStarShip(ShipPtr) — finds the surviving opponent, if any (line 746)
  6. SetWinnerStarShip(winner) — sets PLAY_VICTORY_DITTY on winner, records winner; first call wins in simultaneous death (line 747)
  7. RecordShipDeath(ShipPtr) — decrements battle_counter[playerNr] (skips if fleeing), calls MeleeShipDeath in SUPER_MELEE (line 748)
  Note: RecordShipDeath IS called directly from ship_death (confirmed at tactrans.c:748). It is NOT called from elsewhere in the death chain.

### REQ: Explosion fragment spawning (battle/requirements.md §Explosion fragments)
**Requirement text**: explosion_preprocess spawns fragments per frame over NUM_EXPLOSION_FRAMES*3 (36) frames using an explicit switch schedule that determines spawn count per tick. Special effects occur at specific ticks.

Behavior contract:
- GIVEN: A ship in explosion phase (preprocess_func = explosion_preprocess)
- WHEN: Each frame's PreProcess calls explosion_preprocess
- THEN: Fragment count determined by tick index `i = (NUM_EXPLOSION_FRAMES * 3) - life_span` via switch (tactrans.c:548-575):
  - Tick 25: sets preprocess_func = NULL (disables further preprocessing), then C switch **falls through** (no break) into the case 0/1/2/20-24 group which sets i=1 (tactrans.c:550-561: `case 25: preprocess_func=NULL;` falls to `case 0: ... case 24: i=1; break;`). The subsequent `do { /* alloc+init fragment */ } while (--i);` loop runs exactly once with i=1 (body executes, then --i yields 0 → loop exits) → **spawns 1 fragment**. Note: i is the loop counter, not a "fragment count minus one" — the do-while always executes at least once when i >= 1.
  - Ticks 0, 1, 2, 20, 21, 22, 23, 24: spawn 1 fragment (i=1)
  - Ticks 3, 4, 5, 18, 19: spawn 2 fragments
  - Tick 15: hides ship prim (SetPrimType NO_PRIM, sets CHANGING), then falls through to spawn 3 fragments
  - All other ticks (default): spawn 3 fragments
  - Each fragment: AllocElement, random angle from TFB_Random, random distance (0-7 display units + conditional +8), positioned relative to ship, life_span=9, animation_preprocess for frame advance, random velocity

### REQ: Post-explosion cleanup (battle/requirements.md §Post-explosion cleanup)
**Requirement text**: cleanup_dead_ship saves final crew to starship, clears all elements owned by dead ship, conditionally starts victory ditty, sets up preprocess_dead_ship + new_ship chain with multi-step life_span calculation.

Behavior contract:
- GIVEN: Explosion animation has completed (life_span reaches 0, death_func = cleanup_dead_ship)
- WHEN: cleanup_dead_ship executes (tactrans.c:288-373)
- THEN:
  1. ProcessSound((SOUND)~0, NULL) to flush
  2. crew_level written back to starship from RaceDescPtr->ship_info.crew_level
  3. Iterate all elements: those owned by dead ship (matching StarShipPtr) get SetElementStarShip(0); non-crew-object elements marked for deletion (NO_PRIM, life_span=0, NONSOLID|DISAPPEARING|FINITE_LIFE, all callbacks zeroed); elements with PLAY_VICTORY_DITTY on their starship trigger PlayDitty and clear the flag
  4. Multi-step life_span calculation (tactrans.c:358-371):
     a. life_span = MusicStarted ? MIN_DITTY_FRAME_COUNT : 1
     b. If dead ship IS the winnerStarShip (won but died, e.g. Glory Device): life_span = MIN_DITTY_FRAME_COUNT + 1
     c. Then unconditionally: ++life_span (original code comment says "almost sure it is not needed, but it keeps the original framecount")
  5. death_func set to new_ship, preprocess_func set to preprocess_dead_ship
  6. DISAPPEARING cleared
  7. SetElementStarShip(DeadShipPtr, DeadStarShipPtr) to re-bind

### REQ: Ship replacement (battle/requirements.md §Ship replacement)
**Requirement text**: new_ship checks readyForBattleEnd, stops music/sounds, frees dead ship, gets replacement via GetNextStarShip, updates battle_counter.

Behavior contract:
- GIVEN: Ditty playback period complete (life_span reaches 0, death_func = new_ship)
- WHEN: new_ship executes
- THEN: If readyForBattleEnd(), stop ditty/music/sound, free ship image, GetNextStarShip for replacement, update battle_counter, restart BattleSong

### REQ: Simultaneous death handling (battle/requirements.md §Simultaneous death)
**Requirement text**: checkOtherShipLifeSpan ensures winner stays alive longer than loser. In ties, keeps other dead ships alive for sequential processing.

Behavior contract:
- GIVEN: Both ships die in the same frame
- WHEN: cleanup_dead_ship runs for the first ship
- THEN: Winner ship's life_span extended by 1 to ensure it outlives loser; if no winner (tie), other dead ship kept alive

### REQ: Victory ditty (battle/requirements.md §Victory ditty)
**Requirement text**: PlayDitty plays race victory music, StopDitty stops it, DittyPlaying checks. StopAllBattleMusic calls StopDitty + StopMusic only (does NOT call StopSound — sound effects are stopped separately by new_ship which calls StopSound independently).

Behavior contract:
- GIVEN: A winning ship
- WHEN: PlayDitty is called with the winning starship
- THEN: Race-specific victory music starts playing

- GIVEN: Battle music and ditty need stopping
- WHEN: StopAllBattleMusic is called (tactrans.c:619-623)
- THEN: StopDitty() + StopMusic() are called. StopSound is NOT included — it is a separate responsibility.

StopAllBattleMusic call-site semantics (each placement has a distinct purpose):
- **ship_death entry** (tactrans.c:737): Prevents wrong ditty/song from carrying over into explosion phase. Called BEFORE StartShipExplosion, so any previous battle music or an earlier ship's victory ditty is silenced before the new death sequence begins.
- **Pkunk reincarnation** (pkunk.c:331): Stops music before phoenix rebirth sequence.
- **Shofixti Glory Device** (shofixti.c:325): Stops music before self-destruct sequence.

new_ship (tactrans.c:469-471) does NOT call StopAllBattleMusic. Instead it calls StopDitty() + StopMusic() + StopSound() as three separate calls, because new_ship also needs to stop sound effects (for clean ship selection) and may conditionally restart BattleSong(TRUE) if RestartMusic (opponent still alive, tactrans.c:523-524).

### REQ: Ion trail management
**Requirement text**: spawn_ion_trail creates trail element behind thruster. cycle_ion_trail animates and fades trail.

Behavior contract:
- GIVEN: A ship that just thrusted (not cloaked)
- WHEN: spawn_ion_trail is called
- THEN: A trail element is spawned at ship position offset by facing, with cycle_ion_trail as preprocess

### REQ: Ship death recording
**Requirement text**: RecordShipDeath (tactrans.c:682-700) decrements battle_counter for the player, but **skips the decrement when the ship is fleeing** (mass_points > MAX_SHIP_MASS), because flee-ships are already decremented in DoRunAway(). Always calls MeleeShipDeath in SUPER_MELEE mode regardless of flee status.

Behavior contract:
- GIVEN: A ship has died with mass_points <= MAX_SHIP_MASS (not fleeing)
- WHEN: RecordShipDeath is called (tactrans.c:682-700)
- THEN: battle_counter[deadStarShip->playerNr] decremented by 1

- GIVEN: A ship has died with mass_points > MAX_SHIP_MASS (running away / fleeing)
- WHEN: RecordShipDeath is called
- THEN: battle_counter decrement is SKIPPED (already counted in DoRunAway; tactrans.c:690-696 `if (deadShip->mass_points <= MAX_SHIP_MASS)` guard)

- GIVEN: A ship has died in SUPER_MELEE mode (any flee status)
- WHEN: RecordShipDeath is called and LOBYTE(CurrentActivity) == SUPER_MELEE
- THEN: MeleeShipDeath(deadStarShip) is also called to update melee scoring (tactrans.c:698-699)

### REQ: Battle-end readiness
**Requirement text**: readyForBattleEnd checks battle-end readiness. Non-netplay: checks DittyPlaying (DEMO_MODE always returns true). Netplay: requires ditty stopped AND per-player handler readiness via battleEndReady callbacks (human/computer always ready, network uses readyForBattleEndPlayer protocol with NetState transitions, frame count exchange, and negotiateReady handshake).

Behavior contract:
- GIVEN: Non-netplay mode, ditty not playing
- WHEN: readyForBattleEnd is called (tactrans.c:254-278)
- THEN: Returns true (DEMO_MODE: always true regardless of ditty)

- GIVEN: Netplay mode
- WHEN: readyForBattleEnd is called
- THEN: Returns true only if DittyPlaying() is false AND all players' battleEndReady handlers return true. Network players use readyForBattleEndPlayer (tactrans.c:169-228) which implements a multi-step protocol: (1) signal local readiness via Netplay_localReady, (2) exchange frame counts for synchronised ending, (3) continue simulation until max(local, remote) frame count reached, (4) final negotiateReady handshake to NetState_interBattle

### REQ: Min life span helpers
**Requirement text**: setMinShipLifeSpan, setMinStarShipLifeSpan, checkOtherShipLifeSpan manage simultaneous death timing.

Behavior contract:
- GIVEN: A ship that should outlive another
- WHEN: setMinShipLifeSpan is called
- THEN: Minimum life span set to ensure sequential processing

## Implementation Tasks

### Files to modify

- `rust/src/battle/tactical.rs` — Add death + explosion logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P09`
  - marker: `@requirement REQ-DEATH-CHAIN, REQ-EXPLOSION, REQ-CLEANUP, REQ-REPLACEMENT, REQ-SIMULTANEOUS, REQ-DITTY, REQ-ION-TRAIL`
  - Contents to add:
    - `pub fn ship_death(element: &mut Element)` — Death entry point matching tactrans.c:729-749 ship_death. Exact sequence: StopAllBattleMusic → clear PLAY_VICTORY_DITTY → StartShipExplosion → FindAliveStarShip → SetWinnerStarShip → RecordShipDeath. Note: checkOtherShipLifeSpan is NOT called here; it is called from new_ship during the readyForBattleEnd wait loop.
    - `fn start_ship_explosion(element: &mut Element, play_sound: bool)` — Initialize explosion matching tactrans.c:702-727 StartShipExplosion. Zeros velocity, drains energy via DeltaEnergy, sets life_span=NUM_EXPLOSION_FRAMES*3, clears DISAPPEARING, sets FINITE_LIFE|NONSOLID, sets preprocess_func=explosion_preprocess, postprocess_func=PostProcessStatus, death_func=cleanup_dead_ship, clears hTarget, plays explosion sound if play_sound.
    - `fn explosion_preprocess(element: &mut Element)` — Frame-by-frame fragment spawning matching tactrans.c:542-616 explosion_preprocess. Uses explicit switch schedule on tick index `i = (NUM_EXPLOSION_FRAMES * 3) - life_span` (tactrans.c:548-575): tick 25 sets preprocess_func=NULL then **falls through** (no break) to the case 0/1/2/20-24 group → i=1 → spawns 1 fragment; ticks 0-2,20-24 → i=1 → 1 fragment; ticks 3-5,18-19 → i=2 → 2 fragments; tick 15: hide ship prim (SetPrimType NO_PRIM, CHANGING) then falls through to default → i=3 → 3 fragments; all other ticks (default): i=3 → 3 fragments. Loop: `do { /* alloc+init fragment */ } while (--i);`. Each fragment: AllocElement, APPEARING|FINITE_LIFE|NONSOLID, life_span=9, random angle/distance/velocity from TFB_Random, animation_preprocess for frame advance.
    - `pub fn cleanup_dead_ship(element: &mut Element)` — Post-explosion cleanup matching tactrans.c:288-373. ProcessSound flush, crew_level writeback, iterate all elements: owned by dead ship → SetElementStarShip(0), non-crew-object elements marked for deletion (NO_PRIM, life_span=0, NONSOLID|DISAPPEARING|FINITE_LIFE, all callbacks zeroed); elements with PLAY_VICTORY_DITTY trigger PlayDitty. Multi-step life_span: MusicStarted ? MIN_DITTY_FRAME_COUNT : 1; winner override MIN_DITTY_FRAME_COUNT+1; then ++life_span. Sets death_func=new_ship, preprocess_func=preprocess_dead_ship, clears DISAPPEARING, re-binds element to starship.
    - `fn new_ship(element: &mut Element)` — Replacement matching tactrans.c new_ship. Checks readyForBattleEnd(), stops music/sounds, frees ship, GetNextStarShip, updates battle_counter, netplay negotiation, BattleSong restart.
    - `pub fn spawn_ion_trail(element: &Element)` — Trail matching tactrans.c spawn_ion_trail. AllocElement, position behind ship, cycle_ion_trail as preprocess.
    - `fn cycle_ion_trail(element: &mut Element)` — Animate trail matching tactrans.c cycle_ion_trail. Frame advance, fade, CHANGING flag.
    - `pub fn play_ditty(starship: &Starship)` — Victory music matching tactrans.c PlayDitty.
    - `pub fn stop_ditty()` — Stop victory music.
    - `pub fn ditty_playing() -> bool` — Check victory music.
    - `pub fn stop_all_battle_music()` — Stop all battle music.
    - `fn preprocess_dead_ship(element: &mut Element)` — ProcessSound stub during wait matching tactrans.c:280-285.
    - `pub fn record_ship_death(dead_ship: &Element)` — Decrement battle_counter matching tactrans.c:683-700 RecordShipDeath. Skip if mass_points > MAX_SHIP_MASS (running away). Additionally calls MeleeShipDeath(deadStarShip) when CurrentActivity == SUPER_MELEE.
    - `pub fn ready_for_battle_end() -> bool` — Check ditty/netplay readiness matching tactrans.c readyForBattleEnd.
    - `pub fn set_min_ship_life_span(element: &mut Element, min: u16)` — Minimum life span.
    - `pub fn set_min_starship_life_span(starship: &Starship, min: u16)` — Minimum starship life span.
    - `fn check_other_ship_life_span(element: &Element, player_nr: u8)` — Simultaneous death matching tactrans.c checkOtherShipLifeSpan. Winner gets +1 life_span; in tie, keeps other dead ship alive.

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `ship_death()` | tactrans.c | varies | `ship_death()` | `tactical.rs` |
| `StartShipExplosion()` | tactrans.c | varies | `start_ship_explosion()` | `tactical.rs` |
| `explosion_preprocess()` | tactrans.c | :450-540 | `explosion_preprocess()` | `tactical.rs` |
| `cleanup_dead_ship()` | tactrans.c | :288-400 | `cleanup_dead_ship()` | `tactical.rs` |
| `new_ship()` | tactrans.c | :400-450 | `new_ship()` | `tactical.rs` |
| `spawn_ion_trail()` | tactrans.c | varies | `spawn_ion_trail()` | `tactical.rs` |
| `cycle_ion_trail()` | tactrans.c | varies | `cycle_ion_trail()` | `tactical.rs` |
| `PlayDitty()` | tactrans.c | :67-85 | `play_ditty()` | `tactical.rs` |
| `StopDitty()` | tactrans.c | :88-93 | `stop_ditty()` | `tactical.rs` |
| `DittyPlaying()` | tactrans.c | :96-100 | `ditty_playing()` | `tactical.rs` |
| `StopAllBattleMusic()` | tactrans.c | varies | `stop_all_battle_music()` | `tactical.rs` |
| `preprocess_dead_ship()` | tactrans.c | :280-285 | `preprocess_dead_ship()` | `tactical.rs` |
| `RecordShipDeath()` | tactrans.c | :678-700 | `record_ship_death()` | `tactical.rs` |
| `readyForBattleEnd()` | tactrans.c | :253-278 | `ready_for_battle_end()` | `tactical.rs` |
| `setMinShipLifeSpan()` | tactrans.c | varies | `set_min_ship_life_span()` | `tactical.rs` |
| `setMinStarShipLifeSpan()` | tactrans.c | varies | `set_min_starship_life_span()` | `tactical.rs` |
| `checkOtherShipLifeSpan()` | tactrans.c | :700-800 | `check_other_ship_life_span()` | `tactical.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `NETPLAY` | tactrans.c:108-251 (readyToEndCallback, readyToEnd2Callback, readyForBattleEndPlayer — full per-player handler readiness protocol with NetState transitions, frame count negotiation, waitReady/negotiateReady), tactrans.c:245-251 (battleEndReadyNetwork), tactrans.c:483-499 (new_ship: initBattleStateDataConnections, negotiateReadyConnections to NetState_interBattle checkpoint before ship selection), tactrans.c:510-521 (new_ship: negotiateReadyConnections to NetState_inBattle after ship spawn) | Netplay: multi-step readiness protocol — (1) Ready handshake to start ending, (2) frame count exchange and continuation to max frame, (3) Ready handshake to confirm end. Non-netplay: ditty check only. Rust: feature-gated `netplay` module with state machine |
| `NETPLAY_DEBUG` | tactrans.c:127-128, 144-146 | Debug fprintf for netplay synchronisation. Rust: `tracing::debug!()` behind netplay feature |
| `DEMO_MODE` | tactrans.c:258-264 (readyForBattleEnd: `#if DEMO_MODE` returns true immediately, skipping DittyPlaying check for journal replay frame accuracy) | Demo mode: always ready. Rust: feature flag or cfg |
| `SUPER_MELEE` | tactrans.c:698-699 (RecordShipDeath: calls MeleeShipDeath(deadStarShip) when activity == SUPER_MELEE) | SuperMelee: extra death recording side effect. Rust: activity check |

### Integration points
- P03 `process_loop.rs`: alloc_element(), free_element(), setup_element()
- P07 `ship_runtime.rs`: animation_preprocess() (used by explosion fragments)
- P08 `ship_runtime.rs`: get_next_starship() (used by new_ship)
- P06 `c_bridge.rs`: TFB_Random, play_sound_effect, stop_music, process_sound, etc.
- Phase 1 `element.rs`: Element struct, ElementFlags
- Phase 1 `display_list.rs`: iteration, handles
- Phase 1 `tactical.rs` (types): death pipeline constants (NUM_EXPLOSION_FRAMES, MIN_DITTY_FRAME_COUNT)

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/tactical-transitions.md`: ship_death, explosion_preprocess, cleanup_dead_ship, new_ship sections

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 17 functions implemented in `tactical.rs`
- [ ] Phase 1 type definitions in tactical.rs preserved
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Death chain: ship_death sequence (tactrans.c:729-749): GetElementStarShip → StopAllBattleMusic → clear PLAY_VICTORY_DITTY → StartShipExplosion → FindAliveStarShip → SetWinnerStarShip → RecordShipDeath (all 7 steps, line-verified)
- [ ] Explosion: explicit switch schedule (1/2/3 fragments per tick), tick 15 hides ship prim (falls through to default i=3), tick 25 sets preprocess_func=NULL then falls through to case 0-2/20-24 group (i=1, spawns 1 fragment), 36-frame duration
- [ ] Cleanup: crew writeback, owned elements cleared (except crew with crew_preprocess), victory ditty, callbacks reassigned, multi-step life_span: MusicStarted?MIN_DITTY_FRAME_COUNT:1, winner override +1, then unconditional ++life_span
- [ ] New ship: readyForBattleEnd check, stop music, free ship, GetNextStarShip, BattleSong restart
- [ ] Simultaneous death: winner +1 life_span, tie keeps other alive
- [ ] RecordShipDeath: skip battle_counter decrement if running away (mass_points > MAX_SHIP_MASS, already counted in DoRunAway); call MeleeShipDeath in SUPER_MELEE regardless of flee status
- [ ] readyForBattleEnd: non-netplay=!DittyPlaying(), DEMO_MODE=always true, netplay=!DittyPlaying() AND per-player battleEndReady handlers (network uses multi-step NetState protocol with frame count exchange)
- [ ] StopAllBattleMusic = StopDitty + StopMusic only (NOT StopSound); ship_death calls StopAllBattleMusic to prevent wrong ditty carryover; new_ship calls StopDitty + StopMusic + StopSound as three separate calls (also needs to stop SFX for clean ship selection)
- [ ] Ion trail: spawn behind ship, cycle animation/fade
- [ ] Ditty: play/stop/playing/stop_all
- [ ] MIN_DITTY_FRAME_COUNT = (ONE_SECOND * 3) / BATTLE_FRAME_RATE
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/tactical.rs
```

## Success Criteria
- [ ] All 17 functions implemented and tested
- [ ] 4-phase death chain verified end-to-end
- [ ] Simultaneous death handling correct
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/tactical.rs`
- blocking issues: death chain timing, explosion fragment randomization, netplay readiness

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P09.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P09
- timestamp
- files changed: tactical.rs
- tests added/updated
- verification outputs
- semantic verification summary
