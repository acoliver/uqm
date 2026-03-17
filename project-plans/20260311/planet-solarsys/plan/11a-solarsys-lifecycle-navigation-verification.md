# Phase 11a: Solar-System Lifecycle & Navigation Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets:: --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `solarsys.rs` — no `todo!()` or `unimplemented!()`
- [ ] `navigation.rs` — no `todo!()` or `unimplemented!()`
- [ ] `save_location.rs` — no `todo!()` or `unimplemented!()`
- [ ] `generate.rs` — no undocumented provisional stubs in the completed default-generation paths
- [ ] All test files passing: `navigation_tests.rs`, `save_location_tests.rs`, `persistence_window_tests.rs`

## Semantic Verification Checklist
- [ ] Lifecycle: state created on enter, destroyed on exit, no leaks
- [ ] No overlapping sessions (only one `SolarSysState` active)
- [ ] Load: RNG seeded, planets generated, analyzed, sorted
- [ ] Pending persistence committed before continuing
- [ ] Outer-inner transitions correctly switch descriptor sets
- [ ] `WaitIntersect` prevents collision with recently-departed body
- [ ] Save-location encodes planets and moons with baseline-compatible values
- [ ] Legacy save fixtures decode correctly
- [ ] Persistence get/put occurs only within the host-guaranteed legal window
- [ ] No get/put occurs after solar-system uninit completes
- [ ] Active context clearing prevents stale reads after exit
- [ ] Global navigation compatibility verified at system entry, outer→inner, inner→orbit, leave orbit, leave inner system, and leave solar system
- [ ] NPC init/reinit/uninit lifecycle points are covered explicitly, not deferred to P13 only
- [ ] Dedicated per-star dispatch remains clearly marked as incomplete until P12 wiring is finished

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/solarsys.rs rust/src/planets/navigation.rs rust/src/planets/save_location.rs rust/src/planets/generate.rs
# Must return 0 for completed portions
```

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: fix lifecycle/navigation implementation
