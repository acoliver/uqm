//! SDL surface helpers shared by the canvas modules.
//!
//! The colorkey, alpha-mod and palette helpers do what the engine's SDL2
//! compatibility layer (`TFB_GetColorKey` and friends) does, called directly
//! on `sdl2_sys` so canvas code has no dependency on the C compatibility layer.
use crate::graphics::native_abi::{Color, TFB_ColorMap, BOOLEAN};
use sdl2_sys::{
    SDL_BlendMode, SDL_Color, SDL_GetColorKey, SDL_GetSurfaceBlendMode, SDL_PixelFormat, SDL_Rect,
    SDL_SetColorKey, SDL_SetPaletteColors, SDL_SetSurfaceAlphaMod, SDL_SetSurfaceBlendMode,
    SDL_SetSurfaceRLE, SDL_Surface,
};

pub const TRUE: BOOLEAN = 1;
pub const FALSE: BOOLEAN = 0;

pub(super) unsafe fn format<'a>(surface: *mut SDL_Surface) -> &'a SDL_PixelFormat {
    &*(*surface).format
}

pub(super) unsafe fn get_color_key(surface: *mut SDL_Surface, key: &mut u32) -> i32 {
    if surface.is_null() {
        return -1;
    }
    SDL_GetColorKey(surface, key)
}

pub(super) unsafe fn set_color_key(surface: *mut SDL_Surface, key: u32, rleaccel: i32) -> i32 {
    if surface.is_null() {
        return -1;
    }
    SDL_SetSurfaceRLE(surface, rleaccel);
    SDL_SetColorKey(surface, 1, key)
}

pub(super) unsafe fn disable_color_key(surface: *mut SDL_Surface) -> i32 {
    if surface.is_null() {
        return -1;
    }
    SDL_SetColorKey(surface, 0, 0)
}

pub(super) unsafe fn has_surface_alpha_mod(surface: *mut SDL_Surface) -> bool {
    if surface.is_null() {
        return false;
    }
    let mut mode: u32 = 0;
    if SDL_GetSurfaceBlendMode(surface, (&mut mode as *mut u32).cast::<SDL_BlendMode>()) != 0 {
        return false;
    }
    mode == SDL_BlendMode::SDL_BLENDMODE_BLEND as u32
}

pub(super) unsafe fn set_surface_alpha_mod(surface: *mut SDL_Surface, alpha: u8) -> i32 {
    if surface.is_null() {
        return -1;
    }
    let result = SDL_SetSurfaceBlendMode(surface, SDL_BlendMode::SDL_BLENDMODE_BLEND);
    if result == 0 {
        SDL_SetSurfaceAlphaMod(surface, alpha)
    } else {
        result
    }
}

pub(super) unsafe fn disable_surface_alpha_mod(surface: *mut SDL_Surface) -> i32 {
    if surface.is_null() {
        return -1;
    }
    SDL_SetSurfaceAlphaMod(surface, 255);
    SDL_SetSurfaceBlendMode(surface, SDL_BlendMode::SDL_BLENDMODE_NONE)
}

/// Returns 1 when the palette was updated and 0 otherwise (SDL1 convention).
pub(super) unsafe fn set_colors(
    surface: *mut SDL_Surface,
    colors: *const SDL_Color,
    first_color: i32,
    ncolors: i32,
) -> i32 {
    if surface.is_null()
        || colors.is_null()
        || (*surface).format.is_null()
        || (*(*surface).format).palette.is_null()
    {
        return 0;
    }
    i32::from(SDL_SetPaletteColors((*(*surface).format).palette, colors, first_color, ncolors) == 0)
}

pub(super) fn color_to_native(color: Color) -> SDL_Color {
    SDL_Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

pub(super) fn native_to_color(native: SDL_Color) -> Color {
    Color {
        r: native.r,
        g: native.g,
        b: native.b,
        a: 0xff,
    }
}

pub(super) fn rect_from(corner_x: i16, corner_y: i16, width: i16, height: i16) -> SDL_Rect {
    SDL_Rect {
        x: i32::from(corner_x),
        y: i32::from(corner_y),
        w: i32::from(width),
        h: i32::from(height),
    }
}

/// `log_Fatal` followed by `exit (EXIT_FAILURE)`.
pub(super) fn fatal(message: &str) -> ! {
    log::error!("{message}");
    std::process::exit(1)
}

/// The 256 `SDL_Color` entries of a colormap's native palette.
pub(super) unsafe fn cmap_colors(cmap: *const TFB_ColorMap) -> *const SDL_Color {
    // NativePalette is `SDL_Color colors[256]` and nothing else.
    (*cmap).palette.cast()
}
