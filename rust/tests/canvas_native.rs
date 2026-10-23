//! Canvas/image integration against the production linked providers.
use sdl2_sys::{SDL_CreateRGBSurface, SDL_FreeSurface, SDL_Surface};
use std::{
    ffi::{c_char, c_void},
    ptr,
};
use uqm_rust::graphics::{
    native_abi::{Color, DrawMode, TFB_Image, HOT_SPOT},
    native_canvas::{drawing::*, lifecycle::*, DRAW_REPLACE, TFB_SCALE_NEAREST},
};
#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
extern "C" {
    fn CreateMutex_Core(name: *const c_char, class: u32) -> *mut c_void;
    fn DestroyMutex(mutex: *mut c_void);
}
const RGBA_MASKS: (u32, u32, u32, u32) = (0xff, 0xff00, 0xff_0000, 0xff00_0000);

unsafe fn surface(w: i32, h: i32, alpha: bool) -> *mut SDL_Surface {
    let amask = if alpha { RGBA_MASKS.3 } else { 0 };
    let s = SDL_CreateRGBSurface(0, w, h, 32, RGBA_MASKS.0, RGBA_MASKS.1, RGBA_MASKS.2, amask);
    assert!(!s.is_null());
    s
}

unsafe fn paletted(w: i32, h: i32) -> *mut SDL_Surface {
    let s = SDL_CreateRGBSurface(0, w, h, 8, 0, 0, 0, 0);
    assert!(!s.is_null());
    s
}

unsafe fn px(s: *mut SDL_Surface, x: i32, y: i32) -> u32 {
    *(*s)
        .pixels
        .cast::<u8>()
        .offset((y * (*s).pitch + x * 4) as isize)
        .cast::<u32>()
}

unsafe fn put(s: *mut SDL_Surface, x: i32, y: i32, value: u32) {
    *(*s)
        .pixels
        .cast::<u8>()
        .offset((y * (*s).pitch + x * 4) as isize)
        .cast::<u32>() = value;
}

fn mode(kind: u8, factor: i16) -> DrawMode {
    DrawMode { kind, factor }
}

fn color(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

#[test]
fn unscaled_image_is_drawn_relative_to_its_hotspot() {
    unsafe {
        let normal = surface(2, 2, false);
        put(normal, 0, 0, rgb(1, 2, 3));
        let mut img: TFB_Image = std::mem::zeroed();
        img.NormalImg = normal.cast();
        img.mutex = CreateMutex_Core(c"canvas integration".as_ptr(), 4);
        img.NormalHs = HOT_SPOT { x: 1, y: 1 };
        let dst = surface(4, 4, false);
        TFB_DrawCanvas_Image(
            &mut img,
            2,
            2,
            0,
            0,
            ptr::null_mut(),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        assert_eq!(px(dst, 1, 1), rgb(1, 2, 3));
        assert_eq!(px(dst, 2, 2), 0);
        DestroyMutex(img.mutex);
        SDL_FreeSurface(normal);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn filled_image_paints_present_pixels_and_caches_the_fill() {
    unsafe {
        let normal = surface(2, 1, true);
        put(normal, 0, 0, 0xff10_2030);
        put(normal, 1, 0, 0);
        let mut img: TFB_Image = std::mem::zeroed();
        img.NormalImg = normal.cast();
        img.mutex = CreateMutex_Core(c"canvas integration".as_ptr(), 4);
        let dst = surface(2, 1, true);
        let fill_color = color(1, 2, 3, 255);

        TFB_DrawCanvas_FilledImage(
            &mut img,
            0,
            0,
            0,
            0,
            fill_color,
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        assert_eq!(px(dst, 0, 0), rgb(1, 2, 3) | 0xff00_0000);
        assert_eq!(px(dst, 1, 0), 0);
        assert!(!img.FilledImg.is_null());
        assert_eq!(img.last_fill.r, 1);
        SDL_FreeSurface(img.FilledImg.cast());
        DestroyMutex(img.mutex);
        SDL_FreeSurface(normal);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn filled_paletted_image_recolors_the_whole_palette() {
    unsafe {
        let normal = paletted(2, 1);
        let mut img: TFB_Image = std::mem::zeroed();
        img.NormalImg = normal.cast();
        img.mutex = CreateMutex_Core(c"canvas integration".as_ptr(), 4);
        let dst = surface(2, 1, false);
        TFB_DrawCanvas_FilledImage(
            &mut img,
            0,
            0,
            0,
            0,
            color(4, 5, 6, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        let entry = *(*(*(*normal).format).palette).colors.add(9);
        assert_eq!((entry.r, entry.g, entry.b), (4, 5, 6));
        assert_eq!(img.colormap_version, -1);
        DestroyMutex(img.mutex);
        SDL_FreeSurface(normal);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn scale_target_reuses_matching_canvas_and_replaces_other_kinds() {
    unsafe {
        let src = surface(4, 3, true);
        let first =
            TFB_DrawCanvas_New_ScaleTarget(src.cast(), ptr::null_mut(), TFB_SCALE_NEAREST, 0);
        let first_surf: *mut SDL_Surface = first.cast();
        assert_eq!(((*first_surf).w, (*first_surf).h), (4, 3));

        let reused =
            TFB_DrawCanvas_New_ScaleTarget(src.cast(), first, TFB_SCALE_NEAREST, TFB_SCALE_NEAREST);
        assert_eq!(reused, first);
        TFB_DrawCanvas_Delete(reused);
        SDL_FreeSurface(src);
    }
}
