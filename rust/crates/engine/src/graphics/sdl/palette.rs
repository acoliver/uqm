//! Native SDL palette ABI. Ported from palette.c (GPL-2.0-or-later).
use crate::memory::{rust_hcalloc, rust_hfree};
use sdl2_sys::SDL_Color;

#[repr(C)]
pub struct NativePalette {
    pub colors: [SDL_Color; 256],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[no_mangle]
pub extern "C" fn AllocNativePalette() -> *mut NativePalette {
    unsafe { rust_hcalloc(std::mem::size_of::<NativePalette>()).cast() }
}

/// # Safety
/// `palette` must be null or an allocation returned by AllocNativePalette.
#[no_mangle]
pub unsafe extern "C" fn FreeNativePalette(palette: *mut NativePalette) {
    rust_hfree(palette.cast());
}

/// # Safety
/// `palette` must be live and index must be in 0..256.
#[no_mangle]
pub unsafe extern "C" fn SetNativePaletteColor(
    palette: *mut NativePalette,
    index: i32,
    color: Color,
) {
    assert!((0..256).contains(&index));
    (*palette).colors[index as usize] = SDL_Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    };
}

/// # Safety
/// `palette` must be live and index must be in 0..256.
#[no_mangle]
pub unsafe extern "C" fn GetNativePaletteColor(palette: *mut NativePalette, index: i32) -> Color {
    assert!((0..256).contains(&index));
    let color = (*palette).colors[index as usize];
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_alpha_is_stored_but_engine_reads_opaque() {
        unsafe {
            let p = AllocNativePalette();
            assert_eq!((*p).colors[255].r, 0);
            assert_eq!((*p).colors[255].a, 0);
            let c = Color {
                r: 19,
                g: 87,
                b: 203,
                a: 41,
            };
            SetNativePaletteColor(p, 255, c);
            assert_eq!((*p).colors[255].a, 41);
            assert_eq!(GetNativePaletteColor(p, 255), Color { a: 255, ..c });
            FreeNativePalette(p);
        }
    }
}
