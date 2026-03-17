# Phase 06a: Planetary Analysis Implementation Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P06a`

## Prerequisites
- Required: Phase 06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::calc_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `calc.rs` fully implemented, no stubs
- [ ] `constants.rs` contains complete PLAN_DATA and SUN_DATA arrays
- [ ] All test assertions are specific value comparisons (not range checks only)
- [ ] Fixture data traceable to C runtime capture

## Semantic Verification Checklist
- [ ] Earth-like planet analysis matches C output for Sol system
- [ ] Gas giant analysis produces correct density/radius/gravity
- [ ] Temperature-color covers full range: frozen through inferno
- [ ] Greenhouse quirk produces the known orbit-color mismatch
- [ ] RNG sequence within analysis matches C call-by-call

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/calc.rs
# Must return 0
```

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: fix analysis implementation
