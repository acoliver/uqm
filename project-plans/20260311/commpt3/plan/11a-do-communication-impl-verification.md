# Phase 11a: DoCommunication Implementation Verification

## Phase ID
`PLAN-20260325-COMMPT3.P11a`

## Prerequisites
- Required: Phase P11 completed
- Expected artifacts: Rewritten `ffi.rs` and `talk_segue.rs`

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify enum shape
grep -A10 "enum CommunicationResult" rust/src/comm/talk_segue.rs

# Verify no player_response_input in ffi.rs
grep -c "player_response_input" rust/src/comm/ffi.rs

# Verify single call in do_communication
grep -n "player_response_input" rust/src/comm/talk_segue.rs | grep -v "fn player_response_input\|test\|cfg(test)\|//\|///\|mod "

# Verify lock-drop-before-callback
grep -n "drop\|callback" rust/src/comm/ffi.rs | head -20

# Verify single lock acquisition
awk '/fn rust_DoCommunication/,/^}/' rust/src/comm/ffi.rs | grep -c "COMM_STATE.write"

# Verify select_response return type
grep -A5 "fn select_response" rust/src/comm/talk_segue.rs | head -6
```

## Structural Verification Checklist
- [ ] `CommunicationResult` has exactly 4 variants
- [ ] `do_communication` returns rich result (not always `Talking`)
- [ ] `select_response` returns `Option<(..)>` (not always `None`)
- [ ] `rust_DoCommunication`: one lock, 4 match arms, explicit drop
- [ ] No `player_response_input` in `ffi.rs`
- [ ] No `todo!()` or placeholder markers

## Semantic Verification Checklist (Mandatory)

### Lock Discipline
- [ ] Exactly one `COMM_STATE.write()` acquisition per call
- [ ] `drop(state)` in ALL match arms
- [ ] In Selected arm, `drop(state)` precedes callback
- [ ] No recursive write lock possible

### State Machine
- [ ] `!talking_finished` → `alien_talk_segue`, NOT `player_response_input`
- [ ] `responses.count() > 0` → exactly one `player_response_input`
- [ ] `responses.count() == 0` → `run_last_replay` → `Done`
- [ ] Abort/load → `Done` immediately

### select_response Integrity
- [ ] Pre-callback work under lock (stop track, clear subtitles, fade music, etc.)
- [ ] `talking_finished` set to `false`
- [ ] Returns `None` for null callback
- [ ] Returns `Some((fn, ref))` for valid callback

### Test Coverage
- [ ] All P10 TDD tests PASS
- [ ] All 268+ comm tests pass
- [ ] Both build modes compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Moving `drop(state)` after callback invocation causes test 9 to fail
- [ ] **Confirmed**: Adding duplicate `player_response_input` call causes test 10 to fail
- [ ] **Confirmed**: Allowing response input during talking phase causes test 1 to fail

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P11a.md`
