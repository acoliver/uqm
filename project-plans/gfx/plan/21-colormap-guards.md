# Phase 21: Colormap FFI + C File USE_RUST_GFX Guards

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P21`

## Prerequisites
- Required: Phase P20a (DCQ Implementation Verification) completed
- Expected: Canvas FFI bridge fully implemented (P17)
- Expected: DCQ FFI bridge fully implemented (P20)
- Expected: Rust `cmap.rs` has colormap/fade support

## Requirements Implemented (Expanded)

### REQ-CMAP-010: Colormap FFI Exports
**Requirement text**: The Rust GFX backend shall export colormap management
functions via `#[no_mangle]` FFI, replacing the C `cmap.c` implementation.

Behavior contract:
- GIVEN: C code needs to set/get colormaps for fade effects
- WHEN: `rust_cmap_set(index, colormap_data)` is called
- THEN: The colormap is stored in Rust-managed state

### REQ-CMAP-020: Fade Functions
**Requirement text**: The Rust backend shall export fade-in/fade-out
functions that compute frame-by-frame fade amounts.

Behavior contract:
- GIVEN: A fade-to-black is initiated
- WHEN: `rust_cmap_fade_step(fade_state)` is called each frame
- THEN: Returns the current fade_amount (0–511 per REQ-CLR-070)

### REQ-CMAP-030: Palette Operations
**Requirement text**: The Rust backend shall export palette set/get
functions for indexed color operations.

Behavior contract:
- GIVEN: C code sets a palette via `rust_cmap_set_palette`
- WHEN: The palette is applied
- THEN: Subsequent color lookups use the new palette

### REQ-GUARD-010: USE_RUST_GFX Guards on Drawing Layer
**Requirement text**: All C files in the drawing layer (dcqueue.c,
tfb_draw.c, tfb_prim.c, canvas.c, primitives.c, clipline.c, boxint.c,
bbox.c) shall be wrapped in `#ifndef USE_RUST_GFX` / `#endif` guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 8 drawing-layer C files are excluded from compilation

### REQ-GUARD-020: USE_RUST_GFX Guards on Colormap Layer
**Requirement text**: C files `cmap.c` and `sdl/palette.c` shall be
wrapped in `#ifndef USE_RUST_GFX` / `#endif` guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: `cmap.c` and `palette.c` are excluded from compilation

### REQ-GUARD-030: USE_RUST_GFX Guards on Scaler Layer
**Requirement text**: All scaler C files (2xscalers.c, 2xscalers_mmx.c,
2xscalers_sse.c, 2xscalers_3dnow.c, bilinear2x.c, biadv2x.c, hq2x.c,
nearest2x.c, triscan2x.c, rotozoom.c) shall be wrapped in guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 10 scaler C files are excluded from compilation

### REQ-GUARD-040: USE_RUST_GFX Guards on Core Abstractions (Non-Widget-Dependent Only)
**Requirement text**: C files for core abstractions that have NO widget
dependencies (pixmap.c, intersec.c, gfx_common.c) shall be wrapped in
guards in this phase. Widget-dependent files (frame.c, font.c, context.c,
drawable.c) are deferred to P23.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 3 non-widget-dependent core abstraction C files are excluded

** DEPENDENCY CONSTRAINT**: `frame.c`, `font.c`, `context.c`, and
`drawable.c` must NOT be guarded in this phase. `widgets.c` calls
`DrawBatch`, `DrawStamp` (from `frame.c`), `font_DrawText`,
`font_DrawTracedText` (from `font.c`), `SetContext`,
`SetContextForeGroundColor` (from `context.c`), and `GetFrameCount`
(from `drawable.c`). If those files are guarded before P23 provides
Rust replacements for the APIs widgets.c depends on, the widget system
will fail to link. These 4 files are guarded in P23 after the widget
bridge is complete.

### REQ-GUARD-050: USE_RUST_GFX Guards on SDL Backend Files
**Requirement text**: SDL backend files (sdl2_pure.c, sdl2_common.c,
sdl1_common.c, pure.c, sdluio.c) shall be wrapped in guards. `opengl.c`
is guarded only if the GL backend is fully replaced by Rust; otherwise
it remains unguarded as an optional backend.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 5 mandatory SDL backend files are excluded from compilation;
  `opengl.c` is excluded only if GL backend is replaced (otherwise kept)

## Guard Readiness Gate

Before guarding any file, verify:
1. Every C symbol in the file has a Rust replacement with matching signature
2. `cargo test` passes with the Rust replacement
3. A test exists that exercises the Rust replacement through FFI
4. Dual-path build: `USE_RUST_GFX=0` still compiles (C symbols unchanged)
5. `nm` check: no undefined symbols in either build mode

If any symbol lacks a Rust replacement, do NOT guard the file. Guard
readiness is checked per-file, not per-group — a group can be partially
guarded if some files are ready and others are not.

### Per-File Readiness Checklist

| C File | Key Symbols Requiring Rust Replacement | Rust Module |
|--------|----------------------------------------|-------------|
| `dcqueue.c` | `TFB_DrawCommandQueue_Push`, `TFB_DrawCommandQueue_Pop`, `TFB_FlushGraphics` | `dcq_ffi.rs` |
| `tfb_draw.c` | `TFB_DrawScreen_Line`, `TFB_DrawScreen_Rect`, `TFB_DrawScreen_Image`, `TFB_DrawScreen_Copy`, `TFB_DrawScreen_FontChar` | `dcq_ffi.rs` |
| `tfb_prim.c` | `TFB_Prim_Line`, `TFB_Prim_Rect`, `TFB_Prim_FillRect`, `TFB_Prim_Stamp` | `canvas_ffi.rs` |
| `sdl/canvas.c` | `TFB_DrawCanvas_Line`, `TFB_DrawCanvas_Rect`, `TFB_DrawCanvas_Image`, `TFB_DrawCanvas_FontChar`, `TFB_DrawCanvas_CopyRect` | `canvas_ffi.rs` |
| `sdl/primitives.c` | `putpixel_32`, `getpixel_32` (only 32bpp needed for RGBX8888) | `canvas_ffi.rs` |
| `clipline.c` | `TFB_DrawCanvas_ClipLine` | `canvas_ffi.rs` |
| `boxint.c` | `BoxIntersect`, `BoxUnion` | `canvas_ffi.rs` |
| `bbox.c` | `TFB_BBox_Reset`, `TFB_BBox_RegisterPoint`, `TFB_BBox_GetClipRect` | `canvas_ffi.rs` |
| `cmap.c` | `SetColorMap`, `FadeScreen`, `GetFadeAmount`, `init_colormap`, `uninit_colormap` | `cmap_ffi.rs` |
| `sdl/palette.c` | `TFB_SetPalette`, `TFB_GetPaletteColor` | `cmap_ffi.rs` |
| `pixmap.c` | `TFB_DrawCanvas_ToScreenFormat`, `TFB_DrawCanvas_Initialize` | `canvas_ffi.rs` |
| `intersec.c` | `DrawablesIntersect` | `canvas_ffi.rs` |
| `gfx_common.c` | `TFB_InitGraphics`, `TFB_UninitGraphics`, `TFB_ProcessEvents` | `ffi.rs` (existing) |
| All scaler `.c` files | `Scale_HQ2X`, `Scale_BilinearFilter`, `Scale_Nearest`, etc. | `ffi.rs` (existing) |

## Implementation Tasks

### Colormap FFI exports (~8 functions)

| C Function (`cmap.c`) | Rust FFI Export | Purpose |
|---|---|---|
| `SetColorMap` | `rust_cmap_set` | Set active colormap |
| `GetColorMapAddress` | `rust_cmap_get` | Get colormap data |
| `XFormColorMap_step` | `rust_cmap_xform_step` | Color transform step |
| `FadeScreen` | `rust_cmap_fade_screen` | Initiate fade |
| `GetFadeAmount` | `rust_cmap_get_fade_amount` | Query current fade level |
| `TFB_SetColorMap` | `rust_cmap_tfb_set` | Low-level colormap set |
| `TFB_ColorMapFromIndex` | `rust_cmap_from_index` | Index → colormap |
| `init_colormap` / `uninit_colormap` | `rust_cmap_init` / `rust_cmap_uninit` | Lifecycle |

### C files to add guards (39 files across 5 groups)

Guard format for each file:
```c
/* At top of file, after includes: */
#ifdef USE_RUST_GFX
/* This file is replaced by Rust implementation.
 * See rust/src/graphics/ for the Rust equivalent. */
#else
/* ... existing file content ... */
#endif /* !USE_RUST_GFX */
```

**Group 1 — Drawing layer (8 files):**
- `dcqueue.c` → replaced by `dcqueue.rs` + `dcq_ffi.rs`
- `tfb_draw.c` → replaced by `tfb_draw.rs` + `canvas_ffi.rs`
- `tfb_prim.c` → replaced by `tfb_draw.rs` primitives
- `sdl/canvas.c` → replaced by `canvas_ffi.rs`
- `sdl/primitives.c` → replaced by `tfb_draw.rs`
- `clipline.c` → replaced by `tfb_draw.rs` line clipping
- `boxint.c` → replaced by `tfb_draw.rs` intersection
- `bbox.c` → replaced by `tfb_draw.rs` bounding box

**Group 2 — Colormap/palette (2 files):**
- `cmap.c` → replaced by `cmap.rs` + colormap FFI
- `sdl/palette.c` → replaced by `cmap.rs` palette

**Group 3 — Scalers (10 files):**
- `sdl/2xscalers.c`, `sdl/2xscalers_mmx.c`, `sdl/2xscalers_sse.c`,
  `sdl/2xscalers_3dnow.c`, `sdl/bilinear2x.c`, `sdl/biadv2x.c`,
  `sdl/hq2x.c`, `sdl/nearest2x.c`, `sdl/triscan2x.c`, `sdl/rotozoom.c`
- All replaced by `scaling.rs`

**Group 4 — Core abstractions, non-widget-dependent only (3 files):**
- `pixmap.c` → replaced by `pixmap.rs`
- `intersec.c` → replaced by Rust intersection logic
- `gfx_common.c` → replaced by `gfx_common.rs`

**NOT guarded in this phase — widget-dependent (deferred to P23):**
- `frame.c` — widgets.c calls `DrawBatch`, `DrawStamp`, `GetFrameBounds`
- `font.c` — widgets.c calls `font_DrawText`, `font_DrawTracedText`, `GetCharExtent`
- `context.c` — widgets.c calls `SetContext`, `SetContextForeGroundColor`, `SetContextBackGroundColor`, `SetContextClipRect`
- `drawable.c` — widgets.c calls `GetFrameCount`, `CreateDrawable`

**Group 5 — SDL backend (6 files):**
- `sdl/sdl2_pure.c` → replaced by `ffi.rs` vtable
- `sdl/sdl2_common.c` → replaced by `ffi.rs` + `gfx_common.rs`
- `sdl/sdl1_common.c` → dead code (SDL1 not supported)
- `sdl/pure.c` → replaced by `ffi.rs`
- `sdl/opengl.c` → not yet ported (may remain for now)
- `sdl/sdluio.c` → replaced by Rust UIO

**NOT guarded in this phase — widget-dependent (deferred to P23):**
- `frame.c` — widgets.c depends on `DrawBatch`, `DrawStamp`, `GetFrameBounds`
- `font.c` — widgets.c depends on `font_DrawText`, `font_DrawTracedText`
- `context.c` — widgets.c depends on `SetContext`, `SetContextForeGroundColor`
- `drawable.c` — widgets.c depends on `GetFrameCount`, `CreateDrawable`
- `widgets.c` — widget system (941 lines), guarded after bridge provides replacements

**NOT guarded in this phase — loader files (deferred indefinitely):**
- `gfxload.c` — resource loading, pure I/O, no drawing
- `resgfx.c` — resource management
- `filegfx.c` — file loading
- `loaddisp.c` — display loading
- `sdl/png2sdl.c` — PNG conversion

Loader files compile in both modes — see `00-overview.md` "Deferred to
Future Phase" section.

### Files to create
- `rust/src/graphics/cmap_ffi.rs` — Colormap FFI exports
  - ~8 `#[no_mangle]` functions
  - `catch_unwind` on all exports
  - Wire to `cmap.rs` API
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P21`
  - marker: `@requirement REQ-CMAP-010..030, REQ-FFI-030`

### Files to modify
- `rust/src/graphics/mod.rs` — Add `pub mod cmap_ffi;`
- `sc2/src/libs/graphics/sdl/rust_gfx.h` — Add `rust_cmap_*` declarations
- 32 C files — Add `USE_RUST_GFX` guards (groups 1–5 above)

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify colormap exports
grep -c '#\[no_mangle\]' rust/src/graphics/cmap_ffi.rs
# Expected: >= 8

# Verify guard count in C files
grep -rl 'USE_RUST_GFX' sc2/src/libs/graphics/ | wc -l
# Expected: >= 31 (2 existing + 29 new; widget-dependent files deferred to P23)

# Verify widget-dependent files are NOT guarded yet
for f in frame.c font.c context.c drawable.c; do
  if grep -q 'USE_RUST_GFX' "sc2/src/libs/graphics/$f"; then
    echo "FAIL: $f guarded too early — widgets.c depends on it"
  else
    echo "OK: $f not yet guarded (correct)"
  fi
done

# Build with USE_RUST_GFX to verify guarded files are excluded
cd sc2 && make USE_RUST_GFX=1 2>&1 | head -50
# Should compile without the guarded C files

# Build WITHOUT USE_RUST_GFX to verify C path still works
cd sc2 && make 2>&1 | head -50
# Should compile normally with all C files
```

## Structural Verification Checklist
- [ ] `cmap_ffi.rs` created with ~8 `#[no_mangle]` exports
- [ ] All 32 C files have `USE_RUST_GFX` guards
- [ ] Guard format is consistent (`#ifdef USE_RUST_GFX` / `#else` / `#endif`)
- [ ] Each guarded file has a comment pointing to Rust replacement
- [ ] `mod.rs` updated with `pub mod cmap_ffi`
- [ ] `rust_gfx.h` updated with colormap declarations
- [ ] Tests still pass with and without `USE_RUST_GFX`

## Semantic Verification Checklist (Mandatory)
- [ ] Colormap FFI functions match C function signatures
- [ ] Guards are at file level (entire file content guarded, not partial)
- [ ] No header files are guarded (only .c files)
- [ ] Guard placement is after `#include` directives (to avoid missing types)
- [ ] `sdl_common.c` and `scalers.c` existing guards are not duplicated
- [ ] `opengl.c` guard noted as optional (may remain unguarded if GL still needed)
- [ ] Build system respects guards correctly

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "todo!\|TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/cmap_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] Colormap FFI exports compile and link
- [ ] 29 C files guarded (widget-dependent files deferred to P23)
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds with `USE_RUST_GFX=0` — no undefined symbols
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass

## Dual-Path ABI Verification (Mandatory)
```bash
# Build with USE_RUST_GFX=0 and verify no undefined symbols
cd sc2 && make clean && make USE_RUST_GFX=0 2>&1 | grep -c 'undefined'
# Expected: 0
```

## Failure Recovery
- rollback: `git stash` (many files modified)
- blocking issues: guarded C files expose missing Rust FFI functions at link time

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P21.md`

Contents:
- phase ID: P21
- timestamp
- files created: `rust/src/graphics/cmap_ffi.rs`
- files modified: 29 C files (guards), `mod.rs`, `rust_gfx.h`
- C files guarded: list (29 files; frame.c, font.c, context.c, drawable.c deferred to P23)
- colormap exports: count
- verification: build with and without `USE_RUST_GFX`
