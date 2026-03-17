# Phase 04a: Core Types & Error — TDD Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P04a`

## Prerequisites
- Required: Phase 04 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
# Tests expected to fail (stubs):
cargo test --workspace --all-features supermelee 2>&1 | grep -E "FAILED|panicked"
```

## Structural Verification Checklist
- [ ] Test files exist: `types_tests.rs`, `team_tests.rs`
- [ ] Tests are included via `#[cfg(test)]` modules
- [ ] At least 15 test functions across both files
- [ ] Plan traceability markers present in test comments

## Semantic Verification Checklist
- [ ] Running tests produces failures (not silent passes)
- [ ] Each failing test corresponds to unimplemented behavior
- [ ] No test passes with `todo!()` stub — confirms tests are non-trivial

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: fix tests that pass with stubs

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P04.md`
