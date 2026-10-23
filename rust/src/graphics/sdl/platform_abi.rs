//! SDL2 platform helpers, ported from sdl2_common.c (GPL-2.0-or-later).
use crate::graphics::ffi::SDL_Surface;
use sdl2_sys as sys;
use std::{ffi::c_int, ptr};
extern "C" {
    static mut GfxFlags: c_int;
    static GraphicsDriver: c_int;
    static ScreenWidthActual: c_int;
    static ScreenHeightActual: c_int;
}
extern "C" fn pre_quit() {
    unsafe {
        sys::SDL_Quit();
    }
}
/// # Safety
/// Called on the SDL main thread before graphics initialization.
#[no_mangle]
pub unsafe extern "C" fn TFB_PreInit() {
    if crate::mainloop::init_wrappers::rust_tfb_preinit() != 0 {
        std::process::exit(1);
    }
    libc::atexit(pre_quit);
}
/// # Safety
/// Called on the SDL main thread with graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_ReInitGraphics(
    driver: c_int,
    flags: c_int,
    width: c_int,
    height: c_int,
) -> c_int {
    let toggle = c_int::from(
        GfxFlags == (flags ^ 1)
            && driver == GraphicsDriver
            && width == ScreenWidthActual
            && height == ScreenHeightActual,
    );
    GfxFlags = flags;
    let result = super::pure_abi::TFB_Pure_ConfigureVideo(1, flags, width, height, toggle);
    sys::SDL_ShowCursor(if flags & 1 != 0 { 0 } else { 1 });
    result
}
#[no_mangle]
pub extern "C" fn TFB_SetGamma(_gamma: f32) -> bool {
    false
}
/// # Safety
/// Surface must be null or a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_HasSurfaceAlphaMod(surface: *mut SDL_Surface) -> c_int {
    let mut mode = sys::SDL_BlendMode::SDL_BLENDMODE_NONE;
    if surface.is_null() || sys::SDL_GetSurfaceBlendMode(surface.cast(), &mut mode) != 0 {
        return 0;
    }
    c_int::from(mode == sys::SDL_BlendMode::SDL_BLENDMODE_BLEND)
}
/// # Safety
/// Surface must be null or live and nonnull alpha writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_GetSurfaceAlphaMod(
    surface: *mut SDL_Surface,
    alpha: *mut u8,
) -> c_int {
    if surface.is_null() || alpha.is_null() {
        return -1;
    }
    let mut mode = sys::SDL_BlendMode::SDL_BLENDMODE_NONE;
    if sys::SDL_GetSurfaceBlendMode(surface.cast(), &mut mode) == 0
        && mode == sys::SDL_BlendMode::SDL_BLENDMODE_BLEND
    {
        return sys::SDL_GetSurfaceAlphaMod(surface.cast(), alpha);
    }
    *alpha = 255;
    0
}
/// # Safety
/// Surface must be null or a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_SetSurfaceAlphaMod(surface: *mut SDL_Surface, alpha: u8) -> c_int {
    if surface.is_null() {
        return -1;
    }
    let result =
        sys::SDL_SetSurfaceBlendMode(surface.cast(), sys::SDL_BlendMode::SDL_BLENDMODE_BLEND);
    if result == 0 {
        sys::SDL_SetSurfaceAlphaMod(surface.cast(), alpha)
    } else {
        result
    }
}
/// # Safety
/// Surface must be null or a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DisableSurfaceAlphaMod(surface: *mut SDL_Surface) -> c_int {
    if surface.is_null() {
        return -1;
    }
    sys::SDL_SetSurfaceAlphaMod(surface.cast(), 255);
    sys::SDL_SetSurfaceBlendMode(surface.cast(), sys::SDL_BlendMode::SDL_BLENDMODE_NONE)
}
/// # Safety
/// Surface must be null or live; nonnull key must be writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_GetColorKey(surface: *mut SDL_Surface, key: *mut u32) -> c_int {
    if surface.is_null() || key.is_null() {
        -1
    } else {
        sys::SDL_GetColorKey(surface.cast(), key)
    }
}
/// # Safety
/// Surface must be null or live.
#[no_mangle]
pub unsafe extern "C" fn TFB_SetColorKey(surface: *mut SDL_Surface, key: u32, rle: c_int) -> c_int {
    if surface.is_null() {
        return -1;
    }
    sys::SDL_SetSurfaceRLE(surface.cast(), rle);
    sys::SDL_SetColorKey(surface.cast(), 1, key)
}
/// # Safety
/// Surface must be null or live.
#[no_mangle]
pub unsafe extern "C" fn TFB_DisableColorKey(surface: *mut SDL_Surface) -> c_int {
    if surface.is_null() {
        -1
    } else {
        sys::SDL_SetColorKey(surface.cast(), 0, 0)
    }
}
/// # Safety
/// Surface must be null or live.
#[no_mangle]
pub unsafe extern "C" fn TFB_HasColorKey(surface: *mut SDL_Surface) -> c_int {
    let mut key = 0;
    c_int::from(TFB_GetColorKey(surface, &mut key) == 0)
}
/// # Safety
/// Nonnull surface must be live and colors must hold count entries.
#[no_mangle]
pub unsafe extern "C" fn TFB_SetColors(
    surface: *mut SDL_Surface,
    colors: *mut sys::SDL_Color,
    first: c_int,
    count: c_int,
) -> c_int {
    let surface = surface.cast::<sys::SDL_Surface>();
    if surface.is_null()
        || colors.is_null()
        || (*surface).format.is_null()
        || (*(*surface).format).palette.is_null()
    {
        return 0;
    }
    c_int::from(sys::SDL_SetPaletteColors((*(*surface).format).palette, colors, first, count) == 0)
}
#[no_mangle]
pub extern "C" fn TFB_SupportsHardwareScaling() -> c_int {
    1
}
/// # Safety
/// Surface and current screen-format surfaces must be live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_DisplayFormatAlpha(surface: *mut SDL_Surface) -> *mut SDL_Surface {
    let source = surface.cast::<sys::SDL_Surface>();
    let src = &*(*source).format;
    let target = if src.Amask != 0 {
        super::lifecycle_abi::format_conv_surf
    } else {
        super::lifecycle_abi::SDL_Screen
    }
    .cast::<sys::SDL_Surface>();
    let dst = &*(*target).format;
    if src.BytesPerPixel == dst.BytesPerPixel
        && src.Rmask == dst.Rmask
        && src.Gmask == dst.Gmask
        && src.Bmask == dst.Bmask
        && src.Amask == dst.Amask
    {
        return surface;
    }
    let converted =
        sys::SDL_ConvertSurface(source, ptr::from_ref(dst), (*source).flags).cast::<SDL_Surface>();
    if TFB_HasColorKey(surface) != 0
        && !converted.is_null()
        && TFB_HasColorKey(converted) != 0
        && TFB_HasSurfaceAlphaMod(converted) != 0
    {
        TFB_DisableSurfaceAlphaMod(converted);
    }
    converted
}
