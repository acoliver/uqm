# Phase 04a: Colormap + Music Bridge TDD Verification

## Phase ID
`PLAN-20260325-COMMPT3.P04a`

## Prerequisites
- Required: Phase P04 completed
- Expected artifacts: New/updated tests for colormap/music bridge behavior

## Verification Commands

```bash
# All tests compile
cd rust && cargo test --workspace --all-features --no-run

# Run full suite
cd rust && cargo test --workspace --all-features

# Verify expected failures documented
# (Implementer must provide pass/fail matrix in P04 completion marker)
```

## Structural Verification Checklist
- [ ] Tests for Rust call-site rewiring exist and pass
- [ ] Tests for C bridge behavior exist and are documented as expected-fail
- [ ] No test asserts only mock/interaction internals (behavior-driven only)
- [ ] Plan/requirement markers present in test code

## Semantic Verification Checklist (Mandatory)
- [ ] Tests define real behavioral expectations (CommData read, null guard, PlayMusic args)
- [ ] Expected failures are genuinely caused by stub emptiness, not test bugs
- [ ] "for now" marker absence test passes

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: C bridge behavioral tests (CommData read, null guard) FAIL against P03 stubs
- [ ] **Confirmed**: Removing `c_SetColorMapFromCommData` body entirely does not make
  behavioral tests pass (they require specific CommData.AlienColorMap reference)
- [ ] **Confirmed**: A trivially wrong implementation (e.g., hardcoded constant instead of
  CommData read) would still fail the behavioral tests

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P04a.md`
