# Phase 10: Orbit Entry & Orbital Menu

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P10`

## Prerequisites
- Required: Phase 09.5a (Dispatch / Global-Access Feasibility Verification) completed
- Expected: Scan flow functional, surface generation functional, rendering helpers functional
- Required: Input subsystem APIs accessible for menu input loop
- Required: P09.5 feasibility deliverables checked in for orbit-content handler semantics, external menu dispatch shape, global navigation access, and node-pickup callback route

## Requirements Implemented (Expanded)

### REQ-PSS-ORBIT-001: Orbit entry sequence
**Requirement text**: Entering orbit shall: free IP flight assets if collision-triggered, position ship stamp, load persisted scan state, dispatch orbit-content hook, check activity interrupts, check orbital readiness (topography frame exists), perform planet loading if ready, present orbital menu.

Behavior contract:
- GIVEN: Player ship intersects a planet in inner system
- WHEN: `enter_planet_orbit()` is called
- THEN: Scan state loaded, orbital generation dispatched, readiness checked, menu entered if topo exists

### REQ-PSS-ORBIT-002: Orbital readiness gating
**Requirement text**: If no renderable topography exists after orbit-content processing, or a non-orbital interaction was initiated, skip the orbital menu entirely.

Behavior contract:
- GIVEN: A homeworld whose orbit-content hook triggers an encounter
- WHEN: Orbit entry completes orbit-content processing
- THEN: Activity interrupt detected, orbital menu NOT entered

### REQ-PSS-ORBIT-003: Planet loading
**Requirement text**: Planet loading encompasses surface-node materialization, music setup, orbital display preparation.

### REQ-PSS-ORBIT-004: Post-orbit system reload
**Requirement text**: After leaving orbit without an activity interrupt, reload the solar system and revalidate orbital state.

### REQ-PSS-MENU-001: Orbital menu actions
**Requirement text**: The orbital menu shall present: scan, equip device, cargo, roster, game menu, starmap, and navigation.

Behavior contract:
- GIVEN: Player is in orbit with orbital menu visible
- WHEN: Player selects "Scan"
- THEN: `scan_system()` is called

### REQ-PSS-MENU-002: Rotating planet display
**Requirement text**: While in the orbital menu, display a rotating-planet visual consistent with current world's topography.

### REQ-PSS-MENU-003: External menu dispatch
**Requirement text**: Equip device, cargo, roster, and game menu dispatch to external subsystems and return to orbital menu.

### REQ-PSS-PERSIST-003: Orbit-entry persistence load timing
**Requirement text**: Orbit entry shall load scan state before orbit-content processing and only within the legal persistence window.

### REQ-PSS-PERSIST-004: Node-pickup integration ownership at orbit boundary
**Requirement text**: The orbit/scan flow shall preserve the callback route that allows later lander pickup events to update retrieval state through the subsystem-owned hook path.

## Implementation Tasks

### Files to modify

- `rust/src/planets/orbit.rs` — Full orbit entry and menu implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P10`
  - marker: `@requirement REQ-PSS-ORBIT-001 through REQ-PSS-ORBIT-004, REQ-PSS-MENU-001 through REQ-PSS-MENU-003, REQ-PSS-PERSIST-003 through REQ-PSS-PERSIST-004`
  - Remove all `todo!()` stubs
  - Implement `enter_planet_orbit(state: &mut SolarSysState, target: &WorldRef) -> OrbitalOutcome`:
    - Free IP flight assets if collision-triggered entry
    - Position ship stamp (mid-screen for planets, body origin for moons)
    - Load scan masks via `PlanetInfoManager::get_planet_info()` using explicit persistence identity established by the P09.5 feasibility spike
    - Dispatch orbit-content processing via the audited override/fallback wrapper validated in P09.5
    - If the hook reports not-handled, call default orbital generation
    - Check activity interrupts: abort, load, encounter, crew loss
    - If interrupted, return `OrbitalOutcome::Interrupted`
    - Check observable readiness via `state.topo_frame.is_some()`
    - If not ready, return `OrbitalOutcome::NoTopo`
    - Call `load_planet()` for the broader planet-loading phase
    - Call `planet_orbit_menu()`
    - Call `free_planet()`
    - Reload solar system, revalidate orbits
    - Return `OrbitalOutcome::Normal`
  - Implement `load_planet(state: &mut SolarSysState, target: &WorldRef, surf_def_frame: Option<FrameHandle>)`:
    - Create planet context
    - Draw wait-mode orbital UI (optional)
    - Stop current music
    - Call `generate_planet_surface()`
    - Set planet music
    - Call `generate_planet_side()` for node materialization
    - Preserve the route by which later lander pickup events can be reported back into planets-owned retrieval-state and generation-hook handling
    - Play lander music if applicable
    - Update orbital display
  - Implement `free_planet(state: &mut SolarSysState)`:
    - Free planet context, frames, orbit buffers
    - Reset orbital rendering state
  - Implement `planet_orbit_menu(state: &mut SolarSysState, target: &WorldRef) -> OrbitalOutcome`:
    - Set up rotating planet display (callback to `rotate_planet_sphere`)
    - Menu input loop dispatching to:
      - `SCAN` -> `scan_system(state, target)`
      - `EQUIP_DEVICE` -> FFI to C `DevicesMenu()` (external, not ported)
      - `CARGO` -> FFI to C `CargoMenu()` (external, not ported)
      - `ROSTER` -> FFI to C `RosterMenu()` (external, not ported)
      - `GAME_MENU` -> FFI to C `GameOptions()` (external)
      - `STARMAP` / `NAVIGATION` -> break loop, return leave-orbit
    - Re-enter orbital menu after scan/device/cargo/roster/game sub-flows unless an activity interrupt or leave-orbit action terminates the session
  - Uses pseudocode lines: 130-182
  - Integration: `PlanetInfoManager` for scan mask loading
  - Integration: `scan_system()` from `scan.rs`
  - Integration: `generate_planet_surface()` from `surface.rs`
  - Integration: `generate_planet_side()` from `scan.rs`
  - Integration: Sphere rendering from `render.rs`
  - Integration: Input loop from `rust/src/input/`
  - Integration: Music from `rust/src/sound/`
  - Integration: External menus (cargo, devices, roster, game) via FFI to C

### C files being replaced
- `sc2/src/uqm/planets/planets.c` — orbit menu, planet load/free (~483 lines)
- `sc2/src/uqm/planets/cargo.c` — dispatch entry point only (menu impl stays C)
- `sc2/src/uqm/planets/devices.c` — dispatch entry point only
- `sc2/src/uqm/planets/roster.c` — dispatch entry point only
- `sc2/src/uqm/planets/lander.c` — pickup callback route only (lander gameplay stays C)

### Files to create

- `rust/src/planets/tests/orbit_tests.rs` — Orbit entry tests (created alongside implementation, not separate TDD)
  - Test orbit entry with renderable topography -> menu entered
  - Test orbit entry without topography -> menu skipped
  - Test orbit entry with encounter trigger -> interrupted
  - Test post-orbit reload behavior
  - Test menu action dispatch (mock input)
  - Test per-world persistence identity used for orbit entry get operations
  - Test dedicated orbit-content override vs. default fallback semantics using audited wrappers
  - Test menu loop returns to orbit after scan/device/cargo/roster/game sub-flows
  - Test node-pickup callback route remains connected for later lander-originated events

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod orbit_tests;`

## Pseudocode Traceability
- Implements pseudocode lines: 130-182

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::orbit_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `orbit.rs` fully implemented, no `todo!()`
- [ ] `orbit_tests.rs` created with passing tests
- [ ] External menu dispatch uses FFI (not reimplemented)

## Semantic Verification Checklist
- [ ] Scan masks loaded BEFORE orbit-content processing
- [ ] Override/fallback dispatch follows the audited orbit-content semantics rather than an assumed boolean-only contract
- [ ] Activity interrupts prevent orbital menu entry
- [ ] Topography frame check gates planet loading
- [ ] Planet loading calls surface generation and node materialization
- [ ] Rotating planet display is maintained during menu
- [ ] Menu loop returns to orbit after sub-flows unless leaving orbit or interrupted
- [ ] Post-orbit reload correctly reinitializes system state
- [ ] Per-world persistence addressing is preserved for both planets and moons
- [ ] Orbit-entry persistence get stays inside the host-guaranteed legal window
- [ ] Node-pickup integration route remains explicitly owned by the planets subsystem boundary

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/orbit.rs
# Must return 0
```

## Success Criteria
- [ ] All `orbit_tests` pass
- [ ] Orbit entry sequence matches spec §5.1 exactly
- [ ] Menu actions dispatch correctly
- [ ] No placeholder code remains

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/orbit.rs rust/src/planets/tests/orbit_tests.rs`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P10.md`
