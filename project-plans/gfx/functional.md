# Rust GFX Backend — Functional Specification

**File**: `rust/src/graphics/ffi.rs`
**C Reference**: `sc2/src/libs/graphics/sdl/sdl2_pure.c`
**Date**: 2026-02-23

---

## 1. Purpose

The GFX backend is the final stage of the UQM rendering pipeline. It receives
pixel data that C game code has already drawn into shared `SDL_Surface` objects
and composites those surfaces into a visible frame on the player's display.

The backend does **not** draw game content — sprites, text, lines, rectangles,
and other primitives are drawn by C code directly into surface pixel memory
before the backend is invoked. The backend's sole responsibility is:

1. Clearing the frame
2. Uploading surface pixel data into renderable form
3. Compositing multiple layers with alpha blending into a single frame
4. Overlaying color fills for fade effects
5. Presenting the finished frame to the display

The backend replaces the C `sdl2_pure` driver. The C game code is unaware
of which driver is active — it interacts exclusively through the vtable
defined in `sdl_common.h` and through shared surface pointers.

---
### Assumptions and Risks

- **Black screen root cause is a hypothesis, not proven.** The current Postprocess
  path uploads `surfaces[0]` and calls `present()`, so something should theoretically
  appear. The black screen may involve additional factors beyond the no-op vtable
  functions (e.g., `TFB_SwapBuffers` early-return when no bbox changes are signaled,
  C drawing code not populating surfaces, or timing interactions). Implementing the
  compositing pipeline correctly is required regardless, and is expected to resolve
  the black screen, but this must be verified at runtime.
- **This spec assumes Option A from the inventory**: compositing happens in
  `ScreenLayer` (upload+render per layer), not in `Postprocess`. This is a design
  decision, not the only possible approach.

---



## 2. Actors and Consumers

### 2.1 Primary Consumer: `TFB_SwapBuffers` (`sdl_common.c` lines 275–330)

The sole caller of the 5 vtable entry points. Called from two sites:

- **`TFB_FlushGraphics`** (`dcqueue.c`): After processing all pending draw
  commands, calls `TFB_SwapBuffers(TFB_REDRAW_NO)` to present the frame
  (line 621). Also calls `TFB_SwapBuffers(TFB_REDRAW_FADING)` when the
  draw queue is empty but a fade or transition is actively animating
  (line 343). Additionally, `TFB_DRAWCOMMANDTYPE_REINITVIDEO` commands
  trigger `TFB_SwapBuffers(TFB_REDRAW_YES)` after reinitializing the
  video system (line 606).

- **`TFB_ProcessEvents`** (`sdl_common.c` lines 241–247): On
  `SDL_WINDOWEVENT_EXPOSED`, calls `TFB_SwapBuffers(TFB_REDRAW_EXPOSE)` to
  repaint after the window is uncovered or restored.

### 2.2 Secondary Consumers

- **`TFB_UploadTransitionScreen`** (`sdl_common.c` line 377): Wrapper that
  calls `graphics_backend->uploadTransitionScreen()`. Invoked by C game
  code when preparing a screen transition (after copying the main screen
  surface to the transition screen surface).

- **`TFB_InitGraphics`** (`sdl_common.c` lines 95–178): Calls
  `rust_gfx_init` during startup and retrieves surface pointers. Sets
  `graphics_backend = &rust_backend`.

- **`TFB_UninitGraphics`** (`sdl_common.c` lines 181–205): Calls
  `rust_gfx_uninit` during shutdown.

- **`TFB_ProcessEvents`** (`sdl_common.c`): C code owns SDL event polling.
  `TFB_ProcessEvents` polls SDL events and handles expose/quit/activation.
  When an expose event occurs, it calls `TFB_SwapBuffers(TFB_REDRAW_EXPOSE)`
  which triggers the backend vtable. Note: `rust_gfx_process_events` is
  declared in the header but has **zero C call sites** — it is currently
  unused. C retains full ownership of event polling regardless of backend.

### 2.3 C Drawing Code

C game logic enqueues draw commands into the DCQ (draw command queue). The
main thread's `TFB_FlushGraphics` loop pops these commands and executes them,
drawing directly into `SDL_Screens[i]->pixels` via SDL surface operations.
The backend never sees individual draw commands — it sees only the final
pixel data in the surfaces when `TFB_SwapBuffers` is called.

---

## 3. The Five Vtable Entry Points

The vtable is defined in `sdl_common.h` (lines 30–36):

```c
typedef struct _tfb_graphics_backend {
    void (*preprocess) (int force_redraw, int transition_amount, int fade_amount);
    void (*postprocess) (void);
    void (*uploadTransitionScreen) (void);
    void (*screen) (SCREEN screen, Uint8 alpha, SDL_Rect *rect);
    void (*color) (Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect);
} TFB_GRAPHICS_BACKEND;
```

The Rust backend provides these through static C wrapper functions in
`sdl_common.c` (lines 58–92) that forward to the `rust_gfx_*` FFI symbols
declared in `rust_gfx.h` (lines 29–34).

---

### 3.1 Preprocess

**Signature**: `void preprocess(int force_redraw, int transition_amount, int fade_amount)`
**Rust FFI**: `rust_gfx_preprocess(force_redraw: c_int, transition_amount: c_int, fade_amount: c_int)`

#### Preconditions

- The backend has been initialized via `rust_gfx_init` (returns 0).
- The renderer/canvas exists and is in a presentable state.

#### Inputs

| Parameter | Type | Valid Range | Meaning |
|---|---|---|---|
| `force_redraw` | `int` | `TFB_REDRAW_NO` (0), `TFB_REDRAW_FADING` (1), `TFB_REDRAW_EXPOSE` (2), `TFB_REDRAW_YES` (3) | Indicates why the frame is being redrawn. See `gfx_common.h` lines 35–41. |
| `transition_amount` | `int` | 0–255 | Current transition blend level. 255 = no transition. Not used by the SDL2 backend. |
| `fade_amount` | `int` | 0–511 | Current fade level. 255 = fully visible (no fade). <255 = fade to black. >255 = fade to white. Not used by the SDL2 backend in Preprocess. |

#### Expected Behavior

1. **Clear the renderer to opaque black.** Every frame starts from a clean
   black canvas. The compositing calls that follow (ScreenLayer, ColorLayer)
   layer content on top.
2. The `transition_amount` and `fade_amount` parameters are informational
   only — the C SDL2 backend ignores them in Preprocess, and the Rust
   backend is not required to use them. They exist for backends (like
   OpenGL) that may need them for shader configuration.
3. The `force_redraw` parameter controls dirty-rect optimization in the C
   backend. The Rust backend does not track dirty rects (it re-uploads full
   surfaces unconditionally), so this parameter has no effect on behavior.
   However, the function must still execute the clear regardless of the
   `force_redraw` value.

#### Postconditions

- The renderer's draw target is cleared to solid black (R=0, G=0, B=0, A=255).
- The renderer's blend mode is set to `NONE` (no blending for subsequent
  operations to use as their default).
- No surface data has been read or modified.

#### Error Handling

- If the backend is not initialized, the call is silently ignored (no crash).

---

### 3.2 ScreenLayer

**Signature**: `void screen(SCREEN screen, Uint8 alpha, SDL_Rect *rect)`
**Rust FFI**: `rust_gfx_screen(screen: c_int, alpha: u8, rect: *const SDL_Rect)`

This is the core compositing function. It reads pixel data from a shared
surface and renders it onto the frame being composed.

#### Preconditions

- Preprocess has been called for this frame (renderer is cleared).
- The surface `SDL_Screens[screen]` contains valid pixel data drawn by C
  game code.
- For `TFB_SCREEN_TRANSITION`: `UploadTransitionScreen` has been called at
  least once since the last time C code updated the transition surface's
  pixel content.

#### Inputs

| Parameter | Type | Valid Range | Meaning |
|---|---|---|---|
| `screen` | `SCREEN` (int) | 0 (`TFB_SCREEN_MAIN`), 1 (`TFB_SCREEN_EXTRA`), 2 (`TFB_SCREEN_TRANSITION`) | Which screen surface to render. See `tfb_draw.h` lines 27–33. |
| `alpha` | `Uint8` | 0–255 | Per-layer alpha. 255 = fully opaque (no blending). <255 = semi-transparent, blended over previously rendered content. |
| `rect` | `SDL_Rect*` | Valid pointer or `NULL` | Clipping rectangle in logical coordinates (320×240 space). `NULL` = render the entire surface. Non-NULL = render only the specified rectangular region. |

#### Expected Behavior

1. **Read surface pixel data**: Access the pixel memory of
   `SDL_Screens[screen]` (the shared surface that C code has drawn into).

2. **Upload to renderable form**: Transfer the surface's pixel data so
   it can be composited onto the renderer. The upload must include the
   full surface (the Rust backend does not track dirty regions).

3. **Set alpha blending**:
   - If `alpha == 255`: The layer is fully opaque. Blending is disabled
     (`BLENDMODE_NONE`). The layer's pixels overwrite whatever is beneath.
   - If `alpha < 255`: The layer is semi-transparent. Alpha blending is
     enabled (`BLENDMODE_BLEND`). The layer's alpha modifier is set to
     `alpha`. The compositing formula is:
     `dst = src × alpha/255 + dst × (1 - alpha/255)`.

4. **Render with clipping**:
   - If `rect` is `NULL`: Render the entire surface to the full renderer
     area.
   - If `rect` is non-NULL: Render only the portion of the surface
     specified by `rect`. The destination on screen matches the source
     coordinates — i.e., the rect specifies both which part of the source
     to read AND where on screen to draw it. Source rect and destination
     rect are identical in logical-coordinate space.

5. **Software scaling** (when active): If software scaling is enabled
   (xBRZ or HQ2x), the surface pixel data must be scaled up before
   rendering. The source rect must be scaled by the same factor to index
   into the scaled pixel data. The destination rect remains in logical
   coordinates (SDL2's renderer logical-size mapping handles display
   scaling).

#### Postconditions

- The specified surface's pixel data has been rendered onto the current
  frame, respecting the alpha and clipping parameters.
- The renderer's state has been modified (a textured quad has been
  composited). Previous frame content in the affected region may have been
  overwritten or blended with.
- The source surface pixel data is unchanged (read-only access).

#### Call Patterns in `TFB_SwapBuffers`

ScreenLayer is called up to 3 times per frame, always in this order:

1. `screen(TFB_SCREEN_MAIN, 255, NULL)` — **Always called.** Renders the
   full main game screen opaquely. This is the base layer.

2. `screen(TFB_SCREEN_TRANSITION, 255 - transition_amount, &clip_rect)` —
   **Conditional.** Called only when `transition_amount != 255` (a screen
   transition is in progress). Overlays the old screen image with
   decreasing alpha, clipped to `TransitionClipRect`. The transition
   surface holds a snapshot of the previous screen state.

3. `screen(TFB_SCREEN_MAIN, 255, &system_box)` — **Conditional.** Called
   only when `system_box_active` is true. Re-renders just the system UI
   area from the main screen on top of any fade/transition overlays.
   This ensures the system box (e.g., loading indicators) remains fully
   visible during fades.

#### Error Handling

- If `screen` is out of range `[0, TFB_GFX_NUMSCREENS)`: silently return,
  no rendering.
- If the backend is not initialized: silently return.
- If the surface pointer is null: silently return.
- If the surface has null pixels or zero/negative pitch: silently return.

#### Rect Edge Cases

- **Negative x/y**: SDL2 clips automatically. Rust must pass rect as-is
  to SDL2 copy operations; SDL2 handles negative origin by clipping the
  source and destination rects.
- **Rect extends beyond logical bounds (320×240)**: SDL2 clips. No action
  needed beyond passing the rect through.
- **Zero width or height**: SDL2 renders nothing. This is a valid no-op.
- **In practice**, `TFB_SwapBuffers` only passes `NULL` (full screen) or
  `TransitionClipRect` / `system_box_rect`, both of which are valid
  within logical bounds. Out-of-bounds rects are defensive edge cases
  that should not occur in normal gameplay.

---

### 3.3 ColorLayer

**Signature**: `void color(Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect)`
**Rust FFI**: `rust_gfx_color(r: u8, g: u8, b: u8, a: u8, rect: *const SDL_Rect)`

Draws a solid color rectangle onto the frame. Used exclusively for fade
effects (fade-to-black, fade-to-white).

#### Preconditions

- Preprocess has been called for this frame.
- ScreenLayer for the main screen has already been called (the base game
  image is present).

#### Inputs

| Parameter | Type | Valid Range | Meaning |
|---|---|---|---|
| `r` | `Uint8` | 0–255 | Red component of the fill color. |
| `g` | `Uint8` | 0–255 | Green component. |
| `b` | `Uint8` | 0–255 | Blue component. |
| `a` | `Uint8` | 0–255 | Alpha of the fill. 255 = fully opaque (complete fade). 0 = fully transparent (no visible effect). |
| `rect` | `SDL_Rect*` | Valid pointer or `NULL` | Area to fill. `NULL` = fill the entire screen. |

#### Expected Behavior

1. **Set blending**:
   - If `a == 255`: Blending is disabled (`BLENDMODE_NONE`). The color
     completely overwrites the area.
   - If `a < 255`: Alpha blending is enabled (`BLENDMODE_BLEND`). The
     color is composited over existing content.

2. **Set the draw color** to `(r, g, b, a)`.

3. **Fill the rectangle**:
   - If `rect` is `NULL`: Fill the entire renderer area.
   - If `rect` is non-NULL: Fill only the specified region.

#### Call Patterns in `TFB_SwapBuffers`

ColorLayer is called at most once per frame, only when `fade_amount != 255`:

- **Fade to black** (`fade_amount < 255`):
  `color(0, 0, 0, 255 - fade_amount, NULL)`.
  As `fade_amount` decreases from 255 to 0, the black overlay alpha
  increases from 0 to 255, progressively darkening the screen.

- **Fade to white** (`fade_amount > 255`):
  `color(255, 255, 255, fade_amount - 255, NULL)`.
  As `fade_amount` increases from 255 to 510, the white overlay alpha
  increases from 0 to 255, progressively whitening the screen.

In both cases, `rect` is `NULL` (full-screen fade).

#### Postconditions

- A colored rectangle has been composited onto the current frame.
- If `a == 0`, no visible change occurs (fully transparent fill).
- If `a == 255`, the filled area is completely replaced by the solid color.

#### Error Handling

- If the backend is not initialized: silently return.

---

### 3.4 UploadTransitionScreen

**Signature**: `void uploadTransitionScreen(void)`
**Rust FFI**: `rust_gfx_upload_transition_screen()`

Notifies the backend that the transition screen surface (`SDL_Screens[2]`)
has been updated and its renderable form needs to be refreshed.

#### Preconditions

- The backend is initialized.
- C code has just copied pixel data into `SDL_Screens[TFB_SCREEN_TRANSITION]`
  (typically via `SDL_BlitSurface` from the main screen).

#### Expected Behavior

In the C backend, this function marks the transition screen's dirty flag
so the next `ScreenLayer(TFB_SCREEN_TRANSITION, ...)` call will re-upload
the surface pixels.

**In the Rust backend**: Because the Rust driver unconditionally re-uploads
the full surface on every ScreenLayer call (no dirty tracking), this
function has no required side effects. It is a valid no-op.

**Dependency invariant**: This function may only remain a no-op as long
as ScreenLayer unconditionally uploads the surface. If ScreenLayer is
ever optimized to skip upload for unchanged surfaces, this function must
be changed to set a dirty flag for `TFB_SCREEN_TRANSITION`.

#### Postconditions

- The backend is aware (or does not need to be aware, in the no-dirty-tracking
  case) that the transition surface content has changed.

#### Error Handling

- If the backend is not initialized: silently return (no-op regardless).

---

### 3.5 Postprocess

**Signature**: `void postprocess(void)`
**Rust FFI**: `rust_gfx_postprocess()`

Finalizes and presents the composed frame to the display.

#### Preconditions

- Preprocess, all ScreenLayer calls, and any ColorLayer call for this frame
  have completed.
- The renderer contains the fully composited frame.

#### Expected Behavior

1. Present the composed frame to the display. The frame becomes visible to
   the user.
2. Optionally apply scanline effects before presentation (when
   `GfxFlags & TFB_GFXFLAGS_SCANLINES` is set). Scanlines are a cosmetic
   effect that draws semi-transparent horizontal lines at double resolution
   to simulate CRT scanlines.

#### Critical Constraint

Postprocess must **only** present. It must NOT upload surface pixel data or
render additional textures. All surface-to-renderer compositing is handled
by the preceding ScreenLayer calls. If Postprocess were to also upload and
render surface data, it would overwrite the layered composition (transition
overlays, fade colors, system box) with a stale single-surface render.

#### Postconditions

- The frame has been presented (the display shows the new frame).
- The renderer is ready for the next frame's Preprocess call.

#### Error Handling

- If the backend is not initialized: silently return.

---

## 4. Call Sequence Contract

`TFB_SwapBuffers` (`sdl_common.c` lines 275–330) calls the vtable functions
in a strict, deterministic order. The backend must support this exact
sequence:

```
TFB_SwapBuffers(force_full_redraw):
  1. preprocess(force_redraw, transition_amount, fade_amount)
  2. screen(TFB_SCREEN_MAIN, 255, NULL)                       // ALWAYS
  3. IF transition_amount != 255:
       screen(TFB_SCREEN_TRANSITION, 255 - transition_amount, &clip_rect)
  4. IF fade_amount != 255:
       IF fade_amount < 255:
         color(0, 0, 0, 255 - fade_amount, NULL)
       ELSE:
         color(255, 255, 255, fade_amount - 255, NULL)
  5. IF system_box_active:
       screen(TFB_SCREEN_MAIN, 255, &system_box)
  6. postprocess()
```

### Invariants

- **Preprocess always first**: Clears the frame. No compositing calls
  may precede it.
- **Main screen always renders**: Step 2 is unconditional. Even if no game
  content has changed, the main screen surface is composited.
- **Transition overlays main**: Step 3 renders over step 2's result. The
  transition layer blends on top with decreasing alpha.
- **Fade overlays everything**: Step 4 draws a color fill on top of all
  screen layers. The entire visible frame is affected.
- **System box overrides fade**: Step 5 re-renders the system box region
  from the main screen on top of the fade, ensuring UI elements like
  loading indicators remain visible during fades.
- **Postprocess always last**: Presents the final composed result. No
  rendering may follow it before the next Preprocess.

### Early-Exit Condition

`TFB_SwapBuffers` early-returns (no vtable calls at all) when all of:
- `force_full_redraw == TFB_REDRAW_NO`
- `TFB_BBox.valid` is false (no dirty region from drawing)
- `fade_amount == 255` (no fade) AND `last_fade_amount == 255`
- `transition_amount == 255` (no transition) AND `last_transition_amount == 255`

This means the backend may not be called every frame — only when there is
something to render or animate.

### Fading Force-Redraw Promotion

When `force_full_redraw == TFB_REDRAW_NO` but a fade or transition is
active or was active last frame, `force_full_redraw` is promoted to
`TFB_REDRAW_FADING` before calling Preprocess. This ensures fade
animations continue to render even when no game drawing has occurred.

---

## 5. Surface Contract

### 5.1 Screen Surfaces

Three screen surfaces exist, all 320×240 pixels, 32 bits per pixel,
RGBX8888 format (no alpha channel):

| Index | Name | Constant | Purpose |
|---|---|---|---|
| 0 | Main Screen | `TFB_SCREEN_MAIN` | Primary game rendering target. C drawing code writes here. |
| 1 | Extra Screen | `TFB_SCREEN_EXTRA` | Auxiliary buffer for save/restore operations (`LoadIntoExtraScreen`, `DrawFromExtraScreen`). Never composited by the backend. `TFB_SwapBuffers` never calls `ScreenLayer` with `screen=1`. If `ScreenLayer` receives `screen=1`, it is a no-op (valid input, no action required). |
| 2 | Transition Screen | `TFB_SCREEN_TRANSITION` | Snapshot of the previous screen state. Used during screen transitions to fade from old to new. |

#### Ownership

- **Created by**: The Rust backend (`rust_gfx_init`), via `SDL_CreateRGBSurface`.
- **Destroyed by**: The Rust backend (`rust_gfx_uninit`), via `SDL_FreeSurface`.
- **Written to by**: C game code, which receives pointers via
  `rust_gfx_get_screen_surface(i)`.
- **Read by**: The Rust backend during ScreenLayer calls.

#### Lifetime

Surfaces exist from `rust_gfx_init` return until `rust_gfx_uninit`
completes. C code must not access surface pointers after `rust_gfx_uninit`.
The C code stores copies of the pointers in globals (`SDL_Screens[]`,
`SDL_Screen`, `TransitionScreen`), which `TFB_UninitGraphics` NULLs out
after calling `rust_gfx_uninit`.

#### Format

- Pixel format: 32bpp RGBX8888
  - `R_MASK = 0xFF000000` (bits 24–31)
  - `G_MASK = 0x00FF0000` (bits 16–23)
  - `B_MASK = 0x0000FF00` (bits 8–15)
  - `A_MASK = 0x00000000` (no alpha channel)
- On little-endian systems (macOS arm64/x86_64), in-memory byte order per
  pixel is: `[X, B, G, R]` (byte 0 at lowest address).
- Pitch: `SDL_Surface.pitch` bytes per row (may include padding beyond
  320 × 4 = 1280 bytes).
- Total pixel data size: `pitch × 240` bytes.

#### Access Rules

- C code may read and write surface pixel data at any time between
  `rust_gfx_init` and `rust_gfx_uninit`.
- The backend reads surface pixel data only during ScreenLayer calls.
- **Threading constraint**: All vtable entry points and all auxiliary FFI
  functions SHALL be called only from the graphics/main thread. C enforces
  this via `dcqueue.c` serialization (draw commands are queued from game
  threads, but `TFB_FlushGraphics` processes them on the main thread).
  The Rust backend is NOT thread-safe and requires no internal synchronization.

### 5.2 Format Conversion Surface

- Created by: `rust_gfx_init`, via `SDL_CreateRGBSurface(0, 0, 0, 32, R_MASK, G_MASK, B_MASK, A_MASK_ALPHA)`.
- Dimensions: 0×0 (it is a format template, not a rendering target).
- `A_MASK_ALPHA = 0x000000FF` — this surface **has** an alpha channel.
- Purpose: C code uses its `SDL_PixelFormat*` as a reference format when
  converting sprites and fonts via `TFB_DisplayFormatAlpha`. It determines
  whether loaded images need format conversion.
- The backend never reads pixel data from this surface.

---

## 6. Initialization and Teardown

### 6.1 Initialization (`rust_gfx_init`)

**Signature**: `int rust_gfx_init(int driver, int flags, const char *renderer, int width, int height)`
**Returns**: 0 on success, -1 on failure.

#### What It Must Establish

1. **SDL2 context**: Initialize the SDL2 library.
2. **Video subsystem**: Initialize SDL2 video.
3. **Window**: Create a window at the requested `width × height`.
   If `flags & TFB_GFXFLAGS_FULLSCREEN`, apply fullscreen mode.
4. **Renderer/Canvas**: Create an SDL2 renderer for the window. The Rust
   backend uses software rendering.
5. **Logical size**: Set the renderer's logical size to 320×240. This
   causes SDL2 to scale all rendering output to the window size
   automatically.
6. **Event pump**: Initialize SDL2 event processing.
7. **Three screen surfaces**: Create three 320×240, 32bpp, RGBX8888
   `SDL_Surface` objects (via `SDL_CreateRGBSurface`).
8. **Format conversion surface**: Create one 0×0, 32bpp, RGBA8888
   `SDL_Surface` for format reference.
9. **Software scaling buffers** (conditional): If software scaling flags
   are set (`TFB_GFXFLAGS_SCALE_SOFT_ONLY`), allocate per-screen pixel
   buffers at the scaled resolution.
10. **Store flags and dimensions**: Retain `flags`, `width`, `height`,
    fullscreen state for later queries.

#### What Callers Observe After Success

- `rust_gfx_get_screen_surface(0..2)` returns valid, non-null surface
  pointers.
- `rust_gfx_get_format_conv_surf()` returns a valid surface pointer.
- A window is visible on the user's display.
- The vtable functions are safe to call.

#### Failure Behavior

If any step fails, all previously allocated resources are freed and the
function returns -1. The C caller (`TFB_InitGraphics`) will `exit(EXIT_FAILURE)`.

### 6.2 Teardown (`rust_gfx_uninit`)

**Signature**: `void rust_gfx_uninit(void)`

#### What It Must Clean Up

1. Free software scaling buffers.
2. Free all four SDL surfaces (three screens + format_conv_surf) via
   `SDL_FreeSurface`. Surfaces must be freed **before** the SDL context
   is destroyed.
3. Destroy the renderer/canvas.
4. Destroy the video subsystem.
5. Destroy the SDL2 context.

#### Drop Order Constraint

Resources must be freed in reverse order of dependency:
surfaces → scaled buffers → canvas/renderer → video → SDL context.

#### What Callers Observe After

- All surface pointers previously returned by `rust_gfx_get_*` are invalid.
- The window is closed.
- Further vtable calls will be silently ignored (return early from
  uninitialized state check).

---

## 7. Auxiliary Functions

> **Note**: The following auxiliary functions are declared in `rust_gfx.h`
> and exported by the Rust library, but as of the current codebase
> **none of them have any C call sites** (verified via grep across
> `sc2/src/`). They are available for future integration. Their
> behavioral contracts are documented here for completeness.

### 7.1 `rust_gfx_process_events`

**Signature**: `int rust_gfx_process_events(void)`
**Returns**: 1 if a quit event was received, 0 otherwise.
**C call sites**: None. C retains event polling in `TFB_ProcessEvents`.

Polls the SDL event queue and returns whether the user has requested
application exit (window close, etc.). Currently unused — C's
`TFB_ProcessEvents` handles all event polling including quit detection.
This function exists for potential future use if event handling is
migrated to Rust.

### 7.2 `rust_gfx_toggle_fullscreen`

**Signature**: `int rust_gfx_toggle_fullscreen(void)`
**Returns**: 1 if now fullscreen, 0 if now windowed, -1 on error.

Toggles between fullscreen and windowed modes.

### 7.3 `rust_gfx_is_fullscreen`

**Signature**: `int rust_gfx_is_fullscreen(void)`
**Returns**: 1 if fullscreen, 0 if windowed.

Queries the current fullscreen state.

### 7.4 `rust_gfx_set_gamma`

**Signature**: `int rust_gfx_set_gamma(float gamma)`
**Returns**: 0 on success, -1 if unsupported.

Adjusts display gamma. The software renderer does not support gamma
correction; this function returns -1 unconditionally.

### 7.5 `rust_gfx_get_width` / `rust_gfx_get_height`

**Signatures**: `int rust_gfx_get_width(void)` / `int rust_gfx_get_height(void)`
**Returns**: The logical screen dimensions (always 320 and 240 respectively).

These return the game's logical resolution, not the window's actual pixel
dimensions.

### 7.6 Surface Accessors

- `rust_gfx_get_sdl_screen()` → `surfaces[0]` (main screen)
- `rust_gfx_get_transition_screen()` → `surfaces[2]` (transition screen)
- `rust_gfx_get_screen_surface(int screen)` → `surfaces[screen]`
- `rust_gfx_get_format_conv_surf()` → format conversion surface

All return `NULL` if the backend is not initialized or (for
`get_screen_surface`) if the screen index is out of range.

---

## 8. Observable Behavior — What the User Sees

When the backend is functioning correctly, the user observes:

### 8.1 Normal Gameplay

The game renders at 320×240 logical resolution, scaled to the window size.
Sprites, text, menus, starfields, planet surfaces, and all game content are
visible. The frame updates whenever the game logic produces new drawing
commands.

### 8.2 Screen Transitions

When navigating between screens (e.g., entering a menu, landing on a
planet), the old screen fades out while the new screen fades in. The
transition is clipped to `TransitionClipRect` — typically the full screen,
but potentially a sub-region. The transition surface shows the old screen
state with decreasing alpha, revealing the new content underneath.

### 8.3 Fade Effects

- **Fade to black**: The screen progressively darkens until fully black.
  Used when entering conversations, loading screens, etc.
- **Fade to white**: The screen progressively brightens to white. Used for
  flash effects.
- During fades, the system box area (if active) remains visible at full
  brightness, showing loading indicators or other system status.

### 8.4 Window Management

- Minimizing and restoring the window causes a full repaint (expose redraw).
- Fullscreen toggle switches between windowed and fullscreen modes.
- The window title reads "The Ur-Quan Masters v0.8.0 (Rust)".

### 8.5 When the Backend is Broken

- **ScreenLayer not working**: Black screen — the renderer is cleared each
  frame but no surface data is composited.
- **ColorLayer not working**: Fades don't work — the screen snaps between
  visible and not-visible instead of smoothly fading.
- **Postprocess still uploading surfaces**: Double-rendering artifacts —
  transitions and fades appear to flicker or are overwritten by a raw
  main-screen render.
- **Preprocess not clearing**: Ghost images from previous frames persist,
  creating visual corruption as layers accumulate.

---

## 9. Redraw Modes

The `force_redraw` parameter passed through `TFB_SwapBuffers` to
`Preprocess` has four values defined in `gfx_common.h` (lines 35–41):

| Value | Constant | Meaning |
|---|---|---|
| 0 | `TFB_REDRAW_NO` | Normal frame — only dirty regions changed. |
| 1 | `TFB_REDRAW_FADING` | A fade or transition is animating — full redraw needed even if no drawing commands were issued. |
| 2 | `TFB_REDRAW_EXPOSE` | The window was uncovered/restored — full redraw to repaint. |
| 3 | `TFB_REDRAW_YES` | Unconditional full redraw. |

For the Rust backend (no dirty-rect tracking), these modes all produce the
same behavior: clear, composite all layers, present. The parameter is
present for interface compatibility and may be used by future optimizations.

---

## 10. GFX Flags

Flags are passed to `rust_gfx_init` and stored globally in `GfxFlags`.
Defined in `gfx_common.h` (lines 44–63):

| Bit | Flag | Value | Relevance to Backend |
|---|---|---|---|
| 0 | `TFB_GFXFLAGS_FULLSCREEN` | 0x01 | Init: create window fullscreen. |
| 1 | `TFB_GFXFLAGS_SHOWFPS` | 0x02 | FPS display (handled by DCQ, not backend). |
| 2 | `TFB_GFXFLAGS_SCANLINES` | 0x04 | Postprocess: draw scanline overlay. |
| 3 | `TFB_GFXFLAGS_SCALE_BILINEAR` | 0x08 | Use SDL2 linear texture filtering. |
| 4–6 | Various | 0x10–0x40 | Scaler selections (not used by Rust backend currently). |
| 7 | `TFB_GFXFLAGS_SCALE_HQXX` | 0x80 | Use HQ2x software scaler in ScreenLayer. |
| 8 | `TFB_GFXFLAGS_SCALE_XBRZ3` | 0x100 | Use xBRZ 3× software scaler. |
| 9 | `TFB_GFXFLAGS_SCALE_XBRZ4` | 0x200 | Use xBRZ 4× software scaler. |

`TFB_GFXFLAGS_SCALE_SOFT_ONLY = SCALE_ANY & ~SCALE_BILINEAR`: when set,
ScreenLayer must run a software scaler on surface pixel data before
rendering. When only `SCALE_BILINEAR` is set, SDL2's built-in linear
texture filtering is used instead (no software scaling).


---

## 11. Drawing Pipeline Overview

Sections 1–10 specify the **presentation layer** — the final stage where
already-drawn surfaces are composited into visible frames. Sections 11–17
specify the **drawing pipeline** — the full path from game logic to pixel
data in surfaces. Eliminating all C graphics code requires porting both.

### 11.1 Current C Drawing Pipeline

```
Game logic (C)
  → SetContext (context.c): set fg/bg color, clip rect, draw mode
  → Drawing primitives (frame.c): DrawLine, DrawRectangle, DrawStamp, DrawBatch
    → TFB_Prim_* (tfb_prim.c): translate context-relative → screen-relative
      → TFB_DrawScreen_* (tfb_draw.c): enqueue draw commands into DCQ
        → TFB_FlushGraphics (dcqueue.c): pop + dispatch on main thread
          → TFB_DrawCanvas_* (canvas.c): pixel-level drawing into SDL_Surface
            → putpixel/getpixel (primitives.c): per-pixel access for 8/16/24/32bpp
  → TFB_SwapBuffers (sdl_common.c): composite surfaces → present frame
```

### 11.2 Target Rust Drawing Pipeline

```
Game logic (C, gradually migrating to Rust)
  → SetContext → Rust DrawContext (context.rs, 1,011 lines)
  → Drawing primitives → Rust frame ops (frame.rs, 490 lines)
    → DCQ enqueue → Rust DrawCommandQueue (dcqueue.rs, 1,362 lines)
      → process_commands → Rust DCQ dispatcher
        → Canvas operations → Rust tfb_draw (tfb_draw.rs, 3,405 lines)
          → Rust Canvas pixel buffer (no SDL_Surface dependency)
  → Rust presentation layer (ffi.rs, existing §1–10)
```

### 11.3 Key Architectural Difference

The C pipeline draws directly into `SDL_Surface` pixel memory. The Rust
pipeline draws into its own `Canvas` type (a `Vec<u8>` pixel buffer with
format metadata). For coexistence during incremental porting, the Rust
Canvas must either:

- **Option A**: Wrap an `SDL_Surface` (Canvas holds a raw pointer to
  `surface->pixels`) so C and Rust share the same backing memory.
- **Option B**: Own its own pixel buffer and synchronize to `SDL_Surface`
  at frame boundaries (copy Canvas pixels → surface pixels before
  `TFB_SwapBuffers`).

Option A is preferred for zero-copy performance. Option B is simpler but
adds a full-surface memcpy per frame.

### 11.4 Pipeline Stages and Their C/Rust Status

| Stage | C Files | Lines | Rust Module | Rust Lines | Status |
|---|---|---|---|---|---|
| Context | context.c | 404 | context.rs | 1,011 | Rust implemented, not wired |
| Primitives (frame) | frame.c | 266 | frame.rs | 490 | Rust implemented, not wired |
| TFB_Prim dispatch | tfb_prim.c | 237 | (integrated into dcqueue.rs) | — | C-only |
| DCQ enqueue | tfb_draw.c (TFB_DrawScreen_*) | 493 | dcqueue.rs | 1,362 | Rust implemented, ZERO FFI exports |
| DCQ flush | dcqueue.c | 670 | dcqueue.rs | 1,362 | Rust implemented, ZERO FFI exports |
| Canvas drawing | canvas.c | 2,176 | tfb_draw.rs | 3,405 | Rust implemented, ZERO FFI exports |
| Pixel ops | primitives.c | 633 | (in tfb_draw.rs) | — | Rust integrated |
| Colormaps | cmap.c | 663 | cmap.rs | 774 | Rust implemented, not wired |
| Font rendering | font.c | 334 | font.rs | 957 | Rust implemented, not wired |
| Widgets | widgets.c | 941 | — | 0 | C-only, no Rust equivalent |
| Gfxload | gfxload.c | 597 | — | 0 | C-only, no Rust equivalent |
| Scalers | 7 files | ~5,192 | scaling.rs | 3,470 | Rust implemented, not wired |
| Presentation | sdl2_pure.c | 465 | ffi.rs | 676 | Rust implemented and wired (§1–10) |

---

## 12. Draw Command Queue (DCQ) Integration

### 12.1 C DCQ Architecture

The C DCQ (`dcqueue.c`, 670 lines) is the central serialization point.
Game threads enqueue draw commands; the main thread's `TFB_FlushGraphics`
loop processes them:

```c
// dcqueue.c — TFB_FlushGraphics (simplified)
while ((cmd = TFB_DrawCommandQueue_Pop()) != NULL) {
    switch (cmd->Type) {
        case TFB_DRAWCOMMANDTYPE_LINE:
            TFB_DrawCanvas_Line(...);  // → canvas.c
            break;
        case TFB_DRAWCOMMANDTYPE_RECTANGLE:
            TFB_DrawCanvas_Rect(...);  // → canvas.c
            break;
        case TFB_DRAWCOMMANDTYPE_IMAGE:
            TFB_DrawCanvas_Image(...); // → canvas.c
            break;
        // ... 15 command types total
    }
}
TFB_SwapBuffers(...);  // → presentation layer (§1–10)
```

**C DCQ command types** (from `tfb_draw.h` lines 46–63):

| Type | C Constant | Enqueuer (tfb_draw.c) | Handler (dcqueue.c) |
|---|---|---|---|
| 0 | `TFB_DRAWCOMMANDTYPE_LINE` | `TFB_DrawScreen_Line` | `TFB_DrawCanvas_Line` |
| 1 | `TFB_DRAWCOMMANDTYPE_RECTANGLE` | `TFB_DrawScreen_Rect` | `TFB_DrawCanvas_Rect` |
| 2 | `TFB_DRAWCOMMANDTYPE_IMAGE` | `TFB_DrawScreen_Image` | `TFB_DrawCanvas_Image` |
| 3 | `TFB_DRAWCOMMANDTYPE_FILLEDIMAGE` | `TFB_DrawScreen_FilledImage` | `TFB_DrawCanvas_FilledImage` |
| 4 | `TFB_DRAWCOMMANDTYPE_FONTCHAR` | `TFB_DrawScreen_FontChar` | `TFB_DrawCanvas_FontChar` |
| 5 | `TFB_DRAWCOMMANDTYPE_COPY` | `TFB_DrawScreen_Copy` | `TFB_DrawCanvas_CopyRect` |
| 6 | `TFB_DRAWCOMMANDTYPE_COPYTOIMAGE` | `TFB_DrawScreen_CopyToImage` | `TFB_DrawCanvas_CopyRect` (to image) |
| 7 | `TFB_DRAWCOMMANDTYPE_SCISSORENABLE` | (internal) | Set scissor rect |
| 8 | `TFB_DRAWCOMMANDTYPE_SCISSORDISABLE` | (internal) | Disable scissor |
| 9 | `TFB_DRAWCOMMANDTYPE_SETMIPMAP` | `TFB_DrawScreen_SetMipmap` | Set mipmap on image |
| 10 | `TFB_DRAWCOMMANDTYPE_DELETEIMAGE` | `TFB_DrawScreen_DeleteImage` | Free image |
| 11 | `TFB_DRAWCOMMANDTYPE_DELETEDATA` | `TFB_DrawScreen_DeleteData` | Free data |
| 12 | `TFB_DRAWCOMMANDTYPE_SENDSIGNAL` | `TFB_DrawScreen_WaitForSignal` | Signal condvar |
| 13 | `TFB_DRAWCOMMANDTYPE_REINITVIDEO` | `TFB_DrawScreen_ReinitVideo` | Reinit video |
| 14 | `TFB_DRAWCOMMANDTYPE_CALLBACK` | `TFB_DrawScreen_Callback` | Invoke callback |

### 12.2 Rust DCQ Architecture

The Rust DCQ (`dcqueue.rs`, 1,362 lines) mirrors the C DCQ with all 15
command types as the `DrawCommand` enum. It has:

- `DrawCommandQueue::push()` / `try_push()` — enqueue (blocking/non-blocking)
- `DrawCommandQueue::pop()` — dequeue
- `DrawCommandQueue::batch()` / `unbatch()` — batch mode with RAII guard
- `DrawCommandQueue::process_commands()` — flush loop with livelock detection
- `DrawCommandQueue::handle_command()` — dispatches to Rust `tfb_draw` functions

The Rust DCQ dispatches to Rust canvas operations, NOT to C `TFB_DrawCanvas_*`.

### 12.3 The Wiring Gap

The Rust DCQ has **zero `#[no_mangle]` exports**. C code cannot call it.
C code currently calls:

- `TFB_DrawScreen_Line(...)` → enqueues to C DCQ
- `TFB_DrawScreen_Rect(...)` → enqueues to C DCQ
- `TFB_DrawScreen_Image(...)` → enqueues to C DCQ
- etc.

These are called from:
- `TFB_Prim_*` functions in `tfb_prim.c` (game primitives)
- `frame.c` draw functions (`DrawBatch`, `DrawPoint`, `DrawStamp`)
- `font.c` text rendering (`font_DrawText`, `font_DrawTracedText`)
- `widgets.c` UI drawing (`Widget_Draw*`, `DrawShadowedBox`)

### 12.4 Integration Strategy

**Phase 1 — FFI bridge (minimal change):** Export Rust DCQ equivalents
of `TFB_DrawScreen_*` as `#[no_mangle]` functions. Guard C `tfb_draw.c`
with `#ifdef USE_RUST_GFX`. C callers (tfb_prim.c, frame.c) call the
same function names, which now go to Rust:

```rust
#[no_mangle]
pub extern "C" fn TFB_DrawScreen_Line(
    x1: c_int, y1: c_int, x2: c_int, y2: c_int,
    color: Color, draw_mode: DrawMode, dest: c_int
) {
    // Convert C types → Rust types
    // Push to Rust DrawCommandQueue
}
```

**Phase 2 — Replace C callers:** Port `tfb_prim.c`, `frame.c`, etc. to
Rust. The `TFB_DrawScreen_*` FFI bridge becomes unnecessary as Rust code
calls the Rust DCQ directly.

### 12.5 DCQ Flush Integration

The C `TFB_FlushGraphics` (dcqueue.c line 280) must be modified to call
Rust `process_commands()` instead of its own command loop. This requires
either:

- Exporting `rust_dcq_flush()` as `#[no_mangle]`, called from within
  `TFB_FlushGraphics` after the C DCQ is emptied (or replacing the C
  DCQ entirely)
- Or: redirecting C DCQ enqueue functions to push into the Rust DCQ,
  then flushing the Rust DCQ from `TFB_FlushGraphics`

---

## 13. Canvas Operations

### 13.1 C Canvas System

C uses `SDL_Surface` as its canvas type (`TFB_Canvas` is `typedef void*`
in `tfb_draw.h`). Canvas operations in `canvas.c` (2,176 lines) include:

| Function | Lines | Purpose |
|---|---|---|
| `TFB_DrawCanvas_Line` | ~80 | Bresenham line drawing |
| `TFB_DrawCanvas_Rect` | ~40 | Outlined rectangle |
| `TFB_DrawCanvas_Image` | ~120 | Image blitting with scaling |
| `TFB_DrawCanvas_FilledImage` | ~100 | Filled image (solid color mask) |
| `TFB_DrawCanvas_FontChar` | ~90 | Font character rendering with alpha |
| `TFB_DrawCanvas_CopyRect` | ~80 | Canvas-to-canvas blit |
| `New_TrueColorCanvas` | ~30 | Create RGBA canvas |
| `New_PalettedCanvas` | ~30 | Create 8bpp paletted canvas |
| `TFB_DrawCanvas_Rescale_Nearest` | ~60 | Nearest-neighbor rescale |
| `TFB_DrawCanvas_Rescale_Bilinear` | ~80 | Bilinear rescale |
| `TFB_DrawCanvas_Rescale_Trilinear` | ~80 | Trilinear rescale |
| `TFB_DrawCanvas_GetScaledExtent` | ~40 | Compute scaled dimensions |
| `TFB_DrawCanvas_Rotate` | ~120 | Rotation via rotozoom |
| `TFB_DrawCanvas_Lock/Unlock` | ~20 | SDL surface locking |
| `TFB_DrawCanvas_SetClipRect` | ~10 | Set canvas clip rect |
| `TFB_DrawCanvas_Delete` | ~10 | Free canvas |
| `TFB_DrawCanvas_GetScreenFormat` | ~10 | Query display format |
| `TFB_DrawCanvas_SetPalette` | ~20 | Set canvas palette |
| `TFB_DrawCanvas_Initialize` | ~20 | One-time init |

### 13.2 Rust Canvas System

Rust's `Canvas` type (`tfb_draw.rs`, 3,405 lines) is a self-contained
pixel buffer with:

- `data: Vec<u8>` — owned pixel storage
- `width: i32`, `height: i32` — dimensions
- `format: CanvasFormat` — pixel format (RGBA, RGB, Paletted)
- `scissor: RefCell<ScissorRect>` — clipping state
- Drawing primitives implemented as free functions:
  - `draw_line()` — Bresenham's algorithm
  - `draw_rect()` — outlined rectangle (4 lines)
  - `fill_rect()` — filled rectangle (row-by-row)
  - `copy_canvas()` — canvas-to-canvas blit with clipping
  - `draw_fontchar()` — alpha-blended font character
  - `draw_image()` — TFImage blit with hot spot offset
  - `draw_scaled_image()` — scaled image rendering
  - `draw_filled_image()` — filled image rendering

### 13.3 Canvas Type Mismatch

The critical challenge is that C `TFB_Canvas` is `SDL_Surface*` and Rust
`Canvas` is an owned `Vec<u8>` buffer. During the coexistence period:

- C code continues to write to `SDL_Surface->pixels`
- Rust Canvas operates on its own `Vec<u8>`
- The presentation layer (§1–10) reads from `SDL_Surface->pixels`

**Resolution**: The Rust Canvas must share pixel memory with `SDL_Surface`.
Two approaches:

1. **Canvas wraps SDL_Surface pixels**: Canvas stores a raw pointer to
   `surface->pixels` instead of owning a `Vec<u8>`. Rust draws directly
   into SDL_Surface memory. Zero-copy. Requires `unsafe` for pixel access.

2. **Canvas owns pixels, sync at frame boundary**: Canvas keeps its
   `Vec<u8>`, and a sync step copies Canvas pixels → SDL_Surface pixels
   before `TFB_SwapBuffers`. Adds ~300KB memcpy per frame per screen.

### 13.4 Lock/Unlock Semantics

C's `TFB_DrawCanvas_Lock/Unlock` call `SDL_LockSurface/UnlockSurface`.
The Rust Canvas has no locking concept because it owns its pixel data.
If Canvas wraps `SDL_Surface`, locking may be needed for software surfaces
(though SDL2 software surfaces don't typically require locking).

---

## 14. Colormap and Fade System

### 14.1 C Colormap System

C's colormap system (`cmap.c`, 663 lines) manages:

| Function | Purpose |
|---|---|
| `InitColorMaps` | Allocate colormap pool (250 entries) |
| `SetColorMap` | Apply a colormap to a drawable |
| `FadeScreen` | Initiate a screen fade (to black/white/color) |
| `FlushFadeXForms` | Process pending fade transforms |
| `XFormColorMap_step` | Single step of a colormap transform |
| `GetColorMapAddress` | Retrieve colormap data by index |

The C system uses `XFORM_CONTROL` structs (max 16 active transforms) to
interpolate between palettes over time. `FadeScreen` uses `fade_amount`
(0–511) which the presentation layer (§3.3 ColorLayer) uses for overlay.

### 14.2 Rust Colormap System

Rust's colormap system (`cmap.rs`, 774 lines) provides:

| Type/Function | Purpose |
|---|---|
| `ColorMapManager` | Pool management with indexing (250 entries) |
| `ColorMap` / `ColorMapInner` | Individual colormap with refcount and versioning |
| `NativePalette` | 256-entry color palette storage |
| `FadeController` | Screen fade state machine |
| `XformState` | Transform interpolation tracking |

Constants match C: `MAX_COLORMAPS = 250`, `NUMBER_OF_PLUTVALS = 256`,
`FADE_NO_INTENSITY = 0`, `FADE_NORMAL_INTENSITY = 255`,
`FADE_FULL_INTENSITY = 510`, `MAX_XFORMS = 16`.

### 14.3 FFI Bridge Needed

The C colormap system is called from game logic throughout the codebase
(not just from the graphics subsystem). Key C callers include:

- `InitColorMaps()` — called during game startup
- `SetColorMap(COLORMAPPTR)` — called when changing visual state
- `FadeScreen(FadeType, seconds)` — called during scene transitions
- `FlushFadeXForms()` — called from main loop

Export Rust equivalents via `#[no_mangle]` or guard C `cmap.c` with
`#ifdef USE_RUST_GFX` and provide Rust implementations of the same
function signatures.

### 14.4 Fade Integration with Presentation Layer

The `fade_amount` value computed by the colormap system feeds directly
into the presentation layer's ColorLayer (§3.3). Currently, C code sets
the global `fade_amount` variable, and `TFB_SwapBuffers` reads it. The
Rust colormap's `FadeController` must update this same global (or the
presentation layer must read from the Rust fade state).

---

## 15. Widget System

### 15.1 C Widget System

The widget system (`widgets.c`, 941 lines) draws all settings/options UI
elements. It is a high-level drawing system that uses the graphics
primitives:

| Function | Purpose |
|---|---|
| `DrawShadowedBox` | 3D shadowed rectangle |
| `DrawLabelAsWindow` | Floating label in window frame |
| `Widget_DrawMenuScreen` | Full menu screen layout |
| `Widget_DrawChoice` | Choice/radio button widget |
| `Widget_DrawButton` | Pushbutton widget |
| `Widget_DrawLabel` | Static text label |
| `Widget_DrawSlider` | Slider control |
| `Widget_DrawTextEntry` | Text input field |
| `Widget_DrawControlEntry` | Key/control binding widget |

### 15.2 Widget Dependencies

Widgets depend on:
- `SetContext` / `SetContextForeGroundColor` (context.c)
- `DrawRectangle` / `DrawFilledRectangle` (frame.c)
- `font_DrawText` / `font_DrawTracedText` (font.c)
- `BatchGraphics` / `UnbatchGraphics` (gfx_common.c)
- `SetFlashRect` / `PreUpdateFlashRect` / `PostUpdateFlashRect`

Widgets do NOT directly call `TFB_DrawCanvas_*` or `TFB_DrawScreen_*`.
They operate through the higher-level context/frame API.

### 15.3 Porting Strategy

Widgets are a **low priority** for Rust porting because:

1. They are a consumer of the graphics API, not part of the rendering
   pipeline itself
2. They use the same drawing primitive path (context → frame → DCQ →
   canvas) as all other game code
3. Once the underlying pipeline is ported to Rust, widgets continue to
   work through the C API unchanged
4. Eventually, widgets can be ported to Rust by calling the Rust context
   and frame APIs directly

**Recommendation**: Defer widget porting. Guard `widgets.c` with
`USE_RUST_GFX` only after the context/frame/font layer is fully ported.

---

## 16. Graphics Resource Loading

### 16.1 C Resource Loading

`gfxload.c` (597 lines) loads graphics resources from disk:

| Function | Purpose |
|---|---|
| `process_image` | Load/decode an image frame from resource file |
| `processFontChar` | Load/decode a font character from resource file |
| `GetFrameHandle` | Create a FRAME from loaded image data |
| `_GetCelData` | Parse .cel animation data |
| `_GetFontData` | Parse .fnt font data |

The loading pipeline: resource file → binary decode → `SDL_Surface`
creation → format conversion via `TFB_DisplayFormatAlpha` → store in
`TFB_Image`/`TFB_Char`.

Supporting files:
- `filegfx.c` — file I/O layer for graphics resources
- `resgfx.c` — resource system integration
- `loaddisp.c` — display list loading

### 16.2 Rust Resource Handling

The Rust `font.rs` (957 lines) provides `Font`, `FontPage`, and `TFChar`
types. The Rust `drawable.rs` (861 lines) provides `Drawable` and frame
management. The Rust `render_context.rs` (544 lines) provides
`RenderContext` for image/screen/font registry.

**What's missing in Rust**: The actual file I/O and binary decoding —
reading resource files from disk, parsing `.cel` animation data, parsing
`.fnt` font data, and converting to Rust types. The Rust types exist but
have no loader.

### 16.3 Integration Path

Two approaches:
1. **FFI bridge for loading**: C loads resources as it does today, but
   stores them in both C structures and Rust `RenderContext`. The FFI
   bridge calls `rust_register_image()` / `rust_register_font()` after
   each C load.
2. **Rust native loading**: Port `gfxload.c` to Rust entirely. Rust reads
   resource files, decodes them, creates `TFImage`/`TFChar`/`Canvas`
   objects directly. Requires porting the resource I/O layer.

**Recommendation**: Start with FFI bridge (approach 1) for incremental
porting. Port to native loading later.

---

## 17. C File Elimination Strategy

### 17.1 Priority Groups

All 41 C files in `sc2/src/libs/graphics/` and `sc2/src/libs/graphics/sdl/`
must be addressed. Files are grouped by elimination priority.

### 17.2 Group 1 — Already Guarded (2 files)

These files already have `USE_RUST_GFX` guards:

| File | Lines | Status |
|---|---|---|
| `sdl/sdl_common.c` | ~500 | Partially guarded (init, vtable wiring) |
| `sdl/scalers.c` | ~280 | Partially guarded (scaler init) |

### 17.3 Group 2 — Drawing Pipeline Core (Priority 1, 6 files, ~4,669 lines)

These files implement the drawing pipeline that the Rust modules replace.
Guard with `USE_RUST_GFX` and provide Rust FFI replacements:

| File | Lines | Rust Replacement | Action |
|---|---|---|---|
| `dcqueue.c` | 670 | `dcqueue.rs` (1,362 lines) | Guard entirely; Rust DCQ replaces |
| `tfb_draw.c` | 493 | `dcqueue.rs` enqueue functions | Guard entirely; Rust replaces `TFB_DrawScreen_*` |
| `sdl/canvas.c` | 2,176 | `tfb_draw.rs` (3,405 lines) | Guard entirely; Rust Canvas replaces |
| `sdl/primitives.c` | 633 | `tfb_draw.rs` pixel ops | Guard entirely; integrated in Rust Canvas |
| `tfb_prim.c` | 237 | Integrate into Rust DCQ or context | Guard entirely |
| `gfx_common.c` | 196 | `gfx_common.rs` (1,056 lines) | Guard `FlushGraphics`/`BatchGraphics` |

Guarding note: `gfx_common.c` has functions used both inside and outside
the drawing pipeline. Only the drawing-related functions
(`FlushGraphics`, `BatchGraphics`, `UnbatchGraphics`,
`SetTransitionSource`, `ScreenTransition`) should be guarded; other
functions may need to remain.

### 17.4 Group 3 — Data Types and State (Priority 2, 4 files, ~1,437 lines)

These files define types and state management. Guard with `USE_RUST_GFX`:

| File | Lines | Rust Replacement | Action |
|---|---|---|---|
| `context.c` | 404 | `context.rs` (1,011 lines) | Guard entirely |
| `drawable.c` | 501 | `drawable.rs` (861 lines) | Guard entirely |
| `frame.c` | 266 | `frame.rs` (490 lines) | Guard entirely |
| `font.c` | 334 | `font.rs` (957 lines) | Guard font_DrawText/font_DrawTracedText |

Note: `font.c` also contains `TextRect` (extent calculation) which may
be needed by non-drawing code. Guard selectively.

### 17.5 Group 4 — Colormap (Priority 2, 1 file, 663 lines)

| File | Lines | Rust Replacement | Action |
|---|---|---|---|
| `cmap.c` | 663 | `cmap.rs` (774 lines) | Guard entirely |

### 17.6 Group 5 — Scalers (Priority 3, 9 files, ~5,192 lines)

These files implement software scaling algorithms. The Rust `scaling.rs`
(3,470 lines) replaces all of them:

| File | Lines | Rust Replacement | Action |
|---|---|---|---|
| `sdl/hq2x.c` | 2,888 | `scaling.rs` Hq2xScaler | Guard entirely |
| `sdl/rotozoom.c` | 1,038 | `scaling.rs` (rotation TBD) | Guard entirely |
| `sdl/biadv2x.c` | 532 | `scaling.rs` BiadaptiveScaler | Guard entirely |
| `sdl/2xscalers.c` | 260 | `scaling.rs` | Guard entirely |
| `sdl/nearest2x.c` | 207 | `scaling.rs` NearestScaler | Guard entirely |
| `sdl/triscan2x.c` | 155 | `scaling.rs` TriscanScaler | Guard entirely |
| `sdl/bilinear2x.c` | 112 | `scaling.rs` BilinearScaler | Guard entirely |
| `sdl/2xscalers_sse.c` | ~200 | `scaling.rs` (SIMD via std::arch) | Guard entirely |
| `sdl/2xscalers_mmx.c` | ~200 | (obsolete, MMX not needed) | Guard entirely |
| `sdl/2xscalers_3dnow.c` | ~200 | (obsolete, 3DNow not needed) | Guard entirely |

### 17.7 Group 6 — Resource Loading (Priority 3, 4 files, ~1,100 lines)

These files load and manage graphics resources:

| File | Lines | Rust Replacement | Action |
|---|---|---|---|
| `gfxload.c` | 597 | FFI bridge initially, then native Rust | Guard after loader bridge exists |
| `filegfx.c` | ~150 | Rust resource I/O | Guard after resource layer ported |
| `resgfx.c` | ~200 | Rust resource registry | Guard after resource layer ported |
| `loaddisp.c` | ~150 | Rust display list loader | Guard after resource layer ported |

### 17.8 Group 7 — Presentation Layer (Priority 1, already handled)

| File | Lines | Status |
|---|---|---|
| `sdl/sdl2_pure.c` | 465 | Replaced by `ffi.rs` (§1–10) |
| `sdl/pure.c` | ~300 | SDL1 legacy, replace with Rust |
| `sdl/opengl.c` | ~800 | OpenGL path (optional, Rust has `OpenGlDriver`) |

### 17.9 Group 8 — Utility Files (Priority 4, keep or defer)

These files provide utility functions not specific to the drawing pipeline:

| File | Lines | Action |
|---|---|---|
| `sdl/sdl2_common.c` | ~300 | Keep (SDL2-specific utilities, event handling) |
| `sdl/sdl1_common.c` | ~200 | Keep (SDL1 legacy, no port needed) |
| `sdl/sdluio.c` | ~100 | Keep (SDL I/O utilities) |
| `sdl/png2sdl.c` | ~200 | Keep (PNG loading, used by resource system) |
| `sdl/palette.c` | ~150 | Guard with USE_RUST_GFX (Rust cmap.rs replaces) |
| `bbox.c` | ~100 | Guard (bounding box tracking, Rust handles in DCQ) |
| `boxint.c` | ~100 | Keep (box intersection utility) |
| `clipline.c` | ~150 | Keep (line clipping utility) or integrate into Rust |
| `intersec.c` | ~100 | Keep (intersection testing) |
| `pixmap.c` | ~200 | Guard (Rust pixmap.rs replaces) |
| `widgets.c` | 941 | Defer (§15.3 — last priority) |

### 17.10 Elimination Order Summary

1. **Immediate** (Group 2): Guard `dcqueue.c`, `tfb_draw.c`, `canvas.c`,
   `primitives.c`, `tfb_prim.c`. These are the core drawing pipeline
   that Rust already implements.
2. **Next** (Groups 3–4): Guard `context.c`, `drawable.c`, `frame.c`,
   `font.c`, `cmap.c`. Rust implementations exist.
3. **Then** (Group 5): Guard all scaler C files. Rust `scaling.rs`
   replaces them all.
4. **Later** (Groups 6–8): Guard resource loading, utility files,
   and widgets as their Rust replacements mature.
5. **Already done** (Groups 1, 7): `sdl_common.c` and `sdl2_pure.c`
   are already partially/fully handled.

### 17.11 Metrics

- **Total C files to address**: 41
- **Total C lines**: ~15,000+
- **Already guarded**: 2 files (partially)
- **Rust replacements exist**: ~30 files worth of functionality
- **No Rust replacement yet**: widgets.c (941 lines), gfxload.c (597 lines), filegfx.c, resgfx.c, loaddisp.c
