# Phase 23: Widget + GfxLoad Bridge

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P23`

## Prerequisites
- Required: Phase P22 (Colormap + Guards Verification) completed
- Expected: 34/41 C files guarded, both build paths working
- Expected: DCQ, Canvas, and Colormap FFI bridges fully implemented

## Requirements Implemented (Expanded)

### REQ-WIDGET-010: Widget System Bridge
**Requirement text**: The Rust GFX backend shall either port the C widget
system (`widgets.c`, 941 lines) to Rust or provide a bridge that allows
the C widget code to operate through Rust drawing primitives.

Behavior contract:
- GIVEN: The game needs to render UI widgets (menus, sliders, text boxes)
- WHEN: Widget draw functions are called
- THEN: Widgets are rendered correctly through the Rust graphics pipeline

Design decision (bridge vs port):
- **Bridge approach** (recommended): Keep `widgets.c` but redirect its
  draw calls through `rust_canvas_*` FFI functions. The widget logic
  (layout, hit testing, state management) stays in C. Only the drawing
  calls are replaced.
- **Port approach** (deferred): Full Rust port of widget system. Higher
  effort, more risk, better long-term.

### REQ-WIDGET-020: Widget Draw Redirect
**Requirement text**: Widget drawing functions in `widgets.c` that call
`TFB_DrawCanvas_*` shall be redirected to call `rust_canvas_*` equivalents
when `USE_RUST_GFX` is defined.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: A widget calls `TFB_DrawCanvas_FilledRect(canvas, ...)`
- THEN: The call is redirected to `rust_canvas_fill_rect(canvas, ...)`

### ~~REQ-GFXLOAD-010, -020, -030~~ (DEFERRED — OUT OF SCOPE)

Loader functions (`LoadGraphic`, `LoadFont`, `TFB_LoadPNG`) and their
source files (`gfxload.c`, `filegfx.c`, `resgfx.c`, `loaddisp.c`,
`png2sdl.c`) remain in C and compile in both modes. They are **not
bridged or replaced** in this plan. These requirements are deferred to
a future phase.

### REQ-GUARD-060: Widget-Dependent File Guards + Widget Guard
**Requirement text**: After the widget bridge provides Rust replacements
for the APIs that `widgets.c` depends on, the following files shall be
wrapped in `USE_RUST_GFX` guards: `frame.c`, `font.c`, `context.c`,
`drawable.c`, and `widgets.c` itself.

These files were explicitly deferred from P21 because `widgets.c` calls
into them (`DrawBatch`, `DrawStamp`, `font_DrawText`, `font_DrawTracedText`,
`SetContext`, `SetContextForeGroundColor`, `GetFrameCount`). Guarding
them before the widget bridge exists would break the widget system.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined AND widget bridge is implemented
- WHEN: The build system compiles the graphics library
- THEN: `frame.c`, `font.c`, `context.c`, `drawable.c`, `widgets.c` are excluded

**Loader files (gfxload.c, filegfx.c, resgfx.c, loaddisp.c, png2sdl.c)
are NOT guarded — they remain compiled in both modes.** See `00-overview.md`
"Deferred to Future Phase" section.

## Guard Readiness Gate

Before guarding frame.c/font.c/context.c/drawable.c/widgets.c, ALL
of the following must be true:

1. Every symbol below has a Rust FFI replacement with matching signature
2. `cargo test` passes with all Rust replacements
3. `widgets.c` compiles and links against Rust replacements
4. Widget rendering visually verified (menus render correctly)
5. Dual-path build: `USE_RUST_GFX=0` still compiles
6. `nm` check: no undefined symbols in either build mode

If any symbol lacks a Rust replacement, do NOT guard the corresponding
C file. Widget functionality takes priority over guard coverage.

### Per-File Readiness Checklist

| C File | Key Symbols Requiring Rust Replacement | Rust Module |
|--------|----------------------------------------|-------------|
| `frame.c` | `ClearBackGround`, `DrawPoint`, `DrawRectangle`, `DrawFilledRectangle`, `DrawLine`, `DrawStamp`, `DrawFilledStamp`, `ClearDrawable`, `GetContextValidRect` | `frame_ffi.rs` |
| `font.c` | `SetContextFont`, `DestroyFont`, `font_DrawText`, `font_DrawTracedText`, `TextRect`, `GetContextFontLeading`, `GetContextFontLeadingWidth` | `font_ffi.rs` |
| `context.c` | `SetContext`, `CreateContextAux`, `DestroyContext`, `SetContextForeGroundColor`, `GetContextForeGroundColor`, `SetContextBackGroundColor`, `GetContextBackGroundColor`, `SetContextDrawMode`, `GetContextDrawMode`, `SetContextClipRect`, `GetContextClipRect`, `SetContextOrigin`, `SetContextFontEffect`, `FixContextFontEffect`, `CopyContextRect` | `context_ffi.rs` |
| `drawable.c` | `SetContextFGFrame`, `GetContextFGFrame`, `request_drawable`, `CreateDisplay`, `AllocDrawable`, `CreateDrawable`, `DestroyDrawable`, `GetFrameRect`, `SetFrameHot`, `GetFrameHot`, `RotateFrame`, `RescaleFrame`, `CloneFrame`, `CopyFrameRect`, `SetFrameTransparentColor`, `GetFramePixel`, `ReadFramePixelColors`, `WriteFramePixelColors` | `drawable_ffi.rs` |
| `widgets.c` | All `DrawWidget_*` functions (depend on frame/font/context APIs above) | `widget_ffi.rs` or left in C with Rust API calls |

## Implementation Tasks

### Widget Bridge Strategy

The widget system in `widgets.c` contains:
- `DrawWidget_*` functions (menu items, sliders, text boxes)
- Widget layout/positioning logic
- Widget state management (focus, selection)
- Widget tree traversal

**Bridge approach** (this phase):
1. Add `USE_RUST_GFX` guard to `widgets.c` with conditional compilation
2. Inside the `USE_RUST_GFX` path, replace `TFB_DrawCanvas_*` calls with
   `rust_canvas_*` calls
3. Widget logic (layout, state) remains unchanged in C
4. Alternative: guard the whole file and provide Rust FFI stubs that
   the remaining C code calls

### GfxLoad (Deferred — Out of Scope)

C resource loading files remain unchanged. They compile in both modes:
```
.ani file → gfxload.c:LoadGraphic() → SDL_Surface → frame.c:CreateFrame()
.fon file → gfxload.c:LoadFont() → font.c:CreateFont()
.png file → png2sdl.c:TFB_LoadPNG() → SDL_Surface
```

These are NOT replaced in this plan. See `00-overview.md` "Deferred to
Future Phase" section.

### C functions replaced/bridged

| C Function | Approach | Notes |
|---|---|---|
| `DrawWidget_Menu` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Slider` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_TextEntry` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Choice` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Button` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Label` | Bridge: redirect draw calls | Widget logic stays in C |
| `_CreateFrame` | FFI bridge to Rust | `frame.c` → `rust_frame_create` |
| `DestroyFrame` | FFI bridge to Rust | `frame.c` → `rust_frame_destroy` |

> **Loader functions** (`LoadGraphic`, `LoadFont`, `TFB_LoadPNG`) remain
> in C. Loader files (gfxload.c, filegfx.c, resgfx.c, loaddisp.c, png2sdl.c)
> are OUT OF SCOPE for this plan — they compile in both modes and are not
> bridged or replaced.

### Files to create
- `rust/src/graphics/frame_ffi.rs` (new) — Frame/drawable FFI exports
  - `rust_frame_create`, `rust_frame_destroy`
  - `catch_unwind` on all exports
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P23`
  - marker: `@requirement REQ-WIDGET-010..020, REQ-GUARD-060, REQ-FFI-030`
  - Note: loader functions (`LoadGraphic`, `LoadFont`, `TFB_LoadPNG`)
    are NOT included — they remain in C

### Files to modify
- `rust/src/graphics/mod.rs` — Add `pub mod frame_ffi;` (new module)
- `sc2/src/libs/graphics/sdl/rust_gfx.h` — Add `rust_frame_*` declarations
- `sc2/src/libs/graphics/widgets.c` — Add `USE_RUST_GFX` bridge or guard
- `sc2/src/libs/graphics/frame.c` — Add `USE_RUST_GFX` guard (deferred from P21)
- `sc2/src/libs/graphics/font.c` — Add `USE_RUST_GFX` guard (deferred from P21)
- `sc2/src/libs/graphics/context.c` — Add `USE_RUST_GFX` guard (deferred from P21)
- `sc2/src/libs/graphics/drawable.c` — Add `USE_RUST_GFX` guard (deferred from P21)

**NOT guarded (loader files remain compiled in both modes):**
- `sc2/src/libs/graphics/gfxload.c` — stays unguarded
- `sc2/src/libs/graphics/resgfx.c` — stays unguarded
- `sc2/src/libs/graphics/filegfx.c` — stays unguarded
- `sc2/src/libs/graphics/loaddisp.c` — stays unguarded
- `sc2/src/libs/graphics/sdl/png2sdl.c` — stays unguarded

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify guard status of all C files
for f in $(find sc2/src/libs/graphics -name '*.c' | sort); do
  if grep -q 'USE_RUST_GFX' "$f"; then
    echo "[GUARDED] $f"
  else
    echo "[UNGUARDED] $f"
  fi
done
# Expected: ~36 GUARDED, ~5 UNGUARDED (loader files + sdl_common.c stay unguarded)

# Verify frame/drawable FFI exports
grep -c '#\[no_mangle\]' rust/src/graphics/frame_ffi.rs
# Expected: >= 2 (rust_frame_create, rust_frame_destroy)

# Build with USE_RUST_GFX
cd sc2 && make clean && make USE_RUST_GFX=1 2>&1 | tee /tmp/build_full_rust.log
grep -c 'error:' /tmp/build_full_rust.log
# Expected: 0

# Build without USE_RUST_GFX (backward compat)
cd sc2 && make clean && make 2>&1 | tee /tmp/build_c_path.log
grep -c 'error:' /tmp/build_c_path.log
# Expected: 0
```

## Structural Verification Checklist
- [ ] `frame_ffi.rs` created with >= 2 `#[no_mangle]` exports
- [ ] All drawing-pipeline C files now have `USE_RUST_GFX` guards (~36; loader files excluded)
- [ ] Widget bridge strategy implemented (bridge or guard)
- [ ] `mod.rs` updated with `pub mod frame_ffi`
- [ ] `rust_gfx.h` updated with gfxload declarations
- [ ] Both build paths compile without errors

## Semantic Verification Checklist (Mandatory)
- [ ] Widget rendering produces visually correct output through bridge
- [ ] No memory leaks from Rust-owned resources returned to C
- [ ] Resource destruction functions (`rust_frame_destroy`) properly free Rust memory
- [ ] Loader files (gfxload.c, filegfx.c, resgfx.c, loaddisp.c) remain unguarded and functional

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "todo!\|TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/frame_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] ~36 drawing-pipeline C files guarded (loader files stay unguarded)
- [ ] Widget-dependent files (frame.c, font.c, context.c, drawable.c) now guarded
- [ ] Widget bridge or port functional
- [ ] `rust_frame_create` / `rust_frame_destroy` functional
- [ ] Both build paths work
- [ ] All cargo gates pass

## Dual-Path ABI Verification (Mandatory)
```bash
# Build with USE_RUST_GFX=0 and verify no undefined symbols
cd sc2 && make clean && make USE_RUST_GFX=0 2>&1 | grep -c 'undefined'
# Expected: 0
```

## Failure Recovery
- rollback: `git stash`
- blocking issues: widget system too deeply coupled to C drawing internals

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P23.md`

Contents:
- phase ID: P23
- timestamp
- files created: `rust/src/graphics/frame_ffi.rs`
- files modified: 5 C files (widget-dependent guards), `mod.rs`, `rust_gfx.h`
- C files guarded: ~36/41 drawing-pipeline files (4 loader files + sdl_common.c intentionally unguarded)
- widget approach: bridge or port
- frame FFI exports: count
- verification: both build paths successful
