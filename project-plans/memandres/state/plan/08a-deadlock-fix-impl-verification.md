# Phase 08a: Deadlock Fix — Implementation Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P08a`

## Prerequisites
- Required: Phase P08 completed
- Expected: `rust_copy_game_state` uses single-lock with source snapshot

## Structural Verification
- [ ] `rust_copy_game_state` has exactly one `.lock()` call
- [ ] Source snapshot created via `GameState::from_bytes(state.as_bytes())`
- [ ] `catch_unwind` wraps the body
- [ ] Poisoned mutex handled
- [ ] No `todo!()` markers in `ffi.rs`

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Semantic Verification
- [ ] All 4 copy tests pass
- [ ] All state_file tests pass (no regressions from P05)
- [ ] All other FFI tests pass
- [ ] No deferred implementation markers

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/ffi.rs || echo "CLEAN"
```

## Gate Decision
- [ ] PASS: proceed to P09
- [ ] FAIL: fix implementation
