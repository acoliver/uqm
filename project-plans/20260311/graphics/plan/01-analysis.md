# Phase 01: Analysis

## Phase ID
`PLAN-20260314-GRAPHICS.P01`

## Prerequisites
- Required: Phase P00.5 completed (preflight gate passed)

## Purpose
Document the precise gap locations and data flows for each identified deficiency.

## Gap Analysis

### G1: Canvas Pixel Synchronization (REQ-CAN-006, REQ-INT-006, REQ-INT-007)

**Current state:** `rust_canvas_from_surface()` in `canvas_ffi.rs:87-125` creates a `SurfaceCanvas` that:
1. Stores the `SDL_Surface*` pointer
2. Creates a **fresh** `Canvas::new_rgba(w, h)` — all zeros
3. Never reads existing pixels from the SDL surface into the Canvas
4. On `rust_canvas_destroy()`, drops the Canvas without writing pixels back to the surface
5. Does not define who flushes canvas state before presentation compositing, transition capture, or interoperability reads

**Required state:** Per spec §6.2 and REQ-CAN-006:
- Canvas creation must import existing surface pixel data
- Canvas destruction (or explicit flush) must write modified pixels back to the surface
- Pixel coherence must be maintained before presentation, transition capture, and interop reads
- The synchronization strategy must be end-to-end, not destroy-only

**Synchronization points that must be explicitly satisfied:**
- Before `rust_gfx_screen()` / presentation reads the surface
- Before `SetTransitionSource` / transition capture reads main-screen pixels
- Before screen-to-image copy or any other interop read returns current pixels

**Data flow:**
```
SDL_Surface.pixels (RGBX8888: [X,B,G,R] per pixel, pitch-strided)
    ↓ import on create
SurfaceCanvas.canvas.data (RGBA: [R,G,B,A] per pixel, width-strided)
    ↓ drawing operations modify canvas.data
    ↓ flush at required synchronization points
SDL_Surface.pixels (RGBX8888)
```

**Pixel format conversion needed:**
- Import: `surface[X,B,G,R]` → `canvas[R,G,B,255]` (little-endian RGBX to RGBA)
- Export: `canvas[R,G,B,A]` → `surface[X,B,G,R]` (RGBA to RGBX, alpha discarded)
- Pitch handling: surface has `pitch` (bytes per row, may include padding); canvas is `width * 4`

**Files to modify:**
- `rust/src/graphics/canvas_ffi.rs`: `rust_canvas_from_surface()`, `rust_canvas_destroy()`, add `rust_canvas_flush()`
- `rust/src/graphics/ffi.rs` and/or transition/copy call sites: flush or otherwise synchronize surface-backed canvases before reads

---

### G2: Postprocess Fallback Removal (REQ-RL-004, REQ-INT-002, REQ-INT-010)

**Current state:** `rust_gfx_postprocess()` in `ffi.rs:477-598` contains:
- A full surface→texture upload path
- Scaling logic (nearest, bilinear, software scalers)
- Texture copy to renderer
- `canvas.present()`

This duplicates work done in `rust_gfx_screen()` (ffi.rs:617-816) which already composites screen layers.

**Required state:** Per spec §9.2, `postprocess()` shall:
- Apply optional scanline effect if `SCANLINES` flag is set
- Call `renderer.present()`
- Shall NOT perform texture upload or surface-to-renderer copy

**Files to modify:**
- `rust/src/graphics/ffi.rs`: `rust_gfx_postprocess()` — strip to scanline + present

---

### G3: Scanline Effect (REQ-SCAL-006)

**Current state:** No scanline implementation exists in the Rust graphics code.

**Required state:** Per spec §8.4, when `SCANLINES` flag (bit 2) is set, alternating horizontal lines should be dimmed during postprocess before present.

**Reference:** C implementation at `sc2/src/libs/graphics/sdl/sdl2_pure.c:344-356`.

**Files to modify:**
- `rust/src/graphics/ffi.rs`: Add scanline rendering in `rust_gfx_postprocess()`

---

### G4: Missing DCQ Push Functions (REQ-DQ-001, REQ-INT-001, REQ-INT-003)

**Current state:** `dcq_ffi.rs` exports these push functions:
- [OK] `rust_dcq_push_drawline`
- [OK] `rust_dcq_push_drawrect`
- [OK] `rust_dcq_push_fillrect`
- [OK] `rust_dcq_push_drawimage` (but missing scale/colormap/drawmode — see G5)
- [OK] `rust_dcq_push_copy`
- [OK] `rust_dcq_push_copytoimage`
- [OK] `rust_dcq_push_deleteimage`
- [OK] `rust_dcq_push_waitsignal`
- [OK] `rust_dcq_push_reinitvideo`
- [OK] `rust_dcq_push_setpalette` (but stubbed — see G6)
- [OK] `rust_dcq_push_scissor_enable`
- [OK] `rust_dcq_push_scissor_disable`
- [ERROR] `rust_dcq_push_filledimage` — MISSING
- [ERROR] `rust_dcq_push_fontchar` — MISSING
- [ERROR] `rust_dcq_push_setmipmap` — MISSING
- [ERROR] `rust_dcq_push_deletedata` — MISSING
- [ERROR] `rust_dcq_push_callback` — MISSING

**Required state:** All 16 queue commands from spec §5.1 must have corresponding push functions where caller-facing C/Rust ingress uses the queue.

**Authoritative command inventory:**
1. `Line`
2. `Rect`
3. `Image`
4. `FilledImage`
5. `FontChar`
6. `Copy`
7. `CopyToImage`
8. `SetMipmap`
9. `DeleteImage`
10. `DeleteData`
11. `SendSignal`
12. `ReinitVideo`
13. `SetPalette`
14. `ScissorEnable`
15. `ScissorDisable`
16. `Callback`

**Boundary clarification:** `batch`, `unbatch`, and `set_screen` are not queue commands. They are enqueue/control operations and must be planned separately from the 16-command inventory.

**Files to modify:**
- `rust/src/graphics/dcq_ffi.rs`: Add 5 missing push functions
- `sc2/src/libs/graphics/sdl/rust_gfx.h`: Verify declarations exist against actual C callers/header declarations

---

### G5: DrawImage Missing Parameters (REQ-DQ-005, REQ-IMG-003, REQ-IMG-004, REQ-INT-008)

**Current state:** `rust_dcq_push_drawimage(image_id, x, y)` in `dcq_ffi.rs:286` hardcodes:
- `scale: 0` (should accept scale parameter)
- `scale_mode: ScaleMode::Nearest` (should accept mode parameter)
- `colormap: None` (should accept colormap index)
- `draw_mode: DrawMode::Normal` (should accept draw mode)

**Required state:** Per spec §5.1, Image command includes `image_ref, x, y, scale, scale_mode, colormap, draw_mode, dest`.

**Integration risk:** This is not just an FFI completeness issue. Missing parameters break context-driven draw compatibility because higher-level C graphics/context state cannot propagate draw mode, scale, or colormap choices into the Rust queue.

**Files to modify:**
- `rust/src/graphics/dcq_ffi.rs`: Expand `rust_dcq_push_drawimage` signature
- `sc2/src/libs/graphics/sdl/rust_gfx.h`: Update declaration to match exact C call sites
- `sc2/src/libs/graphics/tfb_draw.c`: Preserve all higher-level state when forwarding to Rust

---

### G6: SetPalette Stub (REQ-DQ-001, REQ-CMAP-003)

**Current state:** `rust_dcq_push_setpalette()` in `dcq_ffi.rs:505-526` doesn't enqueue a `SetPalette` command. It enqueues a `Callback` that logs the request.

**Also:** `DrawCommand` enum has no `SetPalette` variant — this is G13.

**Required state:** A proper `SetPalette` variant should exist in `DrawCommand`, and the push function should enqueue it.

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`: Add `SetPalette` variant to `DrawCommand` enum, handle in `handle_command()`
- `rust/src/graphics/dcq_ffi.rs`: Fix `rust_dcq_push_setpalette` to use real command

---

### G7: ReinitVideo No-Op (REQ-RL-011, REQ-INT-002)

**Current state:** `handle_command()` for `ReinitVideo` in `dcqueue.rs:837-847` only logs.

**Required state:** Per spec §10.4, ReinitVideo triggers full teardown and re-init. On failure, attempt reversion. On double-failure, exit process.

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`: Implement ReinitVideo handler
- `rust/src/graphics/ffi.rs`: Expose reinit path or call through existing `rust_gfx_uninit`/`rust_gfx_init`

---

### G8: C-Side Bridge Wiring and Migration Revalidation (REQ-INT-001, REQ-INT-004, REQ-INT-008)

**Current state:** Zero C call sites for `rust_canvas_*`, `rust_cmap_*`, or `rust_dcq_*`.

**Required state:** C `TFB_DrawScreen_*` functions should forward to `rust_dcq_push_*` when `USE_RUST_GFX` is defined. Colormap bridges should be called from the appropriate C graphics paths. Rust-local validation done before this phase must be treated as provisional for migration-sensitive semantics and then revalidated once real C call sites are live.

**Important boundary clarification from spec:** Draw dispatch redirected through Rust DCQ does **not** automatically require wiring `sc2/src/libs/graphics/sdl/canvas.c`. Canvas.c should only be touched if analysis identifies specific still-active C call sites that bypass the DCQ/canvas ownership transfer.

**Files to modify:**
- `sc2/src/libs/graphics/tfb_draw.c`: Add `#ifdef USE_RUST_GFX` branches in all `TFB_DrawScreen_*`
- `sc2/src/libs/graphics/dcqueue.c`: Add `#ifdef USE_RUST_GFX` to redirect `TFB_FlushGraphics` → `rust_dcq_flush()`
- `sc2/src/libs/graphics/sdl/sdl_common.c` and/or actual lifecycle owner: Wire colormap and DCQ init/uninit at the real graphics lifecycle boundary
- `sc2/src/libs/graphics/cmap.c`: Wire actual colormap operations that remain externally visible

---

### G9: System-Box Compositing (REQ-RL-012)

**Current state:** The plan previously assumed `rust_gfx_screen()` / `ffi.rs` owned the missing behavior, but the specification places system-box sequencing in the C `TFB_SwapBuffers` orchestration path.

**Required state:** Per spec §4.4 step 6, when the system box is active, the presentation sequence must re-composite `TFB_SCREEN_MAIN` at 255 alpha with the system-box clip rect after fade overlay and before postprocess.

**Files to modify:**
- `sc2/src/libs/graphics/sdl/sdl_common.c`: Verify or add the system-box `screen(MAIN, 255, &system_box_rect)` call at the correct point in the C orchestration path
- `rust/src/graphics/ffi.rs`: Verify `rust_gfx_screen()` already handles clipped compositing; change only if actual incompatibility is found

---

### G10: Bounding Box Tracking (REQ-DQ-012)

**Current state:** No bounding-box tracking during DCQ flush.

**Required state:** Track union of all main-screen pixels modified during flush. Reset after each cycle. May be internal-only optimization.

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`: Add bbox accumulation in `handle_command()` for Main screen commands

---

### G11: Rotated Image Object Compatibility, Not Just Pixel Rotation (REQ-IMG-007, REQ-IMG-008, REQ-OWN-002)

**Current state:** `TFB_DrawImage_New_Rotated` is not implemented. The earlier plan only described a low-level rotated-canvas helper and a speculative ID-based FFI export.

**Required state:** The plan must preserve the object-level contract for rotated `TFB_Image` creation:
- Identify where `TFB_DrawImage_New_Rotated(img, angle)` currently lives on the C side
- Identify the real ABI boundary where a rotated `TFB_Image*` is allocated/returned
- Preserve hotspot behavior, extent calculations, and lifecycle/destruction semantics for any derived canvases
- Preserve any cache/derived-field invalidation obligations that apply to the newly created rotated object

**Files to modify:**
- Rust implementation files for pixel rotation helper(s)
- The actual C/Rust image lifecycle boundary implementing `TFB_DrawImage_New_Rotated`
- `sc2/src/libs/graphics/sdl/rust_gfx.h` only if a concrete, caller-backed FFI export is proven necessary

---

### G12: Flush Completion Signal + Empty-Queue Handling (REQ-DQ-006, REQ-DQ-007, REQ-RL-008, REQ-INT-009)

**Current state:** `process_commands()` in `dcqueue.rs` does not:
- Broadcast `RenderingCond` after flush
- Handle empty-queue with active fade/transition

**Required state:** Per spec §4.3:
- Empty queue + active fade/transition → call `TFB_SwapBuffers` with `REDRAW_FADING`
- After all commands processed → broadcast condition variable
- Synchronization behavior must match existing UQM wait/signal usage on the migrated path

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`: Add completion signal and empty-queue redraw handling
- `rust/src/graphics/dcq_ffi.rs` and/or actual C bridge: ensure the established synchronization primitive is driven from the Rust path

---

### G13: Missing `SetPalette` DrawCommand Variant (REQ-DQ-001)

**Current state:** `DrawCommand` enum has no `SetPalette` variant.

**Required state:** The variant must exist so palette changes travel through the queue rather than a callback/logging stub.

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`

---

### G14: Batch Visibility and Nested Batching Not Explicitly Planned (REQ-DQ-003, REQ-DQ-004)

**Current state:** The plan previously assumed queue batching was already correct, but did not analyze or prove it. The requirements make both batch visibility and nested batching normative.

**Required state:** The migration plan must either prove existing Rust batching already satisfies the requirements or repair it. The plan must verify:
- Batched commands are not visible until unbatch exits the active batch scope
- Nested batching keeps commands hidden until the **outermost** unbatch
- The semantics still hold after C `TFB_BatchGraphics` / `TFB_UnbatchGraphics` are wired to Rust

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`: batch-depth / visibility behavior if needed
- `rust/src/graphics/dcq_ffi.rs`: batch/unbatch bridging if needed
- `sc2/src/libs/graphics/*`: actual C batch/unbatch call sites during wiring

---

### G15: Deferred Free Ordering and Image Metadata Synchronization Under-Covered (REQ-OWN-006, REQ-OWN-007)

**Current state:** The plan previously treated ownership coverage as mostly done, but two requirements were not concretely covered:
- Deferred free ordering relative to prior queued uses
- Per-image synchronization obligations when metadata is externally observed concurrently with rendering

**Required state:** The migrated C→Rust path must explicitly verify:
- `DeleteImage` / `DeleteData` occur only after all earlier queued uses complete
- The Rust path preserves any per-image mutex or equivalent synchronization guarantees required by the ABI-visible `TFB_Image` contract

**Files to modify:**
- `rust/src/graphics/dcqueue.rs`: deferred destruction ordering tests / fixes if needed
- Image lifecycle / metadata access layer: preserve or document mutex usage at the actual ABI boundary

---

### G16: Event-Pump Lifecycle and SDL Event Forwarding Revalidation Missing (REQ-RL-001, REQ-RL-011, REQ-INT-001, REQ-INT-002)

**Current state:** The requirements/spec treat SDL event collection/forwarding as part of graphics subsystem scope, but no phase explicitly revalidates `rust_gfx_process_events()` behavior across init, reinit, and uninit. Reinit work can replace event-pump state, yet the plan never names the active event-forwarding entry points or verifies post-reinit behavior.

**Required state:** The migrated path must explicitly verify:
- Event-pump ownership is initialized on the real Rust graphics lifecycle path
- `rust_gfx_process_events()` continues to collect and forward events in order without dropping them
- Event forwarding remains correct after `ReinitVideo`
- Event processing fails safely or no-ops according to the established external contract after uninit / before init

**Files to inspect and modify if required:**
- `rust/src/graphics/ffi.rs`: `rust_gfx_process_events()`, init/uninit/reinit helpers
- `rust/src/graphics/gfx_common.rs` and/or driver state: event-pump state ownership
- Real C lifecycle/orchestration site if event processing is invoked indirectly there

---

### G17: Backup Files (`tfb_draw.rs.bak`, `tfb_draw.rs.bak3`) Pollute Source Tree

**Current state:** Backup files remain in `rust/src/graphics/`.

**Required state:** Remove them once integration coverage is in place.

**Files to modify:**
- Delete backup artifacts in `rust/src/graphics/`

---

## Integration Touchpoints

| Rust module | C module | Integration type |
|-------------|----------|-----------------|
| `canvas_ffi.rs` | `ffi.rs`, transition/copy call sites | Canvas flush/sync before presentation/capture/interop reads |
| `dcq_ffi.rs` | `tfb_draw.c` | Draw command enqueueing replacement |
| `dcq_ffi.rs` | `dcqueue.c` | Flush replacement |
| `dcq_ffi.rs` | batch/unbatch/set_screen call sites | Queue visibility / destination-screen propagation |
| `cmap_ffi.rs` | actual lifecycle + `cmap.c` operations | Colormap lifecycle and behavior replacement |
| `ffi.rs` | `sdl_common.c` | Backend vtable already wired; system-box orchestration remains in C |
| `ffi.rs` / driver state | real event-processing caller(s) | SDL event-pump lifecycle and forwarding revalidation |
| image lifecycle Rust/C boundary | `TFB_DrawImage_New_Rotated` implementation site | Rotated-image object creation and destruction parity |

## Old Code to Replace/Remove

| File | What | Why |
|------|------|-----|
| `rust/src/graphics/tfb_draw.rs.bak` | Backup file | Stale artifact |
| `rust/src/graphics/tfb_draw.rs.bak3` | Backup file | Stale artifact |
| `ffi.rs:478-596` | Postprocess upload/scale fallback | Replaced by screen() compositing |
