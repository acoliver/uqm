# Phase 10: Flee + Warp + Winner

## Phase ID
`PLAN-20260320-BATTLEPT2.P10`

## Prerequisites
- Required: Phase 09a (Death + Explosion Verification) completed with PASS
- Expected files: `tactical.rs` with P09 functions (17 death/explosion functions)
- Expected artifacts: Death callback chain, explosion, cleanup verified

## Requirements Implemented (Expanded)

### REQ: Flee eligibility (battle/requirements.md §Flee eligibility)
**Requirement text**: RunAwayAllowed determines whether escape is available. Flee is blocked in certain encounter types and when only one ship is available.

Behavior contract:
- GIVEN: A player with flee eligibility
- WHEN: BATTLE_ESCAPE input detected during ProcessInput
- THEN: DoRunAway is invoked if RunAwayAllowed returns true

### REQ: Flee animation (battle/requirements.md §Flee animation)
**Requirement text**: flee_preprocess executes a 20-color pulse cycle on the fleeing ship. When the pulse completes, the ship warps out.

Behavior contract:
- GIVEN: A ship that has initiated flee
- WHEN: flee_preprocess executes per frame
- THEN: 20-color pulse cycle applied; after completion, ship warps out (mass_points set > MAX_SHIP_MASS to mark as fleeing)

### REQ: Warp transitions (battle/requirements.md §Warp transitions)
**Requirement text**: ship_transition handles warp-in (APPEARING to active) and warp-out (active to gone). Warp-in is the spawn materialization. Warp-out is the flee departure.

Behavior contract:
- GIVEN: A ship with APPEARING flag
- WHEN: ship_transition is called
- THEN: Warp-in animation sequence plays (ship materializes)

- GIVEN: A fleeing ship
- WHEN: ship_transition warp-out phase completes
- THEN: Ship element removed, crew preserved for fleet tracking

### REQ: Winner determination (battle/requirements.md §Winner determination)
**Requirement text**: FindAliveStarShip, SetWinnerStarShip, GetWinnerStarShip, ResetWinnerStarShip track the battle winner.

Behavior contract:
- GIVEN: One ship alive and one dead
- WHEN: FindAliveStarShip is called
- THEN: Returns the surviving ship's starship pointer

- GIVEN: Simultaneous death (both die same frame)
- WHEN: SetWinnerStarShip is called for first winner
- THEN: First winner preserved (second call is a no-op if winner already set)

### REQ: Opponent alive check (battle/requirements.md §Opponent alive check)
**Requirement text**: OpponentAlive checks whether any opposing ship is still active.

Behavior contract:
- GIVEN: The current player has an active ship
- WHEN: OpponentAlive is called
- THEN: Returns true if any ship belonging to the OTHER player is alive and active

### REQ: DoRunAway (battle/requirements.md §Flee dispatch)
**Requirement text**: DoRunAway initiates the flee sequence for the specified player's ship. Sets mass_points to signal fleeing.

Behavior contract:
- GIVEN: A player wants to flee
- WHEN: DoRunAway is called
- THEN: Ship's mass_points set > MAX_SHIP_MASS (flee signal), flee_preprocess installed, life_span extended

## Implementation Tasks

### Files to modify

- `rust/src/battle/tactical.rs` — Add flee + warp + winner logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P10`
  - marker: `@requirement REQ-FLEE, REQ-WARP, REQ-WINNER, REQ-OPPONENT-ALIVE`
  - Contents to add:
    - `fn flee_preprocess(element: &mut Element)` — 20-color pulse cycle matching tactrans.c flee_preprocess. Cycles through color table per frame, sets CHANGING, when complete triggers warp-out.
    - `pub fn do_run_away(player_nr: u8)` — Flee initiation matching tactrans.c DoRunAway. Finds player's ship element, sets mass_points > MAX_SHIP_MASS (flee marker), installs flee_preprocess, extends life_span.
    - `pub fn ship_transition(element: &mut Element)` — Warp in/out matching tactrans.c ship_transition. Warp-in: materialization animation from small to full size. Warp-out: dematerialization, then removal.
    - `pub fn find_alive_starship(player_nr: u8) -> Option<StarshipRef>` — Alive-ship search matching tactrans.c FindAliveStarShip. Iterates display list for PLAYER_SHIP with correct player, mass_points ≤ MAX_SHIP_MASS+1, crew_level > 0. Pkunk reincarnation: mass_points == MAX_SHIP_MASS+1.
    - `pub fn opponent_alive(player_nr: u8) -> bool` — Opponent check matching tactrans.c OpponentAlive. Iterates display list for PLAYER_SHIP belonging to OTHER player with crew > 0.
    - `pub fn reset_winner_starship()` — Clear winner state matching tactrans.c ResetWinnerStarShip.
    - `pub fn get_winner_starship() -> Option<StarshipRef>` — Get current winner matching tactrans.c GetWinnerStarShip.
    - `pub fn set_winner_starship(starship: &Starship)` — Set winner matching tactrans.c SetWinnerStarShip. Sets PLAY_VICTORY_DITTY flag; preserves first winner (second call no-op if already set).

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `flee_preprocess()` | tactrans.c | varies | `flee_preprocess()` | `tactical.rs` |
| `ship_transition()` | tactrans.c | varies | `ship_transition()` | `tactical.rs` |
| `DoRunAway()` | tactrans.c / battle.c | varies | `do_run_away()` | `tactical.rs` |
| `FindAliveStarShip()` | tactrans.c | :560-620 | `find_alive_starship()` | `tactical.rs` |
| `OpponentAlive()` | tactrans.c | :40-65 | `opponent_alive()` | `tactical.rs` |
| `ResetWinnerStarShip()` | tactrans.c | :715-725 | `reset_winner_starship()` | `tactical.rs` |
| `GetWinnerStarShip()` | tactrans.c | :728-735 | `get_winner_starship()` | `tactical.rs` |
| `SetWinnerStarShip()` | tactrans.c | :620-670 | `set_winner_starship()` | `tactical.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `IN_ENCOUNTER` | DoRunAway/RunAwayAllowed | Encounter-specific flee restrictions |
| `IN_LAST_BATTLE` | RunAwayAllowed | Final battle: flee may be blocked |
| `SUPER_MELEE` | winner tracking | SuperMelee-specific winner behavior |

### Integration points
- P09 `tactical.rs`: ship_death, cleanup_dead_ship (winner tracking)
- P06 `c_bridge.rs`: color table lookups, animation primitives
- P03 `process_loop.rs`: remove_element
- Phase 1 `display_list.rs`: iteration
- Phase 1 `element.rs`: ElementFlags, mass_points, crew_level

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/tactical-transitions.md`: flee_preprocess, ship_transition, DoRunAway, FindAliveStarShip, winner sections

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 8 functions implemented in `tactical.rs`
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Flee eligibility: correct conditions checked
- [ ] DoRunAway: mass_points set > MAX_SHIP_MASS, flee_preprocess installed, life_span extended
- [ ] flee_preprocess: 20-color pulse cycle, CHANGING flag, warp-out trigger at completion
- [ ] ship_transition warp-in: materialization from small to full, APPEARING cleared at end
- [ ] ship_transition warp-out: dematerialization, element removal, crew preserved
- [ ] FindAliveStarShip: PLAYER_SHIP + correct player + mass_points ≤ MAX_SHIP_MASS+1 + crew > 0
- [ ] FindAliveStarShip Pkunk: mass_points == MAX_SHIP_MASS+1 treated as alive
- [ ] OpponentAlive: checks OTHER player's ships
- [ ] SetWinnerStarShip: PLAY_VICTORY_DITTY set; first winner preserved
- [ ] ResetWinnerStarShip/GetWinnerStarShip: correct global state management
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/tactical.rs
```

## Success Criteria
- [ ] All 8 functions implemented and tested
- [ ] Flee sequence end-to-end verified
- [ ] Winner determination correct in all scenarios
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/tactical.rs`
- blocking issues: Color pulse cycle timing, warp animation sprite lookup

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P10.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P10
- timestamp
- files changed: tactical.rs
- tests added/updated
- verification outputs
- semantic verification summary
