# Phase 01: Gap Analysis

## Phase ID
`PLAN-20260314-MEMORY.P01`

## Prerequisites
- Required: Phase P00.5 (Preflight Verification) completed
- All baseline tests pass
- Integration state confirmed

## Purpose
Document the precise gaps between the current implementation and the specification/requirements, with exact file locations and requirement traceability.

## Gap Analysis

### Gap 1: Zero-size allocation OOM paths unchecked

**Location**: `rust/src/memory.rs`
- `rust_hmalloc` lines 11-12: `libc::malloc(1)` result returned without null check
- `rust_hcalloc` lines 44-45: `libc::malloc(1)` result used in `memset` without null check (would crash on null + memset, but crash would be a SIGSEGV, not a clean OOM abort)
- `rust_hrealloc` lines 68-69: `libc::malloc(1)` result returned without null check

**Specification reference**: Specification section 4.1: "If allocation fails for a zero-size request (the 1-byte fallback): this is also an unrecoverable error." Section 6.3: "OOM detection applies to all positive-size allocation requests and to the internal 1-byte allocations used for zero-size normalization."

**Requirements**: REQ-MEM-OOM-001, REQ-MEM-ZERO-001, REQ-MEM-OOM-003

**Fix**: Add null checks with `log_add(Fatal, ...)` + `abort()` after every `libc::malloc(1)` call in the zero-size paths.

**Risk**: Very low. The fix is adding 3-4 lines per function. The behavior change only fires when the system is actually out of memory and `malloc(1)` fails — an extreme edge case where the current behavior (returning null or crashing in memset) is already broken.

### Gap 2: `copy_argv_to_c` uses wrong deallocator for CString pointers

**Location**: `rust/src/memory.rs`
- Error-path cleanup, line 126: `libc::free(*ptr as *mut c_void)` on a `CString::into_raw()` pointer
- Test cleanup, lines 276: `libc::free(ptr as *mut c_void)` on `CString::into_raw()` pointers

**Specification reference**: Specification Appendix A.3: "The individual string pointers are `CString::into_raw()` results. These are Rust-owned allocations made through the Rust standard allocator, not through `HMalloc`. They must be reclaimed by reconstructing them via `CString::from_raw()`... never via `libc::free` or `HFree`."

**Requirements**: REQ-MEM-OWN-006, REQ-MEM-INT-008

**Fix**:
- Error path: replace `libc::free(*ptr as *mut c_void)` with `drop(CString::from_raw(*ptr))`
- Test: replace `libc::free(ptr as *mut c_void)` with `drop(CString::from_raw(ptr))`
- Remove the null check + panic block after `rust_hmalloc`; that branch is unreachable because `rust_hmalloc` aborts on OOM and therefore should be treated as a cleanup consequence of this allocator-family fix, not as an independent gap.

**Risk**: Low. On most platforms libc malloc and Rust's allocator share the same underlying allocator, so this was working by accident. But it's technically UB and the spec explicitly forbids it.

### Gap 3: Missing explicit unit-test coverage required by specification §14.1

**Location**: Current memory tests in `rust/src/memory.rs` test module

**Specification reference**: Specification section 14.1 explicitly requires unit tests covering `HFree(NULL)` safety and `HRealloc(NULL, size)` equivalence to `HMalloc(size)`.

**Requirements**: REQ-MEM-OWN-003, REQ-MEM-ALLOC-010

**Current state**: These behaviors appear to be implemented, but the current plan did not identify them as test-surface gaps even though the specification calls them out explicitly.

**Fix**: Add explicit unit tests in the module test suite for null-free safety and null-pointer realloc-as-malloc behavior. Keep these in the unit-test phase rather than treating them only as later integration-test coverage.

**Risk**: Very low. Additive tests only.

### Gap 4: No project-level mixed-language integration tests

**Specification reference**: Specification section 14.2 and REQ-MEM-INT-009 state that the project test suite should include dedicated mixed-language integration tests that exercise allocation in one language and deallocation in the other, zero-size normalization at the ABI seam, and lifecycle sequencing.

**Requirements**: REQ-MEM-INT-009 (program-level integration obligation)

**Current state**: The only tests are Rust-side unit tests in `memory.rs`. No C-side tests call through the macro boundary. No tests verify true C↔Rust ownership transfer. Rust integration tests against exported symbols are feasible and valuable, but they remain ABI-surface checks rather than full mixed-language seam coverage.

**Fix**: Add Rust-side integration tests in `rust/tests/` that exercise the exported ABI surface and explicitly document them as partial coverage for REQ-MEM-INT-009. Also require a concrete downstream handoff artifact for the remaining true C↔Rust seam harness work, including expected artifact path, project-level ownership, and acceptance criteria for real boundary-crossing tests.

**Risk**: Low. These are additive tests and documentation corrections.

### Gap 5: Missing requirement traceability markers

**Location**: `rust/src/memory.rs` — only `rust_hmalloc` line 18 has a `@requirement` marker.

**Requirements**: Plan guide traceability requirement

**Fix**: Add `@plan PLAN-20260314-MEMORY` and `@requirement REQ-MEM-*` markers to all six exported functions and `copy_argv_to_c`.

**Risk**: None. Documentation-only change.

## Entity/State Transition Notes

The memory subsystem has exactly two lifecycle states:
- **Uninitialized**: Before `rust_mem_init()` is called or after `rust_mem_uninit()` completes
- **Initialized**: After successful `rust_mem_init()` call

Current behavior already satisfies the idempotency contract functionally because the hooks are no-op/logging-compatible. This plan does not treat duplicate informational logging as a compliance defect. If implementation changes for clearer lifecycle state tracking are made later, they should be justified as cleanup rather than requirement gap closure.

## Integration Touchpoints

No new integration points are created. All changes are internal to `rust/src/memory.rs` except for new test files in `rust/tests/` and the required downstream tracking artifact for residual REQ-MEM-INT-009 work.

## Old Code to Replace/Remove

- `copy_argv_to_c` error-path dead branch (lines 122-130): remove entirely as part of the Gap 2 fix
- `copy_argv_to_c` test cleanup using `libc::free`: replace with `CString::from_raw`
