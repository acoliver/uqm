# Phase 03a: Core Types & Enums Verification

## Phase ID
`PLAN-20260314-SHIPS.P03a`

## Prerequisites
- Required: Phase 03 (Core Types & Enums) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/ships/mod.rs` exists and declares all submodules
- [ ] `rust/src/ships/types.rs` exists with all types from Phase 03
- [ ] `rust/src/lib.rs` includes `pub mod ships;`
- [ ] No compilation errors

## Semantic Verification Checklist
- [ ] `SpeciesId` enum has exactly 28 variants matching C `SPECIES_ID` values
- [ ] `SpeciesId::is_melee_eligible()` correctly identifies melee vs non-melee
- [ ] `ShipFlags` bit values match C `races.h:43-58`
- [ ] `StatusFlags` bit values match C `races.h:60-72`
- [ ] `ShipInfo` fields match C `SHIP_INFO` semantics
- [ ] `Characteristics` fields match C `CHARACTERISTIC_STUFF` semantics
- [ ] `RaceDesc` aggregates all sub-structures
- [ ] `Starship` has all combat-runtime fields
- [ ] `ShipFragment` has persistence fields
- [ ] `FleetInfo` has campaign fleet fields

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/types.rs
```

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: return to Phase 03 and fix issues
