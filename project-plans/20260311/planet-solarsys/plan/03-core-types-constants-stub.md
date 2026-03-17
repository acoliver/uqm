# Phase 03: Core Types & Constants (Stub)

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Verify previous phase markers/artifacts exist
- Required: preflight signature inventory for generation handlers completed
- Required: type-model split decisions recorded for boundary-crossing structs

## Requirements Implemented (Expanded)

### REQ-PSS-TYPES-001: Planet descriptor model
**Requirement text**: Each planet and moon in a generated solar system shall be described by a descriptor carrying: a deterministic random seed, a data index, number of child bodies, orbital radius and position, temperature-derived display color, sort-ordering link, display stamp, and parent-body back-reference.

Behavior contract:
- GIVEN: A solar system is being generated
- WHEN: Planets or moons are created
- THEN: Each is represented by a `PlanetDesc` carrying all specified fields

### REQ-PSS-TYPES-002: System information model
**Requirement text**: The subsystem shall maintain per-world analysis data sufficient to represent the full set of planetary physical characteristics, scan-retrieval masks, and predefined surface data.

### REQ-PSS-TYPES-003: Node info model
**Requirement text**: Each surface node shall carry location, density, and type information as defined by the NODE_INFO structure.

### REQ-PSS-LIMITS-001: System limits
**Requirement text**: MAX_SUNS=1, MAX_PLANETS=16, MAX_MOONS=4, NUM_SCAN_TYPES=3, plus planet-side element frame slots.

### REQ-PSS-TYPES-004: Generation-function table contract
**Requirement text**: The subsystem shall be parameterized by a per-system generation-function table with handlers for NPC lifecycle, planet/moon/orbital/name generation, mineral/energy/life generation, and pickup hooks.

## Implementation Tasks

### Phase policy

This phase defines **provisional** Rust types and module boundaries only. Any boundary-adjacent type or generation-handler signature that depends on the C audit must be marked as provisional until P12 confirms the exact FFI shape.

### Files to create

- `rust/src/planets/mod.rs` — Module root
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - Declares all sub-modules, re-exports public types

- `rust/src/planets/constants.rs` — System limits and scaling constants
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - marker: `@requirement REQ-PSS-LIMITS-001`
  - Contains:
    - `MAX_SUNS: usize = 1`
    - `MAX_PLANETS: usize = 16`
    - `MAX_MOONS: usize = 4`
    - `NUM_SCAN_TYPES: usize = 3`
    - `MAX_LIFE_VARIATION: usize = 3`
    - `NUM_SCANDOT_TRANSITIONS: usize = 4`
    - `SCALE_RADIUS` / `UNSCALE_RADIUS` functions
    - `MAX_ZOOM_RADIUS`, `MIN_ZOOM_RADIUS`, `EARTH_RADIUS`
    - `MIN_PLANET_RADIUS`, `MAX_PLANET_RADIUS`
    - `DISPLAY_FACTOR`
    - `MIN_MOON_RADIUS`, `MOON_DELTA`
    - `MAP_WIDTH`, `MAP_HEIGHT`, `MAP_BORDER_HEIGHT`, `SCAN_SCREEN_HEIGHT`
    - `PLANET_ROTATION_TIME`, `PLANET_ROTATION_RATE`, `PLANET_ORG_Y`
    - `NUM_RACE_RUINS`
    - `MAX_SCROUNGED`
    - Disaster enum: `BIOLOGICAL_DISASTER`, `EARTHQUAKE_DISASTER`, etc.
    - Scan type enum/constants matching C `PlanetScanTypes`

- `rust/src/planets/types.rs` — Core data structures
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - marker: `@requirement REQ-PSS-TYPES-001, REQ-PSS-TYPES-002, REQ-PSS-TYPES-003`
  - Contains:
    - Domain structs only for internal Rust use, clearly separated from FFI mirrors
    - `PlanetDesc` domain struct carrying semantically required fields from `PLANET_DESC`
    - `StarDesc` domain struct carrying semantically required fields from `STAR_DESC`
    - `NodeInfo` domain struct carrying semantically required fields from `NODE_INFO`
    - `PlanetOrbit` domain struct for rendering assets; may use owned Rust containers because it is internal-only
    - `PlanetInfo` and `SystemInfo` domain structs for analysis/runtime state
    - Explicit world identity types such as `WorldRef`, `OrbitTarget`, or equivalent planet/moon-ref structs
    - `ScanType` enum: `Mineral = 0, Energy = 1, Biological = 2`
    - `Stamp` struct and graphics type re-exports as needed
    - `OrbitalOutcome`, `ScanOutcome`, `ScanRestriction`, `PlanetError`
    - Provisional `#[repr(C)]` mirrors only where preflight proved they will cross FFI:
      - `CPlanetDesc`
      - `CStarDesc`
      - `CNodeInfo`
      - `CPlanetInfo` / `CSystemInfo` as needed
    - Conversion function declarations or placeholders:
      - `impl From<&CPlanetDesc> for PlanetDesc` or explicit conversion fns
      - `to_c_*` / `from_c_*` stubs as appropriate
  - Design notes:
    - `Option<usize>`, `Vec`, trait objects, owned handles are internal-only and must not appear in any `#[repr(C)]` type
    - Back-reference representation in the domain model may differ from C, but any FFI mirror must preserve C layout exactly

- `rust/src/planets/generate.rs` — Generation-handler boundary (stub)
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - marker: `@requirement REQ-PSS-TYPES-004`
  - Contains:
    - Handler-class-specific abstractions based on preflight audit, not a single assumed trait contract
    - Provisional internal traits / enums / wrapper structs for:
      - override/fallback handlers
      - data-provider handlers
      - side-effect/integration hooks
    - `DefaultGenerateDispatch` or equivalent stub
    - `get_generate_dispatch(star_index: u8)` stub
    - Documentation that signatures remain tied to the audited C semantics and may require raw FFI wrappers in P12

- `rust/src/planets/rng.rs` — RNG wrapper (stub)
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - Contains: `SysGenRng` struct wrapping a deterministic RNG, with `seed()`, `random()` methods
  - Stub only: actual RNG compatibility implemented in P04

- `rust/src/planets/world_class.rs` — World classification (stub)
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - Contains function signatures for world identity helpers using explicit identity types rather than ambiguous raw indices where possible

- `rust/src/planets/calc.rs` — Planetary analysis (stub)
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - Contains: `do_planetary_analysis()` and `compute_temp_color()` signatures with `todo!()` bodies

- `rust/src/planets/solarsys.rs` — Solar-system state (stub)
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`
  - Contains: `SolarSysState` struct definition with internal-only fields and explicit notes about which fields are not ABI-facing
  - Contains: `explore_solar_sys()` stub

- `rust/src/planets/navigation.rs` — Navigation (stub)
  - Contains: `do_ip_flight()`, `enter_inner_system()`, `leave_inner_system()` stubs

- `rust/src/planets/orbit.rs` — Orbit entry (stub)
  - Contains: `enter_planet_orbit()`, `planet_orbit_menu()` stubs

- `rust/src/planets/scan.rs` — Scan flow (stub)
  - Contains: `scan_system()`, `generate_planet_side()` stubs

- `rust/src/planets/surface.rs` — Surface generation (stub)
  - Contains: `generate_planet_surface()` stub

- `rust/src/planets/gentopo.rs` — Topography generation (stub)
  - Contains: `delta_topography()` stub

- `rust/src/planets/render.rs` — Rendering helpers (stub)
  - Contains: sphere rotation, orbit drawing, oval stubs

- `rust/src/planets/save_location.rs` — Save-location (stub)
  - Contains: `save_solar_sys_location()`, `decode_orbit_target()` stubs

- `rust/src/planets/ffi.rs` — FFI bridge (stub)
  - Contains: placeholder for future FFI exports and explicit placeholder declarations for mirror-type conversions

- `rust/src/planets/tests/mod.rs` — Test module root

### Files to modify

- `rust/src/lib.rs`
  - Add `pub mod planets;`
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P03`

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 17 new files created under `rust/src/planets/`
- [ ] `lib.rs` updated with `pub mod planets`
- [ ] No skipped phases
- [ ] Plan/requirement traceability markers present in all files
- [ ] All stubs compile (no type errors)
- [ ] `types.rs` explicitly distinguishes internal domain structs from any `#[repr(C)]` mirrors
- [ ] `generate.rs` documents handler classes separately

## Semantic Verification Checklist
- [ ] `PlanetDesc` domain fields match C `PLANET_DESC` semantically
- [ ] `StarDesc` domain fields match C `STAR_DESC` semantically
- [ ] `NodeInfo` domain fields match C `NODE_INFO` semantically
- [ ] `PlanetOrbit` is clearly internal-only unless a mirror is explicitly justified
- [ ] `SolarSysState` is clearly internal-only and not described as ABI-compatible
- [ ] Any provisional `#[repr(C)]` mirrors are layout-focused and exclude Rust-only types
- [ ] Generation dispatch abstractions match the audited handler classes semantically
- [ ] System limits match C defines exactly
- [ ] Scan type enum values match C enum order (0, 1, 2)

## Deferred Implementation Detection

```bash
# Stubs are EXPECTED in this phase — todo!() is allowed
# Verify stubs exist and are well-formed
grep -RIn "todo!()" rust/src/planets/ | wc -l
# Should be > 0 (stubs present)
# Actual values will be removed in subsequent implementation phases
```

## Success Criteria
- [ ] `cargo build --workspace` succeeds
- [ ] All types compile with correct provisional field types
- [ ] Module structure matches plan
- [ ] No compilation errors
- [ ] No boundary-adjacent type is presented as ABI-compatible unless a `#[repr(C)]` mirror exists

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/ rust/src/lib.rs`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P03.md`
