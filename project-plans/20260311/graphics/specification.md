# Graphics Subsystem — Functional & Technical Specification

This document specifies the desired end state of the Rust graphics subsystem for UQM. It describes responsibilities, boundaries, public behavior, data models, draw pipeline semantics, and integration points. It is not an implementation plan.

### How to read this document

Statements in this specification fall into three categories:

- **Normative compatibility contract** — Externally observable behavior or ABI layout that must be preserved for correct integration with UQM. These are the binding requirements.
- **Intended ownership end state** — Describes which subsystem is intended to own a responsibility when migration is complete. These claims are conditional on preserving every normative contract.
- **Non-normative implementation note** — Describes a current or expected implementation approach. These notes are informational and do not constrain alternative implementations that preserve the normative contracts.

Sections 8–10 and other areas where implementation strategy appears are tagged with these labels where the distinction matters.

### Definition of "externally visible"

Throughout this document, "externally visible" means observable by any of the following:

- **ABI-visible:** Exposed through C struct layout, FFI function signatures, or header-declared contracts.
- **Behavior-visible:** Observable through gameplay rendering, test assertions, or visual output (e.g., pixel content, compositing order, fade/transition effects).
- **Surface-memory-visible:** Accessible to legacy code that reads raw `SDL_Surface*` pixel memory through the surface-pointer accessors.

When a statement says behavior must be "externally visible," it means all three categories unless explicitly narrowed.

### Ownership vocabulary

This document uses four distinct senses of "ownership." Each is used consistently and should not be conflated:

- **Behavioral ownership** — Accountability for correct externally visible results. The behaviorally owning subsystem is responsible for what users and tests observe, regardless of which code currently executes the work.
- **Resource ownership** — Responsibility for allocation and deallocation of a resource (surface, handle, buffer). The resource owner controls the lifetime.
- **Execution ownership** — Which code currently performs the work at runtime. During migration, execution ownership for a domain may remain in C even when behavioral ownership has been assigned to the Rust subsystem.
- **End-state ownership** — The intended final assignment of both behavioral and execution ownership after migration is complete.

Where only one sense applies, the text specifies which. Where a statement applies to the end state only, it is marked "(end-state)."

### Document authority and precedence

When this specification, the requirements document, current C code behavior, and current Rust code behavior conflict, the following precedence applies:

1. **Requirements** govern externally visible obligations. They define what must be true.
2. **This specification** refines and details those obligations, except where marked non-normative. Specification text shall not contradict requirements; if a conflict is found, the requirements govern until the conflict is resolved.
3. **Current C code** is authoritative only in areas where this document or the requirements explicitly delegate to code parity (e.g., loader behavior per §12.5 and `REQ-INT-012`). In those areas, the C code defines the behavioral contract until prose coverage replaces the delegation.
4. **Current Rust code** is evidence of implementation progress but is not authoritative. It may diverge from the specification without invalidating the specification.

---

## 1. Scope and Responsibilities

The following domains describe the **intended end-state ownership** of the Rust graphics subsystem. When migration is complete, the Rust subsystem is intended to hold both behavioral and execution ownership of externally visible behavior for each domain. During migration, the binding contract is behavioral compatibility — every ABI-visible and behavior-visible obligation listed in the requirements document must be preserved, regardless of whether execution ownership for a given domain currently resides in C, in Rust, or is split across both.

1. **SDL backend lifecycle** — SDL2 context initialization, window creation, renderer setup, event pump ownership, and orderly shutdown.
2. **Screen surface management** — Creation, resource ownership, and destruction of the three game screens (`Main`, `Extra`, `Transition`) and the format-conversion surface.
3. **Draw-command queue (DCQ)** — Enqueueing, batching, flushing, and dispatch of all draw commands. The DCQ is the single entry point through which game code submits drawing work.
4. **Canvas pixel operations** — All 2D drawing primitives (line, rect, fill, blit, image draw, font-char draw, copy) operating on screen or off-screen canvases.
5. **Image and font-char lifecycle** — Creation, scaling-cache management, mipmap association, rotation, deletion, and pixel intersection testing of `TFB_Image` and `TFB_Char` objects.
6. **Colormap and fade management** — Palette storage, colormap indexing, screen fade orchestration, and color-transform stepping.
7. **Presentation compositing** — Per-frame compositing of screen layers, transition overlays, and fade color overlays onto the renderer, followed by a single present call.
8. **Scaling for presentation** — Software upscaling (HQ2x, xBRZ 3×/4×) and bilinear/trilinear in-game sprite scaling, including scaled-buffer allocation and pixel-format conversion.
9. **Transition orchestration** — Source-screen capture, transition progression, and transition-screen compositing.
10. **Graphics asset loading** — Reading images and font data from UIO-backed resource files and converting them into the subsystem's internal image and canvas types. *Note:* End-state ownership of loading is aspirational. The prose contract for loader behavior is incomplete; a significant share of the loader contract is currently defined by code parity with the C loading path (see §12.5, §12.6, and `REQ-INT-012`).

### 1.1 Migration-sensitive compatibility domains

The following areas are especially sensitive during ownership transfer because they span the C/Rust boundary today. Each must preserve its full set of ABI-visible and behavior-visible contracts throughout migration. The end-state ownership listed in §1 does not change the binding obligation: behavioral compatibility and defined interop boundaries govern what is mandatory at any given point, not which language currently holds execution ownership for a domain.

- Draw queue semantics (FIFO ordering, batch visibility, signal delivery, callback execution context)
- `TFB_Image` layout, lifetime, and scaling-cache coherence
- `TFB_Char` alpha-map semantics and glyph-rendering contracts
- Screen-surface interoperability (raw `SDL_Surface*` access and pixel coherence)
- Transition behavior (source capture timing, clip interaction, compositing order)
- UIO asset-loading semantics (resource encoding, frame/glyph extraction, error behavior)
- Synchronization semantics (DCQ locking, condition-variable signaling, per-image mutex)

The subsystem shall **not** own:

- Game logic, scene management, or draw-call ordering decisions. Those remain in game code (C or Rust) which calls the public drawing API.
- Audio, input semantics, or filesystem abstractions.

**Event-pump boundary clarification:** The graphics subsystem owns SDL event collection and pump execution (`SDL_PollEvent` / `rust_gfx_process_events`). It is responsible for faithfully collecting and forwarding all SDL events without dropping, transforming, or reordering them. It is **not** responsible for interpreting those events as gameplay input, mapping keys to actions, or defining higher-level input semantics. Event interpretation, gameplay mapping, and higher-level input semantics are owned by the engine input layer, which is outside this 13-subsystem documentation set. If events are dropped or timing-shifted at the SDL pump level, that is a graphics conformance issue. If events are collected faithfully but interpreted incorrectly for gameplay purposes, that is an input-layer issue, not a graphics issue.
- The UIO virtual filesystem itself, though it shall consume UIO streams for asset loading.

---

## 2. Subsystem Boundaries

### 2.1 Ownership boundary

The graphics subsystem holds resource ownership of all SDL resources (context, video subsystem, window, renderer/canvas, surfaces, textures) and all Rust-side graphics objects (images, canvases, colormaps, DCQ state, scaler buffers).

Surface access from external code is governed by the following rules:

- **Permitted compatibility access:** External code may obtain raw `SDL_Surface*` pointers through the designated surface-pointer accessors for read-only inspection or interoperability purposes (e.g., reading surface dimensions, format, or pixel data for transition capture).
- **Forbidden direct mutation (end-state):** In the intended end state, no external code shall directly write pixels into subsystem-owned surfaces, directly create or free SDL resources, or call SDL rendering functions when the Rust backend is enabled. All pixel mutations shall flow through the canvas FFI or DCQ.
- **Transitional exceptions:** During migration, C code may continue to write directly into Rust-owned `SDL_Surface` pixel memory through the existing C draw path. This exception is scoped to the current partial-port boundary where C still holds execution ownership of draw-command dispatch. The subsystem shall preserve pixel coherence across this boundary (see §6.2). As each C draw path is replaced and execution ownership transfers to Rust, the corresponding direct-mutation permission is retired.

### 2.2 C interop boundary

The C/Rust interop boundary currently consists of three tiers, reflecting the migration state. All three tiers are valid interop paths; the distinction is which are currently active versus prepared for future activation.

**Active interop paths (currently wired):**

- The **backend vtable** (`TFB_GRAPHICS_BACKEND`) function pointers, which forward to Rust FFI exports. C code calls these through `TFB_SwapBuffers` to perform presentation compositing.
- **Surface pointer accessors** (`rust_gfx_get_screen_surface`, etc.) that return `SDL_Surface*` pointers. C code retrieves these after Rust init and uses them as draw targets throughout the frame.
- **Init/uninit and auxiliary functions** (`rust_gfx_init`, `rust_gfx_uninit`, `rust_gfx_process_events`, etc.).

**Allowed transitional paths (declared but not yet called from C):**

- The **DCQ FFI** (`rust_dcq_*` functions) for submitting draw commands.
- The **canvas FFI** (`rust_canvas_*` functions) for canvas handle operations.
- The **colormap FFI** (`rust_cmap_*` functions) for palette and fade operations.

These bridges are implemented in Rust and declared in `rust_gfx.h`, but no C call sites currently invoke them. They are prepared for future wiring as execution ownership transfers.

**Intended end-state interop:**

When migration is complete, C code shall interact with the graphics subsystem exclusively through the backend vtable, DCQ FFI, canvas FFI, and colormap FFI. Surface-pointer accessors shall be retained only if compatibility requirements still necessitate raw `SDL_Surface*` access at that point; they are not an affirmative part of the intended end-state interaction surface. At that point, C code shall not directly call `SDL_CreateRGBSurface`, `SDL_FreeSurface`, or any SDL rendering functions. Compatibility shim layers in C may participate as thin forwarding wrappers, but behavioral ownership of all externally visible graphics behavior shall rest with the Rust subsystem.

### 2.3 Operation categories relative to the draw queue

The following table defines which categories of operations must use the draw-command queue and which are intentionally out of band:

| Category | Queue relationship | Rationale |
|----------|-------------------|-----------|
| Deferred render mutations (line, rect, image, font, fill, copy) | **Queue-mandatory.** Must enter through the DCQ. | Preserves FIFO ordering and thread-safe producer/consumer model. |
| Immediate compatibility reads (surface dimensions, pixel format, raw pixel inspection for transition capture) | **Out of band.** Read-only access through surface-pointer accessors. Such reads observe the current committed surface state only — that is, pixels produced by completed flush work and any permitted direct writes already applied to surface memory — and never imply visibility of queued-but-unflushed draw commands. | Read-only observation does not mutate render state; queuing would add unnecessary latency. |
| Transitional direct writes by C | **Temporarily out of band.** C draw paths write directly into surface pixel memory during migration. | These are the active C execution paths that have not yet been replaced. Not a permanent architectural permission. |
| Control and lifecycle operations (init, uninit, reinit, set-screen, batch/unbatch, set-palette) | **Varies by operation.** Reinit, set-palette, and signal are DCQ-mediated. Init/uninit are direct lifecycle calls. Batch/unbatch and set-screen are enqueue-side state changes. | Each follows the existing UQM contract for that operation. |
| Transition-source capture (`SetTransitionSource`) | **Out of band.** Immediate read of current main-screen surface pixel memory. | Capture semantics require reading already-flushed pixels at the point of call; queuing would violate the capture-timing contract (see §4.5). |

### 2.4 Threading model

The graphics subsystem API is divided into the following thread-safety classes:

| API class | Thread constraint | Examples |
|-----------|------------------|----------|
| Enqueue-only APIs | Callable from any game thread; must be thread-safe (commands are pushed atomically) | `rust_dcq_push_*`, `TFB_DrawScreen_*` |
| Flush, presentation, and backend APIs | Graphics/rendering thread only; no internal synchronization | `TFB_FlushGraphics`, `TFB_SwapBuffers`, vtable functions (`preprocess`, `postprocess`, `screen`, `color`) |
| Image metadata access | Protected by per-image mutex (`TFB_Image.mutex`) where game threads may read concurrently with rendering | `TFB_Image` field reads |

This is consistent with UQM's existing model where game threads produce draw commands and the single graphics thread consumes and presents them.

---

## 3. Data Model and ABI-Visible Expectations

### 3.1 Screen enumeration

The subsystem shall recognize exactly three screens:

| Index | Name | Purpose |
|-------|------|---------|
| 0 | `TFB_SCREEN_MAIN` | Primary game display surface |
| 1 | `TFB_SCREEN_EXTRA` | Off-screen scratch buffer (not composited to display) |
| 2 | `TFB_SCREEN_TRANSITION` | Holds the previous frame for transition effects |

The constant `TFB_GFX_NUMSCREENS` = 3 is ABI-fixed.

### 3.2 Screen surfaces

Each screen shall be backed by a 320×240 pixel `SDL_Surface` in RGBX8888 format (32-bit, no alpha channel). Byte-level layout on little-endian platforms:

| Byte 0 | Byte 1 | Byte 2 | Byte 3 |
|--------|--------|--------|--------|
| X (pad) | B | G | R |

Channel masks: `R=0xFF000000`, `G=0x00FF0000`, `B=0x0000FF00`, `A=0x00000000`.

**Normative compatibility contract (conditional):** These exact masks are normative because external C code currently accesses raw `SDL_Surface*` pixel memory through the surface-pointer accessors and depends on the pixel layout for direct reads. This constraint is classified as a **current compatibility requirement due to raw external access**: it applies for as long as any external code path reads raw surface pixels through the surface-pointer accessors. If direct surface access is fully retired and no external caller inspects raw pixel memory, the internal format may vary so long as all externally visible rendering behavior remains identical.

A fourth surface (`format_conv_surf`) shall use the same pixel format but with `A=0x000000FF` (alpha-enabled) and shall be allocated at 0×0 dimensions for use as a format-conversion template by canvas operations requiring alpha.

### 3.3 TFB_Image

`TFB_Image` is the primary image resource type. Its C-visible layout shall remain ABI-compatible:

```
struct TFB_Image {
    TFB_Canvas NormalImg;       // unscaled image canvas
    TFB_Canvas ScaledImg;       // cached scaled variant
    TFB_Canvas MipmapImg;       // mipmap for trilinear scaling
    TFB_Canvas FilledImg;       // monochrome-fill variant
    int colormap_index;         // active colormap index
    int colormap_version;       // tracks colormap changes for cache invalidation
    HOT_SPOT NormalHs;          // hot-spot for unscaled image
    HOT_SPOT MipmapHs;          // hot-spot for mipmap
    HOT_SPOT last_scale_hs;     // hot-spot of most recent scale
    int last_scale;             // most recent scale factor
    int last_scale_type;        // most recent scale algorithm
    Color last_fill;            // most recent fill color
    EXTENT extent;              // original image dimensions
    Mutex mutex;                // per-image lock
    BOOLEAN dirty;              // invalidation flag
};
```

`TFB_Canvas` is typedef'd as `void *`. It represents an opaque handle to an SDL_Surface managed by the canvas subsystem. The Rust graphics subsystem shall hold resource ownership of all `TFB_Canvas` instances: it is responsible for their creation and deletion.

### 3.4 TFB_Char

`TFB_Char` represents a single font glyph:

```
struct TFB_Char {
    EXTENT extent;      // glyph pixel dimensions
    EXTENT disp;        // display extent (advance width/height)
    HOT_SPOT HotSpot;   // rendering origin offset
    BYTE *data;         // alpha-only bitmap (1 byte per pixel)
    DWORD pitch;        // row stride in bytes
};
```

The `data` field points to an alpha-channel-only bitmap. Each byte represents opacity (0 = transparent, 255 = fully opaque). The graphics subsystem shall render glyphs by applying a foreground color modulated by this alpha map.

### 3.5 TFB_ColorMap

Colormaps are indexed palettes of 256 RGB triplets (768 bytes each). The subsystem shall support up to `MAX_COLORMAPS` (250) simultaneously loaded colormaps. Each colormap carries:

- An integer `index` (0–249).
- A `version` counter that increments on any modification, enabling image caching to detect stale palette data.
- A reference count for safe sharing.

### 3.6 Color encoding at the FFI boundary

Colors passed through FFI functions shall be packed as `u32` in RGBA byte order matching the C-side mask convention:

| Bits 31–24 | Bits 23–16 | Bits 15–8 | Bits 7–0 |
|------------|------------|-----------|----------|
| R | G | B | A |

### 3.7 Coordinate system

All coordinates are in logical game pixels (320×240 space). The origin is at the top-left corner. X increases rightward, Y increases downward. Coordinates may be negative (off-screen), and drawing operations shall clip to canvas bounds.

### 3.8 GFX flags

The following flag bits control graphics behavior and are ABI-fixed:

| Bit | Flag | Meaning |
|-----|------|---------|
| 0 | `FULLSCREEN` | Fullscreen mode |
| 1 | `SHOWFPS` | FPS counter overlay |
| 2 | `SCANLINES` | Scanline effect on presentation |
| 3 | `SCALE_BILINEAR` | Bilinear sprite scaling |
| 4 | `SCALE_BIADAPT` | Bi-adaptive scaling |
| 5 | `SCALE_BIADAPTADV` | Advanced bi-adaptive scaling |
| 6 | `SCALE_TRISCAN` | Triscan scaling |
| 7 | `SCALE_HQXX` | HQ2x software scaling |
| 8 | `SCALE_XBRZ3` | xBRZ 3× software scaling |
| 9 | `SCALE_XBRZ4` | xBRZ 4× software scaling |

---

## 4. Draw Pipeline

### 4.1 Overview

The draw pipeline has two phases: **enqueueing** (callable from game threads) and **flushing** (graphics thread only).

```
Game threads ─── enqueue draw commands ──→ DCQ
                                           │
Graphics thread ─── TFB_FlushGraphics ─────┘
                         │
              ┌──────────┴──────────┐
              │  Pop & dispatch     │
              │  each command to    │
              │  canvas operations  │
              └──────────┬──────────┘
                         │
              ┌──────────┴──────────┐
              │  TFB_SwapBuffers    │
              │  (compositing +     │
              │   presentation)     │
              └─────────────────────┘
```

### 4.2 Enqueueing

Game code calls `TFB_DrawScreen_*` wrappers (or their Rust FFI equivalents `rust_dcq_push_*`), which construct `DrawCommand` variants and push them onto the DCQ. Enqueueing shall:

- Be callable from any game thread (commands are pushed atomically). This is the multi-producer entry point to the pipeline.
- Associate each command with a target screen (Main, Extra, or Transition).
- Support batch mode: nested `batch`/`unbatch` calls control visibility. While batched, commands accumulate but are not visible to the flush loop until all batch levels are exited.

### 4.3 Flushing

`TFB_FlushGraphics` is called from the graphics thread once per frame. It shall:

1. Check if the queue is empty. If so and no fade/transition is active, yield and return.
2. If the queue is empty but a fade or transition is active, call `TFB_SwapBuffers` with `REDRAW_FADING` to keep the presentation updated.
3. If commands are present, reset the bounding box tracker, then pop and dispatch commands in FIFO order.
4. Apply livelock deterrence: if the queue grows beyond a threshold during processing, acquire the DCQ lock to prevent game threads from adding more commands.
5. After all commands are processed, call `TFB_SwapBuffers(REDRAW_NO)` to present the frame.
6. Broadcast the rendering condition variable to unblock any game threads waiting on draw completion.

### 4.4 TFB_SwapBuffers (compositing and presentation)

`TFB_SwapBuffers` orchestrates the per-frame compositing sequence through the backend vtable:

1. **Determine redraw necessity.** Compare current fade and transition amounts against their last-frame values. Skip if nothing has changed and no forced redraw is requested.
2. **`preprocess(force_redraw, transition_amount, fade_amount)`** — Clear the renderer to opaque black. Reset blend mode to `None`. The `transition_amount` and `fade_amount` parameters are informational; actual compositing uses them in subsequent calls.
3. **`screen(TFB_SCREEN_MAIN, 255, NULL)`** — Composite the main screen at full opacity, full-screen.
4. **Transition overlay** (if `transition_amount ≠ 255`):
   - `screen(TFB_SCREEN_TRANSITION, 255 - transition_amount, &clip_rect)` — Composite the transition screen at the inverse fade amount, clipped to the transition rectangle.
5. **Fade overlay** (if `fade_amount ≠ 255`):
   - If `fade_amount < 255`: `color(0, 0, 0, 255 - fade_amount, NULL)` — Black overlay with proportional opacity.
   - If `fade_amount > 255`: `color(255, 255, 255, fade_amount - 255, NULL)` — White overlay with proportional opacity.
6. **System box** (if active): `screen(TFB_SCREEN_MAIN, 255, &system_box)` — Re-composite a subregion of the main screen on top to keep system UI visible through fades. This step is required whenever the system box is active, regardless of fade state.
7. **`postprocess()`** — Apply optional scanline effects, then present the final frame to the display.

### 4.5 Transition contract

The transition mechanism shall preserve the following observable behavior:

- **Source capture timing:** Transition source pixels are captured from the main screen (`TFB_SCREEN_MAIN`) at the time `SetTransitionSource` is called. The captured content shall reflect all draw commands that have been flushed to the main screen's surface pixel memory before capture occurs. Queued commands that have not yet been flushed are not part of the capture; only already-flushed pixels are read.
- **Capture ordering relative to flush:** Transition-source capture is defined as an immediate read of current main-screen surface pixel memory at the point of capture. If capture occurs between flushes, the content is whatever the last completed flush (plus any direct surface writes during the transitional migration period) left in main-screen pixel memory. The subsystem is not required to force a flush before capture; the ordering contract is: flushed-then-captured content is stable, unflushed content is excluded.
- **Capture synchronization in mixed-ownership operation:** During migration, both C draw paths and Rust canvas paths may write into main-screen surface pixel memory. The capture operation reads surface pixel memory without additional synchronization beyond the existing UQM threading model (all draw-command dispatch and capture occur on the graphics thread). This means capture is serialized with flush by virtue of single-threaded graphics-thread execution; no additional cross-thread synchronization is required for capture correctness. If a future architecture introduces concurrent surface writers, an explicit serialization point shall be required before capture.
- **Capture scope:** The capture copies a specified rectangular region of the main screen to the corresponding region of `TFB_SCREEN_TRANSITION`. The copied pixels represent already-flushed main-screen content only.
- **Clip interaction:** The transition clip rectangle defines the region where the transition screen is composited during presentation. When software scaling is active, the source rectangle supplied to `screen()` is multiplied by the scale factor; the destination rectangle remains in logical coordinates.
- **Snapshot semantics:** Once captured, the transition screen's content is stable for the duration of the transition effect. The compositing layer reads directly from `TFB_SCREEN_TRANSITION`'s surface pixel memory during each presentation frame.
- **Compositing order interaction:** The transition overlay is composited after the main screen and before the fade overlay. The system-box re-composite occurs after the fade overlay. This means the system box is always visible through both transitions and fades.

---

## 5. Draw Queue Semantics

### 5.1 Command types

The DCQ shall support the following command types:

| Command | Parameters | Behavior |
|---------|-----------|----------|
| `Line` | x1, y1, x2, y2, color, draw_mode, dest | Draw a line using Bresenham's algorithm |
| `Rect` | rect, color, draw_mode, dest | Draw a rectangle outline or filled rectangle |
| `Image` | image_ref, x, y, scale, scale_mode, colormap, draw_mode, dest | Draw a sprite image with optional scaling and palette |
| `FilledImage` | image_ref, x, y, scale, scale_mode, color, draw_mode, dest | Draw a monochrome-filled sprite |
| `FontChar` | char_ref, backing_image, x, y, draw_mode, dest | Render a font glyph with alpha blending |
| `Copy` | src_rect, src_screen, dest_screen | Blit pixels between screens |
| `CopyToImage` | image_ref, src_rect, src_screen | Capture screen pixels into an image |
| `SetMipmap` | image_ref, mipmap_ref, hot_x, hot_y | Associate a mipmap with an image |
| `DeleteImage` | image_ref | Destroy an image and free its resources |
| `DeleteData` | data_ptr | Free an arbitrary heap allocation |
| `SendSignal` | semaphore/atomic | Signal a waiting game thread |
| `ReinitVideo` | driver, flags, width, height | Reinitialize the video subsystem |
| `SetPalette` | colormap_id | Activate a colormap for subsequent draws |
| `ScissorEnable` | rect | Set a clipping rectangle on the current screen |
| `ScissorDisable` | — | Clear the clipping rectangle |
| `Callback` | fn_ptr, arg | Execute an arbitrary callback |

### 5.2 Draw modes

| Mode | Behavior |
|------|----------|
| `Normal` / `Replace` | Overwrite destination pixels |
| `Blended` | Alpha-blend source over destination |

### 5.3 Queue properties

- **FIFO ordering**: Commands are processed in the order they were enqueued.
- **Bounded size**: The queue has a configurable capacity. *Non-normative implementation note:* The current implementation uses `DCQ_FORCE_BREAK_SIZE` and `DCQ_FORCE_SLOWDOWN_SIZE` thresholds. The normative requirement is that the subsystem shall guarantee forward progress by applying backpressure when queue growth would otherwise prevent flush completion (see `REQ-DQ-008`).
- **Batch nesting**: `batch()` increments a depth counter. `unbatch()` decrements it. Commands are visible to the consumer only when depth reaches 0.
- **Screen binding**: Each push operation tags the command with the currently selected target screen. `set_screen(index)` changes the target.

### 5.4 Bounding box tracking

During flush, the subsystem shall track a bounding box of all pixels modified on `TFB_SCREEN_MAIN`. The bounding box is an internal optimization aid. If any external integration code consumes the tracked region, the bounding box shall be a correct superset (union) of all main-screen pixels modified during that flush cycle; false positives (a bounding box larger than the precise modified region) are acceptable. The bounding box is reset after each flush cycle. If no external code depends on the tracked region, the bounding box may be treated as an internal-only optimization.

---

## 6. Canvas and Image Behavior

### 6.1 Canvas operations

A canvas is an opaque drawing surface backed by pixel memory. The subsystem shall provide:

- **`canvas_from_surface(SDL_Surface*) → SurfaceCanvas*`** — Create a canvas handle wrapping an existing SDL surface. The canvas reads the surface's dimensions and pixel format at creation time and provides Rust drawing operations over it.
- **`canvas_destroy(SurfaceCanvas*)`** — Free the Rust-side handle. Does not free the underlying SDL surface.
- **Primitive drawing** — `draw_line`, `draw_rect`, `fill_rect` with packed RGBA color and mode.
- **Canvas-to-canvas copy** — `copy(dst, src, src_rect, dst_x, dst_y)` with optional sub-rectangle.
- **Image blit** — `draw_image(canvas, image_data, w, h, x, y)` from raw RGBA pixel data.
- **Font glyph rendering** — `draw_fontchar(canvas, glyph_data, w, h, x, y, color)` with per-pixel alpha blending from the glyph's alpha bitmap.
- **Scissor (clipping)** — `set_scissor(canvas, x, y, w, h)` and `clear_scissor(canvas)`. All subsequent draw operations on the canvas are clipped to the scissor rectangle.
- **Query** — `get_extent(canvas) → (width, height)`.

### 6.2 Canvas pixel synchronization

**Normative contract:** When the canvas FFI replaces direct SDL surface drawing, the `SurfaceCanvas` must preserve pixel coherence between its drawing operations and the underlying `SDL_Surface` pixel memory that the presentation layer reads during compositing. Specifically:

- A canvas wrapping a surface shall not discard existing surface pixel content on creation.
- Pixels modified through the canvas shall be visible in the underlying surface's pixel memory at each of the following synchronization points:
  - Before presentation compositing reads the surface for screen-layer upload.
  - Before transition-source capture reads the surface (see §4.5).
  - Before any interoperability read that returns current pixel data to external code (e.g., screen-to-image copy, direct surface inspection through surface-pointer accessors).

*Non-normative implementation note:* The expected approach is to import surface pixel data on creation and write modified pixels back on flush or destroy. Alternative synchronization strategies (e.g., drawing directly into the surface's pixel buffer, lazy synchronization triggered by presentation, or shared-memory mapping) are acceptable provided the coherence contract is preserved at every identified synchronization point.

### 6.3 Image lifecycle

| Operation | Behavior |
|-----------|----------|
| `TFB_DrawImage_New(canvas)` | Wrap a canvas in a new `TFB_Image`, initializing all cached fields to defaults |
| `TFB_DrawImage_CreateForScreen(w, h, alpha)` | Create a new image with a fresh canvas in screen pixel format |
| `TFB_DrawImage_New_Rotated(img, angle)` | Create a rotated copy |
| `TFB_DrawImage_SetMipmap(img, mipmap, hx, hy)` | Associate a mipmap image for trilinear scaling |
| `TFB_DrawImage_Delete(img)` | Free all canvases (normal, scaled, mipmap, filled) and the image struct |
| `TFB_DrawImage_FixScaling(img, target, type)` | Invalidate the scaling cache if the target scale or algorithm changed |

### 6.4 Image scaling cache

Each `TFB_Image` maintains a cached `ScaledImg` canvas. When an image draw request specifies a scale factor and algorithm:

1. If `last_scale == target_scale` and `last_scale_type == target_type` and `colormap_version` matches the current colormap version and no fill-related state has changed since the last scale, reuse the cached `ScaledImg`.
2. Otherwise, create a new `ScaledImg` canvas at the target dimensions and apply the requested scaling algorithm.
3. Update `last_scale`, `last_scale_type`, `last_scale_hs`.

All cache-affecting inputs identified in `REQ-IMG-004` — scale factor, scaling algorithm, colormap version, and fill-related state — shall be checked before reuse.

Supported scaling algorithms:

| Algorithm | Enum | Behavior |
|-----------|------|----------|
| Step | `TFB_SCALE_STEP` | No interpolation (pixel snap) |
| Nearest | `TFB_SCALE_NEAREST` | Nearest-neighbor sampling |
| Bilinear | `TFB_SCALE_BILINEAR` | Bilinear interpolation |
| Trilinear | `TFB_SCALE_TRILINEAR` | Bilinear + mipmap blending |

### 6.5 Font rendering

Font glyphs (`TFB_Char`) are rendered using alpha-blended painting:

1. For each pixel in the glyph bitmap, read the alpha value.
2. If `alpha == 0`, skip.
3. Compute effective alpha: `glyph_alpha × foreground_alpha / 255`.
4. Blend: `dst_channel = (fg_channel × eff_alpha + dst_channel × (255 - eff_alpha)) / 255`.

The glyph's `HotSpot` specifies the offset from the draw position to the top-left of the glyph bitmap. The `disp` extent specifies the advance width for text layout.

Font glyphs may be accompanied by a backing `TFB_Image`. When present, the backing image is composited first: the backing image's canvas content is drawn at the glyph position (with the same hot-spot/origin), and then the glyph alpha map is blended on top using the foreground color. Clipping applies identically to both the backing-image draw and the glyph draw.

---

## 7. Colormap and Fade Behavior

### 7.1 Colormap management

- The colormap manager is a singleton initialized during graphics init and destroyed during uninit.
- It stores up to 250 colormaps, each containing 256 RGB triplets (768 bytes).
- `set_colors(index, end_index, data)` writes one or more colormaps from a contiguous byte buffer.
- `get_colormap(index)` returns a reference-counted colormap handle. The caller must return it via `return_colormap()`.
- Each colormap tracks a `version` integer that increments on every modification.

### 7.2 Fade model

Fade intensity is a single integer value with the following semantics:

| Value | Meaning |
|-------|---------|
| 0 (`FADE_NO_INTENSITY`) | Fully black |
| 255 (`FADE_NORMAL_INTENSITY`) | Normal display (no fade) |
| 510 (`FADE_FULL_INTENSITY`) | Fully white |

Fade operations:

- `fade_screen(direction, duration_ms)` — Initiate a fade. Direction 0 = fade to black, 1 = fade to normal, 2 = fade to white. If `duration_ms == 0`, the fade is instantaneous.
- `get_fade_amount()` — Return the current fade intensity.
- `xform_step()` — Advance all active color transformations by one step. Returns whether any transformations are still in progress.
- `flush_xforms()` — Immediately complete all in-progress fades and transformations.

### 7.3 Fade compositing

The presentation layer reads `get_fade_amount()` each frame and applies a color overlay:

- If `fade_amount < 255`: black overlay with alpha `(255 - fade_amount)`.
- If `fade_amount > 255`: white overlay with alpha `(fade_amount - 255)`.
- If `fade_amount == 255`: no overlay.

This overlay is composited after the main and transition screen layers but before postprocess/present.

---

## 8. Scaling and Presentation Behavior

### 8.1 Logical vs. physical resolution

The game operates at a fixed logical resolution of **320×240 pixels**. The physical window may be any size. The renderer's logical-size setting maps logical coordinates to the physical display with letterboxing/pillarboxing as needed.

### 8.2 Presentation-time software scaling

**Normative contract:** When a software scaler flag is active (HQ2x, xBRZ 3×, xBRZ 4×) and the bilinear-only flag is not the sole scaler, the subsystem shall scale visible screen-layer content by the corresponding factor before presentation. When a clipping rect is provided, the source rectangle shall be scaled by the factor while the destination rectangle remains in logical coordinates. The RGBX↔RGBA pixel-format conversions shall preserve color channel values: `[X, B, G, R]` ↔ `[R, G, B, 0xFF]`.

*Non-normative implementation note:* The current approach allocates per-screen scaled buffers of size `(320 × factor) × (240 × factor) × 4` bytes during init and runs the scaler during `screen()` compositing. Alternative implementations (e.g., GPU-based scaling or deferred scaling) are acceptable provided the normative contract is preserved.

### 8.3 Sprite-level scaling

In-game sprite scaling (for zoom effects, etc.) uses the `TFB_Image` scaling cache (§6.4). The scale factor is expressed as an integer where `GSCALE_IDENTITY` (256) means 1:1. Scaling algorithms are selected per-draw-command.

### 8.4 Scanline effect

When the `SCANLINES` flag is set, the postprocess step shall apply a scanline overlay effect before presenting. This effect visually dims alternating horizontal lines to simulate CRT scanlines. Exact scanline intensity and phase are compatibility-relevant only to the extent that the visual result is consistent with the C reference backend's scanline implementation; visually equivalent behavior is acceptable.

### 8.5 Texture strategy

**Normative contract:** The subsystem shall ensure that current screen pixel data is available to the renderer for compositing each frame. The observable contract is: one final display present per frame, correct compositing order, and correct pixel content.

*Non-normative implementation note:* The current expected approach uses per-frame streaming textures — a streaming texture is created, populated, rendered, and destroyed within the scope of a single `screen()` call. This avoids persistent texture caching complexity. Persistent textures with dirty-rectangle tracking are an acceptable alternative optimization.

---

## 9. Backend Integration

### 9.1 Backend vtable

The graphics backend is defined by a vtable struct with five function pointers:

```
struct TFB_GRAPHICS_BACKEND {
    void (*preprocess)(int force_redraw, int transition_amount, int fade_amount);
    void (*postprocess)(void);
    void (*uploadTransitionScreen)(void);
    void (*screen)(SCREEN screen, Uint8 alpha, SDL_Rect *rect);
    void (*color)(Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect);
};
```

When `USE_RUST_GFX` is defined, the active backend shall point to thin C wrapper functions that forward to `rust_gfx_*` FFI exports.

### 9.2 Vtable function contracts

**`preprocess(force_redraw, transition_amount, fade_amount)`**
- Reset the renderer blend mode to `None`.
- Clear the renderer to opaque black.
- The `transition_amount` and `fade_amount` parameters are informational context. The actual fade/transition compositing occurs in `screen()` and `color()`.

**`postprocess()`**
- Apply optional scanline effect if `SCANLINES` flag is set.
- Call `renderer.present()` to display the composed frame.
- Shall not perform any texture upload or surface-to-renderer copy. All compositing must be complete before postprocess is called.

**`uploadTransitionScreen()`**
- *Non-normative implementation note:* The Rust backend currently implements this as a no-op because the `screen()` function unconditionally reads from `TFB_SCREEN_TRANSITION`'s surface pixel data each frame, making a separate upload step unnecessary. The normative contract is that transition-screen content must be correctly composited during presentation (see §4.5); the mechanism by which the pixel data reaches the renderer is an implementation choice.

**`screen(screen_index, alpha, rect)`**

**Normative contract:** The `screen()` function shall composite the specified screen's current pixel content onto the renderer at the requested alpha and clip region. The composited output shall reflect the screen surface's current pixel data, correctly scaled if a software scaler is active. `TFB_SCREEN_EXTRA` (index 1) shall not be composited to the display. The function shall return immediately if the screen index is out of range, the state is uninitialized, or the surface pointer is null. If `rect` is non-null, compositing shall be limited to that rectangle (with the source rectangle scaled by the scale factor when software scaling is active, and the destination rectangle in logical coordinates). If `rect` is null, the full screen shall be composited.

*Non-normative implementation note:* The expected approach reads pixel data from `surfaces[screen_index]`, uploads it to a texture (at scaled dimensions if a software scaler is active, otherwise at 320×240), sets blend mode based on the alpha value, and renders the texture. Per-call texture creation/destruction, persistent texture caching, and other upload strategies are all acceptable provided the normative contract is met.

**`color(r, g, b, a, rect)`**
- Set the renderer draw color to (r, g, b, a).
- Set blend mode: `None` if a == 255, `Blend` otherwise.
- Fill the specified rectangle (or the entire screen if `rect` is null).
- Return immediately if the state is uninitialized or if the rect has negative dimensions.

### 9.3 Surface pointer accessors

The following FFI functions return raw `SDL_Surface*` pointers for interoperability:

| Function | Returns |
|----------|---------|
| `rust_gfx_get_sdl_screen()` | `surfaces[0]` (main screen) |
| `rust_gfx_get_transition_screen()` | `surfaces[2]` |
| `rust_gfx_get_screen_surface(i)` | `surfaces[i]` with range check |
| `rust_gfx_get_format_conv_surf()` | format conversion surface |

These shall return null if the subsystem is not initialized or the index is out of range.

### 9.4 Auxiliary backend functions

| Function | Behavior |
|----------|----------|
| `rust_gfx_process_events()` | Poll the SDL event queue, return 1 if quit requested |
| `rust_gfx_set_gamma(gamma)` | Set display gamma (may return -1 if unsupported) |
| `rust_gfx_toggle_fullscreen()` | Toggle fullscreen mode, return the new state |
| `rust_gfx_is_fullscreen()` | Return current fullscreen state |
| `rust_gfx_get_width()` / `get_height()` | Return the logical screen dimensions (320, 240) |

---

## 10. Lifecycle

### 10.1 Initialization sequence

**Normative contract:** `rust_gfx_init(driver, flags, renderer, width, height)` shall initialize the rendering backend, create the game window, create three RGBX8888 screen surfaces and the format-conversion surface, and prepare any scaling resources required by the active flags. It shall guard against double-init (return -1). On partial failure, it shall clean up all previously created resources and return -1. On success, it shall return 0.

After Rust init returns, C code retrieves surface pointers and stores them in the `SDL_Screens[]` and `SDL_Screen` / `TransitionScreen` globals.

*Non-normative implementation note:* The current approach initializes SDL2, creates a software renderer with vsync and nearest-neighbor scaling hint, sets logical size to 320×240, and allocates per-screen scaled pixel buffers when a non-bilinear software scaler is active.

### 10.2 Per-frame cycle

1. Game threads enqueue draw commands via `TFB_DrawScreen_*` / `rust_dcq_push_*`.
2. Graphics thread calls `TFB_FlushGraphics()`:
   - Pops and dispatches all queued commands against screen canvases.
   - Calls `TFB_SwapBuffers()` → `preprocess` → `screen` × N → `color` (if fading) → `postprocess`.
3. `postprocess` presents the frame.

### 10.3 Shutdown sequence

`rust_gfx_uninit()` shall:

1. Drop per-screen scaled buffers.
2. Free all three screen surfaces and the format-conversion surface via `SDL_FreeSurface` (while SDL is still initialized).
3. Drop the renderer/canvas (which destroys the window).
4. Drop the video subsystem.
5. Drop the SDL context.

The colormap and DCQ subsystems shall be uninitialized before or during `rust_gfx_uninit()`:

- `rust_cmap_uninit()` — calls `manager.uninit()` and drops the singleton.
- `rust_dcq_uninit()` — clears the queue and drops the singleton.

### 10.4 Reinitializing video

The `ReinitVideo` draw command triggers a full teardown and re-initialization of the video backend within the flush loop. If reinitialization fails, the subsystem shall attempt to revert to the previous driver/flags/dimensions. If reversion also fails, the subsystem shall exit the process. This exit-on-irrecoverable-failure behavior is inherited from the C reference implementation (the existing C backend also exits on double-failure) and is preserved as a compatibility requirement.

---

## 11. Error Handling

### 11.1 FFI safety guarantee

**Normative contract:** All FFI-exported functions shall not permit unwinding or equivalent unchecked exception propagation across the language boundary. All fallible operations shall use safe error propagation with early return of error codes. The only non-recoverable exception source is out-of-memory during string formatting, which is not meaningfully catchable.

*Non-normative implementation note:* In Rust implementations, this typically means avoiding `.unwrap()`, `.expect()`, or panicking indexing in FFI functions, using `match`/`if let`/`Result` propagation instead, and optionally wrapping FFI bodies in `catch_unwind` as defense-in-depth. Other languages or approaches are acceptable provided the no-unwind contract is preserved.

### 11.2 Error return conventions

| Return type | Success | Error |
|-------------|---------|-------|
| `c_int` (init/push/set) | 0 | -1 |
| `*mut T` (pointer returns) | valid pointer | null |
| `*const u8` (data returns) | valid pointer | null |
| void functions | — | Silent no-op on error (log and return) |

### 11.3 Uninitialized-state safety

All functions that access singleton state (graphics, DCQ, colormap) shall check for initialization before proceeding. If the state is not initialized:

- Pointer-returning functions return null.
- Integer-returning functions return -1 or a safe default (e.g., `FADE_NORMAL_INTENSITY` for `get_fade_amount`).
- Void functions return immediately.

This ensures that calling any graphics function before init or after uninit is a no-op rather than undefined behavior.

### 11.4 Null pointer safety

All functions accepting pointers shall check for null before dereferencing:

- `SDL_Surface*` — return null/error if null.
- `SDL_Rect*` — treat null as "full screen" (no clipping).
- `SurfaceCanvas*` — return -1 if null.
- `const Uint8*` data pointers — return -1 if null.

### 11.5 Logging

Errors and significant state changes shall be logged through the bridge logging mechanism (`rust_bridge_log_msg`). Log messages shall include the function name and a description of the error. Routine per-frame operations (like texture upload) shall not log unless an error occurs. One-time notices (like scaler selection) shall log once using a flag guard.

---

## 12. Integration Points with the Rest of UQM

### 12.1 Game drawing API

Game code uses the `TFB_DrawScreen_*` functions to submit draw commands. These are the primary integration points:

| C function | Purpose |
|------------|---------|
| `TFB_DrawScreen_Line` | Enqueue a line draw |
| `TFB_DrawScreen_Rect` | Enqueue a rectangle draw |
| `TFB_DrawScreen_Image` | Enqueue a sprite draw with scaling and colormap |
| `TFB_DrawScreen_FilledImage` | Enqueue a monochrome sprite draw |
| `TFB_DrawScreen_FontChar` | Enqueue a font glyph draw |
| `TFB_DrawScreen_Copy` | Enqueue a screen-to-screen blit |
| `TFB_DrawScreen_CopyToImage` | Enqueue a screen-to-image capture |
| `TFB_DrawScreen_SetMipmap` | Enqueue a mipmap association |
| `TFB_DrawScreen_DeleteImage` | Enqueue image deletion |
| `TFB_DrawScreen_DeleteData` | Enqueue data deletion |
| `TFB_DrawScreen_WaitForSignal` | Enqueue a synchronization signal |
| `TFB_DrawScreen_ReinitVideo` | Enqueue a video reinitialization |
| `TFB_DrawScreen_Callback` | Enqueue an arbitrary callback |

In the end state, these C functions shall forward to the Rust DCQ FFI (`rust_dcq_push_*`).

### 12.2 Graphics context

Game code manages a graphics context (`CONTEXT`) that holds the current drawing state: target drawable, foreground/background colors, draw mode, font, and clipping rectangle. The context is not owned by the graphics subsystem but influences how higher-level drawing operations (in `gfxintrn.h` / `context.c`) construct their draw commands.

### 12.3 Transition API

| C function | Purpose |
|------------|---------|
| `SetTransitionSource(rect)` | Copy a region of `SCREEN_MAIN` to `SCREEN_TRANSITION` |
| `ScreenTransition(type, rect)` | Begin a transition animation using the saved source |
| `LoadIntoExtraScreen(rect)` | Copy a region of `SCREEN_MAIN` to `SCREEN_EXTRA` |
| `DrawFromExtraScreen(rect)` | Copy a region of `SCREEN_EXTRA` to `SCREEN_MAIN` |

These functions shall interact with the graphics subsystem through screen-to-screen copy commands (via the DCQ) and by manipulating the global `TransitionAmount` and `TransitionClipRect` variables.

### 12.4 Scale API

| C function | Purpose |
|------------|---------|
| `SetGraphicScale(scale)` | Set the global sprite scale factor |
| `GetGraphicScale()` | Query the current scale factor |
| `SetGraphicScaleMode(mode)` | Set the scaling algorithm |
| `GetGraphicScaleMode()` | Query the current scaling algorithm |

These control in-game sprite scaling, not presentation-time scaling. Values are stored in global state and read by the image-draw code path.

### 12.5 Asset loading

The graphics subsystem's loader-facing API provides functions for loading graphics data from UIO streams; however, detailed behavioral obligations for loader operations remain partially defined by code parity with the C loading path until prose coverage is expanded (see `REQ-INT-012`). The following functions define the loader entry points:

| Function | Purpose |
|----------|---------|
| `TFB_DrawCanvas_LoadFromFile(dir, filename)` | Load an image file (PNG, etc.) from UIO into a canvas |
| `_GetCelData(fp, length)` | Load animation/frame data from a UIO stream |

These loading functions produce `TFB_Canvas` handles that are then wrapped in `TFB_Image` objects for use with the drawing API.

**Loader behavioral-ownership status:** End-state behavioral ownership of loading is listed as a goal in §1, but the prose contract for loader behavior in this specification is incomplete. A significant share of the loader contract is currently defined by code parity with the C loading path rather than by this document. The governing code references are `gfxload.c`, `png2sdl.c`, `sdluio.c`, and the `TFB_Image`/`TFB_Char`/`TFB_Canvas` data structures defined in `tfb_draw.h`. Until a fuller prose loader contract exists, implementers shall treat those C sources as co-authoritative with this specification for loader behavior details including resource-encoding parsing, frame/glyph extraction, color-key/alpha interpretation, and malformed-data handling.

Loader-visible behavior contracts are also defined in the requirements (`REQ-INT-005`, `REQ-INT-012`).

### 12.6 Loader boundary responsibilities

Asset loading spans multiple subsystems. The following table separates loader responsibilities to clarify what belongs to the graphics subsystem, what belongs to the resource/UIO layer, and where behavioral parity with the C path must be preserved.

| Responsibility | Owner | Parity obligation |
|----------------|-------|-------------------|
| **Stream acquisition** — Opening resource files, resolving resource identifiers, managing file handles and read cursors. | UIO / resource system (not graphics). The graphics subsystem receives an open stream or buffer. | The graphics subsystem shall not alter stream-acquisition behavior. |
| **Container parsing** — Interpreting resource-file container formats (e.g., `.ani` frame tables, cel headers, index structures) to extract individual image or glyph records. | Graphics subsystem (execution currently in C `gfxload.c`). | Frame ordering, hotspot extraction, and animation-index semantics shall match the C loading path. This is a code-parity obligation until prose coverage replaces it. |
| **Image/font object construction** — Decoding pixel data (PNG decode, color-key application, alpha interpretation), creating `TFB_Canvas` / `TFB_Image` / `TFB_Char` objects, and populating their fields. | Graphics subsystem (execution currently in C `png2sdl.c`, `canvas.c`, `gfxload.c`). | Color-key semantics, alpha interpretation, pixel format of constructed canvases, and glyph pitch/extent/hotspot population shall match the C path. |
| **Error and malformed-data handling** — Behavior on truncated files, corrupt headers, missing frames, or unsupported formats. | Graphics subsystem. | The subsystem shall not crash or produce undefined behavior on malformed input. Specific error-recovery behavior (e.g., returning null, partial load, logging) is governed by code parity with the C path until prose contracts are added. |

This table is a boundary clarification, not a complete loader specification. The code-parity delegation in §12.5 and `REQ-INT-012` remains in effect for details not covered here.

**Normative loader boundary rules (cross-subsystem seam):** The following rules are normative regardless of code-parity delegation status, and define the verifier-facing pass/fail boundary for loader behavior at the cross-subsystem seam:

1. **Malformed stream/container failure:** If a resource stream is corrupt, truncated, or structurally invalid, the graphics loader shall return a null/error result. It shall not crash, produce undefined behavior, or silently return a partially-constructed object that could cause downstream rendering failures. This is a graphics conformance obligation.
2. **Partial loads:** Partial loads (e.g., loading some frames from a multi-frame animation but failing on others) are not permitted. A loader call either succeeds completely or fails completely and returns null/error.
3. **Frame ordering guarantee:** For multi-frame resources (`.ani` animations), frames shall be extracted and stored in container-defined order. Frame indices in the constructed `TFB_Image` shall correspond 1:1 with the container's frame table entries in order.
4. **Glyph ordering guarantee:** For font resources, glyphs shall be indexed by their character code as defined in the font container format.
5. **Hotspot/colorkey/alpha interpretation:** Hotspot coordinates shall be extracted from the container format and stored in the image/canvas metadata. Color-key transparency and alpha channel interpretation shall be applied during image construction per the container's declared format. The exact pixel-level behavior for each container format is governed by code parity until prose coverage replaces it, but the invariant is: a loaded image with identical source data shall produce identical pixel content across C and Rust implementations.
6. **Resource/UIO preconditions:** The graphics loader assumes it receives a valid open stream from the resource/UIO layer (`file-io/specification.md`). If the stream is null or the resource dispatch layer fails to provide one, that is a resource/file-io conformance issue, not a graphics conformance issue. The graphics loader is not responsible for retrying or recovering from stream-acquisition failures.

### 12.7 Thread synchronization

**Threading contract dependency:** The graphics subsystem's producer/consumer model, DCQ locking, condition-variable signaling, and per-image mutex behavior depend on the threading subsystem's interim acceptance rules (`threading/specification.md`). Specifically, graphics relies on: (1) mutexes are non-recursive (plain), (2) thread creation is immediate (no deferred launch), and (3) no stack-size dependency. These interim rules are the currently controlling cross-subsystem contract for threading semantics. Graphics end-state signoff is contingent on these rules remaining unchanged or being superseded by compatible final rules. If threading's final signoff changes any of these rules, the graphics threading model must be re-evaluated.

The graphics subsystem interacts with UQM's threading model through:

- **`RenderingCond`** — A condition variable broadcast at the end of each flush cycle to wake game threads waiting for draw completion.
- **`TFB_DrawScreen_WaitForSignal`** — Enqueues a semaphore/signal that is released during flush processing, allowing game code to block until a specific draw command has been executed.
- **DCQ locking** — The DCQ's lock mechanism (`Lock_DCQ` / `Unlock_DCQ`) serializes access between the game thread (producer) and graphics thread (consumer) during livelock deterrence.

### 12.8 Thread-safety map

The following table maps each integration surface to its thread-safety contract:

| Integration surface | Thread safety | Notes |
|--------------------|--------------|-------|
| `rust_dcq_push_*` / `TFB_DrawScreen_*` | Multi-producer safe | Atomic push from any game thread |
| `TFB_FlushGraphics` | Graphics thread only | Single consumer |
| `TFB_SwapBuffers` / vtable functions | Graphics thread only | Called within flush |
| `rust_gfx_init` / `rust_gfx_uninit` | Startup/shutdown only | Not reentrant |
| `rust_gfx_get_screen_surface` etc. | Read-only after init | Safe from any thread post-init |
| `TFB_Image.mutex`-protected fields | Per-image mutex | Game threads may read concurrently |
| `Lock_DCQ` / `Unlock_DCQ` | Cross-thread serialization | Livelock deterrence |
| `RenderingCond` | Cross-thread signaling | Graphics thread broadcasts |

### 12.9 Build configuration

The Rust graphics subsystem is conditionally enabled by the `USE_RUST_GFX` preprocessor symbol, defined in the build configuration. When enabled:

- C backend initialization (`sdl2_pure.c` / `opengl.c`) is bypassed.
- The `TFB_GRAPHICS_BACKEND` vtable points to Rust wrapper functions.
- Architecture-specific C scaler implementations are compiled but guarded out.
- The Rust static library (`libuqm_rust.a`) must be linked into the final executable.

When `USE_RUST_GFX` is not defined, the legacy C graphics backend remains active and the Rust graphics code is not linked.

---

## 13. Summary of ABI-Fixed Contracts

The following are ABI commitments that the Rust subsystem must preserve for compatibility with C code and existing game data. Each item is classified by the reason it is ABI-fixed.

### 13.1 Hard ABI contracts

These are fixed by C struct layout, FFI function signatures, or header-declared constants that external code compiles against. Changing them requires coordinated ABI changes across C and Rust.

| Contract | Value/Layout | Fixed by |
|----------|-------------|----------|
| Number of screens | 3 (`TFB_GFX_NUMSCREENS`) | C header constant |
| Logical resolution | 320×240 | C header constant, renderer logical size |
| Max colormaps | 250 | C header constant |
| Colors per colormap | 256 | C struct layout |
| Bytes per color | 3 (R, G, B) | C data format |
| Fade range | 0–510 | C header constants |
| `GSCALE_IDENTITY` | 256 | C header constant |
| FFI color packing | R in bits 31–24, G 23–16, B 15–8, A 7–0 | FFI function signatures |
| `TFB_Image` struct layout | As defined in `tfb_draw.h` | C struct layout |
| `TFB_Char` struct layout | As defined in `tfb_draw.h` | C struct layout |
| `TFB_ColorMap` struct layout | As defined in `cmap.h` | C struct layout |
| `SDL_Rect` layout | `{ int x, y, w, h }` (16 bytes) | SDL2 ABI |
| Backend vtable layout | 5 function pointers in `TFB_GRAPHICS_BACKEND` order | C struct layout |
| GFX flag bit assignments | Bits 0–9 as defined in `gfx_common.h` | C header constants |
| FFI function names | `rust_gfx_*`, `rust_canvas_*`, `rust_cmap_*`, `rust_dcq_*` as declared in `rust_gfx.h` | Explicit FFI/header contract |
| Error return convention | 0 = success, -1 = error for `c_int` returns; null for pointer returns | FFI function signatures |

### 13.2 Current migration-constrained compatibility assumptions

These are true today and must be preserved during migration because external code currently depends on them, but they are not permanently required by ABI structure. They may be relaxed if and when the corresponding external dependency is removed.

| Contract | Value/Layout | Constraint source |
|----------|-------------|-------------------|
| Screen surface format | RGBX8888 (32bpp, no alpha), masks R=0xFF000000 G=0x00FF0000 B=0x0000FF00 A=0x00000000 | Raw surface access by C code through surface-pointer accessors (see §3.2) |
