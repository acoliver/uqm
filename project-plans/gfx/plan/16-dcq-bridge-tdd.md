# Phase 16: DCQ FFI Bridge — TDD

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P16`

## Prerequisites
- Required: Phase P15a (DCQ Stub Verification) completed
- Expected: All `rust_dcq_*` stubs compile and link
- Expected: Global DCQ singleton declared

## Requirements Implemented (Expanded)

### REQ-DCQ-020: DCQ Push Commands (Test Coverage)
**Requirement text**: When C code calls `rust_dcq_push_*` functions, the
backend shall enqueue the corresponding draw command in the Rust DCQ.

Test contracts:
- `test_dcq_push_drawline` — push a line command, verify queue length is 1
- `test_dcq_push_drawrect` — push a rect command, verify queue length is 1
- `test_dcq_push_fillrect` — push a fill command, verify queue length is 1
- `test_dcq_push_drawimage` — push an image command, verify enqueued
- `test_dcq_push_multiple` — push 5 commands, verify queue length is 5

### REQ-DCQ-030: DCQ Flush (Test Coverage)
**Requirement text**: When `rust_dcq_flush` is called, the backend shall
process all enqueued commands in FIFO order.

Test contracts:
- `test_dcq_flush_empty` — flush empty queue, no crash, returns 0
- `test_dcq_flush_processes_all` — push 3 commands, flush, queue is empty
- `test_dcq_flush_fifo_order` — push line then rect, verify line executes first

### REQ-DCQ-040: DCQ Screen Binding (Test Coverage)
**Requirement text**: When `rust_dcq_set_screen` is called, the backend
shall direct subsequent draw commands to the specified screen surface.

Test contracts:
- `test_dcq_set_screen_valid` — set screen 0, get screen returns 0
- `test_dcq_set_screen_invalid` — set screen 99, returns error
- `test_dcq_set_screen_roundtrip` — set 2, get returns 2

### REQ-DCQ-050: DCQ Batch Mode (Test Coverage)
**Requirement text**: When batch mode is active, `rust_dcq_flush` shall
defer processing until `rust_dcq_unbatch` is called.

Test contracts:
- `test_dcq_batch_defers_flush` — batch, push, flush is no-op, unbatch flushes
- `test_dcq_nested_batch` — batch twice, unbatch once, still batched
- `test_dcq_unbatch_without_batch` — unbatch without batch, no crash

### REQ-FFI-030: Panic Safety (Test Coverage)
Test contracts:
- `test_dcq_push_catches_panic` — verify catch_unwind prevents panic propagation
- `test_dcq_flush_catches_panic` — verify flush handles internal errors gracefully

## Implementation Tasks

### Files to modify
- `rust/src/graphics/dcq_ffi.rs`
  - Add `#[cfg(test)] mod tests` block
  - Add all test functions listed above
  - Tests call the FFI functions directly (same-process, no actual C needed)
  - Tests must reset global DCQ state between runs (test isolation)
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P16`
  - marker: `@requirement REQ-DCQ-020, REQ-DCQ-030, REQ-DCQ-040, REQ-DCQ-050, REQ-FFI-030`

### Test Infrastructure
- Each test must initialize and teardown the global DCQ singleton
- Use a `reset_dcq()` helper for test isolation
- Tests should be `#[serial]` if using `serial_test` crate, or use
  internal locking to prevent test parallelism on the global singleton

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify test count
grep -c '#\[test\]' rust/src/graphics/dcq_ffi.rs
# Expected: >= 15

# Run DCQ tests specifically
cd rust && cargo test --lib -- dcq_ffi::tests --nocapture
```

## Structural Verification Checklist
- [ ] All test functions listed above are present
- [ ] Tests are in `#[cfg(test)] mod tests` block
- [ ] Each test has `@requirement` traceability comment
- [ ] Test isolation: global state reset between tests
- [ ] Tests compile (stubs may cause test failures — expected in TDD phase)

## Semantic Verification Checklist (Mandatory)
- [ ] Tests cover push, flush, screen binding, batch mode, and panic safety
- [ ] FIFO ordering test uses distinguishable command types
- [ ] Batch mode test verifies commands accumulate during batch
- [ ] Tests do not depend on SDL initialization (pure queue logic)
- [ ] Each test function name clearly describes the behavior under test

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcq_ffi.rs | grep -v 'todo!(' || echo "CLEAN"
```

## Success Criteria
- [ ] >= 15 test functions written
- [ ] Tests compile (failures expected — stubs still have `todo!()`)
- [ ] `cargo fmt` and `cargo clippy` pass
- [ ] Test names are descriptive and traceable to requirements

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/dcq_ffi.rs`
- blocking issues: DCQ singleton initialization prevents test isolation

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P16.md`

Contents:
- phase ID: P16
- timestamp
- files modified: `rust/src/graphics/dcq_ffi.rs`
- total tests: count
- test results: all compile, expected failures from stubs noted
- verification: cargo suite output
