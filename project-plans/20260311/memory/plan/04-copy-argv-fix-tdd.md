# Phase 04: `copy_argv_to_c` Deallocator Fix (TDD)

## Phase ID
`PLAN-20260314-MEMORY.P04`

## Prerequisites
- Required: Phase P03 completed and verified
- Expected artifacts: `rust/src/memory.rs` with zero-size OOM checks and explicit unit-test gap closure

## Requirements Implemented (Expanded)

### REQ-MEM-OWN-006: No hidden ownership model change at ABI boundary
**Requirement text**: When C code or mixed-language FFI code uses the historical memory API, the subsystem shall preserve raw-pointer ownership expectations rather than requiring those callers to adopt language-native ownership constructs merely to remain correct.

Behavior contract:
- GIVEN: `copy_argv_to_c` creates CString pointers via `CString::into_raw()`
- WHEN: Those pointers need to be freed (error path or cleanup)
- THEN: They must be reclaimed via `CString::from_raw()`, not via `libc::free()` or `HFree`

Why it matters:
- `CString::into_raw()` produces Rust-allocator-owned memory. Using `libc::free()` on it is technically undefined behavior. On most platforms it works by accident because the Rust allocator uses libc malloc, but this is not guaranteed and violates allocator-family rules.

### Program-level note: REQ-MEM-INT-008 local support only
**Requirement text**: When a cross-language API transfers ownership of allocated memory and relies on this allocator family, the API shall document which allocator family owns the result and which deallocator the receiver should use.

Local plan treatment:
- GIVEN: `copy_argv_to_c` returns a tuple of (array_ptr, string_ptrs)
- WHEN: The caller needs to clean up
- THEN: The local doc comment should state that `array_ptr` is freed via `rust_hfree` and string pointers are reclaimed via `CString::from_raw()`

Why it matters:
- Mixed allocator-family code is a source of subtle UB bugs. Clear documentation prevents cross-freeing.
- This improves one local API, but does not claim project-wide closure of REQ-MEM-INT-008, which `requirements.md` classifies as a program-level integration obligation.

### REQ-MEM-OOM-004: No successful-null contract for positive-size requests
**Requirement text**: The subsystem shall not require callers to treat a null return as a normal success-path result.

Behavior contract:
- GIVEN: `copy_argv_to_c` calls `rust_hmalloc` which aborts on OOM
- WHEN: The allocation call is made
- THEN: The null-check branch after `rust_hmalloc` is dead code and should be removed

Why it matters:
- Dead error-handling code is misleading and contains the wrong deallocator as part of the same ownership bug.

## Implementation Tasks

### Files to modify
- `rust/src/memory.rs`
  - **Remove** dead null-check branch in `copy_argv_to_c` (lines 122-130): `rust_hmalloc` aborts on failure, so this branch is unreachable
  - **Update** doc comment on `copy_argv_to_c` to document allocator-family ownership:
    - Array pointer: freed via `rust_hfree`
    - String pointers: reclaimed via `CString::from_raw()`
  - **Fix** test `test_copy_argv_to_c`: replace `libc::free(ptr as *mut c_void)` with `drop(CString::from_raw(ptr))`
  - marker: `@plan PLAN-20260314-MEMORY.P04`
  - marker: `@requirement REQ-MEM-OWN-006`
  - marker: `@requirement REQ-MEM-INT-008`

### Pseudocode traceability
- Uses pseudocode lines: 41-57

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted test
Use the Phase P00.5-confirmed unit-test invocation for `test_copy_argv_to_c`
```

## Structural Verification Checklist
- [ ] Dead null-check branch removed from `copy_argv_to_c`
- [ ] `test_copy_argv_to_c` uses `CString::from_raw()` for string cleanup, not `libc::free()`
- [ ] Doc comment on `copy_argv_to_c` documents allocator-family ownership for both return components
- [ ] No skipped phases
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] `copy_argv_to_c` no longer contains unreachable error-handling code
- [ ] Test cleanup uses correct deallocator for each allocator family
- [ ] Test still passes and verifies string content, null termination, and pointer validity
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs
```

## Success Criteria
- [ ] Dead code removed
- [ ] Correct deallocator used in test
- [ ] Allocator-family ownership documented
- [ ] All verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/memory.rs`
- Blocking issues: None expected

## Phase Completion Marker
Create: `project-plans/20260311/memory/.completed/P04.md`
