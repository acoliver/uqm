# Plan: Planet-SolarSys Subsystem Port (C → Rust)

Plan ID: PLAN-20260314-PLANET-SOLARSYS
Generated: 2026-03-14
Total Phases: 31 (P00.5 through P13, including P09.5/P09.5a dispatch-global-access feasibility spike)
Requirements: REQ-PSS-* (from requirements.md)

## Context

The planet-solarsys subsystem is **unported** — all 16 C files in `sc2/src/uqm/planets/` remain the authoritative runtime for solar-system exploration, orbit entry, scan flow, planet surface generation, planetary analysis, and generation-function dispatch. No Rust implementation exists for any gameplay logic.

The only Rust edge is the **planet-info persistence bridge** (`GetPlanetInfo`/`PutPlanetInfo`) which already lives in `rust/src/state/planet_info.rs` behind `USE_RUST_STATE`. This plan ports the remaining gameplay subsystem to Rust, integrating with already-ported graphics, resource, state, and threading subsystems.

This is a large subsystem (~8,000 lines of C across 16 source files). The plan decomposes it into 12 implementation slices (P03–P13, with P09.5 as an explicit feasibility spike) that progress from foundational data types through core algorithms to full lifecycle integration.

## C Files Being Replaced

| C File | Purpose | Plan Phase |
|--------|---------|------------|
| `sc2/src/uqm/planets/calc.c` | Planetary analysis formulas | P05–P06 |
| `sc2/src/uqm/planets/plangen.c` | Surface generation / topography | P07–P08 |
| `sc2/src/uqm/planets/gentopo.c` | Topography delta generation | P07–P08 |
| `sc2/src/uqm/planets/surface.c` | Surface rendering helpers | P07–P08 |
| `sc2/src/uqm/planets/scan.c` | Scan UI and node materialization | P09 |
| `sc2/src/uqm/planets/planets.c` | Orbit menu, planet load/free | P10 |
| `sc2/src/uqm/planets/solarsys.c` | Solar-system lifecycle, IP flight | P11 |
| `sc2/src/uqm/planets/orbits.c` | Orbit rendering/drawing | P08 |
| `sc2/src/uqm/planets/oval.c` | Oval drawing primitives | P08 |
| `sc2/src/uqm/planets/pl_stuff.c` | Planet display helpers | P08 |
| `sc2/src/uqm/planets/report.c` | Coarse-scan report display | P09 |
| `sc2/src/uqm/planets/pstarmap.c` | Planetary starmap (out of scope, stub only) | P03 |
| `sc2/src/uqm/planets/cargo.c` | Cargo menu (integration boundary) | P10 |
| `sc2/src/uqm/planets/devices.c` | Devices menu (integration boundary) | P10 |
| `sc2/src/uqm/planets/roster.c` | Roster menu (integration boundary) | P10 |
| `sc2/src/uqm/planets/lander.c` | Lander gameplay (out of scope, integration hooks only) | P09, P11, P12 |

## New Rust Module Structure

```
rust/src/planets/
  mod.rs                    — Module root, re-exports
  types.rs                  — Core data types and FFI/domain model split
  constants.rs              — System limits, scaling constants
  rng.rs                    — System generation RNG wrapper
  calc.rs                   — Planetary analysis (DoPlanetaryAnalysis)
  generate.rs               — Generation handler semantics + dispatch wrappers
  solarsys.rs               — SolarSysState, exploration lifecycle
  navigation.rs             — IP flight, inner/outer system transitions
  orbit.rs                  — Orbit entry, orbital menu flow
  scan.rs                   — Scan mode, node materialization, scan display
  surface.rs                — Planet surface generation (topography, sphere)
  gentopo.rs                — Topography delta generation algorithm
  render.rs                 — Orbit rendering, oval drawing, planet display
  save_location.rs          — Save-location encoding/decoding
  world_class.rs            — World classification helpers (isPlanet, isMoon, etc.)
  ffi.rs                    — FFI bridge (C→Rust and Rust→C)
  tests/
    mod.rs
    calc_tests.rs           — Planetary analysis fixture tests
    generate_tests.rs       — Generation dispatch tests
    ffi_tests.rs            — FFI round-trip and marshaling tests
    surface_tests.rs        — Topography determinism tests
    navigation_tests.rs     — IP flight state transition tests
    scan_tests.rs           — Node materialization tests
    persistence_tests.rs    — Scan-state round-trip tests
    persistence_window_tests.rs — Host lifecycle-window and teardown tests
    save_location_tests.rs  — Save-location encoding tests
    orbit_tests.rs          — Orbit-entry gating and dispatch tests
    e2e_parity_tests.rs     — Seeded parity corpus tests
```

## Integration Points with Ported Subsystems

| Ported Subsystem | Integration | Direction |
|-----------------|-------------|-----------|
| **Graphics** (`rust/src/graphics/`) | Frame allocation, drawable management, context ops, sphere rendering, TFB drawing | Planets → Graphics |
| **Resource** (`rust/src/resource/`) | Asset loading (planet frames, colormaps, string banks) | Planets → Resource |
| **State** (`rust/src/state/`) | PlanetInfoManager for scan mask get/put (already ported) | Planets → State |
| **Threading** (`rust/src/threading/`) | RNG isolation, thread-safe state access | Planets → Threading |
| **Input** (`rust/src/input/`) | Input loop driving for IP flight and orbital menu | Planets → Input |
| **Sound** (`rust/src/sound/`) | Planet music selection and playback | Planets → Sound |
| **Comm** (`rust/src/comm/`) | Encounter transitions from orbit/scan | Planets ↔ Comm |
| **Campaign/gameplay host** | Persistence init/uninit lifecycle guarantee, session teardown ordering | Host → Planets |

## Requirement Families

| Family | Scope |
|--------|-------|
| `REQ-PSS-TYPES-*` | Core type/model and world identity semantics |
| `REQ-PSS-LIMITS-*` | System limits and bounds |
| `REQ-PSS-RNG-*` | Deterministic RNG and seed handling |
| `REQ-PSS-WORLD-*` | Planet/moon classification and indexing |
| `REQ-PSS-ANALYSIS-*` | Planetary analysis fidelity |
| `REQ-PSS-SURFACE-*` | Surface generation determinism and rendering assets |
| `REQ-PSS-RENDER-*` | Orbit/surface rendering helpers |
| `REQ-PSS-SCAN-*` | Scan flow and restrictions |
| `REQ-PSS-NODES-*` | Node materialization and node attributes |
| `REQ-PSS-ORBIT-*` | Orbit-entry sequencing and gating |
| `REQ-PSS-MENU-*` | Orbital-menu actions and return behavior |
| `REQ-PSS-LIFECYCLE-*` | Solar-system session lifecycle |
| `REQ-PSS-NAV-*` | Outer/inner/orbit navigation state transitions |
| `REQ-PSS-SAVE-*` | Save-location encoding and restoration |
| `REQ-PSS-PERSIST-*` | Persistence-window legality, addressing consistency, and host-lifecycle boundary obligations |
| `REQ-PSS-FFI-*` | FFI entrypoints and bridge obligations |
| `REQ-PSS-COMPAT-*` | ABI and behavioral compatibility obligations |

## Phase Structure

| Phase | Title | C Files Addressed | Requirements | Est. LoC |
|-------|-------|-------------------|-------------|----------|
| P00.5 | Preflight Verification | — | — | 0 |
| P01 | Analysis | — | All | 0 |
| P01a | Analysis Verification | — | — | 0 |
| P02 | Pseudocode | — | All | 0 |
| P02a | Pseudocode Verification | — | — | 0 |
| P03 | Core Types & Constants (Stub) | planets.h, generate.h, elemdata.h, plandata.h, sundata.h, lifeform.h | REQ-PSS-TYPES-*, REQ-PSS-LIMITS-* | ~800 |
| P03a | Core Types Verification | — | — | 0 |
| P04 | RNG & World Classification (TDD+Impl) | solarsys.c (helpers), calc.c (seed) | REQ-PSS-RNG-*, REQ-PSS-WORLD-* | ~400 |
| P04a | RNG & Classification Verification | — | — | 0 |
| P05 | Planetary Analysis (TDD) | calc.c | REQ-PSS-ANALYSIS-* | ~300 |
| P05a | Analysis TDD Verification | — | — | 0 |
| P06 | Planetary Analysis (Impl) | calc.c | REQ-PSS-ANALYSIS-* | ~600 |
| P06a | Analysis Impl Verification | — | — | 0 |
| P07 | Surface Generation (TDD+Stub) | plangen.c, gentopo.c | REQ-PSS-SURFACE-* | ~400 |
| P07a | Surface Gen Stub Verification | — | — | 0 |
| P08 | Surface Generation & Rendering (Impl) | plangen.c, gentopo.c, surface.c, orbits.c, oval.c, pl_stuff.c | REQ-PSS-SURFACE-*, REQ-PSS-RENDER-* | ~1200 |
| P08a | Surface Gen Impl Verification | — | — | 0 |
| P09 | Scan Flow & Node Materialization | scan.c, report.c, lander.c (hooks only) | REQ-PSS-SCAN-*, REQ-PSS-NODES-*, REQ-PSS-PERSIST-* | ~1000 |
| P09a | Scan Flow Verification | — | — | 0 |
| P09.5 | Dispatch / Global-Access Feasibility Spike | solarsys.c, planets.c, scan.c, generate/*.c (wiring audit only) | REQ-PSS-NAV-*, REQ-PSS-LIFECYCLE-*, REQ-PSS-PERSIST-*, REQ-PSS-FFI-* | ~250 |
| P09.5a | Dispatch / Global-Access Feasibility Verification | — | — | 0 |
| P10 | Orbit Entry & Orbital Menu | planets.c, cargo.c, devices.c, roster.c, lander.c (pickup dispatch path) | REQ-PSS-ORBIT-*, REQ-PSS-MENU-*, REQ-PSS-PERSIST-* | ~800 |
| P10a | Orbit Menu Verification | — | — | 0 |
| P11 | Solar-System Lifecycle & Navigation | solarsys.c | REQ-PSS-LIFECYCLE-*, REQ-PSS-NAV-*, REQ-PSS-SAVE-*, REQ-PSS-PERSIST-* | ~1200 |
| P11a | Lifecycle Verification | — | — | 0 |
| P12 | FFI Bridge & C-Side Wiring | All (FFI layer) | REQ-PSS-FFI-*, REQ-PSS-COMPAT-*, REQ-PSS-PERSIST-* | ~600 (Rust) + ~400 (C) |
| P12a | FFI Bridge Verification | — | — | 0 |
| P13 | End-to-End Integration & Parity Verification | All | All | ~300 |

Total estimated new/modified LoC: ~7,850 (Rust) + ~400 (C)

## Execution Order

```
P00.5 → P01 → P01a → P02 → P02a
      → P03 → P03a → P04 → P04a
      → P05 → P05a → P06 → P06a
      → P07 → P07a → P08 → P08a
      → P09 → P09a → P09.5 → P09.5a
      → P10 → P10a → P11 → P11a
      → P12 → P12a → P13
```

Each phase MUST be completed and verified before the next begins. No skipping.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Generation-handler semantics are audited by handler class before interface commitments harden
6. Boundary-adjacent types are explicitly split into internal Rust models and `#[repr(C)]` mirrors before FFI wiring
7. Persistence-addressing policy for this port is fixed to parity-preserve the existing addressing semantics exactly; no addressing redesign is in scope
8. Orbit/menu/lifecycle phases may not harden assumptions about callback/global access until the P09.5 feasibility spike has produced concrete wrapper and accessor decisions

## Definition of Done

1. All `cargo test --workspace --all-features` pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. Game boots with `USE_RUST_PLANETS=1` and solar-system exploration works correctly
5. Planetary analysis produces identical outputs to C baseline for all seeded reference systems
6. Surface generation is deterministic: same seed → same topography
7. Scan node materialization matches C baseline: correct counts, positions, types, retrieval filtering
8. Save/reload round-trip preserves orbital position and node suppression state
9. Legacy save files load with correct orbital target and retrieval state
10. Generation-function dispatch preserves override/fallback, data-provider, and side-effect semantics
11. Host lifecycle obligations for persistence are verified: no get/put outside the host-guaranteed window, including teardown and save-exit boundaries
12. Global navigation-state compatibility is verified at system entry, outer→inner, inner→orbit, leave orbit, leave inner system, and leave solar system transitions
13. Temperature/orbit-color greenhouse quirk is preserved for initial parity
14. No placeholder stubs or TODO markers remain in implementation code

## Deferred Items

The following are explicitly out of scope for this plan:

- **Per-race generation-function content scripts**: The 50+ system-specific generators in `sc2/src/uqm/planets/generate/` remain in C. This plan preserves their existing semantics and calls them from Rust through audited FFI wrappers rather than porting the content itself.
- **Lander gameplay mechanics**: Surface traversal, hazard interaction, cargo collection remain C-owned or a separate plan. This plan provides the node-population and pickup-hook integration points.
- **Planetary starmap** (`pstarmap.c`): This is navigational UI, not exploration/orbit/scan flow. Stub module boundary only.
- **Advanced rendering features**: HQxx scalers, OpenGL sphere rendering optimizations are graphics subsystem concerns.
- **Temperature/orbit-color quirk correction**: Preserved for parity; correction deferred to post-parity divergence.
- **Persistence addressing redesign**: This plan intentionally preserves the current star/planet/moon addressing semantics exactly for compatibility; any redesign or migration is a separate decision outside this parity port.

## Plan Files

```
plan/
  00-overview.md                                    (this file)
  00a-preflight-verification.md                     P00.5
  01-analysis.md                                    P01
  01a-analysis-verification.md                      P01a
  02-pseudocode.md                                  P02
  02a-pseudocode-verification.md                    P02a
  03-core-types-constants-stub.md                   P03
  03a-core-types-constants-stub-verification.md     P03a
  04-rng-world-classification.md                    P04
  04a-rng-world-classification-verification.md      P04a
  05-planetary-analysis-tdd.md                      P05
  05a-planetary-analysis-tdd-verification.md        P05a
  06-planetary-analysis-impl.md                     P06
  06a-planetary-analysis-impl-verification.md       P06a
  07-surface-generation-tdd-stub.md                 P07
  07a-surface-generation-tdd-stub-verification.md   P07a
  08-surface-generation-rendering-impl.md           P08
  08a-surface-generation-rendering-impl-verification.md  P08a
  09-scan-flow-node-materialization.md              P09
  09a-scan-flow-node-materialization-verification.md     P09a
  09.5-dispatch-global-access-feasibility.md        P09.5
  09.5a-dispatch-global-access-feasibility-verification.md P09.5a
  10-orbit-entry-orbital-menu.md                    P10
  10a-orbit-entry-orbital-menu-verification.md      P10a
  11-solarsys-lifecycle-navigation.md               P11
  11a-solarsys-lifecycle-navigation-verification.md P11a
  12-ffi-bridge-c-wiring.md                         P12
  12a-ffi-bridge-c-wiring-verification.md           P12a
  13-e2e-integration-parity.md                      P13
  execution-tracker.md
```
