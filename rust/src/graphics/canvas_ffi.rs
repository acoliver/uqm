//! Canvas FFI Bridge — Stub
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
//! # Phase
//!
//! This is a **stub phase** — drawing functions return 0 (success) without
//! performing actual drawing. `rust_canvas_from_surface` and
//! `rust_canvas_destroy` are functional (allocate/free).
//!
//! @plan PLAN-20260223-GFX-FULL-PORT.P15
//! @requirement REQ-CANVAS-010, REQ-CANVAS-020, REQ-CANVAS-030, REQ-CANVAS-040,
//!              REQ-CANVAS-050, REQ-CANVAS-060, REQ-CANVAS-070, REQ-FFI-030

use std::ffi::c_int;
use std::panic::catch_unwind;
use std::ptr;

use crate::bridge_log::rust_bridge_log_msg;
use crate::graphics::ffi::{SDL_Rect, SDL_Surface};
use crate::graphics::tfb_draw::Canvas;

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
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
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
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
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
// Drawing stubs
// ============================================================================

/// Draw a line on the canvas.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-020
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_draw_line(
    canvas: *mut SurfaceCanvas,
    _x1: c_int,
    _y1: c_int,
    _x2: c_int,
    _y2: c_int,
    _color: u32,
) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Draw a rectangle outline on the canvas.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-030
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_draw_rect(
    canvas: *mut SurfaceCanvas,
    _x: c_int,
    _y: c_int,
    _w: c_int,
    _h: c_int,
    _color: u32,
) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Fill a rectangle on the canvas.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-030
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_fill_rect(
    canvas: *mut SurfaceCanvas,
    _x: c_int,
    _y: c_int,
    _w: c_int,
    _h: c_int,
    _color: u32,
) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Copy pixels from a source canvas region to a destination canvas.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-070
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_copy(
    dst: *mut SurfaceCanvas,
    _src: *const SurfaceCanvas,
    _src_rect: *const SDL_Rect,
    _dst_x: c_int,
    _dst_y: c_int,
) -> c_int {
    catch_unwind(|| if dst.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Blit image data onto the canvas.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-040
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_draw_image(
    canvas: *mut SurfaceCanvas,
    _image_data: *const u8,
    _image_w: c_int,
    _image_h: c_int,
    _x: c_int,
    _y: c_int,
) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Render a font glyph onto the canvas with alpha blending.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-050
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_draw_fontchar(
    canvas: *mut SurfaceCanvas,
    _glyph_data: *const u8,
    _glyph_w: c_int,
    _glyph_h: c_int,
    _x: c_int,
    _y: c_int,
    _color: u32,
) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Set scissor (clipping) rectangle for subsequent draw operations.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-060
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_set_scissor(
    canvas: *mut SurfaceCanvas,
    _x: c_int,
    _y: c_int,
    _w: c_int,
    _h: c_int,
) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
}

/// Clear the scissor rectangle (disable clipping).
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
/// @requirement REQ-CANVAS-060
// PANIC-FREE: catch_unwind + null check. Stub returns 0.
#[no_mangle]
pub extern "C" fn rust_canvas_clear_scissor(canvas: *mut SurfaceCanvas) -> c_int {
    catch_unwind(|| if canvas.is_null() { -1 } else { 0 }).unwrap_or(-1)
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
/// @plan PLAN-20260223-GFX-FULL-PORT.P15
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

    // ---- Lifecycle tests ----

    #[test]
    fn test_from_surface_null() {
        let handle = unsafe { rust_canvas_from_surface(ptr::null_mut()) };
        assert!(handle.is_null());
    }

    #[test]
    fn test_from_surface_zero_dimensions() {
        let mut surf = make_test_surface(0, 0);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(handle.is_null());
    }

    #[test]
    fn test_from_surface_null_pixels() {
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

    #[test]
    fn test_from_surface_and_destroy() {
        let mut surf = make_test_surface(64, 48);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_destroy_null() {
        unsafe {
            rust_canvas_destroy(ptr::null_mut());
        }
    }

    // ---- get_extent tests ----

    #[test]
    fn test_get_extent() {
        let mut surf = make_test_surface(320, 240);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());

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

    #[test]
    fn test_get_extent_null_handle() {
        let mut w: c_int = 0;
        let mut h: c_int = 0;
        let rc = unsafe { rust_canvas_get_extent(ptr::null_mut(), &mut w, &mut h) };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_get_extent_null_outputs() {
        let mut surf = make_test_surface(100, 50);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());

        let rc = unsafe { rust_canvas_get_extent(handle, ptr::null_mut(), ptr::null_mut()) };
        assert_eq!(rc, 0);

        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    // ---- Drawing stub tests (all return 0 for valid handle, -1 for null) ----

    #[test]
    fn test_draw_line_null_handle() {
        assert_eq!(rust_canvas_draw_line(ptr::null_mut(), 0, 0, 10, 10, 0), -1);
    }

    #[test]
    fn test_draw_line_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_draw_line(handle, 0, 0, 63, 63, 0xFFFFFFFF), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_draw_rect_null_handle() {
        assert_eq!(rust_canvas_draw_rect(ptr::null_mut(), 0, 0, 10, 10, 0), -1);
    }

    #[test]
    fn test_draw_rect_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_draw_rect(handle, 5, 5, 20, 20, 0xFF0000FF), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_fill_rect_null_handle() {
        assert_eq!(rust_canvas_fill_rect(ptr::null_mut(), 0, 0, 10, 10, 0), -1);
    }

    #[test]
    fn test_fill_rect_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_fill_rect(handle, 0, 0, 64, 64, 0x00FF00FF), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_copy_null_handle() {
        assert_eq!(
            rust_canvas_copy(ptr::null_mut(), ptr::null(), ptr::null(), 0, 0),
            -1
        );
    }

    #[test]
    fn test_copy_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_copy(handle, ptr::null(), ptr::null(), 0, 0), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_draw_image_null_handle() {
        assert_eq!(
            rust_canvas_draw_image(ptr::null_mut(), ptr::null(), 10, 10, 0, 0),
            -1
        );
    }

    #[test]
    fn test_draw_image_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_draw_image(handle, ptr::null(), 16, 16, 0, 0), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_draw_fontchar_null_handle() {
        assert_eq!(
            rust_canvas_draw_fontchar(ptr::null_mut(), ptr::null(), 8, 8, 0, 0, 0),
            -1
        );
    }

    #[test]
    fn test_draw_fontchar_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(
            rust_canvas_draw_fontchar(handle, ptr::null(), 8, 16, 10, 10, 0xFFFFFFFF),
            0
        );
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_set_scissor_null_handle() {
        assert_eq!(rust_canvas_set_scissor(ptr::null_mut(), 0, 0, 10, 10), -1);
    }

    #[test]
    fn test_set_scissor_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_set_scissor(handle, 10, 10, 40, 40), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    #[test]
    fn test_clear_scissor_null_handle() {
        assert_eq!(rust_canvas_clear_scissor(ptr::null_mut()), -1);
    }

    #[test]
    fn test_clear_scissor_stub() {
        let mut surf = make_test_surface(64, 64);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());
        assert_eq!(rust_canvas_clear_scissor(handle), 0);
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }

    /// Full lifecycle roundtrip exercising all stub functions.
    #[test]
    fn test_full_lifecycle() {
        let mut surf = make_test_surface(320, 240);
        let handle = unsafe { rust_canvas_from_surface(&mut surf as *mut SDL_Surface) };
        assert!(!handle.is_null());

        // Set scissor
        assert_eq!(rust_canvas_set_scissor(handle, 10, 10, 300, 220), 0);

        // Draw operations
        assert_eq!(rust_canvas_draw_line(handle, 0, 0, 319, 239, 0xFFFFFFFF), 0);
        assert_eq!(
            rust_canvas_draw_rect(handle, 10, 10, 100, 100, 0xFF0000FF),
            0
        );
        assert_eq!(
            rust_canvas_fill_rect(handle, 50, 50, 200, 150, 0x00FF00FF),
            0
        );
        assert_eq!(
            rust_canvas_draw_image(handle, ptr::null(), 32, 32, 100, 100),
            0
        );
        assert_eq!(
            rust_canvas_draw_fontchar(handle, ptr::null(), 8, 16, 200, 100, 0xFFFFFFFF),
            0
        );
        assert_eq!(rust_canvas_copy(handle, ptr::null(), ptr::null(), 0, 0), 0);

        // Get extent
        let mut w: c_int = 0;
        let mut h: c_int = 0;
        assert_eq!(unsafe { rust_canvas_get_extent(handle, &mut w, &mut h) }, 0);
        assert_eq!(w, 320);
        assert_eq!(h, 240);

        // Clear scissor
        assert_eq!(rust_canvas_clear_scissor(handle), 0);

        // Destroy
        unsafe {
            rust_canvas_destroy(handle);
            free_test_surface(&mut surf);
        }
    }
}
