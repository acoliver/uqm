//! Scaled extents and the nearest, bilinear and trilinear rescalers,
//! ported from sdl/canvas.c (GPL-2.0-or-later).
use super::Canvas;
use crate::graphics::geometry::{Color, Extent, HotSpot};

/// `GSCALE_IDENTITY` from gfx_common.h: the fixed-point scale of 1.0.
pub const GSCALE_IDENTITY: i32 = 256;

/// `TFB_SCALE_*` scaler kinds from gfx_common.h (`TFB_SCALE_STEP` is 0).
pub const TFB_SCALE_STEP: i32 = 0;
pub const TFB_SCALE_NEAREST: i32 = 1;
pub const TFB_SCALE_BILINEAR: i32 = 2;
pub const TFB_SCALE_TRILINEAR: i32 = 3;

/// `btable[j][i] = (j * i + 0x80) >> 8`.
static BTABLE: [[u8; 256]; 256] = build_btable();

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

fn btable(a: i32, b: i32) -> u8 {
    BTABLE[a as usize][b as usize]
}

/// RGBA channels in `r, g, b, a` order, the layout of the C `pixel_t` union.
type Pixel = [u8; 4];

/// Weighted channel sum via the weight table; the sum wraps like the C `Uint8`.
fn dot_product_8_4(p: &[Pixel], c: usize, v: &[u8; 4]) -> u8 {
    let sum: u32 = (0..4)
        .map(|i| u32::from(btable(i32::from(p[i][c]), i32::from(v[i]))))
        .sum();
    sum as u8
}

fn weight_product_8_4(p: &[Pixel], c: usize, w: &[u8; 4]) -> u8 {
    let numerator: u32 = (0..4).map(|i| u32::from(p[i][c]) * u32::from(w[i])).sum();
    let denominator: u32 = w.iter().map(|&x| u32::from(x)).sum();
    (numerator / denominator) as u8
}

/// Blend two color values according to ratio (0..256).
fn blend_ratio_2(c1: u8, c2: u8, ratio: i32) -> u8 {
    ((((i32::from(c1) - i32::from(c2)) * ratio) >> 8) + i32::from(c2)) as u8
}

/// How transparent source pixels are recognised: `pixel & mask != key`.
#[derive(Clone, Copy)]
struct Transparency {
    mask: u32,
    key: u32,
}

impl Transparency {
    fn of(canvas: &Canvas) -> Self {
        if canvas.format().has_alpha() {
            Self {
                mask: canvas.format().alpha_mask(),
                key: 0,
            }
        } else if let Some(key) = canvas.color_key() {
            Self {
                mask: u32::MAX,
                key,
            }
        } else {
            Self { mask: 0, key: !0 }
        }
    }
}

/// Read one in-bounds pixel; keyed-out pixels read as zero. `palette`
/// overrides how indexes are looked up (the trilinear mipmap reads through
/// the source palette); a 32-bit canvas read through a palette yields the
/// low byte of each word as its index, as the C byte read did.
fn read(canvas: &Canvas, palette: Option<&[Color; 256]>, t: Transparency, x: i32, y: i32) -> Pixel {
    let c = canvas.pixel(x, y);
    if let Some(palette) = palette {
        let index = c & 0xff;
        if index != t.key {
            let color = palette[index as usize];
            return [color.r, color.g, color.b, 0xff];
        }
        return [0; 4];
    }
    if c & t.mask == t.key {
        return [0; 4];
    }
    [
        (c >> 24) as u8,
        (c >> 16) as u8,
        (c >> 8) as u8,
        if canvas.format().has_alpha() {
            c as u8
        } else {
            0xff
        },
    ]
}

/// A pixel that may lie outside the canvas; outside reads as zero and the
/// canvas reads through its own palette.
fn get(canvas: &Canvas, t: Transparency, x: i32, y: i32) -> Pixel {
    if x < 0 || x >= canvas.width() || y < 0 || y >= canvas.height() {
        return [0; 4];
    }
    read(canvas, canvas.palette(), t, x, y)
}

/// The four neighbours of `(px, py)`, through `palette` when all are inside.
fn quad(
    canvas: &Canvas,
    palette: Option<&[Color; 256]>,
    t: Transparency,
    px: i32,
    py: i32,
) -> [Pixel; 5] {
    let mut p = [[0; 4]; 5];
    if px >= 0 && px + 1 < canvas.width() && py >= 0 && py + 1 < canvas.height() {
        p[0] = read(canvas, palette, t, px, py);
        p[1] = read(canvas, palette, t, px + 1, py);
        p[2] = read(canvas, palette, t, px, py + 1);
        p[3] = read(canvas, palette, t, px + 1, py + 1);
    } else {
        p[0] = get(canvas, t, px, py);
        p[1] = get(canvas, t, px + 1, py);
        p[2] = get(canvas, t, px, py + 1);
        p[3] = get(canvas, t, px + 1, py + 1);
    }
    p
}

fn weights(u: i32, v: i32) -> [u8; 4] {
    [
        btable(255 - u, 255 - v),
        btable(u, 255 - v),
        btable(255 - u, v),
        btable(u, v),
    ]
}

fn pack(rgb: [u8; 3], alpha: u8, with_alpha: bool) -> u32 {
    let word = u32::from(rgb[0]) << 24 | u32::from(rgb[1]) << 16 | u32::from(rgb[2]) << 8;
    if with_alpha {
        word | u32::from(alpha)
    } else {
        word
    }
}

/// Size and hot spot of an image scaled by `scale / GSCALE_IDENTITY`.
pub fn scaled_extent(
    src: &Canvas,
    src_hs: HotSpot,
    mipmap: Option<(&Canvas, HotSpot)>,
    scale: i32,
    kind: i32,
) -> (Extent, HotSpot) {
    let (mut x, mut y, mut w, mut h);
    match mipmap {
        None => {
            w = src.width() * scale;
            h = src.height() * scale;
            x = i32::from(src_hs.x) * scale;
            y = i32::from(src_hs.y) * scale;
        }
        Some((mipmap, mm_hs)) => {
            // interpolate extents between src and mipmap to get smoother
            // transition when surface changes
            let ratio = scale * 2 - GSCALE_IDENTITY;
            assert!(scale >= GSCALE_IDENTITY / 2);
            w = mipmap.width() * GSCALE_IDENTITY + (src.width() - mipmap.width()) * ratio;
            h = mipmap.height() * GSCALE_IDENTITY + (src.height() - mipmap.height()) * ratio;
            // the mipmap hot spot is better: some source and mipmap images
            // have the same dimensions
            let (mm_x, mm_y) = (i32::from(mm_hs.x), i32::from(mm_hs.y));
            x = mm_x * GSCALE_IDENTITY + (i32::from(src_hs.x) - mm_x) * ratio;
            y = mm_y * GSCALE_IDENTITY + (i32::from(src_hs.y) - mm_y) * ratio;
        }
    }
    if kind != TFB_SCALE_NEAREST {
        // align hotspot on a whole pixel
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
        // pad the extent to accommodate fractional edge pixels
        w += GSCALE_IDENTITY - 1;
        h += GSCALE_IDENTITY - 1;
    }
    let mut size = Extent::new((w / GSCALE_IDENTITY) as i16, (h / GSCALE_IDENTITY) as i16);
    let hs = HotSpot::new((x / GSCALE_IDENTITY) as i16, (y / GSCALE_IDENTITY) as i16);
    // the scaled image can be one pixel larger than the source because of
    // hotspot alignment and fractional edge pixels
    assert!(i32::from(size.width) <= src.width() + 1 && i32::from(size.height) <= src.height() + 1);
    if size.width == 0 && src.width() != 0 {
        size.width = 1;
    }
    if size.height == 0 && src.height() != 0 {
        size.height = 1;
    }
    (size, hs)
}

/// Nearest-neighbour rescale. With `scale > 0` the size and hot spot are
/// computed and stored in `size` and `dst_hs`; otherwise `dst` is filled.
pub fn rescale_nearest(
    src: &Canvas,
    dst: &mut Canvas,
    scale: i32,
    src_hs: HotSpot,
    size: &mut Extent,
    dst_hs: &mut HotSpot,
) {
    let (w, h) = if scale > 0 {
        (*size, *dst_hs) = scaled_extent(src, src_hs, None, scale, TFB_SCALE_NEAREST);
        (i32::from(size.width), i32::from(size.height))
    } else {
        (dst.width(), dst.height())
    };
    if w > dst.width() || h > dst.height() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Nearest: Tried to scale image to size {w} {h} when dest_canvas has only dimensions of {} {}! Failing.",
            dst.width(),
            dst.height()
        );
        return;
    }
    // start in 0..0.5 to shift the bigger jumps towards the centre
    let (ssx, ssy) = (0x6000, 0x6000);
    let fsx = if w > 1 {
        ((src.width() - 1) << 16) / (w - 1)
    } else {
        0
    };
    let fsy = if h > 1 {
        ((src.height() - 1) << 16) / (h - 1)
    } else {
        0
    };
    if src.is_indexed() != dst.is_indexed() {
        log::warn!(
            "Tried to deal with unknown BPP: {} -> {}",
            src.format().bits_per_pixel(),
            dst.format().bits_per_pixel()
        );
        return;
    }
    let mut row = 0;
    let mut sy = ssy;
    for y in 0..h {
        row += sy >> 16;
        let mut column = 0;
        let mut sx = ssx;
        for x in 0..w {
            column += sx >> 16;
            dst.set_pixel(x, y, src.pixel(column, row));
            sx &= 0xffff;
            sx += fsx;
        }
        sy &= 0xffff;
        sy += fsy;
    }
}

fn dst_key(dst: &Canvas) -> u32 {
    dst.color_key().unwrap_or(0)
}

/// Bilinear rescale.
pub fn rescale_bilinear(
    src: &Canvas,
    dst: &mut Canvas,
    scale: i32,
    src_hs: HotSpot,
    size: &mut Extent,
    dst_hs: &mut HotSpot,
) {
    let dst_has_alpha = dst.format().has_alpha();
    let transparent = dst_key(dst);
    let alpha_threshold = if dst_has_alpha { 0 } else { 127 };
    let (w, h, fsx, fsy, ssx, ssy);
    if scale > 0 {
        (*size, *dst_hs) = scaled_extent(src, src_hs, None, scale, TFB_SCALE_BILINEAR);
        w = i32::from(size.width);
        h = i32::from(size.height);
        fsx = (GSCALE_IDENTITY << 16) / scale;
        fsy = (GSCALE_IDENTITY << 16) / scale;
        // position the hotspots directly over each other
        ssx = (i32::from(src_hs.x) << 16) - fsx * i32::from(dst_hs.x);
        ssy = (i32::from(src_hs.y) << 16) - fsy * i32::from(dst_hs.y);
    } else {
        w = dst.width();
        h = dst.height();
        fsx = (src.width() << 16) / w;
        fsy = (src.height() << 16) / h;
        // give equal importance to both edges
        ssx = (((src.width() - 1) << 16) - fsx * (w - 1)) >> 1;
        ssy = (((src.height() - 1) << 16) - fsy * (h - 1)) >> 1;
    }
    if w > dst.width() || h > dst.height() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Bilinear: Tried to scale image to size {w} {h} when dest_canvas has only dimensions of {} {}! Failing.",
            dst.width(),
            dst.height()
        );
        return;
    }
    if dst.is_indexed() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Bilinear: Tried to deal with unknown BPP: {} -> {}",
            src.format().bits_per_pixel(),
            dst.format().bits_per_pixel()
        );
        return;
    }
    let t = Transparency::of(src);
    let palette = src.palette();
    let mut sy = ssy;
    for y in 0..h {
        let py = sy >> 16;
        let v = (sy >> 8) & 0xff;
        let mut sx = ssx;
        for x in 0..w {
            let px = sx >> 16;
            let u = (sx >> 8) & 0xff;
            let mut weight = weights(u, v);
            let mut p = quad(src, palette, t, px, py);
            p[4][3] = dot_product_8_4(&p, 3, &weight);
            let pixel = if i32::from(p[4][3]) <= alpha_threshold {
                transparent
            } else if !dst_has_alpha {
                let rgb = [0, 1, 2].map(|c| dot_product_8_4(&p, c, &weight));
                pack(rgb, 0, false)
            } else {
                // do not blend with non-present pixels
                for i in 0..4 {
                    if p[i][3] == 0 {
                        weight[i] = 0;
                    }
                }
                let rgb = [0, 1, 2].map(|c| weight_product_8_4(&p, c, &weight));
                // error-correct nearly opaque alpha to fully opaque
                let alpha = if p[4][3] > 0xf8 { 0xff } else { p[4][3] };
                pack(rgb, alpha, true)
            };
            dst.set_pixel(x, y, pixel);
            sx += fsx;
        }
        sy += fsy;
    }
}

/// Fractional start position and step for one axis of one source image.
struct Axis {
    start: i32,
    step: i32,
}

/// Mipmap-blended rescale of `(src, src_hs)` with its mipmap `(mm, mm_hs)`.
pub fn rescale_trilinear(
    (src, src_hs): (&Canvas, HotSpot),
    (mm, mm_hs): (&Canvas, HotSpot),
    dst: &mut Canvas,
    scale: i32,
    size: &mut Extent,
    dst_hs: &mut HotSpot,
) {
    let dst_has_alpha = dst.format().has_alpha();
    let transparent = dst_key(dst);
    let alpha_threshold = if dst_has_alpha { 0 } else { 127 };
    // src v. mipmap importance factor
    let ratio = scale * 2 - GSCALE_IDENTITY;
    if mm.is_indexed() && !src.is_indexed() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Trilinear: Mipmap is paletted, but source is not! Failing."
        );
        return;
    }
    let (w, h, x0, y0, x1, y1);
    if scale > 0 {
        (*size, *dst_hs) =
            scaled_extent(src, src_hs, Some((mm, mm_hs)), scale, TFB_SCALE_TRILINEAR);
        w = i32::from(size.width);
        h = i32::from(size.height);
        let fw = mm.width() * GSCALE_IDENTITY + (src.width() - mm.width()) * ratio;
        let fh = mm.height() * GSCALE_IDENTITY + (src.height() - mm.height()) * ratio;
        // limits the effective source to 2048x2048 and drops 4 bits of the
        // 16-bit fraction
        let fsx0 = ((src.width() << 20) / fw) << 4;
        let fsy0 = ((src.height() << 20) / fh) << 4;
        let fsx1 = ((mm.width() << 20) / fw) << 4;
        let fsy1 = ((mm.height() << 20) / fh) << 4;
        // position the hotspots directly over each other
        let (dst_x, dst_y) = (i32::from(dst_hs.x), i32::from(dst_hs.y));
        x0 = Axis {
            start: (i32::from(src_hs.x) << 16) - fsx0 * dst_x,
            step: fsx0,
        };
        y0 = Axis {
            start: (i32::from(src_hs.y) << 16) - fsy0 * dst_y,
            step: fsy0,
        };
        x1 = Axis {
            start: (i32::from(mm_hs.x) << 16) - fsx1 * dst_x,
            step: fsx1,
        };
        y1 = Axis {
            start: (i32::from(mm_hs.y) << 16) - fsy1 * dst_y,
            step: fsy1,
        };
    } else {
        w = dst.width();
        h = dst.height();
        let axis = |source: i32, target: i32| {
            let step = (source << 16) / target;
            Axis {
                start: (((source - 1) << 16) - step * (target - 1)) >> 1,
                step,
            }
        };
        x0 = axis(src.width(), w);
        y0 = axis(src.height(), h);
        x1 = axis(mm.width(), w);
        y1 = axis(mm.height(), h);
    }
    if w > dst.width() || h > dst.height() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Trilinear: Tried to scale image to size {w} {h} when dest_canvas has only dimensions of {} {}! Failing.",
            dst.width(),
            dst.height()
        );
        return;
    }
    if dst.is_indexed() {
        log::warn!(
            "TFB_DrawCanvas_Rescale_Trilinear: Tried to deal with unknown BPP: {} -> {}, mipmap {}",
            src.format().bits_per_pixel(),
            dst.format().bits_per_pixel(),
            mm.format().bits_per_pixel()
        );
        return;
    }
    let t0 = Transparency::of(src);
    let t1 = Transparency::of(mm);
    let palette = src.palette();
    let (mut sy0, mut sy1) = (y0.start, y1.start);
    for y in 0..h {
        let (py0, py1) = (sy0 >> 16, sy1 >> 16);
        let (v0, v1) = ((sy0 >> 8) & 0xff, (sy1 >> 8) & 0xff);
        let (mut sx0, mut sx1) = (x0.start, x1.start);
        for x in 0..w {
            let (px0, px1) = (sx0 >> 16, sx1 >> 16);
            let (u0, u1) = ((sx0 >> 8) & 0xff, (sx1 >> 8) & 0xff);
            let mut w0 = weights(u0, v0);
            let mut w1 = weights(u1, v1);
            let mut p0 = quad(src, palette, t0, px0, py0);
            // the mipmap is read through the source palette
            let mut p1 = quad(mm, palette, t1, px1, py1);
            p0[4][3] = dot_product_8_4(&p0, 3, &w0);
            p1[4][3] = dot_product_8_4(&p1, 3, &w1);
            let mut res_a = blend_ratio_2(p0[4][3], p1[4][3], ratio);
            let pixel = if i32::from(res_a) <= alpha_threshold {
                transparent
            } else if !dst_has_alpha {
                for c in 0..3 {
                    p0[4][c] = dot_product_8_4(&p0, c, &w0);
                    p1[4][c] = dot_product_8_4(&p1, c, &w1);
                }
                let rgb = [0, 1, 2].map(|c| blend_ratio_2(p0[4][c], p1[4][c], ratio));
                pack(rgb, 0, false)
            } else {
                // do not blend with non-present pixels
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
                    for c in 0..3 {
                        p0[4][c] = blend_ratio_2(p0[4][c], p1[4][c], ratio);
                    }
                } else if p1[4][3] != 0 {
                    p0[4] = p1[4];
                }
                if res_a > 0xf8 {
                    res_a = 0xff;
                }
                pack([p0[4][0], p0[4][1], p0[4][2]], res_a, true)
            };
            dst.set_pixel(x, y, pixel);
            sx0 += x0.step;
            sx1 += x1.step;
        }
        sy0 += y0.step;
        sy1 += y1.step;
    }
}

#[cfg(test)]
mod tests {
    use super::super::PixelFormat;
    use super::*;

    #[test]
    fn btable_rounds_products() {
        assert_eq!(btable(255, 255), 254);
        assert_eq!(btable(128, 2), 1);
        assert_eq!(btable(0, 200), 0);
    }

    #[test]
    fn scaled_extent_aligns_hot_spots_for_filtered_scalers() {
        let src = Canvas::new(PixelFormat::Rgbx, 10, 6);
        let (size, hs) = scaled_extent(&src, HotSpot::new(3, 1), None, 128, TFB_SCALE_BILINEAR);
        assert_eq!((size, hs), (Extent::new(6, 4), HotSpot::new(2, 1)));
        let (size, hs) = scaled_extent(&src, HotSpot::new(3, 1), None, 128, TFB_SCALE_NEAREST);
        assert_eq!((size, hs), (Extent::new(5, 3), HotSpot::new(1, 0)));
    }

    #[test]
    fn nearest_accumulates_whole_steps_per_row() {
        let mut src = Canvas::new(PixelFormat::Rgbx, 3, 1);
        src.words_mut().copy_from_slice(&[1, 2, 3]);
        let mut dst = Canvas::new(PixelFormat::Rgbx, 5, 1);
        let (mut size, mut hs) = (Extent::default(), HotSpot::default());
        rescale_nearest(&src, &mut dst, 0, HotSpot::default(), &mut size, &mut hs);
        assert_eq!(dst.words(), &[1, 1, 2, 2, 3]);
    }

    #[test]
    fn bilinear_keeps_opaque_rgbx_pixels() {
        let mut src = Canvas::new(PixelFormat::Rgbx, 2, 2);
        src.words_mut().fill(0x4080_c000);
        let mut dst = Canvas::new(PixelFormat::Rgbx, 2, 2);
        let (mut size, mut hs) = (Extent::default(), HotSpot::default());
        rescale_bilinear(&src, &mut dst, 0, HotSpot::default(), &mut size, &mut hs);
        assert_eq!(dst.pixel(0, 0), 0x407f_bf00);
    }
}
