//! Canvas/image integration against the production linked providers.
use sdl2_sys::{SDL_CreateRGBSurface, SDL_FreeSurface, SDL_Surface};
use std::ptr;
use uqm_rust::graphics::{
    native_abi::{Color, DrawMode, TFB_Image, HOT_SPOT},
    native_canvas::{drawing::*, lifecycle::*, DRAW_REPLACE, TFB_SCALE_NEAREST},
};
use uqm_rust::threading::legacy_abi::{CreateMutex_Core, DestroyMutex};
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

#[test]
fn image_scaling_reuses_cache_until_dirty_then_refreshes_pixels() {
    use uqm_rust::graphics::tfb_draw_abi::TFB_DrawImage_FixScaling;
    unsafe {
        let source = surface(4, 4, false);
        put(source, 0, 0, rgb(12, 34, 56));
        let mut image: TFB_Image = std::mem::zeroed();
        image.NormalImg = source.cast();
        image.last_scale_type = -1;
        TFB_DrawImage_FixScaling(&mut image, 128, TFB_SCALE_NEAREST);
        let scaled: *mut SDL_Surface = image.ScaledImg.cast();
        assert_eq!((image.extent.width, image.extent.height), (2, 2));
        assert_eq!(
            (image.last_scale, image.last_scale_type, image.dirty),
            (128, TFB_SCALE_NEAREST, 0)
        );
        assert_eq!(px(scaled, 0, 0), rgb(12, 34, 56));
        put(source, 0, 0, rgb(78, 90, 12));
        TFB_DrawImage_FixScaling(&mut image, 128, TFB_SCALE_NEAREST);
        assert_eq!(image.ScaledImg, scaled.cast());
        assert_eq!(px(scaled, 0, 0), rgb(12, 34, 56));
        image.dirty = 1;
        TFB_DrawImage_FixScaling(&mut image, 128, TFB_SCALE_NEAREST);
        assert_eq!(image.ScaledImg, scaled.cast());
        assert_eq!(image.dirty, 0);
        assert_eq!(px(scaled, 0, 0), rgb(78, 90, 12));
        SDL_FreeSurface(scaled);
        SDL_FreeSurface(source);
    }
}
