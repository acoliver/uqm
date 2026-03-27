# Phase 11: DoCommunication Response Dispatch — Implementation

## Phase ID
`PLAN-20260325-COMMPT3.P11`

## Prerequisites
- Required: Phase P10a (DoCommunication TDD Verification) completed
- Expected: tests written, expected failures documented against stubs

## Requirements Implemented

### REQ-RL-001: Lock Release Before Callback
The `COMM_STATE` write lock SHALL be released before the callback is invoked.

### REQ-RL-002: Select → Extract → Drop → Invoke Sequence
WHEN a response is selected: (1) call `select_response` under lock, (2) extract
`(callback_fn, response_ref)`, (3) drop write guard, (4) invoke callback, (5) return 1.

### REQ-RL-003: No Lock During C Callback
The response-callback dispatch path SHALL NOT hold the COMM_STATE write lock
while executing a C callback.

### REQ-RL-004: Pre-Callback Work Under Lock
`select_response` SHALL perform all pre-callback work (clear responses, stop track,
clear subtitles, fade music, feedback phrase) while holding the lock, then return
the `(callback_fn, response_ref)` tuple.

### REQ-DC-001: Single Frame Iteration
`rust_DoCommunication` SHALL execute exactly one state machine iteration per call.

### REQ-DC-002: No Response Input During Talking
WHILE talking, SHALL NOT process player response input.

### REQ-DC-003: Single Response Input Per Frame
WHILE responses registered, SHALL call `player_response_input` exactly once.

### REQ-DC-004: Done When No Responses
WHEN `talking_finished` and no responses, SHALL return 0 (Done).

### REQ-DC-005: Immediate Exit on Abort/Load
WHEN abort/load flags set, SHALL return 0 immediately.

## Implementation Tasks

### Files to modify
- `rust/src/comm/talk_segue.rs`
  - Rewrite `do_communication()` to return rich `CommunicationResult`:
    - `!talking_finished` → `alien_talk_segue` → `Talking`
    - abort/load → `Done`
    - no responses → `run_last_replay` → `Done`
    - `player_response_input` called exactly once
    - selection confirmed → `select_response` → `Selected(fn, ref)` or `ResponseContinue`
  - Rewrite `select_response()` to return `Option<(extern "C" fn(u32), u32)>`
  - Remove old `CommunicationResult::Continue` variant
  - marker: `@plan PLAN-20260325-COMMPT3.P11`
  - marker: `@requirement REQ-DC-001..005, REQ-RL-004`
  - marker: `@pseudocode 003-do-communication-rewrite lines 07-40`

- `rust/src/comm/ffi.rs`
  - Rewrite `rust_DoCommunication()`:
    - Acquire `COMM_STATE.write()`
    - Call `do_communication(&mut state)`
    - Match: `Talking` → drop → return 1, `ResponseContinue` → drop → return 1,
      `Selected(fn, ref)` → drop → fn(ref) → return 1, `Done` → drop → return 0
    - Remove second `player_response_input` call
    - Remove old convoluted lock-drop pattern
  - marker: `@plan PLAN-20260325-COMMPT3.P11`
  - marker: `@requirement REQ-RL-001..003, REQ-DC-001`
  - marker: `@pseudocode 003-do-communication-rewrite lines 41-81`

### Files to create
- None

## Pseudocode Traceability
- `CommunicationResult` enum: pseudocode `003-do-communication-rewrite.md` lines 01-06
- `do_communication` rewrite: pseudocode `003-do-communication-rewrite.md` lines 07-40
  - Contract: REQ-DC-001..005
- `rust_DoCommunication` rewrite: pseudocode `003-do-communication-rewrite.md` lines 41-64
  - Contract: REQ-RL-001..003
- Lock discipline invariant: pseudocode `003-do-communication-rewrite.md` lines 65-81
  - Contract: REQ-RL-004

## Traceability Markers (in code)
```rust
/// @plan PLAN-20260325-COMMPT3.P11
/// @requirement REQ-RL-001, REQ-RL-002, REQ-DC-001
/// @pseudocode 003-do-communication-rewrite lines 01-81
```

## Verification Commands

```bash
# Full quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify no double player_response_input in ffi.rs
grep -c "player_response_input" rust/src/comm/ffi.rs
# Expected: 0

# Verify single call in do_communication
grep -n "player_response_input" rust/src/comm/talk_segue.rs | grep -v "fn player_response_input\|test\|cfg(test)\|//\|///\|mod "

# Verify lock drop before callback
grep -n "drop\|callback" rust/src/comm/ffi.rs | head -20

# Verify no nested lock acquisition
awk '/fn rust_DoCommunication/,/^}/' rust/src/comm/ffi.rs | grep -c "COMM_STATE.write"
# Expected: 1
```

## Structural Verification Checklist
- [ ] `CommunicationResult` has exactly 4 variants (no old `Continue`)
- [ ] `do_communication` returns `CommunicationResult` (real state machine, not stub)
- [ ] `select_response` returns `Option<(extern "C" fn(u32), u32)>`
- [ ] `rust_DoCommunication`: one `COMM_STATE.write()`, match on 4 arms, explicit `drop`
- [ ] No `player_response_input` in `ffi.rs`
- [ ] Only one `player_response_input` call in `do_communication` body
- [ ] No `todo!()`, `unimplemented!()`, or placeholder markers

## Semantic Verification Checklist (Mandatory)
- [ ] Talking phase: only `alien_talk_segue` runs, no response input (REQ-DC-002)
- [ ] Response phase: `player_response_input` called exactly once (REQ-DC-003)
- [ ] Selected arm: lock dropped BEFORE callback (REQ-RL-001, RL-003)
- [ ] Selected arm: callback invoked AFTER lock drop (REQ-RL-002)
- [ ] Selected arm: returns 1 (continue DoInput loop)
- [ ] Done arm: returns 0 (end DoInput loop)
- [ ] Abort/load → Done immediately (REQ-DC-005)
- [ ] No responses → run_last_replay → Done (REQ-DC-004)
- [ ] `select_response` pre-callback work under lock (REQ-RL-004)
- [ ] All P10 TDD tests now PASS (previously failing against stubs)
- [ ] All 268+ comm tests pass
- [ ] Both build modes compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Negative proof — lock discipline**: Temporarily move `drop(state)` AFTER
  `callback_fn(response_ref)` → lock discipline test 9 fails. Revert.
- [ ] **Negative proof — double call**: Temporarily add second `player_response_input`
  call → test 10 fails. Revert.
- [ ] **Negative proof — talking guard**: Temporarily allow `player_response_input`
  during talking phase → test 1 (DC-002) fails. Revert.

## Deferred Implementation Detection (Mandatory)

```bash
for f in rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs; do
  echo "=== $f ==="
  grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet\|todo!\|unimplemented!' "$f" | while IFS= read -r line; do
    lineno=$(echo "$line" | cut -d: -f1)
    content=$(sed -n "${lineno}p" "$f")
    if echo "$content" | grep -q '^ *///'; then echo "EXEMPT (doc): $f:$line"
    elif echo "$content" | grep -q 'cfg(test)'; then echo "EXEMPT (test): $f:$line"
    elif echo "$content" | grep -q 'stubs in commanim'; then echo "EXEMPT (C ref): $f:$line"
    else echo "FAIL: production marker: $f:$line"; fi
  done
done
```

## Success Criteria
- [ ] All P10 TDD tests now pass
- [ ] Lock discipline upheld: lock always dropped before callback
- [ ] Single-pass state machine: one action per frame
- [ ] No double `player_response_input` call

## Failure Recovery
- rollback: `git restore rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P11.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P11
- files changed: `ffi.rs`, `talk_segue.rs`
- tests that now pass (were failing in P10)
- negative-proof results
- verification outputs
