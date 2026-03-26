# Phase 04a: NPC Phrase Verification

## Phase ID
`PLAN-20260326-COMMPT2.P04a`

## Prerequisites
- Required: Phase 04 (NPC Phrase) completed
- Phase completion marker exists: `project-plans/20260311/commpt2/.completed/P04.md`

## Structural Verification Checklist

- [ ] `rust_NPCPhrase_cb` in ffi.rs has a real implementation (not stub)
- [ ] `rust_NPCPhrase_splice` in ffi.rs has a real implementation (not stub)
- [ ] `c_get_conversation_phrase` is declared as extern "C" in ffi.rs (or a shared bridge module)
- [ ] `c_SpliceTrack` is declared as extern "C" in ffi.rs (or talk_segue.rs c_bridge)
- [ ] ConversationPhrases handle is accessible from the implementation
- [ ] Conversation summary append method exists and is called
- [ ] `@plan PLAN-20260326-COMMPT2.P04` markers present
- [ ] `@requirement REQ-NP-*` markers present
- [ ] No `P11: Stub` or `P11: Track` comments remain in NPCPhrase functions

## Semantic Verification Checklist

- [ ] `rust_NPCPhrase_cb` resolves phrase text (not just passing index through)
- [ ] Phrase text pointer is borrowed, not copied unnecessarily
- [ ] Callback parameter is forwarded to `c_SpliceTrack` correctly
- [ ] `rust_NPCPhrase_splice` calls `rust_NPCPhrase_cb` with `None`/`NULL` callback
- [ ] Invalid index (≤ 0) returns early without crashing
- [ ] Null text pointer from C returns early without crashing
- [ ] COMM_STATE lock is acquired and released properly (no deadlocks)
- [ ] Conversation summary grows with each phrase (verified by existing summary tests if any)
- [ ] Feature is reachable: trace from `commglue.c` → `rust_NPCPhrase_cb` → implementation

## Verification Commands

```bash
# All tests pass
cargo test --workspace --all-features

# Lint gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify no stubs remain
grep -n "P11:" rust/src/comm/ffi.rs | grep -i "stub\|track splicing remains"
# Expected: 0 matches in NPCPhrase functions

# Verify phrase resolution is wired
grep -c "c_get_conversation_phrase" rust/src/comm/ffi.rs
# Expected: at least 1 (the call)

grep -c "c_SpliceTrack" rust/src/comm/ffi.rs
# Expected: at least 1 (the call, may also be in talk_segue.rs)

# Verify conversation summary is updated
grep -n "summary\|append.*phrase\|add.*phrase" rust/src/comm/ffi.rs
# Expected: at least 1 match showing summary update

# Deferred implementation check
grep -A10 "rust_NPCPhrase" rust/src/comm/ffi.rs | grep -ic "todo\|fixme\|stub\|placeholder"
# Expected: 0

# C build
# (project-specific build with USE_RUST_COMM=on)
```

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks pass
- All semantic checks pass
- All 267+ comm tests pass
- No stubs remain in `rust_NPCPhrase_cb` or `rust_NPCPhrase_splice`
- Both build modes compile
- `cargo fmt`, `cargo clippy`, `cargo test` all green

**FAIL if**:
- Either NPCPhrase function is still a stub
- `c_get_conversation_phrase` is not called
- `c_SpliceTrack` is not called
- Conversation summary is not updated
- Any test regression
- Lock handling issues (deadlock potential)
