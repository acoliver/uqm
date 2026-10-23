//! Mipmap-blended (trilinear) rescale.
use super::scaling::{
    at_shift, blend_ratio_2, btable, dot_product_8_4, scale_get_pixel, scale_read_pixel,
    weight_product_8_4, Pixel, TFB_DrawCanvas_GetScaledExtent,
};
use super::surface::{format, get_color_key};
use super::{GSCALE_IDENTITY, TFB_SCALE_TRILINEAR};
use crate::graphics::native_abi::{TFB_Canvas, EXTENT, HOT_SPOT};
use sdl2_sys::{SDL_Color, SDL_LockSurface, SDL_Surface, SDL_UnlockSurface};
use std::ptr;

/// Fractional start positions and steps for one source image.
struct Axis {
    start: i32,
    step: i32,
}

/// # Safety
/// Canvases must be live; hotspot and extent pointers are only used when
/// `scale > 0`.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Rescale_Trilinear(
    src_canvas: TFB_Canvas,
    src_mipmap: TFB_Canvas,
    dst_canvas: TFB_Canvas,
    scale: i32,
    src_hs: *mut HOT_SPOT,
    mm_hs: *mut HOT_SPOT,
    size: *mut EXTENT,
    dst_hs: *mut HOT_SPOT,
) {
    let src: *mut SDL_Surface = src_canvas.cast();
    let dst: *mut SDL_Surface = dst_canvas.cast();
    let mm: *mut SDL_Surface = src_mipmap.cast();
    let srcfmt = format(src);
    let mmfmt = format(mm);
    let dstfmt = format(dst);
    let srcpal: *const SDL_Color = if srcfmt.palette.is_null() {
        ptr::null()
    } else {
        (*srcfmt.palette).colors as *const SDL_Color
    };
    let sbpp = i32::from(srcfmt.BytesPerPixel);
    let mmbpp = i32::from(mmfmt.BytesPerPixel);
    let slen = (*src).pitch;
    let mmlen = (*mm).pitch;
    let dst_has_alpha = dstfmt.Amask != 0;
    let mut transparent = 0u32;
    let alpha_threshold: i32 = if dst_has_alpha { 0 } else { 127 };
    // src v. mipmap importance factor
    let ratio = scale * 2 - GSCALE_IDENTITY;
    // source masks and keys
    let (mut mk0, mut ck0, mut mk1, mut ck1) = (0u32, !0u32, 0u32, !0u32);
    let (w, h): (i32, i32);
    // source fractional starting points and dx and dy increments
    let (x0, y0, x1, y1): (Axis, Axis, Axis, Axis);

    get_color_key(dst, &mut transparent);

    if !mmfmt.palette.is_null() && srcpal.is_null() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Trilinear: Mipmap is paletted, but source is not! Failing."
        );
        return;
    }

    if scale > 0 {
        // Use (scale / GSCALE_IDENTITY) sizing factor
        TFB_DrawCanvas_GetScaledExtent(
            src_canvas,
            src_hs,
            src_mipmap,
            mm_hs,
            scale,
            TFB_SCALE_TRILINEAR,
            size,
            dst_hs,
        );

        w = i32::from((*size).width);
        h = i32::from((*size).height);

        let fw = (*mm).w * GSCALE_IDENTITY + ((*src).w - (*mm).w) * ratio;
        let fh = (*mm).h * GSCALE_IDENTITY + ((*src).h - (*mm).h) * ratio;

        // This limits the effective source dimensions to 2048x2048,
        // and we also lose 4 bits of precision out of 16 (no problem)
        let mut fsx0 = ((*src).w << 20) / fw;
        fsx0 <<= 4;
        let mut fsy0 = ((*src).h << 20) / fh;
        fsy0 <<= 4;

        let mut fsx1 = ((*mm).w << 20) / fw;
        fsx1 <<= 4;
        let mut fsy1 = ((*mm).h << 20) / fh;
        fsy1 <<= 4;

        // position the hotspots directly over each other
        let (dst_x, dst_y) = (i32::from((*dst_hs).x), i32::from((*dst_hs).y));
        x0 = Axis {
            start: (i32::from((*src_hs).x) << 16) - fsx0 * dst_x,
            step: fsx0,
        };
        y0 = Axis {
            start: (i32::from((*src_hs).y) << 16) - fsy0 * dst_y,
            step: fsy0,
        };
        x1 = Axis {
            start: (i32::from((*mm_hs).x) << 16) - fsx1 * dst_x,
            step: fsx1,
        };
        y1 = Axis {
            start: (i32::from((*mm_hs).y) << 16) - fsy1 * dst_y,
            step: fsy1,
        };
    } else {
        // Just go with the dst surface dimensions
        w = (*dst).w;
        h = (*dst).h;

        let fsx0 = ((*src).w << 16) / w;
        let fsy0 = ((*src).h << 16) / h;

        let fsx1 = ((*mm).w << 16) / w;
        let fsy1 = ((*mm).h << 16) / h;

        // give equal importance to both edges
        x0 = Axis {
            start: ((((*src).w - 1) << 16) - fsx0 * (w - 1)) >> 1,
            step: fsx0,
        };
        y0 = Axis {
            start: ((((*src).h - 1) << 16) - fsy0 * (h - 1)) >> 1,
            step: fsy0,
        };
        x1 = Axis {
            start: ((((*mm).w - 1) << 16) - fsx1 * (w - 1)) >> 1,
            step: fsx1,
        };
        y1 = Axis {
            start: ((((*mm).h - 1) << 16) - fsy1 * (h - 1)) >> 1,
            step: fsy1,
        };
    }

    if w > (*dst).w || h > (*dst).h {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Trilinear: Tried to scale image to size {} {} when dest_canvas has only dimensions of {} {}! Failing.",
            w,
            h,
            (*dst).w,
            (*dst).h
        );
        return;
    }

    if (srcfmt.BytesPerPixel != 1 && srcfmt.BytesPerPixel != 4)
        || (mmfmt.BytesPerPixel != 1 && mmfmt.BytesPerPixel != 4)
        || dstfmt.BytesPerPixel != 4
    {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Trilinear: Tried to deal with unknown BPP: {} -> {}, mipmap {}",
            srcfmt.BitsPerPixel,
            dstfmt.BitsPerPixel,
            mmfmt.BitsPerPixel
        );
        return;
    }

    // use colorkeys where appropriate
    if srcfmt.Amask != 0 {
        // alpha transparency
        mk0 = srcfmt.Amask;
        ck0 = 0;
    } else if get_color_key(src, &mut ck0) == 0 {
        // colorkey transparency
        mk0 = !srcfmt.Amask;
        ck0 &= mk0;
    }

    if mmfmt.Amask != 0 {
        // alpha transparency
        mk1 = mmfmt.Amask;
        ck1 = 0;
    } else if get_color_key(mm, &mut ck1) == 0 {
        // colorkey transparency
        mk1 = !mmfmt.Amask;
        ck1 &= mk1;
    }

    SDL_LockSurface(src);
    SDL_LockSurface(dst);
    SDL_LockSurface(mm);

    let (mut sy0, mut sy1) = (y0.start, y1.start);
    for y in 0..h {
        let dst_row = (*dst)
            .pixels
            .cast::<u8>()
            .offset((y * (*dst).pitch) as isize)
            .cast::<u32>();
        let py0 = sy0 >> 16;
        let py1 = sy1 >> 16;
        let src_a0 = (*src)
            .pixels
            .cast::<u8>()
            .wrapping_offset(py0 as isize * slen as isize);
        let src_a1 = (*mm)
            .pixels
            .cast::<u8>()
            .wrapping_offset(py1 as isize * mmlen as isize);
        // retrieve the fractional portions of y
        let v0 = i32::from(((sy0 >> 8) & 0xff) as u8);
        let v1 = i32::from(((sy1 >> 8) & 0xff) as u8);

        let (mut sx0, mut sx1) = (x0.start, x1.start);
        for x in 0..w {
            let dst_p = dst_row.add(x as usize);
            let px0 = sx0 >> 16;
            let px1 = sx1 >> 16;
            // retrieve the fractional portions of x
            let u0 = i32::from(((sx0 >> 8) & 0xff) as u8);
            let u1 = i32::from(((sx1 >> 8) & 0xff) as u8);
            // pixels are examined and numbered in pattern
            //  0  1
            //  2  3
            // the ideal pixel (4) is somewhere between these four
            // and is calculated from these using weight vector (w)
            // with a dot product
            let mut p0: [Pixel; 5] = [[0; 4]; 5];
            let mut p1: [Pixel; 5] = [[0; 4]; 5];

            let mut w0: [u8; 4] = [
                btable(255 - u0, 255 - v0),
                btable(u0, 255 - v0),
                btable(255 - u0, v0),
                btable(u0, v0),
            ];
            let mut w1: [u8; 4] = [
                btable(255 - u1, 255 - v1),
                btable(u1, 255 - v1),
                btable(255 - u1, v1),
                btable(u1, v1),
            ];

            // Collect interesting pixels from src image
            // Optimization: speed is criticial on larger images;
            // most pixel reads fall completely inside the image
            if px0 >= 0 && px0 + 1 < (*src).w && py0 >= 0 && py0 + 1 < (*src).h {
                let src_p = src_a0.offset((px0 * sbpp) as isize);

                p0[0] = scale_read_pixel(src_p, srcfmt, srcpal, mk0, ck0);
                p0[1] = scale_read_pixel(src_p.offset(sbpp as isize), srcfmt, srcpal, mk0, ck0);
                p0[2] = scale_read_pixel(src_p.offset(slen as isize), srcfmt, srcpal, mk0, ck0);
                p0[3] = scale_read_pixel(
                    src_p.offset((sbpp + slen) as isize),
                    srcfmt,
                    srcpal,
                    mk0,
                    ck0,
                );
            } else {
                p0[0] = scale_get_pixel(src, mk0, ck0, px0, py0);
                p0[1] = scale_get_pixel(src, mk0, ck0, px0 + 1, py0);
                p0[2] = scale_get_pixel(src, mk0, ck0, px0, py0 + 1);
                p0[3] = scale_get_pixel(src, mk0, ck0, px0 + 1, py0 + 1);
            }

            // Collect interesting pixels from mipmap image. The engine reads
            // these through the source palette, so that is kept.
            if px1 >= 0 && px1 + 1 < (*mm).w && py1 >= 0 && py1 + 1 < (*mm).h {
                let mm_p = src_a1.offset((px1 * mmbpp) as isize);

                p1[0] = scale_read_pixel(mm_p, mmfmt, srcpal, mk1, ck1);
                p1[1] = scale_read_pixel(mm_p.offset(mmbpp as isize), mmfmt, srcpal, mk1, ck1);
                p1[2] = scale_read_pixel(mm_p.offset(mmlen as isize), mmfmt, srcpal, mk1, ck1);
                p1[3] = scale_read_pixel(
                    mm_p.offset((mmbpp + mmlen) as isize),
                    mmfmt,
                    srcpal,
                    mk1,
                    ck1,
                );
            } else {
                p1[0] = scale_get_pixel(mm, mk1, ck1, px1, py1);
                p1[1] = scale_get_pixel(mm, mk1, ck1, px1 + 1, py1);
                p1[2] = scale_get_pixel(mm, mk1, ck1, px1, py1 + 1);
                p1[3] = scale_get_pixel(mm, mk1, ck1, px1 + 1, py1 + 1);
            }

            p0[4][3] = dot_product_8_4(&p0, 3, &w0);
            p1[4][3] = dot_product_8_4(&p1, 3, &w1);
            let mut res_a = blend_ratio_2(p0[4][3], p1[4][3], ratio);

            if i32::from(res_a) <= alpha_threshold {
                *dst_p = transparent;
            } else if !dst_has_alpha {
                // RGB surface handling
                for c in 0..3 {
                    p0[4][c] = dot_product_8_4(&p0, c, &w0);
                }
                for c in 0..3 {
                    p1[4][c] = dot_product_8_4(&p1, c, &w1);
                }
                for c in 0..3 {
                    p0[4][c] = blend_ratio_2(p0[4][c], p1[4][c], ratio);
                }

                // TODO: we should handle alpha-blending here, but we do
                //   not know the destination color for blending!

                *dst_p = at_shift(p0[4][0], dstfmt.Rshift)
                    | at_shift(p0[4][1], dstfmt.Gshift)
                    | at_shift(p0[4][2], dstfmt.Bshift);
            } else {
                // RGBA surface handling

                // we do not want to blend with non-present pixels
                // (pixels that have alpha == 0) as these will
                // skew the result and make resulting alpha useless
                if p0[4][3] != 0 {
                    for i in 0..4 {
                        if p0[i][3] == 0 {
                            w0[i] = 0;
                        }
                    }
                    for c in 0..3 {
                        p0[4][c] = weight_product_8_4(&p0, c, &w0);
                    }
                }
                if p1[4][3] != 0 {
                    for i in 0..4 {
                        if p1[i][3] == 0 {
                            w1[i] = 0;
                        }
                    }
                    for c in 0..3 {
                        p1[4][c] = weight_product_8_4(&p1, c, &w1);
                    }
                }

                if p0[4][3] != 0 && p1[4][3] != 0 {
                    // blend if both present
                    for c in 0..3 {
                        p0[4][c] = blend_ratio_2(p0[4][c], p1[4][c], ratio);
                    }
                } else if p1[4][3] != 0 {
                    // other pixel is present
                    p0[4] = p1[4];
                }

                // error-correct alpha to fully opaque to remove
                // the often unwanted and unnecessary blending
                if res_a > 0xf8 {
                    res_a = 0xff;
                }

                *dst_p = at_shift(p0[4][0], dstfmt.Rshift)
                    | at_shift(p0[4][1], dstfmt.Gshift)
                    | at_shift(p0[4][2], dstfmt.Bshift)
                    | at_shift(res_a, dstfmt.Ashift);
            }
            sx0 += x0.step;
            sx1 += x1.step;
        }
        sy0 += y0.step;
        sy1 += y1.step;
    }

    SDL_UnlockSurface(mm);
    SDL_UnlockSurface(dst);
    SDL_UnlockSurface(src);
}
