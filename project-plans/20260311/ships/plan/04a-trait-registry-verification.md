# Phase 04a: ShipBehavior Trait & Registry Verification

## Phase ID
`PLAN-20260314-SHIPS.P04a`

## Prerequisites
- Required: Phase 04 (Trait & Registry) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ShipBehavior` trait is object-safe (`Box<dyn ShipBehavior>` compiles)
- [ ] `ShipBehavior` is `Send` (required for potential cross-thread use)
- [ ] All 28 species have match arms or table entries in `create_ship_behavior()`
- [ ] `races/mod.rs` exists
- [ ] No `StubShip` or panic-on-use placeholder type exists

## Semantic Verification Checklist
- [ ] Default trait methods return Ok(()) / Ok(vec![]) / StatusFlags::empty() / None as appropriate
- [ ] `create_race_desc()` constructs a valid `RaceDesc` for implemented species
- [ ] Registry returns explicit Err for invalid or not-yet-implemented species
- [ ] Trait methods can be called on `Box<dyn ShipBehavior>` without issue
- [ ] Any template-only fallback path is safe and non-panicking

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: return to Phase 04 and fix issues
