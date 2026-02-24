# Phase 07: Deadlock Fix — TDD

## Phase ID
`PLAN-20260224-STATE-SWAP.P07`

## Prerequisites
- Required: Phase P06a (Stub Verification) completed
- Expected: `rust_copy_game_state` has `todo!()` body

## Requirements Implemented (Expanded)

### REQ-SF-004: Copy Deadlock Prevention
**Requirement text**: `rust_copy_game_state` acquires global mutex once, copies bits correctly.

Behavior contract:
- GIVEN: GameState with bits set at positions 0–51
- WHEN: `rust_copy_game_state(100, 0, 52)` called (copy bits 0–51 → starting at bit 100)
- THEN: Bits at positions 100–151 match bits at 0–51; no deadlock

### REQ-SF-008: Self-Copy Correctness
**Requirement text**: Copy where source and destination overlap within same array works correctly.

Behavior contract:
- GIVEN: GameState with 0xFF at bits 0–7
- WHEN: `rust_copy_game_state(16, 0, 8)` (copy 8 bits from start=0 to dest_bit=16)
- THEN: Bits 16–23 == 0xFF; original bits 0–7 still == 0xFF

## Implementation Tasks

### Files to modify
- `rust/src/state/ffi.rs` — add tests
  - marker: `@plan PLAN-20260224-STATE-SWAP.P07`
  - marker: `@requirement REQ-SF-004, REQ-SF-008`

### Tests to add

```rust
// @plan PLAN-20260224-STATE-SWAP.P07
// @requirement REQ-SF-004
#[test]
fn test_copy_game_state_no_deadlock() {
    // Initialize global game state with known pattern
    // Call rust_copy_game_state — must complete without hanging
    // Verify copied bits match source
    // Use a timeout mechanism (thread + join with duration) to detect deadlock
}

// @requirement REQ-SF-004
#[test]
fn test_copy_game_state_basic() {
    // Set bits at positions 0-7 to 0xAB
    // Copy from bit 0 to bit 64 (8 bits)
    // Verify bits at 64-71 == 0xAB
    // Verify bits at 0-7 still == 0xAB
}

// @requirement REQ-SF-008
#[test]
fn test_copy_game_state_self_overlapping() {
    // Set bits at positions 0-15 to known pattern
    // Copy from bit 0 to bit 8 (overlapping 8 bits)
    // The C behavior: reads first, then writes (8 bits at a time)
    // Verify correct result matches C copyGameState behavior
}

// @requirement REQ-SF-004
#[test]
fn test_copy_game_state_uninitialized_returns_gracefully() {
    // Ensure calling rust_copy_game_state when GLOBAL_GAME_STATE is None
    // doesn't panic (returns gracefully)
}
```

### Pseudocode traceability
- Tests map to pseudocode lines 119–127

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Tests should FAIL because rust_copy_game_state has todo!()
cd rust && cargo test --workspace --all-features -- copy_game_state 2>&1 || echo "Expected failures (todo!)"
```

## Structural Verification Checklist
- [ ] 4 tests added for copy_game_state
- [ ] Tests have plan/requirement markers
- [ ] Tests compile

## Semantic Verification Checklist (Mandatory)
- [ ] `test_copy_game_state_no_deadlock` uses thread/timeout to detect deadlock
- [ ] `test_copy_game_state_basic` verifies actual bit values after copy
- [ ] `test_copy_game_state_self_overlapping` tests the legacy load scenario (dest==src array)
- [ ] Tests fail because of `todo!()` (RED phase confirmed)

## Success Criteria
- [ ] All 4 tests compile
- [ ] Tests panic on `todo!()` (RED — implementation not yet done)

## Failure Recovery
- rollback: `git checkout -- rust/src/state/ffi.rs`

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P07.md`

Contents:
- phase ID: P07
- files modified: `rust/src/state/ffi.rs`
- tests added: 4 tests for copy deadlock/correctness
- RED phase verified: tests panic on todo!()
