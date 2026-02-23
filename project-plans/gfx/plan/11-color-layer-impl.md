# Phase 11: Color Layer — Implementation

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P11`

## Prerequisites
- Required: Phase P10a (TDD Verification) completed
- Expected files: ColorLayer tests passing

## Requirements Implemented (Expanded)

### REQ-CLR-010: Set Draw Color
**Requirement text**: The backend shall set the renderer draw color to `(r, g, b, a)`.

Behavior contract:
- GIVEN: Backend initialized
- WHEN: `rust_gfx_color(128, 64, 32, 200, null)` called
- THEN: Renderer draw color is (128, 64, 32, 200)

### REQ-CLR-020: Opaque Blend Mode (a=255)
**Requirement text**: Where `a` is 255, set blend mode to `BLENDMODE_NONE`.

Behavior contract:
- GIVEN: a=255
- WHEN: Blend mode set
- THEN: BLENDMODE_NONE (fully opaque, overwrites)

### REQ-CLR-030: Alpha Blend Mode (a<255)
**Requirement text**: Where `a` is less than 255, set blend mode to `BLENDMODE_BLEND`.

Behavior contract:
- GIVEN: a=128
- WHEN: Blend mode set
- THEN: BLENDMODE_BLEND (composited over existing)

### REQ-CLR-040: Fill Entire Screen (rect=NULL)
**Requirement text**: Where `rect` is NULL, fill the entire renderer area.

Behavior contract:
- GIVEN: rect is null
- WHEN: fill_rect called
- THEN: `canvas.fill_rect(None)` — fills entire renderer

### REQ-CLR-050: Fill Rectangle (rect non-NULL)
**Requirement text**: Where `rect` is non-NULL, fill only the specified region.

Behavior contract:
- GIVEN: rect = {x:10, y:20, w:100, h:80}
- WHEN: fill_rect called
- THEN: Only the specified region is filled

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Replace `rust_gfx_color` todo!() with full implementation:
    1. Guards already in place from P09
    2. Set blend mode based on alpha (NONE for 255, BLEND for <255) — REQ-CLR-020/030
    3. Set draw color (r, g, b, a) — REQ-CLR-010
    4. Fill rect: None for null rect, Some(converted) for non-null — REQ-CLR-040/050
  - Reuse `convert_c_rect` helper from P06
  - Add `// SAFETY:` comment for rect pointer dereference
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P11`
  - marker: `@requirement REQ-CLR-010..050`

### Pseudocode traceability
- Uses pseudocode: component-005 lines 1–31

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_color` fully implemented (no todo!())
- [ ] Blend mode set before draw color (ordering per requirements note)
- [ ] Draw color set before fill_rect
- [ ] `convert_c_rect` or equivalent used for rect conversion
- [ ] `// SAFETY:` comment on unsafe rect dereference
- [ ] fill_rect result ignored (let _ =)
- [ ] Plan/requirement markers present

## Semantic Verification Checklist (Mandatory)
- [ ] Matches C reference `TFB_SDL2_ColorLayer` (`sdl2_pure.c` lines 447–454)
- [ ] Blend mode is correct: None for a=255, Blend for a<255
- [ ] Color fill happens for both null and non-null rect
- [ ] No placeholder patterns remain
- [ ] Fades should now work visually (fade-to-black, fade-to-white)

## Deferred Implementation Detection (Mandatory)

```bash
sed -n '/fn rust_gfx_color/,/^pub extern "C" fn\|^#\[no_mangle\]/p' rust/src/graphics/ffi.rs | grep "TODO\|FIXME\|HACK\|todo!\|unimplemented!" && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] ColorLayer fully implemented
- [ ] All P10 tests pass
- [ ] All cargo gates pass
- [ ] No placeholder patterns

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: rect conversion issues

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P11.md`

Contents:
- phase ID: P11
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- functions: rust_gfx_color (complete)
- tests: P10 tests verified passing
- verification: cargo fmt/clippy/test outputs
- semantic: fade effects should now work
