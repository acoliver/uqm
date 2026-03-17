# Phase 13: End-to-End Integration & Parity Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P13`

## Prerequisites
- Required: Phase 12a (FFI Bridge Verification) completed
- Expected: Full build with `USE_RUST_PLANETS=1` links and compiles
- All unit/integration tests passing
- Required: legacy-save fixture corpus captured from baseline C runtime and stored under `rust/src/planets/tests/fixtures/legacy_saves/`

## Requirements Implemented (Expanded)

This phase does not implement new functionality. It verifies that the complete implementation satisfies all requirements through end-to-end testing and parity verification against the C baseline.

### Requirement coverage matrix for this phase
Full end-to-end parity verification covering:
- REQ-PSS-LIFECYCLE-*
- REQ-PSS-NAV-*
- REQ-PSS-ORBIT-*
- REQ-PSS-MENU-*
- REQ-PSS-SCAN-*
- REQ-PSS-NODES-*
- REQ-PSS-ANALYSIS-*
- REQ-PSS-SURFACE-*
- REQ-PSS-TYPES-* (observable effects only)
- REQ-PSS-RNG-*
- REQ-PSS-SAVE-*
- REQ-PSS-PERSIST-*
- REQ-PSS-FFI-*
- REQ-PSS-COMPAT-*

## Implementation Tasks

### Required verification corpus preparation

- Capture the minimum legacy-save fixture corpus from the baseline C build before parity signoff work begins in this phase
- Store the fixtures under `rust/src/planets/tests/fixtures/legacy_saves/`
- Required fixture set:
  - `rust/src/planets/tests/fixtures/legacy_saves/planet_orbit/` — player in orbit around a planet
  - `rust/src/planets/tests/fixtures/legacy_saves/moon_orbit/` — player in orbit around a moon
  - `rust/src/planets/tests/fixtures/legacy_saves/retrieved_nodes/` — previously retrieved nodes suppressed on load
  - `rust/src/planets/tests/fixtures/legacy_saves/pending_planetary_change/` — pending `PLANETARY_CHANGE` commits on reload
- Record the baseline capture harness/commands alongside the fixture corpus so regeneration is repeatable and parity evidence is inspectable
- Wire P11/P13 tests to consume fixtures from this path rather than ad hoc local saves

### End-to-end verification tests

- `rust/src/planets/tests/e2e_parity_tests.rs` — Parity verification suite
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P13`
  - marker: `@requirement REQ-PSS-LIFECYCLE-*, REQ-PSS-NAV-*, REQ-PSS-ORBIT-*, REQ-PSS-MENU-*, REQ-PSS-SCAN-*, REQ-PSS-NODES-*, REQ-PSS-ANALYSIS-*, REQ-PSS-SURFACE-*, REQ-PSS-RNG-*, REQ-PSS-SAVE-*, REQ-PSS-PERSIST-*, REQ-PSS-FFI-*, REQ-PSS-COMPAT-*`

  **1. Seeded reference system tests** (spec Appendix A.1):
  - Sol system: verify planet count, planet seeds, world types, orbital radii, moon counts
  - At least 3 generic systems with different star types (using default gen funcs)
  - At least 1 system with gas giant planets
  - At least 1 shielded world as an explicit named parity case
  - At least 1 encounter-triggering orbit-content or scan-trigger world as an explicit named parity case
  - Several dedicated-generation systems beyond Sol
  - Several moon-bearing planets with differing moon counts/layouts

  **2. Planetary analysis parity** (spec Appendix A.2):
  - For each reference system: all scalar analysis outputs match C baseline
  - Temperature, density, radius, gravity, rotation, tilt, tectonics, atmo, weather, life
  - Temperature-color matches including greenhouse quirk

  **3. Surface generation determinism** (spec Appendix A.2):
  - For representative reference worlds across generic and dedicated systems: topo_data matches C byte-for-byte
  - Sphere rendering assets are deterministic

  **4. Node population parity** (spec Appendix A.2):
  - For representative reference worlds: node counts, positions, types match C
  - Retrieval mask filtering verified
  - Shielded-world suppression verified explicitly
  - Encounter-capable scan-world behavior verified explicitly

  **5. Legacy save fixtures** (spec Appendix A.3):
  - Planet orbital restoration: load fixture from `rust/src/planets/tests/fixtures/legacy_saves/planet_orbit/` -> correct planet
  - Moon orbital restoration: load fixture from `rust/src/planets/tests/fixtures/legacy_saves/moon_orbit/` -> correct moon
  - Retrieved-node suppression: load fixture from `rust/src/planets/tests/fixtures/legacy_saves/retrieved_nodes/` -> correct suppression
  - Pending planetary-change commit: load fixture from `rust/src/planets/tests/fixtures/legacy_saves/pending_planetary_change/` -> changes committed

  **6. Round-trip verification** (spec Appendix A.4):
  - Save then reload: orbital target preserved
  - Save then reload: node suppression state preserved
  - No nodes reappear or disappear

  **7. Generation-handler integration** (spec Appendix A.5):
  - System-specific handler dispatched for correct star
  - Override suppresses default; non-override invokes default
  - Name-generation override/fallback semantics verified explicitly for representative dedicated and default systems
  - Representative planet-name outputs match baseline for selected dedicated systems and generic default-naming systems
  - Data-provider return values consumed for node population
  - Side-effect hooks dispatched at correct lifecycle points
  - Per-star dispatch identity compared directly for representative dedicated systems

  **8. Host lifecycle / persistence boundary verification**:
  - Verify get on orbit entry occurs inside the legal host window
  - Verify put on solar-system load and save-location encoding occurs inside the legal host window
  - Verify no get/put occurs after solar-system uninit
  - Verify session-exit/save boundary ordering with explicit evidence or harness assertions

  **9. Global navigation-state compatibility verification**:
  - Assert exact `ip_planet`, `in_orbit`, and related navigation-global values at:
    - system entry
    - outer→inner transition
    - inner→orbit transition
    - leave orbit
    - leave inner system
    - leave solar system
  - Verify active solar-system context is cleared after exit so stale reads are impossible through planets-owned access paths

  **10. Player-visible orbital/encounter automation coverage**:
  - Add automated integration coverage for orbit-menu re-entry after scan/device/cargo/roster/game sub-flows
  - Add automated integration coverage for encounter-trigger save-and-yield behavior from orbit and scan paths
  - Add automated integration coverage for selected dedicated-system paths that exercise non-default orbit-content behavior

### Runtime verification (manual)

- [ ] Boot game with `USE_RUST_PLANETS=1`
- [ ] Enter a solar system from hyperspace
- [ ] Verify planets visible in outer system view
- [ ] Navigate to planet, verify inner-system transition
- [ ] Enter orbit, verify orbital menu appears
- [ ] Verify rotating planet sphere animation
- [ ] Enter scan mode, verify scan display
- [ ] Verify mineral/energy/bio scan types work
- [ ] Verify node dots appear on scan display
- [ ] Verify shielded world suppresses node population and scan behavior appropriately
- [ ] Verify encounter-triggering world saves location and exits correctly to encounter handling
- [ ] Exit scan, exit orbit, verify return to inner system
- [ ] Exit inner system, verify return to outer system
- [ ] Exit solar system, verify return to hyperspace
- [ ] Save game while in orbit, reload, verify orbit resumption
- [ ] Visit planet, pick up nodes, leave, return — verify nodes stayed picked up
- [ ] Verify Sol system planets match expected layout and naming
- [ ] Verify at least several dedicated-generator systems beyond Sol behave correctly, including naming behavior where the baseline uses dedicated handlers

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod e2e_parity_tests;`

## Verification Commands

```bash
# Full Rust verification
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Focused parity tests
cargo test -p uqm --lib planets::tests::e2e_parity_tests --all-features -- --nocapture

# Full C+Rust build
cd sc2 && make clean && make

# No placeholder code anywhere in planets module
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/planets/
```

## Structural Verification Checklist
- [ ] All 17 Rust files in `rust/src/planets/` have no `todo!()` or `unimplemented!()`
- [ ] All test files pass
- [ ] Build succeeds with both `USE_RUST_PLANETS=0` and `USE_RUST_PLANETS=1`
- [ ] No duplicate implementations (no `*_v2` files)
- [ ] Plan/requirement traceability present throughout
- [ ] Legacy-save fixture corpus exists at `rust/src/planets/tests/fixtures/legacy_saves/` with all four required categories

## Semantic Verification Checklist

### Parity class verification (from spec Appendix A.5)

- [ ] **World layout**: Planet count, seeds, world-type indices, orbital params, moon counts match baseline
- [ ] **Planetary analysis**: All scalar outputs match baseline (temperature, density, radius, gravity, rotation, tilt, tectonics, atmo, weather, life). Temp-color includes greenhouse quirk
- [ ] **Surface generation**: Topo data deterministic per seed+algorithm. Byte-level match for reference worlds
- [ ] **Node population**: Per-scan-type counts, per-node locations/types/quantities match. Retrieval mask filtering correct
- [ ] **Persistence/save**: Orbital encoding round-trip, scan mask get/put round-trip, legacy save load, no node reappearance
- [ ] **Generation-handler integration**: Correct per-star dispatch, override/fallback preserved, explicit name-generation parity verified, data-provider consumed, side-effect hooks dispatched

### Appendix A minimum corpus compliance

- [ ] Several generic systems covered
- [ ] Several dedicated-generation systems beyond Sol covered
- [ ] Gas-giant world covered
- [ ] Shielded world covered explicitly
- [ ] Encounter-triggering world covered explicitly
- [ ] Several moon-bearing planets with differing moon counts covered explicitly
- [ ] Legacy-save fixtures captured from baseline C runtime and consumed from `rust/src/planets/tests/fixtures/legacy_saves/`

### Host-boundary compliance

- [ ] Persistence lifecycle-window obligations from spec §10.1 verified with explicit evidence
- [ ] No get/put after solar-system uninit verified
- [ ] Save/exit transition ordering verified

### Global-state compliance

- [ ] `ip_planet`, `in_orbit`, and related global navigation values match baseline at all major transitions
- [ ] Active solar-system context clearing prevents stale post-exit access

### Cross-subsystem integration

- [ ] Graphics integration: planet rendering, sphere rotation, orbit drawing all work
- [ ] Resource integration: planet frames, colormaps, strings load correctly
- [ ] State integration: `PlanetInfoManager` scan masks work end-to-end
- [ ] Input integration: IP flight and menu input loops work
- [ ] Sound integration: planet music plays correctly
- [ ] Comm integration: encounter transitions from orbit/scan work

### Quality gates

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-features` passes (all tests, not just planets)
- [ ] No regressions in other subsystems

## Definition of Done (Full Plan)

- [ ] All `cargo test` pass
- [ ] All `cargo clippy` pass
- [ ] `cargo fmt` clean
- [ ] Game boots with `USE_RUST_PLANETS=1` and solar-system exploration works
- [ ] Planetary analysis identical to C baseline
- [ ] Surface generation deterministic per seed
- [ ] Scan nodes match C baseline counts/positions/types
- [ ] Save/reload round-trip preserves orbit and node state
- [ ] Legacy saves load correctly
- [ ] Generation-function dispatch correct by handler class, including explicit name-generation parity
- [ ] Persistence host-lifecycle obligations verified
- [ ] Global navigation-state compatibility verified
- [ ] Greenhouse quirk preserved
- [ ] No `todo!()` / `unimplemented!()` / `FIXME` / `HACK` anywhere in `rust/src/planets/`

## Success Criteria
- [ ] All parity tests pass
- [ ] Manual runtime verification passes
- [ ] All Definition of Done items satisfied
- [ ] Plan complete

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P13.md`
