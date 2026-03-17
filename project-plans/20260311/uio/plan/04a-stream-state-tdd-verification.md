# Phase 04a: Stream State Fix — TDD Verification

## Phase ID
`PLAN-20260314-UIO.P04a`

## Prerequisites
- Required: Phase 04 completed
- All test functions added and compiling

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features -- stream_state 2>&1 | head -60
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] 16 new test functions exist in the test module
- [ ] `create_test_stream` helper exists
- [ ] All tests have `@plan` and `@requirement` markers in comments
- [ ] Tests cover: feof (3 states + null), ferror (3 states + null), clearerr (2 + null), fseek (1), fclose (1), fflush null (1), fwrite error (1)

## Semantic Verification Checklist
- [ ] All 16 tests pass
- [ ] Tests actually exercise the functions under test (not no-op assertions)
- [ ] Tests will fail if behavior is reverted (e.g., hardcoded return values would break them)
- [ ] No test depends on another test's side effects

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: fix failing tests before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P04a.md`
