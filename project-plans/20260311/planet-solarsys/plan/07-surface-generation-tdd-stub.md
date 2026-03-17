# Phase 07: Surface Generation (TDD + Stub)

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P07`

## Prerequisites
- Required: Phase 06a (Analysis Impl Verification) completed
- Expected: Planetary analysis fully functional, RNG operational

## Requirements Implemented (Expanded)

### REQ-PSS-SURFACE-001: Surface generation flow
**Requirement text**: Generating a planet's surface shall seed generation from the world seed, initialize orbit rendering buffers, and produce topography using the planet's algorithm classification.

Behavior contract:
- GIVEN: A planet descriptor with seed 0x12345678 and data_index indicating a rocky world
- WHEN: `generate_planet_surface()` is called
- THEN: Topography data is populated deterministically, orbit rendering assets are initialized

### REQ-PSS-SURFACE-002: Algorithm selection
**Requirement text**: Algorithm selection based on world data_index: gas-giant, topographic, or cratered.

Behavior contract:
- GIVEN: A gas giant data_index
- WHEN: Algorithm is selected
- THEN: Gas giant generation algorithm is used

### REQ-PSS-SURFACE-003: Predefined surface support
**Requirement text**: When predefined surface/elevation data is supplied, use it instead of procedural generation.

### REQ-PSS-SURFACE-004: Surface determinism
**Requirement text**: Surface generation for a given world seed shall produce deterministic topography, elevation data, and sphere rendering assets.

## Implementation Tasks

### Files to create

- `rust/src/planets/tests/surface_tests.rs` — Surface generation TDD tests
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P07`
  - marker: `@requirement REQ-PSS-SURFACE-001, REQ-PSS-SURFACE-002, REQ-PSS-SURFACE-004`
  - Test categories:
    1. **Determinism tests**: Same seed produces same topo_data byte-for-byte
    2. **Algorithm selection tests**: gas giant index -> gas giant algo, rocky index -> topo algo, etc.
    3. **DeltaTopography tests**: Known seed + iterations produce known elevation pattern
    4. **Predefined surface tests**: Supplying surf_def_frame bypasses generation
    5. **Dimension tests**: Generated topo_data has correct MAP_WIDTH x MAP_HEIGHT dimensions
    6. **Fixture tests**: Capture C topo_data for 3-5 reference worlds, verify Rust matches

### Files to modify

- `rust/src/planets/surface.rs` — Flesh out stubs with signatures ready for impl
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P07`
  - Define `SurfaceAlgorithm` enum: `GasGiant, Topographic, Cratered`
  - Define `select_algorithm(data_index: u8) -> SurfaceAlgorithm`
  - Refine `generate_planet_surface()` signature:
    ```rust
    pub fn generate_planet_surface(
        planet: &PlanetDesc,
        orbit: &mut PlanetOrbit,
        surf_def_frame: Option<FrameHandle>,
        rng: &mut SysGenRng,
    ) -> Result<(), PlanetError>
    ```

- `rust/src/planets/gentopo.rs` — Refine delta topography signature
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P07`
  - Define `delta_topography()` signature:
    ```rust
    pub fn delta_topography(
        num_iterations: u32,
        depth_array: &mut [i8],
        rect_width: u16,
        rect_height: u16,
        depth_delta: i8,
        rng: &mut SysGenRng,
    )
    ```

- `rust/src/planets/tests/mod.rs`
  - Add `mod surface_tests;`

## Pseudocode Traceability
- Tests target pseudocode lines: 050-082

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Surface tests expected to fail (TDD phase)
```

## Structural Verification Checklist
- [ ] `surface_tests.rs` created with comprehensive test cases
- [ ] `surface.rs` signatures refined (but still `todo!()` bodies)
- [ ] `gentopo.rs` signature refined (but still `todo!()` body)
- [ ] Fixture data for reference surfaces captured from C

## Semantic Verification Checklist
- [ ] Tests cover all three algorithm types
- [ ] Tests verify byte-level determinism for topo_data
- [ ] Tests verify predefined surface bypass
- [ ] Tests reference REQ-PSS-SURFACE-* IDs

## Success Criteria
- [ ] Test file compiles
- [ ] Tests structured to verify real behavior once implemented
- [ ] Tests currently fail (expected)

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P07.md`
