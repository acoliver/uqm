# Execution Tracker

Plan ID: `PLAN-20260314-SHIPS`
Feature: Ships Subsystem Port (28 Races + Shared Infrastructure)

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | |
| P01 | Analysis | ⬜ | ⬜ | ⬜ | |
| P01a | Analysis Verification | ⬜ | ⬜ | ⬜ | |
| P02 | Pseudocode | ⬜ | ⬜ | ⬜ | |
| P02a | Pseudocode Verification | ⬜ | ⬜ | ⬜ | |
| P03 | Core Types & Enums | ⬜ | ⬜ | ⬜ | ~500 LoC |
| P03a | Core Types Verification | ⬜ | ⬜ | ⬜ | |
| P03.5 | FFI Boundary & Ownership Contract | ⬜ | ⬜ | ⬜ | Early ABI/lifetime/layout gate |
| P03.5a | Boundary Contract Verification | ⬜ | ⬜ | ⬜ | |
| P04 | ShipBehavior Trait & Registry | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P04a | Trait & Registry Verification | ⬜ | ⬜ | ⬜ | |
| P05 | Two-Tier Ship Loader | ⬜ | ⬜ | ⬜ | ~500 LoC |
| P05a | Loader Verification | ⬜ | ⬜ | ⬜ | Early C/Rust resource smoke test required |
| P06 | Master Ship Catalog | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P06a | Catalog Verification | ⬜ | ⬜ | ⬜ | |
| P07 | Queue & Build Primitives | ⬜ | ⬜ | ⬜ | ~600 LoC |
| P07a | Queue Verification | ⬜ | ⬜ | ⬜ | Canonical C-owned queue storage |
| P08 | Shared Runtime Pipeline | ⬜ | ⬜ | ⬜ | ~800 LoC |
| P08a | Runtime Pipeline Verification | ⬜ | ⬜ | ⬜ | Early callback/element smoke test required |
| P09 | Ship Spawn & Lifecycle | ⬜ | ⬜ | ⬜ | ~600 LoC |
| P09a | Spawn Verification | ⬜ | ⬜ | ⬜ | Early spawn integration smoke test required |
| P10 | Crew Writeback & Ship Death | ⬜ | ⬜ | ⬜ | ~500 LoC |
| P10a | Writeback Verification | ⬜ | ⬜ | ⬜ | Real queue/fragment integration required |
| P11 | Race Batch 1 — Simple (8) | ⬜ | ⬜ | ⬜ | ~2800 LoC (8×350) |
| P11a | Batch 1 Verification | ⬜ | ⬜ | ⬜ | |
| P12 | Race Batch 2 — Mode-Switching (8) | ⬜ | ⬜ | ⬜ | ~3600 LoC (8×450) |
| P12a | Batch 2 Verification | ⬜ | ⬜ | ⬜ | |
| P13 | Race Batch 3 — Complex/Non-Melee (12) | ⬜ | ⬜ | ⬜ | ~4400 LoC (12×370) |
| P13a | Batch 3 Verification | ⬜ | ⬜ | ⬜ | SIS bridge prerequisites already required |
| P14 | C-Side Bridge Wiring | ⬜ | ⬜ | ⬜ | ~350 Rust + ~400 C |
| P14a | Bridge Wiring Verification | ⬜ | ⬜ | ⬜ | Link/load symbol validation required |
| P15 | E2E Integration Verification | ⬜ | ⬜ | ⬜ | |

Total estimated: ~15,800 Rust LoC + ~400 C LoC

## Execution Rules

1. Phases execute in strict order: P00.5 → P01 → P01a → P02 → P02a → P03 → P03a → P03.5 → P03.5a → ... → P15
2. Each phase MUST be completed and verified before the next begins
3. No skipping phases
4. Phase completion requires creating `project-plans/20260311/ships/.completed/PNN.md`

## Verification Baseline

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```
