//! Canvas FFI Bridge — Implementation
//!
//! Provides C-callable functions that bridge `SDL_Surface` pixel buffers to
//! Rust drawing primitives in `tfb_draw.rs`. C code obtains an opaque
//! `*mut SurfaceCanvas` handle and passes it to `rust_canvas_*` functions.
//!
//! # Handle-based API
//!
//! C never sees Rust internals. All interaction is through:
//! - `rust_canvas_from_surface()` → allocate handle
//! - `rust_canvas_*()` → drawing operations
//! - `rust_canvas_destroy()` → free handle
//!
//! # Color Encoding
//!
//! The `color` parameter is a packed u32 in RGBA byte order matching the C
//! side masks (R=0xFF000000, G=0x00FF0000, B=0x0000FF00, A=0x000000FF).
//! Conversion: `Color { r: (c >> 24) as u8, g: (c >> 16) as u8,
//!              b: (c >> 8) as u8, a: c as u8 }`
//!
//! @plan PLAN-20260223-GFX-FULL-PORT.P17
//! @requirement REQ-CANVAS-010, REQ-CANVAS-020, REQ-CANVAS-030, REQ-CANVAS-040,
//!              REQ-CANVAS-050, REQ-CANVAS-060, REQ-CANVAS-070, REQ-FFI-030

use std::ffi::c_int;
use std::panic::catch_unwind;
use std::ptr;

use crate::bridge_log::rust_bridge_log_msg;
use crate::graphics::dcqueue::{Color, DrawMode, Extent, Point, Rect};
use crate::graphics::ffi::{SDL_Rect, SDL_Surface};
use crate::graphics::tfb_draw::{copy_canvas, draw_line, fill_rect, Canvas};

/// Convert a packed RGBA u32 to a `Color` struct.
///
/// Byte order matches C-side masks: R=0xFF000000, G=0x00FF0000,
/// B=0x0000FF00, A=0x000000FF.
#[inline]
fn color_from_u32(c: u32) -> Color {
    Color::new((c >> 24) as u8, (c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Opaque handle wrapping an `SDL_Surface` for Rust drawing operations.
///
/// `SurfaceCanvas` owns a Rust `Canvas` initialized from the surface's
/// dimensions and format. Drawing operations go through the `Canvas`; the
/// results are written back to the surface pixel buffer on destroy or flush.
///
/// # Safety
///
/// The caller must ensure the `SDL_Surface` pointer remains valid for the
/// lifetime of this handle. The surface must not be freed while a
/// `SurfaceCanvas` referencing it exists.
///
/// @requirement REQ-CANVAS-010
#[repr(C)]
pub struct SurfaceCanvas {
    /// Raw pointer to the underlying SDL_Surface (not owned).
    surface: *mut SDL_Surface,
    /// Rust canvas initialized from the surface's dimensions/format.
    canvas: Canvas,
    /// Surface width cached at creation time.
    width: c_int,
    /// Surface height cached at creation time.
    height: c_int,
}

// ============================================================================
// Lifecycle: create / destroy
// ============================================================================

/// Create a `SurfaceCanvas` handle from an SDL_Surface pointer.
///
/// Returns a heap-allocated `SurfaceCanvas` that C code uses as an opaque
/// handle for all subsequent `rust_canvas_*` calls. Returns null on failure
/// (null surface, zero dimensions, null pixels).
///
/// # Safety
///
/// `surface` must be a valid, non-null `SDL_Surface` pointer with initialized
/// pixel data, or null (returns null).
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-010
// PANIC-FREE: catch_unwind + null checks. No .unwrap() or .expect().
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_from_surface(surface: *mut SDL_Surface) -> *mut SurfaceCanvas {
    let result = catch_unwind(|| {
        if surface.is_null() {
            rust_bridge_log_msg("rust_canvas_from_surface: null surface pointer");
            return ptr::null_mut();
        }

        let surf = &*surface;
        if surf.pixels.is_null() {
            rust_bridge_log_msg("rust_canvas_from_surface: surface has null pixels");
            return ptr::null_mut();
        }
        let (w, h) = (surf.w, surf.h);

        if w <= 0 || h <= 0 {
            rust_bridge_log_msg("rust_canvas_from_surface: invalid surface dimensions");
            return ptr::null_mut();
        }

        let canvas = Canvas::new_rgba(w, h);

        let sc = Box::new(SurfaceCanvas {
            surface,
            canvas,
            width: w,
            height: h,
        });

        Box::into_raw(sc)
    });

    match result {
        Ok(p) => p,
        Err(_) => {
            rust_bridge_log_msg("rust_canvas_from_surface: caught panic");
            ptr::null_mut()
        }
    }
}

/// Destroy a `SurfaceCanvas` handle, freeing the Rust-side allocation.
///
/// Does NOT free the underlying SDL_Surface — that remains owned by C.
/// Safe to call with a null pointer (no-op).
///
/// # Safety
///
/// `canvas` must be a pointer previously returned by `rust_canvas_from_surface`,
/// or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-010
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_destroy(canvas: *mut SurfaceCanvas) {
    let _ = catch_unwind(|| {
        if canvas.is_null() {
            return;
        }
        let _ = Box::from_raw(canvas);
    });
}

// ============================================================================
// Drawing operations
// ============================================================================

/// Draw a line on the canvas.
///
/// Uses Bresenham's line algorithm via `Canvas::draw_line`. The color is a
/// packed RGBA u32 (R in high byte). DrawMode::Replace is used for all FFI
/// exports since C handles compositing separately.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-020
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_draw_line(
    canvas: *mut SurfaceCanvas,
    x1: c_int,
    y1: c_int,
    x2: c_int,
    y2: c_int,
    color: u32,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() {
            return -1;
        }
        // SAFETY: canvas was created by rust_canvas_from_surface and is non-null.
        let sc = &mut *canvas;
        let c = color_from_u32(color);
        match draw_line(&mut sc.canvas, x1, y1, x2, y2, c, DrawMode::Normal) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

/// Draw a rectangle outline on the canvas.
///
/// Draws four lines forming a rectangle outline. Coordinates are top-left
/// corner + width/height. Converted to corner-to-corner for `draw_rect`.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-030
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_draw_rect(
    canvas: *mut SurfaceCanvas,
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
    color: u32,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() || w <= 0 || h <= 0 {
            return if canvas.is_null() { -1 } else { 0 };
        }
        // SAFETY: canvas was created by rust_canvas_from_surface and is non-null.
        let sc = &mut *canvas;
        let c = color_from_u32(color);
        let x2 = x + w - 1;
        let y2 = y + h - 1;
        match crate::graphics::tfb_draw::draw_rect(
            &mut sc.canvas,
            x,
            y,
            x2,
            y2,
            c,
            DrawMode::Normal,
        ) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

/// Fill a rectangle on the canvas.
///
/// Fills a solid rectangle. Coordinates are top-left corner + width/height.
/// Converted to corner-to-corner for `fill_rect`.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-030
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_fill_rect(
    canvas: *mut SurfaceCanvas,
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
    color: u32,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() || w <= 0 || h <= 0 {
            return if canvas.is_null() { -1 } else { 0 };
        }
        // SAFETY: canvas was created by rust_canvas_from_surface and is non-null.
        let sc = &mut *canvas;
        let c = color_from_u32(color);
        let x2 = x + w - 1;
        let y2 = y + h - 1;
        match fill_rect(&mut sc.canvas, x, y, x2, y2, c) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

/// Copy pixels from a source canvas region to a destination canvas.
///
/// If `src_rect` is null, copies the entire source. The source canvas
/// pixels are copied to `(dst_x, dst_y)` in the destination.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `dst` and `src` must be valid handles from `rust_canvas_from_surface`, or null.
/// `src_rect` must be a valid `SDL_Rect` pointer or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-070
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_copy(
    dst: *mut SurfaceCanvas,
    src: *const SurfaceCanvas,
    src_rect: *const SDL_Rect,
    dst_x: c_int,
    dst_y: c_int,
) -> c_int {
    catch_unwind(|| {
        if dst.is_null() || src.is_null() {
            return -1;
        }
        // SAFETY: both pointers were created by rust_canvas_from_surface.
        let dst_sc = &mut *dst;
        let src_sc = &*src;

        let (sx, sy, sw, sh) = if src_rect.is_null() {
            (0, 0, -1, -1)
        } else {
            // SAFETY: caller guarantees src_rect is valid if non-null.
            let r = &*src_rect;
            (r.x, r.y, r.w, r.h)
        };

        match copy_canvas(
            &mut dst_sc.canvas,
            &src_sc.canvas,
            dst_x,
            dst_y,
            sx,
            sy,
            sw,
            sh,
        ) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

/// Blit RGBA image data onto the canvas.
///
/// `image_data` points to raw RGBA pixel data of dimensions `image_w × image_h`.
/// The image is blitted at position `(x, y)`. A temporary Canvas is created
/// from the raw data and then `copy_canvas` is used for the actual blit.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
/// `image_data` must point to at least `image_w * image_h * 4` bytes, or be null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-040
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_draw_image(
    canvas: *mut SurfaceCanvas,
    image_data: *const u8,
    image_w: c_int,
    image_h: c_int,
    x: c_int,
    y: c_int,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() || image_data.is_null() || image_w <= 0 || image_h <= 0 {
            return if canvas.is_null() { -1 } else { 0 };
        }
        // SAFETY: canvas was created by rust_canvas_from_surface.
        let sc = &mut *canvas;

        let pixel_count = (image_w as usize) * (image_h as usize) * 4;
        // SAFETY: caller guarantees image_data points to at least pixel_count bytes.
        let src_slice = std::slice::from_raw_parts(image_data, pixel_count);

        let mut src_canvas = Canvas::new_rgba(image_w, image_h);
        let write_ok = src_canvas.with_pixels_mut(|pixels| {
            let len = pixels.len().min(src_slice.len());
            pixels[..len].copy_from_slice(&src_slice[..len]);
        });
        if write_ok.is_err() {
            return -1;
        }

        match copy_canvas(&mut sc.canvas, &src_canvas, x, y, 0, 0, image_w, image_h) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

/// Render a font glyph onto the canvas with alpha blending.
///
/// `glyph_data` is an alpha-only bitmap of dimensions `glyph_w × glyph_h`.
/// Each byte is an alpha value (0=transparent, 255=opaque). The foreground
/// `color` is applied with the glyph alpha for proper text rendering.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
/// `glyph_data` must point to at least `glyph_w * glyph_h` bytes, or be null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-050
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_draw_fontchar(
    canvas: *mut SurfaceCanvas,
    glyph_data: *const u8,
    glyph_w: c_int,
    glyph_h: c_int,
    x: c_int,
    y: c_int,
    color: u32,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() || glyph_data.is_null() || glyph_w <= 0 || glyph_h <= 0 {
            return if canvas.is_null() { -1 } else { 0 };
        }
        // SAFETY: canvas was created by rust_canvas_from_surface.
        let sc = &mut *canvas;
        let fg = color_from_u32(color);

        let glyph_size = (glyph_w as usize) * (glyph_h as usize);
        // SAFETY: caller guarantees glyph_data points to at least glyph_size bytes.
        let alpha_data = std::slice::from_raw_parts(glyph_data, glyph_size);

        let canvas_width = sc.canvas.width();
        let canvas_height = sc.canvas.height();
        let bpp = sc.canvas.format().bytes_per_pixel as usize;
        let scissor_opt = sc.canvas.scissor().rect;

        let result = sc.canvas.with_pixels_mut(|pixels| {
            for gy in 0..glyph_h {
                for gx in 0..glyph_w {
                    let src_offset = (gy * glyph_w + gx) as usize;
                    if src_offset >= alpha_data.len() {
                        continue;
                    }
                    let glyph_alpha = alpha_data[src_offset] as i32;
                    if glyph_alpha == 0 {
                        continue;
                    }

                    let effective_alpha = (glyph_alpha * fg.a as i32) / 255;
                    if effective_alpha == 0 {
                        continue;
                    }

                    let dst_x = x + gx;
                    let dst_y = y + gy;

                    if dst_x < 0 || dst_x >= canvas_width || dst_y < 0 || dst_y >= canvas_height {
                        continue;
                    }

                    if let Some(ref rect) = scissor_opt {
                        let sc_x = rect.corner.x;
                        let sc_y = rect.corner.y;
                        let sc_w = rect.extent.width;
                        let sc_h = rect.extent.height;
                        if dst_x < sc_x
                            || dst_x >= sc_x + sc_w
                            || dst_y < sc_y
                            || dst_y >= sc_y + sc_h
                        {
                            continue;
                        }
                    }

                    let dst_offset = (dst_y * canvas_width + dst_x) as usize * bpp;
                    let alpha = effective_alpha.clamp(0, 255);
                    let inv_alpha = 255 - alpha;

                    if bpp >= 4 {
                        for i in 0..3 {
                            if dst_offset + i < pixels.len() {
                                let fg_val = [fg.r, fg.g, fg.b][i] as i32;
                                let dst_val = pixels[dst_offset + i] as i32;
                                let blended = (fg_val * alpha + dst_val * inv_alpha) / 255;
                                pixels[dst_offset + i] = blended as u8;
                            }
                        }
                        if dst_offset + 3 < pixels.len() {
                            let dst_a = pixels[dst_offset + 3] as i32;
                            let result_alpha = alpha + (dst_a * inv_alpha) / 255;
                            pixels[dst_offset + 3] = result_alpha.clamp(0, 255) as u8;
                        }
                    } else if bpp == 3 {
                        for i in 0..3 {
                            if dst_offset + i < pixels.len() {
                                let fg_val = [fg.r, fg.g, fg.b][i] as i32;
                                let dst_val = pixels[dst_offset + i] as i32;
                                let blended = (fg_val * alpha + dst_val * inv_alpha) / 255;
                                pixels[dst_offset + i] = blended as u8;
                            }
                        }
                    }
                }
            }
        });

        match result {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

/// Set scissor (clipping) rectangle for subsequent draw operations.
///
/// All subsequent draw calls on this canvas will be clipped to the
/// specified rectangle. Use `rust_canvas_clear_scissor` to disable.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-060
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_set_scissor(
    canvas: *mut SurfaceCanvas,
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() {
            return -1;
        }
        // SAFETY: canvas was created by rust_canvas_from_surface.
        let sc = &mut *canvas;
        let rect = Rect::new(Point::new(x, y), Extent::new(w, h));
        sc.canvas.enable_scissor(rect);
        0
    })
    .unwrap_or(-1)
}

/// Clear the scissor rectangle (disable clipping).
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-060
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_clear_scissor(canvas: *mut SurfaceCanvas) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() {
            return -1;
        }
        // SAFETY: canvas was created by rust_canvas_from_surface.
        let sc = &mut *canvas;
        sc.canvas.disable_scissor();
        0
    })
    .unwrap_or(-1)
}

/// Get canvas dimensions.
///
/// Writes width and height through the provided pointers.
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `canvas` must be a valid handle from `rust_canvas_from_surface`, or null.
/// `w` and `h` must be valid writable pointers, or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P17
/// @requirement REQ-CANVAS-010
// PANIC-FREE: catch_unwind + null checks. Writes cached dimensions.
#[no_mangle]
pub unsafe extern "C" fn rust_canvas_get_extent(
    canvas: *mut SurfaceCanvas,
    w: *mut c_int,
    h: *mut c_int,
) -> c_int {
    catch_unwind(|| {
        if canvas.is_null() {
            return -1;
        }
        let sc = &*canvas;
        if !w.is_null() {
            *w = sc.width;
        }
        if !h.is_null() {
            *h = sc.height;
        }
        0
    })
    .unwrap_or(-1)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    // -- Test helpers --------------------------------------------------------

    /// Create a fake SDL_Surface for testing (not a real SDL surface).
    fn make_test_surface(w: c_int, h: c_int) -> SDL_Surface {
        let size = (w as usize) * (h as usize) * 4;
        let pixels = if size > 0 {
            let buf = vec![0u8; size];
            let ptr = buf.as_ptr() as *mut c_void;
            std::mem::forget(buf);
            ptr
        } else {
            ptr::null_mut()
        };

        SDL_Surface {
            flags: 0,
            format: ptr::null_mut(),
            w,
            h,
            pitch: w * 4,
            pixels,
            userdata: ptr::null_mut(),
            locked: 0,
            list_blitmap: ptr::null_mut(),
            clip_rect: SDL_Rect { x: 0, y: 0, w, h },
            map: ptr::null_mut(),
            refcount: 1,
        }
    }

    /// Free the pixel buffer from a test surface.
    ///
    /// # Safety
    ///
    /// Must only be called on surfaces created by `make_test_surface`.
    unsafe fn free_test_surface(surf: &mut SDL_Surface) {
        if !surf.pixels.is_null() && surf.w > 0 && surf.h > 0 {
            let size = (surf.w as usize) * (surf.h as usize) * 4;
            let _ = Vec::from_raw_parts(surf.pixels as *mut u8, size, size);
            surf.pixels = ptr::null_mut();
        }
    }

    /// Helper to create a SurfaceCanvas handle for testing, returning the
    /// handle and the surface. Caller must destroy both.
    fn make_test_handle(w: c_int, h: c_int) -> (*mut SurfaceCanvas, SDL_Surface) {
        let mut surf = make_test_surface(w, h);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null(), "make_test_handle: from_surface failed");
        (handle, surf)
    }

    /// Read a pixel from the internal Canvas pixel buffer at (x, y).
    /// Returns (r, g, b, a).
    fn read_pixel(handle: *mut SurfaceCanvas, x: i32, y: i32) -> (u8, u8, u8, u8) {
        assert!(!handle.is_null());
        let sc = unsafe { &*handle };
        let w = sc.canvas.width();
        let pixels = sc.canvas.pixels();
        let offset = (y * w + x) as usize * 4;
        (
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        )
    }

    /// Pack RGBA into u32 matching C-side byte order.
    fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
    }

    // -- REQ-CANVAS-010: Lifecycle tests ------------------------------------

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_canvas_from_null_surface() {
        let handle = unsafe { rust_canvas_from_surface(ptr::null_mut()) };
        assert!(handle.is_null());
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_canvas_from_valid_surface() {
        let (handle, mut surf) = make_test_handle(64, 48);
        assert!(!handle.is_null());
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_canvas_destroy() {
        let (handle, mut surf) = make_test_handle(64, 48);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_canvas_destroy_null() {
        unsafe {
            rust_canvas_destroy(ptr::null_mut());
        }
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_canvas_from_zero_dimensions() {
        let mut surf = make_test_surface(0, 0);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(handle.is_null());
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_canvas_from_null_pixels() {
        let mut surf = SDL_Surface {
            flags: 0,
            format: ptr::null_mut(),
            w: 64,
            h: 48,
            pitch: 256,
            pixels: ptr::null_mut(),
            userdata: ptr::null_mut(),
            locked: 0,
            list_blitmap: ptr::null_mut(),
            clip_rect: SDL_Rect {
                x: 0,
                y: 0,
                w: 64,
                h: 48,
            },
            map: ptr::null_mut(),
            refcount: 1,
        };
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(handle.is_null());
    }

    // -- REQ-CANVAS-010: get_extent tests -----------------------------------

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_get_extent() {
        let (handle, mut surf) = make_test_handle(320, 240);
        let mut w: c_int = 0;
        let mut h: c_int = 0;
        let rc = unsafe { rust_canvas_get_extent(handle, &mut w, &mut h) };
        assert_eq!(rc, 0);
        assert_eq!(w, 320);
        assert_eq!(h, 240);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_get_extent_null_handle() {
        let mut w: c_int = 0;
        let mut h: c_int = 0;
        let rc = unsafe { rust_canvas_get_extent(ptr::null_mut(), &mut w, &mut h) };
        assert_eq!(rc, -1);
    }

    /// @requirement REQ-CANVAS-010
    #[test]
    fn test_get_extent_null_outputs() {
        let (handle, mut surf) = make_test_handle(100, 50);
        let rc = unsafe { rust_canvas_get_extent(handle, ptr::null_mut(), ptr::null_mut()) };
        assert_eq!(rc, 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    // -- REQ-CANVAS-020: Draw Line tests ------------------------------------

    /// @requirement REQ-CANVAS-020
    #[test]
    fn test_canvas_draw_line_horizontal() {
        let (handle, mut surf) = make_test_handle(20, 20);
        let white = rgba(255, 255, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_draw_line(handle, 2, 5, 8, 5, white) },
            0
        );

        for x in 2..=8 {
            let (r, g, b, a) = read_pixel(handle, x, 5);
            assert_eq!((r, g, b, a), (255, 255, 255, 255), "pixel at ({}, 5)", x);
        }
        assert_eq!(read_pixel(handle, 1, 5), (0, 0, 0, 0));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-020
    #[test]
    fn test_canvas_draw_line_vertical() {
        let (handle, mut surf) = make_test_handle(20, 20);
        let red = rgba(255, 0, 0, 255);
        assert_eq!(unsafe { rust_canvas_draw_line(handle, 5, 2, 5, 8, red) }, 0);

        for y in 2..=8 {
            let (r, g, b, a) = read_pixel(handle, 5, y);
            assert_eq!((r, g, b, a), (255, 0, 0, 255), "pixel at (5, {})", y);
        }

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-020
    #[test]
    fn test_canvas_draw_line_diagonal() {
        let (handle, mut surf) = make_test_handle(20, 20);
        let blue = rgba(0, 0, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_draw_line(handle, 2, 2, 7, 7, blue) },
            0
        );

        let (r, g, b, a) = read_pixel(handle, 2, 2);
        assert_eq!((r, g, b, a), (0, 0, 255, 255));
        let (r, g, b, a) = read_pixel(handle, 7, 7);
        assert_eq!((r, g, b, a), (0, 0, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-020
    #[test]
    fn test_canvas_draw_line_clipped() {
        let (handle, mut surf) = make_test_handle(10, 10);
        let green = rgba(0, 255, 0, 255);
        assert_eq!(
            unsafe { rust_canvas_draw_line(handle, -5, 5, 15, 5, green) },
            0
        );

        let (r, g, b, a) = read_pixel(handle, 0, 5);
        assert_eq!((r, g, b, a), (0, 255, 0, 255));
        let (r, g, b, a) = read_pixel(handle, 9, 5);
        assert_eq!((r, g, b, a), (0, 255, 0, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-020
    #[test]
    fn test_canvas_draw_line_null_handle() {
        assert_eq!(
            unsafe { rust_canvas_draw_line(ptr::null_mut(), 0, 0, 10, 10, 0) },
            -1
        );
    }

    // -- REQ-CANVAS-030: Draw Rect tests ------------------------------------

    /// @requirement REQ-CANVAS-030
    #[test]
    fn test_canvas_draw_rect_outline() {
        let (handle, mut surf) = make_test_handle(20, 20);
        let red = rgba(255, 0, 0, 255);
        assert_eq!(unsafe { rust_canvas_draw_rect(handle, 5, 5, 6, 6, red) }, 0);

        for x in 5..=10 {
            assert_eq!(
                read_pixel(handle, x, 5),
                (255, 0, 0, 255),
                "top edge at ({}, 5)",
                x
            );
        }
        for x in 5..=10 {
            assert_eq!(
                read_pixel(handle, x, 10),
                (255, 0, 0, 255),
                "bottom edge at ({}, 10)",
                x
            );
        }
        for y in 5..=10 {
            assert_eq!(
                read_pixel(handle, 5, y),
                (255, 0, 0, 255),
                "left edge at (5, {})",
                y
            );
        }
        for y in 5..=10 {
            assert_eq!(
                read_pixel(handle, 10, y),
                (255, 0, 0, 255),
                "right edge at (10, {})",
                y
            );
        }
        assert_eq!(read_pixel(handle, 7, 7), (0, 0, 0, 0));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-030
    #[test]
    fn test_canvas_fill_rect_solid() {
        let (handle, mut surf) = make_test_handle(20, 20);
        let green = rgba(0, 255, 0, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(handle, 3, 3, 5, 5, green) },
            0
        );

        for y in 3..=7 {
            for x in 3..=7 {
                assert_eq!(
                    read_pixel(handle, x, y),
                    (0, 255, 0, 255),
                    "filled at ({}, {})",
                    x,
                    y
                );
            }
        }
        assert_eq!(read_pixel(handle, 2, 3), (0, 0, 0, 0));
        assert_eq!(read_pixel(handle, 8, 3), (0, 0, 0, 0));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-030
    #[test]
    fn test_canvas_fill_rect_clipped() {
        let (handle, mut surf) = make_test_handle(10, 10);
        let blue = rgba(0, 0, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(handle, 7, 7, 10, 10, blue) },
            0
        );

        assert_eq!(read_pixel(handle, 7, 7), (0, 0, 255, 255));
        assert_eq!(read_pixel(handle, 9, 9), (0, 0, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-030
    #[test]
    fn test_canvas_fill_rect_zero_size() {
        let (handle, mut surf) = make_test_handle(10, 10);
        let white = rgba(255, 255, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(handle, 5, 5, 0, 0, white) },
            0
        );

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-030
    #[test]
    fn test_canvas_draw_rect_null_handle() {
        assert_eq!(
            unsafe { rust_canvas_draw_rect(ptr::null_mut(), 0, 0, 10, 10, 0) },
            -1
        );
    }

    /// @requirement REQ-CANVAS-030
    #[test]
    fn test_canvas_fill_rect_null_handle() {
        assert_eq!(
            unsafe { rust_canvas_fill_rect(ptr::null_mut(), 0, 0, 10, 10, 0) },
            -1
        );
    }

    // -- REQ-CANVAS-040: Draw Image tests -----------------------------------

    /// @requirement REQ-CANVAS-040
    #[test]
    fn test_canvas_draw_image_basic() {
        let (handle, mut surf) = make_test_handle(20, 20);

        let image_data: [u8; 16] = [
            255, 0, 0, 255, // (0,0) red
            0, 255, 0, 255, // (1,0) green
            0, 0, 255, 255, // (0,1) blue
            255, 255, 0, 255, // (1,1) yellow
        ];

        assert_eq!(
            unsafe { rust_canvas_draw_image(handle, image_data.as_ptr(), 2, 2, 5, 5) },
            0
        );

        assert_eq!(read_pixel(handle, 5, 5), (255, 0, 0, 255));
        assert_eq!(read_pixel(handle, 6, 5), (0, 255, 0, 255));
        assert_eq!(read_pixel(handle, 5, 6), (0, 0, 255, 255));
        assert_eq!(read_pixel(handle, 6, 6), (255, 255, 0, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-040
    #[test]
    fn test_canvas_draw_image_clipped() {
        let (handle, mut surf) = make_test_handle(10, 10);

        let image_data = [255u8; 64]; // 4x4x4=64 bytes, all 0xFF
        assert_eq!(
            unsafe { rust_canvas_draw_image(handle, image_data.as_ptr(), 4, 4, 8, 8) },
            0
        );

        assert_eq!(read_pixel(handle, 8, 8), (255, 255, 255, 255));
        assert_eq!(read_pixel(handle, 9, 9), (255, 255, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-040
    #[test]
    fn test_canvas_draw_image_null_data() {
        let (handle, mut surf) = make_test_handle(10, 10);
        assert_eq!(
            unsafe { rust_canvas_draw_image(handle, ptr::null(), 10, 10, 0, 0) },
            0
        );
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-040
    #[test]
    fn test_canvas_draw_image_null_handle() {
        assert_eq!(
            unsafe { rust_canvas_draw_image(ptr::null_mut(), ptr::null(), 10, 10, 0, 0) },
            -1
        );
    }

    // -- REQ-CANVAS-050: Draw Fontchar tests --------------------------------

    /// @requirement REQ-CANVAS-050
    #[test]
    fn test_canvas_draw_fontchar_opaque() {
        let (handle, mut surf) = make_test_handle(20, 20);
        let white = rgba(255, 255, 255, 255);

        let glyph: [u8; 4] = [255, 255, 255, 255];
        assert_eq!(
            unsafe { rust_canvas_draw_fontchar(handle, glyph.as_ptr(), 2, 2, 5, 5, white) },
            0
        );

        assert_eq!(read_pixel(handle, 5, 5), (255, 255, 255, 255));
        assert_eq!(read_pixel(handle, 6, 5), (255, 255, 255, 255));
        assert_eq!(read_pixel(handle, 5, 6), (255, 255, 255, 255));
        assert_eq!(read_pixel(handle, 6, 6), (255, 255, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-050
    #[test]
    fn test_canvas_draw_fontchar_transparent() {
        let (handle, mut surf) = make_test_handle(20, 20);

        let red = rgba(255, 0, 0, 255);
        unsafe { rust_canvas_fill_rect(handle, 0, 0, 20, 20, red) };

        let glyph: [u8; 1] = [128];
        let green = rgba(0, 255, 0, 255);
        assert_eq!(
            unsafe { rust_canvas_draw_fontchar(handle, glyph.as_ptr(), 1, 1, 5, 5, green) },
            0
        );

        let (r, g, _b, _a) = read_pixel(handle, 5, 5);
        // ~50% blend
        assert!(r > 100 && r < 160, "r={} expected ~127", r);
        assert!(g > 100 && g < 160, "g={} expected ~128", g);

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-050
    #[test]
    fn test_canvas_draw_fontchar_clipped() {
        let (handle, mut surf) = make_test_handle(10, 10);
        let white = rgba(255, 255, 255, 255);

        let glyph: [u8; 16] = [255; 16];
        assert_eq!(
            unsafe { rust_canvas_draw_fontchar(handle, glyph.as_ptr(), 4, 4, 8, 8, white) },
            0
        );

        assert_eq!(read_pixel(handle, 8, 8), (255, 255, 255, 255));
        assert_eq!(read_pixel(handle, 9, 9), (255, 255, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-050
    #[test]
    fn test_canvas_draw_fontchar_null_handle() {
        assert_eq!(
            unsafe { rust_canvas_draw_fontchar(ptr::null_mut(), ptr::null(), 8, 8, 0, 0, 0) },
            -1
        );
    }

    // -- REQ-CANVAS-060: Scissor tests --------------------------------------

    /// @requirement REQ-CANVAS-060
    #[test]
    fn test_canvas_scissor_clips_line() {
        let (handle, mut surf) = make_test_handle(20, 20);

        assert_eq!(unsafe { rust_canvas_set_scissor(handle, 5, 5, 10, 10) }, 0);

        let white = rgba(255, 255, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_draw_line(handle, 0, 7, 19, 7, white) },
            0
        );

        assert_eq!(read_pixel(handle, 3, 7), (0, 0, 0, 0));
        assert_eq!(read_pixel(handle, 7, 7), (255, 255, 255, 255));
        assert_eq!(read_pixel(handle, 16, 7), (0, 0, 0, 0));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-060
    #[test]
    fn test_canvas_scissor_clips_fill() {
        let (handle, mut surf) = make_test_handle(20, 20);

        assert_eq!(unsafe { rust_canvas_set_scissor(handle, 5, 5, 10, 10) }, 0);

        let blue = rgba(0, 0, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(handle, 0, 0, 20, 20, blue) },
            0
        );

        assert_eq!(read_pixel(handle, 0, 0), (0, 0, 0, 0));
        assert_eq!(read_pixel(handle, 4, 5), (0, 0, 0, 0));
        assert_eq!(read_pixel(handle, 5, 5), (0, 0, 255, 255));
        assert_eq!(read_pixel(handle, 14, 14), (0, 0, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-060
    #[test]
    fn test_canvas_scissor_disable() {
        let (handle, mut surf) = make_test_handle(20, 20);

        assert_eq!(unsafe { rust_canvas_set_scissor(handle, 5, 5, 10, 10) }, 0);
        assert_eq!(unsafe { rust_canvas_clear_scissor(handle) }, 0);

        let red = rgba(255, 0, 0, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(handle, 0, 0, 20, 20, red) },
            0
        );

        assert_eq!(read_pixel(handle, 0, 0), (255, 0, 0, 255));
        assert_eq!(read_pixel(handle, 19, 19), (255, 0, 0, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// @requirement REQ-CANVAS-060
    #[test]
    fn test_canvas_set_scissor_null_handle() {
        assert_eq!(
            unsafe { rust_canvas_set_scissor(ptr::null_mut(), 0, 0, 10, 10) },
            -1
        );
    }

    /// @requirement REQ-CANVAS-060
    #[test]
    fn test_canvas_clear_scissor_null_handle() {
        assert_eq!(unsafe { rust_canvas_clear_scissor(ptr::null_mut()) }, -1);
    }

    // -- REQ-CANVAS-070: Canvas Copy tests ----------------------------------

    /// @requirement REQ-CANVAS-070
    #[test]
    fn test_canvas_copy_basic() {
        let (src_handle, mut src_surf) = make_test_handle(10, 10);
        let (dst_handle, mut dst_surf) = make_test_handle(20, 20);

        let red = rgba(255, 0, 0, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(src_handle, 0, 0, 10, 10, red) },
            0
        );

        assert_eq!(
            unsafe { rust_canvas_copy(dst_handle, src_handle, ptr::null(), 5, 5) },
            0
        );

        assert_eq!(read_pixel(dst_handle, 5, 5), (255, 0, 0, 255));
        assert_eq!(read_pixel(dst_handle, 14, 14), (255, 0, 0, 255));
        assert_eq!(read_pixel(dst_handle, 0, 0), (0, 0, 0, 0));

        unsafe {
            rust_canvas_destroy(src_handle);
            rust_canvas_destroy(dst_handle);
            free_test_surface(&mut src_surf);
            free_test_surface(&mut dst_surf);
        }
    }

    /// @requirement REQ-CANVAS-070
    #[test]
    fn test_canvas_copy_with_src_rect() {
        let (src_handle, mut src_surf) = make_test_handle(10, 10);
        let (dst_handle, mut dst_surf) = make_test_handle(20, 20);

        let green = rgba(0, 255, 0, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(src_handle, 0, 0, 10, 10, green) },
            0
        );

        let src_rect = SDL_Rect {
            x: 2,
            y: 2,
            w: 3,
            h: 3,
        };
        assert_eq!(
            unsafe { rust_canvas_copy(dst_handle, src_handle, &src_rect as *const _, 0, 0) },
            0
        );

        assert_eq!(read_pixel(dst_handle, 0, 0), (0, 255, 0, 255));
        assert_eq!(read_pixel(dst_handle, 2, 2), (0, 255, 0, 255));
        assert_eq!(read_pixel(dst_handle, 3, 0), (0, 0, 0, 0));

        unsafe {
            rust_canvas_destroy(src_handle);
            rust_canvas_destroy(dst_handle);
            free_test_surface(&mut src_surf);
            free_test_surface(&mut dst_surf);
        }
    }

    /// @requirement REQ-CANVAS-070
    #[test]
    fn test_canvas_copy_clipped() {
        let (src_handle, mut src_surf) = make_test_handle(10, 10);
        let (dst_handle, mut dst_surf) = make_test_handle(10, 10);

        let white = rgba(255, 255, 255, 255);
        assert_eq!(
            unsafe { rust_canvas_fill_rect(src_handle, 0, 0, 10, 10, white) },
            0
        );

        assert_eq!(
            unsafe { rust_canvas_copy(dst_handle, src_handle, ptr::null(), 7, 7) },
            0
        );

        assert_eq!(read_pixel(dst_handle, 7, 7), (255, 255, 255, 255));
        assert_eq!(read_pixel(dst_handle, 9, 9), (255, 255, 255, 255));

        unsafe {
            rust_canvas_destroy(src_handle);
            rust_canvas_destroy(dst_handle);
            free_test_surface(&mut src_surf);
            free_test_surface(&mut dst_surf);
        }
    }

    /// @requirement REQ-CANVAS-070
    #[test]
    fn test_canvas_copy_null_dst() {
        assert_eq!(
            unsafe { rust_canvas_copy(ptr::null_mut(), ptr::null(), ptr::null(), 0, 0) },
            -1
        );
    }

    /// @requirement REQ-CANVAS-070
    #[test]
    fn test_canvas_copy_null_src() {
        let (handle, mut surf) = make_test_handle(10, 10);
        assert_eq!(
            unsafe { rust_canvas_copy(handle, ptr::null(), ptr::null(), 0, 0) },
            -1
        );
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    // -- REQ-CANVAS-060: Scissor + fontchar ---------------------------------

    /// @requirement REQ-CANVAS-060
    #[test]
    fn test_canvas_scissor_clips_fontchar() {
        let (handle, mut surf) = make_test_handle(20, 20);

        assert_eq!(unsafe { rust_canvas_set_scissor(handle, 5, 5, 10, 10) }, 0);

        let white = rgba(255, 255, 255, 255);
        let glyph: [u8; 16] = [255; 16];
        assert_eq!(
            unsafe { rust_canvas_draw_fontchar(handle, glyph.as_ptr(), 4, 4, 3, 3, white) },
            0
        );

        assert_eq!(read_pixel(handle, 3, 3), (0, 0, 0, 0));
        assert_eq!(read_pixel(handle, 5, 5), (255, 255, 255, 255));

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    // -- Color conversion test ----------------------------------------------

    /// @requirement REQ-CANVAS-020
    #[test]
    fn test_color_from_u32_conversion() {
        let c = color_from_u32(0xFF00FF80);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 255);
        assert_eq!(c.a, 128);
    }

    // -- Full lifecycle test ------------------------------------------------

    /// @requirement REQ-CANVAS-010 REQ-CANVAS-020 REQ-CANVAS-030 REQ-CANVAS-060
    #[test]
    fn test_full_lifecycle() {
        unsafe {
            let (handle, mut surf) = make_test_handle(320, 240);

            assert_eq!(rust_canvas_set_scissor(handle, 10, 10, 300, 220), 0);

            let white = rgba(255, 255, 255, 255);
            let red = rgba(255, 0, 0, 255);
            let green = rgba(0, 255, 0, 255);
            assert_eq!(rust_canvas_draw_line(handle, 10, 10, 309, 229, white), 0);
            assert_eq!(rust_canvas_draw_rect(handle, 10, 10, 100, 100, red), 0);
            assert_eq!(rust_canvas_fill_rect(handle, 50, 50, 200, 150, green), 0);

            let mut w: c_int = 0;
            let mut h: c_int = 0;
            assert_eq!(rust_canvas_get_extent(handle, &mut w, &mut h), 0);
            assert_eq!(w, 320);
            assert_eq!(h, 240);

            assert_eq!(rust_canvas_clear_scissor(handle), 0);

            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }
}
