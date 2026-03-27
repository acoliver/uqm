# Phase 07: Subtitle Display Bridge — TDD

## Phase ID
`PLAN-20260325-COMMPT3.P07`

## Prerequisites
- Required: Phase P06a (Subtitle Display Stub Verification) completed
- Expected: stubs compile, routing rewired, no functional behavior

## Requirements Tested
- REQ-SD-001: C-side subtitle rendering (not back to Rust)
- REQ-SD-002: ClearSubtitles behavior (clear_subtitles=TRUE, last_subtitle=NULL, etc.)
- REQ-SD-003: CheckSubtitles behavior (GetTrackSubtitle comparison + update)
- REQ-SD-004: RedrawSubtitles behavior (optSubtitles guard, add_text rendering)
- REQ-SD-005: Rust SubtitleDisplay test-only

## Purpose
Write structural and behavioral tests that define expected subtitle bridge behavior.
Tests for C implementation details MUST fail against the current stubs.

## Test Tasks

### Structural tests (grep-based)
1. **verify_routing_not_circular**: `c_ClearSubtitles`/`c_CheckSubtitles`/`c_RedrawSubtitles`
   bodies in `rust_comm.c` call `comm_*` not `rust_*` — should pass (P06 wiring)
2. **verify_comm_clear_sets_vars**: `comm_ClearSubtitles` body contains `clear_subtitles = TRUE`
   and `last_subtitle = NULL` — will fail against stubs
3. **verify_comm_check_calls_gettrack**: `comm_CheckSubtitles` body calls `GetTrackSubtitle()`
   — will fail against stubs
4. **verify_comm_redraw_calls_addtext**: `comm_RedrawSubtitles` body calls `add_text(1, &t)`
   — will fail against stubs
5. **verify_comm_redraw_checks_opt**: `comm_RedrawSubtitles` checks `optSubtitles`
   — will fail against stubs
6. **verify_comm_check_reads_commdata**: `comm_CheckSubtitles` reads
   `CommData.AlienTextBaseline` and `CommData.AlienTextAlign` — will fail against stubs

### Existing Rust tests (unchanged)
7. Existing `#[cfg(test)]` subtitle tests remain and pass — they test the Rust
   model independently per REQ-SD-005 (no changes needed)

### Expected failures against stubs (MUST be documented)
Tests 2-6 MUST fail against the current P06 stubs — this proves they test real behavior.
- `comm_ClearSubtitles` has empty body → tests 2 fails
- `comm_CheckSubtitles` has empty body → tests 3, 6 fail
- `comm_RedrawSubtitles` has empty body → tests 4, 5 fail

## Pseudocode Traceability
- Tests trace to pseudocode `002-subtitle-display-fix.md`:
  - Lines 01-06: `comm_ClearSubtitles` behavior (test 2)
  - Lines 07-29: `comm_CheckSubtitles` behavior (tests 3, 6)
  - Lines 30-37: `comm_RedrawSubtitles` behavior (tests 4, 5)
  - Lines 39-48: routing structure (test 1)

## Traceability Markers (in test code)
```bash
# Tests are grep-based structural verification — markers in commands
# @plan PLAN-20260325-COMMPT3.P07
# @requirement REQ-SD-001..005
# @pseudocode 002-subtitle-display-fix lines 01-54
```

## Verification Commands

```bash
# Existing tests still pass
cd rust && cargo test --workspace --all-features

# Structural test 1 (PASS expected)
echo "=== Test 1: routing not circular ==="
grep -A3 "void c_ClearSubtitles" sc2/src/uqm/rust_comm.c | grep "comm_ClearSubtitles" && echo "PASS" || echo "FAIL"

# Behavioral tests (FAIL expected against stubs)
echo "=== Test 2: clear sets vars (expected FAIL) ==="
grep -A10 "comm_ClearSubtitles" sc2/src/uqm/comm.c | grep "clear_subtitles.*TRUE" && echo "PASS" || echo "EXPECTED FAIL: stub"

echo "=== Test 3: check calls GetTrackSubtitle (expected FAIL) ==="
grep -A20 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "GetTrackSubtitle" && echo "PASS" || echo "EXPECTED FAIL: stub"

echo "=== Test 4: redraw calls add_text (expected FAIL) ==="
grep -A10 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | grep "add_text" && echo "PASS" || echo "EXPECTED FAIL: stub"

echo "=== Test 5: redraw checks optSubtitles (expected FAIL) ==="
grep -A10 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | grep "optSubtitles" && echo "PASS" || echo "EXPECTED FAIL: stub"

echo "=== Test 6: check reads CommData (expected FAIL) ==="
grep -A20 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "CommData.AlienText" && echo "PASS" || echo "EXPECTED FAIL: stub"
```

## Structural Verification Checklist
- [ ] All structural/behavioral tests are defined with clear pass/fail criteria
- [ ] Test 1 (routing) passes against P06 stubs
- [ ] Tests 2-6 (behavioral) fail against P06 stubs as expected
- [ ] Expected failures documented with rationale

## Semantic Verification Checklist (Mandatory)
- [ ] Tests assert real behavior (variable assignments, function calls), not stubs
- [ ] Tests 2-6 genuinely fail against empty stub bodies
- [ ] Existing Rust subtitle tests unaffected

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Tests 2-6 fail against the current P06 empty stubs
- [ ] **Confirmed**: If `comm_ClearSubtitles` only set `clear_subtitles = TRUE` but not
  `last_subtitle = NULL`, test 2 (which checks both) would partially fail
- [ ] **Confirmed**: If `comm_RedrawSubtitles` called `add_text` without checking
  `optSubtitles` first, test 5 would fail

## Success Criteria
- [ ] All tests defined with clear expected outcomes
- [ ] Routing test passes (P06 wiring confirmed)
- [ ] Behavioral tests fail as expected (P08 implementation needed)

## Failure Recovery
- If routing test fails, review P06 stub wiring

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P07.md`

Contents:
- Tests defined with expected failure documentation
- Pass/fail matrix for all tests against stubs
