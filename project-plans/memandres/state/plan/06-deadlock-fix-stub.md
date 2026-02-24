# Phase 06: Deadlock Fix — Stub

## Phase ID
`PLAN-20260224-STATE-SWAP.P06`

## Prerequisites
- Required: Phase P05a (Seek Fix Verification) completed
- Expected: `StateFile` seek/read/write all working with separated used/physical

## Requirements Implemented (Expanded)

### REQ-SF-004: Copy Deadlock Prevention
**Requirement text**: `rust_copy_game_state` shall acquire the global mutex exactly once, not twice.

Behavior contract:
- GIVEN: `GLOBAL_GAME_STATE` contains a `GameState` behind a `Mutex`
- WHEN: `rust_copy_game_state(dest_bit, src_start, src_end)` is called
- THEN: Function completes without blocking; state bits are copied correctly

Why it matters:
- `load_legacy.c` calls `copyGameState(dest, target, src, begin, end)` with `dest == src`.
- Current Rust FFI acquires `GLOBAL_GAME_STATE.lock()` for source read, then tries again for dest write → deadlock on non-reentrant `Mutex`.

## Implementation Tasks

### Files to modify
- `rust/src/state/ffi.rs`
  - Identify the `rust_copy_game_state` function
  - Mark the double-lock section with `todo!("single-lock copy")` placeholder
  - Do NOT change the public FFI signature
  - marker: `@plan PLAN-20260224-STATE-SWAP.P06`
  - marker: `@requirement REQ-SF-004`

### Approach (from pseudocode lines 119–127)
The fix will:
1. Lock `GLOBAL_GAME_STATE` once
2. Snapshot source bytes within the lock
3. Create a temporary `GameState` from the snapshot
4. Call `copy_state(&temp_source, src_start, src_end, dest_bit)` on the locked state
5. Release lock

### Stub phase deliverable
- The `rust_copy_game_state` function body is replaced with `todo!("single-lock copy — P08")`
- This clearly marks that the function will panic if called (acceptable — legacy load path is not exercised in unit tests)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Compilation must succeed; the todo!() is valid Rust
cd rust && cargo check --workspace
```

## Structural Verification Checklist
- [ ] `rust_copy_game_state` function body contains `todo!()` marker
- [ ] Function signature unchanged: `extern "C" fn rust_copy_game_state(dest_bit: c_int, src_start_bit: c_int, src_end_bit: c_int)`
- [ ] No double-lock pattern remains
- [ ] Compilation succeeds

## Semantic Verification Checklist (Mandatory)
- [ ] The deadlocking code path is removed (no more double lock)
- [ ] The function will panic if called (acceptable in stub phase — will be fixed in P08)
- [ ] No other FFI functions were accidentally modified

## Success Criteria
- [ ] `rust_copy_game_state` has clear `todo!()` marker
- [ ] Code compiles
- [ ] Other FFI functions still work

## Failure Recovery
- rollback: `git checkout -- rust/src/state/ffi.rs`

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P06.md`

Contents:
- phase ID: P06
- files modified: `rust/src/state/ffi.rs`
- changes: replaced double-lock in `rust_copy_game_state` with `todo!()` stub
