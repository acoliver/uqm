# Execution Tracker

Plan ID: PLAN-20260314-PLANET-SOLARSYS
Feature: Planet-SolarSys Subsystem Port (C to Rust)

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00.5 | Preflight Verification | -- | -- | N/A | Includes generation-handler signature inventory, type-model split, and host lifecycle audit |
| P01 | Analysis | -- | -- | -- | Distinguishes domain models from FFI mirrors |
| P01a | Analysis Verification | -- | -- | -- | |
| P02 | Pseudocode | -- | -- | -- | Handler-class semantics explicit |
| P02a | Pseudocode Verification | -- | -- | -- | |
| P03 | Core Types & Constants (Stub) | -- | -- | -- | ~800 LoC |
| P03a | Core Types Verification | -- | -- | -- | |
| P04 | RNG & World Classification | -- | -- | -- | ~400 LoC |
| P04a | RNG & Classification Verification | -- | -- | -- | |
| P05 | Planetary Analysis (TDD) | -- | -- | -- | ~300 LoC (tests) |
| P05a | Analysis TDD Verification | -- | -- | -- | |
| P06 | Planetary Analysis (Impl) | -- | -- | -- | ~600 LoC |
| P06a | Analysis Impl Verification | -- | -- | -- | |
| P07 | Surface Generation (TDD+Stub) | -- | -- | -- | ~400 LoC |
| P07a | Surface Gen Stub Verification | -- | -- | -- | |
| P08 | Surface Gen & Rendering (Impl) | -- | -- | -- | ~1200 LoC |
| P08a | Surface Gen Impl Verification | -- | -- | -- | |
| P09 | Scan Flow & Node Materialization | -- | -- | -- | ~1000 LoC |
| P09a | Scan Flow Verification | -- | -- | -- | |
| P10 | Orbit Entry & Orbital Menu | -- | -- | -- | ~800 LoC |
| P10a | Orbit Menu Verification | -- | -- | -- | |
| P11 | Solar-System Lifecycle & Nav | -- | -- | -- | ~1200 LoC |
| P11a | Lifecycle Verification | -- | -- | -- | Includes persistence-window verification |
| P12 | FFI Bridge & C-Side Wiring | -- | -- | -- | ~1000 LoC (Rust+C) |
| P12a | FFI Bridge Verification | -- | -- | -- | |
| P13 | E2E Integration & Parity | -- | -- | -- | ~300 LoC (tests) |

Total estimated: ~7,600 LoC (Rust) + ~400 LoC (C)

## Integration Contract

### Existing Callers
- `sc2/src/uqm/` game loop calls `ExploreSolarSys()` — this becomes the C-to-Rust entry point via FFI
- `sc2/src/uqm/state.c` already routes `GetPlanetInfo`/`PutPlanetInfo` to Rust via `USE_RUST_STATE`
- 50+ system-specific generators in `sc2/src/uqm/planets/generate/` call through `GenerateFunctions` table — remain in C, called from Rust via audited FFI wrappers

### Existing Code Replaced/Removed
- 11 C files fully guarded behind `#ifndef USE_RUST_PLANETS`
- 4 C files partially guarded (integration boundary dispatch only)
- 1 C file out of scope (`pstarmap.c`)

### User Access Path
- Hyperspace navigation -> enter star system -> `ExploreSolarSys()` -> Rust planet-solarsys subsystem

### Data/State Migration
- No data migration: this parity plan preserves current persistence-addressing semantics exactly
- RNG must produce identical sequences to C `RandomContext`
- Boundary-crossing types must use explicit `#[repr(C)]` mirrors; internal domain structs need not be ABI-compatible
- Generation handlers must preserve distinct override/fallback, data-provider, and side-effect semantics

### Host lifecycle boundary
- Campaign/host must initialize persistence before the first legal get/put
- Campaign/host must keep persistence live through orbit-entry get and save/load put operations
- Planet-solarsys must not call get/put after solar-system uninit
- Verification must include save/exit teardown ordering

### End-to-End Verification
- `cargo test -p uqm --lib planets::tests::e2e_parity_tests`
- Manual: boot game with `USE_RUST_PLANETS=1`, enter solar system, complete scan, save/load
- Corpus must include shielded worlds, encounter-triggering worlds, several dedicated systems beyond Sol, and several moon-bearing layouts
