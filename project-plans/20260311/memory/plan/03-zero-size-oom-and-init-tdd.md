# Phase 03: Zero-Size OOM Fix + Unit-Test Gap Closure (TDD)

## Phase ID
`PLAN-20260314-MEMORY.P03`

## Prerequisites
- Required: Phase P02 (Pseudocode) completed and verified
- Expected files from previous phase: `02-pseudocode.md`, `02a-pseudocode-verification.md`

## Requirements Implemented (Expanded)

### REQ-MEM-OOM-001: Fatal OOM policy for positive-size allocation requests
**Requirement text**: When `HMalloc`, `HCalloc`, or `HRealloc` receives a positive-size request that cannot be satisfied, the subsystem shall treat the condition as fatal and shall not require ordinary callers to recover from a null return.

Behavior contract:
- GIVEN: Any allocation function is called (including zero-size requests that internally allocate 1 byte)
- WHEN: The underlying `malloc(1)` returns null
- THEN: The subsystem logs a fatal message and aborts the process

Why it matters:
- Without this fix, `rust_hmalloc(0)` can return null, violating the non-null contract and potentially causing null-pointer dereferences in callers

### REQ-MEM-ZERO-001: Zero-size allocation returns non-null
**Requirement text**: When `HMalloc(0)` or `HCalloc(0)` is called, the subsystem shall return a non-null pointer that is safe to pass later to `HFree` or `HRealloc` under the same subsystem contract.

Behavior contract:
- GIVEN: A zero-size allocation request
- WHEN: The internal 1-byte fallback allocation succeeds
- THEN: A non-null pointer is returned
- WHEN: The internal 1-byte fallback allocation fails
- THEN: The subsystem aborts (not returns null)

Why it matters:
- Callers must never observe a null return from any allocation function

### REQ-MEM-OOM-003: No false OOM fatality for zero-size requests
**Requirement text**: When a zero-size allocation or reallocation request follows the subsystem's defined zero-size contract, the subsystem shall not treat implementation-defined libc zero-size behavior by itself as a positive-size out-of-memory failure.

Behavior contract:
- GIVEN: A zero-size allocation request
- WHEN: The request is processed
- THEN: The zero-size normalization (allocating 1 byte) is applied before any OOM check, so the OOM check applies to the 1-byte allocation, not to the zero-size request

Why it matters:
- The OOM path must fire on actual allocation failure, not on platform-specific `malloc(0)` semantics

### REQ-MEM-OWN-003: Null-free safety
**Requirement text**: When `HFree(NULL)` is called, the subsystem shall treat the call as a safe no-op.

Behavior contract:
- GIVEN: `rust_hfree(std::ptr::null_mut())`
- WHEN: The function is called
- THEN: It completes without crash, logging, or side effects visible to the caller

Why it matters:
- Specification §14.1 requires explicit unit coverage for this contract.

### REQ-MEM-ALLOC-010: Null-pointer reallocation as allocation
**Requirement text**: When `HRealloc(NULL, size)` is called with a positive size, the subsystem shall behave equivalently to `HMalloc(size)`.

Behavior contract:
- GIVEN: `rust_hrealloc(std::ptr::null_mut(), 64)`
- WHEN: The call succeeds
- THEN: It returns a non-null writable allocation compatible with `rust_hfree`

Why it matters:
- Specification §14.1 requires explicit unit coverage for this contract.

## Implementation Tasks

### Files to modify
- `rust/src/memory.rs`
  - `rust_hmalloc`: Add null check + abort after `libc::malloc(1)` in zero-size path (pseudocode lines 04-06)
  - `rust_hcalloc`: Add null check + abort after `libc::malloc(1)` in zero-size path (pseudocode lines 16-18)
  - `rust_hrealloc`: Add null check + abort after `libc::malloc(1)` in zero-size path (pseudocode lines 32-34)
  - `rust_hrealloc`: Remove redundant `&& size > 0` guard on line 74 (size > 0 is always true at that point)
  - Add explicit unit tests covering `HFree(NULL)` and `HRealloc(NULL, size)` in the existing test module
  - marker: `@plan PLAN-20260314-MEMORY.P03`

### Tests to add (in `rust/src/memory.rs` test module)
- `test_null_free_is_safe`: Call `rust_hfree(std::ptr::null_mut())`, verify the test completes without crash
- `test_realloc_null_ptr_acts_as_malloc`: Call `rust_hrealloc(std::ptr::null_mut(), 64)`, assert non-null, verify writable storage, free it

### Pseudocode traceability
- Uses pseudocode lines: 01-40 and 58-66

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted test run
Use the Phase P00.5-confirmed unit-test invocation for the memory module test target
```

## Structural Verification Checklist
- [ ] `rust/src/memory.rs` modified with OOM checks in zero-size paths
- [ ] New tests added for `HFree(NULL)` safety and `HRealloc(NULL, size)` behavior
- [ ] No skipped phases
- [ ] Plan/requirement traceability present in modified code

## Semantic Verification Checklist (Mandatory)
- [ ] Zero-size allocation paths now have null checks with abort
- [ ] `test_null_free_is_safe` passes without crash
- [ ] `test_realloc_null_ptr_acts_as_malloc` passes with non-null result and writable storage
- [ ] All existing tests still pass
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs
```

Note: Existing comments like "In later phases, this might initialize custom allocators" should be removed or updated as part of this phase, since the spec (section 7.3) says these are optional future extensions, not planned work.

## Success Criteria
- [ ] All three zero-size allocation paths have OOM checks
- [ ] Spec-required explicit unit tests for `HFree(NULL)` and `HRealloc(NULL, size)` exist and pass
- [ ] All verification commands pass
- [ ] Semantic checks pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/memory.rs`
- Blocking issues: Resolve repository-specific test invocation mismatches using the P00.5 findings before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/memory/.completed/P03.md`
