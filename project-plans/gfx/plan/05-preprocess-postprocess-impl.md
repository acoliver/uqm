# Phase 05: Preprocess Fix + Postprocess Refactor — Implementation

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P05`

## Prerequisites
- Required: Phase P04a (TDD Verification) completed
- Expected files: Tests passing from P04

## Requirements Implemented (Expanded)

### REQ-PRE-010: Preprocess Blend Mode Reset
**Requirement text**: When `rust_gfx_preprocess` is called, the backend shall set the renderer blend mode to `BLENDMODE_NONE`.

Behavior contract:
- GIVEN: Backend initialized, new frame begins
- WHEN: `rust_gfx_preprocess` called
- THEN: Renderer blend mode is `BLENDMODE_NONE`, renderer cleared to black

### REQ-PRE-020: Preprocess Clear to Black
**Requirement text**: When `rust_gfx_preprocess` is called, the backend shall set the renderer draw color to opaque black (R=0, G=0, B=0, A=255) and clear the entire render target.

Behavior contract:
- GIVEN: Backend initialized
- WHEN: `rust_gfx_preprocess` called
- THEN: Draw color is (0,0,0,255), renderer is cleared

### REQ-POST-010: Postprocess Presents
**Requirement text**: The backend shall call `canvas.present()` to display the composed frame.

Behavior contract:
- GIVEN: All compositing complete
- WHEN: `rust_gfx_postprocess` called
- THEN: Frame is presented to display

### REQ-POST-020: Postprocess No Upload
**Requirement text**: The backend shall NOT upload surface pixel data, create textures, or call `canvas.copy` within `rust_gfx_postprocess`.

Behavior contract:
- GIVEN: ScreenLayer has done all compositing
- WHEN: `rust_gfx_postprocess` called
- THEN: Only `canvas.present()` — no texture, no copy, no upload

### REQ-PRE-040: Ignore Transition/Fade Params
**Requirement text**: `rust_gfx_preprocess` shall ignore the `transition_amount` and `fade_amount` parameters (they are handled by separate ScreenLayer/ColorLayer calls).

Behavior contract:
- GIVEN: `rust_gfx_preprocess(force_redraw, transition_amount, fade_amount)` is called
- WHEN: Parameters are received
- THEN: `transition_amount` and `fade_amount` are unused; only `force_redraw` is relevant (and itself ignored per REQ-PRE-030/REQ-SEQ-065)

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - **`rust_gfx_preprocess`**: Final implementation — already done in P03, verify complete
    - Line: `state.canvas.set_blend_mode(sdl2::render::BlendMode::None);`
    - Line: `state.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 255));`
    - Line: `state.canvas.clear();`
  - **`rust_gfx_postprocess`**: Final implementation — already done in P03, verify complete
    - Body: `if let Some(state) = get_gfx_state() { state.canvas.present(); }`
  - **Cleanup**: Remove any unused imports that were only used by old postprocess code
    (e.g., `Pixmap`, `PixmapFormat`, `Hq2xScaler`, `ScaleParams`, `Scaler`, `scale_rgba` — only if they are no longer used anywhere in the file. They WILL be re-used by ScreenLayer in P08, so defer removal.)
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P05`
  - marker: `@requirement REQ-PRE-010, REQ-PRE-020, REQ-PRE-040, REQ-POST-010, REQ-POST-020`

### Pseudocode traceability
- Uses pseudocode: component-002 lines 1–14
- Uses pseudocode: component-006 lines 1–8

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Preprocess has 3 operations: set_blend_mode, set_draw_color, clear
- [ ] Postprocess has 1 operation: present
- [ ] All tests pass (including new P04 tests)
- [ ] No unused imports (clippy would catch these)
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Preprocess behavior matches C reference (`sdl2_pure.c` lines 381–383)
- [ ] Postprocess behavior matches C reference (`sdl2_pure.c` lines 456–463, minus scanlines)
- [ ] No surface/texture code in postprocess
- [ ] Game will show black screen (expected — ScreenLayer still not implemented)
- [ ] No placeholder/deferred patterns remain in modified functions

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs || echo "CLEAN"
```

Note: `rust_gfx_screen` and `rust_gfx_color` may still have TODO/no-op patterns.
Those are addressed in subsequent phases. Only check the functions modified in this phase.

## Success Criteria
- [ ] Preprocess sets blend mode + color + clears (3 operations)
- [ ] Postprocess only presents (1 operation)
- [ ] All cargo gates pass
- [ ] All tests pass

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: unused import warnings from removing postprocess code

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P05.md`

Contents:
- phase ID: P05
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- tests added/updated: from P04
- verification: cargo fmt/clippy/test outputs
- semantic: preprocess correct, postprocess present-only
