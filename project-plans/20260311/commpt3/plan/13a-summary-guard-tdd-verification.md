# Phase 13a: Summary Guard TDD Verification

## Phase ID
`PLAN-20260325-COMMPT3.P13a`

## Prerequisites
- Required: Phase P13 completed
- Expected artifacts: Tests defined for summary delegation and stale markers

## Verification Commands

```bash
cd rust && cargo test --workspace --all-features

test -f "project-plans/20260311/commpt3/.completed/P13.md" && echo "PASS" || echo "FAIL"
```

## Structural Verification Checklist
- [ ] All 6 test criteria defined with clear pass/fail expectations
- [ ] Summary delegation tests pass
- [ ] Stale marker test (test 3) documented as expected-fail

## Semantic Verification Checklist (Mandatory)
- [ ] Stale marker sweep uses per-match classification (no pipe filtering)
- [ ] Expected failure is genuinely caused by remaining production markers
- [ ] Exemptions are individually validated

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Test 3 fails against current code (markers still in ffi.rs)
- [ ] **Confirmed**: If only exempted markers remained, test 3 would pass
- [ ] **Confirmed**: Adding a new unexempted marker to any file would cause the
  corresponding sweep test to fail

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P13a.md`
