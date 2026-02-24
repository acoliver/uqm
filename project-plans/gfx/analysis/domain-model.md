# Domain Model — GFX Drawing-Pipeline Port

Plan ID: `PLAN-20260223-GFX-FULL-PORT`

---

## 1. Entity Model

### 1.1 RustGraphicsState (singleton, static)

```
RustGraphicsState
├── sdl_context: Sdl                     (SDL2 library handle)
├── video: VideoSubsystem                (video subsystem)
├── canvas: Canvas<Window>               (renderer — software)
├── event_pump: EventPump                (event polling)
├── surfaces: [*mut SDL_Surface; 3]      (shared with C)
│   ├── [0] = TFB_SCREEN_MAIN           (game draws here)
│   ├── [1] = TFB_SCREEN_EXTRA          (save/restore, never composited)
│   └── [2] = TFB_SCREEN_TRANSITION     (transition snapshot)
├── format_conv_surf: *mut SDL_Surface   (format template, 0×0)
├── scaled_buffers: [Option<Vec<u8>>; 3] (software scaler output)
├── hq2x: Hq2xScaler                    (HQ2x scaler instance)
├── hq2x_logged: bool                   (one-time log flag)
├── xbrz_logged: bool                   (one-time log flag)
├── flags: c_int                         (GFX init flags)
├── width: u32                           (window width)
├── height: u32                          (window height)
└── fullscreen: bool                     (fullscreen state)
```

### 1.2 SDL_Surface (C-owned memory layout, #[repr(C)])

```
SDL_Surface
├── flags: u32
├── format: *mut c_void          (SDL_PixelFormat*)
├── w: c_int                     (width — always 320 for screens)
├── h: c_int                     (height — always 240 for screens)
├── pitch: c_int                 (bytes per row, may include padding)
├── pixels: *mut c_void          (pixel data — C writes, Rust reads)
├── clip_rect: SDL_Rect
└── refcount: c_int
```

### 1.3 SDL_Rect (C interop, #[repr(C)])

```
SDL_Rect { x: c_int, y: c_int, w: c_int, h: c_int }
```

Must be converted to `sdl2::rect::Rect(i32, i32, u32, u32)` for crate API.

---

## 2. State Transitions

```
                     rust_gfx_init()
    UNINITIALIZED ───────────────────→ INITIALIZED
         │ ↑                                │
         │ │  rust_gfx_init() fails         │ rust_gfx_uninit()
         │ └────────────────────────────────│
         │                                  │
         └──────────────────────────────────┘
```

### 2.1 Uninitialized State

- `RUST_GFX.0.get() == None`
- All FFI functions return safe defaults (null, 0, -1, or void)
- `rust_gfx_init` is the only function that may transition out

### 2.2 Initialized State

- `RUST_GFX.0.get() == Some(RustGraphicsState)`
- All resources are valid
- vtable functions operate normally
- `rust_gfx_uninit` transitions back to Uninitialized

### 2.3 Failed Init (Transient)

- If `rust_gfx_init` fails partway, all allocated resources are freed
- State remains `None` (Uninitialized)
- Subsequent `rust_gfx_init` call may retry

---

## 3. Per-Frame State Machine (vtable call sequence)

```
TFB_SwapBuffers entry
    │
    ▼
┌──────────────────┐
│ 1. Preprocess     │  Set blend=NONE, color=black, clear renderer
└────────┬─────────┘
         ▼
┌──────────────────┐
│ 2. ScreenLayer    │  Upload surfaces[0], render opaque (full screen)
│    (MAIN, 255)    │
└────────┬─────────┘
         ▼
┌──────────────────┐  Only when transition_amount != 255
│ 3. ScreenLayer    │  Upload surfaces[2], render with alpha blend + clip rect
│    (TRANSITION)   │
└────────┬─────────┘
         ▼
┌──────────────────┐  Only when fade_amount != 255
│ 4. ColorLayer     │  Fill rect with (r,g,b,a) for fade-to-black/white
└────────┬─────────┘
         ▼
┌──────────────────┐  Only when system_box_active
│ 5. ScreenLayer    │  Upload surfaces[0], render opaque (system_box rect)
│    (MAIN, clip)   │
└────────┬─────────┘
         ▼
┌──────────────────┐
│ 6. Postprocess    │  canvas.present() — display the composed frame
└──────────────────┘
```

---

## 4. Error Handling Map

| Function | Error Condition | Response | Log? |
|---|---|---|---|
| `rust_gfx_init` | SDL init fail | Free resources, return -1 | Yes (diagnostic) |
| `rust_gfx_init` | Already initialized | Return -1, no state change | Yes |
| `rust_gfx_uninit` | Not initialized | No-op (void return) | No |
| `rust_gfx_screen` | Not initialized | Return immediately | No |
| `rust_gfx_screen` | screen out of range | Return immediately | No |
| `rust_gfx_screen` | surface is null | Return immediately | No |
| `rust_gfx_screen` | pixels null or pitch ≤ 0 | Return immediately | No |
| `rust_gfx_screen` | screen == 1 (EXTRA) | Return immediately (not compositable) | No |
| `rust_gfx_screen` | rect.w < 0 or rect.h < 0 | Return immediately | No |
| `rust_gfx_screen` | texture creation fails | Return immediately | No |
| `rust_gfx_screen` | texture.update fails | Return immediately (no canvas.copy) | No |
| `rust_gfx_color` | Not initialized | Return immediately | No |
| `rust_gfx_color` | rect.w < 0 or rect.h < 0 | Return immediately | No |
| `rust_gfx_preprocess` | Not initialized | Return immediately | No |
| `rust_gfx_postprocess` | Not initialized | Return immediately | No |

---

## 5. Integration Touchpoints

### 5.1 C → Rust (FFI calls)

| C Caller | Rust Symbol | When |
|---|---|---|
| `TFB_InitGraphics` (sdl_common.c:118) | `rust_gfx_init` | Startup |
| `TFB_InitGraphics` (sdl_common.c:129) | `rust_gfx_get_screen_surface` | After init (loop i=0..2) |
| `TFB_InitGraphics` (sdl_common.c:138) | `rust_gfx_get_format_conv_surf` | After init |
| `TFB_UninitGraphics` (sdl_common.c:188) | `rust_gfx_uninit` | Shutdown |
| `Rust_Preprocess` (sdl_common.c:60) | `rust_gfx_preprocess` | Each frame via vtable |
| `Rust_ScreenLayer` (sdl_common.c:75) | `rust_gfx_screen` | Each frame via vtable |
| `Rust_ColorLayer` (sdl_common.c:80) | `rust_gfx_color` | Frames with active fade |
| `Rust_Postprocess` (sdl_common.c:65) | `rust_gfx_postprocess` | Each frame via vtable |
| `Rust_UploadTransitionScreen` (sdl_common.c:70) | `rust_gfx_upload_transition_screen` | Before transitions |

### 5.2 Rust → C (FFI calls from Rust)

| Rust Caller | C Symbol | Purpose |
|---|---|---|
| `rust_gfx_init` | `SDL_CreateRGBSurface` | Create screen surfaces |
| `rust_gfx_uninit` | `SDL_FreeSurface` | Free screen surfaces |
| Various | `rust_bridge_log_msg` | Diagnostic logging |

### 5.3 Rust Internal Dependencies

| Module | Used By | Purpose |
|---|---|---|
| `crate::graphics::pixmap::Pixmap` | ScreenLayer scaling | Intermediate RGBA buffer |
| `crate::graphics::scaling::Hq2xScaler` | ScreenLayer scaling | HQ2x algorithm |
| `xbrz::scale_rgba` | ScreenLayer scaling | xBRZ algorithm |
| `crate::bridge_log::rust_bridge_log_msg` | Init/uninit | Diagnostic logging |

---

## 6. Old Code to Replace/Remove

### 6.1 `rust_gfx_postprocess` — Lines 410–588 (REPLACE)

The entire 170-line upload+scale+present block must be replaced with
`canvas.present()` only. The upload and scaling logic relocates to
`rust_gfx_screen`.

**Before** (current, wrong):
```rust
pub extern "C" fn rust_gfx_postprocess() {
    if let Some(state) = get_gfx_state() {
        // 170 lines: texture_creator, scaling, upload, canvas.copy
        state.canvas.present();
    }
}
```

**After** (correct):
```rust
pub extern "C" fn rust_gfx_postprocess() {
    if let Some(state) = get_gfx_state() {
        state.canvas.present();
    }
}
```

### 6.2 `rust_gfx_screen` — Lines 597–600 (REPLACE)

The no-op body and incorrect comment must be replaced with full
ScreenLayer implementation.

**Before** (current, broken):
```rust
pub extern "C" fn rust_gfx_screen(_screen: c_int, _alpha: u8, _rect: *const SDL_Rect) {
    // The actual drawing is done by C code directly to the SDL surfaces
}
```

**After**: Full implementation (~80-100 lines) with surface upload, alpha
blending, rect conversion, and optional software scaling.

### 6.3 `rust_gfx_color` — Lines 604–606 (REPLACE)

The no-op body must be replaced with ColorLayer implementation.

**Before** (current, broken):
```rust
pub extern "C" fn rust_gfx_color(_r: u8, _g: u8, _b: u8, _a: u8, _rect: *const SDL_Rect) {
    // TODO: Implement fade overlay
}
```

**After**: Full implementation (~20 lines) with blend mode, draw color,
fill rect.

### 6.4 `rust_gfx_preprocess` — Lines 396–406 (MODIFY)

Add `set_blend_mode(None)` before clear. Minor 2-line change.

### 6.5 `rust_gfx_upload_transition_screen` — Lines 592–594 (MODIFY)

Replace "No-op for now" comment with a proper documentation comment
explaining the architectural invariant (no-op because ScreenLayer
unconditionally uploads).
y uploads).
