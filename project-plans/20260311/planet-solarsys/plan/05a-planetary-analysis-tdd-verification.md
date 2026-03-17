# Phase 05a: Planetary Analysis TDD Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P05a`

## Prerequisites
- Required: Phase 05 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
# Tests should compile but are expected to fail (TDD phase)
cargo test -p uqm --lib planets::tests::calc_tests --all-features 2>&1 | tail -20
```

## Structural Verification Checklist
- [ ] `calc_tests.rs` exists and compiles
- [ ] At least 10 fixture test cases present
- [ ] Property tests using proptest present
- [ ] Temperature-color tests present
- [ ] Greenhouse quirk test present

## Semantic Verification Checklist
- [ ] Fixture data includes planets from at least 3 different star types
- [ ] Tests assert on ALL analysis output fields, not just temperature
- [ ] Tests reference REQ-PSS-ANALYSIS-* IDs
- [ ] Tests call the actual `calc.rs` public API functions

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: add missing test cases
