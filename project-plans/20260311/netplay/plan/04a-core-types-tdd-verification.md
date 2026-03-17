# Phase 04a: Core Types & State Machine — TDD Verification

## Phase ID
`PLAN-20260314-NETPLAY.P04a`

## Prerequisites
- Required: Phase 04 (Core Types TDD) completed
- Expected artifacts: test modules in state.rs, error.rs, options.rs, constants.rs

## Verification Tasks

### Test Quality
- [ ] At least 16 test functions exist across the 4 test modules
- [ ] Every `NetState` variant appears in at least one test
- [ ] State predicate tests enumerate ALL states, not just positive cases
- [ ] Transition tests cover the full valid transition graph
- [ ] Invalid transition tests prove the guard actually rejects bad transitions
- [ ] No test asserts only on internal implementation details

### TDD Integrity
- [ ] Tests that depend on unimplemented `todo!()` stubs are expected to panic
- [ ] Tests for already-implemented pure functions (predicates, defaults) pass
- [ ] Test names clearly describe the behavior being tested

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
# Run tests, allowing panics from todo!() stubs
cargo test --workspace --all-features 2>&1 | head -100
```

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: return to Phase 04 and add missing tests

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P04a.md`
