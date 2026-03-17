# Phase 08: Surface Generation & Rendering (Implementation)

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P08`

## Prerequisites
- Required: Phase 07a (Surface Gen TDD Verification) completed
- Expected: surface_tests.rs compiling with failing tests
- Required: Graphics subsystem APIs functional (FrameHandle, Canvas, drawing primitives)

## Requirements Implemented (Expanded)

### REQ-PSS-SURFACE-001 through REQ-PSS-SURFACE-004
(See Phase 07 for full requirement text)

### REQ-PSS-RENDER-001: Sphere rotation rendering
**Requirement text**: The orbital menu shall display a rotating planet sphere as a background element, using generated topography and rendering assets.

### REQ-PSS-RENDER-002: Orbit drawing
**Requirement text**: Planet and moon orbits shall be drawn as elliptical paths in the solar-system and inner-system views.

### REQ-PSS-RENDER-003: Oval primitives
**Requirement text**: Filled and outlined oval drawing for orbit visualization.

## Implementation Tasks

### Files to modify

- `rust/src/planets/surface.rs` — Full surface generation implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P08`
  - marker: `@requirement REQ-PSS-SURFACE-001, REQ-PSS-SURFACE-002, REQ-PSS-SURFACE-003, REQ-PSS-SURFACE-004`
  - Remove all `todo!()` stubs
  - Implement `generate_planet_surface()`:
    - Seed RNG from planet's rand_seed
    - Initialize PlanetOrbit buffers (topo_data allocation, color arrays)
    - Handle predefined surface frame (load from surf_def_frame)
    - Select algorithm from data_index
    - Dispatch to gas_giant/topo/cratered generation
    - Call `render_topography_frame()` to produce TopoFrame
    - Call `build_sphere_assets()` to produce SphereFrame
  - Implement `init_planet_orbit_buffers()` — allocate topo_data, color arrays
  - Implement `render_topography_frame()` — convert elevation to colored frame
  - Implement `build_sphere_assets()` — generate 3D sphere rotation frames
  - Uses pseudocode lines: 050-068

- `rust/src/planets/gentopo.rs` — Full topography implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P08`
  - marker: `@requirement REQ-PSS-SURFACE-004`
  - Remove `todo!()` stub
  - Implement `delta_topography()` — Bresenham-style random bisecting line algorithm
  - Implement gas giant, topo, and cratered generation variants
  - Uses pseudocode lines: 070-082

- `rust/src/planets/render.rs` — Rendering helpers implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P08`
  - marker: `@requirement REQ-PSS-RENDER-001, REQ-PSS-RENDER-002, REQ-PSS-RENDER-003`
  - Remove `todo!()` stubs
  - Implement:
    - `init_sphere_rotation(direction: i32, shielded: bool)` — set up rotation state
    - `uninit_sphere_rotation()` — tear down rotation assets
    - `prepare_next_rotation_frame()` — advance sphere animation
    - `draw_planet_sphere(x: i32, y: i32)` — render sphere at position
    - `draw_default_planet_sphere()` — render at default orbital position
    - `render_planet_sphere(frame: FrameHandle, offset: i32, do_throb: bool)`
    - `zoom_in_planet_sphere()` — zoom transition animation
    - `rotate_planet_sphere(keep_rate: bool)` — continuous rotation
    - `draw_oval(rect: &Rect, num_off_pixels: u8)` — outlined ellipse
    - `draw_filled_oval(rect: &Rect)` — filled ellipse
    - `fill_orbits(state: &SolarSysState, num_planets: u8, base_desc: &[PlanetDesc], types_defined: bool)` — draw orbital paths
    - `draw_star_background()` — render star backdrop
    - `location_to_display(pt: Point, scale_radius: i16) -> Point` — coordinate transform
    - `display_to_location(pt: Point, scale_radius: i16) -> Point` — reverse transform
    - `planet_outer_location(planet_i: u16) -> Point`
  - Integration: Uses `Canvas` and drawing primitives from `rust/src/graphics/tfb_draw.rs`
  - Integration: Uses `FrameHandle` and `FrameRegistry` from `rust/src/graphics/frame.rs`

### C files being replaced
- `sc2/src/uqm/planets/plangen.c` — surface generation (~1815 lines)
- `sc2/src/uqm/planets/gentopo.c` — topography deltas (~200 lines)
- `sc2/src/uqm/planets/surface.c` — surface rendering helpers (~150 lines)
- `sc2/src/uqm/planets/orbits.c` — orbit rendering (~300 lines)
- `sc2/src/uqm/planets/oval.c` — oval primitives (~100 lines)
- `sc2/src/uqm/planets/pl_stuff.c` — planet display (~200 lines)

## Pseudocode Traceability
- Implements pseudocode lines: 050-082

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::surface_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `surface.rs` fully implemented, no `todo!()`
- [ ] `gentopo.rs` fully implemented, no `todo!()`
- [ ] `render.rs` fully implemented, no `todo!()`
- [ ] All surface_tests pass

## Semantic Verification Checklist
- [ ] Topo data for reference worlds matches C output byte-for-byte
- [ ] Gas giant algorithm produces distinct output from topo algorithm
- [ ] Predefined surface bypass works correctly
- [ ] Sphere rotation produces renderable frames
- [ ] Oval drawing produces correct pixel patterns
- [ ] Coordinate transforms are reversible (location_to_display then display_to_location)

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/surface.rs rust/src/planets/gentopo.rs rust/src/planets/render.rs
# Must return 0
```

## Success Criteria
- [ ] All surface_tests pass
- [ ] Topo determinism verified against C fixtures
- [ ] Rendering helpers compile and integrate with graphics subsystem
- [ ] No placeholder code remains

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/surface.rs rust/src/planets/gentopo.rs rust/src/planets/render.rs`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P08.md`
