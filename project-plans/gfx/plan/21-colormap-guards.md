# Phase 21: Colormap FFI + C File USE_RUST_GFX Guards

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P21`

## Prerequisites
- Required: Phase P20a (Canvas Implementation Verification) completed
- Expected: DCQ FFI bridge fully implemented (P17)
- Expected: Canvas FFI bridge fully implemented (P20)
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

### REQ-GUARD-040: USE_RUST_GFX Guards on Core Abstractions
**Requirement text**: C files for core abstractions (context.c, drawable.c,
frame.c, pixmap.c, intersec.c, gfx_common.c) shall be wrapped in guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 6 core abstraction C files are excluded from compilation

### REQ-GUARD-050: USE_RUST_GFX Guards on SDL Backend Files
**Requirement text**: SDL backend files (sdl2_pure.c, sdl2_common.c,
sdl1_common.c, pure.c, opengl.c, sdluio.c) shall be wrapped in guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 6 SDL backend C files are excluded from compilation

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

**Group 4 — Core abstractions (6 files):**
- `context.c` → replaced by `context.rs`
- `drawable.c` → replaced by `drawable.rs`
- `frame.c` → replaced by `frame.rs`
- `pixmap.c` → replaced by `pixmap.rs`
- `intersec.c` → replaced by Rust intersection logic
- `gfx_common.c` → replaced by `gfx_common.rs`

**Group 5 — SDL backend (6 files):**
- `sdl/sdl2_pure.c` → replaced by `ffi.rs` vtable
- `sdl/sdl2_common.c` → replaced by `ffi.rs` + `gfx_common.rs`
- `sdl/sdl1_common.c` → dead code (SDL1 not supported)
- `sdl/pure.c` → replaced by `ffi.rs`
- `sdl/opengl.c` → not yet ported (may remain for now)
- `sdl/sdluio.c` → replaced by Rust UIO

**NOT guarded in this phase** (deferred to P23/P25):
- `font.c` — complex, needs widget bridge first
- `gfxload.c` — resource loading, needs gfxload bridge first
- `resgfx.c` — resource management
- `filegfx.c` — file loading
- `loaddisp.c` — display loading
- `sdl/png2sdl.c` — PNG conversion
- `widgets.c` — widget system (941 lines)

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
# Expected: >= 34 (2 existing + 32 new)

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
- [ ] 32 C files guarded
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds without `USE_RUST_GFX` (backward compatibility)
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass

## Failure Recovery
- rollback: `git stash` (many files modified)
- blocking issues: guarded C files expose missing Rust FFI functions at link time

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P21.md`

Contents:
- phase ID: P21
- timestamp
- files created: `rust/src/graphics/cmap_ffi.rs`
- files modified: 32 C files (guards), `mod.rs`, `rust_gfx.h`
- C files guarded: list
- colormap exports: count
- verification: build with and without `USE_RUST_GFX`
