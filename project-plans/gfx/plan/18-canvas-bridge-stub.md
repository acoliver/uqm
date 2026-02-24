> **NOTE**: This file's name is a historical artifact from a phase reorder.
> Canonical: Phase P18 = DCQ Bridge — Stub (Slice C)


# Phase 18: DCQ FFI Bridge — Stub

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P18`

## Prerequisites
- Required: Phase P17a (Canvas Implementation Verification) completed
- Expected: Canvas FFI bridge fully implemented (P17), all tests passing
- Expected: Rust dcqueue.rs (1,362 lines) implemented with 15 command types
- Expected: `SurfaceCanvas` (implementing `PixelCanvas` trait) adapter available for DCQ flush dispatch

## Requirements Implemented (Expanded)

### REQ-DCQ-010: DCQ Global Instance
**Requirement text**: The Rust GFX backend shall provide a global DCQ
instance accessible from C via FFI, replacing the C `dcqueue.c` command
queue.

Behavior contract:
- GIVEN: The Rust backend is initialized
- WHEN: C code needs to enqueue draw commands
- THEN: A global `DrawCommandQueue` is available via `unsafe` singleton access

### REQ-DCQ-020: DCQ Push Commands
**Requirement text**: When C code calls `rust_dcq_push_*` functions, the
backend shall enqueue the corresponding draw command in the Rust DCQ.

Behavior contract:
- GIVEN: The DCQ is initialized
- WHEN: `rust_dcq_push_drawline(x1, y1, x2, y2, color)` is called
- THEN: A `DrawLine` command is enqueued with the given parameters

### REQ-DCQ-030: DCQ Flush
**Requirement text**: When `rust_dcq_flush` is called, the backend shall
process all enqueued commands in FIFO order, executing the corresponding
Rust drawing operations via the `SurfaceCanvas` adapter (P17).

Behavior contract:
- GIVEN: The DCQ has N enqueued commands
- WHEN: `rust_dcq_flush` is called
- THEN: All N commands are dequeued and executed in order, queue is empty after
- THEN: Each draw command dispatches through `canvas_ffi.rs` → `tfb_draw.rs`

### REQ-DCQ-040: DCQ Screen Binding
**Requirement text**: When `rust_dcq_set_screen` is called, the backend
shall direct subsequent draw commands to the specified screen surface.

Behavior contract:
- GIVEN: The DCQ is active
- WHEN: `rust_dcq_set_screen(screen_index)` is called
- THEN: Subsequent draw operations target `surfaces[screen_index]`

### REQ-FFI-030: Panic Safety (catch_unwind)
**Requirement text**: No `extern "C" fn` shall allow a Rust panic to
propagate across the FFI boundary.

Behavior contract:
- GIVEN: Any `rust_dcq_*` FFI function
- WHEN: An internal Rust panic occurs
- THEN: `catch_unwind` catches it; function returns a safe default

## Implementation Tasks

### C functions replaced by this phase

These C functions from `dcqueue.c` will have Rust FFI equivalents:

| C Function | Rust FFI Export | Purpose |
|---|---|---|
| `TFB_DrawScreen_Line` | `rust_dcq_push_drawline` | Enqueue line draw |
| `TFB_DrawScreen_Rect` | `rust_dcq_push_drawrect` | Enqueue rect draw |
| `TFB_DrawScreen_FilledRect` | `rust_dcq_push_fillrect` | Enqueue filled rect |
| `TFB_DrawScreen_Image` | `rust_dcq_push_drawimage` | Enqueue image blit |
| `TFB_DrawScreen_Copy` | `rust_dcq_push_copy` | Enqueue screen copy |
| `TFB_DrawScreen_SetPalette` | `rust_dcq_push_setpalette` | Enqueue palette set |
| `TFB_DrawScreen_CopyToImage` | `rust_dcq_push_copytoimage` | Enqueue copy-to-image |
| `TFB_DrawScreen_DeleteImage` | `rust_dcq_push_deleteimage` | Enqueue image deletion |
| `TFB_DrawScreen_WaitForSignal` | `rust_dcq_push_waitsignal` | Enqueue wait-for-signal |
| `TFB_DrawScreen_ReinitVideo` | `rust_dcq_push_reinitvideo` | Enqueue video reinit |
| `TFB_FlushGraphics` | `rust_dcq_flush` | Flush/process all commands |
| `TFB_BatchGraphics` / `TFB_UnbatchGraphics` | `rust_dcq_batch` / `rust_dcq_unbatch` | Batch mode control |
| `TFB_GetScreenIndex` | `rust_dcq_get_screen` | Get current draw screen |
| `TFB_SetScreenIndex` | `rust_dcq_set_screen` | Set current draw screen |

### Surface Accessor Pattern (required for P20)

The stub phase must define the new `get_screen_surface() -> *mut SDL_Surface`
accessor pattern that replaces `get_screen_canvas() -> Arc<RwLock<Canvas>>`.
This accessor is used by DCQ flush (P20) to obtain the raw surface pointer
from which `SurfaceCanvas` is created. The stub can return `std::ptr::null_mut()`
initially, but the function signature and accessor pattern must be established
here so that P20 can wire it to actual surface pointers.

### Files to create
- `rust/src/graphics/dcq_ffi.rs` — New file for DCQ FFI exports
  - Global `DrawCommandQueue` singleton using `UnsafeCell` pattern (like ffi.rs)
  - All ~15 `#[no_mangle] pub extern "C" fn rust_dcq_*` stubs
  - `get_screen_surface(index) -> *mut SDL_Surface` accessor (stub returns null)
  - Each stub: `catch_unwind` wrapper, parameter validation, returns safe default
  - Body: `todo!("DCQ FFI: <function_name>")` (allowed in stub phase)
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P18`
  - marker: `@requirement REQ-DCQ-010, REQ-DCQ-020, REQ-DCQ-030, REQ-DCQ-040, REQ-FFI-030`

### Files to modify
- `rust/src/graphics/mod.rs`
  - Add `pub mod dcq_ffi;`
- `sc2/src/libs/graphics/sdl/rust_gfx.h`
  - Add `rust_dcq_*` function declarations

### Integration Contract

#### Who calls this new behavior?
- C game code currently calls `TFB_DrawScreen_*` in `dcqueue.c`
- These will be redirected to `rust_dcq_*` via `USE_RUST_GFX` guards (in P22/P23)
- For now, stubs are exported but not yet called from C

#### What old behavior gets replaced?
- `dcqueue.c` implements a producer-consumer queue with condition variables
- The Rust DCQ in `dcqueue.rs` has the same architecture
- This phase creates the FFI bridge between them

#### How is backward compatibility handled?
- Stubs return safe defaults (0 or void)
- `USE_RUST_GFX` guard not yet added to `dcqueue.c` — C path still active
- Both paths can coexist during development

#### Dependency on Canvas bridge (P15–P17)
- DCQ flush dispatches draw commands to `tfb_draw.rs` functions
- Those functions accept any type implementing the `PixelCanvas` trait (REQ-CANVAS-150)
- The `SurfaceCanvas` adapter (P17) wraps `SDL_Surface` pixel buffers and implements `PixelCanvas`
- Without P17, DCQ flush would have no surface to draw onto

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify all DCQ exports are present
grep -c '#\[no_mangle\]' rust/src/graphics/dcq_ffi.rs
# Expected: >= 15

# Verify exports are linkable
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep rust_dcq_ | wc -l
# Expected: >= 15

# Verify catch_unwind on all exports
grep -c 'catch_unwind' rust/src/graphics/dcq_ffi.rs
# Expected: >= 15
```

## Structural Verification Checklist
- [ ] `dcq_ffi.rs` created with all ~15 `#[no_mangle]` exports
- [ ] Each export has `catch_unwind` wrapper
- [ ] Global DCQ singleton declared with `UnsafeCell` pattern
- [ ] `mod.rs` updated with `pub mod dcq_ffi`
- [ ] `rust_gfx.h` updated with C declarations
- [ ] All stubs compile and link
- [ ] Plan/requirement traceability comments present

## Semantic Verification Checklist (Mandatory)
- [ ] Each stub has correct C-compatible parameter types
- [ ] Return types match C expectations (0 for success, -1 for error, void)
- [ ] `catch_unwind` returns safe default on panic (not UB)
- [ ] No mutable global access without `UnsafeCell` pattern
- [ ] Singleton follows same safety pattern as `GraphicsStateCell` in ffi.rs

## Deferred Implementation Detection (Mandatory)

```bash
# todo!() is ALLOWED in stub phase — but no other deferred patterns
grep -n "FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcq_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All ~15 DCQ FFI stubs compile
- [ ] All exports are linkable (`nm` shows symbols)
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] C header declarations added

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/dcq_ffi.rs rust/src/graphics/mod.rs`
- blocking issues: dcqueue.rs API incompatible with FFI signatures

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P18.md`

Contents:
- phase ID: P18
- timestamp
- files created: `rust/src/graphics/dcq_ffi.rs`
- files modified: `rust/src/graphics/mod.rs`, `sc2/src/libs/graphics/sdl/rust_gfx.h`
- total `#[no_mangle]` exports: count
- verification: cargo suite output
