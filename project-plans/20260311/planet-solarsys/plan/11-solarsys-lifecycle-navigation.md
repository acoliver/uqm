# Phase 11: Solar-System Lifecycle & Navigation

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P11`

## Prerequisites
- Required: Phase 10a (Orbit Menu Verification) completed
- Expected: All sub-flows (analysis, surface, scan, orbit) fully functional
- Required: Input subsystem for IP flight loop, Sound subsystem for space music
- Required: P09.5 feasibility outputs available for generation dispatch shape, global navigation accessor ownership, and persistence-window boundaries

## Requirements Implemented (Expanded)

### REQ-PSS-LIFECYCLE-001: Solar-system entry
**Requirement text**: Entering a solar system shall resolve the current star, update logged ship coordinates, establish fresh runtime state, select generation-function table, initialize the solar system, and enter the interplanetary flight loop.

Behavior contract:
- GIVEN: Player enters a star system from hyperspace
- WHEN: `explore_solar_sys()` is called
- THEN: State initialized, planets generated, IP flight loop running

### REQ-PSS-LIFECYCLE-002: Solar-system exit
**Requirement text**: On exit, uninitialize state, release resources, clear solar-system context.

Behavior contract:
- GIVEN: Player leaves system boundary
- WHEN: Flight loop terminates
- THEN: All solar-system state is released, no stale context remains

### REQ-PSS-LIFECYCLE-003: No overlapping sessions
**Requirement text**: At most one solar-system context shall be active at a time.

### REQ-PSS-LIFECYCLE-004: Solar-system load
**Requirement text**: Loading shall seed RNG, set up sun, generate planets, commit pending persistence, compute temp colors, sort planets, initialize inner/outer state based on saved position.

### REQ-PSS-NAV-001: Outer-to-inner transition
**Requirement text**: Approaching a planet transitions to inner system: generate moons, switch base descriptors, enter inner navigation.

### REQ-PSS-NAV-002: Inner-to-outer transition
**Requirement text**: Leaving inner system restores outer-system navigation.

### REQ-PSS-NAV-003: Interplanetary flight state
**Requirement text**: Track IP flight status, collision/orbit gating (WaitIntersect).

### REQ-PSS-NAV-004: Generation-function dispatch at init
**Requirement text**: The generation-function table shall be selected per star and used throughout the session.

### REQ-PSS-SAVE-001: Save-location encoding (outside orbit)
**Requirement text**: Outside orbit, delegate to non-orbital location saving.

### REQ-PSS-SAVE-002: Save-location encoding (in orbit)
**Requirement text**: In orbit, commit pending scan changes, encode orbital position (1=planet, 2+=moons).

### REQ-PSS-SAVE-003: Save-location restoration
**Requirement text**: On load with in-orbit indicator, decode to planet or moon descriptor.

### REQ-PSS-SAVE-004: Legacy save compatibility
**Requirement text**: Save files from baseline version load with identical world identity and retrieval state.

### REQ-PSS-PERSIST-005: Persistence write legality during load/save
**Requirement text**: Pending planetary-change commits during solar-system load and save-location encoding shall occur only within the legal host persistence window.

### REQ-PSS-PERSIST-006: Active context clearing and stale-state exclusion
**Requirement text**: After solar-system uninit, active solar-system context and related navigation pointers shall be cleared so stale state is inaccessible.

### REQ-PSS-PERSIST-007: Global navigation-state compatibility
**Requirement text**: The subsystem shall read and write `ip_planet`, `in_orbit`, and related global navigation state with baseline-compatible timing and values.

### REQ-PSS-PERSIST-008: NPC lifecycle and session exclusivity hooks
**Requirement text**: NPC init/reinit/uninit hooks and single-active-session enforcement shall be explicit implementation responsibilities, not end-to-end-only expectations.

## Implementation Tasks

### Files to modify

- `rust/src/planets/solarsys.rs` — Full lifecycle implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P11`
  - marker: `@requirement REQ-PSS-LIFECYCLE-001 through REQ-PSS-LIFECYCLE-004, REQ-PSS-PERSIST-005 through REQ-PSS-PERSIST-008`
  - Remove all `todo!()` stubs
  - Implement `SolarSysState` as an internal runtime struct; any FFI-visible state must go through explicit mirrors/conversions
  - Use explicit world identity types (`WorldRef`, `OrbitTarget`, or equivalent) instead of ambiguous raw indices where planet/moon distinction matters
  - Implement explicit single-active-session enforcement:
    - reject or assert on nested `explore_solar_sys()` entry while another `SolarSysState` is active
    - store/clear the active context in one authoritative place
  - Implement `explore_solar_sys()`:
    - Resolve current star (read from game state)
    - Update SIS coordinates
    - Create `SolarSysState::new()`
    - Call `get_generate_dispatch(star.index)` and install
    - Dispatch NPC init/reinit hook at the baseline-compatible lifecycle point for the session
    - Call `load_solar_sys()` to get optional orbit target
    - If orbit target: `enter_planet_orbit()`
    - Call `do_ip_flight()`
    - Call `uninit_solar_sys()`
  - Implement `load_solar_sys(state, star) -> Option<WorldRef>`:
    - Seed `SysGenRng`
    - Set up sun descriptor
    - Dispatch planet generation via the audited override/fallback wrapper
    - If not handled, call default planet generation
    - Handle pending `PLANETARY_CHANGE`
      - Verify this put occurs inside the host-guaranteed persistence window
      - Clear flag after put
    - Do planetary analysis + temp_color for each planet
    - Sort planets
    - Read global navigation state through the established accessor contract (`ip_planet`, `in_orbit`, related position globals)
    - Init outer or inner based on saved position
    - Decode orbit target if applicable
  - Implement `init_solar_sys()`, `uninit_solar_sys()`
  - Ensure `uninit_solar_sys()`:
    - dispatches NPC uninit at the baseline-compatible lifecycle point
    - clears the active solar-system context, current descriptor pointers, orbit target, and navigation-facing cached state
    - performs no persistence calls after uninit completes
  - Uses pseudocode lines: 190-247

- `rust/src/planets/navigation.rs` — Full navigation implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P11`
  - marker: `@requirement REQ-PSS-NAV-001 through REQ-PSS-NAV-004, REQ-PSS-PERSIST-007`
  - Remove all `todo!()` stubs
  - Implement `do_ip_flight(state: &mut SolarSysState)`:
    - Set `in_ip_flight = true`
    - Main loop: process input, update ship position
    - Check body intersections (respecting `WaitIntersect`)
    - In outer system: intersection triggers `enter_inner_system()`
    - In inner system: intersection triggers `enter_planet_orbit()`
    - Check system boundary exit
    - Render system view
    - Set `in_ip_flight = false` on exit
  - Implement `enter_inner_system(state, planet: &WorldRef)`:
    - Dispatch moon generation via the audited override/fallback wrapper
    - If not handled, call default moon generation
    - Switch base descriptors to MoonDesc
    - Set orbital target to the approached planet
    - Write baseline-compatible global navigation values for current planet / inner-system state
    - Transition to inner-system view
  - Implement `leave_inner_system(state)`:
    - Switch base descriptors back to PlanetDesc
    - Clear orbital target
    - Write baseline-compatible global navigation values for returning to outer system
    - Transition to outer-system view
  - Implement explicit global-state writes for orbit entry/exit transitions so `ip_planet`, `in_orbit`, and related navigation globals change at the same logical boundaries as baseline
  - Uses pseudocode lines: 380-419
  - Integration: Input subsystem for movement
  - Integration: Graphics for system rendering
  - Integration: Sound for space music

- `rust/src/planets/save_location.rs` — Full save-location implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P11`
  - marker: `@requirement REQ-PSS-SAVE-001 through REQ-PSS-SAVE-004, REQ-PSS-PERSIST-005, REQ-PSS-PERSIST-007`
  - Remove all `todo!()` stubs
  - Implement `save_solar_sys_location(state: &SolarSysState)`:
    - If not in orbit: delegate to `save_non_orbital_location()`
    - If in orbit:
      - If `PLANETARY_CHANGE` set: `put_planet_info()`, clear flag
      - Assert and verify this put occurs inside the legal host lifecycle window
      - Encode: planet=1, moon=1+moon_slot
      - Store `in_orbit` value in game globals
    - Preserve baseline-compatible writes to related navigation globals when saving outer/inner/orbit positions
  - Implement `decode_orbit_target(state: &SolarSysState) -> Option<WorldRef>`:
    - Read `in_orbit` value from game globals
    - 0 -> None
    - 1 -> current inner-system planet
    - 2+ -> moon slot corresponding to saved value
  - Uses pseudocode lines: 250-285

- `rust/src/planets/generate.rs` — Default generation behavior and dispatch scaffolding
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P11`
  - marker: `@requirement REQ-PSS-NAV-004, REQ-PSS-PERSIST-008`
  - Remove remaining `todo!()` only for default-generation behavior that is genuinely independent of final C wiring
  - Implement default planet generation (generic layout)
  - Implement default moon generation (generic moons)
  - Implement default name generation (baseline generic planet/moon naming fallback used when dedicated handlers do not override)
  - Implement default orbital generation (standard surface)
  - Implement default mineral/energy/life generation (baseline populations)
  - Implement explicit no-op defaults for NPC init, NPC reinit, NPC uninit, and node-pickup hooks so lifecycle ownership is encoded before final P12 wiring
  - Add unit coverage proving name-generation dispatch uses override/fallback semantics and that fallback naming remains deterministic for representative planet/moon cases
  - Keep per-star dedicated routing provisional until P12 finalizes actual C-table wiring; do not claim lifecycle parity is complete without that wiring

### C files being replaced
- `sc2/src/uqm/planets/solarsys.c` — entire file (~1900 lines)

### Files to create

- `rust/src/planets/tests/navigation_tests.rs` — Navigation state transition tests
  - Test outer-to-inner transition
  - Test inner-to-outer transition
  - Test `WaitIntersect` gating prevents re-entry
  - Test system boundary exit detection
  - Test baseline-compatible global navigation writes at outer→inner, inner→orbit, leave orbit, leave inner system, and leave solar system boundaries

- `rust/src/planets/tests/save_location_tests.rs` — Save-location tests
  - Test planet encoding: `in_orbit=1`
  - Test moon encoding with multiple moon slots
  - Test decode round-trip for planet
  - Test decode round-trip for each moon position
  - Test non-orbital save delegation
  - Test `PLANETARY_CHANGE` commit on save
  - Test legacy save value decoding (fixture from C)
  - Test persistence addressing parity across different moon-count layouts
  - Test baseline-compatible writes to `in_orbit` and related globals during save transitions

- `rust/src/planets/tests/persistence_window_tests.rs` — Host lifecycle boundary tests
  - Verify put on solar-system load occurs before teardown and inside legal host window
  - Verify save-location put occurs before session exit teardown
  - Verify no get/put is attempted after `uninit_solar_sys()` completes
  - Verify active context is cleared after uninit so stale state cannot be read through planets-owned access paths
  - Verify campaign transition boundary assumptions are documented and asserted in tests/mocks where feasible

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod navigation_tests;`, `mod save_location_tests;`, and `mod persistence_window_tests;`

## Pseudocode Traceability
- Implements pseudocode lines: 190-247, 250-285, 380-419

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets:: --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `solarsys.rs` fully implemented, no `todo!()`
- [ ] `navigation.rs` fully implemented, no `todo!()`
- [ ] `save_location.rs` fully implemented, no `todo!()`
- [ ] `generate.rs` default implementations are functional where independent of final C wiring
- [ ] All navigation, save-location, and persistence-window tests pass

## Semantic Verification Checklist
- [ ] `explore_solar_sys()` creates and tears down state cleanly
- [ ] Single-active-session enforcement prevents overlapping solar-system contexts
- [ ] `load_solar_sys()` seeds RNG, generates planets, computes temp colors
- [ ] Pending `PLANETARY_CHANGE` is committed before continuing init
- [ ] Planet sort matches C display ordering
- [ ] Orbit resumption from saved position works for both planets and moons
- [ ] `WaitIntersect` correctly gates re-entry to recently-left bodies
- [ ] Save-location encoding/decoding round-trips correctly
- [ ] Legacy save values decode correctly
- [ ] Persistence writes occur only inside the host-guaranteed legal window
- [ ] No get/put occurs after solar-system uninit
- [ ] Active context clearing prevents stale solar-system state access after exit
- [ ] `ip_planet`, `in_orbit`, and related global navigation values are written/read at baseline-compatible transition points
- [ ] NPC init/reinit/uninit ownership is explicit at the correct lifecycle points
- [ ] Default name-generation fallback exists and is used when dedicated handlers do not override
- [ ] Lifecycle parity claims are limited appropriately until dedicated per-star C generator dispatch is wired in P12

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/solarsys.rs rust/src/planets/navigation.rs rust/src/planets/save_location.rs rust/src/planets/generate.rs
# Must return 0 for completed modules; any remaining provisional dispatch hooks must be documented explicitly
```

## Success Criteria
- [ ] All `navigation_tests` pass
- [ ] All `save_location_tests` pass
- [ ] All `persistence_window_tests` pass
- [ ] Solar-system lifecycle creates/destroys cleanly
- [ ] No stale state after uninit
- [ ] No placeholder code remains in completed portions

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/solarsys.rs rust/src/planets/navigation.rs rust/src/planets/save_location.rs rust/src/planets/generate.rs rust/src/planets/tests/`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P11.md`
