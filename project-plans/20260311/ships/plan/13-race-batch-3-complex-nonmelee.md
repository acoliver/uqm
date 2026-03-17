# Phase 13: Race Batch 3 — Complex & Non-Melee Ships (12 Races)

## Phase ID
`PLAN-20260314-SHIPS.P13`

## Prerequisites
- Required: Phase 12a (Batch 2 Verification) completed and PASS
- Expected files: 16 race implementations from Batches 1 and 2, SIS/module-state bridge prerequisites already established in P03.5/P08/P09

## Requirements Implemented (Expanded)

### Non-Melee Ship Runtime
**Requirement text**: Non-melee ships shall use the same descriptor structure, two-tier loading mechanism, spawn sequence, per-frame pipeline, and behavioral hook dispatch as melee ships. Non-melee ships shall not appear in the master ship catalog. The subsystem shall not assume all ships conform to a single weapon/special pattern.

### Roster Completeness
**Requirement text**: The end-state subsystem shall include every melee-eligible race and every non-melee ship. No ships shall be removed or added.

### Complete Roster Preservation
**Requirement text**: Non-melee ships shall participate in the ship runtime with their established behavioral properties, including the player flagship's configurable loadout, final-battle opponent behavior, and probe behavior.

## Batch 3 Races

These 12 races include the most complex melee ships and all 3 non-melee ships:

### Melee Ships (9 races)

### 1. Chmmr — `rust/src/ships/races/chmmr.rs`
- **C source**: `sc2/src/uqm/ships/chmmr/chmmr.c` (~600 lines)
- **Ship cost**: 30 | **Crew**: 42 | **Energy**: 30
- **Weapon**: Photon crystal — high-energy beam
- **Special**: Tractor beam — pulls enemy toward Chmmr
- **Key complexity**: **ZapSat satellites** — 3 independent orbiting satellite elements that fire at nearby enemies. Satellites persist across frames, need tracking. Tractor beam modifies enemy velocity
- **Private data**: Satellite element handles, ZapSat state
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 2. Chenjesu — `rust/src/ships/races/chenjesu.rs`
- **C source**: `sc2/src/uqm/ships/chenjesu/chenjesu.c` (~500 lines)
- **Ship cost**: 28 | **Crew**: 36 | **Energy**: 30
- **Weapon**: Crystal shard — fragments on impact into seeking sub-shards
- **Special**: DOGI (De-energizing Offensive Guided Interceptor) — slow-moving seeking mine
- **Key complexity**: Crystal shard fragmentation (projectile splits into multiple sub-projectiles on impact), DOGI tracking
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 3. Mycon — `rust/src/ships/races/mycon.rs`
- **C source**: `sc2/src/uqm/ships/mycon/mycon.c` (~400 lines)
- **Ship cost**: 21 | **Crew**: 20 | **Energy**: 42
- **Weapon**: Homing plasmoid — slow homing projectile that grows larger over time
- **Special**: Crew regeneration — regenerates crew over time (CREW_IMMUNE flag interaction)
- **Key complexity**: Plasmoid homing behavior, growth/size change during flight, crew regen mechanic
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 4. Melnorme — `rust/src/ships/races/melnorme.rs`
- **C source**: `sc2/src/uqm/ships/melnorme/melnorme.c` (~500 lines)
- **Ship cost**: 18 | **Crew**: 20 | **Energy**: 42
- **Weapon**: Charge-up shot — power increases the longer WEAPON is held
- **Special**: Confusion pulse — causes enemy controls to reverse
- **Key complexity**: Weapon charging mechanic (preprocess tracks charge time, postprocess fires proportional shot), confusion modifies target's input processing
- **Private data**: Charge state
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 5. Umgah — `rust/src/ships/races/umgah.rs`
- **C source**: `sc2/src/uqm/ships/umgah/umgah.c` (~400 lines)
- **Ship cost**: 7 | **Crew**: 10 | **Energy**: 30
- **Weapon**: Antimatter cone — short-range forward cone of damage
- **Special**: Retro zip — backward thrust/teleport (unique movement)
- **Key complexity**: Cone weapon area calculation, zip modifies velocity for rapid backward movement
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 6. Ur-Quan — `rust/src/ships/races/urquan.rs`
- **C source**: `sc2/src/uqm/ships/urquan/urquan.c` (~500 lines)
- **Ship cost**: 30 | **Crew**: 42 | **Energy**: 42
- **Weapon**: Fusion blast — powerful projectile
- **Special**: Fighter launch — launches autonomous fighter elements that seek and attack enemy
- **Key complexity**: Fighters are independent elements with their own AI, lifetime, and damage. Multiple fighters can be active simultaneously. Fighter recall/management
- **Private data**: Fighter element tracking
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 7. Kohr-Ah / Black Ur-Quan — `rust/src/ships/races/black_urquan.rs`
- **C source**: `sc2/src/uqm/ships/blackurq/blackurq.c` (~500 lines)
- **Ship cost**: 30 | **Crew**: 42 | **Energy**: 42
- **Weapon**: Spinning blade — boomerang-style projectile that returns, continues doing damage on return path
- **Special**: F.R.I.E.D. — expanding ring of fire that damages all ships in radius
- **Key complexity**: Blade return path calculation (boomerang trajectory), FRIED expanding ring with area damage
- **Private data**: Blade tracking, FRIED state
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 8. Slylandro — `rust/src/ships/races/slylandro.rs`
- **C source**: `sc2/src/uqm/ships/slylandr/slylandr.c` (~300 lines)
- **Ship cost**: 17 | **Crew**: 12 | **Energy**: 20
- **Weapon**: Lightning bolt — seeking energy weapon
- **Special**: Harvest — absorbs nearby space debris for energy
- **Key complexity**: Unique ship that rotates and moves differently (always moves), lightning seeking behavior
- **Private data**: None significant
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 9. ZoqFotPik — `rust/src/ships/races/zoqfotpik.rs`
- **C source**: `sc2/src/uqm/ships/zoqfot/zoqfot.c` (~350 lines)
- **Ship cost**: 6 | **Crew**: 10 | **Energy**: 10
- **Weapon**: Anti-matter spray — short-range
- **Special**: Tongue grab — tongue extends and pulls enemy close for damage (stinger attack)
- **Key complexity**: Tongue is a projectile that, on hit, pulls enemy ship toward ZoqFotPik and deals damage. Two-part attack (tongue + stinger)
- **Private data**: Tongue/stinger state
- marker: `@plan PLAN-20260314-SHIPS.P13`

### Non-Melee Ships (3 races)

### 10. SIS Ship (Player Flagship) — `rust/src/ships/races/sis_ship.rs`
- **C source**: `sc2/src/uqm/ships/sis_ship/sis_ship.c` (~600 lines)
- **Ship cost**: N/A (non-melee) | **Crew**: variable | **Energy**: variable
- **Key complexity**: **Configurable loadout** — weapon and special are determined by campaign module slots, not static template. Multiple weapon/special options. Must read campaign state to configure. Characteristics vary based on installed thrusters/turning jets
- **Private data**: Module configuration
- **Non-melee**: Excluded from catalog, special spawn path for hyperspace/encounters
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 11. Sa-Matra — `rust/src/ships/races/samatra.rs`
- **C source**: `sc2/src/uqm/ships/lastbat/lastbat.c` (~600 lines)
- **Ship cost**: N/A (non-melee) | **Crew**: special | **Energy**: special
- **Key complexity**: Final battle opponent. Multiple weapon systems. Unique death sequence. May have phase-based combat
- **Private data**: Phase/state tracking
- **Non-melee**: Excluded from catalog, special spawn for final battle only
- marker: `@plan PLAN-20260314-SHIPS.P13`

### 12. Ur-Quan Probe — `rust/src/ships/races/probe.rs`
- **C source**: `sc2/src/uqm/ships/probe/probe.c` (~200 lines)
- **Ship cost**: N/A (non-melee) | **Crew**: 1 | **Energy**: minimal
- **Key complexity**: Autonomous probe with minimal combat capability. Simple behavior
- **Private data**: None
- **Non-melee**: Excluded from catalog
- marker: `@plan PLAN-20260314-SHIPS.P13`

## Implementation Tasks

### Files to create

12 race files under `rust/src/ships/races/`:
- `chmmr.rs`, `chenjesu.rs`, `mycon.rs`, `melnorme.rs`, `umgah.rs`, `urquan.rs`, `black_urquan.rs`, `slylandro.rs`, `zoqfotpik.rs` (melee)
- `sis_ship.rs`, `samatra.rs`, `probe.rs` (non-melee)

### Files to modify

- `rust/src/ships/races/mod.rs`
  - Add all 12 new race modules
  - Final state: all 28 races declared

- `rust/src/ships/registry.rs`
  - Replace all remaining temporary unsupported-species errors with real implementations
  - Every `SpeciesId` variant maps to a real implementation by the end of this phase

### TDD approach per race
1. Descriptor constants match C (all 12)
2. Weapon mechanics (projectile properties, homing, fragmentation, charge-up, etc.)
3. Special ability mechanics (satellites, fighters, tongue, FRIED, etc.)
4. AI logic
5. For non-melee: verify catalog exclusion, verify special spawn path
6. Edge cases: SIS configurable loadout, Sa-Matra phases, ZapSat persistence

### Special attention items
- **SIS Ship prerequisites**: Phase 03.5 must already have identified the exact campaign/module-state accessors, and earlier bridge phases must already have provided the minimal c_bridge support needed to read flagship configuration. P13 consumes that prerequisite; it does not invent the bridge surface late.
- **SIS Ship**: Descriptor construction must distinguish static template defaults from runtime configuration sourced from campaign/module state.
- **Sa-Matra**: Complex multi-weapon system, unique death, may need special battle-mode integration already analyzed in lifecycle/runtime phases.
- **Chmmr ZapSats**: Persistent satellite elements need lifecycle management across frames.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] 12 race files created under `races/`
- [ ] All 28 races declared in `races/mod.rs`
- [ ] `registry.rs` has real implementations for all 28 species by end of phase
- [ ] Non-melee ships implement same ShipBehavior trait
- [ ] SIS ship reads campaign configuration through the previously-defined bridge
- [ ] Plan/requirement traceability markers present in all race files

## Semantic Verification Checklist (Mandatory)
- [ ] Chmmr: ZapSats orbit, fire at enemies, persist across frames
- [ ] Chenjesu: crystal shards fragment into sub-shards on impact
- [ ] Mycon: plasmoid homes and grows during flight
- [ ] Melnorme: weapon charges proportional to hold time
- [ ] Umgah: antimatter cone area damage, zip backward thrust
- [ ] Ur-Quan: fighters launch, seek, attack independently
- [ ] Kohr-Ah: blade returns on boomerang path, FRIED ring expands
- [ ] Slylandro: continuous movement, lightning seeks
- [ ] ZoqFotPik: tongue grabs and pulls enemy
- [ ] SIS Ship: configurable weapon/special from campaign modules
- [ ] SIS Ship: NOT in master catalog
- [ ] Sa-Matra: multi-weapon systems, final battle behavior
- [ ] Sa-Matra: NOT in master catalog
- [ ] Probe: minimal combat, autonomous
- [ ] Probe: NOT in master catalog
- [ ] All 28 races: descriptor constants match C reference values exactly
- [ ] All 28 races: AI makes reasonable decisions
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
# Check ALL race files
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/
```

## Success Criteria
- [ ] All 28 race implementations compile and pass tests
- [ ] Non-melee ships function correctly outside catalog
- [ ] Complex mechanics (ZapSats, fighters, fragmentation, charging) verified
- [ ] SIS configurable loadout works using the earlier-defined bridge contract
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/races/`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P13.md`
