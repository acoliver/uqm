# Rust GFX Backend — Requirements (EARS Format)

**Source documents**: `inventory.md`, `functional.md`, `technical.md`
**Date**: 2026-02-23
**Notation**: [EARS — Easy Approach to Requirements Syntax](https://alistairmavin.com/ears/)

---

## Notation Guide

Requirements use these EARS sentence patterns:

- **Ubiquitous**: The `<system>` shall `<action>`.
- **Event-driven**: When `<event>`, the `<system>` shall `<action>`.
- **State-driven**: While `<state>`, the `<system>` shall `<action>`.
- **Optional feature**: Where `<condition>`, the `<system>` shall `<action>`.
- **Unwanted behavior**: If `<unwanted condition>`, the `<system>` shall `<action>`.
- **Complex**: While `<state>`, when `<event>`, the `<system>` shall `<action>`.

The **system** is "the Rust GFX backend" unless otherwise noted.

---

## Document Governance

Where `inventory.md`, `functional.md`, and `technical.md` conflict, this
requirements document resolves the conflict explicitly in the
**Resolved Source Conflicts** appendix. The precedence order is:

1. Actual implementation behavior in `ffi.rs` (ground truth)
2. `technical.md` (closest to implementation)
3. `functional.md` (behavioral intent)
4. `inventory.md` (observational)

All conflict resolutions are documented in the traceability notes.

---

## 1. Initialization

### REQ-INIT-010
When `rust_gfx_init` is called, the backend shall initialize the SDL2
library, video subsystem, event pump, and create a window with the
specified `width` and `height`.

### REQ-INIT-015
The `driver` parameter shall be accepted and stored but not validated
(informational only). The `renderer` parameter (`*const c_char` in Rust, `const char *` in C) may be
NULL; where non-NULL, the backend shall accept it for informational
purposes. Neither parameter shall affect rendering behavior in the current
implementation.

### REQ-INIT-020
When `rust_gfx_init` is called, the backend shall create an SDL2 software
renderer with logical size 320×240 and set `SDL_HINT_RENDER_SCALE_QUALITY`
to `"0"` (nearest-neighbor). See REQ-NP-040 for the full flag logic.

### REQ-INIT-030
When `rust_gfx_init` is called, the backend shall create three screen
surfaces (indices 0, 1, 2) via `SDL_CreateRGBSurface` at 320×240, 32bpp,
RGBX8888 format with masks `R=0xFF000000`, `G=0x00FF0000`, `B=0x0000FF00`,
`A=0x00000000`.

### REQ-INIT-040
When `rust_gfx_init` is called, the backend shall create one format
conversion surface via `SDL_CreateRGBSurface` at 0×0, 32bpp, with masks
`R=0xFF000000`, `G=0x00FF0000`, `B=0x0000FF00`, `A=0x000000FF`.

### REQ-INIT-050
Where `flags & TFB_GFXFLAGS_FULLSCREEN` is set, `rust_gfx_init` shall
create the window in fullscreen mode.

### REQ-INIT-055
The backend shall consider software scaling active when `flags & TFB_GFXFLAGS_SCALE_SOFT_ONLY != 0`,
where `SCALE_SOFT_ONLY = SCALE_ANY & ~SCALE_BILINEAR` (i.e., any scaler flag
in bits 4–9; bilinear (bit 3) is excluded by the mask). Bilinear scaling alone does
not activate the software scaling path.

### REQ-INIT-060
Where `flags & TFB_GFXFLAGS_SCALE_SOFT_ONLY` is nonzero, `rust_gfx_init`
shall allocate three software scaling buffers sized at
`(320 × scale_factor) × (240 × scale_factor) × 4` bytes.

### REQ-INIT-070
Where software scaling is active (per REQ-INIT-055), the backend shall
determine the scale factor by checking `flags & (1 << 8)` first (xBRZ 3×,
factor=3), then `flags & (1 << 9)` (xBRZ 4×, factor=4). If neither xBRZ
flag is set, HQ2x (factor=2) is the default among software scalers. If
both bits 8 and 9 are set, 3× takes precedence.

### REQ-INIT-080
When `rust_gfx_init` succeeds, it shall return 0.

### REQ-INIT-090
If any initialization step fails, `rust_gfx_init` shall free all resources
allocated up to the failure point — including explicit `SDL_FreeSurface`
for raw-pointer surfaces not managed by Rust RAII (see REQ-INIT-097) — and
return -1.

### REQ-INIT-095
If `rust_gfx_init` is called when the backend is already initialized, it
shall return -1 without modifying existing state.

### REQ-INIT-096
After a failed `rust_gfx_init` call, a subsequent call to `rust_gfx_init`
shall attempt initialization normally (the failure shall not permanently
disable the backend).

### REQ-INIT-097
During `rust_gfx_init`, if any `SDL_CreateRGBSurface` call fails after
prior surfaces were created, the backend shall explicitly free each
previously created surface via `SDL_FreeSurface` before returning -1. Raw
pointers from C FFI are not managed by Rust RAII.

### REQ-INIT-100
If any initialization step fails, `rust_gfx_init` shall log a diagnostic
message via `rust_bridge_log_msg` before returning.

---

## 2. Teardown

### REQ-UNINIT-010
When `rust_gfx_uninit` is called, the backend shall deallocate scaling
buffers, free all four SDL surfaces via `SDL_FreeSurface`, and destroy
the renderer, video subsystem, and SDL context.

### REQ-UNINIT-020
The backend shall free resources in the following explicit order:
1. Scaling buffers (`scaled_buffers[i] = None`)
2. SDL surfaces — screen surfaces and `format_conv_surf` (via `SDL_FreeSurface`)
3. Renderer/canvas (`drop(canvas)`)
4. Video subsystem (`drop(video)`)
5. SDL context (`drop(sdl_context)`)

Scaling buffers shall be freed before surfaces. Surfaces shall be freed
before the renderer, video subsystem, and SDL context are destroyed.

### REQ-UNINIT-030
While the backend is not initialized, `rust_gfx_uninit` shall return
immediately (void return) with no side effects.

---

## 3. Surface Access

### REQ-SURF-010
When `rust_gfx_get_screen_surface(i)` is called with `i` in range [0, 3),
the backend shall return the `*mut SDL_Surface` for screen index `i`.

### REQ-SURF-020
When `rust_gfx_get_screen_surface(i)` is called with `i` out of range,
the backend shall return a null pointer.

### REQ-SURF-030
While the backend is not initialized, all surface accessor functions shall
return null pointers.

### REQ-SURF-040
The backend shall not modify surface pixel data. Surface pixel memory is
owned by SDL and written to exclusively by C game code.

### REQ-SURF-050
When `rust_gfx_get_format_conv_surf` is called while the backend is
initialized, it shall return a non-null `*mut SDL_Surface` with an alpha
channel mask of `0x000000FF`.

### REQ-SURF-060
When the backend is initialized, `rust_gfx_get_sdl_screen()` shall return
`surfaces[0]` (`*mut SDL_Surface`). When uninitialized, it shall return
null (per REQ-ERR-011).

### REQ-SURF-070
When the backend is initialized, `rust_gfx_get_transition_screen()` shall
return `surfaces[2]` (`*mut SDL_Surface`). When uninitialized, it shall
return null (per REQ-ERR-011).

---

## 4. Preprocess

### REQ-PRE-010
When `rust_gfx_preprocess` is called, the backend shall set the renderer
blend mode to `BLENDMODE_NONE`.

### REQ-PRE-020
When `rust_gfx_preprocess` is called, the backend shall set the renderer
draw color to opaque black (R=0, G=0, B=0, A=255) and clear the entire
render target.

### REQ-PRE-030
The backend shall clear the renderer on every `rust_gfx_preprocess` call
regardless of the `force_redraw` parameter value.

### REQ-PRE-040
The backend shall not use the `transition_amount` or `fade_amount`
parameters in `rust_gfx_preprocess` (they are informational only, matching
the C SDL2 backend's behavior of ignoring them).

### REQ-PRE-050
While the backend is not initialized, `rust_gfx_preprocess` shall return
immediately (void return).

---

## 5. ScreenLayer

### REQ-SCR-010
When `rust_gfx_screen` is called with a compositable screen index (0 or
2), the backend shall read pixel data from `surfaces[screen]` and render
it onto the current frame. Screen index 1 (`TFB_SCREEN_EXTRA`) is valid
but not compositable; see REQ-SCR-090.

### REQ-SCR-020
For compositable screens (0 or 2), the backend shall upload the full
surface pixel data on every `rust_gfx_screen` call (no dirty-region
tracking).

### REQ-SCR-030
Where `alpha` is 255, the backend shall render the texture with blend mode
`BLENDMODE_NONE` (fully opaque, overwriting existing content).

### REQ-SCR-040
Where `alpha` is less than 255, the backend shall render the texture with
blend mode `BLENDMODE_BLEND` and alpha modifier set to `alpha`, compositing
over existing frame content using the formula
`dst = src × (alpha/255) + dst × (1 − alpha/255)`.

### REQ-SCR-050
Where `rect` is NULL, the backend shall render the entire surface to the
full renderer area.

### REQ-SCR-060
Where `rect` is non-NULL, the backend shall render only the rectangular
portion of the surface specified by `rect`. The source rect and destination
rect in logical-coordinate space shall be identical — the rect specifies
both which source pixels to read and where on screen to draw them.

### REQ-SCR-070
For compositable screens (0 or 2), the backend shall create a temporary
streaming texture per `rust_gfx_screen` call, using pixel format `RGBX8888`.
The texture shall be dropped before the function returns.

### REQ-SCR-075
When uploading pixel data to a texture via `texture.update()`, the backend
shall use the surface's `pitch` field as the row stride parameter. The
pitch may differ from `width × bytes_per_pixel` due to SDL row-end padding.

### REQ-SCR-080
The backend shall not modify the source surface's pixel data (read-only
access).

### REQ-SCR-090
Where `screen` is 1 (`TFB_SCREEN_EXTRA`), `rust_gfx_screen` shall return
immediately without rendering (void return).

### REQ-SCR-100
If `screen` is out of range [0, `TFB_GFX_NUMSCREENS`), the backend shall
return immediately without rendering (void return).

### REQ-SCR-110
If the surface pointer for the requested screen is null, the backend shall
return immediately without rendering (void return).

### REQ-SCR-120
If the surface's `pixels` pointer is null or `pitch` is zero or negative,
the backend shall return immediately without rendering (void return).

### REQ-SCR-130
If texture creation fails, the backend shall return immediately without
rendering (void return). The frame will be missing that layer but the
application shall not crash.

### REQ-SCR-140
While the backend is not initialized, `rust_gfx_screen` shall return
immediately (void return).

### REQ-SCR-150
The backend shall pass `rect` values directly to SDL2 copy operations
without additional coordinate transformation. SDL2 handles clipping of
negative coordinates and out-of-bounds regions.

### REQ-SCR-160
If `rect` is non-NULL and `rect->w < 0` or `rect->h < 0`, the backend
shall return immediately without rendering (void return). `SDL_Rect.w` and
`SDL_Rect.h` are `c_int` (signed), but `sdl2::rect::Rect::new` expects
`u32` dimensions; negative values must not be cast to `u32` as this would
cause overflow. Zero-dimension rects are valid and may be passed through
to SDL2, which renders nothing (a valid no-op per functional spec §3.2).

### REQ-SCR-165
Before reading pixel data from a surface for texture upload, the backend
shall verify that `surface->pitch * surface->h` does not exceed the
allocated buffer size implied by the surface format. Where validation
fails, the backend shall skip the upload and return without rendering.

### REQ-SCR-170
When reading pixel data from an SDL surface for texture upload, the backend
shall construct a byte slice using
`std::slice::from_raw_parts(surface.pixels as *const u8, surface.pitch as usize * surface.h as usize)`.
The slice length is `pitch × h` bytes (not `width × height × bytes_per_pixel`),
because `pitch` includes row-end padding. Safety relies on: (a) surface
created by `rust_gfx_init`, (b) C has not freed it, (c) single-threaded
access per REQ-THR-010, (d) pitch verified positive per REQ-SCR-120,
(e) surface dimensions match expected values from init. A `// SAFETY:`
comment shall document these preconditions.

---

## 6. ScreenLayer — Software Scaling

### REQ-SCALE-025
Where only `SCALE_BILINEAR` is set (no `SCALE_SOFT_ONLY` bits active), the
backend shall not allocate software scaling buffers and shall use the
unscaled ScreenLayer path for texture upload.

### REQ-SCALE-010
Where software scaling is active (`state.scaled_buffers[screen]` exists),
`rust_gfx_screen` shall convert surface pixel data from RGBX8888 to RGBA
format before passing it to the scaler.

### REQ-SCALE-020
Where software scaling is active, the backend shall invoke the appropriate
scaler based on GFX flags: xBRZ for bits 8 or 9, HQ2x for bit 7.

### REQ-SCALE-030
Where software scaling is active, the backend shall convert the scaler
output from RGBA back to RGBX8888 before uploading to the texture.

### REQ-SCALE-040
Where software scaling is active, the texture shall be created at
`(320 × scale_factor) × (240 × scale_factor)` resolution.

### REQ-SCALE-050
Where software scaling is active and `rect` is non-NULL, the source rect
coordinates passed to `canvas.copy` shall be multiplied by the scale factor.
The destination rect shall remain in logical coordinates (320×240 space).

### REQ-SCALE-055
Multiplication of source rect coordinates by the scale factor shall not
overflow `i32`. Given the fixed source resolution (320×240) and maximum
scale factor (4), the maximum product (1280) is within `i32` range. This
is satisfied by construction; no runtime overflow check is required.

### REQ-SCALE-060
The RGBX8888-to-RGBA conversion shall transform each pixel from in-memory
byte order `[X, B, G, R]` to `[R, G, B, 0xFF]`.

### REQ-SCALE-070
The RGBA-to-RGBX8888 conversion shall transform each pixel from
`[R, G, B, A]` to in-memory byte order `[0xFF, B, G, R]`.

---

## 7. ColorLayer

> **Note:** REQ-CLR-010 through REQ-CLR-030 specify state that must be set before the fill operation. The required execution order is: set blend mode (REQ-CLR-020/030), set draw color (REQ-CLR-010), fill rectangle (REQ-CLR-040). Requirement IDs do not imply execution order.

### REQ-CLR-010
When `rust_gfx_color` is called, the backend shall set the renderer draw
color to `(r, g, b, a)`.

### REQ-CLR-020
Where `a` is 255, the backend shall set the renderer blend mode to
`BLENDMODE_NONE` (fully opaque fill).

### REQ-CLR-030
Where `a` is less than 255, the backend shall set the renderer blend mode
to `BLENDMODE_BLEND` (alpha-blended fill over existing content).

### REQ-CLR-040
Where `rect` is NULL, the backend shall fill the entire renderer area.

### REQ-CLR-050
Where `rect` is non-NULL, the backend shall fill only the rectangular
region specified by `rect`.

### REQ-CLR-055
If `rect` is non-NULL and `rect->w < 0` or `rect->h < 0`, the backend
shall return immediately without rendering (void return). Zero-dimension
rects are passed through to SDL2 (valid no-op). See REQ-SCR-160 for
rationale.

### REQ-CLR-060
While the backend is not initialized, `rust_gfx_color` shall return
immediately (void return).

### REQ-CLR-070
The backend shall accept `fade_amount` values in the full range 0–511.
`TFB_SwapBuffers` computes the ColorLayer arguments as follows:
- When `fade_amount < 255`: `color(0, 0, 0, 255 - fade_amount, NULL)`
  (fade to black).
- When `fade_amount > 255`: `color(255, 255, 255, fade_amount - 255, NULL)`
  (fade to white).
- When `fade_amount == 255`: ColorLayer is not called (no fade active).

The backend shall handle the resulting `a` parameter across its full 0–255
range without clamping or special-casing.

---

## 8. UploadTransitionScreen

### REQ-UTS-010
While the Rust backend uses unconditional full-surface upload in
ScreenLayer (no dirty-region tracking), `rust_gfx_upload_transition_screen`
shall be a no-op.

### REQ-UTS-020 *(Design Constraint — Non-Normative)*
NOTE: If the compositing architecture is changed to use dirty-region
tracking in ScreenLayer, `rust_gfx_upload_transition_screen` must be
updated to set a dirty flag for `TFB_SCREEN_TRANSITION` so the next
ScreenLayer call re-uploads the surface. This is a design constraint
documenting the coupling between ScreenLayer's upload strategy and
`UploadTransitionScreen`, not a testable requirement.

### REQ-UTS-030
While the backend is not initialized,
`rust_gfx_upload_transition_screen` shall return immediately (void return).

---

## 9. Postprocess

### REQ-POST-010
When `rust_gfx_postprocess` is called, the backend shall call
`canvas.present()` to display the composed frame.

### REQ-POST-020
The backend shall NOT upload surface pixel data, create textures, or call
`canvas.copy` within `rust_gfx_postprocess`. All surface compositing is
the responsibility of ScreenLayer.

### REQ-POST-030
While the backend is not initialized, `rust_gfx_postprocess` shall return
immediately (void return).

---

## 10. Call Sequence Contract

### REQ-SEQ-010
The backend shall support being called in the following deterministic
sequence by `TFB_SwapBuffers`:

1. `preprocess(force_redraw, transition_amount, fade_amount)`
2. `screen(TFB_SCREEN_MAIN, 255, NULL)` — always
3. `screen(TFB_SCREEN_TRANSITION, 255 − transition_amount, &clip_rect)` — when `transition_amount != 255`
4. `color(r, g, b, a, NULL)` — when `fade_amount != 255`
5. `screen(TFB_SCREEN_MAIN, 255, &system_box)` — when `system_box_active`
6. `postprocess()`

### REQ-SEQ-020
The backend shall produce a correctly composed frame when only a subset
of the conditional calls (steps 3–5) are made in a given frame.

### REQ-SEQ-030
The backend shall tolerate `TFB_SwapBuffers` not calling the vtable at all
(early-exit when nothing has changed and no fade/transition is active).

### REQ-SEQ-040
When `TFB_SwapBuffers` is called with `TFB_REDRAW_EXPOSE`, the backend
shall produce a full repaint of the current frame state.

### REQ-SEQ-050
When `TFB_SwapBuffers` is called with `TFB_REDRAW_FADING`, the backend
shall correctly render the current fade/transition animation state even
if no new draw commands were processed.

### REQ-SEQ-060
When `TFB_SwapBuffers` is called with `TFB_REDRAW_YES` (after
`REINITVIDEO`), the backend shall produce a full repaint.

### REQ-SEQ-065
The backend shall produce identical composited output regardless of the
`TFB_REDRAW` mode (`TFB_REDRAW_NO` (0), `TFB_REDRAW_FADING` (1), `TFB_REDRAW_EXPOSE` (2), `TFB_REDRAW_YES` (3)) that triggered the vtable
call sequence, given identical surface contents and state.

### REQ-SEQ-070
If vtable functions are called outside the canonical `TFB_SwapBuffers`
sequence (e.g., ScreenLayer without a preceding Preprocess), the backend
shall not crash or corrupt internal state.

---

## 11. Threading

### REQ-THR-010
The backend shall assume all FFI function calls originate from the
graphics/main thread only.

### REQ-THR-020
The backend shall not contain synchronization primitives (mutexes,
atomics, condvars). Thread safety is enforced by C-side serialization
via `dcqueue.c`.

### REQ-THR-030
The backend shall use `UnsafeCell` (or equivalent single-threaded interior
mutability) for global state access, not `Mutex` or `RwLock`.

### REQ-THR-035
The `GraphicsStateCell` wrapping `UnsafeCell<Option<RustGraphicsState>>`
shall be marked `unsafe impl Sync` to enable storage in a `static`. A
`// SAFETY:` comment documenting the single-threaded access invariant (per
REQ-THR-010) shall accompany the `unsafe impl` block. The safety proof
shall demonstrate: (a) all access is from the main thread, (b) C's draw
command queue serializes calls, (c) no Rust code spawns threads accessing
the state.

---

## 12. Error Handling

### REQ-ERR-010
While the backend is not initialized, all FFI functions **except
`rust_gfx_init`** shall return safe default values without crashing.
`rust_gfx_init` is excluded because it is the function that transitions
the backend from uninitialized to initialized.

### REQ-ERR-011
While the backend is not initialized, surface accessor functions
(`rust_gfx_get_screen_surface`, `rust_gfx_get_sdl_screen`,
`rust_gfx_get_transition_screen`, `rust_gfx_get_format_conv_surf`) shall
return null pointers.

### REQ-ERR-012
While the backend is not initialized, void vtable functions
(`rust_gfx_preprocess`, `rust_gfx_postprocess`, `rust_gfx_screen`,
`rust_gfx_color`, `rust_gfx_upload_transition_screen`) shall return
immediately with no side effects.

### REQ-ERR-013
While the backend is not initialized, auxiliary query functions shall
return safe defaults: `rust_gfx_process_events` returns 0,
`rust_gfx_is_fullscreen` returns 0, `rust_gfx_get_width` returns 0,
`rust_gfx_get_height` returns 0.

### REQ-ERR-014
While the backend is not initialized, auxiliary mutation functions shall
return error codes: `rust_gfx_toggle_fullscreen` returns -1,
`rust_gfx_set_gamma` returns -1.

### REQ-ERR-020
If `rust_gfx_init` fails partway through initialization, it shall free
all previously allocated resources before returning -1.

### REQ-ERR-030
The backend shall not log errors from vtable functions during normal
per-frame operation. Validation failures in ScreenLayer and ColorLayer
shall result in immediate return (void return) without logging.

### REQ-ERR-040
The backend shall log diagnostic messages during `rust_gfx_init` failures
via `rust_bridge_log_msg`.

### REQ-ERR-050
`rust_gfx_uninit` shall be safe to call even if `rust_gfx_init` was never
called or failed (no-op on uninitialized state).

### REQ-ERR-060
If `texture.update`, `canvas.copy`, or `canvas.fill_rect` fails during a
vtable function call, the function shall return immediately (void return)
without crashing and without emitting per-frame log messages. One-time
diagnostic logging (e.g., `log_once`) is permitted to aid debugging; only
repeated per-frame log spam is prohibited.

### REQ-ERR-065
If `texture.update()` fails, the backend shall not call `canvas.copy()` for
that texture and shall return immediately.

---

## 13. Compositing Invariants

### REQ-INV-005
The backend shall not perform primitive or game-object drawing. It shall
only composite existing surfaces and present frames. All sprite, text,
line, and rectangle drawing is performed by C game code into surface pixel
memory before the backend is invoked.

### REQ-INV-010
ScreenLayer and Postprocess shall not both upload and render surface data.
When ScreenLayer composites surfaces onto the renderer, Postprocess shall
only call `canvas.present()`.

### REQ-INV-020
`UploadTransitionScreen` shall be a no-op only while ScreenLayer
unconditionally uploads the full surface on every call. If ScreenLayer is
changed to use dirty-region tracking, `UploadTransitionScreen` shall mark
`TFB_SCREEN_TRANSITION` dirty.

### REQ-INV-030
The backend shall not modify the call sequence or skip vtable calls. The
C code (`TFB_SwapBuffers`) controls which functions are called and in what
order; the backend responds to each call independently.

### REQ-INV-040
Repeated `postprocess()` calls without an intervening `preprocess()` shall
not mutate surface data or corrupt renderer state (the same frame is
presented again).

### REQ-INV-050
Repeated `preprocess()` calls without an intervening `postprocess()` shall
each result in a black frame base (each call clears the renderer).

### REQ-INV-060
The backend state shall be either fully initialized or fully uninitialized
at all times. A failed initialization shall leave the backend in the
uninitialized state with all resources freed.

### REQ-INV-061
After a failed `rust_gfx_init` call, all surface accessor functions shall
return null pointers and all auxiliary query functions shall return 0,
consistent with the uninitialized state defined in REQ-ERR-011 through
REQ-ERR-014.

---

## 14. Pixel Format

### REQ-FMT-010
Screen surfaces shall use RGBX8888 format with masks `R=0xFF000000`,
`G=0x00FF0000`, `B=0x0000FF00`, `A=0x00000000` (no alpha channel).

### REQ-FMT-020
Temporary textures created for ScreenLayer shall use
`PixelFormatEnum::RGBX8888`, matching the surface pixel format to avoid
format conversion during upload.

### REQ-FMT-030
The format conversion surface shall use RGBA format with alpha mask
`0x000000FF`, serving as a format template for C sprite/font loading.

### REQ-FMT-040
The Rust `SDL_Surface` and `SDL_Rect` structs shall be declared with
`#[repr(C)]` and shall be layout-compatible with their SDL2 C counterparts.

---

## 15. Window and Display

### REQ-WIN-010
The backend shall set the renderer logical size to 320×240 so that SDL2
automatically scales rendering output to the window dimensions.

### REQ-WIN-020
The backend shall not apply coordinate transformations beyond what SDL2's
logical size provides. All rect parameters are in 320×240 logical space.

### REQ-WIN-030
Where software scaling is active, the source rect for texture reads shall
be scaled by the scale factor. The destination rect shall remain in logical
coordinates.

---

## 16. Auxiliary Functions

### REQ-AUX-010
`rust_gfx_process_events` shall poll the SDL event queue and return 1 if
a quit event was received, 0 otherwise.

### REQ-AUX-020
`rust_gfx_toggle_fullscreen` shall toggle between fullscreen and windowed
modes, returning 1 if now fullscreen, 0 if now windowed, -1 on error.

### REQ-AUX-030
`rust_gfx_is_fullscreen` shall return 1 if the window is fullscreen,
0 if windowed.

### REQ-AUX-040
`rust_gfx_set_gamma` shall return -1 (unsupported) unconditionally when
using the software renderer backend. The call shall have no side effects
on internal rendering state. The return code convention (0 = success,
-1 = unsupported) is reserved for future hardware-accelerated renderer
variants that may support gamma correction.

### REQ-AUX-041
The `rust_gfx_set_gamma` function shall accept a single `gamma` parameter
of type `f32` (C `float`). The parameter type must match the C declaration
for correct ABI.

### REQ-AUX-050
`rust_gfx_get_width` and `rust_gfx_get_height` shall return 320 and 240
respectively (logical screen dimensions, not window pixel dimensions).

### REQ-AUX-060
While the backend is not initialized, all auxiliary functions shall return
safe defaults without crashing.

---

## 17. Intentional Non-Parity with C

### REQ-NP-010
The backend shall re-upload the full surface on every ScreenLayer call
(no dirty-region tracking). This is an intentional simplification.

### REQ-NP-020
The backend shall use per-call temporary textures rather than persistent
per-screen textures. This is required by the sdl2 crate's lifetime
constraints.

### REQ-NP-025
The `TextureCreator` obtained from `canvas.texture_creator()` and any
`Texture` created from it shall be dropped before the FFI function returns.
The texture must not outlive the `TextureCreator`'s borrow of the canvas.

### REQ-NP-030
The backend shall use the SDL2 software renderer (`.software()` canvas).

### REQ-NP-040
The backend shall set `SDL_HINT_RENDER_SCALE_QUALITY = "0"`
(nearest-neighbor) unconditionally. This requirement adopts the current
Rust implementation behavior (always `"0"`) as documented in technical
§9.5's Rust-specific rationale. The C backend's conditional hint logic
(§9.5 first paragraph) is intentionally not replicated. Setting `"1"`
(linear filtering) when `TFB_GFXFLAGS_SCALE_BILINEAR` is the only active
scaler flag is a deferred enhancement (see RSC-001).

### REQ-NP-050
The backend is not required to render scanline effects in the initial
implementation. Where the `TFB_GFXFLAGS_SCANLINES` flag is set, the
backend shall treat it as a no-op. The flag shall have no effect on any
backend function, including Postprocess (which shall only call
`canvas.present()`). This defers functional §3.5's optional scanline
behavior to a future implementation phase.

### REQ-NP-052
In the current implementation phase, the presence or absence of the
`TFB_GFXFLAGS_SCANLINES` flag shall produce identical pixel output for
the same input surfaces and state.

### REQ-NP-060
Where scaler flags bits 4–6 (`TFB_GFXFLAGS_SCALE_BIADAPT`,
`TFB_GFXFLAGS_SCALE_BIADAPTADV`, `TFB_GFXFLAGS_SCALE_TRISCAN`) are
requested via flags, the backend shall treat the request as a no-op in the
current implementation phase. The BiAdapt, BiAdaptAdv, and TriScan
algorithms are not implemented as distinct scalers; see REQ-NP-061 for
the fallthrough behavior when these bits activate the software scaling path.

### REQ-NP-061
Where bits 4–6 are set without bits 7–9, the software scaling path
activates per REQ-INIT-055 (because these bits are included in
`SCALE_ANY` and thus satisfy `SCALE_SOFT_ONLY`), causing scaling buffers
to be allocated and the software scaling path to run. Since no scaler
matches bits 4–6 specifically, the scale factor determination
(REQ-INIT-070) falls through to the HQ2x default (factor=2).

### REQ-NP-070
The backend shall not handle FPS display (`TFB_GFXFLAGS_SHOWFPS`, bit 1).
FPS rendering is handled by the DCQ layer (`dcqueue.c`).

---

## 18. Assumptions

### REQ-ASM-010
The backend assumes little-endian byte order (macOS arm64/x86_64). The
pixel format masks are hardcoded for little-endian. Big-endian platforms
are not supported.

### REQ-ASM-020
The backend assumes single-threaded access to all state. No thread safety
mechanisms are required or implemented.

### REQ-ASM-030
The Rust backend is conditionally compiled behind `#ifdef USE_RUST_GFX`
in `sdl_common.c`. When this flag is not set, the C `sdl2_pure` driver
is used instead.

### REQ-ASM-040
The C caller provides valid (non-dangling, properly aligned) pointers for
all non-NULL rect and surface arguments passed to FFI functions.

### REQ-ASM-050
The Rust-side constant for `TFB_GFX_NUMSCREENS` shall match the C-side
definition. A compile-time or test-time assertion shall verify
synchronization.

---

## 19. FFI Safety

### REQ-FFI-010
Each `unsafe` dereference of a `*const SDL_Rect` or `*mut SDL_Rect`
received from C shall be preceded by a null check and shall include a
`// SAFETY:` comment referencing REQ-ASM-040 (valid pointer guarantee from
C caller).

### REQ-FFI-020
Each `unsafe` dereference of a `*mut SDL_Surface` stored in
`state.surfaces[screen]` shall be preceded by a null check (REQ-SCR-110)
and shall include a `// SAFETY:` comment documenting that the surface was
created by `rust_gfx_init` and has not been freed.

### REQ-FFI-030
No `extern "C" fn` shall allow a Rust panic to propagate across the FFI
boundary. All panic-capable code paths within FFI functions shall use
`std::panic::catch_unwind` or be provably panic-free.

### REQ-FFI-040
All FFI-exported functions shall use `#[no_mangle]` and `extern "C"`
calling convention for correct C linkage and ABI compatibility.

### REQ-FFI-050
Surface access shall use raw pointer dereferencing only. The backend shall
not create `&mut SDL_Surface` references from `state.surfaces[screen]`
while C code holds pointers to the same surface. Shared references
(`&SDL_Surface`) are permitted only when C is not concurrently writing to
surface header fields.

### REQ-FFI-060
The backend shall not call any FFI-exported function from within another
FFI function's execution. Each mutable reference from `get_gfx_state()`
shall not temporally overlap with another.

---

## Traceability Matrix

| Requirement | Source (Inventory) | Source (Functional) | Source (Technical) |
|---|---|---|---|
| REQ-INIT-010..090 | §2.1, §2.2 | §6.1 | §1.3, §2, §3.1, §6.1 |
| REQ-INIT-095 | §7.2 (error handling) | §6.1 (init preconditions) | §2 (init guard) |
| REQ-INIT-096 | §7.2 (error recovery) | §6.1 (init retry) | §2, §10 (post-failure init) |
| REQ-INIT-100 | §2.1, §2.2 | §6.1 | §1.3, §2 |
| REQ-UNINIT-010..030 | §2.4 | §6.2 [superseded by technical §2.5] | §2.5 |
| REQ-SURF-010..050 | §5.1, §5.2, §5.3 | §5.1, §5.2, §7.6 | §3 |
| REQ-PRE-010..050 | §3.1, §7.1 | §3.1 | §4.2 |
| REQ-SCR-010..150 | §3.2, §7.2 | §3.2 | §4.3, §5 |
| REQ-SCR-160 | §3.2 (rect handling) | §3.2 (rect validation) | §12.2 (rect conversion, `u32`) |
| REQ-SCR-165 | §3.2 (surface upload) | §3.2 (pixel data) | §4.3 (texture upload) |
| REQ-SCALE-025 | §6 (scaler flags) | §3.2 (scaling path) | §6, §9.5 (bilinear-only) |
| REQ-SCALE-010..070 | §6, §3.3 | §3.2 (step 5) | §6 |
| REQ-CLR-010..050 | §3.4, §7.3 | §3.3 | §4.4 |
| REQ-CLR-055 | §3.4 (rect handling) | §3.3 (rect validation) | §4.4, §12.2 |
| REQ-CLR-060..070 | §3.4, §7.3 | §3.3 | §4.4 |
| REQ-UTS-010..030 | §3.5, §7.4 | §3.4 | §8.2 |
| REQ-POST-010..030 | §3.6, §7.5, §10.0 | §3.5 | §4.5, §8.1 |
| REQ-SEQ-010..060 | §3, §4.2, §10.6 | §4 | §4.1, §8.3 |
| REQ-SEQ-065 | §3 (compositing) | §4 (redraw modes) | §4.1 (TFB_REDRAW) |
| REQ-SEQ-070 | §3, §10.6 | §4 (robustness) | §8.3 |
| REQ-THR-010..030 | §5.3 (surface sharing) | §5.1 (access rules) | §8.3 |
| REQ-ERR-010..060 (incl. 011–014) | §7.2, §10 | §3.1–3.5 (error handling) | §10 |
| REQ-INV-005..050 | §1, §10.0 | §1, §3.5 (critical constraint) | §1.1, §8.1, §8.3 |
| REQ-INV-060 | §7.2 (init failure) | §6.1 (init atomicity) | §2, §10 (error recovery) |
| REQ-INV-061 | §7.2 (init failure) | §6.1 (post-failure state) | §2, §10 (error recovery) |
| REQ-FMT-010..040 | §6.1, §6.2 | §5.1 (format), §5.2 | §3.2, §3.4, §12 |
| REQ-WIN-010..030 | §2.1 | §6.1, §8.4 | §8.5 |
| REQ-AUX-010..060 | §2.5, §2.6 | §7 | §7.4 |
| REQ-AUX-041 | §2.6 (set_gamma signature) | §7 (gamma API) | §7.4 (ABI compatibility) |
| REQ-SURF-060 | §5.1 (sdl_screen alias) | §5.1, §7.6 | §3 (surface access) |
| REQ-SURF-070 | §5.1 (transition_screen alias) | §5.1, §7.6 | §3 (surface access) |
| REQ-NP-010..030 | §3.1, §7.5 | §9, §10 | §9, §9.5 |
| REQ-NP-040 | §3.1 (scale hint) | §10 (deferred) | §9.5 (Rust-specific rationale) |
| REQ-NP-050 | §3.5 (scanlines) | §3.5 (optional scanlines) | §9 (not implemented) |
| REQ-NP-052 | §3.5 (scanlines) | §3.5 (scanline output) | §9 (no-op equivalence) |
| REQ-NP-060 | §6 (scaler flags) | §9 (non-parity) | §9 (scaler subset) |
| REQ-NP-061 | §6 (scaler flags) | §9 (non-parity) | §9, §6 (flag fallthrough) |
| REQ-NP-070 | §7.5 (FPS) | §10 | §9 |
| REQ-ASM-010..030 | §6.3 (endianness), §1 | §1 (assumptions) | §1.3, §3.2 |
| REQ-ASM-040 | §5.3 (FFI contract) | §5.1 (caller obligations) | §12 (FFI pointer safety) |
| REQ-INIT-015 | §2.1, §2.2 | §6.1 | §1.3, §2 (driver/renderer params) |
| REQ-THR-035 | §5.3 (global state) | §5.1 (access rules) | §8.3 (UnsafeCell + Sync) |
| REQ-SCR-170 | §3.2 (surface upload) | §3.2 (pixel data) | §4.3, §12 (raw pointer safety) |
| REQ-SCR-075 | §3.2 (surface upload) | §3.2 (pixel data) | §4.3 (texture upload, pitch) |
| REQ-SCALE-055 | §6 (scaling) | §3.2 (scaling path) | §6 (coordinate multiplication) |
| REQ-FFI-010 | §5.3 (FFI contract) | §5.1 (caller obligations) | §12, §12.2 (rect pointer safety) |
| REQ-FFI-020 | §5.3 (FFI contract) | §5.1 (caller obligations) | §12 (surface pointer safety) |
| REQ-FFI-030 | §5.3 (FFI contract) | §5.1 (panic safety) | §12 (FFI boundary safety) |
| REQ-FFI-040 | §5.3 (FFI contract) | §5.1 (C linkage) | §12 (ABI compatibility) |
| REQ-FFI-050 | §5.3 (FFI contract) | §5.1 (surface access) | §12 (aliasing rules) |
| REQ-FFI-060 | §5.3 (FFI contract) | §5.1 (re-entrancy) | §12 (mutable reference exclusivity) |
| REQ-NP-025 | §3.2 (texture lifetime) | §3.2 (texture management) | §4.3, §9.5 (sdl2 crate lifetimes) |
| REQ-ERR-065 | §7.2 (error handling) | §3.2 (texture upload errors) | §10 (error recovery) |
| REQ-INIT-055 | §6 (scaler flags) | §6.1 (scaling activation) | §6, §9.5 (SCALE_SOFT_ONLY mask) |
| REQ-INIT-060 | §6 (scaler buffers) | §6.1 (scaling buffers) | §6 (buffer allocation) |
| REQ-INIT-097 | §7.2 (error handling) | §6.1 (init cleanup) | §2 (surface FFI cleanup) |
| REQ-ASM-050 | §5.3 (constants) | §5.1 (screen count) | §3 (TFB_GFX_NUMSCREENS sync) |
| RSC-002 | §5.3 (global state) | §5.1 (access rules) | §2.1 (UnsafeCell), §8.3 (RefCell mention) |

### Traceability Notes

**REQ-UNINIT-020 [superseded] (functional §6.2)**: The functional spec
states the teardown order as "surfaces → scaled buffers → canvas/renderer →
video → SDL context", placing surfaces before scaling buffers. The technical
spec (§2.5) and actual implementation (`ffi.rs` `rust_gfx_uninit`) both
free scaling buffers first, then surfaces. REQ-UNINIT-020 follows the
technical spec and implementation. The functional spec ordering is
superseded; the `[superseded by technical §2.5]` marker in the traceability
matrix reflects this.

> **Source inconsistency (REQ-UNINIT-020)**: The functional spec (§6.2)
> states the order as "surfaces → scaled buffers → canvas/renderer →
> video → SDL context", placing surfaces before scaling buffers. The
> technical spec (§2.5) states "scaled_buffers → surfaces → canvas →
> video → SDL context". The order in REQ-UNINIT-020 matches the technical
> spec and the actual implementation in `ffi.rs` (`rust_gfx_uninit`),
> where `scaled_buffers` are set to `None` before `SDL_FreeSurface` is
> called on surfaces. The functional spec ordering is incorrect.

**REQ-NP-040 (technical §9.5 [normative], functional §10)**: The technical
spec (§9.5) specifies always `"0"` (nearest-neighbor). The functional spec
does not clearly specify conditional behavior. The current implementation
unconditionally sets `"0"`. REQ-NP-040 follows the technical spec and
implementation. See RSC-001 for history; bilinear-only `"1"` is a deferred
enhancement.

**REQ-SCR-160**: Technical §12.2 (rect conversion, `u32` width/height).
`SDL_Rect.w` and `SDL_Rect.h` are `c_int` (signed), but
`sdl2::rect::Rect::new` expects `u32` dimensions; negative values must be
rejected before casting.

**REQ-CLR-070**: Functional §3.3 (ColorLayer Call Patterns), Inventory
§3.4. The `fade_amount` 0–511 range and the black/white fade split at 255
are derived from `TFB_SwapBuffers` in the functional spec; the inventory
documents the parameter range.

**REQ-INV-010**: Inventory §10.0, Functional §3.5 (Critical Constraint).
ScreenLayer composites surfaces; Postprocess only calls
`canvas.present()`. This separation is a critical architectural constraint.

---

## Resolved Source Conflicts

### RSC-001: Scale Quality Hint Behavior

**Conflict**: The source documents disagree on `SDL_HINT_RENDER_SCALE_QUALITY`
behavior:

- **Technical spec (§9.5)**: States the Rust backend always sets `"0"`
  (nearest-neighbor). Provides a rationale: with software scalers (xBRZ,
  HQ2x), linear texture filtering is redundant because the scaler itself
  handles interpolation. Nearest-neighbor at the renderer level preserves
  the scaler's output.
- **Functional spec (§6.1, implied by §9)**: Does not clearly specify
  conditional behavior. Does not contradict the technical spec.
- **Inventory**: Does not specify conditional behavior. Consistent with
  always `"0"`.
- **Implementation (`ffi.rs` line 208)**: Unconditionally sets `"0"`.

**Decision**: REQ-INIT-020 and REQ-NP-040 are normative. The backend shall
unconditionally set `SDL_HINT_RENDER_SCALE_QUALITY` to `"0"`
(nearest-neighbor). This matches the technical spec (§9.5), the inventory,
and the current implementation. The implementation is **conforming**.

**Deferred enhancement**: Setting `SDL_HINT_RENDER_SCALE_QUALITY` to `"1"`
(linear filtering) when `TFB_GFXFLAGS_SCALE_BILINEAR` is the only active
scaler flag is a potential future enhancement. The bilinear-only case (no
software scaler active) could benefit from SDL2's built-in texture
filtering for upscaling. This is NOT a normative requirement and SHALL NOT
block implementation.

**Rationale**: When software scalers are active, nearest-neighbor is
correct because the scaler has already performed high-quality
interpolation. The unconditional `"0"` approach is simpler, matches all
non-requirements sources, and avoids introducing flag-dependent behavior
that none of the source documents consistently require.

### RSC-002: UnsafeCell vs RefCell

**Conflict**: Technical §2.1 specifies `UnsafeCell` for
`GraphicsStateCell`; §8.3 mentions `RefCell`.

**Decision**: This document mandates `UnsafeCell` (no runtime
borrow-checking overhead) as the correct choice for single-threaded FFI
code where C guarantees no re-entrancy.

---

## Appendix A: Notes (Non-Normative)

### NOTE-001: Black Screen Hypothesis

The black screen root cause is hypothesized to be the no-op ScreenLayer
and architecturally incorrect Postprocess. Implementing the compositing
pipeline per these requirements is expected to resolve the black screen,
but this must be verified at runtime. See Inventory §1 and Functional §1
for additional context on the black screen analysis.
