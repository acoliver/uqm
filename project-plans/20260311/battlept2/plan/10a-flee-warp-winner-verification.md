# Phase 10a: Flee + Warp + Winner Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P10a`

## Prerequisites
- Required: Phase 10 (Flee + Warp + Winner) completed
- Expected artifacts: 8 functions in `tactical.rs`

## Structural Verification Checklist
- [ ] All 8 functions present: flee_preprocess, do_run_away, ship_transition, find_alive_starship, opponent_alive, reset_winner_starship, get_winner_starship, set_winner_starship
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory — Most Important)

### flee_preprocess equivalence with C (tactrans.c)
- [ ] **20-color pulse**: cycles through color table entries frame by frame
- [ ] **CHANGING flag**: set each frame to trigger visual update
- [ ] **Completion**: after full cycle (20 frames), triggers warp-out via ship_transition
- [ ] **Test**: verify pulse cycle count and completion trigger

### DoRunAway equivalence with C (tactrans.c/battle.c)
- [ ] Finds player's ship element in display list
- [ ] Sets mass_points > MAX_SHIP_MASS (flee signal — distinguishes fleeing from dead)
- [ ] Installs flee_preprocess as preprocess_func
- [ ] Extends life_span to cover flee animation duration
- [ ] Preserves original crew_level (fleeing ship keeps crew)

### ship_transition warp-in (tactrans.c)
- [ ] Materialization: sprite starts small, grows to full size over animation frames
- [ ] APPEARING flag cleared at animation end
- [ ] Ship becomes active (collidable, velocity-responsive) after warp-in completes
- [ ] Sound: warp-in sound effect played

### ship_transition warp-out (tactrans.c)
- [ ] Dematerialization: sprite shrinks from full to zero
- [ ] Element removed from display list after animation
- [ ] Crew preserved in starship descriptor (not lost)
- [ ] Sound: warp-out sound effect played

### FindAliveStarShip equivalence with C (tactrans.c:560-620)
- [ ] Iterates display list head to tail
- [ ] Matches: PLAYER_SHIP flag + correct playerNr
- [ ] Alive criteria: mass_points ≤ MAX_SHIP_MASS + 1 AND crew_level > 0
- [ ] **Pkunk reincarnation**: mass_points == MAX_SHIP_MASS + 1 treated as alive (special case)
- [ ] Returns None if no alive ship found

### OpponentAlive equivalence with C (tactrans.c:40-65)
- [ ] Searches for PLAYER_SHIP with playerNr != given player
- [ ] Checks crew_level > 0 (active crew = alive)
- [ ] Returns bool

### Winner state management
- [ ] **SetWinnerStarShip**: sets PLAY_VICTORY_DITTY on winning starship; if winner already set, second call is no-op (first winner preserved)
- [ ] **GetWinnerStarShip**: returns current winner or None
- [ ] **ResetWinnerStarShip**: clears winner state
- [ ] Test: simultaneous death → first SetWinnerStarShip wins

## Branch-Parity Verification
- [ ] `IN_ENCOUNTER`: flee restrictions in encounter mode (specific encounters may block flee)
- [ ] `IN_LAST_BATTLE`: flee blocked in final battle
- [ ] `SUPER_MELEE`: winner tracking differences in SuperMelee mode

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/tactical.rs
```

## Pass/Fail Gate Criteria
- **PASS:** Flee sequence: DoRunAway → flee_preprocess 20-frame pulse → warp-out complete. Warp-in materialization correct. Winner determination first-wins-preserved. FindAliveStarShip handles Pkunk. No TODO/FIXME/HACK.
- **FAIL:** Flee doesn't set mass_points > MAX_SHIP_MASS. Pulse wrong frame count. Warp-in doesn't clear APPEARING. Winner overwritten by second call. Pkunk reincarnation not handled.
