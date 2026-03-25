# Phase 08a: Ship Spawn + Init Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P08a`

## Prerequisites
- Required: Phase 08 (Ship Spawn + Init) completed
- Expected artifacts: spawn_ship, get_next_starship, get_initial_starships in `ship_runtime.rs`

## Structural Verification Checklist
- [ ] 3 functions present: spawn_ship, get_next_starship, get_initial_starships
- [ ] Plan/requirement traceability markers present
- [ ] No new module files

## Semantic Verification Checklist (Mandatory — Most Important)

### spawn_ship equivalence with C (ship.c:525-593)
- [ ] **Allocation**: AllocElement called; returns None on failure
- [ ] **Descriptor init**: mass, hit_points, image from RaceDescPtr; max_crew, max_energy from characteristics
- [ ] **Callback assignment**: preprocess_func = ship_preprocess, postprocess_func = ship_postprocess, collision_func = ship_collision, death_func = ship_death (P09)
- [ ] **State flags**: APPEARING | PLAYER_SHIP | IGNORE_SIMILAR (exactly these flags)
- [ ] **Life span**: NORMAL_LIFE
- [ ] **Random position**: TFB_Random for x and y coordinates
- [ ] **Gravity avoidance**: retry position if distance to any gravity well < GRAVITY_THRESHOLD
- [ ] **HyperSpace position**: single-ship spawn uses specific HyperSpace position (not random)
- [ ] **Display list insertion**: element inserted into disp_q via push_back

### get_next_starship equivalence with C (ship.c:475-525)
- [ ] **Queue walk**: starts from successor of `last` (or head if None)
- [ ] **Ship availability**: checks crew_level > 0 AND not already spawned
- [ ] **Ship loading**: loads ship resources if not already loaded
- [ ] **SUPER_MELEE recycling**: when tail reached, wraps to head for infinite fleet
- [ ] **Encounter finite**: returns None when fleet exhausted
- [ ] **Element reuse**: if starship already has an element, reuse it (don't re-alloc)
- [ ] **Binding**: element bound to queue entry via SetElementStarShip

### get_initial_starships equivalence with C (ship.c:530-593)
- [ ] **SUPER_MELEE path**: calls pick-melee UI bridge for ship selection
- [ ] **Encounter path**: auto-selects first available ship per side
- [ ] **Return value**: number of ships initially spawned
- [ ] **HyperSpace path**: single ship with no selection (GetNextStarShip with null)

## Branch-Parity Verification
- [ ] `SUPER_MELEE`: infinite fleet recycling vs finite encounter fleet
- [ ] `IN_ENCOUNTER`: encounter-specific loading vs SuperMelee selection
- [ ] `inHyperSpace()`: single-ship spawn path

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ship_runtime.rs
```

## Pass/Fail Gate Criteria
- **PASS:** spawn_ship allocates, inits, positions, and inserts correctly. Queue traversal with recycling correct. Initial selection dispatches correct path per game mode. No TODO/FIXME/HACK.
- **FAIL:** Spawn doesn't avoid gravity wells. Queue recycling missing for SUPER_MELEE. Callbacks not assigned. HyperSpace path not handled.
