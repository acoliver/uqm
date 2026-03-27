# Phase 10: DoCommunication Response Dispatch — TDD

## Phase ID
`PLAN-20260325-COMMPT3.P10`

## Prerequisites
- Required: Phase P09a (DoCommunication Stub Verification) completed
- Expected: new enum compiles, stubs in place, tests compile

## Requirements Tested
- REQ-RL-001: Lock release before callback
- REQ-RL-002: Select → extract → drop → invoke sequence
- REQ-RL-003: No lock during C callback
- REQ-RL-004: Pre-callback work under lock
- REQ-DC-001: Single frame iteration
- REQ-DC-002: No response input during talking
- REQ-DC-003: Single response input per frame
- REQ-DC-004: Done when no responses after talking
- REQ-DC-005: Immediate exit on abort/load

## Purpose
Write behavior-driven tests for the DoCommunication state machine and lock
discipline. Tests MUST fail against the current stubs.

## Test Tasks

### State machine tests (Rust `#[cfg(test)]`)
1. **test_do_comm_talking_phase**: `talking_finished == false` →
   `do_communication` returns `Talking`, `alien_talk_segue` called,
   `player_response_input` NOT called (REQ-DC-002)
2. **test_do_comm_abort_exit**: abort flag set → returns `Done` (REQ-DC-005)
3. **test_do_comm_load_exit**: load flag set → returns `Done` (REQ-DC-005)
4. **test_do_comm_no_responses_done**: `talking_finished == true`,
   `responses.count() == 0` → returns `Done` (REQ-DC-004)
5. **test_do_comm_response_continue**: `talking_finished == true`,
   responses registered, no selection confirmed → returns `ResponseContinue`,
   `player_response_input` called exactly once (REQ-DC-003)
6. **test_do_comm_selected**: selection confirmed → returns
   `Selected(fn, ref)` with correct callback (REQ-DC-001)
7. **test_select_response_returns_tuple**: valid callback →
   `select_response` returns `Some((fn, ref))` (REQ-RL-004)
8. **test_select_response_null_callback**: null callback →
   `select_response` returns `None`

### Lock discipline tests (Rust `#[cfg(test)]`)
9. **test_lock_dropped_before_callback**: `rust_DoCommunication` drops
   `COMM_STATE` write guard before invoking callback (REQ-RL-001, RL-003)
10. **test_no_double_player_response_input**: `player_response_input` called
    at most once per `do_communication` invocation (REQ-DC-003)

### Expected failures against stubs (MUST be documented)
All tests 1-10 MUST fail against the P09 stubs:
- `do_communication` always returns `Talking` → tests 2-6 fail
- `select_response` always returns `None` → test 7 fails
- Lock discipline not implemented → test 9 fails (or cannot verify)
- Double call still present → test 10 fails

## Pseudocode Traceability
- Tests trace to pseudocode `003-do-communication-rewrite.md`:
  - Lines 07-18: talking phase (test 1)
  - Lines 19-22: abort/load check (tests 2, 3)
  - Lines 23-26: no-responses exit (test 4)
  - Lines 27-35: response input + selection (tests 5, 6, 10)
  - Lines 36-40: select_response return (tests 7, 8)
  - Lines 41-64: rust_DoCommunication dispatch + lock discipline (test 9)
  - Lines 65-81: lock discipline invariant (tests 9, 10)

## Traceability Markers (in test code)
```rust
/// @plan PLAN-20260325-COMMPT3.P10
/// @requirement REQ-RL-001..004, REQ-DC-001..005
/// @pseudocode 003-do-communication-rewrite lines 01-81
```

## Verification Commands

```bash
# Tests compile
cd rust && cargo test --workspace --all-features --no-run

# Run tests — document failures against stubs
cd rust && cargo test --workspace --all-features 2>&1 | tee /tmp/tdd-p10-results.txt

# Verify expected failures
grep -c "FAILED\|test result: FAILED" /tmp/tdd-p10-results.txt
```

## Structural Verification Checklist
- [ ] All 10 tests compile
- [ ] Tests use behavioral assertions (return values, call counts), not implementation internals
- [ ] Expected failures documented with rationale

## Semantic Verification Checklist (Mandatory)
- [ ] Tests assert state-machine transitions (Talking/Done/Selected), not enum internals
- [ ] Lock discipline test verifies drop-before-callback sequence
- [ ] No tests that pass with non-functional stub implementation

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Tests 2-6 fail because `do_communication` stub returns `Talking` unconditionally
- [ ] **Confirmed**: Test 7 fails because `select_response` stub returns `None` unconditionally
- [ ] **Confirmed**: If `do_communication` returned `Done` unconditionally (different wrong stub),
  tests 1 and 5 would fail
- [ ] **Confirmed**: If lock were never dropped before callback, test 9 would fail

## Success Criteria
- [ ] All tests compile
- [ ] Expected failures against stubs documented
- [ ] Test design covers all 9 DC/RL requirements

## Failure Recovery
- rollback: `git restore` test files

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P10.md`

Contents:
- Tests written with expected failure documentation
- Pass/fail matrix for all tests against stubs
