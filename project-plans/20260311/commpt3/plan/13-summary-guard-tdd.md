# Phase 13: Summary Guard + Stale Marker Elimination — TDD

## Phase ID
`PLAN-20260325-COMMPT3.P13`

## Prerequisites
- Required: Phase P12a (Summary Guard Stub Verification) completed
- Expected: cfg-bifurcation in place, stale markers still present

## Requirements Tested
- REQ-CS-002: Production delegates to C summary
- REQ-CS-003: No Rust SummaryView in production
- REQ-SM-001: Zero stale markers in production paths
- REQ-SM-002: Permitted marker exemptions

## Purpose
Write tests that verify summary delegation and define the stale-marker-free
requirement. Tests for marker elimination MUST fail against the current code
(which still has stale markers).

## Test Tasks

### Summary delegation tests
1. **verify_production_delegates_to_c**: grep-based: production path of
   `rust_ShowConversationSummary` calls `c_SelectConversationSummary`, NOT `SummaryView`
   — should pass (P12 wiring in place)
2. **verify_no_summaryview_in_production**: grep-based: no `SummaryView` reference
   outside `#[cfg(test)]` blocks — should pass (P12 bifurcation)

### Stale marker sweep tests
3. **verify_zero_stale_markers_ffi**: grep-based: zero production markers in `ffi.rs`
   — will FAIL (stale "abort not yet wired" + "not yet implemented" still present)
4. **verify_zero_stale_markers_talk_segue**: grep-based: zero production markers in
   `talk_segue.rs` — should pass (P05 already cleaned markers)
5. **verify_zero_stale_markers_c**: grep-based: zero markers in `rust_comm.c`
   — should pass (P05/P08 already cleaned)
6. **verify_exemptions_valid**: grep-based: confirm known exemptions (C ref, design
   semantics, sentinel) are valid per REQ-SM-002

### Expected failures against current state (MUST be documented)
Test 3 MUST fail — "abort not yet wired" and "input handling is not yet implemented"
comments are still in `ffi.rs` production paths.

## Pseudocode Traceability
- Tests trace to pseudocode `004-summary-guard-stale-markers.md`:
  - Lines 01-24: summary delegation (tests 1, 2)
  - Lines 25-32: stale marker removal (tests 3, 4, 5)
  - Lines 33-40: exemption verification (test 6)
  - Lines 41-47: sweep procedure (tests 3-5)

## Traceability Markers (in test code)
```bash
# @plan PLAN-20260325-COMMPT3.P13
# @requirement REQ-CS-002, REQ-CS-003, REQ-SM-001, REQ-SM-002
# @pseudocode 004-summary-guard-stale-markers lines 01-47
```

## Verification Commands

```bash
cd rust && cargo test --workspace --all-features

# Test 1: production delegates to C (PASS expected)
echo "=== Test 1: production delegates ==="
grep -A5 "cfg(not(test))" rust/src/comm/ffi.rs | grep "c_SelectConversationSummary" && echo "PASS" || echo "FAIL"

# Test 2: no SummaryView in production (PASS expected)
echo "=== Test 2: no SummaryView in production ==="
# Check lines NOT inside cfg(test) for SummaryView
grep "SummaryView" rust/src/comm/ffi.rs | grep -v "cfg(test)" | grep -v "///" | grep -v "test" && echo "FAIL" || echo "PASS"

# Test 3: zero stale markers in ffi.rs production (FAIL expected)
echo "=== Test 3: ffi.rs stale markers (expected FAIL) ==="
grep -n 'not yet wired\|not yet implemented' rust/src/comm/ffi.rs | grep -v 'cfg(test)\|///\|stubs in commanim' && echo "EXPECTED FAIL: markers present" || echo "PASS"

# Test 4: zero stale markers in talk_segue.rs (PASS expected)
echo "=== Test 4: talk_segue.rs stale markers ==="
grep -n 'for now\|not yet wired\|not yet implemented' rust/src/comm/talk_segue.rs | grep -v 'cfg(test)\|///' && echo "FAIL" || echo "PASS"

# Test 5: zero stale markers in rust_comm.c (PASS expected)
echo "=== Test 5: rust_comm.c markers ==="
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet' sc2/src/uqm/rust_comm.c && echo "FAIL" || echo "PASS"

# Test 6: exemptions valid
echo "=== Test 6: exemptions ==="
grep -n "stubs in commanim" rust/src/comm/ffi.rs && echo "EXEMPT: C reference"
grep -n "not yet disabled this encounter" rust/src/comm/phrase_state.rs && echo "EXEMPT: design"
grep -n "not yet initialized" rust/src/comm/state.rs && echo "EXEMPT: sentinel"
```

## Structural Verification Checklist
- [ ] All 6 test criteria defined
- [ ] Tests 1, 2 pass (summary delegation confirmed)
- [ ] Test 3 fails as expected (stale markers remain)
- [ ] Tests 4, 5 pass (earlier phases already cleaned)
- [ ] Test 6 validates exemptions

## Semantic Verification Checklist (Mandatory)
- [ ] Stale marker tests use per-match classification, not pipe filtering
- [ ] Expected failure genuinely caused by remaining stale comments
- [ ] Summary delegation tests verify real cfg structure

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Test 3 fails because "abort not yet wired" and "not yet implemented"
  are still in `ffi.rs` production code
- [ ] **Confirmed**: If only one of the two stale comments were removed, test 3 would
  still fail (it catches all markers, not just one)
- [ ] **Confirmed**: Exemptions (C ref, design, sentinel) are correctly excluded by
  the per-match classifier

## Success Criteria
- [ ] Tests compiled and defined
- [ ] Summary delegation tests pass
- [ ] Stale marker test fails as expected (P14 needed)

## Failure Recovery
- If tests 1 or 2 fail, review P12 bifurcation

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P13.md`

Contents:
- Tests defined with expected failure documentation
- Pass/fail matrix
