# Phase 07a: Subtitle Display Bridge TDD Verification

## Phase ID
`PLAN-20260325-COMMPT3.P07a`

## Prerequisites
- Required: Phase P07 completed
- Expected artifacts: Structural/behavioral tests defined for subtitle bridges

## Verification Commands

```bash
# Existing tests still pass
cd rust && cargo test --workspace --all-features

# Confirm expected failures documented in P07 completion marker
test -f "project-plans/20260311/commpt3/.completed/P07.md" && echo "PASS: marker exists" || echo "FAIL"
```

## Structural Verification Checklist
- [ ] All 7 test criteria defined
- [ ] Routing test (test 1) passes against stubs
- [ ] Behavioral tests (tests 2-6) documented as expected-fail
- [ ] Existing Rust subtitle tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Tests define real behavioral expectations (variable assignments, GetTrackSubtitle call, add_text call)
- [ ] Expected failures genuinely caused by empty stub bodies
- [ ] No test vacuously passes with non-functional implementation

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Tests 2-6 fail against P06 stubs (empty bodies produce no grep matches)
- [ ] **Confirmed**: A trivially wrong implementation (e.g., `comm_ClearSubtitles` that only
  sets `clear_subtitles = FALSE`) would fail test 2 (which expects `TRUE`)
- [ ] **Confirmed**: `comm_RedrawSubtitles` with `add_text(0, &t)` (wrong first arg)
  would fail test 4 (which greps for `add_text(1, &t)`)

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P07a.md`
