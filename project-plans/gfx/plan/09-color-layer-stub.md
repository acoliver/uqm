# Phase 09: Color Layer — Stub

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P09`

## Prerequisites
- Required: Phase P08a (Screen Compositing Verification) completed
- Expected files: ScreenLayer unscaled path working

## Requirements Implemented (Expanded)

### REQ-CLR-060: ColorLayer Uninitialized Guard
**Requirement text**: While the backend is not initialized, `rust_gfx_color` shall return immediately (void return).

Behavior contract:
- GIVEN: Backend not initialized
- WHEN: `rust_gfx_color` called
- THEN: Returns immediately, no side effects

### REQ-CLR-055: ColorLayer Negative Rect Guard
**Requirement text**: If `rect` is non-NULL and `rect->w < 0` or `rect->h < 0`, return immediately.

Behavior contract:
- GIVEN: Backend initialized
- WHEN: `rust_gfx_color` called with negative rect dimensions
- THEN: Returns immediately without rendering

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Replace `rust_gfx_color` no-op with compile-safe skeleton:
    - Guard: uninitialized check
    - Guard: negative rect dimension check
    - Body: `todo!("ColorLayer blend+fill")` (allowed in stub phase)
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P09`
  - marker: `@requirement REQ-CLR-060, REQ-CLR-055`

### Pseudocode traceability
- Uses pseudocode: component-005 lines 1–10 (guards)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_color` has guard clauses
- [ ] Parameters no longer prefixed with `_`
- [ ] `todo!()` in body only
- [ ] Compiles without errors

## Semantic Verification Checklist (Mandatory)
- [ ] Guard clauses return early for uninitialized and negative rect
- [ ] No fake behavior
- [ ] Existing tests still pass

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase — todo!() is expected
grep -n "todo!\|unimplemented!" rust/src/graphics/ffi.rs || echo "NONE"
```

## Success Criteria
- [ ] Stub compiles
- [ ] Guards in place
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P09.md`
