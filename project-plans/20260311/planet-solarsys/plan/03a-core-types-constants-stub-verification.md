# Phase 03a: Core Types & Constants Stub Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P03a`

## Prerequisites
- Required: Phase 03 completed
- Expected files from Phase 03: all files listed in P03 implementation tasks

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/planets/mod.rs` exists and declares all sub-modules
- [ ] `rust/src/planets/constants.rs` contains all system limit constants
- [ ] `rust/src/planets/types.rs` contains domain types and any needed provisional FFI mirror types
- [ ] `rust/src/planets/generate.rs` contains handler-class-specific dispatch abstractions or wrappers
- [ ] `rust/src/planets/solarsys.rs` contains `SolarSysState` struct
- [ ] `rust/src/planets/rng.rs` contains `SysGenRng` stub
- [ ] All remaining stub files exist (world_class, calc, navigation, orbit, scan, surface, gentopo, render, save_location, ffi)
- [ ] `rust/src/lib.rs` includes `pub mod planets`
- [ ] Tests module exists at `rust/src/planets/tests/mod.rs`

## Semantic Verification Checklist
- [ ] Domain `PlanetDesc` fields semantically match C `PLANET_DESC`
- [ ] Domain `StarDesc` fields semantically match C `STAR_DESC`
- [ ] System limits are exact numeric matches to C defines
- [ ] Handler-class dispatch signatures are semantically correct for audited C usage
- [ ] ScanType enum values match C `PlanetScanTypes` order
- [ ] Rust-only types (`Option`, `Vec`, trait objects) are confined to internal structs and absent from any `#[repr(C)]` mirrors
- [ ] No type mismatches remain between declared mirror structs and C originals where mirrors exist

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix issues and re-verify
