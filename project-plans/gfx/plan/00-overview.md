# Plan: Full Rust GFX Drawing-Pipeline Port

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Generated: 2026-02-23
Total Phases: 30 (P00.5 through P26)
Requirements: REQ-INIT-*, REQ-UNINIT-*, REQ-SURF-*, REQ-PRE-*, REQ-SCR-*,
  REQ-SCALE-*, REQ-CLR-*, REQ-UTS-*, REQ-POST-*, REQ-SEQ-*, REQ-THR-*,
  REQ-ERR-*, REQ-INV-*, REQ-FMT-*, REQ-WIN-*, REQ-AUX-*, REQ-NP-*,
  REQ-ASM-*, REQ-FFI-*, REQ-DCQ-*, REQ-CANVAS-*, REQ-CMAP-*,
  REQ-GUARD-*, REQ-WIDGET-*, REQ-GFXLOAD-*, REQ-COMPAT-*

## Plan History

This plan supersedes `PLAN-20260223-GFX-VTABLE-FIX`, which was originally
scoped as a 5-function vtable fix for a black screen bug. The black screen
turned out to be caused by `USE_RUST_THREADS`, not the GFX layer. The
existing vtable code works. The scope is now expanded to cover the **full
GFX drawing-pipeline port**: eliminating all C drawing-pipeline code.
Loader files (gfxload.c, filegfx.c, resgfx.c, loaddisp.c) are explicitly
deferred — see "Deferred to Future Phase" section below.

### What exists and works:
- **ffi.rs vtable**: init, uninit, preprocess, postprocess, surface
  accessors, scaling — 17 `#[no_mangle]` exports, ALL WORKING
- **Rust drawing modules** (implemented but NOT wired via FFI):
  - `tfb_draw.rs` (3,405 lines) — line, rect, image, fontchar, scissor, copy
  - `dcqueue.rs` (1,362 lines) — draw command queue with 15 command types
  - `scaling.rs` (3,470 lines) — HQ2x, xBRZ, bilinear scalers
  - `cmap.rs` — colormap/fade support
  - `context.rs` — drawing context management
  - `drawable.rs` — drawable abstraction
  - `frame.rs` — frame/animation support
  - `pixmap.rs` — pixel buffer management

### What's missing (vtable gaps in ffi.rs):
- `rust_gfx_screen` — no-op, needs alpha compositing
- `rust_gfx_color` — no-op, needs fade overlay
- `rust_gfx_upload_transition_screen` — no-op
- `catch_unwind` on all `extern "C"` functions (REQ-FFI-030)
- Real fullscreen toggle (currently just flips a bool)

### What's missing entirely (no FFI bridge):
- DCQ has 0 `#[no_mangle]` exports — C can't call any Rust DCQ functions
- tfb_draw has 0 `#[no_mangle]` exports — C can't call any Rust draw functions
- Canvas↔SDL_Surface adapter doesn't exist
- Only 2 of 41 C files have `USE_RUST_GFX` guards (sdl_common.c, scalers.c)
- Widget system (widgets.c, 941 lines) has no Rust equivalent
- gfxload.c bridge doesn't exist

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. `unsafe` is explicitly approved for FFI boundary code in this feature
6. Phases P03–P14 modify `rust/src/graphics/ffi.rs` (vtable completion)
7. Phases P15–P26 modify multiple files across `rust/src/graphics/` and
   `sc2/src/libs/graphics/` (full port)

## Slices

The implementation is divided into these logical slices:

| Slice | Name | Description |
|---|---|---|
| A | Vtable Completion | Implement ScreenLayer, ColorLayer, UploadTransitionScreen, catch_unwind |
| B | Canvas FFI Bridge | Canvas↔SDL_Surface adapter, export draw ops to C |
| C | DCQ FFI Bridge | Export Rust DCQ functions via `#[no_mangle]`, wire C callers |
| D | Colormap FFI Bridge | Export Rust colormap/fade functions to C |
| E | C File Guards | Add `USE_RUST_GFX` guards to all 39 unguarded C files |
| F | Widget Bridge | Either port widgets or bridge them through Rust context |
| G | GfxLoad Bridge | Wire Rust resource loading to graphics frame/font loading |
| H | Integration + Verification | End-to-end verification, guard finalization, dual-path validation |

Slice A is covered by existing phases P03–P14.
Slice B (Canvas) is P15–P17, Slice C (DCQ) is P18–P20.
Slices D–H are covered by phases P21–P26.

> **Note on file naming**: Phase files `15-dcq-bridge-stub.md` through
> `17-dcq-bridge-impl.md` contain **Canvas** content (Slice B), and
> `18-canvas-bridge-stub.md` through `20-canvas-bridge-impl.md` contain
> **DCQ** content (Slice C). The filenames are historical artifacts from
> an earlier ordering. The phase content, not the filename, is
> authoritative.

**Rationale for B (Canvas) before C (DCQ)**: DCQ's `process_commands()` dispatches to
`tfb_draw.rs` functions that operate on `Canvas` objects. The
Canvas↔SDL_Surface adapter must exist before DCQ can be fully wired,
because DCQ flush needs `SurfaceCanvas` to execute draw commands against
the screen surfaces.

## Phase Map

| Phase | Type | Slice | Description |
|---|---|---|---|
| P00.5 | Preflight | — | Toolchain, deps, types, test infra |
| P01 | Analysis | — | Domain model, flow analysis |
| P01a | Verification | — | Analysis verification |
| P02 | Pseudocode | — | Algorithmic pseudocode |
| P02a | Verification | — | Pseudocode verification |
| P03 | Stub | A | Preprocess fix + Postprocess refactor stubs |
| P03a | Verification | A | Stub verification |
| P04 | TDD | A | Tests for preprocess fix + postprocess refactor |
| P04a | Verification | A | TDD verification |
| P05 | Impl | A | Implement preprocess fix + postprocess present-only |
| P05a | Verification | A | Implementation verification |
| P06 | Stub | A | ScreenLayer stub (compile-safe skeleton) |
| P06a | Verification | A | Stub verification |
| P07 | TDD | A | ScreenLayer tests |
| P07a | Verification | A | TDD verification |
| P08 | Impl | A | ScreenLayer unscaled implementation |
| P08a | Verification | A | Implementation verification |
| P09 | Stub | A | ColorLayer stub |
| P09a | Verification | A | Stub verification |
| P10 | TDD | A | ColorLayer tests |
| P10a | Verification | A | TDD verification |
| P11 | Impl | A | ColorLayer implementation |
| P11a | Verification | A | Implementation verification |
| P12 | Stub+TDD+Impl | A | Scaling integration (relocate from postprocess) |
| P12a | Verification | A | Scaling verification |
| P13 | Stub+TDD+Impl | A | Error handling hardening |
| P13a | Verification | A | Error handling verification |
| P14 | Integration | A | Vtable end-to-end verification |
| P14a | Verification | A | Vtable integration verification |
| P15 | Stub | B | Canvas FFI bridge — SurfaceCanvas adapter stubs |
| P15a | Verification | B | Canvas stub verification |
| P16 | TDD | B | Canvas FFI bridge — tests for draw ops through FFI |
| P16a | Verification | B | Canvas TDD verification |
| P17 | Impl | B | Canvas FFI bridge — full implementation |
| P17a | Verification | B | Canvas implementation verification |
| P18 | Stub | C | DCQ FFI bridge — export ~15 rust_dcq_* functions |
| P18a | Verification | C | DCQ stub verification |
| P19 | TDD | C | DCQ FFI bridge — tests for push/pop/flush through FFI |
| P19a | Verification | C | DCQ TDD verification |
| P20 | Impl | C | DCQ FFI bridge — full implementation |
| P20a | Verification | C | DCQ implementation verification |
| P21 | Stub+TDD+Impl | D+E | Colormap FFI + C file USE_RUST_GFX guards |
| P21a | Verification | D+E | Colormap + guards verification |
| P22 | Verification | D+E | All guards work, Rust builds without C graphics |
| P23 | Stub+TDD+Impl | F+G | Widget + GfxLoad bridge |
| P23a | Verification | F+G | Widget + GfxLoad verification |
| P24 | Integration | H | End-to-end testing, visual equivalence |
| P24a | Verification | H | Integration verification |
| P25 | Impl | H | Guard finalization — all drawing-pipeline C files guarded |
| P25a | Verification | H | C removal verification |
| P26 | Integration | H | Final verification — zero C graphics code compiled |

## C File Inventory (41 files)

### Already guarded (2 files):
- `sdl/sdl_common.c` — vtable wiring
- `sdl/scalers.c` — scaler selection

### Needs guards — Drawing layer (Slice B+C target, 8 files):
- `dcqueue.c` — draw command queue (replaced by dcqueue.rs)
- `tfb_draw.c` — draw primitives (replaced by tfb_draw.rs)
- `tfb_prim.c` — primitive helpers
- `sdl/canvas.c` — SDL canvas operations
- `sdl/primitives.c` — SDL primitive drawing
- `clipline.c` — line clipping
- `boxint.c` — box intersection
- `bbox.c` — bounding box

### Needs guards — Colormap/palette (Slice D target, 2 files):
- `cmap.c` — colormap management (replaced by cmap.rs)
- `sdl/palette.c` — SDL palette operations

### Needs guards — Scalers (already partially guarded, 10 files):
- `sdl/2xscalers.c` — 2x scaler implementations
- `sdl/2xscalers_mmx.c` — MMX scaler variants
- `sdl/2xscalers_sse.c` — SSE scaler variants
- `sdl/2xscalers_3dnow.c` — 3DNow! scaler variants
- `sdl/bilinear2x.c` — bilinear scaler
- `sdl/biadv2x.c` — advanced bilinear scaler
- `sdl/hq2x.c` — HQ2x scaler (replaced by scaling.rs)
- `sdl/nearest2x.c` — nearest-neighbor scaler
- `sdl/triscan2x.c` — triscan scaler
- `sdl/rotozoom.c` — rotation/zoom

### Deferred — compile in both modes (5 files, NOT guarded):
- `gfxload.c` — graphics resource loading (pure I/O, no drawing)
- `resgfx.c` — resource GFX management (handle management, no pixel ops)
- `filegfx.c` — file-based GFX loading (disk I/O utilities)
- `loaddisp.c` — display loading (splash/loading screen assets)
- `sdl/png2sdl.c` — PNG to SDL conversion (image decode, no drawing)

### Needs guards — Core abstractions (Slice B/C/F target, 8 files):
- `context.c` — drawing context (replaced by context.rs)
- `drawable.c` — drawable management (replaced by drawable.rs)
- `frame.c` — frame/animation (replaced by frame.rs)
- `pixmap.c` — pixmap management (replaced by pixmap.rs)
- `intersec.c` — intersection calculations
- `font.c` — font rendering
- `gfx_common.c` — common GFX utilities (replaced by gfx_common.rs)
- `widgets.c` — widget system (941 lines, Slice F)

### Needs guards — SDL backend (Slice A already covers, 6 files):
- `sdl/sdl2_pure.c` — SDL2 pure backend (vtable target)
- `sdl/sdl2_common.c` — SDL2 common utilities
- `sdl/sdl1_common.c` — SDL1 compatibility (may be dead code)
- `sdl/pure.c` — pure software backend
- `sdl/opengl.c` — OpenGL backend (may be dead code)
- `sdl/sdluio.c` — SDL UIO integration

## End-State Definition

The goal of this plan is **"zero C drawing-pipeline code compiling"** when
`USE_RUST_GFX=1` — NOT "zero C graphics code." The drawing pipeline
(DCQ, tfb_draw, canvas, scalers, colormaps, context, drawable, frame,
pixmap, gfx_common, primitives, clipping) is fully replaced by Rust.

Resource-loading files remain in C and compile in both modes. They read
data from disk and parse it into structures; they do not participate in
the drawing pipeline and are not safety-critical.

### Deferred to Future Phase (Loader Files)

These files stay **unguarded** and compile in both `USE_RUST_GFX=0` and
`USE_RUST_GFX=1` modes:

| File | Purpose | Why Deferred |
|---|---|---|
| `gfxload.c` | Graphics resource loading (`LoadGraphic`) | Reads `.ani` files into SDL_Surface; pure I/O, no drawing |
| `filegfx.c` | File-based GFX loading helpers | Disk I/O utilities for resource pipeline |
| `resgfx.c` | Resource GFX management (`_GetCelData`) | Resource handle management, no pixel ops |
| `loaddisp.c` | Display loading (`LoadDisplay`) | Loads splash/loading screen assets |
| `sdl/png2sdl.c` | PNG to SDL conversion | Image decode utility, no drawing |

These 5 files interact with the Rust pipeline only by producing
`SDL_Surface` or `FRAME` values that are then consumed by the drawing
layer. They can be ported in a future phase without affecting drawing
correctness.

## Symbol Migration Ledger

For each phase that adds `USE_RUST_GFX` guards, this ledger tracks which
C symbols are removed, which Rust symbols replace them, and which remain
unchanged.

### P21 — Drawing Layer + Colormap + Scalers + Core Abstractions Guards

**Symbols removed from C** (when `USE_RUST_GFX=1`):

| C File | Symbols Guarded | C-path owner (USE_RUST_GFX=0) |
|---|---|---|
| `dcqueue.c` | `TFB_DrawCommandQueue_Push`, `TFB_DrawCommandQueue_Pop`, `TFB_ProcessDrawCommand` | `dcqueue.c` (self) |
| `tfb_draw.c` | `TFB_DrawScreen_Line`, `TFB_DrawScreen_Rect`, `TFB_DrawScreen_FilledRect`, `TFB_DrawScreen_Image`, `TFB_DrawScreen_Copy`, `TFB_DrawScreen_SetPalette`, `TFB_DrawScreen_CopyToImage`, `TFB_DrawScreen_DeleteImage`, `TFB_DrawScreen_WaitForSignal`, `TFB_DrawScreen_ReinitVideo`, `TFB_FlushGraphics`, `TFB_BatchGraphics`, `TFB_UnbatchGraphics` | `tfb_draw.c` (self) |
| `tfb_prim.c` | `TFB_Prim_Line`, `TFB_Prim_Rect`, `TFB_Prim_FillRect`, `TFB_Prim_Stamp` | `tfb_prim.c` (self) |
| `sdl/canvas.c` | `TFB_DrawCanvas_Line`, `TFB_DrawCanvas_Rect`, `TFB_DrawCanvas_FilledRect`, `TFB_DrawCanvas_Image`, `TFB_DrawCanvas_FontChar`, `TFB_DrawCanvas_CopyRect`, `TFB_DrawCanvas_SetClipRect`, `TFB_DrawCanvas_GetExtent` | `sdl/canvas.c` (self) |
| `sdl/primitives.c` | `putpixel_8`, `putpixel_16`, `putpixel_24`, `putpixel_32`, `getpixel_8`, `getpixel_16`, `getpixel_24`, `getpixel_32` | `sdl/primitives.c` (self) |
| `clipline.c` | `TFB_DrawCanvas_ClipLine` | `clipline.c` (self) |
| `boxint.c` | `BoxIntersect`, `BoxUnion` | `boxint.c` (self) |
| `bbox.c` | `TFB_BBox_Reset`, `TFB_BBox_RegisterPoint`, `TFB_BBox_GetClipRect` | `bbox.c` (self) |
| `cmap.c` | `SetColorMap`, `GetColorMapAddress`, `XFormColorMap_step`, `FadeScreen`, `GetFadeAmount`, `TFB_SetColorMap`, `TFB_ColorMapFromIndex`, `init_colormap`, `uninit_colormap` | `cmap.c` (self) |
| `sdl/palette.c` | `TFB_SetPalette`, `TFB_GetPaletteColor` | `sdl/palette.c` (self) |
| `pixmap.c` | `TFB_DrawCanvas_ToScreenFormat`, `TFB_DrawCanvas_Initialize` | `pixmap.c` (self) |
| `intersec.c` | `DrawablesIntersect`, `frame_intersect` (calls `BoxIntersect` from `boxint.c`) | `intersec.c` (self) |
| `gfx_common.c` | `TFB_InitGraphics`, `TFB_UninitGraphics`, `TFB_ProcessEvents`, `TFB_GetScreenWidth`, `TFB_GetScreenHeight` | `gfx_common.c` (self) |
| `sdl/2xscalers.c` | `Scale_2xSaI`, `Scale_Super2xSaI`, `Scale_SuperEagle` | `sdl/2xscalers.c` (self) |
| `sdl/hq2x.c` | `Scale_HQ2X` | `sdl/hq2x.c` (self) |
| `sdl/bilinear2x.c` | `Scale_BilinearFilter` | `sdl/bilinear2x.c` (self) |
| `sdl/biadv2x.c` | `Scale_AdvMAME2x` | `sdl/biadv2x.c` (self) |
| `sdl/nearest2x.c` | `Scale_Nearest` | `sdl/nearest2x.c` (self) |
| `sdl/triscan2x.c` | `Scale_TriScan` | `sdl/triscan2x.c` (self) |
| `sdl/rotozoom.c` | `rotozoomSurface`, `zoomSurface` | `sdl/rotozoom.c` (self) |

> `context.c` and `drawable.c` are **deferred to P23** (widget-dependent).
> See REQ-GUARD-040 in P21 for the dependency constraint.

**Symbols added by Rust** (must match signatures):

> **Provider column note**: Files marked with **(new)** will be created
> during the listed phase. All new FFI modules must be added to
> `rust/src/graphics/mod.rs` and linked into the staticlib.

| Rust Export | Defined In | Replaces | C-path owner (USE_RUST_GFX=0) |
|---|---|---|---|
| `rust_dcq_push_drawline` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_Line` | `tfb_draw.c` |
| `rust_dcq_push_drawrect` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_Rect` | `tfb_draw.c` |
| `rust_dcq_push_fillrect` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_FilledRect` | `tfb_draw.c` |
| `rust_dcq_push_drawimage` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_Image` | `tfb_draw.c` |
| `rust_dcq_push_copy` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_Copy` | `tfb_draw.c` |
| `rust_dcq_push_setpalette` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_SetPalette` | `tfb_draw.c` |
| `rust_dcq_push_copytoimage` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_CopyToImage` | `tfb_draw.c` |
| `rust_dcq_push_deleteimage` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_DeleteImage` | `tfb_draw.c` |
| `rust_dcq_push_waitsignal` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_WaitForSignal` | `tfb_draw.c` |
| `rust_dcq_push_reinitvideo` | `dcq_ffi.rs` (new) | `TFB_DrawScreen_ReinitVideo` | `tfb_draw.c` |
| `rust_dcq_flush` | `dcq_ffi.rs` (new) | `TFB_FlushGraphics` | `dcqueue.c` |
| `rust_dcq_batch` / `rust_dcq_unbatch` | `dcq_ffi.rs` (new) | `TFB_BatchGraphics` / `TFB_UnbatchGraphics` | `dcqueue.c` |
| `rust_canvas_draw_line` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_Line` | `sdl/canvas.c` |
| `rust_canvas_draw_rect` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_Rect` | `sdl/canvas.c` |
| `rust_canvas_fill_rect` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_FilledRect` | `sdl/canvas.c` |
| `rust_canvas_draw_image` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_Image` | `sdl/canvas.c` |
| `rust_canvas_draw_fontchar` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_FontChar` | `sdl/canvas.c` |
| `rust_canvas_copy` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_CopyRect` | `sdl/canvas.c` |
| `rust_canvas_set_scissor` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_SetClipRect` | `sdl/canvas.c` |
| `rust_canvas_get_extent` | `canvas_ffi.rs` (new) | `TFB_DrawCanvas_GetExtent` | `sdl/canvas.c` |
| `rust_canvas_from_surface` | `canvas_ffi.rs` (new) | `SDL_CreateSurfaceCanvas` | `sdl/canvas.c` |
| `rust_canvas_destroy` | `canvas_ffi.rs` (new) | `SDL_DestroySurfaceCanvas` | `sdl/canvas.c` |
| `rust_cmap_set` | `cmap_ffi.rs` (new) | `SetColorMap` | `cmap.c` |
| `rust_cmap_get` | `cmap_ffi.rs` (new) | `GetColorMapAddress` | `cmap.c` |
| `rust_cmap_xform_step` | `cmap_ffi.rs` (new) | `XFormColorMap_step` | `cmap.c` |
| `rust_cmap_fade_screen` | `cmap_ffi.rs` (new) | `FadeScreen` | `cmap.c` |
| `rust_cmap_get_fade_amount` | `cmap_ffi.rs` (new) | `GetFadeAmount` | `cmap.c` |
| `rust_cmap_init` / `rust_cmap_uninit` | `cmap_ffi.rs` (new) | `init_colormap` / `uninit_colormap` | `cmap.c` |

**Verification**: After this phase, both builds must succeed:
```
USE_RUST_GFX=1: symbols from Rust column resolve
USE_RUST_GFX=0: symbols from C-path owner column resolve (unchanged)
```

**Symbols unchanged** (still provided by C in both modes):
- All symbols in `sdl_common.c` (vtable shim, unguarded)
- All symbols in loader files: `gfxload.c`, `filegfx.c`, `resgfx.c`, `loaddisp.c`

### P23 — Widget-Dependent File Guards

**Symbols removed from C** (when `USE_RUST_GFX=1`):

| C File | Symbols Guarded | C-path owner (USE_RUST_GFX=0) |
|---|---|---|
| `frame.c` | `ClearBackGround`, `ClearDrawable`, `DrawPoint`, `DrawRectangle`, `DrawFilledRectangle`, `DrawLine`, `DrawStamp`, `DrawFilledStamp`, `GetContextValidRect` | `frame.c` (self) |
| `font.c` | `SetContextFont`, `DestroyFont`, `font_DrawText`, `font_DrawTracedText`, `GetContextFontLeading`, `GetContextFontLeadingWidth`, `TextRect`, `_text_blt` | `font.c` (self) |
| `context.c` | `SetContext`, `CreateContextAux`, `DestroyContext`, `SetContextForeGroundColor`, `GetContextForeGroundColor`, `SetContextBackGroundColor`, `GetContextBackGroundColor`, `SetContextDrawMode`, `GetContextDrawMode`, `SetContextClipRect`, `GetContextClipRect`, `SetContextOrigin`, `SetContextFontEffect`, `FixContextFontEffect`, `CopyContextRect`, `GetContextName`, `GetFirstContext`, `GetNextContext` | `context.c` (self) |
| `drawable.c` | `SetContextFGFrame`, `GetContextFGFrame`, `request_drawable`, `CreateDisplay`, `AllocDrawable`, `CreateDrawable`, `DestroyDrawable`, `GetFrameRect`, `SetFrameHot`, `GetFrameHot`, `RotateFrame`, `SetFrameTransparentColor`, `GetFramePixel`, `makeMatchingFrame`, `CopyFrameRect`, `CloneFrame`, `RescaleFrame`, `ReadFramePixelColors`, `WriteFramePixelColors`, `ReadFramePixelIndexes`, `WriteFramePixelIndexes` | `drawable.c` (self) |
| `widgets.c` | All widget drawing functions (widgets depend on frame/font/context APIs above) | `widgets.c` (self) |

**Symbols added by Rust**:

| Rust Export | Defined In | Replaces | C-path owner (USE_RUST_GFX=0) |
|---|---|---|---|
| `rust_frame_clear_background` | `frame_ffi.rs` (new) | `ClearBackGround` | `frame.c` |
| `rust_frame_draw_point` | `frame_ffi.rs` (new) | `DrawPoint` | `frame.c` |
| `rust_frame_draw_rect` | `frame_ffi.rs` (new) | `DrawRectangle` | `frame.c` |
| `rust_frame_draw_filled_rect` | `frame_ffi.rs` (new) | `DrawFilledRectangle` | `frame.c` |
| `rust_frame_draw_line` | `frame_ffi.rs` (new) | `DrawLine` | `frame.c` |
| `rust_frame_draw_stamp` | `frame_ffi.rs` (new) | `DrawStamp` | `frame.c` |
| `rust_frame_draw_filled_stamp` | `frame_ffi.rs` (new) | `DrawFilledStamp` | `frame.c` |
| `rust_font_set_context_font` | `font_ffi.rs` (new) | `SetContextFont` | `font.c` |
| `rust_font_draw_text` | `font_ffi.rs` (new) | `font_DrawText` | `font.c` |
| `rust_font_draw_traced_text` | `font_ffi.rs` (new) | `font_DrawTracedText` | `font.c` |
| `rust_font_text_rect` | `font_ffi.rs` (new) | `TextRect` | `font.c` |
| `rust_context_set` | `context_ffi.rs` (new) | `SetContext` | `context.c` |
| `rust_context_create` | `context_ffi.rs` (new) | `CreateContextAux` | `context.c` |
| `rust_context_destroy` | `context_ffi.rs` (new) | `DestroyContext` | `context.c` |
| `rust_context_set_fg_color` | `context_ffi.rs` (new) | `SetContextForeGroundColor` | `context.c` |
| `rust_context_set_bg_color` | `context_ffi.rs` (new) | `SetContextBackGroundColor` | `context.c` |
| `rust_context_set_clip_rect` | `context_ffi.rs` (new) | `SetContextClipRect` | `context.c` |
| `rust_context_get_clip_rect` | `context_ffi.rs` (new) | `GetContextClipRect` | `context.c` |
| `rust_context_set_draw_mode` | `context_ffi.rs` (new) | `SetContextDrawMode` | `context.c` |
| `rust_context_set_origin` | `context_ffi.rs` (new) | `SetContextOrigin` | `context.c` |
| `rust_drawable_create` | `drawable_ffi.rs` (new) | `CreateDrawable` | `drawable.c` |
| `rust_drawable_destroy` | `drawable_ffi.rs` (new) | `DestroyDrawable` | `drawable.c` |
| `rust_drawable_create_display` | `drawable_ffi.rs` (new) | `CreateDisplay` | `drawable.c` |
| `rust_drawable_alloc` | `drawable_ffi.rs` (new) | `AllocDrawable` | `drawable.c` |
| `rust_frame_get_rect` | `drawable_ffi.rs` (new) | `GetFrameRect` | `drawable.c` |
| `rust_frame_set_hot` | `drawable_ffi.rs` (new) | `SetFrameHot` | `drawable.c` |
| `rust_frame_rescale` | `drawable_ffi.rs` (new) | `RescaleFrame` | `drawable.c` |
| `rust_frame_rotate` | `drawable_ffi.rs` (new) | `RotateFrame` | `drawable.c` |

> **Note**: Not every C function needs a 1:1 Rust FFI export. Some C
> functions are file-internal (`static`) or only called by other C
> functions within the same guarded file. The ledger lists public API
> functions that external callers depend on.

Verification: After this phase, both builds must succeed:
```
USE_RUST_GFX=1: symbols from Rust column resolve
USE_RUST_GFX=0: symbols from C column resolve (unchanged)
```

> **Loader files (gfxload.c, filegfx.c, resgfx.c, loaddisp.c) are OUT OF
> SCOPE for this plan.** They compile in both modes and are not tracked in
> the symbol ledger. `LoadGraphic`, `LoadFont`, and `TFB_LoadPNG` remain
> in C unconditionally.

### P25 — SDL Backend File Guards

**Symbols removed from C** (when `USE_RUST_GFX=1`):

| C File | Symbols Guarded | C-path owner (USE_RUST_GFX=0) |
|---|---|---|
| `sdl/sdl2_pure.c` | `TFB_Pure_ConfigureVideo`, `TFB_SDL2_Preprocess`, `TFB_SDL2_Postprocess`, `TFB_SDL2_ScreenLayer`, `TFB_SDL2_ColorLayer` | `sdl/sdl2_pure.c` (self) |
| `sdl/sdl2_common.c` | `TFB_SDL2_InitDisplay`, `TFB_SDL2_UninitDisplay` | `sdl/sdl2_common.c` (self) |
| `sdl/pure.c` | `TFB_Pure_InitGraphics` | `sdl/pure.c` (self) |
| `sdl/sdl1_common.c` | (dead code — SDL1 not supported) | `sdl/sdl1_common.c` (self) |
| `sdl/opengl.c` | `TFB_GL_ConfigureVideo`, `TFB_GL_ScreenLayer` | `sdl/opengl.c` (self) |
| `sdl/sdluio.c` | `TFB_SDL_UIO_*` functions | `sdl/sdluio.c` (self) |

> `sdl/png2sdl.c` is a loader file — it stays unguarded and compiles in
> both modes. See "Deferred to Future Phase" above.

Verification: After this phase, both builds must succeed:
  `USE_RUST_GFX=1`: symbols from Rust column resolve (via ffi.rs vtable)
  `USE_RUST_GFX=0`: symbols from C column resolve (unchanged)

**Verification step for each guard phase (P21, P23, P25)**:
```bash
# Build with USE_RUST_GFX=0 and verify no undefined symbols
cd sc2 && make clean && make USE_RUST_GFX=0 2>&1 | grep -c 'undefined'
# Expected: 0
```

## Canonical Phase Index

Due to a phase reorder (Canvas before DCQ), some filenames don't match
their content. This table is authoritative:

| Phase | Content | Slice | Filename (historical) |
|-------|---------|-------|-----------------------|
| P15 | Canvas Bridge — Stub | B | `15-dcq-bridge-stub.md` |
| P16 | Canvas Bridge — TDD | B | `16-dcq-bridge-tdd.md` |
| P17 | Canvas Bridge — Impl | B | `17-dcq-bridge-impl.md` |
| P18 | DCQ Bridge — Stub | C | `18-canvas-bridge-stub.md` |
| P19 | DCQ Bridge — TDD | C | `19-canvas-bridge-tdd.md` |
| P20 | DCQ Bridge — Impl | C | `20-canvas-bridge-impl.md` |

All other phase filenames match their content.

## Phase Completion Markers

Phase completion marker files (`.completed/PNN.md`) are **post-execution
artifacts** — they are created after each phase is successfully executed
and verified, not as part of the plan itself. The `.completed/` directory
starts empty (with a `.gitkeep`). As each phase is executed:

1. Execute all implementation tasks in the phase
2. Run all verification commands and pass all gates
3. Create `.completed/PNN.md` with the contents specified in the phase file
4. Update the Execution Tracker below (Status → [OK], Verified → [OK])

The absence of `.completed/PNN.md` files before execution begins is
expected and correct.

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00.5 | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
| P10   | ⬜     | ⬜       | ⬜                |       |
| P10a  | ⬜     | ⬜       | ⬜                |       |
| P11   | ⬜     | ⬜       | ⬜                |       |
| P11a  | ⬜     | ⬜       | ⬜                |       |
| P12   | ⬜     | ⬜       | ⬜                |       |
| P12a  | ⬜     | ⬜       | ⬜                |       |
| P13   | ⬜     | ⬜       | ⬜                |       |
| P13a  | ⬜     | ⬜       | ⬜                |       |
| P14   | ⬜     | ⬜       | ⬜                |       |
| P14a  | ⬜     | ⬜       | ⬜                |       |
| P15   | ⬜     | ⬜       | ⬜                |       |
| P15a  | ⬜     | ⬜       | ⬜                |       |
| P16   | ⬜     | ⬜       | ⬜                |       |
| P16a  | ⬜     | ⬜       | ⬜                |       |
| P17   | ⬜     | ⬜       | ⬜                |       |
| P17a  | ⬜     | ⬜       | ⬜                |       |
| P18   | ⬜     | ⬜       | ⬜                |       |
| P18a  | ⬜     | ⬜       | ⬜                |       |
| P19   | ⬜     | ⬜       | ⬜                |       |
| P19a  | ⬜     | ⬜       | ⬜                |       |
| P20   | ⬜     | ⬜       | ⬜                |       |
| P20a  | ⬜     | ⬜       | ⬜                |       |
| P21   | ⬜     | ⬜       | ⬜                |       |
| P21a  | ⬜     | ⬜       | ⬜                |       |
| P22   | ⬜     | ⬜       | ⬜                |       |
| P23   | ⬜     | ⬜       | ⬜                |       |
| P23a  | ⬜     | ⬜       | ⬜                |       |
| P24   | ⬜     | ⬜       | ⬜                |       |
| P24a  | ⬜     | ⬜       | ⬜                |       |
| P25   | ⬜     | ⬜       | ⬜                |       |
| P25a  | ⬜     | ⬜       | ⬜                |       |
| P26   | ⬜     | ⬜       | ⬜                |       |
