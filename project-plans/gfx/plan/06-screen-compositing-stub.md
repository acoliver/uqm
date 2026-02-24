# Phase 06: Screen Compositing — Stub

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P06`

## Prerequisites
- Required: Phase P05a (Preprocess/Postprocess Verification) completed
- Expected files: Preprocess and Postprocess finalized

## Requirements Implemented (Expanded)

### REQ-SCR-140: ScreenLayer Uninitialized Guard
**Requirement text**: While the backend is not initialized, `rust_gfx_screen` shall return immediately (void return).

Behavior contract:
- GIVEN: Backend is not initialized
- WHEN: `rust_gfx_screen` is called
- THEN: Returns immediately with no side effects

### REQ-SCR-100: ScreenLayer Out-of-Range Screen
**Requirement text**: If `screen` is out of range [0, `TFB_GFX_NUMSCREENS`), the backend shall return immediately.

Behavior contract:
- GIVEN: Backend is initialized
- WHEN: `rust_gfx_screen` is called with screen < 0 or screen >= 3
- THEN: Returns immediately without rendering

### REQ-SCR-090: ScreenLayer Extra Screen Skip
**Requirement text**: Where `screen` is 1 (`TFB_SCREEN_EXTRA`), `rust_gfx_screen` shall return immediately.

Behavior contract:
- GIVEN: Backend is initialized
- WHEN: `rust_gfx_screen(1, alpha, rect)` is called
- THEN: Returns immediately without rendering

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Replace `rust_gfx_screen` no-op with a compile-safe skeleton:
    - Guard: uninitialized check
    - Guard: screen range check
    - Guard: screen == 1 skip
    - Guard: null surface check
    - Body: `todo!("ScreenLayer upload+render")` (allowed in stub phase)
  - Add helper function `convert_c_rect` for SDL_Rect → sdl2::rect::Rect
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P06`
  - marker: `@requirement REQ-SCR-140, REQ-SCR-100, REQ-SCR-090, REQ-SCR-110`

### Pseudocode traceability
- Uses pseudocode: component-003 lines 1–13 (guards)
- Uses pseudocode: component-003B lines 1–11 (convert_rect)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_screen` has guard clauses for uninitialized, range, screen==1, null surface
- [ ] `convert_c_rect` helper function exists
- [ ] Parameters are no longer prefixed with `_` (they are used)
- [ ] Function compiles without errors
- [ ] `todo!()` only appears in the body (not in guards)

## Semantic Verification Checklist (Mandatory)
- [ ] Guard clauses return early (void) for all specified conditions
- [ ] `todo!()` is clearly temporary (stub phase allowance)
- [ ] No fake success behavior
- [ ] No hidden production shortcuts

## Deferred Implementation Detection (Mandatory)

```bash
# In stub phase, todo!() IS allowed but must be removed in impl phase
grep -n "todo!\|unimplemented!" rust/src/graphics/ffi.rs || echo "NONE"
# This should find the todo!() in the stub — that's expected for this phase
```

## Success Criteria
- [ ] Stub compiles
- [ ] Guards are in place
- [ ] `convert_c_rect` helper is reusable
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: compilation errors from new guard code

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P06.md`
