//! Palette, colorkey and transparent-color canvas operations.
use super::surface::{
    color_to_native, disable_color_key, disable_surface_alpha_mod, format, get_color_key,
    native_to_color, set_color_key, set_colors, FALSE, TRUE,
};
use crate::graphics::native_abi::{Color, TFB_Canvas, BOOLEAN};
use crate::memory::rust_hcalloc;
use sdl2_sys::{SDL_Color, SDL_GetRGB, SDL_MapRGBA, SDL_Surface};

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_IsPaletted(canvas: TFB_Canvas) -> BOOLEAN {
    BOOLEAN::from(!format(canvas.cast()).palette.is_null())
}

/// Returns a zero-filled 256 entry array allocated with HCalloc, or null for a
/// canvas without a palette.
///
/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_ExtractPalette(canvas: TFB_Canvas) -> *mut Color {
    let surf: *mut SDL_Surface = canvas.cast();
    let palette = format(surf).palette;
    if palette.is_null() {
        return std::ptr::null_mut();
    }
    // There may be less colors in the surface than 256. Init to 0 first.
    let result: *mut Color = rust_hcalloc(std::mem::size_of::<Color>() * 256).cast();
    assert!((*palette).ncolors <= 256);
    for i in 0..(*palette).ncolors as usize {
        *result.add(i) = native_to_color(*(*palette).colors.add(i));
    }
    result
}

/// # Safety
/// Target must be a live SDL surface and palette must hold 256 colors.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_SetPalette(target: TFB_Canvas, palette: *mut Color) {
    let mut colors = [SDL_Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    }; 256];
    for (i, native) in colors.iter_mut().enumerate() {
        *native = color_to_native(*palette.add(i));
    }
    set_colors(target.cast(), colors.as_ptr(), 0, 256);
}

/// Deliberately keeps the engine's test polarity: `tfb_get_color_key` returns 0
/// when a colorkey exists, and the original only reports the key on failure.
///
/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetTransparentIndex(canvas: TFB_Canvas) -> i32 {
    let mut colorkey = 0u32;
    if get_color_key(canvas.cast(), &mut colorkey) != 0 {
        return colorkey as i32;
    }
    -1
}

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_SetTransparentIndex(
    canvas: TFB_Canvas,
    index: i32,
    rleaccel: BOOLEAN,
) {
    let surf: *mut SDL_Surface = canvas.cast();
    if index >= 0 {
        set_color_key(surf, index as u32, rleaccel as i32);
        if TFB_DrawCanvas_IsPaletted(canvas) == FALSE {
            // disables surface alpha so color key transparency actually works
            disable_surface_alpha_mod(surf);
        }
    } else {
        disable_color_key(surf);
    }
}

/// # Safety
/// Both canvases must be live SDL surfaces.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_CopyTransparencyInfo(
    src_canvas: TFB_Canvas,
    dst_canvas: TFB_Canvas,
) {
    if !format(src_canvas.cast()).palette.is_null() {
        let index = TFB_DrawCanvas_GetTransparentIndex(src_canvas);
        TFB_DrawCanvas_SetTransparentIndex(dst_canvas, index, FALSE);
    } else {
        let mut color = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        if TFB_DrawCanvas_GetTransparentColor(src_canvas, &mut color) != FALSE {
            TFB_DrawCanvas_SetTransparentColor(dst_canvas, color, FALSE);
        }
    }
}

/// # Safety
/// Canvas must be a live SDL surface and color must be writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetTransparentColor(
    canvas: TFB_Canvas,
    color: *mut Color,
) -> BOOLEAN {
    let surf: *mut SDL_Surface = canvas.cast();
    let mut colorkey = 0u32;
    if TFB_DrawCanvas_IsPaletted(canvas) == FALSE && get_color_key(surf, &mut colorkey) == 0 {
        let (mut ur, mut ug, mut ub) = (0u8, 0u8, 0u8);
        SDL_GetRGB(colorkey, (*surf).format, &mut ur, &mut ug, &mut ub);
        (*color).r = ur;
        (*color).g = ug;
        (*color).b = ub;
        (*color).a = 0xff;
        return TRUE;
    }
    FALSE
}

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_SetTransparentColor(
    canvas: TFB_Canvas,
    color: Color,
    rleaccel: BOOLEAN,
) {
    let surf: *mut SDL_Surface = canvas.cast();
    let sdl_color = SDL_MapRGBA((*surf).format, color.r, color.g, color.b, 0);
    set_color_key(surf, sdl_color, rleaccel as i32);
    if TFB_DrawCanvas_IsPaletted(canvas) == FALSE {
        // disables surface alpha so color key transparency actually works
        disable_surface_alpha_mod(surf);
    }
}
