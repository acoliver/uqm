# Phase 13: Error Handling Hardening — Stub + TDD + Implementation

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P13`

## Prerequisites
- Required: Phase P12a (Scaling Verification) completed
- Expected files: All vtable functions implemented

## Requirements Implemented (Expanded)

### REQ-INIT-095: Already-Initialized Guard
> **Note**: Implementation of this guard was moved to P03 (earliest implementation phase) to prevent state leaks during iterative development. P13 verifies the guard is still present and adds a dedicated test.

### REQ-INIT-096: Post-Failure Init Retry
**Requirement text**: After a failed `rust_gfx_init` call, a subsequent call shall attempt initialization normally.

Behavior contract:
- GIVEN: Previous `rust_gfx_init` failed
- WHEN: `rust_gfx_init` called again
- THEN: Initialization attempted normally (failure does not permanently disable)

### REQ-ERR-050: Uninit Safe When Never Initialized
**Requirement text**: `rust_gfx_uninit` shall be safe to call even if init was never called.

Behavior contract:
- GIVEN: Backend never initialized
- WHEN: `rust_gfx_uninit` called
- THEN: No-op, no crash

### REQ-INV-060: Atomic Init State
**Requirement text**: The backend state shall be either fully initialized or fully uninitialized at all times.

Behavior contract:
- GIVEN: Init fails partway
- WHEN: Failure occurs
- THEN: All resources freed, state is fully uninitialized

### REQ-INV-061: Post-Failure Accessor Behavior
**Requirement text**: After a failed init, surface accessors return null and query functions return 0.

Behavior contract:
- GIVEN: Previous init failed
- WHEN: Surface accessors called
- THEN: Return null pointers

### REQ-FFI-030: No Panic Across FFI
**Requirement text**: No `extern "C" fn` shall allow a panic to propagate across the FFI boundary.

Behavior contract:
- GIVEN: Any FFI function is called
- WHEN: Internal error occurs
- THEN: Error is handled without panic (or catch_unwind is used)

### REQ-UTS-010: UploadTransitionScreen No-Op Documentation
**Requirement text**: While Rust backend uses unconditional full-surface upload, this is a no-op.

Behavior contract:
- GIVEN: Backend initialized or not
- WHEN: `rust_gfx_upload_transition_screen` called
- THEN: No-op (valid by design)

### REQ-SEQ-070: Out-of-Sequence Robustness
**Requirement text**: Backend shall not crash or corrupt state if vtable functions are called outside canonical sequence.

Behavior contract:
- GIVEN: ScreenLayer called without preceding Preprocess
- WHEN: Function executes
- THEN: No crash, no state corruption (may produce visual artifacts)

### REQ-INV-040: Repeated Postprocess Safety
**Requirement text**: Repeated `postprocess()` calls without intervening `preprocess()` shall not corrupt state.

Behavior contract:
- GIVEN: `postprocess` already called
- WHEN: `postprocess` called again without preprocess
- THEN: Same frame presented again, no corruption

### REQ-INV-050: Repeated Preprocess Clears
**Requirement text**: Repeated `preprocess()` calls shall each clear to black.

Behavior contract:
- GIVEN: `preprocess` called
- WHEN: `preprocess` called again without postprocess
- THEN: Renderer cleared to black again

### REQ-UNINIT-020: Resource Free Order
Uninit shall free resources in order: scaling buffers → surfaces → renderer → video → SDL.

### REQ-UNINIT-030: Not-Initialized Guard
If uninit is called when not initialized, it shall return immediately (no-op).

### REQ-SURF-020: Out-of-Range Index Returns Null
`rust_gfx_get_screen_surface(index)` with index outside `[0, TFB_GFX_NUMSCREENS)` shall return null.

### REQ-SURF-030: Uninitialized Returns Null
All surface accessors shall return null when the backend is not initialized.

### REQ-SURF-040: Backend Does Not Modify Surface Pixels
The backend shall never write to surface pixel data; surfaces are read-only from the backend's perspective.

### REQ-SURF-050: Format Conv Surface Has Alpha
`rust_gfx_get_format_conv_surf()` shall return a surface with an alpha mask (RGBA format).

### REQ-SURF-060: get_sdl_screen Returns surfaces[0]
`rust_gfx_get_sdl_screen()` shall return `surfaces[0]`.

### REQ-SURF-070: get_transition_screen Returns surfaces[2]
`rust_gfx_get_transition_screen()` shall return `surfaces[2]`.

### REQ-AUX-020: Toggle Fullscreen
`rust_gfx_toggle_fullscreen()` shall return 1 if now fullscreen, 0 if now windowed, -1 on error.

### REQ-AUX-030: Is Fullscreen
`rust_gfx_is_fullscreen()` shall return 1 if fullscreen, 0 if windowed.

### REQ-AUX-040: Set Gamma Returns -1
`rust_gfx_set_gamma()` shall return -1 unconditionally (gamma not supported by software renderer).

### REQ-AUX-041: Gamma Parameter Type
`rust_gfx_set_gamma` accepts an `f32` parameter.

### REQ-AUX-050: Get Width/Height
`rust_gfx_get_width()` shall return 320; `rust_gfx_get_height()` shall return 240.

### REQ-AUX-060: Uninitialized Safe Defaults
When not initialized, auxiliary functions shall return safe defaults (0 for queries, -1 for operations).

### REQ-FFI-050: Raw Pointer Only
FFI boundary shall use raw pointers (`*mut`, `*const`) only — no `&mut` references across FFI.

## FFI Panic Boundary Policy (REQ-FFI-030)

Every `#[no_mangle] pub extern "C" fn` in `ffi.rs` must satisfy one of:

1. **Provably panic-free**: The function body contains no `.unwrap()`,
   `.expect()`, array indexing that could panic, or arithmetic overflow
   paths. All fallible operations use `if let`, `match`, or `?`-to-default
   patterns. Document with `// PANIC-FREE: <reason>`.

2. **Wrapped in `catch_unwind`**: The function body is wrapped in
   `std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { ... }))`
   with a safe default return on panic. Use this only if proving
   panic-freedom is impractical.

### Verification procedure

```bash
# List all extern "C" functions
grep -n '#\[no_mangle\]' rust/src/graphics/ffi.rs

# For each, verify either PANIC-FREE comment or catch_unwind wrapper
grep -A2 'pub extern "C" fn' rust/src/graphics/ffi.rs | grep -E 'PANIC-FREE|catch_unwind' || echo "AUDIT NEEDED"

# Verify no unwrap()/expect() in production FFI paths (outside #[cfg(test)])
# Manual review: extract all code outside mod tests, grep for .unwrap() .expect()
```

### Exact `catch_unwind` pattern

When a function cannot be proven panic-free, use this exact pattern:

```rust
#[no_mangle]
pub extern "C" fn rust_gfx_example(arg: c_int) -> c_int {
    // CATCH_UNWIND: <reason this function needs catch_unwind>
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // --- function body here ---
        let state = match get_gfx_state() {
            Some(s) => s,
            None => return -1,
        };
        // ... rest of logic ...
        0
    })) {
        Ok(ret) => ret,
        Err(_) => {
            // Panic caught at FFI boundary — return safe default
            -1 // or 0, null, etc. depending on return type
        }
    }
}
```

For void-returning functions:
```rust
#[no_mangle]
pub extern "C" fn rust_gfx_example_void() {
    // CATCH_UNWIND: <reason>
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // --- function body here ---
    }));
}
```

### Policy for this plan

All FFI functions in this plan are designed to be provably panic-free:
- Uninitialized guards use `if let Some(state) = get_gfx_state()` (no unwrap)
- All fallible SDL operations use `if let Ok(...)` or `.ok()` (no unwrap)
- Array indexing uses range checks before access
- Rect conversion checks for negative values before `as u32` cast

If any function cannot be proven panic-free during implementation, it
MUST be wrapped in `catch_unwind` using the exact pattern above before
P13 is marked complete.

## Orphaned Error Requirement Coverage

### REQ-ERR-010: Uninitialized Default Safety
**Requirement text**: While the backend is not initialized, all FFI functions except `rust_gfx_init` shall return safe default values without crashing.

Behavior contract:
- GIVEN: Backend is not initialized
- WHEN: Any FFI function (except init) is called
- THEN: Returns safe default (null, 0, -1, or void) without crash

Test name: `test_all_ffi_functions_uninitialized_defaults` — @requirement REQ-ERR-010

### REQ-ERR-020: Partial Init Cleanup
**Requirement text**: If `rust_gfx_init` fails partway through initialization, it shall free all previously allocated resources before returning -1.

Behavior contract:
- GIVEN: Init begins and allocates some resources
- WHEN: A subsequent init step fails
- THEN: All previously allocated resources are freed, returns -1

Test name: `test_init_partial_failure_cleanup` — @requirement REQ-ERR-020
(Note: requires SDL2, may need `#[ignore]` for headless CI)

### REQ-ERR-030: No Per-Frame Logging
**Requirement text**: The backend shall not log errors from vtable functions during normal per-frame operation.

Behavior contract:
- GIVEN: Backend is initialized and running
- WHEN: Validation failures occur in ScreenLayer/ColorLayer
- THEN: Immediate return without logging

Verified by: code inspection — no `rust_bridge_log_msg` calls in `rust_gfx_screen` or `rust_gfx_color` validation paths.

### REQ-ERR-040: Init Failure Logging
**Requirement text**: The backend shall log diagnostic messages during `rust_gfx_init` failures via `rust_bridge_log_msg`.

Behavior contract:
- GIVEN: Init is in progress
- WHEN: An initialization step fails
- THEN: Diagnostic message logged before returning -1

Verified by: code inspection of `rust_gfx_init` error paths.

### REQ-ERR-060: Silent Vtable Failures (No Per-Frame Logging)
**Requirement text**: If `texture.update`, `canvas.copy`, or `canvas.fill_rect` fails during a vtable function call, the function shall return immediately without crashing and without emitting per-frame log messages. One-time diagnostic logging (e.g., `log_once`) is permitted to aid debugging; only repeated per-frame log spam is prohibited.

Behavior contract:
- GIVEN: A vtable function is executing
- WHEN: An SDL operation fails
- THEN: Returns immediately, no crash, no per-frame log. A one-time `log_once` diagnostic is allowed.

Test name: `test_vtable_functions_handle_sdl_errors_gracefully` — @requirement REQ-ERR-060
(Note: hard to trigger SDL failures in unit tests; verified by code inspection of `if let Ok(...)` patterns)

### REQ-FFI-040: `#[no_mangle] extern "C"` on All Exports
**Requirement text**: Every Rust function exported across the FFI boundary shall have both `#[no_mangle]` and `extern "C"` attributes.

Behavior contract:
- GIVEN: The `ffi.rs` module contains exported functions
- WHEN: Compiled and linked with C code
- THEN: Every `pub extern "C" fn` has a preceding `#[no_mangle]` attribute, ensuring symbol names are not mangled and calling convention is correct

Verification:
```bash
# Every pub extern "C" fn must be preceded by #[no_mangle]
grep -B1 'pub extern "C" fn' rust/src/graphics/ffi.rs | grep -v '#\[no_mangle\]' | grep 'pub extern' && echo "FAIL: missing #[no_mangle]" || echo "PASS"
```

### REQ-FFI-060: No Re-Entrant Mutable Access
**Requirement text**: No FFI-exported function shall call another FFI-exported function, preventing re-entrant mutable access to global state.

Behavior contract:
- GIVEN: An FFI-exported function is executing
- WHEN: It accesses global state via `get_gfx_state()`
- THEN: It never calls another `rust_gfx_*` function that would also access global state

Verification:
```bash
# No rust_gfx_* calls inside function bodies (exclude fn declarations and test code)
# Extract function bodies (between fn declaration and next #[no_mangle] or mod tests)
grep -n 'rust_gfx_' rust/src/graphics/ffi.rs | grep -v 'pub extern\|#\[no_mangle\]\|fn rust_gfx_\|// \|/// \|#\[cfg(test)\]' | grep -v 'mod tests' || echo "PASS: no re-entrant calls"
```

### REQ-THR-010: Single-Thread Assumption
**Requirement text**: The backend shall assume all vtable calls originate from a single thread (the C graphics thread).

Behavior contract:
- GIVEN: The `ffi.rs` source code
- WHEN: Searched for thread-spawning constructs
- THEN: No `thread::spawn`, `tokio`, `async`, `rayon`, or `crossbeam` found

Test name: `test_no_thread_spawning_in_ffi` — @requirement REQ-THR-010

### REQ-THR-020: No Synchronization Primitives
**Requirement text**: The backend shall not use `Mutex`, `RwLock`, `Atomic*`, or `Condvar` in `ffi.rs`.

Behavior contract:
- GIVEN: The `ffi.rs` source code
- WHEN: Searched for synchronization constructs
- THEN: No `Mutex`, `RwLock`, `Atomic*`, `Condvar` found

Test name: `test_no_sync_primitives_in_ffi` — @requirement REQ-THR-020

### REQ-THR-030: UnsafeCell for Global State
**Requirement text**: The backend's global state container (`GraphicsStateCell`) shall use `UnsafeCell` for interior mutability.

Behavior contract:
- GIVEN: The `ffi.rs` source code
- WHEN: `GraphicsStateCell` definition is inspected
- THEN: It wraps its inner state in `UnsafeCell`

Verified by: code inspection of `GraphicsStateCell` definition

### REQ-THR-035: `unsafe impl Sync` with Safety Proof
**Requirement text**: The `unsafe impl Sync for GraphicsStateCell` block shall include a `// SAFETY:` comment documenting the single-threaded access invariant.

Behavior contract:
- GIVEN: The `ffi.rs` source code
- WHEN: The `unsafe impl Sync` block is inspected
- THEN: A `// SAFETY:` comment is present explaining why single-threaded access makes this safe

Verification:
```bash
grep -A2 'unsafe impl Sync' rust/src/graphics/ffi.rs | grep -i 'SAFETY' || echo "FAIL: missing SAFETY comment"
```

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - **Verify REQ-INIT-095**: Confirm the already-initialized guard added in P03 is still present. Add test `test_init_already_initialized_returns_negative_one` if not already present.
  - **Update `rust_gfx_upload_transition_screen`**: Replace "No-op for now" comment with proper doc comment explaining the architectural invariant
  - **Panic boundary audit**: For every `#[no_mangle] pub extern "C" fn`, verify panic-free status per the policy above. Add `// PANIC-FREE:` comments or `catch_unwind` wrappers as needed.
  - **Verify REQ-FFI-040**: Confirm every `pub extern "C" fn` has `#[no_mangle]`. Add missing attributes if needed.
  - **Verify REQ-FFI-060**: Confirm no FFI function calls another FFI function. Document with grep verification.
  - **Verify REQ-THR-010/020/030/035**: Confirm threading model constraints. Add `// SAFETY:` comment to `unsafe impl Sync` if missing.
  - **Add tests**:
    - `test_uninit_without_init_no_panic` — @requirement REQ-ERR-050
    - `test_upload_transition_screen_no_panic` — @requirement REQ-UTS-010
    - `test_upload_transition_screen_uninitialized_no_panic` — @requirement REQ-UTS-030
    - `test_screen_uninitialized_all_variants` — @requirement REQ-ERR-012
    - `test_surface_accessors_uninitialized` — @requirement REQ-ERR-011
    - `test_aux_functions_uninitialized` — @requirement REQ-ERR-013, REQ-ERR-014
    - `test_out_of_sequence_no_crash` — @requirement REQ-SEQ-070
    - `test_repeated_postprocess_no_crash` — @requirement REQ-INV-040
    - `test_repeated_preprocess_no_crash` — @requirement REQ-INV-050
    - `test_all_ffi_functions_uninitialized_defaults` — @requirement REQ-ERR-010
    - `test_init_already_initialized_returns_negative_one` — @requirement REQ-INIT-095
    - `test_no_per_frame_logging_in_vtable` — @requirement REQ-ERR-030 (code inspection / grep-based)
    - `test_no_thread_spawning_in_ffi` — @requirement REQ-THR-010 (grep-based: no thread::spawn/tokio/async/rayon)
    - `test_no_sync_primitives_in_ffi` — @requirement REQ-THR-020 (grep-based: no Mutex/RwLock/Atomic*/Condvar)
    - `test_all_exports_have_no_mangle` — @requirement REQ-FFI-040 (grep-based: every extern "C" has #[no_mangle])
    - `test_no_reentrant_ffi_calls` — @requirement REQ-FFI-060 (grep-based: no rust_gfx_* calls inside function bodies)
  - **Verify no-panic safety**: Audit all FFI functions for potential panic paths. Ensure all match/unwrap/expect are in non-production test code only.
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P13`
  - marker: `@requirement REQ-INIT-095, REQ-INIT-096, REQ-ERR-010, REQ-ERR-020, REQ-ERR-030, REQ-ERR-040, REQ-ERR-050, REQ-ERR-060, REQ-INV-060, REQ-INV-061, REQ-FFI-030, REQ-FFI-040, REQ-FFI-050, REQ-FFI-060, REQ-THR-010, REQ-THR-020, REQ-THR-030, REQ-THR-035, REQ-SEQ-070, REQ-INV-040, REQ-INV-050, REQ-UNINIT-020, REQ-UNINIT-030, REQ-SURF-020, REQ-SURF-030, REQ-SURF-040, REQ-SURF-050, REQ-SURF-060, REQ-SURF-070, REQ-AUX-020, REQ-AUX-030, REQ-AUX-040, REQ-AUX-041, REQ-AUX-050, REQ-AUX-060`

### Pseudocode traceability
- Uses pseudocode: component-001A lines 2–4 (init guard)
- Uses pseudocode: component-001B lines 2–4 (uninit guard)
- Uses pseudocode: component-006B lines 1–6 (upload transition)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] REQ-INIT-095 guard exists in `rust_gfx_init` (added in P03, verified here)
- [ ] `rust_gfx_upload_transition_screen` has proper doc comment
- [ ] All new tests compile and pass
- [ ] All existing tests still pass
- [ ] No `unwrap()` or `expect()` in production FFI paths
- [ ] Every `#[no_mangle] pub extern "C" fn` has `// PANIC-FREE:` comment or `catch_unwind` wrapper (REQ-FFI-030)
- [ ] Plan/requirement markers present on new tests
- [ ] REQ-ERR-010 tested: `test_all_ffi_functions_uninitialized_defaults`
- [ ] REQ-ERR-020 tested: `test_init_partial_failure_cleanup` (may need `#[ignore]`)
- [ ] REQ-ERR-030 verified: `test_no_per_frame_logging_in_vtable` (code inspection / grep)
- [ ] REQ-ERR-040 verified: code inspection of `rust_gfx_init` error paths for `rust_bridge_log_msg`
- [ ] REQ-ERR-060 tested: `test_vtable_functions_handle_sdl_errors_gracefully` (code inspection of `if let Ok(...)` patterns)
- [ ] REQ-FFI-040 verified: `test_all_exports_have_no_mangle` — every `pub extern "C" fn` has `#[no_mangle]`
- [ ] REQ-FFI-060 verified: `test_no_reentrant_ffi_calls` — no `rust_gfx_*` calls inside function bodies
- [ ] REQ-THR-010 verified: `test_no_thread_spawning_in_ffi` — no thread::spawn/tokio/async/rayon
- [ ] REQ-THR-020 verified: `test_no_sync_primitives_in_ffi` — no Mutex/RwLock/Atomic*/Condvar
- [ ] REQ-THR-030 verified: `GraphicsStateCell` uses `UnsafeCell` (code inspection)
- [ ] REQ-THR-035 verified: `unsafe impl Sync` has `// SAFETY:` comment (code inspection)

## Semantic Verification Checklist (Mandatory)
- [ ] All FFI functions tested with uninitialized state (REQ-ERR-010)
- [ ] Uninit-without-init is safe (REQ-ERR-050)
- [ ] Out-of-sequence calls don't crash (REQ-SEQ-070)
- [ ] Repeated preprocess/postprocess calls don't corrupt (REQ-INV-040/050)
- [ ] Surface accessors return null when uninitialized (REQ-ERR-011)
- [ ] Auxiliary functions return correct defaults when uninitialized (REQ-ERR-013/014)
- [ ] No per-frame logging in vtable functions (REQ-ERR-030)
- [ ] Init failure logs diagnostics (REQ-ERR-040)
- [ ] Panic boundary policy satisfied for all extern "C" functions (REQ-FFI-030)
- [ ] All exports use `#[no_mangle]` + `extern "C"` (REQ-FFI-040)
- [ ] No re-entrant FFI calls in function bodies (REQ-FFI-060)
- [ ] No thread spawning in ffi.rs (REQ-THR-010)
- [ ] No synchronization primitives in ffi.rs (REQ-THR-020)
- [ ] Global state uses UnsafeCell (REQ-THR-030)
- [ ] `unsafe impl Sync` has SAFETY comment (REQ-THR-035)

## Deferred Implementation Detection (Mandatory)

```bash
# Final check — NO deferred patterns should exist anywhere in ffi.rs
grep -n "TODO\|FIXME\|HACK\|todo!\|unimplemented!\|for now\|will be implemented\|placeholder" rust/src/graphics/ffi.rs && echo "FAIL: deferred code found" || echo "CLEAN"
```

## Success Criteria
- [ ] All error handling requirements verified by tests
- [ ] No deferred patterns in entire ffi.rs
- [ ] All cargo gates pass
- [ ] No `unwrap()`/`expect()` in production FFI code
- [ ] Every `extern "C" fn` satisfies panic boundary policy (PANIC-FREE or catch_unwind)
- [ ] All REQ-ERR-* requirements have explicit test names or verified-by-inspection notes
- [ ] All REQ-FFI-040/060 verified (no_mangle + extern "C", no re-entrant calls)
- [ ] All REQ-THR-010/020/030/035 verified (threading model constraints)

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: discovering panic paths that need catch_unwind

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P13.md`

Contents:
- phase ID: P13
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- tests added: error handling + robustness tests
- verification: cargo fmt/clippy/test outputs
- semantic: all error paths tested, no deferred patterns
