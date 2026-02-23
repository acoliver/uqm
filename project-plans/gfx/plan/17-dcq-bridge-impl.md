> **NOTE**: This file's name is a historical artifact from a phase reorder.
> Canonical: Phase P17 = Canvas Bridge — Impl (Slice B)


# Phase 17: Canvas FFI Bridge — Implementation

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P17`

## Prerequisites
- Required: Phase P16a (Canvas TDD Verification) completed
- Expected: All canvas tests written, stubs compile
- Expected: `tfb_draw.rs` provides draw_line, draw_rect, fill_rect,
  draw_image, draw_fontchar, copy_canvas, scissor support

## Requirements Implemented (Expanded)

### REQ-CANVAS-010: SurfaceCanvas Adapter (Full Implementation)
Implementation:
- `rust_canvas_from_surface(surface: *mut SDL_Surface) -> *mut SurfaceCanvas`
  - Null check on surface pointer
  - Read `w`, `h`, `pitch`, `pixels`, `format` from SDL_Surface
  - Derive `CanvasFormat` from surface pixel format masks
  - Create `Canvas` wrapping the raw pixel pointer
  - Box and return raw pointer as opaque handle
- `rust_canvas_destroy(canvas: *mut SurfaceCanvas)`
  - Null check, `Box::from_raw`, drop

### REQ-CANVAS-020–070: All Draw Operations (Full Implementation)
Each draw function:
1. Null-check the canvas handle
2. Dereference to get `&mut SurfaceCanvas`
3. Call the corresponding `tfb_draw.rs` function on the inner `Canvas`
4. Canvas writes directly to `SDL_Surface.pixels` memory
5. Return 0 on success, -1 on error

### Pixel Memory Safety

The `SurfaceCanvas` adapter accesses `SDL_Surface.pixels` directly:

```
rust_canvas_draw_line(handle, x1, y1, x2, y2, color)
  → handle.canvas.draw_line(x1, y1, x2, y2, color)
    → writes to surface.pixels[y * pitch + x * bpp]
```

Safety invariants:
- Surface pointer is valid (created by SDL, not freed)
- `pixels` pointer is non-null and properly aligned
- Surface is not locked by another thread (single-threaded per REQ-THR-010)
- Canvas bounds checking prevents out-of-buffer writes
- `// SAFETY:` comments document each invariant

See `technical.md` §8.7 for the full SurfaceCanvas adapter contract
(lifetime, locking, aliasing, pitch, format, thread affinity).

### C functions fully replaced

| C Function | Rust Implementation | Notes |
|---|---|---|
| `TFB_DrawCanvas_Line` | `rust_canvas_draw_line` → `Canvas::draw_line` | Bresenham's algorithm |
| `TFB_DrawCanvas_Rect` | `rust_canvas_draw_rect` → `Canvas::draw_rect` | 4 lines |
| `TFB_DrawCanvas_FilledRect` | `rust_canvas_fill_rect` → `Canvas::fill_rect` | Row-by-row fill |
| `TFB_DrawCanvas_Image` | `rust_canvas_draw_image` → `Canvas::draw_image` | Hotspot + blit |
| `TFB_DrawCanvas_FontChar` | `rust_canvas_draw_fontchar` → `Canvas::draw_fontchar` | Alpha compositing |
| `TFB_DrawCanvas_CopyRect` | `rust_canvas_copy` → `Canvas::copy_canvas` | Region blit |
| `TFB_DrawCanvas_SetClipRect` | `rust_canvas_set_scissor` → `Canvas::set_scissor` | Clip rect |
| `TFB_DrawCanvas_GetExtent` | `rust_canvas_get_extent` → `Canvas::width/height` | Dimensions |
| `SDL_CreateSurfaceCanvas` | `rust_canvas_from_surface` | Adapter creation |
| `SDL_DestroySurfaceCanvas` | `rust_canvas_destroy` | Handle cleanup |

## Implementation Tasks

### Files to modify
- `rust/src/graphics/canvas_ffi.rs`
  - Replace all `todo!()` stubs with full implementations
  - Implement `SurfaceCanvas::from_surface()` adapter
  - Wire each draw export to `tfb_draw.rs` Canvas methods
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P17`
  - marker: `@requirement REQ-CANVAS-010..070, REQ-FFI-030`

### Integration with DCQ

After this phase, the DCQ bridge (P18–P20) can dispatch draw commands
through the canvas FFI:

```
DCQ flush → DrawLine command → rust_canvas_draw_line → Canvas::draw_line → pixels
```

This is why Canvas must be implemented before DCQ — DCQ flush needs
`SurfaceCanvas` to execute draw commands against the screen surfaces.

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# All canvas tests must pass now
cd rust && cargo test --lib -- canvas_ffi::tests --nocapture
# Expected: all >= 20 tests pass

# Verify no deferred patterns
grep -n "todo!\|TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/canvas_ffi.rs && echo "FAIL" || echo "CLEAN"

# Verify all exports still linkable
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep rust_canvas_ | wc -l
# Expected: >= 10
```

## Structural Verification Checklist
- [ ] All `todo!()` stubs replaced with implementations
- [ ] `SurfaceCanvas` adapter correctly wraps SDL_Surface
- [ ] All ~10 FFI functions have full implementations
- [ ] `catch_unwind` on every `extern "C" fn`
- [ ] Handle-based API prevents dangling references
- [ ] No deferred patterns

## Semantic Verification Checklist (Mandatory)
- [ ] `SurfaceCanvas::from_surface` reads correct fields from SDL_Surface
- [ ] Canvas pixel writes go to `SDL_Surface.pixels` memory (no copy)
- [ ] draw_line uses Bresenham's — verified by pixel-checking test
- [ ] fill_rect fills correct pixel region — verified by pixel-checking test
- [ ] draw_fontchar alpha blending matches C reference — verified by test
- [ ] Scissor clipping restricts all draw operations — verified by test
- [ ] Canvas copy handles overlapping regions — verified by test
- [ ] Bounds checking prevents buffer overflows — verified by clipping tests

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "todo!\|TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/canvas_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All canvas FFI functions fully implemented
- [ ] All >= 20 tests pass
- [ ] Pixel-level verification of draw operations
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] No deferred patterns

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/canvas_ffi.rs`
- blocking issues: tfb_draw.rs Canvas API doesn't match SDL_Surface layout

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P17.md`

Contents:
- phase ID: P17
- timestamp
- files modified: `rust/src/graphics/canvas_ffi.rs`
- total tests: count (all passing)
- total `#[no_mangle]` canvas exports: count
- verification: cargo suite output
- semantic: pixel-level draw verification confirmed
