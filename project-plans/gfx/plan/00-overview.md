# Plan: Full Rust GFX Port — Eliminate C Graphics Code

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
GFX port**: eliminating all C graphics code.

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
| B | DCQ FFI Bridge | Export Rust DCQ functions via `#[no_mangle]`, wire C callers |
| C | Canvas FFI Bridge | Canvas↔SDL_Surface adapter, export draw ops to C |
| D | Colormap FFI Bridge | Export Rust colormap/fade functions to C |
| E | C File Guards | Add `USE_RUST_GFX` guards to all 39 unguarded C files |
| F | Widget Bridge | Either port widgets or bridge them through Rust context |
| G | GfxLoad Bridge | Wire Rust resource loading to graphics frame/font loading |
| H | Integration + Cleanup | End-to-end verification, remove C fallback code |

Slices A is covered by existing phases P03–P14.
Slices B–H are covered by new phases P15–P26.

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
| P15 | Stub | B | DCQ FFI bridge — export ~15 rust_dcq_* functions |
| P15a | Verification | B | DCQ stub verification |
| P16 | TDD | B | DCQ FFI bridge — tests for push/pop/flush through FFI |
| P16a | Verification | B | DCQ TDD verification |
| P17 | Impl | B | DCQ FFI bridge — full implementation |
| P17a | Verification | B | DCQ implementation verification |
| P18 | Stub | C | Canvas FFI bridge — SurfaceCanvas adapter stubs |
| P18a | Verification | C | Canvas stub verification |
| P19 | TDD | C | Canvas FFI bridge — tests for draw ops through FFI |
| P19a | Verification | C | Canvas TDD verification |
| P20 | Impl | C | Canvas FFI bridge — full implementation |
| P20a | Verification | C | Canvas implementation verification |
| P21 | Stub+TDD+Impl | D+E | Colormap FFI + C file USE_RUST_GFX guards |
| P21a | Verification | D+E | Colormap + guards verification |
| P22 | Verification | D+E | All guards work, Rust builds without C graphics |
| P23 | Stub+TDD+Impl | F+G | Widget + GfxLoad bridge |
| P23a | Verification | F+G | Widget + GfxLoad verification |
| P24 | Integration | H | End-to-end testing, visual equivalence |
| P24a | Verification | H | Integration verification |
| P25 | Impl | H | C code removal — delete guarded C fallback paths |
| P25a | Verification | H | C removal verification |
| P26 | Integration | H | Final verification — zero C graphics code compiled |

## C File Inventory (41 files)

### Already guarded (2 files):
- `sdl/sdl_common.c` — vtable wiring
- `sdl/scalers.c` — scaler selection

### Needs guards — Drawing layer (Slice B/C target, 8 files):
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

### Needs guards — Resource loading (Slice G target, 5 files):
- `gfxload.c` — graphics resource loading
- `resgfx.c` — resource GFX management
- `filegfx.c` — file-based GFX loading
- `loaddisp.c` — display loading
- `sdl/png2sdl.c` — PNG to SDL conversion

### Needs guards — Core abstractions (Slice C/F target, 8 files):
- `context.c` — drawing context (replaced by context.rs)
- `drawable.c` — drawable management (replaced by drawable.rs)
- `frame.c` — frame/animation (replaced by frame.rs)
- `pixmap.c` — pixmap management (replaced by pixmap.rs)
- `intersec.c` — intersection calculations
- `font.c` — font rendering
- `gfx_common.c` — common GFX utilities (replaced by gfx_common.rs)
- `widgets.c` — widget system (941 lines, Slice F)

### Needs guards — SDL backend (Slice A already covers, 4 files):
- `sdl/sdl2_pure.c` — SDL2 pure backend (vtable target)
- `sdl/sdl2_common.c` — SDL2 common utilities
- `sdl/sdl1_common.c` — SDL1 compatibility (may be dead code)
- `sdl/pure.c` — pure software backend
- `sdl/opengl.c` — OpenGL backend (may be dead code)
- `sdl/sdluio.c` — SDL UIO integration

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
