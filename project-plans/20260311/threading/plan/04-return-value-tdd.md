# Phase 04: Return Value Propagation — TDD

## Phase ID
`PLAN-20260314-THREADING.P04`

## Prerequisites
- Required: Phase 03a (Stub Verification) completed
- `spawn_c_thread` returns `Result<Thread<c_int>>`
- `rust_thread_join` internally casts to `Thread<c_int>` (signature still unchanged)
- All existing tests pass

## Requirements Implemented (Expanded)

### Thread return value propagation (plan-internal test slice)
**Requirement text**: "When WaitThread is called with a valid joinable thread handle and a non-null status pointer, the threading subsystem shall block until the target thread has terminated and shall store the thread entry function's integer return value in the caller-provided status location."

This phase establishes **Rust-generic** tests only. These tests are a plan-internal slice validating that `Thread<c_int>` preserves return values through `join()`. They are not a substitute for adapter/public-API verification; Phase 05 must add direct adapter-level coverage because the original bug lived at the C↔Rust boundary.

Behavior contracts to test:

1. GIVEN: A spawned `Thread<c_int>` returning 42
   WHEN: `join()` is called
   THEN: Returns `Ok(42)`

2. GIVEN: A spawned `Thread<c_int>` returning 0
   WHEN: `join()` is called
   THEN: Returns `Ok(0)` (distinguishable from error — `Err` would mean join failed)

3. GIVEN: A spawned `Thread<c_int>` returning -1
   WHEN: `join()` is called
   THEN: Returns `Ok(-1)`

4. GIVEN: Multiple threads each returning distinct `c_int` values
   WHEN: Each is joined
   THEN: Each returns its specific value

## Implementation Tasks

### Files to modify

#### `rust/src/threading/tests.rs` — New tests

All tests use the Rust-internal `Thread<c_int>` API directly (not FFI). This validates that the type change from P03 correctly preserves return values through the generic join path, which is necessary groundwork before adapter-level testing in P05.

1. **`test_thread_c_int_return_positive`**
   ```rust
   /// @plan PLAN-20260314-THREADING.P04
   #[test]
   fn test_thread_c_int_return_positive() {
       let thread = Thread::<c_int>::spawn(Some("ret_pos"), || 42)
           .expect("spawn should succeed");
       let result = thread.join().expect("join should succeed");
       assert_eq!(result, 42);
   }
   ```
   - marker: `@plan PLAN-20260314-THREADING.P04`

2. **`test_thread_c_int_return_zero`**
   ```rust
   /// @plan PLAN-20260314-THREADING.P04
   #[test]
   fn test_thread_c_int_return_zero() {
       let thread = Thread::<c_int>::spawn(Some("ret_zero"), || 0)
           .expect("spawn should succeed");
       let result = thread.join().expect("join should succeed");
       assert_eq!(result, 0);  // 0 is a valid return, not an error
   }
   ```

3. **`test_thread_c_int_return_negative`**
   ```rust
   /// @plan PLAN-20260314-THREADING.P04
   #[test]
   fn test_thread_c_int_return_negative() {
       let thread = Thread::<c_int>::spawn(Some("ret_neg"), || -1)
           .expect("spawn should succeed");
       let result = thread.join().expect("join should succeed");
       assert_eq!(result, -1);
   }
   ```

4. **`test_thread_c_int_return_multiple_values`**
   ```rust
   /// @plan PLAN-20260314-THREADING.P04
   #[test]
   fn test_thread_c_int_return_multiple_values() {
       let values: Vec<c_int> = vec![0, 1, -1, 42, 255, -128, i32::MAX, i32::MIN];
       let threads: Vec<_> = values.iter().enumerate().map(|(i, &v)| {
           Thread::<c_int>::spawn(Some(&format!("ret_{}", i)), move || v)
               .expect("spawn should succeed")
       }).collect();
       for (thread, &expected) in threads.into_iter().zip(values.iter()) {
           let result = thread.join().expect("join should succeed");
           assert_eq!(result, expected);
       }
   }
   ```

### Adapter-level coverage reserved for P05

Phase 05 must add at least one direct adapter/public-API verification path, for example:
- a Rust test invoking exported `rust_thread_spawn` / `rust_thread_join` through unsafe FFI with a C-compatible callback, or
- a C-side test exercising `CreateThread` / `WaitThread(&status)` end-to-end.

That phase must explicitly cover positive, zero, and negative return values so the legacy "0 means either success-with-zero or failure" public-API limitation is preserved while adapter-level success/failure remains distinguishable.

### Pseudocode traceability
- Tests verify behavior from pseudocode lines: 04-09 (closure captures c_int), 33-37 (join returns Ok(status))

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

**Expected:** All tests (old and new) should PASS. The `Thread<T>` generic already correctly wraps `JoinHandle<T>`, so `Thread<c_int>::spawn` returning a `c_int` and `join()` producing `Ok(c_int)` should work immediately. These tests validate the type change from P03, but they do not by themselves prove adapter correctness.

## Structural Verification Checklist
- [ ] 4 new test functions added to `tests.rs`
- [ ] Each test has a plan marker in doc comments
- [ ] Tests use `c_int` type (from `std::ffi::c_int`)
- [ ] `use std::ffi::c_int;` added to test imports if not present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Tests verify actual return values (42, 0, -1, boundary values), not just success/failure
- [ ] Zero return is tested separately to verify it's not confused with error in the generic join path
- [ ] Negative return is tested to verify sign preservation
- [ ] Boundary values (i32::MAX, i32::MIN) are tested
- [ ] This phase is explicitly limited to Rust-generic coverage; adapter/public-API coverage is deferred to P05, not omitted
- [ ] All existing tests still pass (baseline + 4)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/threading/tests.rs
```

No TODOs should appear in test code.

## Success Criteria
- [ ] 4 new tests compile and pass
- [ ] All existing tests pass (baseline + 4)
- [ ] Rust-generic test contracts established for P05 adapter/public-API verification

## Failure Recovery
- rollback: `git checkout -- rust/src/threading/tests.rs`
- blocking: if `Thread<c_int>` tests fail, revisit P03 stub

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P04.md`
