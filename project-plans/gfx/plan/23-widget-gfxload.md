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

### REQ-GFXLOAD-010: Graphics Resource Loading Bridge
**Requirement text**: The Rust backend shall provide FFI functions for
loading graphics resources (frames, fonts) into Rust-managed structures.

Behavior contract:
- GIVEN: C code loads a `.ani` or `.fon` resource
- WHEN: `rust_gfxload_frame(data, size)` is called
- THEN: The frame data is parsed and stored in Rust `Frame` / `Pixmap`

### REQ-GFXLOAD-020: Font Loading Bridge
**Requirement text**: The Rust backend shall provide FFI functions for
loading font resources into Rust-managed font structures.

Behavior contract:
- GIVEN: C code loads a `.fon` resource
- WHEN: `rust_gfxload_font(data, size)` is called
- THEN: The font data is parsed into Rust `FontPage` structures

### REQ-GFXLOAD-030: PNG to Surface Bridge
**Requirement text**: The Rust backend shall provide a function that loads
a PNG file into an SDL_Surface-compatible pixel buffer.

Behavior contract:
- GIVEN: A PNG file path
- WHEN: `rust_gfxload_png(path)` is called
- THEN: Returns `*mut SDL_Surface` with decoded pixel data

### REQ-GUARD-060: Widget and GfxLoad Guards
**Requirement text**: After bridging, `widgets.c`, `gfxload.c`, `resgfx.c`,
`filegfx.c`, `loaddisp.c`, `sdl/png2sdl.c`, and `font.c` shall be
wrapped in `USE_RUST_GFX` guards.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: The build system compiles the graphics library
- THEN: The 7 remaining C files are excluded from compilation

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

### GfxLoad Bridge

C resource loading pipeline:
```
.ani file → gfxload.c:LoadGraphic() → SDL_Surface → frame.c:CreateFrame()
.fon file → gfxload.c:LoadFont() → font.c:CreateFont()
.png file → png2sdl.c:TFB_LoadPNG() → SDL_Surface
```

Rust bridge:
```
.ani file → rust_gfxload_frame() → Rust Frame/Pixmap
.fon file → rust_gfxload_font() → Rust FontPage
.png file → rust_gfxload_png() → *mut SDL_Surface (via image crate or libpng FFI)
```

### C functions replaced/bridged

| C Function | Approach | Notes |
|---|---|---|
| `DrawWidget_Menu` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Slider` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_TextEntry` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Choice` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Button` | Bridge: redirect draw calls | Widget logic stays in C |
| `DrawWidget_Label` | Bridge: redirect draw calls | Widget logic stays in C |
| `LoadGraphic` | FFI bridge to Rust | `gfxload.c` → `rust_gfxload_frame` |
| `LoadFont` | FFI bridge to Rust | `gfxload.c` → `rust_gfxload_font` |
| `TFB_LoadPNG` | FFI bridge to Rust | `png2sdl.c` → `rust_gfxload_png` |
| `_CreateFrame` | FFI bridge to Rust | `frame.c` → `rust_frame_create` |
| `DestroyFrame` | FFI bridge to Rust | `frame.c` → `rust_frame_destroy` |

### Files to create
- `rust/src/graphics/gfxload_ffi.rs` — GfxLoad FFI exports
  - `rust_gfxload_frame`, `rust_gfxload_font`, `rust_gfxload_png`
  - `rust_frame_create`, `rust_frame_destroy`
  - `catch_unwind` on all exports
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P23`
  - marker: `@requirement REQ-GFXLOAD-010..030, REQ-WIDGET-010..020, REQ-FFI-030`

### Files to modify
- `rust/src/graphics/mod.rs` — Add `pub mod gfxload_ffi;`
- `sc2/src/libs/graphics/sdl/rust_gfx.h` — Add `rust_gfxload_*` declarations
- `sc2/src/libs/graphics/widgets.c` — Add `USE_RUST_GFX` bridge or guard
- `sc2/src/libs/graphics/gfxload.c` — Add `USE_RUST_GFX` guard
- `sc2/src/libs/graphics/resgfx.c` — Add `USE_RUST_GFX` guard
- `sc2/src/libs/graphics/filegfx.c` — Add `USE_RUST_GFX` guard
- `sc2/src/libs/graphics/loaddisp.c` — Add `USE_RUST_GFX` guard
- `sc2/src/libs/graphics/sdl/png2sdl.c` — Add `USE_RUST_GFX` guard
- `sc2/src/libs/graphics/font.c` — Add `USE_RUST_GFX` guard

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify all C files now guarded
for f in $(find sc2/src/libs/graphics -name '*.c' | sort); do
  if grep -q 'USE_RUST_GFX' "$f"; then
    echo "[GUARDED] $f"
  else
    echo "[UNGUARDED] $f"
  fi
done
# Expected: 41 GUARDED, 0 UNGUARDED

# Verify gfxload exports
grep -c '#\[no_mangle\]' rust/src/graphics/gfxload_ffi.rs
# Expected: >= 5

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
- [ ] `gfxload_ffi.rs` created with >= 5 `#[no_mangle]` exports
- [ ] All 41 C files now have `USE_RUST_GFX` guards
- [ ] Widget bridge strategy implemented (bridge or guard)
- [ ] `mod.rs` updated with `pub mod gfxload_ffi`
- [ ] `rust_gfx.h` updated with gfxload declarations
- [ ] Both build paths compile without errors

## Semantic Verification Checklist (Mandatory)
- [ ] Widget rendering produces visually correct output through bridge
- [ ] GfxLoad correctly parses `.ani` frame data (or provides working stubs)
- [ ] Font loading correctly parses `.fon` data (or provides working stubs)
- [ ] PNG loading produces correct pixel data in SDL_Surface
- [ ] No memory leaks from Rust-owned resources returned to C
- [ ] Resource destruction functions (`rust_frame_destroy`) properly free Rust memory

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "todo!\|TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/gfxload_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All 41 C files guarded
- [ ] Widget bridge or port functional
- [ ] GfxLoad FFI bridge functional
- [ ] Both build paths work
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: widget system too deeply coupled to C drawing internals

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P23.md`

Contents:
- phase ID: P23
- timestamp
- files created: `rust/src/graphics/gfxload_ffi.rs`
- files modified: 7 C files (remaining guards), `mod.rs`, `rust_gfx.h`
- C files guarded: 41/41
- widget approach: bridge or port
- gfxload exports: count
- verification: both build paths successful
