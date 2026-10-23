//! Canvas creation, deletion, format queries and raw pixel access.
use super::rotation::TFB_DrawCanvas_GetRotatedExtent;
use super::surface::{disable_color_key, fatal, format, set_color_key, FALSE};
use super::transparency::{TFB_DrawCanvas_CopyTransparencyInfo, TFB_DrawCanvas_SetPalette};
use super::TFB_SCALE_BILINEAR;
use super::{TFB_SCALE_NEAREST, TFB_SCALE_TRILINEAR};
use crate::graphics::native_abi::{Color, TFB_Canvas, TFB_PixelFormat, BOOLEAN, EXTENT};
use crate::graphics::sdl::sdluio::sdluio_load_image;
use sdl2_sys::{
    SDL_CreateRGBSurface, SDL_FreeSurface, SDL_GetError, SDL_LockSurface, SDL_PixelFormat,
    SDL_SetError, SDL_Surface, SDL_UnlockSurface,
};
use std::ffi::{c_char, c_int, c_void};

const SDL_SWSURFACE: u32 = 0;

extern "C" {
    static SDL_Screen: *mut SDL_Surface;
    static format_conv_surf: *mut SDL_Surface;
    fn TFB_DisplayFormatAlpha(surface: *mut SDL_Surface) -> *mut SDL_Surface;
}

/// `btable` is a compile-time table in `scaling`, so there is nothing to
/// initialize at run time. The export remains for the C caller.
#[no_mangle]
pub extern "C" fn TFB_DrawCanvas_Initialize() {}

unsafe fn create_like(fmt: &SDL_PixelFormat, w: i32, h: i32, amask: u32) -> *mut SDL_Surface {
    SDL_CreateRGBSurface(
        SDL_SWSURFACE,
        w,
        h,
        i32::from(fmt.BitsPerPixel),
        fmt.Rmask,
        fmt.Gmask,
        fmt.Bmask,
        amask,
    )
}

pub(super) unsafe fn new_true_color_like(
    fmt: &SDL_PixelFormat,
    w: i32,
    h: i32,
    has_alpha: BOOLEAN,
) -> *mut SDL_Surface {
    let new_surf = create_like(fmt, w, h, if has_alpha != FALSE { fmt.Amask } else { 0 });
    if new_surf.is_null() {
        fatal(&format!(
            "INTERNAL PANIC: Failed to create TFB_Canvas: {}",
            sdl_error()
        ));
    }
    new_surf
}

unsafe fn sdl_error() -> String {
    std::ffi::CStr::from_ptr(SDL_GetError())
        .to_string_lossy()
        .into_owned()
}

/// # Safety
/// `format_conv_surf` must be initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_New_TrueColor(
    w: c_int,
    h: c_int,
    hasalpha: BOOLEAN,
) -> TFB_Canvas {
    new_true_color_like(&*(*format_conv_surf).format, w, h, hasalpha).cast()
}

/// # Safety
/// `SDL_Screen` and `format_conv_surf` must be initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_New_ForScreen(
    w: c_int,
    h: c_int,
    withalpha: BOOLEAN,
) -> TFB_Canvas {
    let mut fmt: *const SDL_PixelFormat = (*SDL_Screen).format;
    let new_surf: *mut SDL_Surface = if !(*fmt).palette.is_null() {
        log::warn!("TFB_DrawCanvas_New_ForScreen() WARNING:Paletted display format will be slow");
        TFB_DrawCanvas_New_TrueColor(w, h, withalpha).cast()
    } else {
        if withalpha != FALSE && (*fmt).Amask == 0 {
            fmt = (*format_conv_surf).format; // Screen has no alpha and we need it
        }
        create_like(
            &*fmt,
            w,
            h,
            if withalpha != FALSE { (*fmt).Amask } else { 0 },
        )
    };
    if new_surf.is_null() {
        fatal(&format!(
            "TFB_DrawCanvas_New_ForScreen() INTERNAL PANIC:Failed to create TFB_Canvas: {}",
            sdl_error()
        ));
    }
    new_surf.cast()
}

/// # Safety
/// Palette must be null or hold 256 colors.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_New_Paletted(
    w: c_int,
    h: c_int,
    palette: *mut Color,
    transparent_index: c_int,
) -> TFB_Canvas {
    let new_surf = SDL_CreateRGBSurface(SDL_SWSURFACE, w, h, 8, 0, 0, 0, 0);
    if new_surf.is_null() {
        fatal(&format!(
            "INTERNAL PANIC: Failed to create TFB_Canvas: {}",
            sdl_error()
        ));
    }
    if !palette.is_null() {
        TFB_DrawCanvas_SetPalette(new_surf.cast(), palette);
    }
    if transparent_index >= 0 {
        set_color_key(new_surf, transparent_index as u32, 0);
    } else {
        disable_color_key(new_surf);
    }
    new_surf.cast()
}

/// # Safety
/// Canvas must be live; the old canvas must be null or live and is consumed.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_New_ScaleTarget(
    canvas: TFB_Canvas,
    oldcanvas: TFB_Canvas,
    mut type_: c_int,
    mut last_type: c_int,
) -> TFB_Canvas {
    let src: *mut SDL_Surface = canvas.cast();
    let mut old = oldcanvas;

    // For the purposes of this function, bilinear == trilinear
    if type_ == TFB_SCALE_TRILINEAR {
        type_ = TFB_SCALE_BILINEAR;
    }
    if last_type == TFB_SCALE_TRILINEAR {
        last_type = TFB_SCALE_BILINEAR;
    }

    if !old.is_null() && type_ != last_type {
        TFB_DrawCanvas_Delete(old);
        old = std::ptr::null_mut();
    }
    if !old.is_null() {
        return old; /* can just reuse the old one */
    }

    let newsurf: *mut SDL_Surface;
    if type_ == TFB_SCALE_NEAREST {
        let fmt = format(src);
        newsurf = SDL_CreateRGBSurface(
            SDL_SWSURFACE,
            (*src).w,
            (*src).h,
            i32::from(fmt.BitsPerPixel),
            fmt.Rmask,
            fmt.Gmask,
            fmt.Bmask,
            fmt.Amask,
        );
        TFB_DrawCanvas_CopyTransparencyInfo(canvas, newsurf.cast());
    } else {
        // The scaled image may in fact be larger by 1 pixel than the source
        // because of hotspot alignment and fractional edge pixels
        newsurf = if (*(*SDL_Screen).format).BitsPerPixel == 32 {
            TFB_DrawCanvas_New_ForScreen((*src).w + 1, (*src).h + 1, 1).cast()
        } else {
            TFB_DrawCanvas_New_TrueColor((*src).w + 1, (*src).h + 1, 1).cast()
        };
    }
    newsurf.cast()
}

/// # Safety
/// Source canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_New_RotationTarget(
    src_canvas: TFB_Canvas,
    angle: c_int,
) -> TFB_Canvas {
    let src: *mut SDL_Surface = src_canvas.cast();
    let mut size = EXTENT {
        width: 0,
        height: 0,
    };
    TFB_DrawCanvas_GetRotatedExtent(src_canvas, angle, &mut size);

    let fmt = format(src);
    let newsurf = SDL_CreateRGBSurface(
        SDL_SWSURFACE,
        i32::from(size.width),
        i32::from(size.height),
        i32::from(fmt.BitsPerPixel),
        fmt.Rmask,
        fmt.Gmask,
        fmt.Bmask,
        fmt.Amask,
    );
    if newsurf.is_null() {
        fatal(&format!(
            "TFB_DrawCanvas_New_RotationTarget() INTERNAL PANIC: Failed to create TFB_Canvas: {}",
            sdl_error()
        ));
    }
    TFB_DrawCanvas_CopyTransparencyInfo(src_canvas, newsurf.cast());
    newsurf.cast()
}

/// # Safety
/// Directory and file name must be valid UIO inputs.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_LoadFromFile(
    dir: *mut c_void,
    file_name: *const c_char,
) -> TFB_Canvas {
    let mut surf = sdluio_load_image(dir, file_name);
    if surf.is_null() {
        return std::ptr::null_mut();
    }
    if (*(*surf).format).BitsPerPixel < 8 {
        SDL_SetError(c"unsupported image format (min 8bpp)".as_ptr());
        SDL_FreeSurface(surf);
        surf = std::ptr::null_mut();
    }
    surf.cast()
}

/// # Safety
/// Canvas must be null or a live SDL surface that is not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Delete(canvas: TFB_Canvas) {
    if canvas.is_null() {
        log::warn!("INTERNAL PANIC: Attempted to delete a NULL canvas!");
        /* Should we actually die here? */
    } else {
        SDL_FreeSurface(canvas.cast());
    }
}

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Lock(canvas: TFB_Canvas) {
    SDL_LockSurface(canvas.cast());
}

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Unlock(canvas: TFB_Canvas) {
    SDL_UnlockSurface(canvas.cast());
}

pub(super) unsafe fn fill_pixel_format(sdl: &SDL_PixelFormat, fmt: *mut TFB_PixelFormat) {
    if !sdl.palette.is_null() {
        log::warn!("TFB_DrawCanvas_GetScreenFormat() WARNING:Paletted display format will be slow");
        (*fmt).BitsPerPixel = 32;
        (*fmt).Rmask = 0x0000_00ff;
        (*fmt).Gmask = 0x0000_ff00;
        (*fmt).Bmask = 0x00ff_0000;
        (*fmt).Amask = 0xff00_0000;
    } else {
        (*fmt).BitsPerPixel = i32::from(sdl.BitsPerPixel);
        (*fmt).Rmask = sdl.Rmask;
        (*fmt).Gmask = sdl.Gmask;
        (*fmt).Bmask = sdl.Bmask;
        (*fmt).Amask = sdl.Amask;
    }
}

/// # Safety
/// `SDL_Screen` must be initialized and `fmt` writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetScreenFormat(fmt: *mut TFB_PixelFormat) {
    fill_pixel_format(&*(*SDL_Screen).format, fmt);
}

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetStride(canvas: TFB_Canvas) -> c_int {
    (*canvas.cast::<SDL_Surface>()).pitch
}

/// # Safety
/// Canvas must be a live SDL surface and `line` must be inside it.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetLine(canvas: TFB_Canvas, line: c_int) -> *mut c_void {
    let surf: *mut SDL_Surface = canvas.cast();
    (*surf)
        .pixels
        .cast::<u8>()
        .wrapping_offset((*surf).pitch as isize * line as isize)
        .cast()
}

/// # Safety
/// Canvas must be a live SDL surface; ownership passes to the result.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_ToScreenFormat(canvas: TFB_Canvas) -> TFB_Canvas {
    let result = TFB_DisplayFormatAlpha(canvas.cast());
    if result.is_null() {
        log::debug!("WARNING: Could not convert sprite-canvas to display format.");
        canvas
    } else if result.cast::<c_void>() == canvas {
        // no conversion was necessary
        canvas
    } else {
        // converted
        TFB_DrawCanvas_Delete(canvas);
        result.cast()
    }
}
