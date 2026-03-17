# Execution Tracker

Plan ID: PLAN-20260314-CAMPAIGN

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00.5 | Preflight Verification | -- | -- | N/A | |
| P01 | Analysis | -- | -- | -- | |
| P01a | Analysis Verification | -- | -- | -- | |
| P02 | Pseudocode | -- | -- | -- | |
| P02a | Pseudocode Verification | -- | -- | -- | |
| P03 | Types & Domain Model | -- | -- | -- | |
| P03a | Types Verification | -- | -- | -- | |
| P03.5 | C-State Accessor Bridge & Ownership Model | -- | -- | -- | |
| P03.5a | C-State Accessor Bridge Verification | -- | -- | -- | |
| P04 | Event Catalog & Handlers | -- | -- | -- | |
| P04a | Event Catalog Verification | -- | -- | -- | |
| P05 | Save Summary & Export Types | -- | -- | -- | |
| P05a | Save Summary Verification | -- | -- | -- | |
| P06 | Save Serialization | -- | -- | -- | |
| P06a | Save Serialization Verification | -- | -- | -- | |
| P07 | Load Deserialization & Validation | -- | -- | -- | |
| P07a | Load Validation Verification | -- | -- | -- | |
| P08 | Legacy Save Compatibility | -- | -- | -- | |
| P08a | Legacy Save Verification | -- | -- | -- | |
| P09 | Deferred Transitions & Dispatch | -- | -- | -- | |
| P09a | Dispatch Verification | -- | -- | -- | |
| P10 | Hyperspace Navigation Transitions | -- | -- | -- | |
| P10a | Navigation Transitions Verification | -- | -- | -- | |
| P11 | Encounter Handoff & Post-Encounter | -- | -- | -- | |
| P11a | Encounter Verification | -- | -- | -- | |
| P12 | Starbase Visit Flow | -- | -- | -- | |
| P12a | Starbase Verification | -- | -- | -- | |
| P13 | Hyperspace Menu & Clock Rate | -- | -- | -- | |
| P13a | Hyper Menu Verification | -- | -- | -- | |
| P14 | Canonical Export Document | -- | -- | -- | |
| P14a | Canonical Export Verification | -- | -- | -- | |
| P15 | C-Side Bridge & Build Toggle | -- | -- | -- | |
| P15a | Build Toggle Verification | -- | -- | -- | |
| P16 | End-to-End Integration | -- | -- | -- | |

Update after each phase.

## Dependency Notes

- P03-P05: Rust domain and persistence-surface types
- P03.5: establishes validated ownership/access seams before orchestration phases
- P06-P08: Save/load depends on `rust/src/io/`, `rust/src/state/`, bridge snapshot/adjunct classification
- P09: Campaign loop depends on clock subsystem (`rust/src/time/`) and validated state seams
- P10: Transitions depend on clock rate API and queue/access bridge
- P11: Encounter depends on comm and battle FFI
- P12: Starbase depends on comm FFI, clock day-advance
- P13: Hyper menu depends on multiple subsystem FFI calls
- P14: Export depends on serde_json crate and verifier-facing reporting helpers
- P15: Build toggle depends on validated seam inventory + all Rust modules + C build system
- P16: Integration depends on all subsystems working together and on the claim-family verification matrix

## Estimated Effort by Phase

| Phase | Est. LoC (Rust) | Est. LoC (C) |
|-------|----------------|--------------|
| P03 | ~600 | 0 |
| P03.5 | ~350 | ~150 |
| P04 | ~800 | 0 |
| P05 | ~500 | 0 |
| P06 | ~500 | 0 |
| P07 | ~700 | 0 |
| P08 | ~400 | 0 |
| P09 | ~700 | 0 |
| P10 | ~500 | 0 |
| P11 | ~700 | 0 |
| P12 | ~800 | 0 |
| P13 | ~500 | 0 |
| P14 | ~600 | 0 |
| P15 | ~500 | ~400 |
| P16 | ~200 | 0 |
| **Total** | **~8150** | **~550** |
