# Phase 09a: Scan Flow & Node Materialization Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P09a`

## Prerequisites
- Required: Phase 09 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::scan_tests --all-features -- --nocapture
cargo test -p uqm --lib planets::tests::persistence_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `scan.rs` — no `todo!()` or `unimplemented!()`
- [ ] `scan_tests.rs` and `persistence_tests.rs` exist with passing tests
- [ ] Integration with `PlanetInfoManager` verified via persistence_tests

## Semantic Verification Checklist
- [ ] `is_node_retrieved` correctly checks bit positions in 32-bit mask
- [ ] Node count matches generation function output minus retrieved nodes
- [ ] Iteration order (bio, energy, mineral) matches C code
- [ ] Mineral node density encodes gross_size (low byte) and fine_size (high byte)
- [ ] Bio variation is capped by world's life_variation limit
- [ ] Encounter trigger correctly calls save_solar_sys_location
- [ ] Persistence round-trip: put then get then materialize produces correct filtering

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/scan.rs
# Must return 0
```

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: fix scan implementation
