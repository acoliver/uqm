# Rust GFX Driver Inventory — Finishing the Backend Vtable

**File**: `rust/src/graphics/ffi.rs` (676 lines)
**C Reference**: `sc2/src/libs/graphics/sdl/sdl2_pure.c` (465 lines)
**Date**: 2026-02-23

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [What's Already Working](#2-whats-already-working)
3. [The 5 Vtable Entry Points — C Reference Behavior](#3-the-5-vtable-entry-points--c-reference-behavior)
4. [Data Flow: From Game Drawing to Pixels on Screen](#4-data-flow-from-game-drawing-to-pixels-on-screen)
5. [Surface Layout and Sharing](#5-surface-layout-and-sharing)
6. [Pixel Format Concerns](#6-pixel-format-concerns)
7. [What Each Broken/No-Op Function Needs to Do](#7-what-each-brokenno-op-function-needs-to-do)
8. [Available SDL2 Rust Crate APIs](#8-available-sdl2-rust-crate-apis)
9. [Estimated Scope](#9-estimated-scope)
10. [Risk Areas](#10-risk-areas)
11. [Suggested Implementation Order](#11-suggested-implementation-order)

---

## 1. Executive Summary

The Rust GFX driver in `ffi.rs` replaces the C `sdl2_pure` backend. It owns the SDL2 window, renderer, and screen surfaces. C game code draws directly to shared `SDL_Surface` objects. The 5 vtable entry points are responsible for **compositing** those surfaces into a final frame and presenting it.

**Current status:**

| Function | Status | Severity |
|---|---|---|
| `Postprocess` | [WRONG ARCHITECTURE] Uploads surfaces[0] + scales + presents — must be reduced to just `present()` | Blocks correct composition |
| `Preprocess` | [OK] Clears canvas to black (matches C behavior) — minor tweak for blend mode | Low |
| `ScreenLayer` | [ERROR] NO-OP | Does nothing — core compositing is missing |
| `ColorLayer` | [ERROR] NO-OP | Does nothing — fades don't work |
| `UploadTransitionScreen` | [OK as no-op] Rust driver always re-uploads surfaces, so dirty marking is unnecessary | Low |

**Observed symptom**: The game shows a black screen at runtime.

**Black screen analysis**: The current `Postprocess` does upload `surfaces[0]` and call `canvas.copy` + `present()`, so the Postprocess path alone should theoretically produce visible output if C game code has drawn to `surfaces[0]`. The black screen root cause is not definitively proven to be the no-op vtable functions alone — it may also involve upstream factors such as `TFB_SwapBuffers` early-returning when no bbox/fade/transition changes are signaled, C drawing code not populating surfaces, or timing interactions. **Hypothesis**: Fixing ScreenLayer and restructuring Postprocess will likely resolve the black screen, but this must be verified at runtime.

**Compositing breakage (separate, proven concern)**: Regardless of the black screen root cause, the missing `ScreenLayer` breaks the multi-layer compositing pipeline. Transitions, fades, and system_box overlays cannot work without it. Fixing `ScreenLayer` + reducing `Postprocess` to just `present()` is required for correct rendering.


**WARNING: CRITICAL CONSTRAINT**: When `ScreenLayer` is implemented, `Postprocess` MUST be reduced to only `canvas.present()`. If the current Postprocess upload-and-render logic is kept alongside a working ScreenLayer, the result will be double-rendering with incorrect layer order, clobbering transition/fade/system_box composition.

---

## 2. What's Already Working

### 2.1 Initialization (`rust_gfx_init`, lines 139–315)

Fully functional. Creates:
- SDL2 context, video subsystem, window, software canvas renderer
- Sets logical size to 320×240 (`set_logical_size`)
- 3 real `SDL_Surface` objects via `SDL_CreateRGBSurface` (32bpp, RGBX8888 with `A_MASK=0`)
- 1 `format_conv_surf` (32bpp, RGBA with `A_MASK=0xFF`)
- Soft-scaling buffers for HQ2x/xBRZ when scaler flags are set
- Event pump

### 2.2 Surface Access (lines 356–388)

Working. C code gets real `SDL_Surface*` pointers:
- `rust_gfx_get_sdl_screen()` → `surfaces[0]`
- `rust_gfx_get_transition_screen()` → `surfaces[2]`
- `rust_gfx_get_screen_surface(i)` → `surfaces[i]`
- `rust_gfx_get_format_conv_surf()` → `format_conv_surf`

### 2.3 Postprocess (`rust_gfx_postprocess`, lines 410–588)

**Functionally present but architecturally wrong** — it performs surface upload + scaling + texture rendering + present, all in one function. In the correct vtable architecture, compositing (upload + render per layer) belongs in `ScreenLayer`, and Postprocess should only call `canvas.present()`. Step by step of what it currently does:

1. Creates a streaming texture via `texture_creator.create_texture_streaming(RGBX8888, ...)`
2. **If soft scaler enabled**: Reads `surfaces[0]` pixels, converts RGBX8888→RGBA, runs xBRZ or HQ2x, converts back to RGBX8888, uploads to texture
3. **If no soft scaler**: Reads `surfaces[0]` pixels directly, uploads raw bytes to texture
4. Copies texture to canvas via `canvas.copy(&texture, None, None)` (line 583)
5. Calls `canvas.present()` (line 586)

**Architecture problem**: Postprocess currently only reads `surfaces[0]` (the main screen) and handles everything itself. In the correct vtable flow, `ScreenLayer` composites all layers onto the renderer BEFORE `Postprocess` is called. `Postprocess` should only call `canvas.present()`. The current monolithic approach in Postprocess must be split: upload+render logic moves to `ScreenLayer`, and Postprocess becomes trivial.

### 2.4 Uninit (`rust_gfx_uninit`, lines 318–350)

Working. Proper cleanup with correct drop order.

### 2.5 Event Processing (`rust_gfx_process_events`, lines 614–624)

Working. Polls SDL events, returns 1 on quit.

### 2.6 Utility Functions (lines 630–675)

Working: `toggle_fullscreen`, `is_fullscreen`, `set_gamma` (returns -1, unsupported), `get_width`, `get_height`.

---

## 3. The 5 Vtable Entry Points — C Reference Behavior

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

They are called from `TFB_SwapBuffers` in `sdl_common.c` lines 275–330 in this exact order:

```
1. backend->preprocess(force_redraw, transition_amount, fade_amount)
2. backend->screen(TFB_SCREEN_MAIN, 255, NULL)             // always
3. backend->screen(TFB_SCREEN_TRANSITION, 255-ta, &r)      // if transition active
4. backend->color(r, g, b, a, NULL)                         // if fade active
5. backend->screen(TFB_SCREEN_MAIN, 255, &system_box)      // if system_box active
6. backend->postprocess()
```

### 3.1 C `TFB_SDL2_Preprocess` (`sdl2_pure.c` lines 358–384)

```c
static void
TFB_SDL2_Preprocess (int force_full_redraw, int transition_amount, int fade_amount)
{
    (void) transition_amount;
    (void) fade_amount;

    if (force_full_redraw == TFB_REDRAW_YES)
    {
        SDL2_Screens[TFB_SCREEN_MAIN].updated.x = 0;
        SDL2_Screens[TFB_SCREEN_MAIN].updated.y = 0;
        SDL2_Screens[TFB_SCREEN_MAIN].updated.w = ScreenWidth;
        SDL2_Screens[TFB_SCREEN_MAIN].updated.h = ScreenHeight;
        SDL2_Screens[TFB_SCREEN_MAIN].dirty = TRUE;
    }
    else if (TFB_BBox.valid)
    {
        SDL2_Screens[TFB_SCREEN_MAIN].updated.x = TFB_BBox.region.corner.x;
        SDL2_Screens[TFB_SCREEN_MAIN].updated.y = TFB_BBox.region.corner.y;
        SDL2_Screens[TFB_SCREEN_MAIN].updated.w = TFB_BBox.region.extent.width;
        SDL2_Screens[TFB_SCREEN_MAIN].updated.h = TFB_BBox.region.extent.height;
        SDL2_Screens[TFB_SCREEN_MAIN].dirty = TRUE;
    }

    SDL_SetRenderDrawBlendMode (renderer, SDL_BLENDMODE_NONE);
    SDL_SetRenderDrawColor (renderer, 0, 0, 0, 255);
    SDL_RenderClear (renderer);
}
```

**What it does:**
1. Marks the main screen's dirty region based on `force_full_redraw` or `TFB_BBox`
2. **Always** clears the renderer to opaque black (this is correct — the compositor rebuilds the frame from scratch each time by layering screen textures on top)

**Key insight**: The C driver tracks per-screen dirty rects and textures persistently. The Rust driver does NOT have persistent per-screen textures or dirty tracking. This is a design difference. Since the Rust driver recreates the main texture each frame in Postprocess anyway, the dirty-rect optimization is less critical. However, the clearing is still needed.

**Rust driver divergence (intentional non-parity)**: The Rust driver has no dirty-rect tracking model. The C driver uses `dirty`/`updated` fields on per-screen state to avoid re-uploading unchanged surfaces — this is central to the C ScreenLayer's upload logic. The Rust driver instead re-uploads the full surface unconditionally on every `ScreenLayer` call. This is an intentional design simplification that trades a small performance cost for simpler code. The `force_redraw` and dirty-rect parameters in Preprocess are therefore unused in the Rust driver (the C driver also ignores `transition_amount`/`fade_amount` in SDL2 mode).

### 3.2 C `TFB_SDL2_Unscaled_ScreenLayer` (`sdl2_pure.c` lines 386–404)

```c
static void
TFB_SDL2_Unscaled_ScreenLayer (SCREEN screen, Uint8 a, SDL_Rect *rect)
{
    SDL_Texture *texture = SDL2_Screens[screen].texture;
    if (SDL2_Screens[screen].dirty)
    {
        TFB_SDL2_UpdateTexture (texture, SDL_Screens[screen], &SDL2_Screens[screen].updated);
    }
    if (a == 255)
    {
        SDL_SetTextureBlendMode (texture, SDL_BLENDMODE_NONE);
    }
    else
    {
        SDL_SetTextureBlendMode (texture, SDL_BLENDMODE_BLEND);
        SDL_SetTextureAlphaMod (texture, a);
    }
    SDL_RenderCopy (renderer, texture, rect, rect);
}
```

**What it does step by step:**
1. Gets the persistent texture for this screen index
2. If dirty, uploads the SDL_Surface pixels to the texture (`TFB_SDL2_UpdateTexture`)
3. Sets blend mode: `NONE` if alpha=255 (opaque), `BLEND` + alpha mod if semi-transparent
4. Copies the texture to the renderer with `SDL_RenderCopy(renderer, texture, rect, rect)` — source and dest rect are the same (logical coordinates)

**Critical**: This is the function that actually puts game graphics on screen. Without it, the renderer is just a black rectangle.

### 3.3 C `TFB_SDL2_Scaled_ScreenLayer` (`sdl2_pure.c` lines 406–445)

Same as unscaled but runs the software scaler first and doubles the source rect coordinates for the 2x-resolution texture. Not critical for initial implementation — the Rust driver already handles scaling in Postprocess.

### 3.4 C `TFB_SDL2_ColorLayer` (`sdl2_pure.c` lines 447–454)

```c
static void
TFB_SDL2_ColorLayer (Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect)
{
    SDL_SetRenderDrawBlendMode (renderer, a == 255 ? SDL_BLENDMODE_NONE
            : SDL_BLENDMODE_BLEND);
    SDL_SetRenderDrawColor (renderer, r, g, b, a);
    SDL_RenderFillRect (renderer, rect);
}
```

**What it does:**
1. Sets blend mode on the renderer: `NONE` if opaque, `BLEND` if semi-transparent
2. Sets draw color to (r, g, b, a)
3. Fills the rect (or full screen if rect is NULL)

**Called from** `TFB_SwapBuffers` for fades:
- Fade to black: `color(0, 0, 0, 255 - fade_amount, NULL)` where `fade_amount < 255`
- Fade to white: `color(255, 255, 255, fade_amount - 255, NULL)` where `fade_amount > 255`

### 3.5 C `TFB_SDL2_UploadTransitionScreen` (`sdl2_pure.c` lines 303–311)

```c
static void
TFB_SDL2_UploadTransitionScreen (void)
{
    SDL2_Screens[TFB_SCREEN_TRANSITION].updated.x = 0;
    SDL2_Screens[TFB_SCREEN_TRANSITION].updated.y = 0;
    SDL2_Screens[TFB_SCREEN_TRANSITION].updated.w = ScreenWidth;
    SDL2_Screens[TFB_SCREEN_TRANSITION].updated.h = ScreenHeight;
    SDL2_Screens[TFB_SCREEN_TRANSITION].dirty = TRUE;
}
```

**What it does**: Marks the entire transition screen surface as dirty, so the next `ScreenLayer(TFB_SCREEN_TRANSITION, ...)` call will upload it to the GPU texture.

**Context**: Before a screen transition, C code copies the current main screen to `SDL_Screens[2]` (the transition screen). Then `UploadTransitionScreen` is called to flag that the texture needs updating. During the transition, `ScreenLayer(TRANSITION, alpha, clip_rect)` is called each frame to overlay the old image.

### 3.6 C `TFB_SDL2_Postprocess` (`sdl2_pure.c` lines 456–463)

```c
static void
TFB_SDL2_Postprocess (void)
{
    if (GfxFlags & TFB_GFXFLAGS_SCANLINES)
        TFB_SDL2_ScanLines ();
    SDL_RenderPresent (renderer);
}
```

**What it does**: Optionally draws scanlines, then presents the frame. The Rust driver's postprocess is more complex because it must also **upload** the surface pixels to a texture (the C driver does this in ScreenLayer).

---

## 4. Data Flow: From Game Drawing to Pixels on Screen

### 4.1 C Game Code Drawing Phase

```
C game logic → TFB_DrawScreen_* commands → dcqueue.c processes them →
  draws to SDL_Screens[0] (main screen surface) using SDL primitives →
  TFB_BBox tracks dirty region
```

The surfaces are shared: Rust creates them, C gets pointers via `rust_gfx_get_screen_surface()`, C draws directly to pixel memory.

### 4.2 Frame Presentation Phase (TFB_SwapBuffers)

```
TFB_SwapBuffers called from dcqueue.c
  ├── Preprocess(force, transition, fade)    → Clear renderer
  ├── ScreenLayer(MAIN, 255, NULL)           → Upload surfaces[0] to texture, render
  ├── ScreenLayer(TRANSITION, alpha, &r)     → Upload surfaces[2], alpha-blend over
  ├── ColorLayer(r, g, b, a, NULL)           → Draw fade overlay
  ├── ScreenLayer(MAIN, 255, &system_box)    → Re-draw system area on top
  └── Postprocess()                          → Present frame
```

### 4.3 Key Architectural Difference: C vs Rust

**C sdl2_pure driver**: Maintains persistent `SDL_Texture` objects per screen. `ScreenLayer` uploads the dirty region and renders the texture. `Postprocess` just calls `SDL_RenderPresent`.

**Rust driver (current)**: Has NO persistent textures. Creates a temporary texture in `Postprocess`, reads `surfaces[0]` pixels, uploads, renders, presents. `ScreenLayer` is a no-op.

**What needs to change**: The Rust driver must either:
- **(Option A)** Implement `ScreenLayer` to upload each surface to a temporary texture and render it (compositing onto the renderer). Move `present()` call to be the only thing in `Postprocess`. Scaling moves to `ScreenLayer`.
- **(Option B)** Keep the upload-in-postprocess approach but have `ScreenLayer` composite surfaces onto `surfaces[0]` using CPU blitting first, then let Postprocess upload the composited result.

**Option A** is recommended — it matches the C architecture, uses GPU compositing (SDL2 renderer handles alpha blending natively), and is cleaner. The Rust driver uses a software canvas renderer anyway, so "GPU" here means SDL2's software renderer — still faster than manual pixel blitting.

---

## 5. Surface Layout and Sharing

### 5.1 The Three Screen Surfaces

Defined in `tfb_draw.h` (lines 27–33):

```c
typedef enum {
    TFB_SCREEN_MAIN,        // 0 - Primary game screen, C draws here
    TFB_SCREEN_EXTRA,       // 1 - Extra buffer (save/restore operations)
    TFB_SCREEN_TRANSITION,  // 2 - Holds old screen during transitions

    TFB_GFX_NUMSCREENS      // 3
} SCREEN;
```

All created identically in `rust_gfx_init` (lines 239–266):
- `SDL_CreateRGBSurface(0, 320, 240, 32, R_MASK, G_MASK, B_MASK, 0)` — 32bpp, no alpha channel
- Global C pointers `SDL_Screens[]`, `SDL_Screen`, `TransitionScreen` are set in `sdl_common.c` lines 127–137

### 5.2 The format_conv_surf

Created in `rust_gfx_init` (lines 269–281):
- `SDL_CreateRGBSurface(0, 0, 0, 32, R_MASK, G_MASK, B_MASK, A_MASK_ALPHA)` — 0×0 size, but has alpha mask
- Used by C code for format conversion reference (what pixel format to use for sprites/fonts)
- Not a rendering target — it's a template surface

### 5.3 Surface Sharing Mechanism

```
Rust (ffi.rs)                          C (sdl_common.c)
─────────────                          ─────────────────
Creates SDL_Surface via C API  ──→     Gets pointer via rust_gfx_get_screen_surface(i)
state.surfaces[i]              ──→     SDL_Screens[i] = rust_gfx_get_screen_surface(i)
                                       SDL_Screen = SDL_Screens[0]
                                       TransitionScreen = SDL_Screens[2]
```

C code writes directly to `SDL_Screens[i]->pixels`. No synchronization (single-threaded graphics).

---

## 6. Pixel Format Concerns

### 6.1 The RGBX8888 Format

Surface masks (`ffi.rs` lines 29–33):
```rust
const R_MASK: u32 = 0xFF000000;  // R in bits 24-31
const G_MASK: u32 = 0x00FF0000;  // G in bits 16-23
const B_MASK: u32 = 0x0000FF00;  // B in bits 8-15
const A_MASK_SCREEN: u32 = 0x00000000; // no alpha on screen surfaces
```

C reference (`sdl2_pure.c` lines 43–53, little-endian):
```c
#define A_MASK 0x000000ff
#define B_MASK 0x0000ff00
#define G_MASK 0x00ff0000
#define R_MASK 0xff000000
```

**These match on little-endian (macOS arm64 and x86_64 are both little-endian).** In memory, a pixel is stored as bytes `[X/A, B, G, R]` where byte 0 is at the lowest address.

### 6.2 Texture Format

The Rust postprocess creates textures as `PixelFormatEnum::RGBX8888` (line 437). The C driver does the same: `SDL_PIXELFORMAT_RGBX8888` (`sdl2_pure.c` line 230, 252).

### 6.3 Gotchas

1. **Screen surfaces have NO alpha channel** (`A_MASK=0`). This means `SDL_BlitSurface` with `SDL_BLENDMODE_BLEND` will treat source as fully opaque unless you use `SDL_SetSurfaceAlphaMod` to set a per-surface alpha.
2. **The format_conv_surf HAS alpha** (`A_MASK=0xFF`). Sprites loaded via this format can have per-pixel alpha.
3. **The Postprocess pixel swizzle** (lines 481–495, 513–528) converts between RGBX8888 memory layout and RGBA for xBRZ/HQ2x. This is only needed for software scalers.
4. **If using SDL2 renderer compositing** (Option A approach), pixel format conversion is unnecessary — SDL2 handles it internally when uploading surface pixels to textures via `SDL_UpdateTexture`.

---

## 7. What Each Broken/No-Op Function Needs to Do

### 7.1 `rust_gfx_preprocess` — MINOR TWEAK

**Current code** (lines 396–406):
```rust
pub extern "C" fn rust_gfx_preprocess(
    _force_redraw: c_int,
    _transition_amount: c_int,
    _fade_amount: c_int,
) {
    if let Some(state) = get_gfx_state() {
        state.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        state.canvas.clear();
    }
}
```

**The clearing behavior is correct** — the C `TFB_SDL2_Preprocess` also always clears the renderer to black before the compositing calls. The parameters `transition_amount` and `fade_amount` are not used by the C SDL2 backend either (explicitly cast to `void`). The `force_redraw` parameter controls dirty rect tracking which the Rust driver doesn't use (it re-uploads full surfaces each frame). The only needed tweak is minor:

1. The parameters `transition_amount` and `fade_amount` are **not used** by the C SDL2 Preprocess either — it explicitly casts them to `(void)`. Keep the leading underscores since they are intentionally unused.
2. The `force_redraw` parameter: The C driver uses it to decide the dirty rect. Since the Rust driver doesn't track dirty rects (it re-uploads the full surface each frame), this parameter is also not needed. Keep the leading underscore.

**Verdict**: Preprocess is actually **functionally okay** once ScreenLayer works. The canvas clear is correct. It could optionally set the blend mode to `None` before clearing (the C driver does `SDL_SetRenderDrawBlendMode(renderer, SDL_BLENDMODE_NONE)` before clearing), but this is unlikely to matter for a clear operation.

**Recommended change** (~5 lines):
```rust
pub extern "C" fn rust_gfx_preprocess(
    _force_redraw: c_int,
    _transition_amount: c_int,
    _fade_amount: c_int,
) {
    if let Some(state) = get_gfx_state() {
        state.canvas.set_blend_mode(sdl2::render::BlendMode::None);
        state.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 255));
        state.canvas.clear();
    }
}
```

### 7.2 `rust_gfx_screen` (ScreenLayer) — IMPLEMENT

**Current code** (lines 597–600):
```rust
pub extern "C" fn rust_gfx_screen(_screen: c_int, _alpha: u8, _rect: *const SDL_Rect) {
    // The actual drawing is done by C code directly to the SDL surfaces
}
```

**This comment is WRONG.** C code draws to `SDL_Screens[screen]`, but the backend must composite that surface onto the renderer. Without this, nothing appears.

**Required implementation** — matching `TFB_SDL2_Unscaled_ScreenLayer`:

1. Validate `screen` is in range `[0, TFB_GFX_NUMSCREENS)`
2. Get `surfaces[screen]` — the `SDL_Surface*` that C drew to
3. Create a temporary streaming texture (RGBX8888, 320×240)
4. Upload the surface's pixel data to the texture via `texture.update()`
5. If `alpha == 255`: set texture blend mode to `None`
6. If `alpha < 255`: set texture blend mode to `Blend`, set alpha mod to `alpha`
7. Convert the `rect` pointer to an `Option<sdl2::rect::Rect>`
8. Call `canvas.copy(&texture, rect, rect)` — source and dest rect are the same
9. If soft scaling is enabled, the texture should be at scaled resolution and the source rect should be scaled accordingly (match C `TFB_SDL2_Scaled_ScreenLayer` behavior)

**For the non-scaled path** (~40-50 lines of Rust):

```rust
pub extern "C" fn rust_gfx_screen(screen: c_int, alpha: u8, rect: *const SDL_Rect) {
    if screen < 0 || screen >= TFB_GFX_NUMSCREENS as c_int {
        return;
    }
    let Some(state) = get_gfx_state() else { return };
    let src_surface = state.surfaces[screen as usize];
    if src_surface.is_null() {
        return;
    }

    let texture_creator = state.canvas.texture_creator();

    // Create texture and upload surface pixels
    let use_soft_scaler = state.scaled_buffers[screen as usize].is_some();
    let (tex_w, tex_h, scale_factor) = if use_soft_scaler {
        // Determine scale factor from GFX flags (matching ffi.rs line 304):
        // flags & (1<<8) => xBRZ3 (3x), flags & (1<<9) => xBRZ4 (4x), else HQ2x (2x)
        // Note: bit 8 is checked first — if both set, 3x wins (matches existing Rust code)
        let sf = if (state.flags & (1 << 8)) != 0 { 3 }
                 else if (state.flags & (1 << 9)) != 0 { 4 }
                 else { 2 };
        (SCREEN_WIDTH * sf, SCREEN_HEIGHT * sf, sf)
    } else {
        (SCREEN_WIDTH, SCREEN_HEIGHT, 1)
    };

    let Ok(mut texture) = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGBX8888, tex_w, tex_h,
    ) else { return };

    // Upload pixel data (with optional scaling)
    if use_soft_scaler {
        // Run scaler, upload scaled buffer
        // ... (reuse existing scaling logic from postprocess)
    } else {
        unsafe {
            let surf = &*src_surface;
            if !surf.pixels.is_null() && surf.pitch > 0 {
                let pitch = surf.pitch as usize;
                let total = pitch * SCREEN_HEIGHT as usize;
                let pixels = std::slice::from_raw_parts(surf.pixels as *const u8, total);
                let _ = texture.update(None, pixels, pitch);
            }
        }
    }

    // Set blend mode
    if alpha == 255 {
        texture.set_blend_mode(sdl2::render::BlendMode::None);
    } else {
        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        texture.set_alpha_mod(alpha);
    }

    // Convert rect
    let sdl_rect = if rect.is_null() {
        None
    } else {
        let r = unsafe { &*rect };
        Some(sdl2::rect::Rect::new(r.x, r.y, r.w as u32, r.h as u32))
    };

    // For scaled path, source rect needs to be multiplied by scale_factor
    let src_rect = if use_soft_scaler && sdl_rect.is_some() {
        let r = sdl_rect.unwrap();
        Some(sdl2::rect::Rect::new(
            r.x() * scale_factor as i32,
            r.y() * scale_factor as i32,
            r.width() * scale_factor,
            r.height() * scale_factor,
        ))
    } else {
        sdl_rect
    };

    let _ = state.canvas.copy(&texture, src_rect, sdl_rect);
}
```

### 7.3 `rust_gfx_color` (ColorLayer) — IMPLEMENT

**Current code** (lines 604–606):
```rust
pub extern "C" fn rust_gfx_color(_r: u8, _g: u8, _b: u8, _a: u8, _rect: *const SDL_Rect) {
    // TODO: Implement fade overlay
}
```

**Required implementation** — matching `TFB_SDL2_ColorLayer`:

```rust
pub extern "C" fn rust_gfx_color(r: u8, g: u8, b: u8, a: u8, rect: *const SDL_Rect) {
    let Some(state) = get_gfx_state() else { return };

    if a == 255 {
        state.canvas.set_blend_mode(sdl2::render::BlendMode::None);
    } else {
        state.canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    }
    state.canvas.set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));

    if rect.is_null() {
        let _ = state.canvas.fill_rect(None);
    } else {
        let r_val = unsafe { &*rect };
        let _ = state.canvas.fill_rect(Some(
            sdl2::rect::Rect::new(r_val.x, r_val.y, r_val.w as u32, r_val.h as u32)
        ));
    }
}
```

**~15 lines**. Straightforward.

### 7.4 `rust_gfx_upload_transition_screen` — IMPLEMENT

**Current code** (lines 592–594):
```rust
pub extern "C" fn rust_gfx_upload_transition_screen() {
    // No-op for now
}
```

**Required implementation**: The C driver simply marks the transition screen as dirty. Since the Rust driver doesn't track dirty flags (it re-uploads every time), this function can remain a no-op **if** `ScreenLayer` always uploads.

However, if we add dirty tracking for optimization, this should set `dirty[TFB_SCREEN_TRANSITION] = true`. For now:

```rust
pub extern "C" fn rust_gfx_upload_transition_screen() {
    // The Rust driver re-uploads surface data on every ScreenLayer call,
    // so no explicit dirty marking is needed.
}
```

**~0 lines of real change needed** — it's correctly a no-op for our architecture.

**INVARIANT**: This no-op is safe ONLY because `ScreenLayer` always uploads the full surface unconditionally. If `ScreenLayer` is ever optimized to use dirty-region tracking (skipping upload when surface hasn't changed), then `UploadTransitionScreen` MUST be changed to set a dirty flag for `TFB_SCREEN_TRANSITION`. This dependency must be maintained.

### 7.5 `rust_gfx_postprocess` — REFACTOR

**Current code** (lines 410–588): 178 lines of working upload+scale+present code.

**What needs to change**: When ScreenLayer handles uploading and rendering each surface, Postprocess should only need to call `canvas.present()`. The scaling logic must move from Postprocess to ScreenLayer.

**However**, a simpler incremental approach is:

1. **Keep Postprocess as-is** for the first iteration — it already uploads surfaces[0] and presents
2. Make ScreenLayer do the compositing work described above
3. **Problem**: With the current design, both ScreenLayer and Postprocess would upload surfaces[0] — ScreenLayer to composite, and Postprocess to present with scaling
4. **Resolution**: Remove the texture upload and copy from Postprocess, keeping only `canvas.present()`. All compositing (including scaling) happens in ScreenLayer.

**Recommended final state of Postprocess**:
```rust
pub extern "C" fn rust_gfx_postprocess() {
    if let Some(state) = get_gfx_state() {
        state.canvas.present();
    }
}
```

**The scaling logic** from the current Postprocess (~170 lines) should be moved into `rust_gfx_screen` (ScreenLayer).

---

## 8. Available SDL2 Rust Crate APIs

The project uses `sdl2 = "0.37"`. Relevant APIs on `Canvas<Window>`:

### 8.1 Canvas Rendering

```rust
// Already used in ffi.rs:
canvas.set_draw_color(Color::RGBA(r, g, b, a));
canvas.clear();                           // Clears to draw color
canvas.copy(&texture, src_rect, dst_rect); // Renders texture
canvas.present();                          // Presents frame

// Needed for ColorLayer:
canvas.set_blend_mode(BlendMode::None);    // or BlendMode::Blend
canvas.fill_rect(Option<Rect>);            // Fills rect with draw color

// TextureCreator (already used):
let tc = canvas.texture_creator();
let mut texture = tc.create_texture_streaming(format, w, h);
texture.update(rect, pixels, pitch);
texture.set_blend_mode(BlendMode::Blend);
texture.set_alpha_mod(alpha);
```

### 8.2 Blend Modes

```rust
use sdl2::render::BlendMode;
BlendMode::None   // No blending, overwrites
BlendMode::Blend  // Alpha blending: dstRGB = srcRGB * srcA + dstRGB * (1-srcA)
BlendMode::Add    // Additive: dstRGB = srcRGB * srcA + dstRGB
BlendMode::Mod    // Color modulate: dstRGB = srcRGB * dstRGB
```

### 8.3 Rect Conversion

The Rust FFI uses a custom `SDL_Rect` (lines 53–60) for C interop. The sdl2 crate has its own `sdl2::rect::Rect`. Conversion:

```rust
let sdl2_rect = sdl2::rect::Rect::new(c_rect.x, c_rect.y, c_rect.w as u32, c_rect.h as u32);
```

Note: `sdl2::rect::Rect::new` takes `(i32, i32, u32, u32)` — width/height are unsigned.

### 8.4 NOT Available via sdl2 Crate (Raw C API)

The following SDL2 C functions used by the C driver are NOT wrapped by the `sdl2` Rust crate's safe API but **are not needed** for the texture-based approach:

- `SDL_BlitSurface` — not needed; we upload surfaces to textures instead
- `SDL_SetSurfaceAlphaMod` — not needed; we use texture alpha mod
- `SDL_SetSurfaceBlendMode` — not needed; same reason
- `SDL_FillRect` on surfaces — not needed; we use `canvas.fill_rect()`

If surface blitting were needed, raw FFI declarations would be required:
```rust
extern "C" {
    fn SDL_UpperBlit(src: *mut SDL_Surface, srcrect: *const SDL_Rect,
                     dst: *mut SDL_Surface, dstrect: *mut SDL_Rect) -> c_int;
    fn SDL_SetSurfaceBlendMode(surface: *mut SDL_Surface, blendMode: c_int) -> c_int;
    fn SDL_SetSurfaceAlphaMod(surface: *mut SDL_Surface, alpha: u8) -> c_int;
}
```

**These are NOT needed for the recommended approach** — the sdl2 crate's `Canvas` + `Texture` API handles everything.

---

## 9. Estimated Scope

### 9.1 Functions to Change

| Function | Action | Est. Lines Changed |
|---|---|---|
| `rust_gfx_preprocess` | Minor fix: add `set_blend_mode(None)` | ~3 lines |
| `rust_gfx_screen` | Full implementation: upload surface, set blend, render | ~60-80 lines |
| `rust_gfx_color` | Full implementation: set blend, fill rect | ~15 lines |
| `rust_gfx_upload_transition_screen` | Add comment explaining no-op | ~2 lines |
| `rust_gfx_postprocess` | Refactor: move upload logic to screen, keep present | -150 lines removed, ~5 lines remain |
| Scaling refactor | Move xBRZ/HQ2x from postprocess to screen | ~0 net (relocated) |

**Total**: ~80-100 net new lines, ~150 lines relocated, ~5 lines removed.

### 9.2 New Code Needed

1. `rust_gfx_screen` body: ~60-80 lines
2. `rust_gfx_color` body: ~15 lines
3. Helper: `c_rect_to_sdl2_rect` conversion function: ~8 lines
4. `rust_gfx_preprocess` tweak: ~3 lines
5. `rust_gfx_postprocess` simplification: ~5 lines (replacing ~170)

### 9.3 Code to Delete/Relocate

The 170-line upload+scale block in `rust_gfx_postprocess` (lines 411–587) needs to be either:
- Moved into `rust_gfx_screen` (if scaling per-layer is desired)
- Or kept as a helper function called from `rust_gfx_screen`

---

## 10. Risk Areas

### 10.0 CRITICAL: Double-Render Guard (Postprocess vs ScreenLayer)

When `ScreenLayer` is implemented, `Postprocess` MUST be reduced to just `canvas.present()`. If the current 170-line upload+render block in Postprocess is left intact, both `ScreenLayer` and `Postprocess` will render `surfaces[0]` to the canvas — resulting in double-rendering with wrong layer order. Specifically:
- ScreenLayer renders: main -> transition (alpha) -> fade color -> system_box
- Then Postprocess would re-upload surfaces[0] ON TOP, clobbering the transition/fade/system_box composition

**Runtime invariant**: At runtime, ScreenLayer and Postprocess must not both upload/render surfaces. If ScreenLayer renders layers onto the canvas AND Postprocess also uploads/renders surfaces[0], the result will be double-rendering with incorrect layer order. The concrete requirement: when ScreenLayer is active, Postprocess must only call `present()` — no surface upload or `canvas.copy`.

### 10.1 HIGH RISK: Texture Lifetime and Ownership

The sdl2 crate has strict lifetime rules: `Texture` cannot outlive its `TextureCreator`, which cannot outlive its `Canvas`. Currently, `rust_gfx_postprocess` creates a `TextureCreator` each frame:

```rust
let texture_creator = state.canvas.texture_creator();  // line 413
```

This works because the texture is dropped before the function returns. If `ScreenLayer` creates textures per-call the same way, each is dropped at end of `rust_gfx_screen` — this should be safe but means **3 texture creates per frame** in the worst case (main, transition, main again for system_box).

**Mitigation**: This is fine for a software renderer. If performance matters later, persistent textures can be stored (with unsafe lifetime tricks).

### 10.2 HIGH RISK: Scaling Architecture Change

Moving scaling from Postprocess to ScreenLayer means the scaled buffer logic must work per-screen-index, not just for index 0. Currently:
- `state.scaled_buffers[0]` is used in Postprocess
- `state.scaled_buffers[1]` and `[2]` exist but are never read

After the change, `ScreenLayer(screen=0, ...)` and `ScreenLayer(screen=2, ...)` would both need to scale. The buffers already exist, so this is mostly wiring.

**Gotcha**: The C SDL2 driver marks screen index 1 (`TFB_SCREEN_EXTRA`) as `active = FALSE` (`sdl2_pure.c` line 178). It never creates a texture for it. The Rust driver should skip creating scaled buffers and textures for screen 1 when it's not active.

### 10.3 MEDIUM RISK: Alpha Blending Correctness

When `ScreenLayer(TRANSITION, 128, &clip_rect)` is called:
- The transition texture should be blended with alpha=128 over whatever was already rendered (the main screen)
- The `clip_rect` constrains which area of the screen is affected
- SDL2's `BLEND` mode: `dst = src * srcA + dst * (1 - srcA)` — this is correct for the use case

**Potential issue**: The screen surfaces have `A_MASK=0`, meaning every pixel has alpha=0 in the surface. When uploading to an `RGBX8888` texture, SDL2 ignores the alpha channel. The texture-level alpha mod (`SDL_SetTextureAlphaMod`) applies a uniform alpha to the whole texture. This matches the C behavior.

### 10.4 MEDIUM RISK: Rect NULL vs Full Screen

C passes `NULL` for "whole screen". In Rust:
- `canvas.copy(&texture, None, None)` = full texture → full canvas
- `canvas.fill_rect(None)` = fill entire canvas
- Must handle the `*const SDL_Rect` being null correctly (already standard in Rust FFI)

### 10.5 LOW RISK: Multiple ScreenLayer Calls Per Frame

`TFB_SwapBuffers` calls `ScreenLayer` up to 3 times per frame:
1. `screen(MAIN, 255, NULL)` — always
2. `screen(TRANSITION, alpha, &clip_rect)` — during transitions
3. `screen(MAIN, 255, &system_box)` — when system UI is active

Call #3 re-renders the main screen into just the system_box area. This works because SDL2 renderer compositing is additive per-call — each `RenderCopy` draws over what's already there. The clip rect ensures only that region is affected.

**No risk here** as long as `ScreenLayer` creates a fresh texture each call (or uploads the same persistent one).

### 10.6 MEDIUM RISK: Additional Backend Call Paths

Besides the main `dcqueue.c -> TFB_SwapBuffers` path, the backend vtable is also triggered from:
- **Expose events**: `TFB_ProcessEvents` in `sdl_common.c` can call `TFB_SwapBuffers(TFB_REDRAW_EXPOSE)` when the window is exposed/uncovered — this triggers a full redraw
- **Fade animation**: `TFB_FlushGraphics` in `dcqueue.c` can call `TFB_SwapBuffers(TFB_REDRAW_FADING)` even when the draw command queue is empty, to animate ongoing fades/transitions
- **Transition upload**: `TFB_UploadTransitionScreen` wrapper in `sdl_common.c` is called by C code before starting a screen transition

These paths mean the backend can be called more frequently than just once per game frame. The implementation must be re-entrant safe and handle all redraw modes correctly.

### 10.7 LOW RISK: Scanlines Not Implemented

The C `TFB_SDL2_Postprocess` applies scanline overlay if `GfxFlags & TFB_GFXFLAGS_SCANLINES`. The Rust Postprocess does not implement scanlines. This is not black-screen critical but is part of full backend parity.

### 10.8 LOW RISK: Pixel Format on Big-Endian

The masks are currently hardcoded for little-endian (matching macOS x86/ARM). Big-endian would need different masks. This is a pre-existing limitation, not introduced by this work.

---

## 11. Suggested Implementation Order

### Phase 1: Get Something Visible (~80 lines)

1. **Implement `rust_gfx_color`** (~15 lines) — simplest, enables fades
2. **Implement `rust_gfx_screen`** (unscaled path only, ~50 lines) — upload surface to texture, set blend mode, `canvas.copy()`
3. **Simplify `rust_gfx_postprocess`** — strip down to just `canvas.present()`, remove the 170-line upload block
4. **Fix `rust_gfx_preprocess`** — add `set_blend_mode(None)` before clear

**After Phase 1**: Game should be visible with no soft scaling (hardware scaling via SDL2 logical size still applies), fades should work, transitions should work. If soft-scaler flags (HQ2x/xBRZ) are active at runtime, the unscaled-only ScreenLayer will render at native 320x240 resolution without software upscaling — visually correct but lower quality until Phase 2.

### Phase 2: Restore Scaling (~60 lines relocated)

5. **Move xBRZ/HQ2x scaling into `rust_gfx_screen`** — when `state.scaled_buffers[screen]` exists, run the scaler, upload the scaled buffer to a 2x texture, adjust source rect
6. **Handle the scaled source rect** — multiply by scale factor when a clip rect is provided (matching `TFB_SDL2_Scaled_ScreenLayer`)

**After Phase 2**: Full feature parity with current driver, but now with working compositing.

### Phase 3: Polish (optional)

7. **Add dirty tracking** — avoid re-uploading unchanged surfaces each frame (optimization)
8. **Add `upload_transition_screen` dirty flag** — set a flag that `rust_gfx_screen` checks before uploading screen 2
9. **Scanline support** — draw semi-transparent horizontal lines at 2x resolution (matches `TFB_SDL2_ScanLines`)

---

## Appendix A: Key Constants and Enums

From `gfx_common.h`:

```c
// Redraw modes (passed to preprocess via TFB_SwapBuffers)
TFB_REDRAW_NO     = 0  // No redraw needed
TFB_REDRAW_FADING = 1  // Fading in progress
TFB_REDRAW_EXPOSE = 2  // Window exposed, redraw everything
TFB_REDRAW_YES    = 3  // Force full redraw

// GFX flags
TFB_GFXFLAGS_FULLSCREEN       = 1<<0  // 0x01
TFB_GFXFLAGS_SHOWFPS          = 1<<1  // 0x02
TFB_GFXFLAGS_SCANLINES        = 1<<2  // 0x04
TFB_GFXFLAGS_SCALE_BILINEAR   = 1<<3  // 0x08
TFB_GFXFLAGS_SCALE_BIADAPT    = 1<<4  // 0x10
TFB_GFXFLAGS_SCALE_BIADAPTADV = 1<<5  // 0x20
TFB_GFXFLAGS_SCALE_TRISCAN    = 1<<6  // 0x40
TFB_GFXFLAGS_SCALE_HQXX       = 1<<7  // 0x80
TFB_GFXFLAGS_SCALE_XBRZ3      = 1<<8  // 0x100
TFB_GFXFLAGS_SCALE_XBRZ4      = 1<<9  // 0x200
```

## Appendix B: Acceptance Criteria

The implementation is complete when all of the following are verified against `TFB_SwapBuffers` call sequence in `sdl_common.c`:

1. **Main layer visible**: `screen(TFB_SCREEN_MAIN, 255, NULL)` renders the game screen — the game is no longer a black window
2. **Transition overlay visible**: `screen(TFB_SCREEN_TRANSITION, alpha, &clip_rect)` renders the old screen with alpha blending during screen transitions (e.g. entering/exiting menus)
3. **Fade overlay visible**: `color(r, g, b, a, NULL)` draws a semi-transparent colored rectangle for fade-to-black / fade-to-white effects
4. **System box redraw**: `screen(TFB_SCREEN_MAIN, 255, &system_box)` re-renders just the system area on top of fades/transitions
5. **No postprocess clobber**: `Postprocess` only calls `canvas.present()` — it does NOT upload/render any surfaces, which would overwrite the layered composition from steps 1-4
6. **Expose redraw works**: Window un-minimize / un-occlude triggers `TFB_SwapBuffers(TFB_REDRAW_EXPOSE)` and correctly redraws
7. **Fade animation works**: `TFB_FlushGraphics` fade path calls `TFB_SwapBuffers(TFB_REDRAW_FADING)` and correctly animates

**Explicitly out of scope for black-screen fix**: Scanline rendering (`TFB_GFXFLAGS_SCANLINES`) is not implemented in the Rust backend and is not required for the black-screen fix. It is a Phase 3 polish item.

## Appendix C: File Cross-Reference

| File | Role | Key Sections |
|---|---|---|
| `rust/src/graphics/ffi.rs` | Rust GFX driver (this file) | All vtable functions |
| `sc2/src/libs/graphics/sdl/sdl2_pure.c` | C SDL2 reference driver | Lines 358-463: all 5 vtable functions |
| `sc2/src/libs/graphics/sdl/sdl_common.c` | Vtable wiring + TFB_SwapBuffers | Lines 58-92: rust_backend vtable, Lines 275-330: SwapBuffers |
| `sc2/src/libs/graphics/sdl/sdl_common.h` | TFB_GRAPHICS_BACKEND struct def | Lines 30-36 |
| `sc2/src/libs/graphics/sdl/rust_gfx.h` | C header for Rust FFI functions | Lines 29-34: vtable function declarations |
| `sc2/src/libs/graphics/gfx_common.h` | Flags, enums, globals | Lines 37-63: redraw modes, gfx flags |
| `sc2/src/libs/graphics/tfb_draw.h` | SCREEN enum, TFB_GFX_NUMSCREENS | Lines 27-33 |
| `sc2/src/libs/graphics/bbox.h` | TFB_BoundingBox struct | Lines 29-34 |
