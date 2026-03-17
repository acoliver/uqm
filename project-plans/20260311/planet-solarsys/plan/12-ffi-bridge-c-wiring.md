# Phase 12: FFI Bridge & C-Side Wiring

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P12`

## Prerequisites
- Required: Phase 11a (Lifecycle Verification) completed
- Expected: Rust logic complete for internal behavior, with any still-provisional dispatch points clearly limited to final FFI wiring
- Required: Build system access for adding `USE_RUST_PLANETS` toggle

## Requirements Implemented (Expanded)

### REQ-PSS-FFI-001: C-to-Rust entry points
**Requirement text**: The FFI bridge shall expose entry points that C code can call to invoke the Rust planet-solarsys subsystem when `USE_RUST_PLANETS` is enabled.

Behavior contract:
- GIVEN: `USE_RUST_PLANETS` is defined at build time
- WHEN: C code calls `rust_ExploreSolarSys()`
- THEN: The Rust `explore_solar_sys()` implementation executes

### REQ-PSS-FFI-002: C generation-function callbacks
**Requirement text**: The 50+ system-specific C generators must remain callable from Rust through an FFI shim.

Behavior contract:
- GIVEN: Star index maps to a C-defined `GenerateFunctions` table
- WHEN: Rust calls `get_generate_dispatch(star_index)`
- THEN: A wrapper preserving the audited handler-class semantics is returned

### REQ-PSS-FFI-003: C-side conditional compilation
**Requirement text**: All replaced C code shall be guarded behind `#ifndef USE_RUST_PLANETS` so that enabling the toggle routes to Rust.

### REQ-PSS-COMPAT-001: ABI compatibility
**Requirement text**: FFI data types must be compatible between Rust and C (matching sizes, alignments, field orders for any types passed across the boundary).

### REQ-PSS-COMPAT-002: Build system toggle
**Requirement text**: A `USE_RUST_PLANETS` build toggle must be added alongside existing Rust bridge toggles.

### REQ-PSS-PERSIST-009: Persistence-window and global-access bridge fidelity
**Requirement text**: FFI shims shall preserve the legal persistence-window boundary and global navigation accessor contract established earlier in the plan.

## Implementation Tasks

### Files to modify

- `rust/src/planets/ffi.rs` — Full FFI bridge
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P12`
  - marker: `@requirement REQ-PSS-FFI-001, REQ-PSS-FFI-002, REQ-PSS-COMPAT-001, REQ-PSS-PERSIST-009`
  - Remove stub
  - Implement `#[no_mangle] pub extern "C"` exports:
    - `rust_ExploreSolarSys()` — entry into Rust solar-system loop
    - `rust_DoPlanetaryAnalysis(sys_info_ptr, planet_ptr)` — analysis from C if still needed by callers
    - `rust_GeneratePlanetSurface(planet_ptr, surf_def_frame)` — surface gen from C if still needed by callers
    - `rust_ScanSystem()` — scan from C
    - `rust_PlanetOrbitMenu()` — orbital menu from C
    - `rust_SaveSolarSysLocation()` — save location from C
    - `rust_LoadPlanet(surf_def_frame)` — planet loading from C
    - `rust_FreePlanet()` — planet cleanup from C
    - Helper FFI exports as needed for C callers
  - Implement `extern "C"` imports (Rust calling C):
    - `c_getGenerateFunctions(star_index: u8) -> *const c_void` — get C function table
    - C GenerateFunctions table accessor shims preserving handler class semantics:
      - override/fallback wrappers for planet/moon/name/orbit-content slots
      - data-provider wrappers for mineral/energy/life count and per-node queries
      - side-effect hook wrappers for NPC lifecycle and pickup hooks
    - `c_DevicesMenu()`, `c_CargoMenu()`, `c_RosterMenu()`, `c_GameOptions()` — external menu dispatch
    - `c_DoInput(input_func: ...)` — input loop driver (if not yet accessible from Rust)
    - Game state accessors: `c_get_ip_planet()`, `c_get_in_orbit()`, `c_set_in_orbit(val)`, related inner/outer position accessors, and any active-context accessors required by the P09.5 feasibility decisions
  - Implement raw C-table wrapper structs based on handler class rather than a single normalized trait shape
  - Type marshaling functions using explicit mirrors only:
    - `planet_desc_to_c(desc: &PlanetDesc) -> CPlanetDesc`
    - `planet_desc_from_c(c_desc: &CPlanetDesc) -> PlanetDesc`
    - `node_info_to_c(info: &NodeInfo) -> CNodeInfo`
    - `star_desc_from_c(c_star: &CStarDesc) -> StarDesc`
    - `system_info_to_c(info: &SystemInfo) -> CSystemInfo` / reverse only if actually required
  - Add explicit `sizeof`/layout assertions where possible
  - Encode the persistence-window and global-access bridge explicitly:
    - do not invent alternate accessor paths after P09.5
    - preserve the same call ordering for persistence get/put and navigation-global reads/writes that earlier phases validated

- `rust/src/planets/generate.rs` — Wire up C generator dispatch
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P12`
  - Update `get_generate_dispatch()`:
    - Call `c_getGenerateFunctions(star_index)` via FFI
    - Wrap returned pointer in audited handler-class wrappers
    - Return internal dispatch object preserving override/fallback, data-provider, side-effect semantics, and NPC/pickup hook lifecycle points
    - Return a concrete wrapper for the name-generation slot rather than leaving naming implicit in other dispatch paths
    - If C returns the default table and Rust-native defaults are intentionally used, document and verify semantic equivalence explicitly

### C-side files to create

- `sc2/src/uqm/rust_planets.h` — Rust FFI declarations for planet subsystem
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P12`
  - Declare all `rust_*` functions
  - Declare `c_gen_*` accessor shims or class-specific wrappers
  - Include guards with `USE_RUST_PLANETS`

- `sc2/src/uqm/rust_planets.c` — C-side FFI shim implementations
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P12`
  - Implement `c_getGenerateFunctions()` — returns pointer to C function table
  - Implement class-specific generation wrappers rather than assuming every slot shares one return convention
  - Implement explicit wrapper/accessor support for the name-generation handler slot
  - Implement game state accessor shims
  - Implement any required active-context / navigation-global accessor shims selected in P09.5
  - Compile only when `USE_RUST_PLANETS` is defined

### C-side files to modify

- `sc2/src/uqm/planets/solarsys.c`
  - Add `#ifndef USE_RUST_PLANETS` guard around function bodies
  - Add `#ifdef USE_RUST_PLANETS` blocks that call `rust_ExploreSolarSys()`, etc.

- `sc2/src/uqm/planets/planets.c`
  - Add `#ifndef USE_RUST_PLANETS` guard around `LoadPlanet`, `FreePlanet`, `PlanetOrbitMenu`

- `sc2/src/uqm/planets/calc.c`
  - Add `#ifndef USE_RUST_PLANETS` guard around `DoPlanetaryAnalysis`

- `sc2/src/uqm/planets/plangen.c`
  - Add `#ifndef USE_RUST_PLANETS` guard around `GeneratePlanetSurface`

- `sc2/src/uqm/planets/scan.c`
  - Add `#ifndef USE_RUST_PLANETS` guard around `ScanSystem`, `GeneratePlanetSide`

- `sc2/src/uqm/planets/orbits.c`, `oval.c`, `pl_stuff.c`, `gentopo.c`, `surface.c`, `report.c`
  - Add `#ifndef USE_RUST_PLANETS` guards around function bodies

### Build system files to modify

- `sc2/build/unix/build.config`
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P12`
  - marker: `@requirement REQ-PSS-COMPAT-002`
  - Add `SYMBOL_USE_RUST_PLANETS_DEF` alongside other Rust bridge symbols
  - Add `USE_RUST_PLANETS` to the Rust bridge toggle menu/set
  - Add `-DUSE_RUST_PLANETS` to compiler flags when enabled

- `sc2/config_unix.h.in` (or equivalent)
  - Add `#undef USE_RUST_PLANETS` / `#define USE_RUST_PLANETS` placeholder

### Files to create

- `rust/src/planets/tests/ffi_tests.rs` — FFI marshaling and dispatch tests
  - Verify layout assertions for every actual boundary-crossing mirror type
  - Verify `PlanetDesc`, `NodeInfo`, save-location values, and any required `SystemInfo` fields round-trip across the C/Rust boundary
  - Verify per-star dispatch identity: the expected C table/wrapper is selected for specific dedicated stars
  - Verify override/fallback handlers, data-provider handlers, and side-effect hooks each preserve their distinct semantics through FFI
  - Verify the name-generation slot preserves override/fallback semantics through FFI and assigns the same names as the baseline for representative dedicated and default systems
  - Verify NPC lifecycle hooks and node-pickup hooks dispatch at the same lifecycle points validated in earlier phases
  - Verify external menu and global-state accessor shims compile and link
  - Verify the selected persistence-window/global-access shim set preserves earlier-phase call ordering and value semantics

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod ffi_tests;`

## Pseudocode Traceability
- FFI bridge connects all pseudocode components (001-419) to C callers

## Verification Commands

```bash
# Rust side
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::ffi_tests --all-features -- --nocapture

# C side (full build with USE_RUST_PLANETS disabled — no regression)
cd sc2 && make clean && make
# Then with USE_RUST_PLANETS enabled
# Configure with Rust planets toggle enabled and rebuild
```

## Structural Verification Checklist
- [ ] `ffi.rs` fully implemented, no `todo!()`
- [ ] `rust_planets.h` and `rust_planets.c` created
- [ ] All replaced C files have `#ifndef USE_RUST_PLANETS` guards
- [ ] Build system has `USE_RUST_PLANETS` toggle
- [ ] `config_unix.h` has `USE_RUST_PLANETS` definition
- [ ] Every boundary-crossing type names a concrete mirror and conversion path

## Semantic Verification Checklist
- [ ] Building with `USE_RUST_PLANETS=0` produces identical behavior to pre-plan state
- [ ] Building with `USE_RUST_PLANETS=1` links Rust implementations
- [ ] C `GenerateFunctions` tables are accessible from Rust via FFI
- [ ] Override/fallback handlers dispatch correctly through FFI
- [ ] Name-generation handlers dispatch correctly through FFI, including fallback to default naming when dedicated handlers do not override
- [ ] Data-provider handlers dispatch correctly through FFI for count and per-node queries
- [ ] Side-effect hooks dispatch correctly through FFI at the right lifecycle points
- [ ] Type marshaling preserves field values across C/Rust boundary
- [ ] External menu dispatch (cargo, devices, roster) works via FFI
- [ ] Navigation-global accessor shims preserve baseline-compatible values and timing
- [ ] Persistence-window bridge preserves earlier validated get/put legality ordering
- [ ] All system-specific C generators still compile and are callable
- [ ] Per-star dispatch identity is verified, not just downstream behavior after dispatch

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/ffi.rs
# Must return 0
```

## Success Criteria
- [ ] Clean build with `USE_RUST_PLANETS=0` (no regression)
- [ ] Clean build with `USE_RUST_PLANETS=1` (Rust linked)
- [ ] FFI round-trip tests pass
- [ ] No placeholder code remains

## Failure Recovery
- rollback Rust: `git checkout -- rust/src/planets/ffi.rs rust/src/planets/generate.rs rust/src/planets/tests/ffi_tests.rs`
- rollback C: `git checkout -- sc2/src/uqm/planets/*.c sc2/src/uqm/rust_planets.* sc2/build/`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P12.md`
