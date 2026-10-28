//! Canvas/image integration against the production linked providers.
use sdl2_sys::{SDL_CreateRGBSurface, SDL_FreeSurface, SDL_Surface};
use std::ptr;
use uqm_rust::graphics::{
    native_abi::{Color, DrawMode, TFB_Image, EXTENT, HOT_SPOT},
    native_canvas::{
        drawing::*,
        lifecycle::*,
        rotation::*,
        surface::{FALSE, TRUE},
        transparency::*,
        DRAW_REPLACE, TFB_SCALE_NEAREST,
    },
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

#[test]
fn rotation_preserves_pixels_at_zero_and_transfers_transparent_color() {
    // SAFETY: both SDL surfaces are owned by this test and remain live through each operation.
    unsafe {
        let src = surface(2, 2, false);
        let dst = surface(2, 2, false);
        put(src, 0, 0, rgb(10, 20, 30));
        put(src, 1, 0, rgb(40, 50, 60));
        put(src, 0, 1, rgb(70, 80, 90));
        put(src, 1, 1, rgb(100, 110, 120));
        TFB_DrawCanvas_SetTransparentColor(src.cast(), color(1, 2, 3, 255), FALSE);
        let size = EXTENT {
            width: 2,
            height: 2,
        };
        TFB_DrawCanvas_Rotate(src.cast(), dst.cast(), 0, size);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(px(dst, x, y), px(src, x, y));
            }
        }
        let mut transparent = color(0, 0, 0, 0);
        assert_eq!(
            TFB_DrawCanvas_GetTransparentColor(dst.cast(), &mut transparent),
            TRUE
        );
        assert_eq!((transparent.r, transparent.g, transparent.b), (1, 2, 3));
        TFB_DrawCanvas_Rotate(
            src.cast(),
            dst.cast(),
            90,
            EXTENT {
                width: 3,
                height: 2,
            },
        );
        assert_eq!(px(dst, 0, 0), rgb(10, 20, 30));
        let mut extent = EXTENT {
            width: 0,
            height: 0,
        };
        TFB_DrawCanvas_GetRotatedExtent(src.cast(), 90, &mut extent);
        assert!(extent.width >= 2 && extent.height >= 2);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn truecolor_transparency_copy_keeps_the_source_key_and_palette_absence() {
    // SAFETY: both SDL surfaces are owned by this test and remain live through each operation.
    unsafe {
        let src = surface(2, 2, false);
        let dst = surface(2, 2, false);
        assert!(TFB_DrawCanvas_ExtractPalette(src.cast()).is_null());
        TFB_DrawCanvas_SetTransparentColor(src.cast(), color(11, 22, 33, 255), FALSE);
        TFB_DrawCanvas_CopyTransparencyInfo(src.cast(), dst.cast());
        let mut found = color(0, 0, 0, 0);
        assert_eq!(
            TFB_DrawCanvas_GetTransparentColor(dst.cast(), &mut found),
            TRUE
        );
        assert_eq!((found.r, found.g, found.b, found.a), (11, 22, 33, 255));
        TFB_DrawCanvas_SetTransparentIndex(src.cast(), -1, FALSE);
        assert_eq!(
            TFB_DrawCanvas_GetTransparentColor(src.cast(), &mut found),
            FALSE
        );
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn palette_round_trip_and_transparency_copy_follow_indexed_surface_rules() {
    // SAFETY: each SDL surface and allocated palette is owned until freed below.
    unsafe {
        let src = paletted(2, 2);
        let dst = paletted(2, 2);
        let mut colors = [color(0, 0, 0, 255); 256];
        colors[7] = color(12, 34, 56, 255);
        TFB_DrawCanvas_SetPalette(src.cast(), colors.as_mut_ptr());
        let extracted = TFB_DrawCanvas_ExtractPalette(src.cast());
        assert!(!extracted.is_null());
        let entry = *extracted.add(7);
        assert_eq!((entry.r, entry.g, entry.b), (12, 34, 56));
        uqm_rust::memory::rust_hfree(extracted.cast());
        assert_eq!(TFB_DrawCanvas_IsPaletted(src.cast()), TRUE);
        TFB_DrawCanvas_SetTransparentIndex(src.cast(), 7, FALSE);
        assert_eq!(TFB_DrawCanvas_GetTransparentIndex(src.cast()), -1);
        TFB_DrawCanvas_CopyTransparencyInfo(src.cast(), dst.cast());
        let mut native_key = 0;
        assert_ne!(sdl2_sys::SDL_GetColorKey(dst, &mut native_key), 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}
