# Phase 05a: Core Types & Error — Implementation Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P05a`

## Prerequisites
- Required: Phase 05 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# All tests must pass
cargo test --workspace --all-features supermelee 2>&1 | grep -c "test result: ok"
```

## Structural Verification Checklist
- [ ] No `todo!()` in `types.rs` or `team.rs`
- [ ] All P04 tests pass
- [ ] No new compilation warnings

## Semantic Verification Checklist
- [ ] MeleeShip enum values match C `meleeship.h` exactly
- [ ] Team serialization format matches C `MeleeTeam_serialize` byte layout
- [ ] Fleet value calculation matches C `MeleeTeam_getValue` behavior

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/supermelee/types.rs rust/src/supermelee/setup/team.rs rust/src/supermelee/error.rs
```

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: fix implementation issues

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P05.md`
