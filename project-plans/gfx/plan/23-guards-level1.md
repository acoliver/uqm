# Phase 23: C File Guards — Level 1-2 (Canvas, DCQ, TFB_Draw, CMap, SDL Backend)

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P23`

## Prerequisites
- Required: Phase P22a (Level 0 Guards Verification) completed
- Expected: 17 C files guarded (2 pre-existing + 15 Level 0)
- Expected: Canvas FFI bridge (P17), DCQ FFI bridge (P20), Colormap FFI (P21) all implemented

## Requirements Implemented (Expanded)

### REQ-GUARD-010: USE_RUST_GFX Guards on Drawing Layer
**Requirement text**: Drawing layer C files (dcqueue.c, tfb_draw.c,
tfb_prim.c, canvas.c) shall be wrapped in `#ifndef USE_RUST_GFX` guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The drawing-layer C files are excluded from compilation

### REQ-GUARD-020: USE_RUST_GFX Guards on Colormap Layer
**Requirement text**: C files `cmap.c` and `sdl/palette.c` shall be
wrapped in `#ifndef USE_RUST_GFX` / `#endif` guards.

### REQ-GUARD-040: USE_RUST_GFX Guards on Core Abstractions (Non-Widget-Dependent Only)
**Requirement text**: C files for core abstractions that have NO widget
dependencies (pixmap.c, gfx_common.c) shall be wrapped in guards in this
phase. Widget-dependent files (frame.c, font.c, context.c, drawable.c)
are deferred to P24.

**DEPENDENCY CONSTRAINT**: `frame.c`, `font.c`, `context.c`, and
`drawable.c` must NOT be guarded in this phase. `widgets.c` calls APIs
from those files. They are guarded in P24 after the widget bridge.

### REQ-GUARD-050: USE_RUST_GFX Guards on SDL Backend Files
**Requirement text**: SDL backend files (sdl2_pure.c, sdl2_common.c,
sdl1_common.c, pure.c, sdluio.c) shall be wrapped in guards.

## Guard Readiness Gate

Before guarding any file, verify:
1. Every C symbol in the file has a Rust replacement with matching signature
2. `cargo test` passes with the Rust replacement
3. A test exists that exercises the Rust replacement through FFI
4. Dual-path build: `USE_RUST_GFX=0` still compiles
5. `nm` check: no undefined symbols in either build mode

## C Files to Guard in This Phase (~14 files)

Guard format (same as P22):
```c
#ifdef USE_RUST_GFX
/* This file is replaced by Rust implementation. */
#else
/* ... existing file content ... */
#endif /* !USE_RUST_GFX */
```

**Group 1 — Drawing layer (4 files):**
- `dcqueue.c` → replaced by `dcqueue.rs` + `dcq_ffi.rs`
- `tfb_draw.c` → replaced by `tfb_draw.rs` + `canvas_ffi.rs`
- `tfb_prim.c` → replaced by `tfb_draw.rs` primitives
- `sdl/canvas.c` → replaced by `canvas_ffi.rs`

**Group 2 — Colormap/palette (2 files):**
- `cmap.c` → replaced by `cmap.rs` + `cmap_ffi.rs`
- `sdl/palette.c` → replaced by `cmap.rs` palette

**Group 3 — Core abstractions, non-widget-dependent (2 files):**
- `pixmap.c` → replaced by `pixmap.rs`
- `gfx_common.c` → replaced by `gfx_common.rs`

**Group 4 — SDL backend (6 files):**
- `sdl/sdl2_pure.c` → replaced by `ffi.rs` vtable
- `sdl/sdl2_common.c` → replaced by `ffi.rs` + `gfx_common.rs`
- `sdl/sdl1_common.c` → dead code (SDL1 not supported)
- `sdl/pure.c` → replaced by `ffi.rs`
- `sdl/opengl.c` → not yet ported (may remain for now)
- `sdl/sdluio.c` → replaced by Rust UIO

**NOT guarded in this phase — widget-dependent (deferred to P24):**
- `frame.c`, `font.c`, `context.c`, `drawable.c`, `widgets.c`

**NOT guarded — loader files (deferred indefinitely):**
- `gfxload.c`, `resgfx.c`, `filegfx.c`, `loaddisp.c`, `sdl/png2sdl.c`

### Per-File Readiness Checklist

| C File | Key Symbols Requiring Rust Replacement | Rust Module |
|--------|----------------------------------------|-------------|
| `dcqueue.c` | `TFB_DrawCommandQueue_Push`, `TFB_DrawCommandQueue_Pop`, `TFB_FlushGraphics` | `dcq_ffi.rs` |
| `tfb_draw.c` | `TFB_DrawScreen_Line`, `TFB_DrawScreen_Rect`, `TFB_DrawScreen_Image` | `dcq_ffi.rs` |
| `tfb_prim.c` | `TFB_Prim_Line`, `TFB_Prim_Rect`, `TFB_Prim_FillRect`, `TFB_Prim_Stamp` | `canvas_ffi.rs` |
| `sdl/canvas.c` | `TFB_DrawCanvas_Line`, `TFB_DrawCanvas_Rect`, `TFB_DrawCanvas_Image` | `canvas_ffi.rs` |
| `cmap.c` | `SetColorMap`, `FadeScreen`, `GetFadeAmount`, `init_colormap` | `cmap_ffi.rs` |
| `sdl/palette.c` | `TFB_SetPalette`, `TFB_GetPaletteColor` | `cmap_ffi.rs` |
| `pixmap.c` | `TFB_DrawCanvas_ToScreenFormat`, `TFB_DrawCanvas_Initialize` | `canvas_ffi.rs` |
| `gfx_common.c` | `TFB_InitGraphics`, `TFB_UninitGraphics`, `TFB_ProcessEvents` | `ffi.rs` |
| SDL backend files | Various SDL2 backend functions | `ffi.rs` |

## Files to Modify
- ~14 C files — Add `USE_RUST_GFX` guards (groups 1–4 above)

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify guard count in C files
grep -rl 'USE_RUST_GFX' sc2/src/libs/graphics/ | wc -l
# Expected: >= 31 (17 from P22 + 14 new)

# Verify widget-dependent files are NOT guarded yet
for f in frame.c font.c context.c drawable.c widgets.c; do
  if grep -q 'USE_RUST_GFX' "sc2/src/libs/graphics/$f"; then
    echo "FAIL: $f guarded too early — widgets.c depends on it"
  else
    echo "OK: $f not yet guarded (correct)"
  fi
done

# Build with USE_RUST_GFX
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | head -50

# Build WITHOUT USE_RUST_GFX
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | head -50
```

## Structural Verification Checklist
- [ ] All ~14 Level 1-2 C files have `USE_RUST_GFX` guards
- [ ] Guard format is consistent
- [ ] Widget-dependent files NOT guarded (frame.c, font.c, context.c, drawable.c, widgets.c)
- [ ] Loader files NOT guarded (gfxload.c, etc.)
- [ ] No header files are guarded (only .c files)
- [ ] `sdl_common.c` and `scalers.c` existing guards not duplicated

## Semantic Verification Checklist (Mandatory)
- [ ] Guards are at file level (entire file content guarded)
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds with `USE_RUST_GFX=0`
- [ ] `opengl.c` guard noted as optional

## Dual-Path ABI Verification (Mandatory)
```bash
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | grep -c 'undefined'
# Expected: 0
```

## Success Criteria
- [ ] ~14 Level 1-2 C files guarded (total ~31)
- [ ] Both build paths compile without errors
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: guarded C files expose missing Rust FFI functions at link time

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P23.md`

Contents:
- phase ID: P23
- timestamp
- files modified: ~14 C files (Level 1-2 guards)
- guard count: ~31 total (17 from P22 + 14 new)
- widget-dependent files deferred to P24
- verification: build with and without `USE_RUST_GFX`
