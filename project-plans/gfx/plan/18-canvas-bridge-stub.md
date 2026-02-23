# Phase 18: Canvas FFI Bridge — Stub

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P18`

## Prerequisites
- Required: Phase P17a (DCQ Implementation Verification) completed
- Expected: DCQ FFI bridge fully implemented, all tests passing
- Expected: Rust `tfb_draw.rs` (3,405 lines) has draw_line, draw_rect,
  fill_rect, draw_image, draw_fontchar, copy_canvas, scissor support

## Requirements Implemented (Expanded)

### REQ-CANVAS-010: SurfaceCanvas Adapter
**Requirement text**: The Rust GFX backend shall provide a `SurfaceCanvas`
adapter that wraps an `SDL_Surface` pointer and exposes it as a Rust
`Canvas` for drawing operations.

Behavior contract:
- GIVEN: A valid `*mut SDL_Surface` from C
- WHEN: `rust_canvas_from_surface(surface)` is called
- THEN: A `SurfaceCanvas` handle is returned that can be used for draw calls

### REQ-CANVAS-020: Draw Line Export
**Requirement text**: When `rust_canvas_draw_line` is called, the backend
shall draw a line on the target surface using Bresenham's algorithm from
`tfb_draw.rs`.

Behavior contract:
- GIVEN: A valid canvas handle and line coordinates
- WHEN: `rust_canvas_draw_line(canvas, x1, y1, x2, y2, color)` is called
- THEN: A line is drawn on the surface's pixel buffer

### REQ-CANVAS-030: Draw Rect Export
**Requirement text**: When `rust_canvas_draw_rect` / `rust_canvas_fill_rect`
are called, the backend shall draw rectangles on the target surface.

Behavior contract:
- GIVEN: A valid canvas handle and rect coordinates
- WHEN: `rust_canvas_fill_rect(canvas, x, y, w, h, color)` is called
- THEN: A filled rectangle is drawn on the surface's pixel buffer

### REQ-CANVAS-040: Draw Image Export
**Requirement text**: When `rust_canvas_draw_image` is called, the backend
shall blit image data onto the target surface.

Behavior contract:
- GIVEN: A valid canvas handle and image data
- WHEN: `rust_canvas_draw_image(canvas, image, x, y, flags)` is called
- THEN: The image is composited onto the surface's pixel buffer

### REQ-CANVAS-050: Draw Fontchar Export
**Requirement text**: When `rust_canvas_draw_fontchar` is called, the
backend shall render a font glyph onto the target surface with alpha
blending.

Behavior contract:
- GIVEN: A valid canvas handle and font character data
- WHEN: `rust_canvas_draw_fontchar(canvas, page, char_idx, x, y, color)` is called
- THEN: The glyph is rendered with proper alpha compositing

### REQ-CANVAS-060: Scissor Rect Support
**Requirement text**: When `rust_canvas_set_scissor` is called, subsequent
draw operations shall be clipped to the scissor rectangle.

Behavior contract:
- GIVEN: A canvas with a scissor rect set
- WHEN: Any draw operation is called
- THEN: Pixels outside the scissor rect are not modified

### REQ-CANVAS-070: Canvas Copy (Blit)
**Requirement text**: When `rust_canvas_copy` is called, the backend shall
copy pixels from one surface region to another.

Behavior contract:
- GIVEN: Source and destination surfaces
- WHEN: `rust_canvas_copy(dst, src, src_rect, dst_x, dst_y)` is called
- THEN: Pixels are copied with proper clipping

## Implementation Tasks

### C functions replaced by this phase

These C functions from `tfb_draw.c` and `sdl/canvas.c` will have Rust FFI equivalents:

| C Function | Rust FFI Export | Source Module |
|---|---|---|
| `TFB_DrawCanvas_Line` | `rust_canvas_draw_line` | tfb_draw.rs |
| `TFB_DrawCanvas_Rect` | `rust_canvas_draw_rect` | tfb_draw.rs |
| `TFB_DrawCanvas_FilledRect` | `rust_canvas_fill_rect` | tfb_draw.rs |
| `TFB_DrawCanvas_Image` | `rust_canvas_draw_image` | tfb_draw.rs |
| `TFB_DrawCanvas_FontChar` | `rust_canvas_draw_fontchar` | tfb_draw.rs |
| `TFB_DrawCanvas_CopyRect` | `rust_canvas_copy` | tfb_draw.rs |
| `TFB_DrawCanvas_SetClipRect` | `rust_canvas_set_scissor` | tfb_draw.rs |
| `TFB_DrawCanvas_GetExtent` | `rust_canvas_get_extent` | tfb_draw.rs |
| `SDL_CreateSurfaceCanvas` | `rust_canvas_from_surface` | canvas_ffi.rs |
| `SDL_DestroySurfaceCanvas` | `rust_canvas_destroy` | canvas_ffi.rs |

### Files to create
- `rust/src/graphics/canvas_ffi.rs` — New file for Canvas FFI exports
  - `SurfaceCanvas` struct wrapping `*mut SDL_Surface` + Rust `Canvas`
  - Handle-based API: C gets opaque `*mut SurfaceCanvas` handles
  - All `#[no_mangle] pub extern "C" fn rust_canvas_*` stubs
  - Each stub: `catch_unwind` wrapper, null check, returns safe default
  - Body: `todo!("Canvas FFI: <function_name>")` (allowed in stub phase)
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P18`
  - marker: `@requirement REQ-CANVAS-010..070, REQ-FFI-030`

### Files to modify
- `rust/src/graphics/mod.rs`
  - Add `pub mod canvas_ffi;`
- `sc2/src/libs/graphics/sdl/rust_gfx.h`
  - Add `rust_canvas_*` function declarations
  - Add `typedef struct SurfaceCanvas SurfaceCanvas;` opaque type

### Surface↔Canvas Adapter Design

The `SurfaceCanvas` adapter bridges C's `SDL_Surface` pixel buffers with
Rust's `Canvas` abstraction from `tfb_draw.rs`:

```
C SDL_Surface (pixel buffer) ──→ SurfaceCanvas adapter ──→ Rust Canvas
     ↑                                                        ↓
     └──────────────── pixels written back ◄──────────────────┘
```

Key design decisions:
- Canvas wraps raw pixel pointer from `SDL_Surface.pixels`
- Canvas dimensions from `SDL_Surface.w` / `SDL_Surface.h`
- Canvas pitch from `SDL_Surface.pitch`
- Canvas format derived from `SDL_Surface.format` masks
- **No copy**: Canvas operates directly on surface pixel memory
- **Lock/unlock**: May need `SDL_LockSurface` / `SDL_UnlockSurface` for
  hardware surfaces (software surfaces don't need locking)

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify all canvas exports are present
grep -c '#\[no_mangle\]' rust/src/graphics/canvas_ffi.rs
# Expected: >= 10

# Verify exports are linkable
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep rust_canvas_ | wc -l
# Expected: >= 10

# Verify catch_unwind on all exports
grep -c 'catch_unwind' rust/src/graphics/canvas_ffi.rs
# Expected: >= 10
```

## Structural Verification Checklist
- [ ] `canvas_ffi.rs` created with all ~10 `#[no_mangle]` exports
- [ ] `SurfaceCanvas` struct defined with surface pointer + canvas fields
- [ ] Each export has `catch_unwind` wrapper
- [ ] `mod.rs` updated with `pub mod canvas_ffi`
- [ ] `rust_gfx.h` updated with C declarations
- [ ] All stubs compile and link

## Semantic Verification Checklist (Mandatory)
- [ ] `SurfaceCanvas` operates on surface pixel memory without copying
- [ ] Handle-based API prevents C from accessing Rust internals
- [ ] Null surface pointer handled in `rust_canvas_from_surface`
- [ ] Canvas format derived correctly from SDL_Surface format masks
- [ ] All parameter types are C-compatible

## Deferred Implementation Detection (Mandatory)

```bash
# todo!() is ALLOWED in stub phase
grep -n "FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/canvas_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All ~10 canvas FFI stubs compile
- [ ] All exports are linkable
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] C header declarations added

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/canvas_ffi.rs rust/src/graphics/mod.rs`
- blocking issues: tfb_draw.rs Canvas API incompatible with SDL_Surface layout

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P18.md`

Contents:
- phase ID: P18
- timestamp
- files created: `rust/src/graphics/canvas_ffi.rs`
- files modified: `rust/src/graphics/mod.rs`, `sc2/src/libs/graphics/sdl/rust_gfx.h`
- total `#[no_mangle]` exports: count
- verification: cargo suite output
