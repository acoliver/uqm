# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P00.5`

## Purpose
Verify assumptions about toolchain, dependencies, types, call paths, existing ported subsystem interfaces, generation-handler semantics, and host-boundary obligations before implementation begins.

## Toolchain Verification

```bash
cargo --version
rustc --version
cargo clippy --version
```

- [ ] Rust toolchain is 1.75+ (required for `LazyLock` and other stable features)
- [ ] `parking_lot` crate present in `Cargo.toml`
- [ ] `serial_test` crate present in dev-dependencies

## Dependency Verification

- [ ] `libc` present in `Cargo.toml` (for FFI types)
- [ ] `thiserror` present in `Cargo.toml` (for error types)
- [ ] `anyhow` present in `Cargo.toml`
- [ ] No planet-specific feature flags needed initially (will add `USE_RUST_PLANETS` gating later)

## Type/Interface Verification

### Existing Rust State Subsystem
- [ ] `PlanetInfoManager` exists in `rust/src/state/planet_info.rs`
- [ ] `ScanRetrieveMask` exists with `mineral`, `energy`, `biological` fields
- [ ] `get_planet_info(star_index, planet_index, moon_index, planet_num_moons)` signature confirmed
- [ ] `put_planet_info(star_index, planet_index, moon_index, mask, planet_num_moons)` signature confirmed
- [ ] `StateFileManager` is accessible from other modules

### Existing Rust Graphics Subsystem
- [ ] `FrameHandle` / `FrameRegistry` accessible from `rust/src/graphics/frame.rs`
- [ ] `Canvas` and drawing primitives accessible from `rust/src/graphics/tfb_draw.rs`
- [ ] `DrawableRegistry` and `DrawableType` from `rust/src/graphics/drawable.rs`
- [ ] `RenderContext` from `rust/src/graphics/render_context.rs`
- [ ] `Coord`, `Extent`, `HotSpot`, `Point`, `Rect` types from `rust/src/graphics/drawable.rs`
- [ ] Color types available from graphics module

### Existing Rust Resource Subsystem
- [ ] Resource loading APIs accessible from `rust/src/resource/`
- [ ] String bank loading for planet names
- [ ] Colormap loading capabilities

### Existing Rust Input Subsystem
- [ ] `VControl` accessible from `rust/src/input/`
- [ ] Input loop driving mechanism available for IP flight and menu loops

### Existing Rust Sound Subsystem
- [ ] Music playback API accessible from `rust/src/sound/`

### C Side — Core Data Structures
- [ ] `PLANET_DESC` struct fields in `sc2/src/uqm/planets/planets.h:107-120`
  - `rand_seed: DWORD`, `data_index: BYTE`, `NumPlanets: BYTE`, `radius: SIZE`
  - `location: POINT`, `temp_color: Color`, `NextIndex: COUNT`, `image: STAMP`
  - `pPrevDesc: PLANET_DESC*`
- [ ] `STAR_DESC` struct fields in `sc2/src/uqm/planets/planets.h:122-128`
  - `star_pt: POINT`, `Type: BYTE`, `Index: BYTE`, `Prefix: BYTE`, `Postfix: BYTE`
- [ ] `NODE_INFO` struct fields in `sc2/src/uqm/planets/planets.h:130-148`
  - `loc_pt: POINT`, `density: COUNT`, `type: COUNT`
- [ ] `SOLARSYS_STATE` struct fields in `sc2/src/uqm/planets/planets.h:183-252`
- [ ] `PLANET_ORBIT` struct fields in `sc2/src/uqm/planets/planets.h:152-176`
- [ ] `GenerateFunctions` struct in `sc2/src/uqm/planets/generate.h:88-101`
- [ ] `SYSTEM_INFO` / `PLANET_INFO` structs accessible
- [ ] System limits: `MAX_SUNS=1`, `MAX_PLANETS=16`, `MAX_MOONS=4`

### C Side — Key Function Signatures
- [ ] `ExploreSolarSys(void)` in `solarsys.c`
- [ ] `LoadSolarSys(void)` returning `PLANET_DESC*` for orbit resumption
- [ ] `DoPlanetaryAnalysis(SYSTEM_INFO*, PLANET_DESC*)` in `calc.c`
- [ ] `GeneratePlanetSurface(PLANET_DESC*, FRAME)` in `plangen.c`
- [ ] `ScanSystem(void)` in `scan.c`
- [ ] `PlanetOrbitMenu(void)` in `planets.c`
- [ ] `SaveSolarSysLocation(void)` in `solarsys.c`
- [ ] `getGenerateFunctions(BYTE)` returning `const GenerateFunctions*`

### Generation-handler signature inventory
- [ ] Audit every handler slot in `GenerateFunctions` and classify it per spec §9.2: override/fallback, data-provider, or side-effect/integration hook
- [ ] Capture the exact C signature, return type, and parameter semantics for planet generation, moon generation, orbit content, name generation, mineral generation, energy generation, life generation, NPC init/reinit/uninit, and pickup hooks
- [ ] Confirm whether override/fallback handlers actually signal handled/not-handled via return value, out-param mutation, sentinel state, or another mechanism
- [ ] Confirm data-provider count-query vs. per-node-query conventions and any shared-state dependencies
- [ ] Audit representative dedicated generators from `sc2/src/uqm/planets/generate/` including Sol, at least one shielded world, and at least one encounter-triggering world
- [ ] Record any handler signatures or conventions that cannot be normalized safely; if found, revise the plan before P03 hardens interfaces

### C Side — Build Configuration
- [ ] Verify `USE_RUST_STATE` is defined and operational in `sc2/build/unix/build.config`
- [ ] Determine where to add `USE_RUST_PLANETS` toggle (likely `sc2/build/unix/build.config:86-107`)
- [ ] Verify conditional compilation pattern for `#ifndef USE_RUST_PLANETS` guards

### C Side — PlanData & Element Data
- [ ] `PlanData` array/structure in `sc2/src/uqm/planets/plandata.h` — world type parameters
- [ ] `ELEMENT_ENTRY` / element data in `sc2/src/uqm/planets/elemdata.h` — mineral types
- [ ] `LIFEFORM_ENTRY` / lifeform data in `sc2/src/uqm/planets/lifeform.h` — creature types
- [ ] `SunData` / star type data in `sc2/src/uqm/planets/sundata.h` — star energy values

## Type-model split verification

- [ ] Enumerate every boundary-crossing type that may need a `#[repr(C)]` mirror (`PLANET_DESC`, `STAR_DESC`, `NODE_INFO`, `PLANET_ORBIT`, `SYSTEM_INFO`, `PLANET_INFO`, save-location values, any callback argument structs)
- [ ] For each type, explicitly decide whether Rust will use:
  - an internal domain model only,
  - a C-layout mirror only, or
  - both with conversion functions
- [ ] Confirm that internal-only convenience types (`Option<usize>`, `Vec`, trait objects, owned handles) do not appear in boundary-adjacent APIs without a conversion layer
- [ ] Document candidate mirror file ownership and conversion boundaries for P03/P12

## Call-Path Feasibility

### Solar-system entry path
- [ ] Trace: game loop → `ExploreSolarSys()` → `LoadSolarSys()` → `DoIpFlight()`
- [ ] Identify the caller of `ExploreSolarSys()` — this is the C-to-Rust entry point
- [ ] Verify `pSolarSysState` global pointer pattern can be replaced with Rust-owned state

### Orbit entry path
- [ ] Trace: IP flight collision → `EnterPlanetOrbit()` → `GetPlanetInfo()` → `generateOrbital` → `LoadPlanet()` → `PlanetOrbitMenu()`
- [ ] Verify `GetPlanetInfo()` already routes to Rust via `USE_RUST_STATE`

### Scan flow path
- [ ] Trace: orbital menu → `ScanSystem()` → `GeneratePlanetSide()` → node materialization
- [ ] Verify `isNodeRetrieved()` reads from scan masks populated by `GetPlanetInfo()`

### Generation-function dispatch
- [ ] Trace: `getGenerateFunctions(CurStarDescPtr->Index)` → function table lookup
- [ ] Verify all 50+ system-specific generators are in `sc2/src/uqm/planets/generate/`
- [ ] Confirm they all follow `GenerateFunctions` struct contract

### Persistence path
- [ ] Trace: `PutPlanetInfo()` call sites in `LoadSolarSys()` and `SaveSolarSysLocation()`
- [ ] Verify `PLANETARY_CHANGE` flag semantics
- [ ] Confirm round-trip: put then get returns same masks

### Host lifecycle / persistence window path
- [ ] Trace the hosting-layer init/uninit path for planet-info persistence from campaign/session entry through solar-system teardown
- [ ] Identify the first legal `GetPlanetInfo`/`PutPlanetInfo` call site and the last legal call site per spec §10.1
- [ ] Confirm no solar-system path performs get/put after solar-system uninit or after host teardown begins
- [ ] Identify campaign transition boundaries that must flush pending writes before teardown (save, encounter exit, starbase entry, load into non-solar-system state)

## Decision checkpoints

- [ ] Persistence-addressing decision recorded explicitly: this parity port preserves current star/planet/moon semantic addressing exactly; no redesign or migration in scope
- [ ] Orbit-content interface cleanup deferred for parity unless the signature inventory proves a blocking mismatch

## Test Infrastructure Verification

- [ ] `cargo test --workspace --all-features` passes currently
- [ ] Existing planet_info tests in `rust/src/state/planet_info.rs` all pass
- [ ] `proptest` crate available for determinism property tests
- [ ] `insta` crate available for snapshot tests against C baseline
- [ ] `rstest` crate available for parameterized fixture tests

## Blocking Issues

[List any blockers discovered. If non-empty, stop and revise plan first.]

Potential blockers to check:
- If `RandomContext` (the C RNG) is not accessible via FFI, a compatible Rust RNG must be implemented first
- If `SYSTEM_INFO` / `PLANET_INFO` binary layouts are required by save/load, the Rust/C type-model split must be finalized before implementation
- If `getGenerateFunctions()` dispatch table is not accessible from Rust, a C-side shim layer is needed
- If generation handlers use mixed semantics that do not safely fit a single normalized Rust trait, the interface plan must be revised before P03
- If graphics context management APIs are not yet functional in the Rust graphics subsystem
- If host lifecycle guarantees for persistence are not observable or cannot be verified at call sites, add explicit host-boundary instrumentation before proceeding

## Gate Decision

- [ ] PASS: proceed to Phase 1
- [ ] FAIL: revise plan (document blocking issues)
