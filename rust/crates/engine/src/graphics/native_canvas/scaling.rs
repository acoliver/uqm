//! Scaled extents, nearest and bilinear rescale, and the shared fixed-point
//! pixel helpers also used by the trilinear scaler.
use super::surface::{format, get_color_key};
use super::{GSCALE_IDENTITY, TFB_SCALE_BILINEAR, TFB_SCALE_NEAREST};
use crate::graphics::native_abi::{TFB_Canvas, EXTENT, HOT_SPOT};
use sdl2_sys::{SDL_Color, SDL_LockSurface, SDL_PixelFormat, SDL_Surface, SDL_UnlockSurface};
use std::ptr;

/// BYTE x BYTE weight (mult >> 8) table; `BTABLE[j][i] = (j * i + 0x80) >> 8`.
pub(super) static BTABLE: [[u8; 256]; 256] = build_btable();

const fn build_btable() -> [[u8; 256]; 256] {
    let mut table = [[0u8; 256]; 256];
    let mut j = 0;
    while j < 256 {
        let mut i = 0;
        while i < 256 {
            table[j][i] = ((j * i + 0x80) >> 8) as u8;
            i += 1;
        }
        j += 1;
    }
    table
}

/// `btable[a][b]` with channel-sized operands.
pub(super) fn btable(a: i32, b: i32) -> u8 {
    BTABLE[a as usize][b as usize]
}

/// RGBA channels in `r, g, b, a` order, the layout of the C `pixel_t` union.
pub(super) type Pixel = [u8; 4];

/// Weighted channel sum via the weight table; the sum wraps like the C `Uint8`.
pub(super) fn dot_product_8_4(p: &[Pixel], c: usize, v: &[u8; 4]) -> u8 {
    let sum: u32 = (0..4)
        .map(|i| u32::from(btable(i32::from(p[i][c]), i32::from(v[i]))))
        .sum();
    sum as u8
}

pub(super) fn weight_product_8_4(p: &[Pixel], c: usize, w: &[u8; 4]) -> u8 {
    let numerator: u32 = (0..4).map(|i| u32::from(p[i][c]) * u32::from(w[i])).sum();
    let denominator: u32 = w.iter().map(|&x| u32::from(x)).sum();
    (numerator / denominator) as u8
}

/// Blend two color values according to ratio (0..256); identical to proper
/// alpha blending.
pub(super) fn blend_ratio_2(c1: u8, c2: u8, ratio: i32) -> u8 {
    ((((i32::from(c1) - i32::from(c2)) * ratio) >> 8) + i32::from(c2)) as u8
}

/// Reads one source pixel; non-present (keyed out) pixels read as zero.
///
/// # Safety
/// `ppix` must point at a whole pixel of the surface described by `fmt`/`pal`.
pub(super) unsafe fn scale_read_pixel(
    ppix: *const u8,
    fmt: &SDL_PixelFormat,
    pal: *const SDL_Color,
    mask: u32,
    key: u32,
) -> Pixel {
    // start off with non-present pixel
    let mut p: Pixel = [0; 4];

    if !pal.is_null() {
        // paletted pixel; mask not used
        let c = u32::from(*ppix);
        if c != key {
            let color = *pal.add(c as usize);
            p = [color.r, color.g, color.b, 0xff];
        }
    } else {
        // RGB(A) pixel; (pix & mask) != key
        let c = ptr::read_unaligned(ppix.cast::<u32>());
        if (c & mask) != key {
            // Assume 8 bits/channel; a safe assumption with 32bpp surfaces
            p[0] = (c >> fmt.Rshift) as u8;
            p[1] = (c >> fmt.Gshift) as u8;
            p[2] = (c >> fmt.Bshift) as u8;
            p[3] = if fmt.Amask != 0 {
                (c >> fmt.Ashift) as u8
            } else {
                0xff
            };
        }
    }
    p
}

/// # Safety
/// `src` must be a live surface.
pub(super) unsafe fn scale_get_pixel(
    src: *mut SDL_Surface,
    mask: u32,
    key: u32,
    x: i32,
    y: i32,
) -> Pixel {
    let fmt = format(src);
    let pal = if fmt.palette.is_null() {
        ptr::null()
    } else {
        (*fmt.palette).colors as *const SDL_Color
    };

    if x < 0 || x >= (*src).w || y < 0 || y >= (*src).h {
        return [0; 4];
    }

    scale_read_pixel(
        (*src)
            .pixels
            .cast::<u8>()
            .offset((y * (*src).pitch + x * i32::from(fmt.BytesPerPixel)) as isize),
        fmt,
        pal,
        mask,
        key,
    )
}

/// Packs a channel at its shift; widened first so alpha at bit 24 is exact.
pub(super) fn at_shift(channel: u8, shift: u8) -> u32 {
    u32::from(channel) << shift
}

/// # Safety
/// Source canvas must be live, the mipmap null or live, and the extent and
/// hotspot pointers writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetScaledExtent(
    src_canvas: TFB_Canvas,
    src_hs: *mut HOT_SPOT,
    src_mipmap: TFB_Canvas,
    mm_hs: *mut HOT_SPOT,
    scale: i32,
    type_: i32,
    size: *mut EXTENT,
    hs: *mut HOT_SPOT,
) {
    let src: *mut SDL_Surface = src_canvas.cast();
    let (mut x, mut y, mut w, mut h): (i32, i32, i32, i32);

    if src_mipmap.is_null() {
        w = (*src).w * scale;
        h = (*src).h * scale;
        x = i32::from((*src_hs).x) * scale;
        y = i32::from((*src_hs).y) * scale;
    } else {
        // interpolates extents between src and mipmap to get smoother
        // transition when surface changes
        let mipmap: *mut SDL_Surface = src_mipmap.cast();
        let ratio = scale * 2 - GSCALE_IDENTITY;

        assert!(scale >= GSCALE_IDENTITY / 2);

        w = (*mipmap).w * GSCALE_IDENTITY + ((*src).w - (*mipmap).w) * ratio;
        h = (*mipmap).h * GSCALE_IDENTITY + ((*src).h - (*mipmap).h) * ratio;

        // Seems it is better to use mipmap hotspot because some
        // source and mipmap images have the same dimensions!
        let (mm_x, mm_y) = (i32::from((*mm_hs).x), i32::from((*mm_hs).y));
        x = mm_x * GSCALE_IDENTITY + (i32::from((*src_hs).x) - mm_x) * ratio;
        y = mm_y * GSCALE_IDENTITY + (i32::from((*src_hs).y) - mm_y) * ratio;
    }

    if type_ != TFB_SCALE_NEAREST {
        // align hotspot on an whole pixel
        if x & (GSCALE_IDENTITY - 1) != 0 {
            let frac = GSCALE_IDENTITY - (x & (GSCALE_IDENTITY - 1));
            x += frac;
            w += frac;
        }
        if y & (GSCALE_IDENTITY - 1) != 0 {
            let frac = GSCALE_IDENTITY - (y & (GSCALE_IDENTITY - 1));
            y += frac;
            h += frac;
        }
        // pad the extent to accomodate fractional edge pixels
        w += GSCALE_IDENTITY - 1;
        h += GSCALE_IDENTITY - 1;
    }

    (*size).width = (w / GSCALE_IDENTITY) as i16;
    (*size).height = (h / GSCALE_IDENTITY) as i16;
    (*hs).x = (x / GSCALE_IDENTITY) as i16;
    (*hs).y = (y / GSCALE_IDENTITY) as i16;

    // Scaled image can be larger than the source by 1 pixel
    // because of hotspot alignment and fractional edge pixels
    assert!(i32::from((*size).width) <= (*src).w + 1 && i32::from((*size).height) <= (*src).h + 1);

    if (*size).width == 0 && (*src).w != 0 {
        (*size).width = 1;
    }
    if (*size).height == 0 && (*src).h != 0 {
        (*size).height = 1;
    }
}

/// # Safety
/// Canvas must be a live SDL surface and size writable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetExtent(canvas: TFB_Canvas, size: *mut EXTENT) {
    let src: *mut SDL_Surface = canvas.cast();
    (*size).width = (*src).w as i16;
    (*size).height = (*src).h as i16;
}

/// # Safety
/// Canvases must be live; `src_hs`, `size` and `dst_hs` are only read or
/// written when `scale > 0`.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Rescale_Nearest(
    src_canvas: TFB_Canvas,
    dst_canvas: TFB_Canvas,
    scale: i32,
    src_hs: *mut HOT_SPOT,
    size: *mut EXTENT,
    dst_hs: *mut HOT_SPOT,
) {
    let src: *mut SDL_Surface = src_canvas.cast();
    let dst: *mut SDL_Surface = dst_canvas.cast();
    let (mut fsx, mut fsy) = (0i32, 0i32); // source fractional dx and dy increments
                                           // We start with a value in 0..0.5 range to shift the bigger
                                           // jumps towards the center of the image
    let (ssx, ssy) = (0x6000i32, 0x6000i32); // source fractional x and y starting points
    let (w, h): (i32, i32);

    if scale > 0 {
        TFB_DrawCanvas_GetScaledExtent(
            src_canvas,
            src_hs,
            ptr::null_mut(),
            ptr::null_mut(),
            scale,
            TFB_SCALE_NEAREST,
            size,
            dst_hs,
        );

        w = i32::from((*size).width);
        h = i32::from((*size).height);
    } else {
        // Just go with the dst surface dimensions
        w = (*dst).w;
        h = (*dst).h;
    }

    if w > (*dst).w || h > (*dst).h {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Nearest: Tried to scale image to size {} {} when dest_canvas has only dimensions of {} {}! Failing.",
            w,
            h,
            (*dst).w,
            (*dst).h
        );
        return;
    }

    if w > 1 {
        fsx = (((*src).w - 1) << 16) / (w - 1);
    }
    if h > 1 {
        fsy = (((*src).h - 1) << 16) / (h - 1);
    }

    SDL_LockSurface(src);
    SDL_LockSurface(dst);

    let sbpp = format(src).BytesPerPixel;
    let dbpp = format(dst).BytesPerPixel;
    if sbpp == 1 && dbpp == 1 {
        let mut csp = (*src).pixels.cast::<u8>();
        let mut cdp = (*dst).pixels.cast::<u8>();
        let mut sy = ssy;

        for _ in 0..h {
            csp = csp.wrapping_offset((sy >> 16) as isize * (*src).pitch as isize);
            let mut sp = csp;
            let mut dp = cdp;
            let mut sx = ssx;
            for _ in 0..w {
                sp = sp.wrapping_offset((sx >> 16) as isize);
                *dp = *sp;
                sx &= 0xffff;
                sx += fsx;
                dp = dp.wrapping_add(1);
            }
            sy &= 0xffff;
            sy += fsy;
            cdp = cdp.wrapping_offset((*dst).pitch as isize);
        }
    } else if sbpp == 4 && dbpp == 4 {
        let sgap = ((*src).pitch >> 2) as isize;
        let dgap = ((*dst).pitch >> 2) as isize;
        let mut csp = (*src).pixels.cast::<u32>();
        let mut cdp = (*dst).pixels.cast::<u32>();
        let mut sy = ssy;

        for _ in 0..h {
            csp = csp.wrapping_offset((sy >> 16) as isize * sgap);
            let mut sp = csp;
            let mut dp = cdp;
            let mut sx = ssx;
            for _ in 0..w {
                sp = sp.wrapping_offset((sx >> 16) as isize);
                *dp = *sp;
                sx &= 0xffff;
                sx += fsx;
                dp = dp.wrapping_add(1);
            }
            sy &= 0xffff;
            sy += fsy;
            cdp = cdp.wrapping_offset(dgap);
        }
    } else {
        log::warn!(
            "Tried to deal with unknown BPP: {} -> {}",
            format(src).BitsPerPixel,
            format(dst).BitsPerPixel
        );
    }
    SDL_UnlockSurface(dst);
    SDL_UnlockSurface(src);
}

/// # Safety
/// Canvases must be live; hotspot and extent pointers are only used when
/// `scale > 0`.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Rescale_Bilinear(
    src_canvas: TFB_Canvas,
    dst_canvas: TFB_Canvas,
    scale: i32,
    src_hs: *mut HOT_SPOT,
    size: *mut EXTENT,
    dst_hs: *mut HOT_SPOT,
) {
    let src: *mut SDL_Surface = src_canvas.cast();
    let dst: *mut SDL_Surface = dst_canvas.cast();
    let srcfmt = format(src);
    let dstfmt = format(dst);
    let srcpal = if srcfmt.palette.is_null() {
        ptr::null()
    } else {
        (*srcfmt.palette).colors as *const SDL_Color
    };
    let sbpp = i32::from(srcfmt.BytesPerPixel);
    let slen = (*src).pitch;
    let dst_has_alpha = dstfmt.Amask != 0;
    let mut srckey = 0u32;
    let mut transparent = 0u32;
    let alpha_threshold: i32 = if dst_has_alpha { 0 } else { 127 };
    // source masks and keys
    let (mut mk, mut ck) = (0u32, !0u32);
    let (fsx, fsy): (i32, i32);
    let (ssx, ssy): (i32, i32);
    let (w, h): (i32, i32);

    // Get destination transparent color if it exists
    get_color_key(dst, &mut transparent);

    if scale > 0 {
        // Use (scale / GSCALE_IDENTITY) sizing factor
        TFB_DrawCanvas_GetScaledExtent(
            src_canvas,
            src_hs,
            ptr::null_mut(),
            ptr::null_mut(),
            scale,
            TFB_SCALE_BILINEAR,
            size,
            dst_hs,
        );

        w = i32::from((*size).width);
        h = i32::from((*size).height);
        fsx = (GSCALE_IDENTITY << 16) / scale;
        fsy = (GSCALE_IDENTITY << 16) / scale;

        // position the hotspots directly over each other
        ssx = (i32::from((*src_hs).x) << 16) - fsx * i32::from((*dst_hs).x);
        ssy = (i32::from((*src_hs).y) << 16) - fsy * i32::from((*dst_hs).y);
    } else {
        // Just go with the dst surface dimensions
        w = (*dst).w;
        h = (*dst).h;
        fsx = ((*src).w << 16) / w;
        fsy = ((*src).h << 16) / h;

        // give equal importance to both edges
        ssx = ((((*src).w - 1) << 16) - fsx * (w - 1)) >> 1;
        ssy = ((((*src).h - 1) << 16) - fsy * (h - 1)) >> 1;
    }

    if w > (*dst).w || h > (*dst).h {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Bilinear: Tried to scale image to size {} {} when dest_canvas has only dimensions of {} {}! Failing.",
            w,
            h,
            (*dst).w,
            (*dst).h
        );
        return;
    }

    if (srcfmt.BytesPerPixel != 1 && srcfmt.BytesPerPixel != 4) || dstfmt.BytesPerPixel != 4 {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Bilinear: Tried to deal with unknown BPP: {} -> {}",
            srcfmt.BitsPerPixel,
            dstfmt.BitsPerPixel
        );
        return;
    }

    // use colorkeys where appropriate
    if srcfmt.Amask != 0 {
        // alpha transparency
        mk = srcfmt.Amask;
        ck = 0;
    } else if get_color_key(src, &mut srckey) == 0 {
        // colorkey transparency
        mk = !srcfmt.Amask;
        ck = srckey & mk;
    }

    SDL_LockSurface(src);
    SDL_LockSurface(dst);

    let mut sy = ssy;
    for y in 0..h {
        let dst_row = (*dst)
            .pixels
            .cast::<u8>()
            .offset((y * (*dst).pitch) as isize)
            .cast::<u32>();
        let py = sy >> 16;
        let src_a = (*src)
            .pixels
            .cast::<u8>()
            .wrapping_offset(py as isize * slen as isize);
        // retrieve the fractional portions of y
        let v = ((sy >> 8) & 0xff) as u8;

        let mut sx = ssx;
        for x in 0..w {
            let dst_p = dst_row.add(x as usize);
            let px = sx >> 16;
            // retrieve the fractional portions of x
            let u = ((sx >> 8) & 0xff) as u8;
            // pixels are examined and numbered in pattern
            //  0  1
            //  2  3
            // the ideal pixel (4) is somewhere between these four
            // and is calculated from these using weight vector (weight)
            // with a dot product
            let mut p: [Pixel; 5] = [[0; 4]; 5];
            let (iu, iv) = (i32::from(u), i32::from(v));
            let mut weight: [u8; 4] = [
                btable(255 - iu, 255 - iv),
                btable(iu, 255 - iv),
                btable(255 - iu, iv),
                btable(iu, iv),
            ];

            // Collect interesting pixels from src image
            // Optimization: speed is criticial on larger images;
            // most pixel reads fall completely inside the image
            if px >= 0 && px + 1 < (*src).w && py >= 0 && py + 1 < (*src).h {
                let src_p = src_a.offset((px * sbpp) as isize);

                p[0] = scale_read_pixel(src_p, srcfmt, srcpal, mk, ck);
                p[1] = scale_read_pixel(src_p.offset(sbpp as isize), srcfmt, srcpal, mk, ck);
                p[2] = scale_read_pixel(src_p.offset(slen as isize), srcfmt, srcpal, mk, ck);
                p[3] =
                    scale_read_pixel(src_p.offset((sbpp + slen) as isize), srcfmt, srcpal, mk, ck);
            } else {
                p[0] = scale_get_pixel(src, mk, ck, px, py);
                p[1] = scale_get_pixel(src, mk, ck, px + 1, py);
                p[2] = scale_get_pixel(src, mk, ck, px, py + 1);
                p[3] = scale_get_pixel(src, mk, ck, px + 1, py + 1);
            }

            p[4][3] = dot_product_8_4(&p, 3, &weight);

            if i32::from(p[4][3]) <= alpha_threshold {
                *dst_p = transparent;
            } else if !dst_has_alpha {
                // RGB surface handling
                p[4][0] = dot_product_8_4(&p, 0, &weight);
                p[4][1] = dot_product_8_4(&p, 1, &weight);
                p[4][2] = dot_product_8_4(&p, 2, &weight);

                // TODO: we should handle alpha-blending here, but we do
                //   not know the destination color for blending!

                *dst_p = at_shift(p[4][0], dstfmt.Rshift)
                    | at_shift(p[4][1], dstfmt.Gshift)
                    | at_shift(p[4][2], dstfmt.Bshift);
            } else {
                // RGBA surface handling

                // we do not want to blend with non-present pixels
                // (pixels that have alpha == 0) as these will
                // skew the result and make resulting alpha useless
                for i in 0..4 {
                    if p[i][3] == 0 {
                        weight[i] = 0;
                    }
                }

                p[4][0] = weight_product_8_4(&p, 0, &weight);
                p[4][1] = weight_product_8_4(&p, 1, &weight);
                p[4][2] = weight_product_8_4(&p, 2, &weight);

                // error-correct alpha to fully opaque to remove
                // the often unwanted and unnecessary blending
                if p[4][3] > 0xf8 {
                    p[4][3] = 0xff;
                }

                *dst_p = at_shift(p[4][0], dstfmt.Rshift)
                    | at_shift(p[4][1], dstfmt.Gshift)
                    | at_shift(p[4][2], dstfmt.Bshift)
                    | at_shift(p[4][3], dstfmt.Ashift);
            }
            sx += fsx;
        }
        sy += fsy;
    }

    SDL_UnlockSurface(dst);
    SDL_UnlockSurface(src);
}
