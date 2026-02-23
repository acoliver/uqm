# Phase 04: Preprocess Fix + Postprocess Refactor — TDD

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P04`

## Prerequisites
- Required: Phase P03a (Stub Verification) completed
- Expected files: `rust/src/graphics/ffi.rs` with postprocess reduced

## Requirements Implemented (Expanded)

### REQ-PRE-050: Preprocess Uninitialized Guard
**Requirement text**: While the backend is not initialized, `rust_gfx_preprocess` shall return immediately (void return).

Behavior contract:
- GIVEN: The backend is not initialized (no `rust_gfx_init` called)
- WHEN: `rust_gfx_preprocess` is called
- THEN: Returns immediately with no side effects

### REQ-POST-030: Postprocess Uninitialized Guard
**Requirement text**: While the backend is not initialized, `rust_gfx_postprocess` shall return immediately (void return).

Behavior contract:
- GIVEN: The backend is not initialized
- WHEN: `rust_gfx_postprocess` is called
- THEN: Returns immediately with no side effects

### REQ-INV-050: Repeated Preprocess Clears
**Requirement text**: Repeated `preprocess()` calls without an intervening `postprocess()` shall each result in a black frame base.

Behavior contract:
- GIVEN: Preprocess has already been called
- WHEN: Preprocess is called again without postprocess between
- THEN: Renderer is cleared to black again (no accumulation)

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Add tests to `#[cfg(test)] mod tests`:
    - `test_preprocess_uninitialized_no_panic` — verify no crash when uninitialized
    - `test_postprocess_uninitialized_no_panic` — verify no crash when uninitialized
    - `test_postprocess_has_no_texture_operations` — static analysis (grep-based or code inspection)
      - Harness: This is a static analysis verification, not a runtime test. Verify via: `grep -n 'texture\|update\|copy' rust/src/graphics/ffi.rs | grep postprocess` returns no matches. Add to verification checklist, not test suite.
    - `test_sdl_rect_default` — verify SDL_Rect::default() is zeroed
    - `test_init_creates_surfaces` — after init, `get_screen_surface(0..2)` returns non-null — @requirement REQ-INIT-030
    - `test_init_creates_renderer` — after init, canvas operations (e.g. set_draw_color) do not panic — @requirement REQ-INIT-020
    - `test_init_fullscreen_flag` — init with fullscreen bit set stores the flag in state — @requirement REQ-INIT-050
    - `test_init_scaling_buffers` — init with SCALE_SOFT_ONLY flag allocates 3 scaled_buffers — @requirement REQ-INIT-060
    - `test_init_returns_zero` — successful init returns 0 — @requirement REQ-INIT-080
    - `test_init_partial_failure_cleanup` — if surface allocation fails partway, all prior surfaces are freed (no leak) — @requirement REQ-INIT-090
      - Harness: Uses a test-only error injection point: pass an invalid display index or impossible resolution to force SDL init failure. Verify surfaces created before failure point are freed.
    - `test_init_logs_on_failure` — a failing init path emits a diagnostic via rust_bridge_log_msg — @requirement REQ-INIT-100
      - Harness: Uses a test logger sink (Vec<String> behind Mutex) registered before test. After forcing init failure, assert log buffer contains expected diagnostic message.
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P04`
  - marker: `@requirement REQ-PRE-050, REQ-POST-030, REQ-INV-050, REQ-INIT-020, REQ-INIT-030, REQ-INIT-050, REQ-INIT-060, REQ-INIT-080, REQ-INIT-090, REQ-INIT-100`

### Pseudocode traceability
- Tests validate: component-002 lines 2–5, component-006 lines 2–3

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] New test functions added to `mod tests`
- [ ] Tests compile
- [ ] Tests have plan/requirement markers in comments
- [ ] No production code changes in this phase

## Semantic Verification Checklist (Mandatory)
- [ ] Tests verify behavior (uninitialized safety), not implementation internals
- [ ] Tests would fail if the uninitialized guard were removed
- [ ] No mock-only assertions

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All new tests pass
- [ ] Tests verify the stated behavior contracts

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: SDL2 test harness issues

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P04.md`