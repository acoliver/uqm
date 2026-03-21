# Execution Tracker

Plan ID: `PLAN-20260320-BATTLE`
Feature: Battle Engine Subsystem — Phase 1 Rust Port

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | Includes VelocityState byte-order fix verification |
| P01 | Analysis | ⬜ | ⬜ | ⬜ | |
| P01a | Analysis Verification | ⬜ | ⬜ | ⬜ | |
| P02 | Pseudocode | ⬜ | ⬜ | ⬜ | 5 algorithms |
| P02a | Pseudocode Verification | ⬜ | ⬜ | ⬜ | |
| P03 | Shared Foundation — `battle_types` | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P03a | Shared Foundation Verification | ⬜ | ⬜ | ⬜ | Must verify 47 ships tests pass |
| P04 | Core Types & Constants | ⬜ | ⬜ | ⬜ | ~600 LoC |
| P04a | Core Types Verification | ⬜ | ⬜ | ⬜ | Compile-time size/offset assertions |
| P05 | Element Methods & Lifecycle | ⬜ | ⬜ | ⬜ | ~300 LoC |
| P05a | Element Methods Verification | ⬜ | ⬜ | ⬜ | |
| P06 | Display List, Pool & Registry | ⬜ | ⬜ | ⬜ | ~600 LoC |
| P06a | Display List Verification | ⬜ | ⬜ | ⬜ | |
| P07 | Velocity System | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P07a | Velocity Verification | ⬜ | ⬜ | ⬜ | Bit-identical to C |
| P08 | Collision System | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P08a | Collision Verification | ⬜ | ⬜ | ⬜ | |
| P09 | Weapon System | ⬜ | ⬜ | ⬜ | ~500 LoC; may split |
| P09a | Weapon Verification | ⬜ | ⬜ | ⬜ | |
| P10 | Process Loop Types | ⬜ | ⬜ | ⬜ | ~350 LoC |
| P10a | Process Loop Types Verification | ⬜ | ⬜ | ⬜ | |
| P11 | Battle Lifecycle Types | ⬜ | ⬜ | ⬜ | ~350 LoC |
| P11a | Battle Lifecycle Verification | ⬜ | ⬜ | ⬜ | BattleState layout assertions |
| P12 | Ship Runtime Within Battle Types | ⬜ | ⬜ | ⬜ | ~300 LoC |
| P12a | Ship Runtime Verification | ⬜ | ⬜ | ⬜ | |
| P13 | Tactical Transition Types & Constants | ⬜ | ⬜ | ⬜ | ~450 LoC |
| P13a | Tactical Transition Verification | ⬜ | ⬜ | ⬜ | |
| P14 | AI Dispatch Types & Constants | ⬜ | ⬜ | ⬜ | ~200 LoC |
| P14a | AI Dispatch Verification | ⬜ | ⬜ | ⬜ | |
| P15 | Netplay Integration Types & CRC | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P15a | Netplay Verification | ⬜ | ⬜ | ⬜ | CRC bit-identical to C |
| P16 | Integration Point Contracts | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P16a | Integration Contracts Verification | ⬜ | ⬜ | ⬜ | |
| P17 | FFI Layer & C-Side Bridge | ⬜ | ⬜ | ⬜ | ~400 Rust + ~200 C |
| P17a | FFI Verification | ⬜ | ⬜ | ⬜ | Symbol resolution, ABI |
| P18 | End-to-End Integration | ⬜ | ⬜ | ⬜ | ~100 LoC |
| P18a | End-to-End Verification | ⬜ | ⬜ | ⬜ | Final gate |

Total estimated: ~5,150 Rust LoC + ~200 C LoC = ~5,350 total

## Execution Rules

1. Phases execute in strict order: P00.5 → P01 → P01a → P02 → P02a → P03 → P03a → ... → P18 → P18a
2. Each phase MUST be completed and verified before the next begins
3. No skipping phases
4. Phase completion requires creating `project-plans/20260311/battle/.completed/PNN.md`

## Cross-Plan Dependencies

| Battle Phase | Blocks | Dependency |
|-------------|--------|------------|
| P04 (Element type) | Ships P14-WIRE | `CElement` alias needs `battle::Element` |
| P09 (Weapon types) | Ships P14-WIRE | `LaserBlock`/`MissileBlock` needed by weapon bridge |
| P17 (FFI bridge) | Ships P14-WIRE | `rust_ships_init_weapon` adapter |

**Rule:** Ships P14-WIRE must not begin until battle P09a verification is complete.

## Verification Baseline

All phases must pass these cargo gates:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```
