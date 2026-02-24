# Phase 22: C File Guards — Level 0 (Scalers + Primitives + Geometry)

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P22`

## Prerequisites
- Required: Phase P21a (Colormap FFI Verification) completed
- Expected: Colormap FFI bridge fully implemented
- Expected: Canvas FFI bridge fully implemented (P17)
- Expected: DCQ FFI bridge fully implemented (P20)

## Requirements Implemented (Expanded)

### REQ-GUARD-030: USE_RUST_GFX Guards on Scaler Layer
**Requirement text**: All scaler C files shall be wrapped in
`#ifndef USE_RUST_GFX` / `#endif` guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 10 scaler C files are excluded from compilation

### REQ-GUARD-010 (partial): Primitives + Geometry Guards
**Requirement text**: Primitives and geometry C files that have NO
dependencies on other guarded files (Level 0 in the dependency graph)
shall be wrapped in guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 5 Level 0 primitive/geometry files are excluded

## Rationale: Level 0 First

Level 0 files have no dependencies on other guarded files — they are
leaf nodes in the dependency graph (see `technical.md` §9.2). They can
be guarded independently without risk of breaking unguarded files that
depend on them. This is the safest starting point for guard application.

## Guard Readiness Gate

Before guarding any file, verify:
1. Every C symbol in the file has a Rust replacement with matching signature
2. `cargo test` passes with the Rust replacement
3. Dual-path build: `USE_RUST_GFX=0` still compiles (C symbols unchanged)
4. `nm` check: no undefined symbols in either build mode

## C Files to Guard in This Phase (~15 files)

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

**Group 1 — Scalers (10 files):**
- `sdl/2xscalers.c` → replaced by `scaling.rs`
- `sdl/2xscalers_mmx.c` → replaced by `scaling.rs`
- `sdl/2xscalers_sse.c` → replaced by `scaling.rs`
- `sdl/2xscalers_3dnow.c` → replaced by `scaling.rs`
- `sdl/bilinear2x.c` → replaced by `scaling.rs`
- `sdl/biadv2x.c` → replaced by `scaling.rs`
- `sdl/hq2x.c` → replaced by `scaling.rs`
- `sdl/nearest2x.c` → replaced by `scaling.rs`
- `sdl/triscan2x.c` → replaced by `scaling.rs`
- `sdl/rotozoom.c` → replaced by `scaling.rs`

**Group 2 — Primitives + Geometry (5 files):**
- `sdl/primitives.c` → replaced by `tfb_draw.rs` (pixel ops)
- `clipline.c` → replaced by `tfb_draw.rs` line clipping
- `boxint.c` → replaced by `tfb_draw.rs` intersection
- `bbox.c` → replaced by `tfb_draw.rs` bounding box
- `intersec.c` → replaced by Rust intersection logic

### Per-File Readiness Checklist

| C File | Key Symbols Requiring Rust Replacement | Rust Module |
|--------|----------------------------------------|-------------|
| All scaler `.c` files | `Scale_HQ2X`, `Scale_BilinearFilter`, `Scale_Nearest`, etc. | `scaling.rs` / `ffi.rs` |
| `sdl/primitives.c` | `putpixel_32`, `getpixel_32` (only 32bpp needed) | `tfb_draw.rs` (internal) |
| `clipline.c` | `TFB_DrawCanvas_ClipLine` | `tfb_draw.rs` (line clipping internal) |
| `boxint.c` | `BoxIntersect`, `BoxUnion` | `tfb_draw.rs` (geometry utils) |
| `bbox.c` | `TFB_BBox_Reset`, `TFB_BBox_RegisterPoint`, `TFB_BBox_GetClipRect` | `tfb_draw.rs` (bounding box) |
| `intersec.c` | `DrawablesIntersect`, `frame_intersect` | `drawable.rs` (geometry) |

## Files to Modify
- 15 C files — Add `USE_RUST_GFX` guards (groups 1–2 above)

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify guard count in C files
grep -rl 'USE_RUST_GFX' sc2/src/libs/graphics/ | wc -l
# Expected: >= 17 (2 existing + 15 new)

# Build with USE_RUST_GFX to verify guarded files are excluded
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | head -50

# Build WITHOUT USE_RUST_GFX to verify C path still works
# (ensure build.vars has USE_RUST_GFX=0)
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | head -50
```

## Structural Verification Checklist
- [ ] All 15 Level 0 C files have `USE_RUST_GFX` guards
- [ ] Guard format is consistent (`#ifdef USE_RUST_GFX` / `#else` / `#endif`)
- [ ] Each guarded file has a comment pointing to Rust replacement
- [ ] No header files are guarded (only .c files)
- [ ] Guard placement is after `#include` directives
- [ ] `sdl_common.c` and `scalers.c` existing guards are not duplicated

## Semantic Verification Checklist (Mandatory)
- [ ] Guards are at file level (entire file content guarded, not partial)
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds with `USE_RUST_GFX=0` — no undefined symbols
- [ ] Build system respects guards correctly

## Dual-Path ABI Verification (Mandatory)
```bash
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | grep -c 'undefined'
# Expected: 0
```

## Success Criteria
- [ ] 15 Level 0 C files guarded
- [ ] Both build paths compile without errors
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git stash` (many files modified)
- blocking issues: guarded C files expose missing Rust FFI functions at link time

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P22.md`

Contents:
- phase ID: P22
- timestamp
- files modified: 15 C files (Level 0 guards)
- guard count: 17 total (2 pre-existing + 15 new)
- verification: build with and without `USE_RUST_GFX`
