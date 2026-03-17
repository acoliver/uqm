# Phase 10a: Orbit Entry & Orbital Menu Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P10a`

## Prerequisites
- Required: Phase 10 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::orbit_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `orbit.rs` — no `todo!()` or `unimplemented!()`
- [ ] `orbit_tests.rs` exists with passing tests
- [ ] External menu dispatch (cargo, devices, roster) uses FFI, not reimplementation

## Semantic Verification Checklist
- [ ] Persistence get called before orbit-content hook
- [ ] Override/fallback dispatch follows the audited wrapper contract established by P09.5
- [ ] Activity interrupt (encounter flag) prevents menu entry
- [ ] No-topo case returns `OrbitalOutcome::NoTopo` without crash
- [ ] Planet load triggers surface generation and node materialization
- [ ] Menu loop correctly re-enters after scan/device/cargo/roster/game sub-flows
- [ ] Leave-orbit action exits menu loop and triggers system reload
- [ ] Orbit-entry persistence get occurs inside the legal host window
- [ ] Node-pickup callback route remains preserved for later lander-originated pickup handling

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/orbit.rs
# Must return 0
```

## Gate Decision
- [ ] PASS: proceed to Phase 11
- [ ] FAIL: fix orbit/menu implementation
