# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-SHIPS.P00.5`

## Purpose
Verify assumptions about toolchain, dependencies, types, call paths, and integration feasibility before implementation begins.

## Toolchain Verification

```bash
cargo --version
rustc --version
cargo clippy --version
```

- [ ] Rust toolchain is 1.75+ (required for `LazyLock` stabilization)
- [ ] `parking_lot` crate present in `Cargo.toml`
- [ ] `serial_test` crate present in dev-dependencies
- [ ] `bindgen` present in build-dependencies (for C struct access)

## Dependency Verification

- [ ] `parking_lot` version in `/Users/acoliver/projects/uqm/rust/Cargo.toml`
- [ ] `serial_test` version in `/Users/acoliver/projects/uqm/rust/Cargo.toml`
- [ ] `bindgen` version in `/Users/acoliver/projects/uqm/rust/Cargo.toml`
- [ ] No `ships`-specific feature flags needed in `Cargo.toml` initially

## Type/Interface Verification

### C Side — `races.h` Contract Surface
- [ ] `SPECIES_ID` enum: `ARILOU_ID` through `MMRNMHRM_ID` (25 melee IDs), `SIS_SHIP_ID`, `SA_MATRA_ID`, `UR_QUAN_PROBE_ID`, `NUM_SPECIES_ID` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:74-108`
- [ ] `LAST_MELEE_ID = MMRNMHRM_ID` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:103`
- [ ] `SHIP_INFO` struct fields: ship_flags, ship_cost, crew_level, max_crew, energy_level, max_energy, race_strings, icons, melee_icon in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:160-177`
- [ ] `CHARACTERISTIC_STUFF` fields: max_thrust, thrust_increment, energy_regeneration, weapon_energy_cost, special_energy_cost, energy_wait, turn_wait, thrust_wait, weapon_wait, special_wait, ship_mass in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:145-158`
- [ ] `DATA_STUFF` fields: ship frames, weapon frames, special frames, captain_stuff, victory_ditty, ship_sounds in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:187-201`
- [ ] `INTEL_STUFF` fields: ManeuverabilityIndex, WeaponRange, intelligence_func in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:136-143`
- [ ] `RACE_DESC` fields: ship_info, fleet, characteristics, ship_data, cyborg_control, uninit_func, preprocess_func, postprocess_func, init_weapon_func, data, CodeRef in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:204-227`
- [ ] `STARSHIP` fields: RaceDescPtr, crew, icons, counters, flags, hShip, ShipFacing, playerNr, control in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:245-285`
- [ ] `SHIP_FRAGMENT` fields in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:301-324`
- [ ] `FLEET_INFO` fields in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:337-382`
- [ ] Ship capability flags: `SEEKING_WEAPON`, `POINT_DEFENSE`, `IMMEDIATE_WEAPON`, `CREW_IMMUNE`, `FIRES_FORE`, `SHIELD_DEFENSE`, etc. in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:43-58`
- [ ] Status flags: `LEFT`, `RIGHT`, `THRUST`, `WEAPON`, `SPECIAL`, etc. in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:60-72`

### C Side — Battle Engine Interface
- [ ] `ELEMENT` struct and its callback fields (preprocess_func, postprocess_func, death_func, collision_func) in `/Users/acoliver/projects/uqm/sc2/src/uqm/element.h`
- [ ] `GetElementStarShip()` / `SetElementStarShip()` macros/functions
- [ ] `spawn_ship()` signature in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:379`
- [ ] `InitShips()` / `UninitShips()` signatures in `/Users/acoliver/projects/uqm/sc2/src/uqm/init.c`
- [ ] `LoadMasterShipList()` / `FreeMasterShipList()` signatures in `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c`
- [ ] `Build()` / `CloneShipFragment()` signatures in `/Users/acoliver/projects/uqm/sc2/src/uqm/build.c`
- [ ] `TrackShip()` function availability for AI/autoaim
- [ ] `SetVelocityVector()` function availability for movement
- [ ] `initialize_laser()` / `initialize_missile()` weapon helper availability

### C Side — Resource System
- [ ] `InstallResTypeVectors()` for registering "SHIP" resource type
- [ ] `res_GetResource()` / `res_DetachResource()` for ship code resource loading
- [ ] `LoadGraphic()` / `LoadMusic()` / `LoadSound()` / `LoadStringTable()` for battle assets

### Rust Side — Existing Prototype Surface
- [ ] `rust/src/game_init/init.rs` contains stub `init_ships()` / `uninit_ships()` (confirm placeholder-only)
- [ ] `rust/src/game_init/master.rs` contains stub `MasterShipList` with hardcoded test data (confirm placeholder-only)
- [ ] `rust/src/game_init/ffi.rs` exports `rust_init_ships`, `rust_uninit_ships`, `rust_load_master_ship_list`, `rust_free_master_ship_list` (confirm no C callers)
- [ ] `rust/src/lib.rs` has no `ships` module (confirm)
- [ ] Determine if existing `game_init` module should be modified or if `ships` module replaces ship-related portions

## Call-Path Feasibility

### Master Catalog Load Path
- [ ] C: `starcon.c` → `LoadMasterShipList()` → `load_ship(species, FALSE)` → `GetCodeResData()` → `init_<race>()`
- [ ] Rust equivalent: startup FFI → `rust_load_master_ship_list()` → `catalog::load()` → `loader::load_ship(species, MetadataOnly)` → `registry::create_descriptor(species)`
- [ ] Verify resource subsystem FFI is available for loading icons/strings from Rust

### Battle Spawn Path
- [ ] C: `ship.c:spawn_ship()` → `load_ship(species, TRUE)` → race init → element creation
- [ ] Rust equivalent: `ffi::rust_spawn_ship()` → `lifecycle::spawn_ship()` → `loader::load_ship(species, BattleReady)` → element creation via `c_bridge`
- [ ] Verify ELEMENT allocation is accessible from Rust (AllocElement or equivalent)
- [ ] Verify element callback registration is accessible from Rust

### Per-Frame Pipeline Path
- [ ] C: battle loop → `ship_preprocess()` → race preprocess → shared logic → `ship_postprocess()` → race postprocess
- [ ] Rust equivalent: C battle loop calls Rust preprocess/postprocess FFI per frame
- [ ] Verify element iteration is accessible from Rust during frame processing
- [ ] Determine: does Rust register callbacks on C elements, or does C call into Rust per-frame?

### Ship Death Path
- [ ] C: `tactrans.c:new_ship()` → `free_ship()` → `GetNextStarShip()`
- [ ] Verify death callback can invoke Rust teardown
- [ ] Verify crew writeback path from Rust to C ship fragments

## Test Infrastructure Verification

- [ ] `cargo test --workspace --all-features` passes currently
- [ ] `#[serial]` attribute available for FFI tests requiring global state
- [ ] Determine testing strategy for per-race ships without full battle engine (unit test preprocess/postprocess/weapon with mock BattleContext)

## Integration Complexity Assessment

### High-Risk Integration Points
- [ ] Element callback registration: Rust functions must be callable from C's element dispatch
- [ ] Display-list element creation: Rust ships need to create weapon/special elements on C display list
- [ ] Sound/graphics API: Rust ships need to call ProcessSound, LoadGraphic, etc.
- [ ] Shared mutable state: RACE_DESC mutations during combat must work through trait system

### Dependency Analysis
- [ ] Ships subsystem depends on: resource, graphics, sound, input, element/display-list, collision
- [ ] Which of these are already Rust-owned vs C-owned?
- [ ] For C-owned dependencies: verify FFI bridge functions exist or plan their creation

## Blocking Issues

- If element creation/callback APIs are not accessible from Rust via FFI, an element-bridge layer must be created first.
- If resource loading (LoadGraphic, LoadSound, etc.) is not accessible from Rust, a resource-bridge layer is needed.
- If the display-list iteration model is incompatible with Rust ownership, the per-frame pipeline integration model must be adjusted.

## Gate Decision

- [ ] PASS: proceed to Phase 1
- [ ] FAIL: revise plan (document blocking issues and required prerequisite work)
