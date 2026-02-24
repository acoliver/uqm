# Phase 06a: Deadlock Fix — Stub Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P06a`

## Prerequisites
- Required: Phase P06 completed
- Expected: `rust_copy_game_state` has `todo!()` body

## Structural Verification
- [ ] `rust_copy_game_state` function exists with correct FFI signature
- [ ] Function body contains `todo!()` (no double-lock pattern)
- [ ] `cargo check --workspace` passes

## Semantic Verification
- [ ] Double-lock removed: no two calls to `GLOBAL_GAME_STATE.lock()` in same function
- [ ] Other FFI functions unaffected (spot-check 2-3 functions compile and pass tests)

## Gate Decision
- [ ] PASS: proceed to P07
- [ ] FAIL: fix stub
