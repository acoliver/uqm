# Phase 08: Screen Compositing — Implementation

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P08`

## Prerequisites
- Required: Phase P07a (TDD Verification) completed
- Expected files: All ScreenLayer tests passing

## Requirements Implemented (Expanded)

### REQ-SCR-010: ScreenLayer Composites Screen Surfaces
**Requirement text**: When `rust_gfx_screen` is called with a compositable screen (0 or 2), the backend shall read pixel data from `surfaces[screen]` and render it onto the current frame.

Behavior contract:
- GIVEN: Backend initialized, C has drawn to surfaces[screen]
- WHEN: `rust_gfx_screen(0, 255, NULL)` called
- THEN: Surface pixel data is uploaded to a texture and rendered onto the frame

### REQ-SCR-020: Full Surface Upload
**Requirement text**: The backend shall upload the full surface pixel data on every call.

Behavior contract:
- GIVEN: Backend initialized
- WHEN: ScreenLayer called for any compositable screen
- THEN: Full surface uploaded (no dirty-region optimization)

### REQ-SCR-030: Opaque Rendering (alpha=255)
**Requirement text**: Where `alpha` is 255, render with `BLENDMODE_NONE`.

Behavior contract:
- GIVEN: ScreenLayer called with alpha=255
- WHEN: Texture is rendered
- THEN: Blend mode is NONE (overwrites existing content)

### REQ-SCR-040: Alpha-Blended Rendering (alpha<255)
**Requirement text**: Where `alpha` is less than 255, render with `BLENDMODE_BLEND` and alpha modifier.

Behavior contract:
- GIVEN: ScreenLayer called with alpha=128
- WHEN: Texture is rendered
- THEN: Blend mode is BLEND, alpha mod is 128

### REQ-SCR-050: Full Screen (rect=NULL)
**Requirement text**: Where `rect` is NULL, render entire surface to full renderer area.

Behavior contract:
- GIVEN: ScreenLayer called with NULL rect
- WHEN: canvas.copy called
- THEN: src_rect=None, dst_rect=None (full screen)

### REQ-SCR-060: Clipped Region (rect non-NULL)
**Requirement text**: Where `rect` is non-NULL, render only the specified region. Source and destination rect are identical.

Behavior contract:
- GIVEN: ScreenLayer called with rect {x:10, y:20, w:100, h:80}
- WHEN: canvas.copy called
- THEN: src_rect = dst_rect = {10, 20, 100, 80}

### REQ-SCR-070: Per-Call Temporary Texture
**Requirement text**: The backend shall create a temporary streaming texture per call, RGBX8888 format.

Behavior contract:
- GIVEN: ScreenLayer called
- WHEN: Implementation executes
- THEN: A streaming texture is created, used, and dropped within the call

### REQ-SCR-075: Use Surface Pitch for Upload
**Requirement text**: Use surface's `pitch` field as the row stride for texture upload.

Behavior contract:
- GIVEN: Surface with pitch (may include padding)
- WHEN: texture.update called
- THEN: Pitch is used as stride parameter

### REQ-SCR-170: Pixel Slice Construction
**Requirement text**: Construct byte slice using `from_raw_parts(pixels, pitch * h)`.

Behavior contract:
- GIVEN: Valid surface with non-null pixels, positive pitch
- WHEN: Pixel data accessed
- THEN: Slice is `pitch * SCREEN_HEIGHT` bytes from `pixels` pointer

### REQ-SCR-130: Texture Creation Failure
If `texture_creator.create_texture_streaming()` fails, the function shall return immediately without calling `canvas.copy()`.

### REQ-SCR-150: Rect Passthrough
When `rect` is non-NULL, pass the SDL_Rect values to `canvas.copy` without coordinate transformation (src_rect == dst_rect for unscaled path).

### REQ-FMT-020: Texture Uses RGBX8888
Streaming textures shall be created with `PixelFormatEnum::RGBX8888` to match surface format.

### REQ-ERR-065: No Copy on Update Failure
**Requirement text**: If `texture.update()` fails, do not call `canvas.copy()`.

Behavior contract:
- GIVEN: texture.update returns Err
- WHEN: Error handled
- THEN: Function returns immediately, no canvas.copy call

### REQ-NP-025: Texture Dropped Before Return
**Requirement text**: TextureCreator and Texture shall be dropped before FFI function returns.

Behavior contract:
- GIVEN: Texture created in function scope
- WHEN: Function returns
- THEN: Texture is dropped (Rust ownership ensures this)

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Replace `rust_gfx_screen` stub body with full unscaled implementation:
    1. Guards already in place from P06
    2. Rect validation (negative w/h check) — REQ-SCR-160
    3. Surface pixel validation — REQ-SCR-120
    4. Pitch/size validation — REQ-SCR-165
    5. Check if scaled path needed — delegate to helper if yes
    6. Create texture_creator + streaming texture — REQ-SCR-070
    7. Read pixel data via unsafe from_raw_parts — REQ-SCR-170
    8. Upload via texture.update — REQ-SCR-075
    9. Set blend mode + alpha mod — REQ-SCR-030/040
    10. Call canvas.copy with src=dst rect — REQ-SCR-060
  - Add `// SAFETY:` comments for all unsafe blocks — REQ-FFI-020
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P08`
  - marker: `@requirement REQ-SCR-010..170, REQ-SCR-130, REQ-SCR-150, REQ-FMT-020`

### Mutable state access pattern

All vtable function implementations must use a consistent mutable state
accessor pattern. The global `RustGraphicsState` is accessed via
`let state = unsafe { &mut *GFX_STATE.get() }` (or equivalent
`get_gfx_state_mut()` helper). This single mutable reference is obtained
once at the top of each FFI entry point and passed by `&mut` to any
internal helpers (e.g., `screen_layer_scaled`). Helpers must NOT
re-borrow the global — they receive the reference from their caller.
This pattern must be consistent across all vtable functions (preprocess,
screen, color, postprocess) and documented in each function's SAFETY
comment.

### Pseudocode traceability
- Uses pseudocode: component-003 lines 1–84 (full unscaled path)
- Uses pseudocode: component-003B lines 1–11 (convert_rect)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_screen` fully implemented (no todo!() or unimplemented!())
- [ ] `convert_c_rect` helper used for rect conversion
- [ ] `// SAFETY:` comments present on all unsafe blocks
- [ ] `@plan` and `@requirement` markers present
- [ ] Texture created and dropped within function scope
- [ ] canvas.copy uses src_rect == dst_rect for unscaled path
- [ ] All tests from P07 still pass

## Semantic Verification Checklist (Mandatory)
- [ ] ScreenLayer reads surface pixels (not just presenting nothing)
- [ ] Texture format is RGBX8888 matching surface format
- [ ] Blend mode set correctly for alpha=255 (None) vs alpha<255 (Blend)
- [ ] Alpha mod set when alpha < 255
- [ ] canvas.copy called with correct rect parameters
- [ ] Function returns early on texture.update failure (REQ-ERR-065)
- [ ] No per-frame logging (REQ-ERR-030)
- [ ] Surface pixels are NOT modified (REQ-SCR-080)
- [ ] Game should now show visible content (the main screen at minimum)

## Deferred Implementation Detection (Mandatory)

```bash
# Must find NO todo/fixme in rust_gfx_screen
sed -n '/fn rust_gfx_screen/,/^pub extern "C" fn\|^}/p' rust/src/graphics/ffi.rs | grep "TODO\|FIXME\|HACK\|todo!\|unimplemented!" && echo "FAIL: deferred code found" || echo "CLEAN"
```

## Success Criteria
- [ ] ScreenLayer fully implemented for unscaled path
- [ ] All P07 tests pass
- [ ] All cargo gates pass
- [ ] No placeholder patterns in rust_gfx_screen

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: texture lifetime issues, incorrect pixel format

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P08.md`

Contents:
- phase ID: P08
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- functions: rust_gfx_screen (unscaled path complete)
- tests: P07 tests verified passing
- verification: cargo fmt/clippy/test outputs
- semantic: surface pixels uploaded and rendered, game shows content
