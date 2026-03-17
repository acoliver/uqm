# Phase 01: Analysis

## Phase ID
`PLAN-20260314-SHIPS.P01`

## Prerequisites
- Required: Phase 0.5 (Preflight) completed and PASS

## Purpose
Detailed analysis of the ships subsystem: entity model, state transitions, integration touchpoints, C code to replace, and requirement coverage mapping.

## 0. Canonical Requirement Index

This plan assigns canonical requirement IDs to every EARS requirement in `/Users/acoliver/projects/uqm/project-plans/20260311/ships/requirements.md`. All later phase markers and coverage tables must use these IDs exactly.

### 0.1 Ship identity and catalog
- `REQ-SHIP-IDENTITY` — unique species identity across melee and non-melee ships
- `REQ-MELEE-BOUNDARY` — clear melee/non-melee boundary in species space
- `REQ-CATALOG` — master ship catalog exists for melee-eligible ships
- `REQ-CATALOG-SORT` — master catalog sorted by race name
- `REQ-CATALOG-STARTUP` — catalog loaded before interactive setup begins
- `REQ-CATALOG-SHUTDOWN` — catalog freed at engine shutdown
- `REQ-CATALOG-EXCLUSION` — non-melee ships excluded from master catalog
- `REQ-CATALOG-LOOKUP` — catalog lookup by species/index/cost/icons without battle loading

### 0.2 Two-tier ship loading
- `REQ-METADATA-LOAD` — metadata-only load allocates descriptor and metadata assets only
- `REQ-BATTLE-LOAD` — battle-ready load adds full battle assets
- `REQ-DESCRIPTOR-FREE` — free releases tier-appropriate assets and invokes teardown hook

### 0.3 Ship descriptor and runtime data model
- `REQ-SHIP-DESCRIPTOR` — descriptor aggregates ship info, fleet, characteristics, battle data, AI, and hooks
- `REQ-PRIVATE-STATE` — descriptor carries opaque private-data slot
- `REQ-DESCRIPTOR-INSTANCE` — each loaded ship uses isolated mutable runtime state
- `REQ-DESCRIPTOR-MUTATION` — races may mutate descriptor instance fields and callbacks during combat

### 0.4 Ship capability flags
- `REQ-CAPABILITY-FLAGS` — subsystem defines externally observable combat capability flags
- `REQ-FLAGS-READABLE` — capability flags readable by shared runtime, AI, and collision handling

### 0.5 Ship-private state
- `REQ-PRIVATE-STATE-ALLOC` — races can allocate/store per-instance private state through opaque slot
- `REQ-PRIVATE-STATE-OPAQUE` — shared runtime does not interpret private state contents
- `REQ-PRIVATE-STATE-LIFETIME` — private state lifetime cannot exceed owning descriptor instance
- `REQ-PRIVATE-STATE-TEARDOWN` — teardown hook runs before descriptor release when private state exists

### 0.6 Race-specific behavioral hooks
- `REQ-HOOKS-REGISTRATION` — per-instance preprocess/postprocess/weapon/AI/teardown hooks supported
- `REQ-PREPROCESS-HOOK` — preprocess hook invoked once per ship per frame before shared logic
- `REQ-POSTPROCESS-HOOK` — postprocess hook invoked once per ship per frame after shared logic
- `REQ-WEAPON-HOOK` — weapon init hook invoked on primary fire
- `REQ-AI-HOOK` — AI intelligence hook invoked for computer-controlled ships
- `REQ-TEARDOWN-HOOK` — teardown hook invoked before descriptor release
- `REQ-NULL-HOOK-NOOP` — null hooks behave as no-op
- `REQ-HOOK-CHANGE` — races may change their own hooks during combat
- `REQ-HOOK-SERIALIZED` — hook calls serialized per descriptor instance

### 0.7 Collision behavior
- `REQ-COLLISION-CORRECT` — correct collision behavior against ships/projectiles/planets/crew
- `REQ-COLLISION-OVERRIDE` — collision override honored when race replaces callback
- `REQ-COLLISION-COMPATIBILITY` — per-race collision outcomes remain compatible

### 0.8 Shared ship runtime pipeline
- `REQ-PIPELINE-ORDER` — shared per-frame pipeline steps and ordering preserved
- `REQ-MOVEMENT-INERTIAL` — inertial movement model preserved
- `REQ-MOVEMENT-DETERMINISTIC` — movement deterministic for same inputs/state
- `REQ-ENERGY-REGEN` — energy regeneration and max-energy behavior preserved
- `REQ-WEAPON-FIRE` — weapon fire hook and energy deduction behavior preserved
- `REQ-SPECIAL-ACTIVATION` — special activation rules preserved

### 0.9 Battle initialization and teardown
- `REQ-BATTLE-INIT` — battle initialization loads ship-runtime resources and battle-active state
- `REQ-BATTLE-TEARDOWN` — battle teardown stops audio, frees runtime assets/descriptors, writes back crew, clears state

### 0.10 Ship selection and spawn
- `REQ-SPAWN-SEQUENCE` — spawn performs battle-ready load, descriptor bind, crew patch, element setup, callback binding, hook registration, active mark
- `REQ-SPAWN-IDEMPOTENT` — spawn idempotent per queue entry within a battle
- `REQ-SPAWN-ENTRYPOINT` — subsystem exposes queue contracts and spawn entrypoint for external callers

### 0.11 Ship death, transition, and replacement
- `REQ-DEATH-SEQUENCE` — death frees descriptor, records final crew, marks queue entry inactive
- `REQ-REPLACEMENT-SPAWN` — replacement uses standard spawn sequence after external selection
- `REQ-NO-REPLACEMENT-SIGNAL` — subsystem signals no-further-ships when no replacement exists
- `REQ-AUDIO-RESET` — audio stopped/reset during death-to-replacement transitions

### 0.12 Persistence-sensitive crew writeback
- `REQ-WRITEBACK` — surviving crew written back to persistent fragments at death/teardown
- `REQ-WRITEBACK-MATCHING` — writeback matches by queue ordering and species identity, not pointer identity
- `REQ-WRITEBACK-CAMPAIGN` — campaign writeback preserves surviving and zero-crew results
- `REQ-WRITEBACK-MELEE` — SuperMelee still maintains internal crew accounting
- `REQ-FLOATING-CREW` — floating crew accounted for at teardown

### 0.13 Non-melee ships
- `REQ-NONMELEE-SAME-RUNTIME` — non-melee ships use same descriptor/loading/spawn/pipeline/hook model
- `REQ-NONMELEE-CATALOG-EXCLUSION` — non-melee ships excluded from master catalog
- `REQ-NONMELEE-SPAWN` — non-melee ships can spawn through special selection paths without catalog enumeration
- `REQ-NONMELEE-UNIQUE` — subsystem supports unique non-melee behavioral properties through same hook contract

### 0.14 Queue and build primitives
- `REQ-QUEUE-MODEL` — shared combat queue data contracts and helper operations provided
- `REQ-QUEUE-OWNER-BOUNDARY` — external systems own creation/enqueue/selection policy; ships provides consumed contracts/helpers
- `REQ-FRAGMENT-MODEL` — persistent ship-fragment model supported
- `REQ-FRAGMENT-CLONE` — cloning/copying preserves icon/name/crew/energy metadata
- `REQ-FLEET-INFO` — fleet-info model supports allied state, fleet size/growth, encounter composition, known location, sphere tracking, and actual strength

### 0.15 Error handling and failure behavior
- `REQ-LOAD-FAILURE` — load failure frees partial resources, prevents partial descriptor escape, reports diagnostic
- `REQ-SPAWN-FAILURE` — spawn failure treated as no replacement for that entry
- `REQ-FAILURE-ISOLATION` — one ship's load/spawn failure does not corrupt other active ships
- `REQ-PRIVATE-STATE-LEAK` — missing teardown with apparent private state logs diagnostic without unsafe free
- `REQ-TEARDOWN-ROBUSTNESS` — teardown robust against absent hooks, unspawned/already-freed descriptors, empty entries

### 0.16 Roster and catalog preservation
- `REQ-ROSTER-PRESERVE` — full melee and non-melee runtime roster preserved
- `REQ-CATALOG-PRESERVE` — catalog preserves same melee roster, costs, sort order, and metadata
- `REQ-MUTATION-PRESERVE` — runtime descriptor mutation behavior preserved
- `REQ-QUEUE-PRESERVE` — queue construction, fragment identity, and writeback semantics preserved
- `REQ-NONMELEE-PRESERVE` — non-melee ships preserve established behavioral properties

## 1. Entity and State Model

### 1.1 SpeciesId and Ship Identity

**Current state (C):** `SPECIES_ID` enum in `races.h:74-108` with 25 melee IDs + 3 non-melee IDs + `NUM_SPECIES_ID`.

**Target state (Rust):** `SpeciesId` enum with same values, `#[repr(u8)]` or `#[repr(i32)]` for FFI compatibility. Clear `is_melee_eligible()` method.

**Requirements:** `REQ-SHIP-IDENTITY` (unique species identity), `REQ-MELEE-BOUNDARY` (clear melee/non-melee boundary)

### 1.2 ShipInfo

**Current state (C):** `SHIP_INFO` struct with ship_flags, ship_cost, crew_level, max_crew, energy_level, max_energy, resource IDs for strings/icons/melee_icons, and loaded handles.

**Target state (Rust):** `ShipInfo` struct with typed fields, `ShipFlags` bitflags type.

**Requirements:** `REQ-CAPABILITY-FLAGS` (all flags defined), `REQ-SHIP-DESCRIPTOR` (aggregated in descriptor), `REQ-FLAGS-READABLE` (runtime/AI/collision access)

### 1.3 CharacteristicStuff

**Current state (C):** `CHARACTERISTIC_STUFF` macro-initialized with movement/energy/combat timing parameters.

**Target state (Rust):** `Characteristics` struct with typed fields, default construction from race constants.

**Requirements:** `REQ-SHIP-DESCRIPTOR` (part of descriptor), `REQ-MOVEMENT-INERTIAL` (movement parameters)

### 1.4 DataStuff

**Current state (C):** `DATA_STUFF` with ship/weapon/special frame arrays (3 resolution levels each), captain stuff, victory ditty, ship sounds.

**Target state (Rust):** `ShipData` struct with `[FrameHandle; 3]` for each frame set, CaptainStuff, sound/music handles.

**Requirements:** `REQ-METADATA-LOAD`, `REQ-BATTLE-LOAD`

### 1.5 IntelStuff

**Current state (C):** `INTEL_STUFF` with ManeuverabilityIndex, WeaponRange, intelligence_func pointer.

**Target state (Rust):** `IntelStuff` struct. The intelligence_func is absorbed into `ShipBehavior::intelligence()`.

**Requirements:** `REQ-AI-HOOK` (AI intelligence callback)

### 1.6 RaceDesc

**Current state (C):** `RACE_DESC` aggregates all sub-structs plus function pointers (uninit_func, preprocess_func, postprocess_func, init_weapon_func) plus opaque `data` pointer and `CodeRef`.

**Target state (Rust):** `RaceDesc` struct holds `ShipInfo`, `FleetStuff`, `Characteristics`, `ShipData`, `IntelStuff`, and `Box<dyn ShipBehavior>`. Private data moves into each `ShipBehavior` implementation as fields.

**Requirements:** `REQ-SHIP-DESCRIPTOR` (aggregated), `REQ-PRIVATE-STATE` (opaque data slot), `REQ-DESCRIPTOR-INSTANCE` (isolated mutable state), `REQ-DESCRIPTOR-MUTATION` (mutable runtime instance)

### 1.7 Starship

**Current state (C):** `STARSHIP` is the per-battle queue entry with `RaceDescPtr`, crew state, icons, counters, status/input flags, `hShip`, `ShipFacing`, `playerNr`, control.

**Target state (Rust):** `Starship` struct with similar fields. `race_desc` is `Option<Box<RaceDesc>>` (set when spawned). Element handle remains as opaque `u32`/`usize`.

**Requirements:** `REQ-QUEUE-MODEL` (combat queue entry), `REQ-SPAWN-SEQUENCE` (bind descriptor to queue entry)

### 1.8 ShipFragment

**Current state (C):** `SHIP_FRAGMENT` carries species, crew, display metadata for persistent queue.

**Target state (Rust):** `ShipFragment` struct.

**Requirements:** `REQ-FRAGMENT-MODEL` (persistent ship fragment)

### 1.9 FleetInfo

**Current state (C):** `FLEET_INFO` carries campaign fleet state — allied/hostile, fleet size, encounter composition, SOI tracking.

**Target state (Rust):** `FleetInfo` struct.

**Requirements:** `REQ-FLEET-INFO` (campaign fleet state including allied state, composition, and sphere tracking)

## 2. State Transitions

### 2.1 Ship Lifecycle

```
Unloaded → MetadataLoaded → BattleReady → Spawned → Active → Dead → Freed
                ↑                                                       |
                └───────────────────────────────────────────────────────┘
```

Key transitions:
- `Unloaded → MetadataLoaded`: `load_ship(species, MetadataOnly)` — loads icons, strings only
- `MetadataLoaded → Freed`: `free_ship()` — frees metadata assets
- `Unloaded → BattleReady`: `load_ship(species, BattleReady)` — loads all assets
- `BattleReady → Spawned`: `spawn_ship()` — creates element, binds callbacks
- `Spawned → Active`: first frame preprocess sets initial state
- `Active → Dead`: crew reaches 0, death sequence begins
- `Dead → Freed`: `free_ship()` after death animation — calls uninit hook, frees assets

### 2.2 Master Catalog Lifecycle

```
Empty → Loaded → Freed
```

- `Empty → Loaded`: `load_master_ship_list()` — iterates species, loads metadata-only, sorts by name
- `Loaded → Freed`: `free_master_ship_list()` — frees all catalog entries

### 2.3 Per-Race Mode State Machines

Some races have internal mode state machines:
- **Androsynth:** Normal ↔ Blazer (modifies characteristics, swaps frames/collision)
- **Mmrnmhrm:** X-Form ↔ Y-Form (swaps entire characteristics and frame sets)
- **Chmmr:** Normal + ZapSat tracking (maintains satellite element list)
- **Pkunk:** Normal → Dead → Resurrected (chance-based resurrection with re-spawn)
- **Orz:** Ship-mode ↔ Marine-mode (spawns independent marine elements)

These state machines live inside the `ShipBehavior` implementation, using the struct's own fields as state storage.

## 3. Integration Touchpoints

### 3.1 Battle Engine → Ships (C calls Rust)

| Call Site (C) | Current C Function | Rust FFI Export | Purpose |
|--------------|-------------------|----------------|---------|
| `battle.c:414` | `InitShips()` | `rust_init_ships()` | Battle start |
| `battle.c:511` | `UninitShips()` | `rust_uninit_ships()` | Battle end |
| `ship.c:379` | `spawn_ship()` | `rust_spawn_ship()` | Ship enters combat |
| `ship.c:149` | `ship_preprocess()` | (callback on element) | Per-frame preprocess |
| `ship.c:282` | `ship_postprocess()` | (callback on element) | Per-frame postprocess |
| `ship.c:352` | collision handler | (callback on element) | Ship collision |
| `tactrans.c:466` | `free_ship()` in `new_ship()` | `rust_free_ship()` | Death transition |
| `starcon.c:93` | `LoadMasterShipList()` | `rust_load_master_ship_list()` | Startup catalog |
| `master.c:93` | `FreeMasterShipList()` | `rust_free_master_ship_list()` | Shutdown catalog |
| `build.c:29` | `Build()` | `rust_build_ship()` | Queue entry creation |
| `build.c:53` | `GetStarShipFromIndex()` | `rust_get_starship_from_index()` | Queue lookup |
| `build.c:461` | `CloneShipFragment()` | `rust_clone_ship_fragment()` | Fragment cloning |
| `master.c:117` | `FindMasterShip()` | `rust_find_master_ship()` | Catalog lookup |

### 3.2 Ships → Battle Engine (Rust calls C)

| Purpose | C Function | Rust Bridge Call |
|---------|-----------|-----------------|
| Create weapon element | `AllocElement()` + init | `c_bridge::alloc_element()` |
| Set element velocity | `SetVelocityVector()` | `c_bridge::set_velocity_vector()` |
| Track target | `TrackShip()` | `c_bridge::track_ship()` |
| Initialize laser | `initialize_laser()` | `c_bridge::initialize_laser()` |
| Initialize missile | `initialize_missile()` | `c_bridge::initialize_missile()` |
| Play sound | `ProcessSound()` | `c_bridge::process_sound()` |
| Get element starship | `GetElementStarShip()` | `c_bridge::get_element_starship()` |
| Load graphic resource | `LoadGraphic()` | `c_bridge::load_graphic()` |

### 3.3 Shared Data Across FFI

| Data | Direction | Format |
|------|-----------|--------|
| Species ID | C→Rust, Rust→C | `i32` (enum value) |
| Element handle | C→Rust, Rust→C | opaque `usize` |
| Starship handle | C→Rust, Rust→C | opaque `usize` |
| StatusFlags | C↔Rust | `u16` bitfield |
| Ship capability flags | C↔Rust | `u16` bitfield |
| Crew count | C↔Rust | `u16` |
| Energy level | C↔Rust | `u16` |
| Facing direction | C↔Rust | `i32` (0-15 typically) |
| Position | C↔Rust | `(i32, i32)` |
| Velocity | C↔Rust | `(i32, i32)` |

## 4. Old Code to Replace/Remove

### Files guarded behind `#ifndef USE_RUST_SHIPS`

| C File | Functions Guarded |
|--------|------------------|
| `sc2/src/uqm/dummy.c` | `GetCodeResData()`, `CodeResToInitFunc()`, `InstallCodeResType()` |
| `sc2/src/uqm/loadship.c` | `load_ship()`, `free_ship()` |
| `sc2/src/uqm/master.c` | `LoadMasterShipList()`, `FreeMasterShipList()`, `FindMasterShip()`, accessors |
| `sc2/src/uqm/build.c` | `Build()`, `GetStarShipFromIndex()`, `CloneShipFragment()`, escort helpers, `EscortFeasibilityStudy()`, `StartSphereTracking()` |
| `sc2/src/uqm/ship.c` | `ship_preprocess()`, `ship_postprocess()`, `spawn_ship()`, `GetNextStarShip()`, `GetInitialStarShips()` |
| `sc2/src/uqm/init.c` | `InitShips()`, `UninitShips()`, ship-runtime `InitSpace()`/`UninitSpace()` participation only |
| `sc2/src/uqm/ships/*//*.c` | All 28 race `init_*()` functions (guarded, not deleted) |

### Rust prototype code to replace

| Rust File | Current State | Action |
|-----------|--------------|--------|
| `rust/src/game_init/init.rs` | Stub placeholders | Replace ship functions with delegation to `ships` module |
| `rust/src/game_init/master.rs` | Hardcoded test data | Replace with delegation to `ships::catalog` |
| `rust/src/game_init/ffi.rs` | Unused FFI exports | Replace ship FFI with delegation to `ships::ffi` |

## 5. Requirement Coverage Map

| Requirement Area | Requirements | Covered In Phase |
|-----------------|-------------|-----------------|
| Ship identity and catalog | `REQ-SHIP-IDENTITY`, `REQ-MELEE-BOUNDARY`, `REQ-CATALOG`, `REQ-CATALOG-SORT`, `REQ-CATALOG-STARTUP`, `REQ-CATALOG-SHUTDOWN`, `REQ-CATALOG-EXCLUSION`, `REQ-CATALOG-LOOKUP` | P03, P04, P05, P06 |
| Two-tier loading | `REQ-METADATA-LOAD`, `REQ-BATTLE-LOAD`, `REQ-DESCRIPTOR-FREE` | P05 |
| Ship descriptor | `REQ-SHIP-DESCRIPTOR`, `REQ-PRIVATE-STATE`, `REQ-DESCRIPTOR-INSTANCE`, `REQ-DESCRIPTOR-MUTATION` | P03, P04 |
| Capability flags | `REQ-CAPABILITY-FLAGS`, `REQ-FLAGS-READABLE` | P03 |
| Ship-private state | `REQ-PRIVATE-STATE-ALLOC`, `REQ-PRIVATE-STATE-OPAQUE`, `REQ-PRIVATE-STATE-LIFETIME`, `REQ-PRIVATE-STATE-TEARDOWN` | P03, P04, P10 |
| Behavioral hooks | `REQ-HOOKS-REGISTRATION`, `REQ-PREPROCESS-HOOK`, `REQ-POSTPROCESS-HOOK`, `REQ-WEAPON-HOOK`, `REQ-AI-HOOK`, `REQ-TEARDOWN-HOOK`, `REQ-NULL-HOOK-NOOP`, `REQ-HOOK-CHANGE`, `REQ-HOOK-SERIALIZED` | P04, P08 |
| Collision | `REQ-COLLISION-CORRECT`, `REQ-COLLISION-OVERRIDE`, `REQ-COLLISION-COMPATIBILITY` | P08, P11-P13 |
| Runtime pipeline | `REQ-PIPELINE-ORDER`, `REQ-MOVEMENT-INERTIAL`, `REQ-MOVEMENT-DETERMINISTIC`, `REQ-ENERGY-REGEN`, `REQ-WEAPON-FIRE`, `REQ-SPECIAL-ACTIVATION` | P08 |
| Battle lifecycle | `REQ-BATTLE-INIT`, `REQ-BATTLE-TEARDOWN` | P09 |
| Spawn | `REQ-SPAWN-SEQUENCE`, `REQ-SPAWN-IDEMPOTENT`, `REQ-SPAWN-ENTRYPOINT` | P09 |
| Death/transition | `REQ-DEATH-SEQUENCE`, `REQ-REPLACEMENT-SPAWN`, `REQ-NO-REPLACEMENT-SIGNAL`, `REQ-AUDIO-RESET` | P10 |
| Crew writeback | `REQ-WRITEBACK`, `REQ-WRITEBACK-MATCHING`, `REQ-WRITEBACK-CAMPAIGN`, `REQ-WRITEBACK-MELEE`, `REQ-FLOATING-CREW` | P10 |
| Non-melee ships | `REQ-NONMELEE-SAME-RUNTIME`, `REQ-NONMELEE-CATALOG-EXCLUSION`, `REQ-NONMELEE-SPAWN`, `REQ-NONMELEE-UNIQUE` | P04, P05, P13 |
| Queue/build | `REQ-QUEUE-MODEL`, `REQ-QUEUE-OWNER-BOUNDARY`, `REQ-FRAGMENT-MODEL`, `REQ-FRAGMENT-CLONE`, `REQ-FLEET-INFO` | P03.5, P07 |
| Error handling | `REQ-LOAD-FAILURE`, `REQ-SPAWN-FAILURE`, `REQ-FAILURE-ISOLATION`, `REQ-TEARDOWN-ROBUSTNESS`, `REQ-PRIVATE-STATE-LEAK` | P05, P09, P10 |
| Preservation | `REQ-ROSTER-PRESERVE`, `REQ-CATALOG-PRESERVE`, `REQ-MUTATION-PRESERVE`, `REQ-QUEUE-PRESERVE`, `REQ-NONMELEE-PRESERVE` | P04, P06, P07, P11-P13, P15 |

## 6. Edge Cases and Error Handling

### 6.1 Load Failure
- Missing resource files → clean up partially loaded assets, return error, do not leave dangling descriptor
- Current C has known TODO for incomplete failure cleanup (`loadship.c:159`) — Rust implementation should handle this correctly

### 6.2 Spawn Failure
- Load failure during spawn → side treated as having no replacement
- Element allocation failure → same as load failure
- Must not corrupt other active ship state

### 6.3 Double-Free Prevention
- Descriptor freed during mid-battle transition must not be freed again during teardown
- Queue entries with no associated descriptor must be handled gracefully

### 6.4 Race-Specific Edge Cases
- Pkunk resurrection: death hook must check resurrection chance before finalizing death
- Androsynth mode switch: characteristics mutation during combat must not be restricted
- Mmrnmhrm X-form: complete frame set and characteristics swap mid-combat
- Chmmr ZapSat: satellite elements survive ship, need proper cleanup
- Shofixti Glory Device: self-destruct damages nearby ships including own side
- SIS Ship: configurable weapon/special from campaign state, not static template

## Verification

### Analysis Verification Checklist
- [ ] Canonical requirement index assigns exactly one plan ID to every EARS requirement in `requirements.md`
- [ ] All later plan files use only canonical REQ IDs from this index
- [ ] All entity types from `races.h` are mapped to Rust equivalents
- [ ] All state transitions are documented
- [ ] All integration touchpoints (C→Rust and Rust→C) are identified
- [ ] All C files to be guarded are listed
- [ ] All requirements from `requirements.md` are mapped to implementation phases
- [ ] Edge cases and error handling scenarios are documented
- [ ] Race-specific complexity is identified and assigned to appropriate batch
