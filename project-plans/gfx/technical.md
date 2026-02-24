# Rust GFX Backend — Technical Specification

**File**: `rust/src/graphics/ffi.rs` (676 lines)
**C Reference**: `sc2/src/libs/graphics/sdl/sdl2_pure.c` (465 lines)
**Date**: 2026-02-23

---

## 1. Architecture Overview

### 1.1 Position in the C/Rust Hybrid

The UQM game engine is written in C. The Rust GFX backend replaces the C
`sdl2_pure` driver for the frame-presentation stage only. The boundary is
narrow and well-defined:

```
C game logic
  → DCQ (draw command queue)
  → TFB_FlushGraphics (main thread, C)
    → draws to SDL_Surface pixel memory (C)
    → TFB_SwapBuffers (C)
      → graphics_backend->* vtable calls (C wrappers)
        → rust_gfx_* FFI functions (Rust)
          → SDL2 renderer operations (Rust, via sdl2 crate)
```

C code is responsible for all game rendering (sprites, text, primitives).
Rust is responsible only for compositing the finished surfaces into frames
and presenting them.

### 1.2 FFI Boundary

The Rust backend exposes `#[no_mangle] pub extern "C" fn` symbols that
are declared in `rust_gfx.h` and linked by the C build system. C code
does not call these directly in most cases — instead, C wrapper functions
in `sdl_common.c` (lines 58–92) forward to the Rust symbols through the
`TFB_GRAPHICS_BACKEND` vtable:

```c
// sdl_common.c — wrapper functions
static void Rust_Preprocess(int force_redraw, int transition_amount, int fade_amount) {
    rust_gfx_preprocess(force_redraw, transition_amount, fade_amount);
}
// ... (one wrapper per vtable entry) ...

static TFB_GRAPHICS_BACKEND rust_backend = {
    Rust_Preprocess,
    Rust_Postprocess,
    Rust_UploadTransitionScreen,
    Rust_ScreenLayer,
    Rust_ColorLayer
};
```

`TFB_InitGraphics` sets `graphics_backend = &rust_backend` (line 124).
All subsequent vtable calls go through this indirection.

### 1.3 Compilation Gate

The Rust backend is conditionally compiled behind `#ifdef USE_RUST_GFX`
in `sdl_common.c`. When the flag is not set, the C `sdl2_pure` driver is
used instead.

---

## 2. State Model

### 2.1 Global State Container

All Rust GFX state is held in a single static:

```rust
static RUST_GFX: GraphicsStateCell = GraphicsStateCell(UnsafeCell::new(None));
```

`GraphicsStateCell` wraps `UnsafeCell<Option<RustGraphicsState>>` and is
manually marked `unsafe impl Sync` with the justification that graphics
state is only accessed from the main thread.

### 2.2 `RustGraphicsState` Fields

```rust
struct RustGraphicsState {
    sdl_context: sdl2::Sdl,
    video: sdl2::VideoSubsystem,
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
    surfaces: [*mut SDL_Surface; TFB_GFX_NUMSCREENS],   // 3 screen surfaces
    format_conv_surf: *mut SDL_Surface,                   // format template
    scaled_buffers: [Option<Vec<u8>>; TFB_GFX_NUMSCREENS], // soft-scaler output
    hq2x: Hq2xScaler,                                    // HQ2x scaler instance
    hq2x_logged: bool,                                    // one-time log flag
    xbrz_logged: bool,                                    // one-time log flag
    flags: c_int,                                         // GFX init flags
    width: u32,                                           // window width
    height: u32,                                          // window height
    fullscreen: bool,                                     // fullscreen state
}
```

### 2.3 Ownership Rules

| Resource | Owner | Lifetime | Drop Responsibility |
|---|---|---|---|
| `sdl_context` | `RustGraphicsState` | init → uninit | Dropped last during uninit |
| `video` | `RustGraphicsState` | init → uninit | Dropped after canvas |
| `canvas` (owns `Window`) | `RustGraphicsState` | init → uninit | Dropped first during uninit |
| `event_pump` | `RustGraphicsState` | init → uninit | Dropped with state |
| `surfaces[0..3]` | Rust (created via C FFI) | init → uninit | `SDL_FreeSurface` before SDL context drop |
| `format_conv_surf` | Rust (created via C FFI) | init → uninit | `SDL_FreeSurface` before SDL context drop |
| `scaled_buffers` | `RustGraphicsState` | init → uninit | Dropped with state (Vec dealloc) |
| Temporary textures | Created per-call | single function call | Dropped at function return |

### 2.4 State Access

```rust
fn get_gfx_state() -> Option<&'static mut RustGraphicsState>
fn set_gfx_state(state: Option<RustGraphicsState>)
pub(crate) fn with_gfx_state<F, R>(f: F) -> Option<R>
    where F: FnOnce(&mut Canvas<Window>, u32, u32) -> R
```

`get_gfx_state()` returns `None` when uninitialized. Every FFI function
guards against this with an early return pattern:

```rust
let Some(state) = get_gfx_state() else { return };
```

### 2.5 Drop Order in `rust_gfx_uninit`

The uninit function (`ffi.rs` lines 318–350) takes ownership of the state
and drops resources in explicit order:

1. `scaled_buffers[i] = None` (free scaling memory)
2. `SDL_FreeSurface(surfaces[i])` for each surface
3. `SDL_FreeSurface(format_conv_surf)`
4. `drop(state.canvas)` — destroys renderer and window
5. `drop(state.video)` — destroys video subsystem
6. `drop(state.sdl_context)` — destroys SDL

This order is critical: SDL surfaces must be freed while the SDL library
is still initialized.

---

## 3. Surface Management

### 3.1 Creation

Screen surfaces are created via raw C FFI calls (`ffi.rs` lines 239–266):

```rust
extern "C" {
    fn SDL_CreateRGBSurface(flags: u32, width: c_int, height: c_int,
        depth: c_int, Rmask: u32, Gmask: u32, Bmask: u32, Amask: u32)
        -> *mut SDL_Surface;
    fn SDL_FreeSurface(surface: *mut SDL_Surface);
}
```

The Rust code calls `SDL_CreateRGBSurface` directly rather than using the
`sdl2` crate's surface API because:
- The returned `*mut SDL_Surface` must be passed to C code as a raw pointer
- The `sdl2` crate's `Surface` type has lifetime constraints that prevent
  sharing across the FFI boundary
- The C code needs to write directly to `surface->pixels`

### 3.2 Pixel Format Details

Screen surfaces (indices 0, 1, 2):

```rust
const R_MASK: u32 = 0xFF000000;
const G_MASK: u32 = 0x00FF0000;
const B_MASK: u32 = 0x0000FF00;
const A_MASK_SCREEN: u32 = 0x00000000;  // no alpha
```

This produces `SDL_PIXELFORMAT_RGBX8888`. On little-endian (all macOS
targets), the in-memory byte layout per pixel is:

```
Byte 0 (lowest address): X (padding, value ignored)
Byte 1: B
Byte 2: G
Byte 3: R
```

The C reference (`sdl2_pure.c` lines 43–53) uses identical masks on
little-endian, so both drivers produce format-compatible surfaces. C
drawing code (primitives, blits, font rendering) operates directly on this
memory layout.

Format conversion surface:

```rust
const A_MASK_ALPHA: u32 = 0x000000FF;  // has alpha channel
```

Created with 0×0 dimensions — it serves as a format template only. Its
`SDL_PixelFormat*` is used by `TFB_DisplayFormatAlpha` (`sdl_common.c`
lines 333–364) to determine the target format for sprite/font surface
conversion.

### 3.3 Surface Sharing Protocol

```
rust_gfx_init():
  Rust creates surfaces[0..3] via SDL_CreateRGBSurface
  Stores raw pointers in RustGraphicsState

TFB_InitGraphics() [C]:
  Calls rust_gfx_get_screen_surface(i) for i=0,1,2
  Stores returned pointers in SDL_Screens[i]
  Sets SDL_Screen = SDL_Screens[0]
  Sets TransitionScreen = SDL_Screens[2]
  Calls rust_gfx_get_format_conv_surf()
  Stores in format_conv_surf global

C drawing code:
  Uses SDL_Screens[i]->pixels for direct pixel manipulation
  No locking needed (single-threaded graphics)

rust_gfx_screen() [Rust]:
  Reads state.surfaces[screen]->pixels
  Uploads to texture for rendering
```

Both C and Rust hold the same `*mut SDL_Surface` values. The surface is a
C-allocated SDL object; Rust creates it via C FFI and accesses it via raw
pointer. No Rust-side wrapper type is used.

### 3.4 `SDL_Surface` Repr

The Rust code declares a `#[repr(C)]` struct matching SDL2's surface
layout (`ffi.rs` lines 36–50):

```rust
#[repr(C)]
pub struct SDL_Surface {
    pub flags: u32,
    pub format: *mut c_void,
    pub w: c_int,
    pub h: c_int,
    pub pitch: c_int,
    pub pixels: *mut c_void,
    pub userdata: *mut c_void,
    pub locked: c_int,
    pub list_blitmap: *mut c_void,
    pub clip_rect: SDL_Rect,
    pub map: *mut c_void,
    pub refcount: c_int,
}
```

Only `pixels`, `pitch`, `w`, and `h` are read by the Rust backend. The
declaration must remain layout-compatible with SDL2's `SDL_Surface`.

---

## 4. Compositing Pipeline

### 4.1 Overview

Each frame follows this pipeline through the vtable:

```
┌──────────────┐
│  Preprocess   │  Clear renderer to black
└──────┬───────┘
       ▼
┌──────────────┐
│ ScreenLayer   │  Upload surfaces[0] → texture, render (opaque, full screen)
│   (MAIN)      │
└──────┬───────┘
       ▼
┌──────────────┐
│ ScreenLayer   │  Upload surfaces[2] → texture, render (alpha blend, clipped)
│ (TRANSITION)  │  [only during transitions]
└──────┬───────┘
       ▼
┌──────────────┐
│  ColorLayer   │  Fill rect with (r,g,b,a) for fade effect
│               │  [only during fades]
└──────┬───────┘
       ▼
┌──────────────┐
│ ScreenLayer   │  Upload surfaces[0] → texture, render (opaque, system_box rect)
│ (MAIN, clip)  │  [only when system_box_active]
└──────┬───────┘
       ▼
┌──────────────┐
│ Postprocess   │  Present frame to display
└──────────────┘
```

### 4.2 Preprocess — Clear

Sets the renderer blend mode to `NONE`, draw color to opaque black
(0, 0, 0, 255), and clears the entire render target.

The blend mode reset before clearing matches the C reference
(`sdl2_pure.c` line 381: `SDL_SetRenderDrawBlendMode(renderer, SDL_BLENDMODE_NONE)`).
While SDL2's `RenderClear` ignores blend mode, setting it to `NONE`
establishes a clean renderer state for subsequent ScreenLayer/ColorLayer
calls.

### 4.3 ScreenLayer — Upload and Render

For each ScreenLayer call, the function must:

1. **Validate** the screen index (0–2).
2. **Read** `state.surfaces[screen]->pixels` (unsafe pointer dereference).
3. **Create** a streaming texture at the appropriate resolution.
4. **Upload** pixel data to the texture via `texture.update()`.
5. **Set blend mode** and alpha mod on the texture.
6. **Render** via `canvas.copy(&texture, src_rect, dst_rect)`.
7. **Drop** the texture (it's a per-call temporary).

The unscaled path uploads at 320×240. The scaled path runs a software
scaler and uploads at the scaled resolution (640×480 for 2×, 960×720 for
3×, 1280×960 for 4×).

### 4.4 ColorLayer — Fill

Sets the renderer's draw color and blend mode, then calls
`canvas.fill_rect()`. When `rect` is null, fills the entire renderer area.
Straightforward delegation to SDL2 renderer draw operations.

### 4.5 Postprocess — Present

Calls `canvas.present()`. No surface access, no texture creation.
Optionally draws scanline effects before presenting (when
`GfxFlags & TFB_GFXFLAGS_SCANLINES`).

---

## 5. Texture Strategy

### 5.1 Per-Call Temporary Textures

The Rust backend creates and destroys textures within each ScreenLayer
call. This is fundamentally different from the C backend, which maintains
persistent per-screen `SDL_Texture` objects across frames.

```rust
// Per-call texture creation (inside rust_gfx_screen):
let texture_creator = state.canvas.texture_creator();
let mut texture = texture_creator.create_texture_streaming(
    PixelFormatEnum::RGBX8888, width, height
)?;
texture.update(None, pixel_data, pitch)?;
canvas.copy(&texture, src_rect, dst_rect)?;
// texture dropped here
```

### 5.2 Why Temporary Textures

The `sdl2` Rust crate enforces lifetime relationships:
- `Texture` borrows from `TextureCreator`
- `TextureCreator` borrows from `Canvas`
- Storing `Texture` in `RustGraphicsState` alongside `Canvas` creates a
  self-referential struct, which Rust's borrow checker prohibits

The C backend does not have this constraint — it stores raw
`SDL_Texture*` pointers without lifetime tracking.

Workarounds exist (pin, unsafe erasure of lifetimes) but are unnecessary
given the performance characteristics:

### 5.3 Performance Implications

Each ScreenLayer call:
- Creates a streaming texture (~1 allocation + GPU/software buffer setup)
- Uploads 320×240×4 = 307,200 bytes (or scaled equivalent)
- Renders one texture copy
- Destroys the texture

Worst case per frame: 3 ScreenLayer calls = 3 texture create/destroy cycles.
With a software renderer, "GPU" operations are CPU memcpy/blit. The total
per-frame overhead is ~3 × 300KB memcpy + render ≈ sub-millisecond on
modern hardware. This is acceptable for a game targeting 30 FPS at 320×240.

### 5.4 `TextureCreator` Lifetime

`TextureCreator` is obtained from `canvas.texture_creator()` within each
FFI function. It borrows from `canvas` for the scope of the function call.
The texture must be used and dropped before the function returns. This is
naturally satisfied by the per-call pattern.

### 5.5 Texture Format

All textures use `PixelFormatEnum::RGBX8888`, matching:
- The surface pixel format (same masks)
- The C backend's texture format (`SDL_PIXELFORMAT_RGBX8888`, `sdl2_pure.c`
  lines 230, 252)

This avoids any pixel format conversion during `texture.update()` — the
source bytes are uploaded directly.

### 5.6 Blend Mode and Alpha Mod on Textures

For opaque layers (`alpha == 255`):
```rust
texture.set_blend_mode(BlendMode::None);
```

For semi-transparent layers (`alpha < 255`):
```rust
texture.set_blend_mode(BlendMode::Blend);
texture.set_alpha_mod(alpha);
```

`BlendMode::Blend` uses the formula:
`dst = src × (alpha/255) + dst × (1 - alpha/255)`.

The screen surfaces have `A_MASK = 0` (no per-pixel alpha). The texture
inherits this — individual pixels are fully opaque. The `alpha_mod` applies
a uniform opacity to the entire texture, which is exactly what the
transition overlay needs.

---

## 6. Software Scaling Integration

### 6.1 When Scaling Is Active

Software scaling is enabled when `GfxFlags & TFB_GFXFLAGS_SCALE_SOFT_ONLY`
is nonzero. `SCALE_SOFT_ONLY` is defined as `SCALE_ANY & ~SCALE_BILINEAR`:
any scaler flag except bilinear triggers software scaling.

| Flag | Constant | Scale Factor | Scaler |
|---|---|---|---|
| bit 7 | `TFB_GFXFLAGS_SCALE_HQXX` | 2× | HQ2x (Rust `Hq2xScaler`) |
| bit 8 | `TFB_GFXFLAGS_SCALE_XBRZ3` | 3× | xBRZ (`xbrz::scale_rgba`) |
| bit 9 | `TFB_GFXFLAGS_SCALE_XBRZ4` | 4× | xBRZ (`xbrz::scale_rgba`) |

When bilinear is the only scaler flag, SDL2's built-in texture filtering is
used (`SDL_HINT_RENDER_SCALE_QUALITY = "1"`). No software scaling occurs.

### 6.2 Scale Factor Determination

```rust
let scale_factor = if (flags & (1 << 8)) != 0 { 3 }     // xBRZ3
                   else if (flags & (1 << 9)) != 0 { 4 } // xBRZ4
                   else { 2 };                             // HQ2x default
```

This logic appears in `rust_gfx_init` (line 304) for buffer allocation and
must be replicated in ScreenLayer for texture sizing. If both xBRZ3 and
xBRZ4 flags are set, xBRZ3 (3×) takes precedence (bit 8 checked first).

### 6.3 Scaled Buffer Allocation

During `rust_gfx_init` (`ffi.rs` lines 301–309):

```rust
let buffer_size = (SCREEN_WIDTH * scale_factor * SCREEN_HEIGHT * scale_factor * 4) as usize;
for i in 0..TFB_GFX_NUMSCREENS {
    state.scaled_buffers[i] = Some(vec![0u8; buffer_size]);
}
```

Three buffers are allocated (one per screen), though in practice only
screens 0 and 2 are used for compositing. Screen 1 (`TFB_SCREEN_EXTRA`)
is marked inactive in the C backend (`sdl2_pure.c` line 178) and is never
passed to ScreenLayer.

### 6.4 Pixel Format Conversion for Scalers

The xBRZ and HQ2x scalers operate on RGBA pixel data (byte order
`[R, G, B, A]`). The screen surfaces use RGBX8888 (byte order
`[X, B, G, R]` on little-endian). Conversion is required:

**Before scaling** (RGBX8888 → RGBA):
```
source[0] (X) → ignored
source[1] (B) → dest[2]
source[2] (G) → dest[1]
source[3] (R) → dest[0]
dest[3] = 0xFF (opaque alpha)
```

**After scaling** (RGBA → RGBX8888 for texture upload):
```
source[0] (R) → dest[3]
source[1] (G) → dest[2]
source[2] (B) → dest[1]
dest[0] = 0xFF (X padding)
```

**Current state (to be changed)**: This conversion is implemented in
`rust_gfx_postprocess` (`ffi.rs` lines 481–528). Per the target
architecture, this logic belongs in ScreenLayer (each layer upload
converts its own surface). Postprocess shall only present the frame.

### 6.5 Scaled Texture and Rect Handling

When software scaling is active:

- The texture is created at `SCREEN_WIDTH × scale_factor` by
  `SCREEN_HEIGHT × scale_factor`.
- The scaled pixel data is uploaded to this larger texture.
- If a clip `rect` is provided, the **source rect** must be multiplied by
  `scale_factor` to index into the scaled texture correctly.
- The **destination rect** remains in logical coordinates (320×240 space)
  — SDL2's `RenderSetLogicalSize` handles the display scaling.

This matches the C `TFB_SDL2_Scaled_ScreenLayer` behavior (`sdl2_pure.c`
lines 435–444):

```c
if (rect) {
    srcRect = *rect;
    srcRect.x *= 2;
    srcRect.y *= 2;
    srcRect.w *= 2;
    srcRect.h *= 2;
    pSrcRect = &srcRect;
}
SDL_RenderCopy(renderer, texture, pSrcRect, rect);
```

### 6.6 xBRZ Scaling Path

The `xbrz-rs` crate provides `scale_rgba(src_data, width, height, factor)`
which returns a `Vec<u8>` of scaled RGBA pixels. The Rust code must:

1. Convert surface pixels from RGBX8888 to RGBA (into a `Pixmap`)
2. Call `xbrz::scale_rgba` with the appropriate factor
3. Convert the result from RGBA back to RGBX8888 (into the scaled buffer)
4. Upload the scaled buffer to the texture

### 6.7 HQ2x Scaling Path

The project's `Hq2xScaler` (from `crate::graphics::scaling`) is used:

1. Convert surface pixels from RGBX8888 to RGBA (into a `Pixmap`)
2. Call `state.hq2x.scale(&pixmap, ScaleParams::new(512, RustScaleMode::Hq2x))`
3. Convert the result from RGBA back to RGBX8888 (into the scaled buffer)
4. Upload the scaled buffer to the texture

The scale factor for HQ2x is always 2×.

---

## 7. C Interface Points

Every `#[no_mangle] pub extern "C" fn` in `ffi.rs`, its C declaration in
`rust_gfx.h`, and the C code that calls it:

### 7.1 Initialization / Teardown

| Rust Symbol | C Declaration (`rust_gfx.h`) | Called From |
|---|---|---|
| `rust_gfx_init(driver, flags, renderer, width, height) -> c_int` | `int rust_gfx_init(int driver, int flags, const char *renderer, int width, int height)` | `TFB_InitGraphics` (`sdl_common.c` line 118) |
| `rust_gfx_uninit()` | `void rust_gfx_uninit(void)` | `TFB_UninitGraphics` (`sdl_common.c` line 188) |

### 7.2 Surface Accessors

| Rust Symbol | C Declaration | Called From |
|---|---|---|
| `rust_gfx_get_sdl_screen() -> *mut SDL_Surface` | `SDL_Surface* rust_gfx_get_sdl_screen(void)` | Not directly called; convenience alias for `get_screen_surface(0)` |
| `rust_gfx_get_transition_screen() -> *mut SDL_Surface` | `SDL_Surface* rust_gfx_get_transition_screen(void)` | Not directly called; convenience alias for `get_screen_surface(2)` |
| `rust_gfx_get_screen_surface(screen: c_int) -> *mut SDL_Surface` | `SDL_Surface* rust_gfx_get_screen_surface(int screen)` | `TFB_InitGraphics` (`sdl_common.c` line 129), in loop for i=0..2 |
| `rust_gfx_get_format_conv_surf() -> *mut SDL_Surface` | `SDL_Surface* rust_gfx_get_format_conv_surf(void)` | `TFB_InitGraphics` (`sdl_common.c` line 138) |

### 7.3 Vtable Functions

| Rust Symbol | C Declaration | C Wrapper | Vtable Field |
|---|---|---|---|
| `rust_gfx_preprocess(force_redraw, transition_amount, fade_amount)` | `void rust_gfx_preprocess(int, int, int)` | `Rust_Preprocess` (line 60) | `preprocess` |
| `rust_gfx_postprocess()` | `void rust_gfx_postprocess(void)` | `Rust_Postprocess` (line 65) | `postprocess` |
| `rust_gfx_upload_transition_screen()` | `void rust_gfx_upload_transition_screen(void)` | `Rust_UploadTransitionScreen` (line 70) | `uploadTransitionScreen` |
| `rust_gfx_screen(screen, alpha, rect)` | `void rust_gfx_screen(int, Uint8, SDL_Rect*)` | `Rust_ScreenLayer` (line 75) | `screen` |
| `rust_gfx_color(r, g, b, a, rect)` | `void rust_gfx_color(Uint8, Uint8, Uint8, Uint8, SDL_Rect*)` | `Rust_ColorLayer` (line 80) | `color` |

### 7.4 Auxiliary Functions

> **Important**: All auxiliary functions below are declared in `rust_gfx.h`
> and exported by the Rust library, but have **zero C call sites** in the
> current codebase (verified via `grep -rn` across `sc2/src/`). They exist
> for future integration and are not required for the black-screen fix.

| Rust Symbol | C Declaration | Called From |
|---|---|---|
| `rust_gfx_process_events() -> c_int` | `int rust_gfx_process_events(void)` | **No C call sites.** C retains event polling in `TFB_ProcessEvents`. |
| `rust_gfx_set_gamma(gamma: f32) -> c_int` | `int rust_gfx_set_gamma(float gamma)` | **No C call sites.** C gamma path uses `SDL_SetWindowBrightness` directly. |
| `rust_gfx_toggle_fullscreen() -> c_int` | `int rust_gfx_toggle_fullscreen(void)` | **No C call sites.** C fullscreen toggle in `sdl_common.c` uses SDL directly. |
| `rust_gfx_is_fullscreen() -> c_int` | `int rust_gfx_is_fullscreen(void)` | **No C call sites.** |
| `rust_gfx_get_width() -> c_int` | `int rust_gfx_get_width(void)` | **No C call sites.** C uses `ScreenWidth` global. |
| `rust_gfx_get_height() -> c_int` | `int rust_gfx_get_height(void)` | **No C call sites.** C uses `ScreenHeight` global. |

### 7.5 C-side Vtable Wiring

The vtable struct in `sdl_common.c` (lines 85–91):

```c
static TFB_GRAPHICS_BACKEND rust_backend = {
    Rust_Preprocess,
    Rust_Postprocess,
    Rust_UploadTransitionScreen,
    Rust_ScreenLayer,
    Rust_ColorLayer
};
```

Assigned at `sdl_common.c` line 124: `graphics_backend = &rust_backend`.

---

## 8. Invariants and Constraints

### 8.1 Double-Render Guard

**Invariant**: ScreenLayer and Postprocess must not both upload and render
surface data.

When ScreenLayer is implemented (uploading surface pixels, creating textures,
calling `canvas.copy`), Postprocess must be reduced to only `canvas.present()`.
If both functions upload `surfaces[0]` to a texture and render it, the
result is:

1. ScreenLayer renders: main screen → transition overlay → fade color →
   system box clip
2. Postprocess re-uploads `surfaces[0]` and renders it on top, clobbering
   the entire layered composition

This produces a frame where transitions, fades, and system box overlays
are invisible — the final `canvas.copy` in Postprocess overwrites them.

**Current state** (per inventory): Postprocess contains the entire 170-line
upload-and-render block. ScreenLayer is a no-op. These are inverted from
the correct architecture. The fix requires moving upload/render logic from
Postprocess to ScreenLayer and reducing Postprocess to `canvas.present()`.

### 8.2 UploadTransitionScreen Dependency on ScreenLayer

**Invariant**: `UploadTransitionScreen` may be a no-op only if ScreenLayer
unconditionally uploads the full surface on every call.

Currently the Rust backend has no dirty-rect tracking — ScreenLayer always
uploads the complete surface. Under this design, `UploadTransitionScreen`
has no work to do. If ScreenLayer is ever optimized to skip uploads for
unchanged surfaces, `UploadTransitionScreen` must set a dirty flag for
`TFB_SCREEN_TRANSITION` so the next ScreenLayer call knows to re-upload.

### 8.3 Re-Entrancy Safety

The vtable functions may be called from multiple code paths:

- Normal frame rendering: `TFB_FlushGraphics` → `TFB_SwapBuffers`
- Fade animation: `TFB_FlushGraphics` (empty DCQ path) → `TFB_SwapBuffers`
- Window expose: `TFB_ProcessEvents` → `TFB_SwapBuffers`

All paths run on the same thread (the main/graphics thread). C enforces
this via `dcqueue.c` serialization — draw commands are queued from game
threads, but `TFB_FlushGraphics` processes them on the main thread only.

**Threading constraint**: All vtable entry points and all auxiliary FFI
functions are called exclusively from the graphics/main thread. The Rust
backend contains no synchronization primitives and is NOT thread-safe.
`RustGraphicsState` uses `RefCell` (single-threaded interior mutability),
not `Mutex`.

The functions must be safe to call with any combination of active/inactive
transitions, fades, and system box state. They must not assume a specific
call pattern beyond the sequence defined in `TFB_SwapBuffers`.

### 8.4 Screen Index 1 (Extra) Is Never Composited

The C backend marks screen index 1 (`TFB_SCREEN_EXTRA`) as
`active = FALSE` (`sdl2_pure.c` line 178). `TFB_SwapBuffers` never passes
screen index 1 to ScreenLayer. The Rust backend allocates a surface and
scaled buffer for it (for C code to use in save/restore operations), but
ScreenLayer will never be called with `screen == 1` under normal operation.

### 8.5 Rect Coordinate System

All `SDL_Rect` values passed to vtable functions are in logical coordinates
(320×240 space). The renderer's logical size is set to 320×240 via
`set_logical_size`, so SDL2 automatically scales these coordinates to the
actual window dimensions. The Rust backend must not apply any additional
coordinate transformation.

Exception: when software scaling is active, the **source rect** for
texture reads must be multiplied by the scale factor because the texture
is at scaled resolution. The destination rect remains in logical
coordinates.

---

## 9. Intentional Non-Parity with C

### 9.1 No Dirty-Rect Tracking

**C behavior**: The C backend tracks per-screen `dirty` flags and `updated`
rects. `Preprocess` sets the dirty region based on `force_redraw` and
`TFB_BBox`. `ScreenLayer` only uploads the dirty region to the texture.
`UploadTransitionScreen` marks the transition screen dirty.

**Rust behavior**: No dirty tracking. Every ScreenLayer call uploads the
full surface unconditionally. `UploadTransitionScreen` is a no-op.
`Preprocess` ignores `force_redraw` (it always clears; the full surface is
always re-uploaded in ScreenLayer).

**Rationale**: Simpler code. The per-frame cost of uploading 300KB of
pixel data is negligible on modern hardware with a software renderer.
Dirty tracking adds complexity and is an optimization that can be added
later without changing the external contract.

### 9.2 No Persistent Textures

**C behavior**: The C backend creates one `SDL_Texture` per screen during
`TFB_Pure_ConfigureVideo` and reuses them across frames. Textures persist
for the entire session.

**Rust behavior**: Textures are created and destroyed within each
ScreenLayer call. See §5 (Texture Strategy) for rationale.

### 9.3 No Scanlines (Yet)

**C behavior**: `TFB_SDL2_Postprocess` calls `TFB_SDL2_ScanLines()` when
`GfxFlags & TFB_GFXFLAGS_SCANLINES`. This draws semi-transparent black
horizontal lines at 2× logical resolution to simulate CRT scanlines.

**Rust behavior**: Scanlines are not implemented. `Postprocess` only calls
`canvas.present()`.

**Rationale**: Scanlines are a cosmetic feature that does not affect game
functionality. It is a post-compositing effect that can be added to
Postprocess without affecting any other function.

### 9.4 Software Renderer

**C behavior**: `TFB_Pure_ConfigureVideo` calls `SDL_CreateRenderer` with
`FindBestRenderDriver()`, which may select a hardware-accelerated backend
(OpenGL, Metal, Direct3D, etc.).

**Rust behavior**: `rust_gfx_init` creates the canvas with `.software()`,
forcing the SDL2 software renderer.

**Rationale**: Avoids GPU pixel format surprises across platforms. The
software renderer guarantees deterministic pixel behavior. Hardware
acceleration can be added later.

### 9.5 Render Scale Quality

**C behavior**: Sets `SDL_HINT_RENDER_SCALE_QUALITY` to `"1"` (linear)
when any scaler flag is set, `"0"` (nearest) otherwise.

**Rust behavior**: Always sets the hint to `"0"` (nearest-neighbor) in
`rust_gfx_init` (`ffi.rs` line 208).

**Rationale**: With a software renderer and explicit software scalers
(xBRZ, HQ2x), linear texture filtering is handled by the scaler itself.
Nearest-neighbor at the renderer level preserves the scaler's output.

---

## 10. Error Handling Strategy

### 10.1 General Pattern

Every FFI function follows this pattern:

```rust
#[no_mangle]
pub extern "C" fn rust_gfx_<name>(...) -> <return_type> {
    let Some(state) = get_gfx_state() else { return <default>; };
    // ... implementation ...
}
```

If the backend is uninitialized, functions return silently with safe
defaults (0, -1, null pointer, or void).

### 10.2 Initialization Errors

`rust_gfx_init` returns -1 for any initialization failure:
- SDL2 init failure
- Video subsystem failure
- Window creation failure
- Canvas/renderer creation failure
- Event pump failure
- Surface creation failure (with cleanup of previously created surfaces)
- Format conversion surface creation failure (with cleanup)

Each failure logs a diagnostic message via `rust_bridge_log_msg`. The C
caller (`TFB_InitGraphics`) calls `exit(EXIT_FAILURE)` on non-zero return.

**Partial initialization cleanup**: If `rust_gfx_init` fails partway through
(e.g., window created but canvas creation fails), it must free all resources
allocated up to the failure point before returning -1. The Rust
implementation achieves this by not storing state until all initialization
succeeds — if any step fails, local variables are dropped and Rust's
ownership system handles cleanup. `rust_gfx_uninit` must also be safe to
call even if `rust_gfx_init` was never called or failed (no-op on
uninitialized state).

### 10.3 ScreenLayer Validation

`rust_gfx_screen` must validate:
- `screen` in range `[0, TFB_GFX_NUMSCREENS)` → return if out of range
- `state.surfaces[screen]` is non-null → return if null
- `surface.pixels` is non-null and `surface.pitch > 0` → return if invalid
- `rect` pointer: null = full screen, non-null = dereference safely

All validation failures are silent returns (no crash, no log spam — these
functions are called at ~30 FPS).

### 10.4 Texture Creation Failure

If `texture_creator.create_texture_streaming()` fails (returns `Err`),
ScreenLayer returns silently. The frame will be missing that layer but the
application will not crash. In practice, texture creation failure with a
software renderer is extremely rare (would indicate out-of-memory).

### 10.5 Surface Pixel Access Safety

Reading surface pixels (`ffi.rs` lines 461–471 in current Postprocess):

```rust
unsafe {
    let surf = &*src_surface;          // deref raw pointer
    if !surf.pixels.is_null() && surf.pitch > 0 {
        let pitch = surf.pitch as usize;
        let total = pitch * SCREEN_HEIGHT as usize;
        let pixels = std::slice::from_raw_parts(surf.pixels as *const u8, total);
        // ... use pixels ...
    }
}
```

The `unsafe` block is required because:
- `src_surface` is a raw pointer from C
- `surf.pixels` is a raw pointer to pixel memory
- `from_raw_parts` constructs a slice from a raw pointer

Safety is ensured by:
- The surface was created by `rust_gfx_init` via `SDL_CreateRGBSurface`
- The surface is 320×240×32bpp, so `pitch × 240` is within allocated bounds
- Single-threaded access (no data races)
- The null/pitch checks guard against uninitialized surfaces

---

## 11. Dependencies

### 11.1 sdl2 Crate

**Version**: `"0.37"` (from project `Cargo.toml`)

Provides safe Rust bindings to SDL2:
- `sdl2::init()` → SDL context
- `sdl2::VideoSubsystem` → window management
- `sdl2::render::Canvas<Window>` → renderer (clear, copy, present, fill_rect)
- `sdl2::render::TextureCreator` → texture factory
- `sdl2::render::Texture` → GPU/software texture
- `sdl2::render::BlendMode` → None, Blend, Add, Mod
- `sdl2::pixels::PixelFormatEnum` → RGBX8888 format constant
- `sdl2::pixels::Color` → RGBA color values
- `sdl2::rect::Rect` → rectangle type for canvas operations
- `sdl2::EventPump` → event polling
- `sdl2::hint::set` → SDL hint configuration

The crate is linked to the system SDL2 library. Surface creation bypasses
the crate (uses raw `SDL_CreateRGBSurface` via C FFI) for cross-language
pointer sharing.

### 11.2 xbrz-rs

**Crate**: `xbrz`

Provides `xbrz::scale_rgba(src_data: &[u8], width: usize, height: usize, factor: usize) -> Vec<u8>`.

Input: RGBA pixel data (`[R, G, B, A]` byte order, 4 bytes per pixel).
Output: Scaled RGBA pixel data at `width*factor × height*factor`.
Used for xBRZ 3× and 4× software scaling.

### 11.3 Project Internal: Scaling Module

**Module**: `crate::graphics::scaling`

Provides:
- `Hq2xScaler` — HQ2x scaling implementation
- `ScaleParams` — scaling configuration (scale value, mode)
- `ScaleMode` (aliased as `RustScaleMode`) — scaling algorithm selection
- `Scaler` trait — common interface for scalers

### 11.4 Project Internal: Pixmap Module

**Module**: `crate::graphics::pixmap`

Provides:
- `Pixmap` — pixel buffer with format metadata
- `PixmapFormat::Rgba32` — RGBA 32-bit format descriptor
- Used as intermediate format for scaler input/output

### 11.5 Project Internal: Bridge Logging

**Module**: `crate::bridge_log`

Provides `rust_bridge_log_msg(&str)` for logging to the C log system.
Used throughout `ffi.rs` for initialization diagnostics and one-time
scaler activation messages.

---

## 12. SDL_Rect Interop

### 12.1 FFI Rect Type

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SDL_Rect {
    pub x: c_int,
    pub y: c_int,
    pub w: c_int,
    pub h: c_int,
}
```

This matches SDL2's `SDL_Rect` layout. Used for C interface parameters.

### 12.2 Conversion to sdl2 Crate Rect

The `sdl2` crate's `Rect` type has a different constructor signature:
`Rect::new(x: i32, y: i32, width: u32, height: u32)`. Width and height
are unsigned.

Conversion from `*const SDL_Rect`:

```rust
let sdl2_rect = if rect.is_null() {
    None
} else {
    let r = unsafe { &*rect };
    Some(sdl2::rect::Rect::new(r.x, r.y, r.w as u32, r.h as u32))
};
```

A helper function for this conversion would reduce duplication across
ScreenLayer and ColorLayer.

---

## 13. Current State Summary

Per the inventory (`inventory.md`):

| Function | Current State | Required State |
|---|---|---|
| `rust_gfx_preprocess` | Clears to black, missing blend mode reset | Clear to black with `BLENDMODE_NONE` |
| `rust_gfx_screen` | **No-op** | Full upload + blend + render per §4.3 |
| `rust_gfx_color` | **No-op** | Full blend mode + fill per §4.4 |
| `rust_gfx_upload_transition_screen` | No-op (correct for no-dirty-tracking design) | No-op (unchanged) |
| `rust_gfx_postprocess` | **170-line upload+scale+render+present** | `canvas.present()` only, per §4.5 and §8.1 |

The compositing logic currently in Postprocess (surface read, pixel
conversion, scaler invocation, texture upload, canvas copy) must be
relocated to ScreenLayer. Postprocess must be reduced to presentation only.


---

## 8. FFI Bridge Architecture for Drawing Pipeline

Sections 1–7 cover the presentation-layer FFI (the vtable). This section
covers the FFI bridge needed to wire the Rust drawing pipeline into the
C game engine — replacing C's DCQ, canvas, colormap, and context systems.

### 8.1 Two Strategies

**Strategy A — Export Rust functions to C callers:**

Rust exports `#[no_mangle] pub extern "C" fn` equivalents of the C
functions that game code currently calls. C callers are unchanged; they
just link to Rust symbols instead of C symbols. Guard C implementations
with `#ifdef USE_RUST_GFX`.

Advantages:
- Minimal changes to C game code
- Incremental — can replace one function at a time
- C callers don't need to know about Rust types

Disadvantages:
- Large FFI surface area (potentially 40+ exported functions)
- Must translate between C types and Rust types at every call
- Two DCQ instances risk if both C and Rust DCQ are partially active

**Strategy B — Replace C callers entirely:**

Port the C caller layers (tfb_prim.c, frame.c, font.c) to Rust. Game
logic calls Rust directly through a thinner FFI. The entire drawing
pipeline runs in Rust.

Advantages:
- Clean architecture — no dual-language drawing pipeline
- Rust types flow end-to-end (no C↔Rust type conversion)
- Fewer FFI boundary crossings

Disadvantages:
- Larger upfront effort
- Must port all callers before any work (no incremental benefit)
- Game logic still in C, needs FFI to call Rust frame/context

**Recommended: Strategy A first, then B.** Start by exporting Rust
functions to C, then incrementally port C callers to Rust.

### 8.2 FFI Bridge for DCQ (Strategy A)

The DCQ bridge replaces `TFB_DrawScreen_*` functions from `tfb_draw.c`
(493 lines). Each C enqueue function has a Rust equivalent with `rust_`
prefix (see §8.2.1 for naming convention):

```rust
// In dcq_ffi.rs

#[no_mangle]
pub extern "C" fn rust_dcq_push_drawline(
    x1: c_int, y1: c_int, x2: c_int, y2: c_int,
    color: FfiColor, draw_mode: FfiDrawMode, dest: c_int
) {
    let cmd = DrawCommand::Line {
        x1, y1, x2, y2,
        color: color.into(),
        draw_mode: draw_mode.into(),
        dest: Screen::from_c(dest),
    };
    global_dcq().push(cmd).ok();
}

#[no_mangle]
pub extern "C" fn rust_dcq_push_drawrect(
    rect: *const FfiRect, color: FfiColor,
    draw_mode: FfiDrawMode, dest: c_int
) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_dcq_push_drawimage(
    img: *mut FfiTfbImage, x: c_int, y: c_int,
    scale: c_int, scalemode: c_int, colormap: *mut c_void,
    draw_mode: FfiDrawMode, dest: c_int
) { /* ... */ }

// ... 12 more for all 15 command types
```

The C `tfb_draw.c` is guarded:
```c
#ifndef USE_RUST_GFX
void TFB_DrawScreen_Line(...) { /* original C implementation */ }
// ...
#endif
```

#### 8.2.1 Symbol Naming Convention

All Rust FFI exports use the `rust_` prefix to avoid collision with C
symbols. C callers are guarded by `#ifdef USE_RUST_GFX` and call the
`rust_*` names directly:

- DCQ exports: `rust_dcq_push_drawline`, `rust_dcq_flush`, `rust_dcq_batch`, etc.
- Canvas exports: `rust_canvas_draw_line`, `rust_canvas_fill_rect`, etc.
- Context exports: `rust_context_set`, `rust_context_set_fg_color`, etc.
- Colormap exports: `rust_cmap_init`, `rust_cmap_set`, `rust_cmap_fade_screen`, etc.

This convention means there is no collision risk with C symbols even when
both paths are linked (the `USE_RUST_GFX` guard prevents duplicate
definitions, and the `rust_` prefix prevents name clashes).

### 8.3 FFI Bridge for DCQ Flush

```rust
#[no_mangle]
pub extern "C" fn rust_dcq_flush() {
    // Process all Rust DCQ commands
    global_dcq().process_commands().ok();
}
```

Called from within `TFB_FlushGraphics` (dcqueue.c):
```c
#ifdef USE_RUST_GFX
    rust_dcq_flush();
#else
    // Original C command processing loop
    while ((cmd = Pop()) != NULL) { ... }
#endif
```

### 8.4 PixelCanvas Trait and Canvas↔SDL_Surface Adapter

#### 8.4.0 PixelCanvas Trait

Drawing functions in `tfb_draw.rs` are generic over a `PixelCanvas` trait
rather than taking `&mut Canvas` directly. This is necessary because screen
surfaces use `SurfaceCanvas` (borrowed `SDL_Surface` pixel data), while
images/offscreen buffers use the owned `Canvas` type. A trait abstraction
lets both types share the same drawing code:

```rust
pub trait PixelCanvas {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pitch(&self) -> usize;
    fn pixels(&self) -> &[u8];
    fn pixels_mut(&mut self) -> &mut [u8];
    fn format(&self) -> PixmapFormat;
}
```

Drawing functions become generic:
```rust
pub fn draw_line<C: PixelCanvas>(canvas: &mut C, x1: i32, y1: i32,
    x2: i32, y2: i32, color: Color) -> Result<(), CanvasError> { ... }
```

**Why generics over `&mut Canvas`:**
- `&mut C` enforces exclusive access at compile time (no runtime locks)
- No enum arms to maintain
- New canvas types (video sequences, offscreen buffers) just implement the trait
- The existing drawing logic already operates on raw pixel slices internally

**DCQ dispatch**: The DCQ's `RenderContext` screen storage will change from
`Arc<RwLock<Canvas>>` to a trait-object-based or generic approach during
P18–P20 implementation. During DCQ flush, `SurfaceCanvas` is created from
the current screen's `SDL_Surface`, and draw commands are dispatched through
the `PixelCanvas` trait.

See REQ-CANVAS-150 in `requirements.md` for the normative requirement.

#### 8.4.1 Adapter Design: Canvas Wrapping SDL_Surface

The most critical bridge element. Rust drawing code must interoperate with
`SDL_Surface` so that:
1. Rust drawing operations produce pixels visible to the presentation
   layer (which reads `SDL_Surface->pixels`)
2. C code that still writes to `SDL_Surface` has its pixels visible to
   Rust code

Both `Canvas` (owned pixels) and `SurfaceCanvas` (borrowed from SDL_Surface)
implement the `PixelCanvas` trait defined above.

```rust
/// Canvas variant that wraps an SDL_Surface's pixel buffer
pub struct SurfaceCanvas {
    /// Raw pointer to SDL_Surface
    surface: *mut SDL_Surface,
    /// Cached dimensions (avoid repeated unsafe deref)
    width: i32,
    height: i32,
    pitch: i32,
    format: CanvasFormat,
    scissor: RefCell<ScissorRect>,
}

impl SurfaceCanvas {
    /// Create from an existing SDL_Surface (unsafe: caller must
    /// guarantee surface lifetime exceeds SurfaceCanvas lifetime)
    pub unsafe fn from_surface(surface: *mut SDL_Surface) -> Self {
        let surf = &*surface;
        Self {
            surface,
            width: surf.w,
            height: surf.h,
            pitch: surf.pitch,
            format: CanvasFormat::from_sdl_surface(surf),
            scissor: RefCell::new(ScissorRect::disabled()),
        }
    }

    /// Get a mutable slice of the pixel data
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        unsafe {
            let surf = &*self.surface;
            std::slice::from_raw_parts_mut(
                surf.pixels as *mut u8,
                (surf.pitch * surf.h) as usize,
            )
        }
    }
}
```

Both `SurfaceCanvas` and the owned `Canvas` implement the `PixelCanvas`
trait (§8.4.0), so all `tfb_draw.rs` functions work with either type via
generics: `fn draw_line<C: PixelCanvas>(canvas: &mut C, ...)`. Screen
canvases use `SurfaceCanvas` (wrapping the 3 SDL_Screens); image canvases
use the existing owned `Canvas`.

#### 8.4.2 Alternative: Pixel Sync

```rust
/// Sync Rust Canvas pixels to SDL_Surface at frame boundary
pub fn sync_canvas_to_surface(canvas: &Canvas, surface: *mut SDL_Surface) {
    unsafe {
        let surf = &mut *surface;
        let src = canvas.data();
        let dst = std::slice::from_raw_parts_mut(
            surf.pixels as *mut u8,
            (surf.pitch * surf.h) as usize,
        );
        dst.copy_from_slice(src);
    }
}
```

Called from `TFB_FlushGraphics` after processing all draw commands and
before `TFB_SwapBuffers`.

### 8.5 FFI Bridge for Context

The context bridge replaces `context.c` (404 lines). All exports use the
`rust_` prefix per §8.2.1:

```rust
#[no_mangle]
pub extern "C" fn rust_context_set(context: *mut FfiContext) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_context_set_fg_color(color: FfiColor) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_context_set_bg_color(color: FfiColor) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_context_set_clip_rect(rect: *const FfiRect) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_context_set_draw_mode(mode: FfiDrawMode) { /* ... */ }
```

### 8.6 FFI Bridge for Colormap

The colormap bridge replaces `cmap.c` (663 lines). All exports use the
`rust_` prefix per §8.2.1:

```rust
#[no_mangle]
pub extern "C" fn rust_cmap_init() { /* init Rust ColorMapManager */ }

#[no_mangle]
pub extern "C" fn rust_cmap_set(cmap_ptr: *const c_void) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_cmap_fade_screen(fade_type: c_int, seconds: c_int) { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_cmap_flush_xforms() { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_cmap_xform_step() -> c_int { /* ... */ }

#[no_mangle]
pub extern "C" fn rust_cmap_get(index: c_int) -> *const c_void { /* ... */ }
```

### 8.7 SurfaceCanvas Adapter Contract

The `SurfaceCanvas` is the critical bridge between C's `SDL_Surface` pixel
buffers and Rust's drawing abstraction. `SurfaceCanvas` implements the
`PixelCanvas` trait (§8.4.0), so all `tfb_draw.rs` drawing functions work
with it through generics. This section defines the precise contract for its
ownership, lifetime, locking, aliasing, format, pitch, and thread affinity.

#### 8.7.1 Ownership

`SurfaceCanvas` **borrows** a `*mut SDL_Surface` — it does not own or
free the surface. The surface is owned by `RustGraphicsState.surfaces[]`
(created in `rust_gfx_init`, freed in `rust_gfx_uninit`). A
`SurfaceCanvas` is created at the start of a DCQ flush and dropped at the
end:

```rust
// During rust_dcq_flush:
let surface: *mut SDL_Surface = state.surfaces[screen_index];
{
    // SAFETY: surface is valid for the duration of this block.
    // SDL_LockSurface is called before, SDL_UnlockSurface after.
    unsafe { SDL_LockSurface(surface); }
    let canvas = unsafe { SurfaceCanvas::from_surface(surface) };
    // ... dispatch all queued draw commands via canvas ...
    drop(canvas);
    unsafe { SDL_UnlockSurface(surface); }
}
```

#### 8.7.2 Locking

- `SDL_LockSurface(surface)` must be called **before** creating a
  `SurfaceCanvas`. This ensures `surface->pixels` is a valid, stable
  pointer for the duration of drawing.
- `SDL_UnlockSurface(surface)` must be called **after** the
  `SurfaceCanvas` is dropped.
- Software surfaces (which all screen surfaces are) do not strictly
  require locking, but the lock/unlock protocol is followed for
  correctness if the surface type ever changes.
- The lock/unlock pair brackets a single DCQ flush — NOT individual draw
  commands. Locking per-command would be a performance disaster.
- **RAII wrapper**: Implement a `LockedSurface<'a>` guard that calls
  `SDL_LockSurface` on construction and `SDL_UnlockSurface` on `Drop`.
  `SurfaceCanvas` is created from a `LockedSurface`, not directly from
  a raw `*mut SDL_Surface`. This prevents forgetting the unlock:

```rust
struct LockedSurface<'a> {
    surface: &'a mut SDL_Surface,
}

impl<'a> LockedSurface<'a> {
    unsafe fn new(surface: *mut SDL_Surface) -> Self {
        SDL_LockSurface(surface);
        Self { surface: &mut *surface }
    }
    fn as_canvas(&mut self) -> SurfaceCanvas<'_> {
        // borrows self, so canvas cannot outlive lock
        SurfaceCanvas::from_locked(self)
    }
}

impl Drop for LockedSurface<'_> {
    fn drop(&mut self) {
        unsafe { SDL_UnlockSurface(self.surface as *mut _); }
    }
}
```

#### 8.7.3 Aliasing

- **No other code may access `surface->pixels` while a `SurfaceCanvas`
  exists.** This is guaranteed by the main-thread-only flush model:
  DCQ flush runs on the main/graphics thread (per REQ-THR-010), and no
  other thread accesses screen surface pixels during flush.
- C game threads enqueue draw commands (they do NOT touch pixels); only
  the flush thread reads/writes pixels.
- The presentation layer (`rust_gfx_screen`) reads `surface->pixels`
  only during `TFB_SwapBuffers`, which runs AFTER flush completes.
  There is no temporal overlap.

#### 8.7.4 Pitch

- `SurfaceCanvas` reads `surface->pitch` directly from the SDL_Surface
  and uses it for all row-stride calculations.
- Pitch is NOT assumed to equal `width * bytes_per_pixel`. SDL may add
  padding for alignment (though in practice RGBX8888 at 320px width =
  1280 bytes/row, which is already 16-byte aligned).
- Pitch is cached at `SurfaceCanvas` creation time and used for all
  subsequent pixel address calculations:
  `pixel_addr = pixels + y * pitch + x * bpp`

#### 8.7.5 Format

- Only `SDL_PIXELFORMAT_RGBX8888` is supported. The SDL2 constant value
  is `0x16261804`. The mask decomposition is:
  R=0xFF000000, G=0x00FF0000, B=0x0000FF00, A=0x00000000.
- The surface pixel format is validated at `SurfaceCanvas` construction
  time. Construction **returns `Err(GraphicsError::UnsupportedFormat)`**
  if the format doesn't match RGBX8888. This is validated by reading
  `surface->format->format` (the `SDL_PixelFormatEnum` field), not by
  inspecting individual masks.
- Byte layout on little-endian: `[X, B, G, R]` per pixel (4 bytes).
- Future expansion to support RGBA8888 (for alpha-channel surfaces) is
  a documented TODO but not required for the initial port.

#### 8.7.6 Thread Affinity

- `SurfaceCanvas` is `!Send + !Sync`. It must NOT be sent to another
  thread or shared across threads.
- It is created and used exclusively on the DCQ consumer thread (which
  is the main/graphics thread).
- This is enforced by NOT implementing `Send` or `Sync` on
  `SurfaceCanvas`. The raw pointer field (`*mut SDL_Surface`) already
  prevents auto-impl of these traits.

#### 8.7.7 Lifetime

- `SurfaceCanvas<'a>` borrows the surface pointer. The lifetime `'a`
  is tied to the flush scope — the canvas cannot outlive the
  lock/unlock bracket.
- Concretely:

```rust
struct SurfaceCanvas<'a> {
    pixels: &'a mut [u8],   // borrows surface->pixels
    width: i32,
    height: i32,
    pitch: i32,
    format: CanvasFormat,
    scissor: ScissorRect,
}
```

- The `&'a mut [u8]` borrow prevents the `SurfaceCanvas` from outliving
  the scope in which `surface->pixels` is valid (between lock/unlock).
- If the Rust implementation uses a raw pointer instead of a reference
  (for FFI ergonomics), the lifetime constraint must be enforced by
  construction — `SurfaceCanvas` is created and dropped within the
  same function scope as the lock/unlock calls.

#### 8.7.8 Self-Blit Prohibition

When a draw command copies pixels from one region of a surface to another
region of the **same** surface (self-blit), the source and destination
memory may overlap. Reading and writing overlapping `&mut [u8]` slices is
undefined behavior in Rust.

**Rule**: `SurfaceCanvas` does NOT support self-blit via raw pointer
aliasing. If a draw command requires copying within the same surface:
1. Allocate a temporary buffer (`Vec<u8>`) for the source region.
2. Copy source pixels into the buffer.
3. Copy buffer pixels to the destination region.

This is the same approach used by `memmove` vs `memcpy` in C, but made
explicit because Rust's borrow checker cannot enforce non-overlap of raw
pointer-derived slices at compile time.

The performance cost is acceptable: self-blits are rare (primarily
`CopyContextRect` for scroll effects) and the temporary buffer is small
(typically one screen-sized rect at most).

#### 8.7.9 Summary Table

| Property | Value |
|---|---|
| Ownership | Borrows `*mut SDL_Surface`, does not free |
| Lock protocol | `SDL_LockSurface` before create, `SDL_UnlockSurface` after drop |
| Aliasing | Exclusive pixel access during flush (main-thread-only) |
| Pitch | Read from `surface->pitch`, NOT assumed `width * bpp` |
| Format | RGBX8888 only (validated at init, cached) |
| Thread affinity | `!Send + !Sync`, main/graphics thread only |
| Lifetime | Cannot outlive the flush scope; tied to lock/unlock bracket |
| Granularity | One lock/unlock per flush, NOT per draw command |
| Mutable access | One row slice at a time via `row_mut(y)` |

#### 8.7.9 Mutable Access

`SurfaceCanvas` provides `&mut [u8]` access to pixel data. To prevent
aliasing of overlapping rows when `pitch != width * bpp` (padding bytes
between rows), mutable access is granted one row at a time:

```rust
impl<'a> SurfaceCanvas<'a> {
    /// Returns a mutable slice for row `y` (width * bpp bytes).
    /// Panics if `y >= height`.
    pub fn row_mut(&mut self, y: i32) -> &mut [u8] {
        let offset = (y as usize) * (self.pitch as usize);
        let row_bytes = (self.width as usize) * self.bpp();
        &mut self.pixels[offset..offset + row_bytes]
    }
}
```

The raw `pixels` field (`&'a mut [u8]`) covers the entire locked surface
buffer (`pitch * height` bytes). `row_mut()` returns a sub-slice that
does NOT include padding bytes. This means:
- Two calls to `row_mut(y1)` and `row_mut(y2)` where `y1 != y2` produce
  non-overlapping slices (safe).
- Callers must NOT hold two `row_mut` borrows simultaneously (the
  `&mut self` receiver prevents this at compile time).
- Bulk operations that need multiple rows should use the raw `pixels`
  slice directly (via an `unsafe` accessor) with appropriate safety
  documentation.

### 8.8 Global DCQ Instance

The Rust DCQ must be accessible from both Rust code and FFI functions:

```rust
static GLOBAL_DCQ: OnceLock<DrawCommandQueue> = OnceLock::new();

fn global_dcq() -> &'static DrawCommandQueue {
    GLOBAL_DCQ.get_or_init(|| {
        DrawCommandQueue::with_config(
            DcqConfig::standard(),
            Arc::new(RwLock::new(RenderContext::new())),
        )
    })
}
```

Initialized during `rust_gfx_init` (or lazily on first use). The same
instance is used by both FFI `TFB_DrawScreen_*` wrappers and Rust-native
callers.

### 8.9 FFI Function Count Summary

| Bridge Area | C Functions Replaced | Rust Exports Needed |
|---|---|---|
| DCQ enqueue (`tfb_draw.c`) | 15 | 15 (`TFB_DrawScreen_*`) |
| DCQ flush (`dcqueue.c`) | 3 | 3 (`rust_dcq_flush_graphics`, `BatchGraphics`, `UnbatchGraphics`) |
| Context (`context.c`) | ~10 | ~10 (`SetContext*`, `CreateContext`, etc.) |
| Colormap (`cmap.c`) | ~8 | ~8 (`InitColorMaps`, `SetColorMap`, etc.) |
| Canvas utility (`canvas.c`) | ~10 | ~10 (`New_TrueColorCanvas`, `Delete`, etc.) |
| **Total** | **~46** | **~46** |

---

## 9. USE_RUST_GFX Guard Strategy

### 9.1 Guard Placement Pattern

Each C file to be replaced follows this pattern:

```c
// At the top of the file, after includes:
#ifdef USE_RUST_GFX
// Rust implementations are linked via the Rust library.
// This file is excluded from compilation when USE_RUST_GFX is defined.
#else

// ... entire original C implementation ...

#endif /* USE_RUST_GFX */
```

For files where only some functions are replaced:

```c
void always_needed_function(void) {
    // This function is NOT replaced by Rust
}

#ifndef USE_RUST_GFX
void replaced_function(void) {
    // This function IS replaced by Rust
}
#endif
```

### 9.2 Dependency Graph for Guard Order

Files must be guarded in dependency order — a file cannot be guarded if
unguarded files depend on it:

```
Level 0 (no dependencies on other guarded files):
  ├── sdl/primitives.c  (pixel ops — used by canvas.c only)
  ├── sdl/hq2x.c        (scaler — used by scalers.c only)
  ├── sdl/biadv2x.c     (scaler)
  ├── sdl/bilinear2x.c  (scaler)
  ├── sdl/nearest2x.c   (scaler)
  ├── sdl/triscan2x.c   (scaler)
  ├── sdl/2xscalers.c   (scaler dispatch)
  └── sdl/rotozoom.c     (rotation)

Level 1 (depends on Level 0):
  ├── sdl/canvas.c       (depends on primitives.c)
  └── sdl/scalers.c      (depends on 2xscaler files) [already partially guarded]

Level 2 (depends on Level 1):
  ├── dcqueue.c          (depends on canvas.c for dispatch)
  ├── cmap.c             (standalone, but shares types)
  └── context.c          (standalone state management)

Level 3 (depends on Level 2):
  ├── tfb_draw.c         (depends on dcqueue.c for enqueue)
  ├── tfb_prim.c         (depends on tfb_draw.c for enqueue)
  └── frame.c            (depends on tfb_prim.c)

Level 4 (depends on Level 3):
  ├── font.c             (depends on frame.c, context.c)
  ├── drawable.c         (depends on frame.c, canvas.c)
  └── gfx_common.c       (depends on dcqueue.c)

Level 5 (depends on Level 4):
  ├── gfxload.c          (depends on drawable.c, canvas.c)
  └── widgets.c          (depends on context.c, frame.c, font.c)
```

**Guard bottom-up**: Start at Level 0, work up. Each level can be guarded
independently once its level is fully implemented in Rust.

### 9.3 Files That Can Be Guarded Independently

These files have no reverse dependencies (nothing else in the graphics
system calls them) and can be guarded in any order:

- All Level 0 scaler files
- `sdl/primitives.c` (only used by `canvas.c`)
- `cmap.c` (called from game code, not from graphics internals)

### 9.4 Files With Cross-Dependencies

These files have bidirectional or complex dependencies:

- `dcqueue.c` ↔ `tfb_draw.c`: DCQ pops commands that tfb_draw.c enqueued.
  Must guard simultaneously.
- `canvas.c` ↔ `dcqueue.c`: DCQ dispatches to canvas drawing functions.
  Must guard simultaneously.
- `context.c` ↔ `frame.c`: Frame functions read context state.
  Should guard together.
- `drawable.c` ↔ `canvas.c`: Drawable creation allocates canvases.
  Should guard together.

### 9.5 Header Files

C header files (`.h`) do NOT need `USE_RUST_GFX` guards for type
definitions — C code outside the graphics system may use these types.
Only function declarations need guards if the C implementation is
excluded:

```c
// tfb_draw.h
// Type definitions — always available
typedef struct tfb_image { ... } TFB_Image;

// Function declarations — guarded
#ifndef USE_RUST_GFX
void TFB_DrawScreen_Line(int x1, int y1, int x2, int y2, ...);
// ...
#endif
```

However, when Rust exports the same symbol names, the declarations must
remain (possibly with `extern` linkage) so C callers can reference them.

### 9.6 Build System Integration

The `USE_RUST_GFX` flag is defined in the build configuration. When set:
1. C files guarded with `#ifndef USE_RUST_GFX` compile to empty
   translation units
2. The Rust library (`libuqm_rust.a`) provides the replacement symbols
3. The linker resolves `TFB_DrawScreen_*` etc. from the Rust library

Build system changes needed:
- `Cargo.toml`: ensure all FFI exports are included in the library crate
- C `Makefile`/`build.sh`: define `USE_RUST_GFX` as a compile flag
- Linker: link `libuqm_rust.a` (or `.so`) with the C object files

---

## 10. Type Mapping

### 10.1 Core Type Mappings

| C Type | C File | Rust Type | Rust Module | Notes |
|---|---|---|---|---|
| `TFB_Image` | `tfb_draw.h` | `TFImage` | `tfb_draw.rs` | Image with canvas, hotspot, mipmap |
| `TFB_Canvas` (`void*`) | `tfb_draw.h` | `Canvas` | `tfb_draw.rs` | Pixel buffer; C uses SDL_Surface* |
| `TFB_Char` | `tfb_draw.h` | `TFChar` | `font.rs` | Font character glyph |
| `RECT` | `gfx_common.h` | `Rect` | `dcqueue.rs` | `{corner: Point, extent: Extent}` |
| `POINT` | `gfx_common.h` | `Point` | `dcqueue.rs` | `{x: i32, y: i32}` |
| `EXTENT` | `gfx_common.h` | `Extent` | `dcqueue.rs` | `{width: i32, height: i32}` |
| `Color` | `gfx_common.h` | `Color` | `dcqueue.rs` | `{r: u8, g: u8, b: u8, a: u8}` |
| `DrawMode` | `gfx_common.h` | `DrawMode` | `context.rs` | `{kind: DrawKind, factor: i16}` |
| `SCREEN` (`int`) | `tfb_draw.h` | `Screen` | `dcqueue.rs` | `enum { Main=0, Extra=1, Transition=2 }` |
| `HOT_SPOT` | `tfb_draw.h` | `HotSpot` | `tfb_draw.rs` | `{x: i32, y: i32}` |
| `COLORMAPPTR` | `cmap.h` | `ColorMapRef` | `dcqueue.rs` | Handle type `(u32)` |
| `CONTEXT` | `context.h` | `DrawContext` | `context.rs` | Drawing state container |
| `FRAME` | `drawable.h` | `Frame` (via DrawableRegistry) | `frame.rs` | Frame handle |
| `DRAWABLE` | `drawable.h` | `Drawable` | `drawable.rs` | Multi-frame container |
| `SDL_Surface*` | SDL2 | `*mut SDL_Surface` | `ffi.rs` | Raw pointer, `#[repr(C)]` |
| `SDL_Rect` | SDL2 | `SDL_Rect` | `ffi.rs` | `#[repr(C)]` `{x, y, w, h}` |
| `TFB_PixelFormat` | `tfb_draw.h` | `CanvasFormat` | `tfb_draw.rs` | `{kind, bpp, Bpp}` |

### 10.2 FFI Repr Types

For FFI boundary crossing, Rust types need `#[repr(C)]` equivalents:

```rust
/// FFI-safe Color matching C's Color struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<FfiColor> for crate::graphics::dcqueue::Color {
    fn from(c: FfiColor) -> Self {
        Self::new(c.r, c.g, c.b, c.a)
    }
}

/// FFI-safe DrawMode matching C's DrawMode struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiDrawMode {
    pub kind: u8,   // 0=Replace, 1=Additive, 2=Alpha
    pub factor: i16,
}

/// FFI-safe RECT matching C's RECT struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiRect {
    pub corner: FfiPoint,
    pub extent: FfiExtent,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiPoint {
    pub x: c_int,
    pub y: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiExtent {
    pub width: c_int,
    pub height: c_int,
}
```

### 10.3 Handle Types

C uses opaque pointers (`FRAME`, `DRAWABLE`, `COLORMAP_REF`) that are
actually integer handles or tagged pointers. Rust uses newtype wrappers:

| C Handle | C Representation | Rust Handle | Conversion |
|---|---|---|---|
| `FRAME` | `void*` (tagged pointer with index) | `FrameRef(u32)` | Extract index from tag bits |
| `DRAWABLE` | `void*` (pointer to DrawableDesc) | `DrawableRef(u32)` | Registry lookup by ID |
| `COLORMAP_REF` | `COLORMAPPTR` (byte array pointer) | `ColorMapRef(u32)` | Registry index |

### 10.4 Enum Mappings

| C Enum/Define | Values | Rust Enum | Values |
|---|---|---|---|
| `TFB_SCREEN_MAIN` | 0 | `Screen::Main` | 0 |
| `TFB_SCREEN_EXTRA` | 1 | `Screen::Extra` | 1 |
| `TFB_SCREEN_TRANSITION` | 2 | `Screen::Transition` | 2 |
| `DRAW_REPLACE` | 0 | `DrawKind::Replace` | 0 |
| `DRAW_ADDITIVE` | 1 | `DrawKind::Additive` | 1 |
| `DRAW_ALPHA` | 2 | `DrawKind::Alpha` | 2 |
| `GSCALE_IDENTITY` | 256 | Scale 1:1 | `scale == GSCALE_IDENTITY` |
| `TFB_SCALE_STEP` | 0 | `ScaleMode::Step` | 0 |
| `TFB_SCALE_NEAREST` | 1 | `ScaleMode::Nearest` | 1 |
| `TFB_SCALE_BILINEAR` | 2 | `ScaleMode::Bilinear` | 2 |
| `TFB_SCALE_TRILINEAR` | 3 | `ScaleMode::Trilinear` | 3 |

### 10.5 Struct Layout Verification

Critical: C and Rust structs passed across FFI must have identical memory
layout. Verification methods:

1. **Compile-time**: `static_assert!(std::mem::size_of::<FfiColor>() == 4)`
2. **Compile-time**: `static_assert!(std::mem::align_of::<FfiRect>() == 4)`
3. **Test-time**: `assert_eq!(std::mem::offset_of!(FfiRect, extent), 8)`
4. **C-side**: `_Static_assert(sizeof(Color) == 4, "Color size mismatch")`

These assertions catch layout mismatches at compile time rather than
causing silent corruption at runtime.
