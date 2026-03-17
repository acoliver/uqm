# Phase 14: C-Side Bridge Wiring

## Phase ID
`PLAN-20260314-SHIPS.P14`

## Prerequisites
- Required: Phase 13a (Batch 3 Verification) completed and PASS
- Expected files: All 28 race implementations, all shared infrastructure, all Rust FFI exports shaped by the earlier ABI contract

## Requirements Implemented (Expanded)

This phase connects the Rust ships subsystem to the C runtime. No new ship behavior is added — this is pure wiring and final enablement of the already-defined boundary contract.

### Build Toggle
Add `USE_RUST_SHIPS` to the build configuration, matching the pattern established by `USE_RUST_COMM`, `USE_RUST_VIDEO`, etc.

### C-Side Guards
Guard all replaced C functions behind `#ifndef USE_RUST_SHIPS` so the C code remains compilable but is bypassed when Rust is active.

### FFI Export Surface
Create the FFI export layer in Rust that provides `extern "C"` functions callable from C, using the exact typed signatures defined in Phase 03.5 rather than ad hoc `usize`/`*const c_void` placeholders.

## Implementation Tasks

### Files to create

- `rust/src/ships/ffi.rs` — FFI export surface
  - marker: `@plan PLAN-20260314-SHIPS.P14`
  - Contents:
    - Typed `extern "C"` exports matching the contract defined in `ffi_contract.rs`
    - Representative examples (final names/signatures must match actual C declarations):
      - `#[no_mangle] pub extern "C" fn rust_init_ships() -> COUNT`
      - `#[no_mangle] pub extern "C" fn rust_uninit_ships()`
      - `#[no_mangle] pub extern "C" fn rust_spawn_ship(starship: *mut STARSHIP) -> BOOLEAN`
      - `#[no_mangle] pub extern "C" fn rust_load_master_ship_list() -> BOOLEAN`
      - `#[no_mangle] pub extern "C" fn rust_free_master_ship_list()`
      - `#[no_mangle] pub extern "C" fn rust_find_master_ship(species_id: SPECIES_ID) -> *const MASTER_SHIP_INFO`
      - `#[no_mangle] pub extern "C" fn rust_find_master_ship_by_index(index: COUNT) -> *const MASTER_SHIP_INFO`
      - `#[no_mangle] pub extern "C" fn rust_get_ship_cost_from_index(index: COUNT) -> COUNT`
      - `#[no_mangle] pub extern "C" fn rust_build_ship(queue_head: HSTARSHIP, species_id: SPECIES_ID) -> HSTARSHIP`
      - `#[no_mangle] pub extern "C" fn rust_get_starship_from_index(queue_head: HSTARSHIP, index: COUNT) -> *mut STARSHIP`
      - `#[no_mangle] pub extern "C" fn rust_clone_ship_fragment(src: *const SHIP_FRAGMENT, dst: *mut SHIP_FRAGMENT) -> BOOLEAN`
      - `#[no_mangle] pub extern "C" fn rust_add_escort_ships(fleet: *mut FLEET_INFO, built_queue: HSTARSHIP, max_build: COUNT) -> COUNT`
      - `#[no_mangle] pub extern "C" fn rust_count_escort_ships(fleet: *const FLEET_INFO) -> COUNT`
      - `#[no_mangle] pub extern "C" fn rust_have_escort_ship(fleet: *const FLEET_INFO, species_id: SPECIES_ID) -> BOOLEAN`
      - `#[no_mangle] pub extern "C" fn rust_set_race_allied(fleet: *mut FLEET_INFO, allied: BOOLEAN)`
      - `#[no_mangle] pub extern "C" fn rust_escort_feasibility_study(fleet: *mut FLEET_INFO, crew_budget: COUNT, fuel_budget: COUNT) -> BOOLEAN`
      - `#[no_mangle] pub extern "C" fn rust_start_sphere_tracking(fleet: *mut FLEET_INFO)`
      - `#[no_mangle] pub extern "C" fn rust_load_ship(species_id: SPECIES_ID, battle_ready: BOOLEAN) -> *mut RACE_DESC`
      - `#[no_mangle] pub extern "C" fn rust_free_ship(desc: *mut RACE_DESC, free_battle: BOOLEAN, free_metadata: BOOLEAN)`
      - `#[no_mangle] pub extern "C" fn rust_ship_preprocess(element: *mut ELEMENT)`
      - `#[no_mangle] pub extern "C" fn rust_ship_postprocess(element: *mut ELEMENT)`
      - `#[no_mangle] pub extern "C" fn rust_ship_death(element: *mut ELEMENT)`
      - `#[no_mangle] pub extern "C" fn rust_install_code_res_type() -> BOOLEAN`
    - No primary ABI-facing API should use raw `usize` / `*const c_void` in place of a concrete ships/battle type

### Files to modify (C side)

- `sc2/build/unix/build.config`
  - Add `USE_RUST_SHIPS` to the substitution-variable list
  - Add `-DUSE_RUST_SHIPS` to the enabled Rust bridge action

- `sc2/config_unix.h`
  - Add `#define USE_RUST_SHIPS 1` (conditional on `HAVE_RUST_BRIDGE`)

- `sc2/src/uqm/dummy.c`
  - Guard entire body behind `#ifndef USE_RUST_SHIPS`
  - In `#else` block: forward public API to typed Rust exports

- `sc2/src/uqm/loadship.c`
  - Guard `load_ship()` and `free_ship()` behind `#ifndef USE_RUST_SHIPS`
  - In `#else` block: forward to typed Rust equivalents

- `sc2/src/uqm/master.c`
  - Guard `LoadMasterShipList()`, `FreeMasterShipList()`, and all lookup functions behind `#ifndef USE_RUST_SHIPS`
  - In `#else` block: forward to typed Rust equivalents

- `sc2/src/uqm/build.c`
  - Guard `Build()`, `GetStarShipFromIndex()`, `CloneShipFragment()`, escort helpers, `EscortFeasibilityStudy()`, and `StartSphereTracking()` behind `#ifndef USE_RUST_SHIPS`
  - In `#else` block: forward to typed Rust equivalents

- `sc2/src/uqm/ship.c`
  - Guard `ship_preprocess()`, `ship_postprocess()`, `spawn_ship()`, `GetNextStarShip()`, `GetInitialStarShips()` behind `#ifndef USE_RUST_SHIPS`
  - In `#else` block: forward to typed Rust equivalents
  - Keep `animation_preprocess()` and `inertial_thrust()` available only if the earlier contract requires C-side reuse; otherwise route consistently through Rust

- `sc2/src/uqm/init.c`
  - Guard `InitShips()` and `UninitShips()` behind `#ifndef USE_RUST_SHIPS`
  - If `InitSpace()` / `UninitSpace()` contain mixed responsibilities, only route the ship-runtime-owned subset through Rust and leave broader battle/environment orchestration in C as defined by P09

- `sc2/src/uqm/ships/*/*.c` (all 28 race files)
  - Guard each race's `init_*()` function behind `#ifndef USE_RUST_SHIPS`
  - The race files remain compilable but unused when Rust ships are active

### Files to modify (Rust side)

- `rust/src/ships/mod.rs`
  - Add `pub mod ffi;`

- `rust/src/game_init/ffi.rs`
  - Update ship-related FFI exports to delegate to `ships::ffi` instead of local stubs
  - Remove duplicate ship FFI surface (delegate, don't duplicate)

### Pseudocode traceability
- N/A — this is wiring, not algorithm

## Verification Commands

```bash
# Rust side
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C side — verify C still compiles with USE_RUST_SHIPS both enabled and disabled
# With USE_RUST_SHIPS disabled (C path):
cd sc2 && ./build.sh uqm

# With USE_RUST_SHIPS enabled (Rust path):
cd sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `ffi.rs` created with all extern "C" exports
- [ ] `build.config` has `USE_RUST_SHIPS` toggle
- [ ] `config_unix.h` has `#define USE_RUST_SHIPS`
- [ ] All shared C files have `#ifndef USE_RUST_SHIPS` guards
- [ ] All 28 race C files have guards on `init_*()` functions
- [ ] `game_init/ffi.rs` delegates to `ships::ffi`
- [ ] No duplicate FFI surface between `game_init` and `ships`
- [ ] FFI signatures match the earlier Phase 03.5 contract exactly
- [ ] Campaign/build helper exports include escort feasibility and sphere tracking helpers
- [ ] `init.c` wiring preserves the documented split between ships-owned runtime init and C-owned environment orchestration

## Semantic Verification Checklist (Mandatory)
- [ ] C builds successfully with `USE_RUST_SHIPS` **disabled** (C path unchanged)
- [ ] Rust-enabled build compiles and links
- [ ] FFI function signatures match what C callers expect
- [ ] FFI functions correctly delegate to Rust ship subsystem
- [ ] No symbol conflicts between old FFI surface and new
- [ ] Link/load symbol validation passes before gameplay scenarios
- [ ] Element callback registration works through FFI (Rust callbacks callable from C battle loop)
- [ ] Campaign/build helpers continue to satisfy the active `build.c` integration boundary
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/ffi.rs
```

## Success Criteria
- [ ] Build toggle works in both directions
- [ ] All C guards compile correctly
- [ ] FFI surface is complete and ABI-correct
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- sc2/src/uqm/dummy.c sc2/src/uqm/loadship.c sc2/src/uqm/master.c sc2/src/uqm/build.c sc2/src/uqm/ship.c sc2/src/uqm/init.c sc2/build/unix/build.config sc2/config_unix.h rust/src/ships/ffi.rs`
- Note: Race file guards can be reverted with `git checkout -- sc2/src/uqm/ships/`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P14.md`
