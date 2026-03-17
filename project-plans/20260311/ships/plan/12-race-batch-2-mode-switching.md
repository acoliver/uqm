# Phase 12: Race Batch 2 — Mode-Switching & Complex State Ships (8 Races)

## Phase ID
`PLAN-20260314-SHIPS.P12`

## Prerequisites
- Required: Phase 11a (Batch 1 Verification) completed and PASS
- Expected files: 8 race implementations from Batch 1, verified working

## Requirements Implemented (Expanded)

### Descriptor Mutation
**Requirement text**: Races that mutate their descriptor instance at runtime (changing characteristics, swapping callbacks, switching frame sets, altering collision behavior) shall continue to produce the same externally observable effects. The subsystem shall not restrict descriptor mutability.

### Private State
**Requirement text**: When a race implementation requires per-instance mutable state beyond the descriptor's standard fields, the subsystem shall support allocation and storage through the descriptor's opaque private-data slot. The shared runtime shall not interpret or depend on the contents.

### Hook Change
**Requirement text**: A race may change its own hooks on its descriptor instance during the ship's combat lifetime.

## Batch 2 Races

These 8 races have mode-switching, private data allocation, or complex state management:

### 1. Androsynth — `rust/src/ships/races/androsynth.rs`
- **C source**: `sc2/src/uqm/ships/androsyn/androsyn.c` (529 lines)
- **Ship cost**: 15 | **Crew**: 20 | **Energy**: 24
- **Weapon (Normal mode)**: Acid bubbles — seeking, bouncing projectiles
- **Special**: Toggle Blazer mode — transforms ship into comet form
- **Blazer mode**: Becomes a fast-moving damaging projectile itself, swaps collision, changes characteristics (thrust=60, turn_wait=1, mass=1)
- **Private data**: `SetCustomShipData()` / `GetCustomShipData()` — stores blazer state
- **Key complexity**: Mode switch mutates `characteristics`, swaps `ship_data.ship`↔`ship_data.special` frames, replaces `collision_func` with blazer collision
- **AI**: Mode-aware — enters blazer when close to enemy
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 2. Mmrnmhrm — `rust/src/ships/races/mmrnmhrm.rs`
- **C source**: `sc2/src/uqm/ships/mmrnmhrm/mmrnmhrm.c` (~400 lines)
- **Ship cost**: 19 | **Crew**: 20 | **Energy**: 10
- **X-Form**: Laser wing + fast turn | **Y-Form**: Twin missile + fast thrust
- **Special**: Transform between X and Y forms
- **Key complexity**: Full characteristics swap on transform (thrust, turn_wait, weapon, etc.), frame set swap
- **Private data**: Stores form state
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 3. Orz — `rust/src/ships/races/orz.rs`
- **C source**: `sc2/src/uqm/ships/orz/orz.c` (~500 lines)
- **Ship cost**: 23 | **Crew**: 16 | **Energy**: 20
- **Weapon**: Howitzer turret — rotating turret that aims independently
- **Special**: Space marine deployment — launches marines that board enemy ships
- **Key complexity**: Marines are independent elements that seek enemy, board on collision (crew steal), turret rotation independent of ship facing
- **Private data**: Marine tracking state
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 4. Pkunk — `rust/src/ships/races/pkunk.rs`
- **C source**: `sc2/src/uqm/ships/pkunk/pkunk.c` (~400 lines)
- **Ship cost**: 20 | **Crew**: 8 | **Energy**: 12
- **Weapon**: Triple spread shot — `initialize_bug_missile()`
- **Special**: Insult — taunts enemy, regenerates energy
- **Key complexity**: **Resurrection** — on death, 50% chance to respawn with full crew. Death callback must check resurrection before finalizing
- **Private data**: Resurrection state, insult tracking
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 5. Shofixti — `rust/src/ships/races/shofixti.rs`
- **C source**: `sc2/src/uqm/ships/shofixti/shofixti.c` (~350 lines)
- **Ship cost**: 5 | **Crew**: 6 | **Energy**: 4
- **Weapon**: Dart gun — simple short-range projectile
- **Special**: Glory Device — self-destruct with massive area damage. Damages ALL nearby ships including own side
- **Key complexity**: Self-destruct kills own ship, area damage calculation, affects friendly ships
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 6. Syreen — `rust/src/ships/races/syreen.rs`
- **C source**: `sc2/src/uqm/ships/syreen/syreen.c` (~350 lines)
- **Ship cost**: 13 | **Crew**: 12 | **Energy**: 16
- **Weapon**: Particle beam — short range
- **Special**: Siren Song — steals crew from nearby enemy ship, spawns crew elements that float to Syreen ship
- **Key complexity**: Crew transfer mechanic — enemy loses crew, floating crew elements are created that seek Syreen ship
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 7. Utwig — `rust/src/ships/races/utwig.rs`
- **C source**: `sc2/src/uqm/ships/utwig/utwig.c` (~350 lines)
- **Ship cost**: 22 | **Crew**: 20 | **Energy**: 10
- **Weapon**: Energy bolt — 6-shot spread
- **Special**: Absorption shield — absorbs incoming damage and converts to energy
- **Key complexity**: Shield collision override converts damage to energy gain. SHIELD_DEFENSE flag. Shield must be timed (on only during special press)
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P12`

### 8. Vux — `rust/src/ships/races/vux.rs`
- **C source**: `sc2/src/uqm/ships/vux/vux.c` (~400 lines)
- **Ship cost**: 12 | **Crew**: 20 | **Energy**: 40
- **Weapon**: Laser — short-range
- **Special**: Limpet — attaches to enemy ship, reduces max speed permanently
- **Key complexity**: **Warp-in advantage** — VUX spawns close to enemy at battle start. Limpet modifies target ship's characteristics (reduces max_thrust). Private data tracks limpet state
- **Private data**: Limpet tracking, warp-in setup
- marker: `@plan PLAN-20260314-SHIPS.P12`

## Implementation Tasks

### Files to create

For each race, create `rust/src/ships/races/<name>.rs`:
- Define `<Name>Ship` struct with appropriate private state fields
- Implement `ShipBehavior` trait with full behavior
- Port all weapon/special/AI logic from corresponding C file
- Ensure descriptor mutation patterns work correctly (Androsynth blazer, Mmrnmhrm transform)

### Files to modify

- `rust/src/ships/races/mod.rs`
  - Add modules for all 8 races

- `rust/src/ships/registry.rs`
  - Replace StubShip match arms for these 8 races with real implementations

### TDD approach per race
1. Test descriptor_template constants match C
2. Test weapon creation and projectile properties
3. Test special ability (mode switch / self-destruct / crew steal / etc.)
4. Test mode-switch characteristic mutation (Androsynth, Mmrnmhrm)
5. Test private data lifecycle (allocation in new(), cleanup in uninit())
6. Test AI basic decisions
7. Test edge cases (Pkunk resurrection, Shofixti self-damage, VUX warp-in)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] 8 race files created under `races/`
- [ ] `races/mod.rs` declares all 8 new races
- [ ] `registry.rs` uses real impls for all 16 races total (Batch 1 + 2)
- [ ] Descriptor constants match C exactly
- [ ] Private data stored as struct fields (not raw pointers)
- [ ] Mode-switching races correctly mutate characteristics

## Semantic Verification Checklist (Mandatory)
- [ ] Androsynth: blazer mode changes characteristics, swaps frames, collision override works, returns to normal
- [ ] Mmrnmhrm: X↔Y transform swaps complete characteristics set
- [ ] Orz: marines spawn as independent elements, board enemy on collision
- [ ] Pkunk: resurrection check on death, 50% chance, full crew restore
- [ ] Shofixti: glory device damages all nearby ships including allies
- [ ] Syreen: siren song creates crew elements that seek Syreen ship
- [ ] Utwig: shield absorbs damage → energy gain, timed to special input
- [ ] Vux: limpet reduces target max speed, warp-in places ship near enemy
- [ ] All 8: private data properly managed (no leaks, cleaned up in uninit)
- [ ] All 8: AI makes mode-appropriate decisions
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/races/androsynth.rs rust/src/ships/races/mmrnmhrm.rs rust/src/ships/races/orz.rs rust/src/ships/races/pkunk.rs rust/src/ships/races/shofixti.rs rust/src/ships/races/syreen.rs rust/src/ships/races/utwig.rs rust/src/ships/races/vux.rs
```

## Success Criteria
- [ ] 8 mode-switching race implementations compile and pass tests
- [ ] Mode-switch/mutation patterns work correctly
- [ ] Private data lifecycle tests pass
- [ ] Edge case tests pass (resurrection, self-destruct, limpet, warp-in)
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/races/`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P12.md`
