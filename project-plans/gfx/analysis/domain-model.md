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

---

## 7. Drawing Pipeline Entity Model (Expanded Scope)

The expanded port (PLAN-20260223-GFX-FULL-PORT) replaces the entire C
drawing pipeline, not just the vtable. This section documents entities
beyond the presentation layer.

### 7.1 PixelCanvas Trait

```
trait PixelCanvas
├── fn width(&self) -> u32
├── fn height(&self) -> u32
├── fn pitch(&self) -> usize
├── fn pixels(&self) -> &[u8]
├── fn pixels_mut(&mut self) -> &mut [u8]
└── fn format(&self) -> PixelFormat
```

Unified abstraction over pixel buffers. Drawing functions in `tfb_draw.rs`
are generic over `PixelCanvas` rather than taking concrete types. This
enables both `SurfaceCanvas` (borrowed SDL_Surface pixels) and
`LockedCanvas` (owned `Canvas` via `MutexGuard`) to share the same
drawing code.

### 7.2 PixelFormat (Unified)

```
enum PixelFormat
├── Rgba32    (4 bpp, RGBA order)
├── Rgbx32    (4 bpp, RGBX order, alpha ignored)
└── Indexed8  (1 bpp, palette-indexed)
```

Convertible from both `CanvasFormat` and `PixmapFormat` via `From` impls.
Resolves the type mismatch between the two existing format enums.

### 7.3 SurfaceCanvas (SDL_Surface adapter)

```
SurfaceCanvas<'a>
├── pixels: &'a mut [u8]           (borrows surface->pixels)
├── width: i32                      (cached from surface)
├── height: i32                     (cached from surface)
├── pitch: i32                      (cached from surface)
├── format: CanvasFormat            (RGBX8888 validated at creation)
└── scissor: ScissorRect            (clipping rectangle)
```

- **Ownership**: Borrows `*mut SDL_Surface`, does NOT own or free it
- **Lock protocol**: `SDL_LockSurface` before create, `SDL_UnlockSurface`
  after drop
- **Aliasing**: Exclusive pixel access during DCQ flush (main-thread-only)
- **Thread affinity**: `!Send + !Sync`
- **Lifetime**: Cannot outlive the flush scope; tied to lock/unlock bracket
- **Format**: RGBX8888 only (validated at construction, error on mismatch)
- **Self-blit**: Prohibited via raw pointer aliasing; uses temp buffer

### 7.4 LockedCanvas (Canvas adapter)

```
LockedCanvas<'a>
├── guard: MutexGuard<'a, CanvasInner>   (holds lock)
└── implements PixelCanvas               (delegates to guard fields)
```

Bridges the existing `Canvas` (which uses `Arc<Mutex<CanvasInner>>`) to
the `PixelCanvas` trait. Created via `canvas.lock_pixels()`. Lock held
for the duration of the drawing operation, not per pixel.

### 7.5 LockedSurface (RAII lock guard)

```
LockedSurface<'a>
├── surface: &'a mut SDL_Surface
├── fn new(surface: *mut SDL_Surface) -> Self  (calls SDL_LockSurface)
├── fn as_canvas(&mut self) -> SurfaceCanvas<'_>
└── Drop: calls SDL_UnlockSurface
```

RAII wrapper that prevents forgetting the unlock. `SurfaceCanvas` is
created from `LockedSurface`, not directly from raw `*mut SDL_Surface`.

### 7.6 DrawCommandQueue (DCQ)

```
DrawCommandQueue (global, static via OnceLock)
├── commands: Vec<DrawCommand>      (queued draw operations)
├── batch_depth: AtomicUsize        (batch nesting counter)
├── config: DcqConfig               (queue configuration)
└── context: Arc<RwLock<RenderContext>>  (current drawing state)
```

The Rust DCQ replaces C's `dcqueue.c` (670 lines). Game threads enqueue
draw commands; the main/graphics thread flushes them.

**15 Command Types:**
DrawLine, DrawRect, FillRect, DrawImage, Copy, SetPalette,
CopyToImage, DeleteImage, WaitForSignal, ReinitVideo,
DrawPoint, DrawStamp, DrawFilledStamp, DrawFontChar, SetContext

### 7.7 RenderContext

```
RenderContext
├── screens: [ScreenState; 3]       (per-screen drawing state)
├── fg_color: Color                 (foreground color)
├── bg_color: Color                 (background color)
├── draw_mode: DrawMode             (replace/additive/alpha)
├── clip_rect: Option<Rect>         (clipping rectangle)
├── origin: Point                   (coordinate origin offset)
└── font: Option<FontRef>           (current font)
```

State container mirroring C's `CONTEXT`. Set via `rust_context_set_*`
FFI functions before draw commands reference it.

### 7.8 ColorMapManager

```
ColorMapManager (singleton)
├── maps: Vec<ColorMap>              (registered colormaps)
├── active_xforms: Vec<XFormState>   (in-progress transforms)
├── fade_amount: i32                 (current fade level, 0–511)
└── fade_type: FadeType              (black/white/custom)
```

Replaces C's `cmap.c` (663 lines). Manages palette-based color
transformations and screen fade effects.

---

## 8. FFI Bridge Entity Model

### 8.1 New FFI Modules

| Module | Replaces | Export Count | Slice |
|---|---|---|---|
| `dcq_ffi.rs` (new) | `tfb_draw.c` enqueue functions | ~15 | C (DCQ) |
| `canvas_ffi.rs` (new) | `sdl/canvas.c` draw operations | ~10 | B (Canvas) |
| `cmap_ffi.rs` (new) | `cmap.c` colormap operations | ~8 | D (Colormap) |
| `context_ffi.rs` (new) | `context.c` state management | ~10 | B/C |
| `frame_ffi.rs` (new) | `frame.c` frame operations | ~8 | F (Widget) |
| `font_ffi.rs` (new) | `font.c` font rendering | ~4 | F (Widget) |
| `drawable_ffi.rs` (new) | `drawable.c` management | ~10 | F (Widget) |

Total: ~46 new `#[no_mangle] pub extern "C" fn` exports across 7 modules.

### 8.2 FFI Repr Types (#[repr(C)])

| Rust FFI Type | C Type | Size (bytes) | Purpose |
|---|---|---|---|
| `FfiColor` | `Color` | 4 | `{ r: u8, g: u8, b: u8, a: u8 }` |
| `FfiDrawMode` | `DrawMode` | 4 | `{ kind: u8, factor: i16 }` |
| `FfiRect` | `RECT` | 16 | `{ corner: FfiPoint, extent: FfiExtent }` |
| `FfiPoint` | `POINT` | 8 | `{ x: c_int, y: c_int }` |
| `FfiExtent` | `EXTENT` | 8 | `{ width: c_int, height: c_int }` |

All require compile-time `static_assert!` for size and alignment.

### 8.3 Handle Types (C opaque pointers → Rust newtypes)

| C Handle | C Representation | Rust Handle | Conversion |
|---|---|---|---|
| `FRAME` | `void*` (tagged pointer) | `FrameRef(u32)` | Extract index from tag bits |
| `DRAWABLE` | `void*` (pointer to DrawableDesc) | `DrawableRef(u32)` | Registry lookup by ID |
| `COLORMAP_REF` | `COLORMAPPTR` (byte array pointer) | `ColorMapRef(u32)` | Registry index |

---

## 9. Error Handling Map (Expanded — Drawing Pipeline)

### 9.1 DCQ FFI Functions

| Function | Error Condition | Response | Log? |
|---|---|---|---|
| `rust_dcq_push_*` | DCQ not initialized | Return immediately (void) | No |
| `rust_dcq_push_*` | Queue full (livelock) | Drop command, return void | Yes (once) |
| `rust_dcq_push_*` | Invalid screen index | Clamp or return void | No |
| `rust_dcq_flush` | DCQ not initialized | Return immediately (void) | No |
| `rust_dcq_flush` | Draw command fails | Skip command, continue | No |
| `rust_dcq_batch` | Already batched (nesting) | Increment depth, return | No |
| `rust_dcq_unbatch` | Not batched | No-op, return | No |

### 9.2 Canvas FFI Functions

| Function | Error Condition | Response | Log? |
|---|---|---|---|
| `rust_canvas_from_surface` | Null surface pointer | Return null | No |
| `rust_canvas_from_surface` | Unsupported pixel format | Return null | Yes (once) |
| `rust_canvas_draw_*` | Null canvas handle | Return immediately | No |
| `rust_canvas_draw_*` | Coordinates out of bounds | Clipping (not error) | No |
| `rust_canvas_destroy` | Null handle | No-op | No |

### 9.3 Colormap FFI Functions

| Function | Error Condition | Response | Log? |
|---|---|---|---|
| `rust_cmap_init` | Already initialized | No-op, return | No |
| `rust_cmap_set` | Null colormap pointer | Return immediately | No |
| `rust_cmap_set` | Invalid colormap index | Return immediately | No |
| `rust_cmap_fade_screen` | Invalid fade type | Clamp to valid range | No |
| `rust_cmap_xform_step` | No active transforms | Return 0 | No |
| `rust_cmap_get` | Index out of range | Return null | No |

### 9.4 Context/Frame/Drawable FFI Functions

| Function | Error Condition | Response | Log? |
|---|---|---|---|
| `rust_context_set` | Null context pointer | Return immediately | No |
| `rust_context_create` | Allocation failure | Return null | Yes |
| `rust_context_destroy` | Null handle | No-op | No |
| `rust_drawable_create` | Allocation failure | Return null | Yes |
| `rust_frame_draw_*` | No active context | Return immediately | No |
| `rust_frame_draw_*` | No destination frame | Return immediately | No |

### 9.5 Universal FFI Error Patterns (REQ-ERR-*, REQ-FFI-030)

All `extern "C"` functions in the expanded scope follow:
1. **catch_unwind**: Wrap body in `std::panic::catch_unwind` to prevent
   panics from propagating across FFI boundary
2. **Null-check all pointer parameters** before dereferencing
3. **Return safe defaults** for any error (0, null, or void)
4. **No per-frame logging** (30 FPS hot path constraint applies to
   drawing pipeline same as vtable)

---

## 10. C File Guard Dependency Graph

### 10.1 Guard Levels

```
Level 0 (no dependencies on other guarded files):
  ├── sdl/primitives.c    (pixel ops)
  ├── sdl/hq2x.c          (scaler)
  ├── sdl/biadv2x.c       (scaler)
  ├── sdl/bilinear2x.c    (scaler)
  ├── sdl/nearest2x.c     (scaler)
  ├── sdl/triscan2x.c     (scaler)
  ├── sdl/2xscalers.c     (scaler dispatch)
  ├── sdl/rotozoom.c       (rotation)
  ├── clipline.c           (line clipping)
  ├── boxint.c             (box intersection)
  ├── bbox.c               (bounding box)
  └── intersec.c           (intersection)

Level 1 (depends on Level 0):
  ├── sdl/canvas.c         (depends on primitives.c)
  ├── sdl/scalers.c        (depends on 2xscaler files) [already guarded]
  └── pixmap.c             (depends on canvas format)

Level 2 (depends on Level 1):
  ├── dcqueue.c            (depends on canvas.c for dispatch)
  ├── cmap.c               (standalone, shares types)
  ├── sdl/palette.c        (palette operations)
  ├── gfx_common.c         (depends on dcqueue.c)
  ├── tfb_draw.c           (depends on dcqueue.c for enqueue)
  └── tfb_prim.c           (depends on tfb_draw.c)

Level 3 (depends on Level 2):
  ├── context.c            (standalone state management)
  ├── drawable.c           (depends on frame.c, canvas.c)
  ├── frame.c              (depends on tfb_prim.c)
  └── font.c               (depends on frame.c, context.c)

Level 4 (depends on Level 3):
  └── widgets.c            (depends on context.c, frame.c, font.c)

SDL Backend (guarded as group):
  ├── sdl/sdl2_pure.c
  ├── sdl/sdl2_common.c
  ├── sdl/sdl1_common.c    (dead code)
  ├── sdl/pure.c
  ├── sdl/opengl.c
  └── sdl/sdluio.c

Deferred (never guarded):
  ├── gfxload.c
  ├── resgfx.c
  ├── filegfx.c
  ├── loaddisp.c
  └── sdl/png2sdl.c
```

### 10.2 Guard Strategy

- `#ifndef USE_RUST_GFX` wraps entire C file when fully replaced
- `#ifndef USE_RUST_GFX` wraps individual functions when partially replaced
- Header files: type definitions remain; function declarations guarded
- Guard bottom-up: Level 0 → Level 1 → Level 2 → Level 3 → Level 4
- Files with cross-dependencies must be guarded simultaneously:
  - `dcqueue.c` ↔ `tfb_draw.c` (DCQ pops what tfb_draw enqueues)
  - `canvas.c` ↔ `dcqueue.c` (DCQ dispatches to canvas functions)
  - `context.c` ↔ `frame.c` (frame reads context state)

### 10.3 Guard Phase Mapping

| Phase | Files Guarded | Cumulative Total |
|---|---|---|
| Pre-plan | sdl_common.c, scalers.c | 2 |
| P22 | Level 0 (15 files) | 17 |
| P23 | Level 1-2 + SDL backend (14 files) | 31 |
| P24 | Level 3-4 widget-dependent (5 files) | 36 |
| — | Deferred loaders (5 files) | 36 (5 unguarded) |

---

## 11. Integration Touchpoints (Expanded — Full Drawing Pipeline)

### 11.1 C → Rust (FFI calls — drawing pipeline)

| C Caller | Rust Symbol | When | Bridge Module |
|---|---|---|---|
| `TFB_DrawScreen_Line` (tfb_draw.c) | `rust_dcq_push_drawline` | Game draws line | `dcq_ffi.rs` |
| `TFB_DrawScreen_Rect` (tfb_draw.c) | `rust_dcq_push_drawrect` | Game draws rect | `dcq_ffi.rs` |
| `TFB_DrawScreen_Image` (tfb_draw.c) | `rust_dcq_push_drawimage` | Game draws sprite | `dcq_ffi.rs` |
| `TFB_FlushGraphics` (dcqueue.c) | `rust_dcq_flush` | Frame boundary | `dcq_ffi.rs` |
| `TFB_BatchGraphics` (dcqueue.c) | `rust_dcq_batch` | Batch start | `dcq_ffi.rs` |
| `TFB_UnbatchGraphics` (dcqueue.c) | `rust_dcq_unbatch` | Batch end | `dcq_ffi.rs` |
| `TFB_DrawCanvas_Line` (canvas.c) | `rust_canvas_draw_line` | Direct canvas draw | `canvas_ffi.rs` |
| `TFB_DrawCanvas_Image` (canvas.c) | `rust_canvas_draw_image` | Direct canvas blit | `canvas_ffi.rs` |
| `SetColorMap` (cmap.c) | `rust_cmap_set` | Colormap assignment | `cmap_ffi.rs` |
| `FadeScreen` (cmap.c) | `rust_cmap_fade_screen` | Screen fade start | `cmap_ffi.rs` |
| `SetContext` (context.c) | `rust_context_set` | Context switch | `context_ffi.rs` |
| `DrawStamp` (frame.c) | `rust_frame_draw_stamp` | Stamp rendering | `frame_ffi.rs` |

(Representative sample — full list is ~46 exports per §8.1)

### 11.2 Rust Internal Dependencies (Expanded)

| Module | Used By | Purpose |
|---|---|---|
| `tfb_draw.rs` | `dcq_ffi.rs` (via DCQ flush) | All drawing primitives |
| `dcqueue.rs` | `dcq_ffi.rs` | Command queue management |
| `context.rs` | `context_ffi.rs`, DCQ flush | Drawing state |
| `cmap.rs` | `cmap_ffi.rs` | Colormap/fade management |
| `drawable.rs` | `drawable_ffi.rs` | Drawable abstraction |
| `frame.rs` | `frame_ffi.rs` | Frame/animation support |
| `pixmap.rs` | ScreenLayer scaling | Intermediate RGBA buffer |
| `scaling.rs` | ScreenLayer scaling | HQ2x, xBRZ algorithms |

### 11.3 Data Flow Through Drawing Pipeline

```
C game logic
  │
  ▼
C enqueue functions (tfb_draw.c / tfb_prim.c / frame.c)
  │ [guarded with #ifndef USE_RUST_GFX]
  │
  ├──→ [USE_RUST_GFX=0]: C DCQ (dcqueue.c) → C canvas (canvas.c) → SDL_Surface
  │
  └──→ [USE_RUST_GFX=1]: rust_dcq_push_* (dcq_ffi.rs)
         │
         ▼
       Rust DCQ (dcqueue.rs)
         │ rust_dcq_flush()
         ▼
       LockedSurface → SurfaceCanvas (PixelCanvas trait)
         │
         ▼
       tfb_draw.rs drawing functions (draw_line, fill_rect, draw_image, etc.)
         │
         ▼
       SDL_Surface pixels (shared with C presentation layer)
         │
         ▼
       rust_gfx_screen() → texture upload → canvas.copy()
         │
         ▼
       rust_gfx_postprocess() → canvas.present()
```

---

## 12. Requirement Group Coverage (Analysis Scope)

All requirement groups from specification.md are analyzed. This section
maps each group to the domain model section that covers it.

### 12.1 Vtable Groups (covered by §§1–6)

| REQ Group | Domain Model Section | Status |
|---|---|---|
| REQ-PRE-* | §3 (Per-Frame State Machine), §6.4 | [OK] Covered |
| REQ-SCR-* | §3, §6.2 (ScreenLayer replace) | [OK] Covered |
| REQ-SCALE-* | §1.1 (scaled_buffers, hq2x), §5.3 (internal deps) | [OK] Covered |
| REQ-CLR-* | §3, §6.3 (ColorLayer replace) | [OK] Covered |
| REQ-POST-* | §3, §6.1 (Postprocess replace) | [OK] Covered |
| REQ-UTS-* | §3, §6.5 (UploadTransitionScreen) | [OK] Covered |
| REQ-SEQ-* | §3 (call sequence), §4 (error map) | [OK] Covered |
| REQ-ERR-* | §4 (error handling map), §9.5 (expanded) | [OK] Covered |
| REQ-INV-* | §3, §6.1 (double-render guard) | [OK] Covered |
| REQ-FFI-* | §5 (integration touchpoints), §9.5 (catch_unwind) | [OK] Covered |

### 12.2 Expanded Groups (covered by §§7–11)

| REQ Group | Domain Model Section | Status |
|---|---|---|
| REQ-DCQ-* | §7.6 (DCQ entity), §8.1 (dcq_ffi.rs), §9.1 (error map) | [OK] Covered |
| REQ-CANVAS-* | §7.1–7.5 (PixelCanvas, SurfaceCanvas, LockedCanvas), §8.1, §9.2 | [OK] Covered |
| REQ-CMAP-* | §7.8 (ColorMapManager), §8.1, §9.3 | [OK] Covered |
| REQ-GUARD-* | §10 (C file guard dependency graph) | [OK] Covered |
| REQ-WIDGET-* | §8.1 (font_ffi.rs, drawable_ffi.rs), §10.1 (Level 3-4) | [OK] Covered |
| REQ-GFXLOAD-* | §10.1 (Deferred, never guarded), §11.3 (data flow) | [OK] Covered |
| REQ-COMPAT-* | §10.2 (guard strategy — both modes build) | [OK] Covered |

### 12.3 Additional Groups (covered by existing sections)

| REQ Group | Domain Model Section | Status |
|---|---|---|
| REQ-INIT-* | §1 (entity model), §2 (state transitions) | [OK] Covered |
| REQ-UNINIT-* | §2 (state transitions) | [OK] Covered |
| REQ-SURF-* | §1.2 (SDL_Surface), §5 (touchpoints) | [OK] Covered |
| REQ-THR-* | §2 (single-threaded access), §7.3 (!Send+!Sync) | [OK] Covered |
| REQ-FMT-* | §1.2 (pixel format), §7.3 (RGBX8888) | [OK] Covered |
| REQ-WIN-* | §1.1 (width, height), §3 (logical coordinates) | [OK] Covered |
| REQ-AUX-* | §4 (error map), §5.1 (C→Rust calls) | [OK] Covered |
| REQ-NP-* | §3 (no dirty tracking), §6 (old code) | [OK] Covered |
| REQ-ASM-* | §1.2 (little-endian), §2 (single-threaded) | [OK] Covered |

---

## 13. Drop Order Constraint (REQ-UNINIT-020)

Per technical.md §2.5, the uninit drop order is:

1. `scaled_buffers[i] = None` — free scaling memory
2. `SDL_FreeSurface(surfaces[i])` for each screen surface
3. `SDL_FreeSurface(format_conv_surf)` — free format template
4. `drop(state.canvas)` — destroys renderer and window
5. `drop(state.video)` — destroys video subsystem
6. `drop(state.sdl_context)` — destroys SDL library

**Critical invariant**: SDL surfaces must be freed while the SDL library
is still initialized. Steps 2–3 must precede steps 4–6.

For the expanded scope, DCQ and drawing pipeline state must also be
cleaned up during uninit:
- Global DCQ instance: flushed and drained before surface cleanup
- SurfaceCanvas instances: must not exist at uninit time (scoped to flush)
- ColorMapManager: cleared during uninit (after DCQ drain, before surfaces)

---

## 14. Compositing Invariant (REQ-INV-010)

**Statement**: ScreenLayer and Postprocess shall NOT both upload and
render surface data.

**Verification**: When ScreenLayer is implemented (uploading surface
pixels, creating textures, calling `canvas.copy`), Postprocess must be
reduced to `canvas.present()` only. If both upload, the result clobbers
the layered composition (transitions, fades, and system box become
invisible).

This is the root cause of the original black-screen bug (inverted
architecture) and remains the most critical invariant in the expanded
port.
