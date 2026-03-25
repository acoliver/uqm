# Phase 08: Ship Spawn + Init

## Phase ID
`PLAN-20260320-BATTLEPT2.P08`

## Prerequisites
- Required: Phase 07a (Ship Runtime Verification) completed with PASS
- Expected files: `ship_runtime.rs` with P07 functions
- Expected artifacts: ship_preprocess, ship_postprocess, inertial_thrust, animation_preprocess, ship_collision verified

## Requirements Implemented (Expanded)

### REQ: Ship spawning (battle/requirements.md §Ship spawning)
**Requirement text**: spawn_ship (ship.c:393-515, static BOOLEAN) allocates a ship element, initializes it from the race descriptor, sets callbacks, positions it based on context, and sets APPEARING. It does NOT handle warp-in animation or side-based entry — that is done by ship_transition() called from ship_preprocess's APPEARING path.

Behavior contract:
- GIVEN: A Starship descriptor (StarShipPtr)
- WHEN: spawn_ship() is called (ship.c:393-515)
- THEN: Full initialization sequence:
  1. load_ship(SpeciesID, TRUE) → RDPtr (return FALSE if fails)
  2. Zero starship input/status fields (ship_input_state, cur_status_flags, old_status_flags)
  3. Crew level handling: if IN_ENCOUNTER or IN_LAST_BATTLE, copy starship->crew_level to descriptor (except SIS with crew_level==0), cap at max_crew
  4. Zero energy/weapon/special counters
  5. Allocate element (reuse hShip if exists, else AllocElement + InsertElement at head)
  6. Init element: playerNr, crew_level=0, mass_points from characteristics, state_flags=APPEARING|PLAYER_SHIP|IGNORE_SIMILAR, turn_wait=0, thrust_wait=0, life_span=NORMAL_LIFE, colorCycleIndex=0, PrimType=STAMP_PRIM, image.farray=ship_data.ship
  7. **Position depends on context** (ship.c:459-499):
     - **Sa-Matra** (NPC + IN_LAST_BATTLE): facing=0, position=center (LOG_SPACE_WIDTH/2, LOG_SPACE_HEIGHT/2), life_span incremented by 1
     - **HyperSpace** (inHQSpace()): facing from GLOBAL(ShipFacing) adjusted by -1, single ship
     - **Normal encounter**: random facing via TFB_Random(), random position via `do { WRAP_X(DISPLAY_ALIGN_X(TFB_Random())), WRAP_Y(DISPLAY_ALIGN_Y(TFB_Random())) } while (CalculateGravity(element) || TimeSpaceMatterConflict(element))` — retries until position avoids gravity wells AND other ships/objects
  8. Set callbacks: preprocess_func=ship_preprocess, postprocess_func=ship_postprocess, death_func=ship_death, collision_func=collision
  9. ZeroVelocityComponents, SetElementStarShip, hTarget=0
  10. Returns hShip != 0 (BOOLEAN success/failure)

### REQ: Ship queue management (battle/requirements.md §Ship queue management)
**Requirement text**: GetNextStarShip (ship.c:518-552, BOOLEAN return) selects and spawns the next ship. It uses GetEncounterStarShip to pick the next ship from the queue, handles element recycling (transfers hShip from last to new), calls spawn_ship, and returns BOOLEAN success/failure. It does NOT return a ship reference — it returns whether spawning succeeded.

Behavior contract:
- GIVEN: A player has ships remaining in their fleet queue
- WHEN: GetNextStarShip(LastStarShipPtr, which_side) is called (ship.c:518-552)
- THEN: GetEncounterStarShip finds next ship → if same as last (infinite recycling), clear LastStarShipPtr; else transfer hShip from last → call spawn_ship(StarShipPtr) → if spawn fails return FALSE → clear LastStarShipPtr->hShip → return hBattleShip != 0. Returns BOOLEAN (true=ship spawned, false=no ship available or spawn failed).

- GIVEN: A SUPER_MELEE game with infinite fleet
- WHEN: All ships are exhausted
- THEN: GetEncounterStarShip handles recycling; GetNextStarShip detects recycled ship (StarShipPtr == LastStarShipPtr) and clears LastStarShipPtr to 0

### REQ: Initial ship selection (battle/requirements.md §Initial ship selection)
**Requirement text**: GetInitialStarShips either presents a ship selection menu (SuperMelee) or auto-selects from encounter fleet.

Behavior contract:
- GIVEN: A SuperMelee game starting
- WHEN: GetInitialStarShips is called
- THEN: Pick-melee selection screen is presented

- GIVEN: An encounter starting
- WHEN: GetInitialStarShips is called
- THEN: Ships are auto-selected from encounter fleet (first available for each side)

## Implementation Tasks

### Files to modify

- `rust/src/battle/ship_runtime.rs` — Add spawn + queue logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P08`
  - marker: `@requirement REQ-SHIP-SPAWN, REQ-SHIP-QUEUE`
  - Contents to add:
    - `pub fn spawn_ship(starship: &mut Starship) -> bool` — Full spawn matching ship.c:393-515 (static BOOLEAN). load_ship → init starship fields → crew level handling (IN_ENCOUNTER/IN_LAST_BATTLE) → zero counters → alloc/reuse element → init element (mass, state_flags=APPEARING|PLAYER_SHIP|IGNORE_SIMILAR, life_span=NORMAL_LIFE) → context-dependent positioning: Sa-Matra (center, facing=0, life_span+1), HyperSpace (facing from GLOBAL(ShipFacing)), normal (random x/y via TFB_Random with retry loop avoiding CalculateGravity and TimeSpaceMatterConflict) → set callbacks (preprocess=ship_preprocess, postprocess=ship_postprocess, collision=collision, death_func=ship_death) → zero velocity → bind starship. Returns bool (hShip != 0).
    - `pub fn get_next_starship(last: Option<&mut Starship>, which_side: u8) -> bool` — Ship selection + spawn matching ship.c:518-552 (BOOLEAN return). Calls GetEncounterStarShip to find next ship handle → detects recycling (same ship returned) → transfers hShip from last to new StarShipPtr → calls spawn_ship → on failure returns false → clears LastStarShipPtr->hShip → returns hBattleShip != 0. Returns BOOLEAN success, NOT a ship reference.
    - `pub fn get_initial_starships() -> bool` — Initial selection matching ship.c:554-591. SUPER_MELEE: GetInitialMeleeStarShips for pick UI, then spawn_ship per player in GetPlayerOrder sequence. Non-SuperMelee: loop GetNextStarShip(NULL, i-1) for each player. Returns TRUE if all spawns succeed.

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `spawn_ship()` | ship.c | :393-515 | `spawn_ship()` | `ship_runtime.rs` |
| `GetNextStarShip()` | ship.c | :518-552 | `get_next_starship()` | `ship_runtime.rs` |
| `GetInitialStarShips()` | ship.c | :554-591 | `get_initial_starships()` | `ship_runtime.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `SUPER_MELEE` | ship.c GetInitialStarShips, GetNextStarShip | SuperMelee: pick-melee UI, infinite recycling. Encounter: auto-select, finite fleet. |
| `IN_ENCOUNTER` | ship.c GetInitialStarShips | Encounter-specific ship loading path |
| `inHyperSpace()` | ship.c spawn_ship | HyperSpace spawn: single ship, special position |

### Integration points
- P03 `process_loop.rs`: alloc_element(), setup_element()
- P07 `ship_runtime.rs`: ship_preprocess, ship_postprocess, ship_collision (used as callback values)
- P09 `tactical.rs`: ship_death (used as death_func callback) — forward reference
- P06 `c_bridge.rs`: TFB_Random, gravity well checks, pick-melee bridge
- Phase 1 `display_list.rs`: push_back (insert into display list)
- Phase 1 `element.rs`: Element initialization

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/ship-runtime.md`: spawn_ship, GetNextStarShip, GetInitialStarShips sections

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] 3 functions added to `ship_runtime.rs`
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] spawn_ship: load_ship → init starship fields → crew handling → alloc/reuse element → returns bool
- [ ] spawn_ship: state_flags = APPEARING | PLAYER_SHIP | IGNORE_SIMILAR
- [ ] spawn_ship: callbacks set (preprocess=ship_preprocess, postprocess=ship_postprocess, collision=collision, death_func=ship_death)
- [ ] spawn_ship: life_span = NORMAL_LIFE; Sa-Matra gets life_span+1
- [ ] spawn_ship positioning: Sa-Matra=center+facing 0, HyperSpace=facing from GLOBAL(ShipFacing), normal=random TFB_Random with retry loop (CalculateGravity || TimeSpaceMatterConflict)
- [ ] get_next_starship: returns BOOLEAN (success/failure), NOT a ship reference
- [ ] get_next_starship: GetEncounterStarShip → hShip transfer → spawn_ship → clear last hShip
- [ ] get_next_starship: detects recycling (same StarShipPtr returned)
- [ ] get_initial_starships: SUPER_MELEE → GetInitialMeleeStarShips + spawn per player
- [ ] get_initial_starships: non-SUPER_MELEE → GetNextStarShip(NULL, i-1) per player
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ship_runtime.rs
```

## Success Criteria
- [ ] All 3 functions implemented and tested
- [ ] Spawn matches C behavior exactly
- [ ] Queue traversal matches C
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/ship_runtime.rs`
- blocking issues: ship_death forward reference (P09 not yet implemented), pick-melee bridge

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P08.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P08
- timestamp
- files changed: ship_runtime.rs
- tests added/updated
- verification outputs
- semantic verification summary
