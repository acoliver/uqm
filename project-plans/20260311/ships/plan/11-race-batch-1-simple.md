# Phase 11: Race Batch 1 — Simple Ships (8 Races)

## Phase ID
`PLAN-20260314-SHIPS.P11`

## Prerequisites
- Required: Phase 10a (Writeback Verification) completed and PASS
- Expected files: All shared infrastructure (types, traits, registry, loader, catalog, queue, runtime, lifecycle, writeback)

## Requirements Implemented (Expanded)

### Per-Race Behavioral Hooks
**Requirement text**: Each race implementation may register behavioral hooks. Races that mutate their descriptor instance at runtime shall continue to produce the same externally observable effects.

### Roster Preservation
**Requirement text**: The end-state subsystem shall include every melee-eligible race.

### Collision Compatibility
**Requirement text**: Collision outcomes relevant to ship behavior shall match established combat behavior for each race.

## Batch 1 Races

These 8 races have relatively straightforward weapon/special mechanics with minimal private state:

### 1. Arilou — `rust/src/ships/races/arilou.rs`
- **C source**: `sc2/src/uqm/ships/arilou/arilou.c` (304 lines)
- **Ship cost**: 16 | **Crew**: 6 | **Energy**: 20
- **Weapon**: Auto-aiming tracking laser (IMMEDIATE_WEAPON) — `initialize_autoaim_laser()`
- **Special**: Quasi-space teleport — random position jump
- **AI**: `arilou_intelligence()` — teleport-heavy, evasive
- **Private data**: None
- **Key behaviors**: TrackShip() for autoaim, random teleport position, HYPER_LIFE countdown for teleport
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 2. Human/Earthling — `rust/src/ships/races/human.rs`
- **C source**: `sc2/src/uqm/ships/human/human.c` (~500 lines)
- **Ship cost**: 11 | **Crew**: 18 | **Energy**: 18
- **Weapon**: Spread-fire laser — `initialize_laser()` with slight spread
- **Special**: Point-defense nuclear missile (seeking, detonation area damage)
- **AI**: `human_intelligence()` — standard combat with nuke usage
- **Private data**: None
- **Key behaviors**: Point-defense laser tracking, nuclear detonation area effect, SEEKING_WEAPON flag
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 3. Spathi — `rust/src/ships/races/spathi.rs`
- **C source**: `sc2/src/uqm/ships/spathi/spathi.c` (~400 lines)
- **Ship cost**: 18 | **Crew**: 30 | **Energy**: 10
- **Weapon**: Forward torpedo — `initialize_torpedo()`
- **Special**: Rear-firing BUTT missile (backward-launched seeking missile)
- **AI**: `spathi_intelligence()` — cowardly, prefers rear attacks
- **Private data**: None
- **Key behaviors**: FIRES_AFT for BUTT missile, backward missile launch, torpedo lifetime
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 4. Supox — `rust/src/ships/races/supox.rs`
- **C source**: `sc2/src/uqm/ships/supox/supox.c` (~300 lines)
- **Ship cost**: 13 | **Crew**: 12 | **Energy**: 16
- **Weapon**: Forward globule spray — `initialize_gob()`
- **Special**: Lateral/reverse thrust — allows strafing movement
- **AI**: `supox_intelligence()` — uses lateral movement
- **Private data**: None
- **Key behaviors**: preprocess modifies velocity for lateral/reverse thrust during SPECIAL
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 5. Thraddash — `rust/src/ships/races/thraddash.rs`
- **C source**: `sc2/src/uqm/ships/thradd/thradd.c` (~400 lines)
- **Ship cost**: 10 | **Crew**: 8 | **Energy**: 24
- **Weapon**: Forward plasma blast — `initialize_flame()`
- **Special**: Afterburner with damaging exhaust trail
- **AI**: `thraddash_intelligence()` — uses afterburner trail offensively
- **Private data**: None
- **Key behaviors**: SPECIAL creates trailing exhaust elements that damage enemy, afterburner sets high thrust
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 6. Yehat — `rust/src/ships/races/yehat.rs`
- **C source**: `sc2/src/uqm/ships/yehat/yehat.c` (~350 lines)
- **Ship cost**: 23 | **Crew**: 20 | **Energy**: 10
- **Weapon**: Twin pulse cannon — `initialize_twin_pulse()`
- **Special**: Energy shield — absorbs damage, costs energy per frame
- **AI**: `yehat_intelligence()` — shield-aware combat
- **Private data**: None
- **Key behaviors**: SHIELD_DEFENSE flag, shield collision override (absorb damage → gain energy), twin projectile spawn
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 7. Druuge — `rust/src/ships/races/druuge.rs`
- **C source**: `sc2/src/uqm/ships/druuge/druuge.c` (~300 lines)
- **Ship cost**: 17 | **Crew**: 14 | **Energy**: 32
- **Weapon**: Mass driver cannon with recoil — `initialize_mass_driver()`, recoil applies backward velocity
- **Special**: Furnace — sacrifice crew member for energy (crew_level--, energy += amount)
- **AI**: `druuge_intelligence()` — uses furnace when energy low
- **Private data**: None
- **Key behaviors**: Weapon recoil modifies velocity, special decrements crew for energy
- marker: `@plan PLAN-20260314-SHIPS.P11`

### 8. Ilwrath — `rust/src/ships/races/ilwrath.rs`
- **C source**: `sc2/src/uqm/ships/ilwrath/ilwrath.c` (~350 lines)
- **Ship cost**: 10 | **Crew**: 22 | **Energy**: 16
- **Weapon**: Hellfire blast — `initialize_flame()` short-range area
- **Special**: Cloaking device — makes ship invisible
- **AI**: `ilwrath_intelligence()` — uses cloak to ambush
- **Private data**: None
- **Key behaviors**: Cloak toggles ship visibility flags, hellfire is short-range with area effect
- marker: `@plan PLAN-20260314-SHIPS.P11`

## Implementation Tasks

### Files to create

For each race, create `rust/src/ships/races/<name>.rs`:
- Define `<Name>Ship` struct (empty or minimal fields for Batch 1)
- Implement `ShipBehavior` trait with:
  - `descriptor_template()` — returns RaceDescTemplate with correct constants matching C #defines
  - `preprocess()` — race-specific per-frame logic (if any)
  - `postprocess()` — race-specific per-frame logic (if any)
  - `init_weapon()` — weapon creation matching C behavior
  - `intelligence()` — AI logic ported from C
  - `uninit()` — cleanup (trivial for Batch 1)
  - `collision_override()` — only for Yehat (shield)

### Files to modify

- `rust/src/ships/races/mod.rs`
  - Add `pub mod arilou;`, `pub mod human;`, etc. for all 8 races
  - Re-export race types

- `rust/src/ships/registry.rs`
  - Replace `StubShip` match arms for these 8 races with real implementations
  - `SpeciesId::Arilou => Box::new(arilou::ArilouShip::new())`
  - etc.

### Pseudocode traceability
- Uses pseudocode component 9, lines 400-446 (template pattern)

### TDD approach per race
1. Write test asserting `descriptor_template()` returns correct constants (cost, crew, energy, flags)
2. Write test for weapon initialization (correct element type, range, damage)
3. Write test for special ability behavior
4. Write test for AI intelligence basic decisions
5. Implement to satisfy tests

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] 8 race files created under `races/`
- [ ] `races/mod.rs` declares and re-exports all 8
- [ ] `registry.rs` updated with real impls for 8 races
- [ ] Each race's descriptor_template constants match C #defines exactly
- [ ] Plan/requirement traceability markers present in each file

## Semantic Verification Checklist (Mandatory)
- [ ] Arilou: autoaim laser tracks target, teleport randomizes position
- [ ] Human: point-defense nuke seeks and detonates with area damage
- [ ] Spathi: BUTT missile fires backward, torpedo fires forward
- [ ] Supox: lateral thrust during special modifies velocity correctly
- [ ] Thraddash: afterburner trail elements created behind ship, trail damages
- [ ] Yehat: shield absorbs damage and converts to energy, twin pulse fires two projectiles
- [ ] Druuge: weapon recoil modifies velocity backward, furnace trades crew for energy
- [ ] Ilwrath: cloak toggles visibility, hellfire has short-range area effect
- [ ] All 8 races: descriptor constants match C values (cost, crew, energy, characteristics)
- [ ] All 8 races: weapon energy cost and cooldown match C values
- [ ] All 8 races: AI makes reasonable decisions (thrust, weapon, special usage)
- [ ] No placeholder/deferred implementation patterns remain
- [ ] StubShip is NOT used for any of these 8 races anymore

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/races/arilou.rs rust/src/ships/races/human.rs rust/src/ships/races/spathi.rs rust/src/ships/races/supox.rs rust/src/ships/races/thraddash.rs rust/src/ships/races/yehat.rs rust/src/ships/races/druuge.rs rust/src/ships/races/ilwrath.rs
```

## Success Criteria
- [ ] 8 race implementations compile and pass tests
- [ ] Registry dispatch uses real implementations
- [ ] Descriptor constants match C reference exactly
- [ ] Weapon/special behavior tests pass
- [ ] AI tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/races/`
- blocking issues: c_bridge functions not available for element creation

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P11.md`
