# Phase 07a: Deadlock Fix — TDD Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P07a`

## Prerequisites
- Required: Phase P07 completed
- Expected: 4 tests for `rust_copy_game_state` in ffi.rs

## Structural Verification
- [ ] 4 tests present in `rust/src/state/ffi.rs` test module
- [ ] Tests have `@plan` and `@requirement` markers
- [ ] Tests compile: `cargo test --workspace --all-features --no-run`

## Semantic Verification
- [ ] `test_copy_game_state_no_deadlock` has timeout mechanism
- [ ] `test_copy_game_state_basic` checks actual bit values before and after
- [ ] `test_copy_game_state_self_overlapping` tests adjacent bit ranges
- [ ] Tests fail due to `todo!()` panic (RED phase confirmed)

## Gate Decision
- [ ] PASS: proceed to P08
- [ ] FAIL: fix tests
