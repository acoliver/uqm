//! Canvas rotation and rotated extents.
use super::surface::FALSE;
use super::transparency::{TFB_DrawCanvas_GetTransparentColor, TFB_DrawCanvas_SetTransparentColor};
use crate::graphics::native_abi::{Color, TFB_Canvas, EXTENT};
use crate::graphics::sdl::rotozoom::{rotateSurface, rotozoomSurfaceSize};
use sdl2_sys::{SDL_FillRect, SDL_MapRGBA, SDL_Surface};
use std::ptr;

/// # Safety
/// Both canvases must be live SDL surfaces.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Rotate(
    src_canvas: TFB_Canvas,
    dst_canvas: TFB_Canvas,
    angle: i32,
    size: EXTENT,
) {
    let src: *mut SDL_Surface = src_canvas.cast();
    let dst: *mut SDL_Surface = dst_canvas.cast();
    let mut color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    if i32::from(size.width) > (*dst).w || i32::from(size.height) > (*dst).h {
        log::warn!(
            "TFB_DrawCanvas_Rotate: Tried to rotate image to size {} {} when dst_canvas has only dimensions of {} {}! Failing.",
            size.width,
            size.height,
            (*dst).w,
            (*dst).h
        );
        return;
    }

    if TFB_DrawCanvas_GetTransparentColor(src_canvas, &mut color) != FALSE {
        TFB_DrawCanvas_SetTransparentColor(dst_canvas, color, FALSE);
        /* fill destination with transparent color before rotating */
        SDL_FillRect(
            dst,
            ptr::null(),
            SDL_MapRGBA((*dst).format, color.r, color.g, color.b, 0),
        );
    }

    let ret = rotateSurface(src, dst, f64::from(angle), 0);
    if ret != 0 {
        log::warn!("TFB_DrawCanvas_Rotate: WARNING: actual rotation func returned failure");
    }
}

/// # Safety
/// Canvas must be a live SDL surface and size writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetRotatedExtent(
    src_canvas: TFB_Canvas,
    angle: i32,
    size: *mut EXTENT,
) {
    let src: *mut SDL_Surface = src_canvas.cast();
    let (mut dstw, mut dsth) = (0i32, 0i32);
    rotozoomSurfaceSize(
        (*src).w,
        (*src).h,
        f64::from(angle),
        1.0,
        &mut dstw,
        &mut dsth,
    );
    (*size).height = dsth as i16;
    (*size).width = dstw as i16;
}
