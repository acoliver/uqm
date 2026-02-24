> **NOTE**: This file's name is a historical artifact from a phase reorder.
> Canonical: Phase P20 = DCQ Bridge — Impl (Slice C)


# Phase 20: DCQ FFI Bridge — Implementation

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P20`

## Prerequisites
- Required: Phase P19a (DCQ TDD Verification) completed
- Expected: All DCQ tests written, stubs compile
- Expected: `dcqueue.rs` provides `DrawCommandQueue` with push/flush/batch API
- Expected: Canvas FFI bridge fully implemented (P17), providing `SurfaceCanvas` (implements `PixelCanvas` trait)

## Requirements Implemented (Expanded)

### REQ-DCQ-010: DCQ Global Instance
**Requirement text**: The Rust GFX backend shall provide a global DCQ
instance accessible from C via FFI.

Implementation:
- Global `DCQ_STATE: DcqStateCell` using `UnsafeCell<Option<DcqState>>`
- `DcqState` wraps `DrawCommandQueue` + current screen index + batch depth
- `unsafe impl Sync for DcqStateCell` with safety proof (single-threaded per REQ-THR-010)
- `rust_dcq_init()` creates the instance, `rust_dcq_uninit()` destroys it
- Called from `rust_gfx_init()` / `rust_gfx_uninit()` respectively

### REQ-DCQ-020: DCQ Push Commands
Implementation for each command type:
- Translate C parameters (raw pointers, ints) to Rust types
- Validate parameters (null checks, range checks)
- Construct appropriate `DrawCommand` variant
- Push onto `DrawCommandQueue`
- Return 0 on success, -1 on error

### REQ-DCQ-030: DCQ Flush
Implementation:
- Dequeue all commands in FIFO order
- For each command, dispatch to the corresponding `tfb_draw.rs` generic function
- Commands operate on the current screen's `SurfaceCanvas` (wrapping `SDL_Surface`,
  implementing `PixelCanvas` trait per REQ-CANVAS-150)
- Drawing functions are called with `<SurfaceCanvas>` as the `PixelCanvas` impl
- `SurfaceCanvas` is created at flush start, dropped at flush end (see §8.7 contract)
- Return number of commands processed (or 0 if empty)

### REQ-DCQ-040: DCQ Screen Binding
Implementation:
- Store current screen index in `DcqState`
- Validate screen index range [0, TFB_GFX_NUMSCREENS)
- Draw commands use `SurfaceCanvas::from_surface(surfaces[current_screen])` as target

> **Note**: DCQ's `RenderContext` screen storage will change from
> `Arc<RwLock<Canvas>>` to a `PixelCanvas` trait-object-based or generic
> approach during implementation. The `SurfaceCanvas` wrapping borrowed
> `SDL_Surface` pixels cannot be stored behind `Arc<RwLock<>>` — it is
> created transiently during flush and dropped after. See technical.md
> §8.4.0 and §8.7.

### REQ-DCQ-050: DCQ Batch Mode
Implementation:
- `batch_depth: u32` counter in `DcqState`
- `rust_dcq_batch()` increments depth
- `rust_dcq_unbatch()` decrements depth; if depth reaches 0, auto-flush
- `rust_dcq_flush()` is a no-op while `batch_depth > 0`

### REQ-FFI-030: Panic Safety
Implementation:
- Every `extern "C" fn` body wrapped in `std::panic::catch_unwind(AssertUnwindSafe(|| { ... }))`
- Panic → return safe default (0 for int, void for void)

## Implementation Tasks

### C functions fully replaced

| C Function (`dcqueue.c`) | Rust Implementation | Key Differences |
|---|---|---|
| `TFB_DrawScreen_Line` | `rust_dcq_push_drawline` → `dcqueue.rs::push(DrawLine{...})` | Type-safe command enum |
| `TFB_DrawScreen_Rect` | `rust_dcq_push_drawrect` → `dcqueue.rs::push(DrawRect{...})` | No raw pointer arithmetic |
| `TFB_DrawScreen_FilledRect` | `rust_dcq_push_fillrect` → `dcqueue.rs::push(FillRect{...})` | Bounds-checked |
| `TFB_DrawScreen_Image` | `rust_dcq_push_drawimage` → `dcqueue.rs::push(DrawImage{...})` | Lifetime-safe image refs |
| `TFB_DrawScreen_Copy` | `rust_dcq_push_copy` → `dcqueue.rs::push(CopyScreen{...})` | Validated screen indices |
| `TFB_DrawScreen_SetPalette` | `rust_dcq_push_setpalette` → `dcqueue.rs::push(SetPalette{...})` | Palette data copied |
| `TFB_DrawScreen_CopyToImage` | `rust_dcq_push_copytoimage` → `dcqueue.rs::push(CopyToImage{...})` | Safe surface access |
| `TFB_DrawScreen_DeleteImage` | `rust_dcq_push_deleteimage` → `dcqueue.rs::push(DeleteImage{...})` | RAII cleanup |
| `TFB_DrawScreen_WaitForSignal` | `rust_dcq_push_waitsignal` → `dcqueue.rs::push(WaitSignal)` | Condvar signaling |
| `TFB_DrawScreen_ReinitVideo` | `rust_dcq_push_reinitvideo` → `dcqueue.rs::push(ReinitVideo{...})` | Safe reinit |
| `TFB_FlushGraphics` | `rust_dcq_flush` → `dcqueue.rs::flush()` → `SurfaceCanvas` dispatch | Batch-aware |
| `TFB_BatchGraphics` | `rust_dcq_batch` → depth++ | Nestable |
| `TFB_UnbatchGraphics` | `rust_dcq_unbatch` → depth--, auto-flush at 0 | Nestable |

### DCQ `handle_command` Dispatch Rework

Refactor `DrawCommandQueue::handle_command()` — all 9 dispatch arms (Line,
Rect, Image, FilledImage, FontChar, Copy, CopyToImage, ScissorEnable,
ScissorDisable) must call generic drawing functions via `PixelCanvas` trait.
The `get_screen_surface()` method returns the `*mut SDL_Surface` pointer;
the flush function creates a `SurfaceCanvas` from it, then passes
`&mut SurfaceCanvas` to the generic drawing functions. For owned `Canvas`
targets (offscreen buffers), flush calls `canvas.lock_pixels()` to obtain
a `LockedCanvas<'_>`, then passes `&mut LockedCanvas` instead.

Estimated: ~150 LoC changes in `dcqueue.rs`.

### Files to modify
- `rust/src/graphics/dcq_ffi.rs`
  - Replace all `todo!()` stubs with full implementations
  - Wire DCQ commands to `dcqueue.rs` API
  - Wire flush dispatch to `tfb_draw.rs` functions via `SurfaceCanvas`
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P20`
  - marker: `@requirement REQ-DCQ-010..050, REQ-FFI-030`

- `rust/src/graphics/dcqueue.rs`
  - Refactor `handle_command()` to dispatch through `PixelCanvas` trait
  - Replace `Arc<RwLock<Canvas>>` screen access with `SurfaceCanvas` /
    `LockedCanvas` pattern per technical.md §8.4.0 and §8.4.0a

- `rust/src/graphics/ffi.rs`
  - Add `rust_dcq_init()` call inside `rust_gfx_init()`
  - Add `rust_dcq_uninit()` call inside `rust_gfx_uninit()`

### Integration Contract

#### Who calls this new behavior?
- C game code → `TFB_DrawScreen_*` → (via `USE_RUST_GFX` guard, added in P23) → `rust_dcq_*`
- `rust_gfx_init` / `rust_gfx_uninit` → `rust_dcq_init` / `rust_dcq_uninit`

#### How can a user trigger this end-to-end?
1. Build with `USE_RUST_GFX` (after P23 adds guards to dcqueue.c)
2. Game enqueues draw commands → Rust DCQ stores them
3. `TFB_FlushGraphics` → `rust_dcq_flush` → creates `SurfaceCanvas` from current screen
4. Rust executes all draws on `SurfaceCanvas` → pixels land in `SDL_Surface.pixels`
5. Screen shows Rust-rendered primitives

#### Dependency on Canvas bridge (P15–P17)
- DCQ flush creates `SurfaceCanvas::from_surface(surfaces[screen_index])`
- Each draw command dispatches to generic `tfb_draw.rs` functions via `PixelCanvas` trait
- `SurfaceCanvas` implements `PixelCanvas`, so `draw_line::<SurfaceCanvas>(...)` etc. work
- Without the Canvas adapter, DCQ flush cannot execute draw commands

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# All DCQ tests must pass now (no more todo!())
cd rust && cargo test --lib -- dcq_ffi::tests --nocapture
# Expected: all >= 17 tests pass

# Verify no deferred patterns
grep -n "todo!\|TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/dcq_ffi.rs && echo "FAIL" || echo "CLEAN"

# Verify all exports still linkable
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep rust_dcq_ | wc -l
# Expected: >= 15
```

## Structural Verification Checklist
- [ ] All `todo!()` stubs replaced with implementations
- [ ] Global DCQ singleton properly initialized/torn down
- [ ] All ~15 FFI functions have full implementations
- [ ] `catch_unwind` on every `extern "C" fn`
- [ ] `rust_dcq_init` called from `rust_gfx_init`
- [ ] `rust_dcq_uninit` called from `rust_gfx_uninit`
- [ ] Plan/requirement traceability comments present

## Semantic Verification Checklist (Mandatory)
- [ ] Push functions correctly translate C types to Rust `DrawCommand` variants
- [ ] Flush processes commands in FIFO order — verified by test
- [ ] Flush creates `SurfaceCanvas` from current screen surface — verified by test
- [ ] Batch mode correctly defers flush — verified by test
- [ ] Screen binding validates index range — verified by test
- [ ] Null pointer parameters handled safely (no UB)
- [ ] `catch_unwind` returns safe defaults on panic — verified by test

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "todo!\|TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcq_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All DCQ FFI functions fully implemented
- [ ] All >= 17 tests pass
- [ ] DCQ init/uninit wired into GFX init/uninit
- [ ] DCQ flush dispatches through SurfaceCanvas to tfb_draw.rs
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] No deferred patterns

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/dcq_ffi.rs rust/src/graphics/ffi.rs`
- blocking issues: `dcqueue.rs` API doesn't support needed operations

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P20.md`

Contents:
- phase ID: P20
- timestamp
- files modified: `rust/src/graphics/dcq_ffi.rs`, `rust/src/graphics/ffi.rs`
- total tests: count (all passing)
- total `#[no_mangle]` DCQ exports: count
- verification: cargo suite output
- semantic: DCQ push/flush/batch verified end-to-end via SurfaceCanvas
