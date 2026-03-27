# Phase 10a: DoCommunication TDD Verification

## Phase ID
`PLAN-20260325-COMMPT3.P10a`

## Prerequisites
- Required: Phase P10 completed
- Expected artifacts: 10 new/updated tests for DoCommunication behavior

## Verification Commands

```bash
cd rust && cargo test --workspace --all-features --no-run
cd rust && cargo test --workspace --all-features

# Confirm expected failures documented
test -f "project-plans/20260311/commpt3/.completed/P10.md" && echo "PASS" || echo "FAIL"
```

## Structural Verification Checklist
- [ ] All 10 tests compile
- [ ] Tests for state machine transitions exist (Talking, Done, Selected, ResponseContinue)
- [ ] Tests for lock discipline exist
- [ ] Expected failures documented in completion marker

## Semantic Verification Checklist (Mandatory)
- [ ] Tests define real behavioral expectations (state transitions, callback dispatch, lock lifecycle)
- [ ] Expected failures genuinely caused by stub behavior
- [ ] No test asserts only mock interactions

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: State machine tests fail against P09 stubs (always-Talking or always-None)
- [ ] **Confirmed**: A partially-correct implementation (e.g., correct Done exit but no Selected handling)
  would pass some tests and fail others — tests are independent
- [ ] **Confirmed**: Lock discipline test would fail if `drop(state)` were placed AFTER callback

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P10a.md`
