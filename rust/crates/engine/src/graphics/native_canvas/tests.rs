use super::drawing::*;
use super::lifecycle::*;
use super::pixels::*;
use super::rotation::*;
use super::scaling::*;
use super::surface::{get_color_key, has_surface_alpha_mod, FALSE, TRUE};
use super::transparency::*;
use super::trilinear::*;
use super::{DRAW_ADDITIVE, DRAW_ALPHA, DRAW_REPLACE, TFB_SCALE_BILINEAR, TFB_SCALE_NEAREST};
use crate::graphics::native_abi::{
    Color, DrawMode, TFB_Char, TFB_Image, TFB_PixelFormat, EXTENT, HOT_SPOT, POINT, RECT,
};
use crate::memory::rust_hfree;
use sdl2_sys::{SDL_CreateRGBSurface, SDL_FreeSurface, SDL_Rect, SDL_Surface};
use std::ptr;

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

fn rect(x: i16, y: i16, w: i16, h: i16) -> RECT {
    RECT {
        corner: POINT { x, y },
        extent: EXTENT {
            width: w,
            height: h,
        },
    }
}

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

#[test]
fn weight_table_and_fixed_point_helpers_match_the_c_arithmetic() {
    assert_eq!(BTABLE[255][255], 254);
    assert_eq!(BTABLE[0][200], 0);
    assert_eq!(BTABLE[7][200], BTABLE[200][7]);
    assert_eq!(BTABLE[100][254], 99);
    // the weighted sum wraps like the C Uint8 return value
    assert_eq!(dot_product_8_4(&[[0, 0, 0, 255]; 4], 3, &[254; 4]), 244);
    assert_eq!(blend_ratio_2(200, 100, 128), 150);
    assert_eq!(blend_ratio_2(100, 200, 128), 150);
    assert_eq!(
        weight_product_8_4(
            &[[10, 0, 0, 0], [30, 0, 0, 0], [0; 4], [0; 4]],
            0,
            &[1, 1, 0, 0]
        ),
        20
    );
}

#[test]
fn rect_replace_fills_only_inside_the_clip_rect() {
    unsafe {
        let dst = surface(6, 4, false);
        TFB_DrawCanvas_SetClipRect(dst.cast(), &rect(1, 1, 3, 2));
        let mut area = rect(0, 0, 6, 4);
        TFB_DrawCanvas_Rect(
            &mut area,
            color(10, 20, 30, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        let fill = rgb(10, 20, 30);
        assert_eq!(px(dst, 1, 1), fill);
        assert_eq!(px(dst, 3, 2), fill);
        assert_eq!(px(dst, 0, 0), 0);
        assert_eq!(px(dst, 4, 1), 0);
        assert_eq!(px(dst, 1, 3), 0);

        TFB_DrawCanvas_SetClipRect(dst.cast(), ptr::null());
        assert_eq!(((*dst).clip_rect.w, (*dst).clip_rect.h), (6, 4));
        SDL_FreeSurface(dst);
    }
}

#[test]
fn translucent_color_on_opaque_surface_becomes_alpha_blend() {
    unsafe {
        let dst = surface(2, 2, false);
        let mut area = rect(0, 0, 2, 2);
        TFB_DrawCanvas_Rect(
            &mut area,
            color(100, 100, 100, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        TFB_DrawCanvas_Rect(
            &mut area,
            color(200, 200, 200, 128),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        assert_eq!(px(dst, 1, 1), rgb(150, 150, 150));
        SDL_FreeSurface(dst);
    }
}

#[test]
fn additive_rect_saturates_each_channel() {
    unsafe {
        let dst = surface(1, 1, false);
        let mut area = rect(0, 0, 1, 1);
        TFB_DrawCanvas_Rect(
            &mut area,
            color(200, 5, 0, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        TFB_DrawCanvas_Rect(
            &mut area,
            color(100, 50, 0, 255),
            mode(DRAW_ADDITIVE, 255),
            dst.cast(),
        );
        assert_eq!(px(dst, 0, 0), rgb(255, 55, 0));
        SDL_FreeSurface(dst);
    }
}

#[test]
fn rect_matching_the_colorkey_on_an_alpha_surface_is_transparent() {
    unsafe {
        let dst = surface(2, 2, true);
        TFB_DrawCanvas_SetTransparentColor(dst.cast(), color(5, 6, 7, 255), FALSE);
        let mut area = rect(0, 0, 2, 2);
        TFB_DrawCanvas_Rect(
            &mut area,
            color(5, 6, 7, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        assert_eq!(px(dst, 1, 1), rgb(5, 6, 7));
        TFB_DrawCanvas_Rect(
            &mut area,
            color(9, 6, 7, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        assert_eq!(px(dst, 1, 1), rgb(9, 6, 7) | 0xff00_0000);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn line_is_clipped_to_the_surface_clip_rect() {
    unsafe {
        let dst = surface(5, 3, false);
        TFB_DrawCanvas_SetClipRect(dst.cast(), &rect(1, 0, 3, 3));
        TFB_DrawCanvas_Line(
            -2,
            1,
            6,
            1,
            color(9, 9, 9, 255),
            mode(DRAW_REPLACE, 0),
            dst.cast(),
        );
        for x in 0..5 {
            let expected = if (1..=3).contains(&x) {
                rgb(9, 9, 9)
            } else {
                0
            };
            assert_eq!(px(dst, x, 1), expected, "x={x}");
            assert_eq!(px(dst, x, 0), 0);
        }
        SDL_FreeSurface(dst);
    }
}

#[test]
fn blit_modes_replace_surface_alpha_and_additive() {
    unsafe {
        let src = surface(2, 1, false);
        put(src, 0, 0, rgb(10, 10, 10));
        put(src, 1, 0, rgb(20, 20, 20));
        let dst = surface(4, 1, false);
        for x in 0..4 {
            put(dst, x, 0, rgb(100, 100, 100));
        }

        let mut at = SDL_Rect {
            x: 1,
            y: 0,
            w: 2,
            h: 1,
        };
        blit(src, ptr::null_mut(), dst, &mut at, mode(DRAW_REPLACE, 0));
        assert_eq!(px(dst, 0, 0), rgb(100, 100, 100));
        assert_eq!(px(dst, 1, 0), rgb(10, 10, 10));
        assert_eq!(px(dst, 2, 0), rgb(20, 20, 20));

        for x in 0..4 {
            put(dst, x, 0, rgb(100, 100, 100));
        }
        let mut at = SDL_Rect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        };
        blit(src, ptr::null_mut(), dst, &mut at, mode(DRAW_ALPHA, 128));
        let blended = px(dst, 0, 0) & 0xff;
        assert!(10 < blended && blended < 100, "blended={blended}");
        assert!(
            !has_surface_alpha_mod(src),
            "surface alpha must be restored"
        );

        for x in 0..4 {
            put(dst, x, 0, rgb(50, 50, 50));
        }
        let mut at = SDL_Rect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        };
        blit(src, ptr::null_mut(), dst, &mut at, mode(DRAW_ADDITIVE, 255));
        assert_eq!(px(dst, 0, 0), rgb(60, 60, 60));
        assert_eq!(px(dst, 1, 0), rgb(70, 70, 70));
        assert_eq!(px(dst, 2, 0), rgb(50, 50, 50));
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn copy_rect_moves_a_sub_rectangle() {
    unsafe {
        let src = surface(3, 3, false);
        put(src, 1, 1, rgb(1, 2, 3));
        let dst = surface(3, 3, false);
        TFB_DrawCanvas_CopyRect(
            src.cast(),
            &rect(1, 1, 1, 1),
            dst.cast(),
            POINT { x: 2, y: 0 },
        );
        assert_eq!(px(dst, 2, 0), rgb(1, 2, 3));
        assert_eq!(px(dst, 1, 1), 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn fill_replaces_colorkeyed_source_with_fill_color() {
    unsafe {
        let src = surface(2, 1, false);
        put(src, 0, 0, rgb(1, 1, 1));
        put(src, 1, 0, rgb(2, 2, 2));
        super::surface::set_color_key(src, rgb(1, 1, 1), 0);
        let dst = surface(2, 1, false);
        fill(src, rgb(7, 8, 9), dst);
        assert_eq!(px(dst, 0, 0), 0);
        assert_eq!(px(dst, 1, 0), rgb(7, 8, 9));
        let mut key = 1;
        assert_eq!(get_color_key(dst, &mut key), 0);
        assert_eq!(key, 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn fill_keeps_source_alpha_and_modulates_it_for_draw_alpha() {
    unsafe {
        let src = surface(3, 1, true);
        put(src, 0, 0, 0);
        put(src, 1, 0, 0xff00_0000);
        put(src, 2, 0, 0xff00_0000);
        let dst = surface(3, 1, true);
        fill(src, rgb(7, 8, 9) | 0xff00_0000, dst);
        assert_eq!(px(dst, 0, 0), 0);
        assert_eq!(px(dst, 1, 0), rgb(7, 8, 9) | 0xff00_0000);

        fill(src, rgb(7, 8, 9) | 0x8000_0000, dst);
        assert_eq!(px(dst, 2, 0), rgb(7, 8, 9) | 0x7f00_0000);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn font_char_alpha_is_transferred_to_the_backing_surface() {
    unsafe {
        let backing_surf = surface(4, 4, true);
        for y in 0..4 {
            for x in 0..4 {
                put(backing_surf, x, y, 0x00ff_ffff);
            }
        }
        let mut backing: TFB_Image = std::mem::zeroed();
        backing.NormalImg = backing_surf.cast();
        let mut data = [255u8, 0, 128, 64];
        let mut glyph: TFB_Char = std::mem::zeroed();
        glyph.extent = EXTENT {
            width: 2,
            height: 2,
        };
        glyph.pitch = 2;
        glyph.data = data.as_mut_ptr();
        glyph.HotSpot = HOT_SPOT { x: 0, y: 0 };
        let dst = surface(4, 4, true);

        draw_font_char_locked(&mut glyph, &mut backing, 1, 1, mode(DRAW_REPLACE, 0), dst);
        assert_eq!(px(backing_surf, 0, 0) >> 24, 255);
        assert_eq!(px(backing_surf, 1, 0) >> 24, 0);
        assert_eq!(px(backing_surf, 0, 1) >> 24, 128);
        assert_eq!(px(backing_surf, 1, 1) >> 24, 64);
        assert_eq!(px(dst, 1, 1), 0xffff_ffff);
        assert_eq!(px(dst, 2, 1), 0);

        draw_font_char_locked(&mut glyph, &mut backing, 0, 0, mode(DRAW_ALPHA, 128), dst);
        assert_eq!(px(backing_surf, 0, 0) >> 24, 127);
        assert_eq!(px(backing_surf, 0, 1) >> 24, 64);
        assert_eq!(px(backing_surf, 1, 1) >> 24, 32);
        SDL_FreeSurface(backing_surf);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn font_char_rejects_a_backing_surface_that_is_too_small() {
    unsafe {
        let backing_surf = surface(1, 1, true);
        let mut backing: TFB_Image = std::mem::zeroed();
        backing.NormalImg = backing_surf.cast();
        let mut data = [255u8; 4];
        let mut glyph: TFB_Char = std::mem::zeroed();
        glyph.extent = EXTENT {
            width: 2,
            height: 2,
        };
        glyph.pitch = 2;
        glyph.data = data.as_mut_ptr();
        let dst = surface(4, 4, true);
        draw_font_char_locked(&mut glyph, &mut backing, 0, 0, mode(DRAW_REPLACE, 0), dst);
        assert_eq!(px(dst, 0, 0), 0);
        assert_eq!(px(backing_surf, 0, 0), 0);
        SDL_FreeSurface(backing_surf);
        SDL_FreeSurface(dst);
    }
}

unsafe fn extent_of(
    src: *mut SDL_Surface,
    hs: (i16, i16),
    scale: i32,
    kind: i32,
) -> (EXTENT, HOT_SPOT) {
    let mut src_hs = HOT_SPOT { x: hs.0, y: hs.1 };
    let mut size = EXTENT {
        width: 0,
        height: 0,
    };
    let mut out_hs = HOT_SPOT { x: 0, y: 0 };
    TFB_DrawCanvas_GetScaledExtent(
        src.cast(),
        &mut src_hs,
        ptr::null_mut(),
        ptr::null_mut(),
        scale,
        kind,
        &mut size,
        &mut out_hs,
    );
    (size, out_hs)
}

#[test]
fn scaled_extent_aligns_hotspot_and_pads_for_filtered_scalers() {
    unsafe {
        let src = surface(10, 10, true);
        let (size, hs) = extent_of(src, (5, 5), 128, TFB_SCALE_NEAREST);
        assert_eq!((size.width, size.height, hs.x, hs.y), (5, 5, 2, 2));

        let (size, hs) = extent_of(src, (5, 5), 128, TFB_SCALE_BILINEAR);
        assert_eq!((size.width, size.height, hs.x, hs.y), (6, 6, 3, 3));

        let (size, _) = extent_of(src, (0, 0), 1, TFB_SCALE_NEAREST);
        assert_eq!((size.width, size.height), (1, 1), "never collapses to zero");

        let mut extent = EXTENT {
            width: 0,
            height: 0,
        };
        TFB_DrawCanvas_GetExtent(src.cast(), &mut extent);
        assert_eq!((extent.width, extent.height), (10, 10));
        SDL_FreeSurface(src);
    }
}

#[test]
fn mipmap_scaled_extent_interpolates_between_source_and_mipmap() {
    unsafe {
        let src = surface(8, 8, true);
        let mm = surface(4, 4, true);
        let mut src_hs = HOT_SPOT { x: 4, y: 4 };
        let mut mm_hs = HOT_SPOT { x: 2, y: 2 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut hs = HOT_SPOT { x: 0, y: 0 };
        // ratio = 256: the extent is exactly the source extent
        TFB_DrawCanvas_GetScaledExtent(
            src.cast(),
            &mut src_hs,
            mm.cast(),
            &mut mm_hs,
            256,
            super::TFB_SCALE_TRILINEAR,
            &mut size,
            &mut hs,
        );
        assert_eq!((size.width, size.height, hs.x, hs.y), (8, 8, 4, 4));
        SDL_FreeSurface(src);
        SDL_FreeSurface(mm);
    }
}

#[test]
fn nearest_rescale_picks_source_pixels_with_the_biased_step() {
    unsafe {
        let src = surface(4, 4, false);
        for y in 0..4 {
            for x in 0..4 {
                put(src, x, y, (y * 4 + x + 1) as u32);
            }
        }
        let dst = surface(2, 2, false);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Nearest(
            src.cast(),
            dst.cast(),
            128,
            &mut src_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!((size.width, size.height), (2, 2));
        assert_eq!(px(dst, 0, 0), 1);
        assert_eq!(px(dst, 1, 0), 4);
        assert_eq!(px(dst, 0, 1), 13);
        assert_eq!(px(dst, 1, 1), 16);

        // scale <= 0 targets the destination dimensions and leaves the
        // extent and hotspot pointers alone
        let dst = surface(2, 2, false);
        TFB_DrawCanvas_Rescale_Nearest(
            src.cast(),
            dst.cast(),
            -1,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        assert_eq!(px(dst, 1, 1), 16);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn nearest_rescale_copies_eight_bit_indexes() {
    unsafe {
        let src = paletted(4, 1);
        for x in 0..4 {
            *(*src).pixels.cast::<u8>().add(x) = 10 + x as u8;
        }
        let dst = paletted(2, 1);
        TFB_DrawCanvas_Rescale_Nearest(
            src.cast(),
            dst.cast(),
            -1,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        assert_eq!(*(*dst).pixels.cast::<u8>(), 10);
        assert_eq!(*(*dst).pixels.cast::<u8>().add(1), 13);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn nearest_rescale_refuses_a_destination_that_is_too_small() {
    unsafe {
        let src = surface(4, 4, false);
        put(src, 0, 0, 5);
        let dst = surface(1, 1, false);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Nearest(
            src.cast(),
            dst.cast(),
            256,
            &mut src_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!(px(dst, 0, 0), 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

unsafe fn uniform(w: i32, h: i32, alpha: bool, r: u8, g: u8, b: u8) -> *mut SDL_Surface {
    let s = surface(w, h, alpha);
    let value = rgb(u32::from(r), u32::from(g), u32::from(b)) | if alpha { 0xff00_0000 } else { 0 };
    for y in 0..h {
        for x in 0..w {
            put(s, x, y, value);
        }
    }
    s
}

#[test]
fn bilinear_identity_scale_reproduces_a_uniform_opaque_image() {
    unsafe {
        let src = uniform(4, 4, false, 10, 20, 30);
        let dst = surface(5, 5, false);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Bilinear(
            src.cast(),
            dst.cast(),
            256,
            &mut src_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!((size.width, size.height), (4, 4));
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(px(dst, x, y), rgb(10, 20, 30), "({x},{y})");
            }
        }
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn bilinear_rgba_destination_snaps_near_opaque_alpha_to_opaque() {
    unsafe {
        let src = uniform(4, 4, true, 10, 20, 30);
        let dst = surface(5, 5, true);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Bilinear(
            src.cast(),
            dst.cast(),
            256,
            &mut src_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!(px(dst, 1, 1), rgb(10, 20, 30) | 0xff00_0000);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn bilinear_keyed_out_pixels_become_the_destination_transparent_color() {
    unsafe {
        let src = uniform(4, 4, false, 10, 20, 30);
        super::surface::set_color_key(src, rgb(10, 20, 30), 0);
        let dst = surface(5, 5, false);
        put(dst, 0, 0, 0x00ab_cdef);
        super::surface::set_color_key(dst, 0x0011_2233, 0);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Bilinear(
            src.cast(),
            dst.cast(),
            256,
            &mut src_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!(px(dst, 1, 1), 0x0011_2233);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn trilinear_blends_uniform_source_and_mipmap() {
    unsafe {
        let src = uniform(4, 4, false, 100, 100, 100);
        let mm = uniform(2, 2, false, 100, 100, 100);
        let dst = surface(5, 5, false);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut mm_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Trilinear(
            src.cast(),
            mm.cast(),
            dst.cast(),
            256,
            &mut src_hs,
            &mut mm_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!((size.width, size.height), (4, 4));
        // weights of 254/255 map 100 to 99 in both images; the blend keeps it
        assert_eq!(px(dst, 0, 0), rgb(99, 99, 99));
        SDL_FreeSurface(src);
        SDL_FreeSurface(mm);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn trilinear_refuses_a_paletted_mipmap_for_a_truecolor_source() {
    unsafe {
        let src = uniform(4, 4, false, 1, 2, 3);
        let mm = paletted(2, 2);
        let dst = surface(5, 5, false);
        let mut src_hs = HOT_SPOT { x: 0, y: 0 };
        let mut mm_hs = HOT_SPOT { x: 0, y: 0 };
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        let mut dst_hs = HOT_SPOT { x: 0, y: 0 };
        TFB_DrawCanvas_Rescale_Trilinear(
            src.cast(),
            mm.cast(),
            dst.cast(),
            256,
            &mut src_hs,
            &mut mm_hs,
            &mut size,
            &mut dst_hs,
        );
        assert_eq!(px(dst, 0, 0), 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(mm);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn pixel_colors_round_trip_and_clamp_to_the_canvas() {
    unsafe {
        let canvas = surface(2, 2, true);
        let colors = [color(1, 2, 3, 4), color(5, 6, 7, 8), color(9, 10, 11, 12)];
        // a 3x1 array on a 2x2 canvas only transfers the overlapping pixels
        assert_eq!(
            TFB_DrawCanvas_SetPixelColors(canvas.cast(), colors.as_ptr(), 3, 1),
            TRUE
        );
        assert_eq!(px(canvas, 0, 0), 0x0403_0201);
        assert_eq!(px(canvas, 1, 0), 0x0807_0605);
        assert_eq!(px(canvas, 0, 1), 0);

        let mut out = [color(0, 0, 0, 0); 3];
        assert_eq!(
            TFB_DrawCanvas_GetPixelColors(canvas.cast(), out.as_mut_ptr(), 3, 1),
            TRUE
        );
        assert_eq!((out[1].r, out[1].a), (5, 8));
        assert_eq!(out[2].r, 0, "outside the canvas is untouched");
        assert_eq!(
            TFB_DrawCanvas_GetPixelColors(ptr::null_mut(), out.as_mut_ptr(), 1, 1),
            FALSE
        );
        SDL_FreeSurface(canvas);
    }
}

#[test]
fn pixel_indexes_round_trip_on_indexed_canvases_only() {
    unsafe {
        let canvas = paletted(3, 2);
        let data = [1u8, 2, 3, 4, 5, 6];
        assert_eq!(
            TFB_DrawCanvas_SetPixelIndexes(canvas.cast(), data.as_ptr(), 3, 2),
            TRUE
        );
        let mut out = [0u8; 6];
        assert_eq!(
            TFB_DrawCanvas_GetPixelIndexes(canvas.cast(), out.as_mut_ptr(), 3, 2),
            TRUE
        );
        assert_eq!(out, data);

        let truecolor = surface(3, 2, false);
        assert_eq!(
            TFB_DrawCanvas_GetPixelIndexes(truecolor.cast(), out.as_mut_ptr(), 3, 2),
            FALSE
        );
        SDL_FreeSurface(canvas);
        SDL_FreeSurface(truecolor);
    }
}

#[test]
fn get_pixel_reads_in_bounds_and_returns_zero_outside() {
    unsafe {
        let canvas = surface(2, 2, false);
        put(canvas, 1, 1, rgb(11, 22, 33));
        let c = TFB_DrawCanvas_GetPixel(canvas.cast(), 1, 1);
        assert_eq!((c.r, c.g, c.b, c.a), (11, 22, 33, 255));
        let outside = TFB_DrawCanvas_GetPixel(canvas.cast(), 2, 0);
        assert_eq!((outside.r, outside.g, outside.b, outside.a), (0, 0, 0, 0));
        SDL_FreeSurface(canvas);
    }
}

#[test]
fn font_char_data_uses_alpha_or_weighted_intensity() {
    unsafe {
        let alpha = surface(2, 1, true);
        put(alpha, 0, 0, 0x4000_0000);
        put(alpha, 1, 0, 0xff00_0000);
        let mut out = [0u8; 2];
        assert_eq!(
            TFB_DrawCanvas_GetFontCharData(alpha.cast(), out.as_mut_ptr(), 2),
            TRUE
        );
        assert_eq!(out, [0x40, 0xff]);

        let plain = surface(2, 1, false);
        put(plain, 0, 0, rgb(7, 7, 7));
        put(plain, 1, 0, rgb(255, 0, 0));
        assert_eq!(
            TFB_DrawCanvas_GetFontCharData(plain.cast(), out.as_mut_ptr(), 2),
            TRUE
        );
        assert_eq!(out, [7, 72]);
        assert_eq!(
            TFB_DrawCanvas_GetFontCharData(ptr::null_mut(), out.as_mut_ptr(), 2),
            FALSE
        );
        SDL_FreeSurface(alpha);
        SDL_FreeSurface(plain);
    }
}

#[test]
fn intersect_reports_overlap_of_non_transparent_pixels() {
    unsafe {
        let one = surface(3, 3, true);
        let two = surface(3, 3, true);
        put(one, 2, 2, 0xff00_0001);
        put(two, 0, 0, 0xff00_0001);
        let inter = rect(2, 2, 1, 1);
        let hit = TFB_DrawCanvas_Intersect(
            one.cast(),
            POINT { x: 0, y: 0 },
            two.cast(),
            POINT { x: 2, y: 2 },
            &inter,
        );
        assert_eq!(hit, TRUE);

        put(two, 0, 0, 0);
        let miss = TFB_DrawCanvas_Intersect(
            one.cast(),
            POINT { x: 0, y: 0 },
            two.cast(),
            POINT { x: 2, y: 2 },
            &inter,
        );
        assert_eq!(miss, FALSE);
        SDL_FreeSurface(one);
        SDL_FreeSurface(two);
    }
}

#[test]
fn intersect_uses_colorkey_when_the_surface_has_no_alpha() {
    unsafe {
        let one = surface(2, 1, false);
        let two = surface(2, 1, false);
        for s in [one, two] {
            put(s, 0, 0, rgb(1, 1, 1));
            put(s, 1, 0, rgb(9, 9, 9));
            super::surface::set_color_key(s, rgb(1, 1, 1), 0);
        }
        let origin = POINT { x: 0, y: 0 };
        let only_key = rect(0, 0, 1, 1);
        assert_eq!(
            TFB_DrawCanvas_Intersect(one.cast(), origin, two.cast(), origin, &only_key),
            FALSE
        );
        let both = rect(0, 0, 2, 1);
        assert_eq!(
            TFB_DrawCanvas_Intersect(one.cast(), origin, two.cast(), origin, &both),
            TRUE
        );
        SDL_FreeSurface(one);
        SDL_FreeSurface(two);
    }
}

#[test]
fn transparent_color_round_trips_on_truecolor_canvases() {
    unsafe {
        let canvas = surface(2, 2, false);
        let mut out = color(0, 0, 0, 0);
        assert_eq!(
            TFB_DrawCanvas_GetTransparentColor(canvas.cast(), &mut out),
            FALSE
        );
        TFB_DrawCanvas_SetTransparentColor(canvas.cast(), color(1, 2, 3, 255), FALSE);
        assert_eq!(
            TFB_DrawCanvas_GetTransparentColor(canvas.cast(), &mut out),
            TRUE
        );
        assert_eq!((out.r, out.g, out.b, out.a), (1, 2, 3, 255));

        let copy = surface(2, 2, false);
        TFB_DrawCanvas_CopyTransparencyInfo(canvas.cast(), copy.cast());
        assert_eq!(
            TFB_DrawCanvas_GetTransparentColor(copy.cast(), &mut out),
            TRUE
        );
        assert_eq!((out.r, out.g, out.b), (1, 2, 3));
        SDL_FreeSurface(canvas);
        SDL_FreeSurface(copy);
    }
}

#[test]
fn paletted_canvas_keeps_its_palette_and_transparent_index() {
    unsafe {
        let mut palette = [color(0, 0, 0, 255); 256];
        palette[3] = color(30, 40, 50, 255);
        let canvas = TFB_DrawCanvas_New_Paletted(2, 2, palette.as_mut_ptr(), 3);
        assert_eq!(TFB_DrawCanvas_IsPaletted(canvas), TRUE);
        let mut key = 0;
        assert_eq!(get_color_key(canvas.cast(), &mut key), 0);
        assert_eq!(key, 3);

        let extracted = TFB_DrawCanvas_ExtractPalette(canvas);
        assert_eq!((*extracted.add(3)).g, 40);
        assert_eq!((*extracted.add(3)).a, 255);
        rust_hfree(extracted.cast());

        let no_key = TFB_DrawCanvas_New_Paletted(2, 2, ptr::null_mut(), -1);
        assert_eq!(get_color_key(no_key.cast(), &mut key), -1);
        let truecolor = surface(1, 1, false);
        assert!(TFB_DrawCanvas_ExtractPalette(truecolor.cast()).is_null());
        TFB_DrawCanvas_Delete(canvas);
        TFB_DrawCanvas_Delete(no_key);
        SDL_FreeSurface(truecolor);
    }
}

#[test]
fn true_color_canvases_follow_the_template_format() {
    unsafe {
        let template = surface(1, 1, true);
        let with_alpha = new_true_color_like(&*(*template).format, 3, 2, TRUE);
        assert_eq!(((*with_alpha).w, (*with_alpha).h), (3, 2));
        assert_eq!((*(*with_alpha).format).Amask, 0xff00_0000);
        let without_alpha = new_true_color_like(&*(*template).format, 3, 2, FALSE);
        assert_eq!((*(*without_alpha).format).Amask, 0);
        assert_eq!((*(*without_alpha).format).Rmask, 0xff);
        SDL_FreeSurface(template);
        SDL_FreeSurface(with_alpha);
        SDL_FreeSurface(without_alpha);
    }
}

#[test]
fn screen_format_substitutes_rgba_for_paletted_displays() {
    unsafe {
        let mut fmt: TFB_PixelFormat = std::mem::zeroed();
        let indexed = paletted(1, 1);
        fill_pixel_format(&*(*indexed).format, &mut fmt);
        assert_eq!(
            (fmt.BitsPerPixel, fmt.Rmask, fmt.Amask),
            (32, 0xff, 0xff00_0000)
        );

        let truecolor = surface(1, 1, false);
        fill_pixel_format(&*(*truecolor).format, &mut fmt);
        assert_eq!((fmt.BitsPerPixel, fmt.Gmask, fmt.Amask), (32, 0xff00, 0));
        SDL_FreeSurface(indexed);
        SDL_FreeSurface(truecolor);
    }
}

#[test]
fn rotation_target_matches_the_rotated_extent_and_source_format() {
    unsafe {
        let src = surface(4, 4, true);
        let mut size = EXTENT {
            width: 0,
            height: 0,
        };
        TFB_DrawCanvas_GetRotatedExtent(src.cast(), 0, &mut size);
        assert!(size.width >= 4 && size.height >= 4);
        let target = TFB_DrawCanvas_New_RotationTarget(src.cast(), 0);
        let target_surf: *mut SDL_Surface = target.cast();
        assert_eq!(
            ((*target_surf).w, (*target_surf).h),
            (i32::from(size.width), i32::from(size.height))
        );
        assert_eq!((*(*target_surf).format).Amask, 0xff00_0000);
        TFB_DrawCanvas_Delete(target);
        SDL_FreeSurface(src);
    }
}

#[test]
fn rotate_refuses_an_undersized_destination() {
    unsafe {
        let src = uniform(4, 4, false, 9, 9, 9);
        let dst = surface(2, 2, false);
        TFB_DrawCanvas_Rotate(
            src.cast(),
            dst.cast(),
            0,
            EXTENT {
                width: 4,
                height: 4,
            },
        );
        assert_eq!(px(dst, 0, 0), 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}

#[test]
fn stride_and_line_address_the_pixel_rows() {
    unsafe {
        let canvas = surface(3, 2, false);
        let stride = TFB_DrawCanvas_GetStride(canvas.cast());
        assert_eq!(stride, (*canvas).pitch);
        let row1 = TFB_DrawCanvas_GetLine(canvas.cast(), 1);
        assert_eq!(
            row1,
            (*canvas).pixels.cast::<u8>().add(stride as usize).cast()
        );
        TFB_DrawCanvas_Lock(canvas.cast());
        TFB_DrawCanvas_Unlock(canvas.cast());
        SDL_FreeSurface(canvas);
    }
}

#[test]
fn deleting_a_null_canvas_is_reported_not_fatal() {
    unsafe {
        TFB_DrawCanvas_Delete(ptr::null_mut());
    }
}

#[test]
fn extracted_palette_preserves_every_rgb_entry_and_copied_index_polarity() {
    // SAFETY: the test owns the live surfaces and the extracted palette allocation.
    unsafe {
        let src = paletted(1, 1);
        let dst = paletted(1, 1);
        let mut colors = [color(0, 0, 0, 255); 256];
        for (i, entry) in colors.iter_mut().enumerate() {
            *entry = color(i as u8, 255 - i as u8, 33, 255);
        }
        TFB_DrawCanvas_SetPalette(src.cast(), colors.as_mut_ptr());
        let extracted = TFB_DrawCanvas_ExtractPalette(src.cast());
        assert!(!extracted.is_null());
        for (i, expected) in colors.iter().enumerate() {
            let actual = *extracted.add(i);
            assert_eq!(
                (actual.r, actual.g, actual.b),
                (expected.r, expected.g, expected.b)
            );
        }
        crate::memory::rust_hfree(extracted.cast());
        TFB_DrawCanvas_SetTransparentIndex(src.cast(), 7, FALSE);
        assert_eq!(TFB_DrawCanvas_GetTransparentIndex(src.cast()), -1);
        TFB_DrawCanvas_CopyTransparencyInfo(src.cast(), dst.cast());
        let mut key = 0;
        assert_ne!(sdl2_sys::SDL_GetColorKey(dst, &mut key), 0);
        SDL_FreeSurface(src);
        SDL_FreeSurface(dst);
    }
}
