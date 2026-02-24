# Phase 08: Deadlock Fix — Implementation

## Phase ID
`PLAN-20260224-STATE-SWAP.P08`

## Prerequisites
- Required: Phase P07a (TDD Verification) completed
- Expected: 4 failing tests for `rust_copy_game_state`

## Requirements Implemented (Expanded)

### REQ-SF-004: Copy Deadlock Prevention
**Requirement text**: `rust_copy_game_state` acquires global mutex exactly once.

### REQ-SF-008: Self-Copy Correctness
**Requirement text**: Source and destination may overlap in the same byte array.

## Implementation Tasks

### Files to modify
- `rust/src/state/ffi.rs`
  - marker: `@plan PLAN-20260224-STATE-SWAP.P08`
  - marker: `@requirement REQ-SF-004, REQ-SF-008`

### Specific changes (pseudocode lines 119–127)

Replace the `todo!()` body of `rust_copy_game_state` with:

```rust
#[no_mangle]
pub extern "C" fn rust_copy_game_state(
    dest_bit: c_int,
    src_start_bit: c_int,
    src_end_bit: c_int,
) {
    // @plan PLAN-20260224-STATE-SWAP.P08
    // @requirement REQ-SF-004
    let result = std::panic::catch_unwind(|| {
        let mut guard = match GLOBAL_GAME_STATE.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let state = match guard.as_mut() {
            Some(s) => s,
            None => return,  // not initialized
        };

        // Snapshot source bytes to avoid aliasing issues
        let src_snapshot = GameState::from_bytes(state.as_bytes());

        // Copy bits from snapshot to live state
        state.copy_state(
            &src_snapshot,
            src_start_bit as u32,
            src_end_bit as u32,
            dest_bit as u32,
        );
    });

    if result.is_err() {
        // Panic caught at FFI boundary — silently handle
    }
}
```

**Key design decisions:**
1. **Single lock acquisition**: Mutex locked once, held for entire operation
2. **Source snapshot**: `GameState::from_bytes(state.as_bytes())` creates a copy of the current state
3. **Copy from snapshot**: `state.copy_state(&src_snapshot, ...)` reads from the snapshot, writes to the live state
4. **Panic safety**: `catch_unwind` prevents panics crossing FFI boundary
5. **Poisoned mutex**: `into_inner()` recovers from prior panic

### Pseudocode traceability
- Uses pseudocode lines: 119–127

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_copy_game_state` acquires `GLOBAL_GAME_STATE.lock()` exactly once
- [ ] No second `.lock()` call within the function
- [ ] `GameState::from_bytes` used to snapshot source
- [ ] `state.copy_state` called with snapshot as source
- [ ] `catch_unwind` wraps the entire operation
- [ ] Poisoned mutex handled with `into_inner()`
- [ ] No `todo!()` markers remain in function

## Semantic Verification Checklist (Mandatory)
- [ ] `test_copy_game_state_no_deadlock` passes (no timeout/hang)
- [ ] `test_copy_game_state_basic` passes (bits copied correctly)
- [ ] `test_copy_game_state_self_overlapping` passes (overlapping ranges correct)
- [ ] `test_copy_game_state_uninitialized_returns_gracefully` passes
- [ ] All existing FFI tests pass
- [ ] All state_file tests pass
- [ ] No `todo!()`, `FIXME`, or `HACK` in ffi.rs

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All 4 copy tests pass (GREEN)
- [ ] All existing tests pass
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] No deferred implementation markers

## Failure Recovery
- rollback: `git checkout -- rust/src/state/ffi.rs`
- blocking issues: if `GameState::from_bytes` doesn't exist or `copy_state` signature mismatch

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P08.md`

Contents:
- phase ID: P08
- files modified: `rust/src/state/ffi.rs`
- changes: single-lock copy with source snapshot
- tests: all 4 copy tests pass, all existing tests pass
- verification: cargo fmt/clippy/test all clean
