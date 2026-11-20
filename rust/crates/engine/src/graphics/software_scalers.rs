//! Integer 2x screen filters, ported from the SDL software scaler family.
//!
//! Packed-channel truncation and the order of fixed-point YUV operations are
//! intentional: changing either changes pixels produced by the original game.

use super::canvas::Canvas;
use std::sync::OnceLock;

mod advanced;
mod biadaptive;
mod hq_rules;

/// The channel shifts of a 32-bit pixel layout.
#[derive(Clone, Copy, Debug)]
struct Format {
    r: u32,
    g: u32,
    b: u32,
}

/// The canvas layout: RGBX8888.
const SCREEN_FORMAT: Format = Format { r: 24, g: 16, b: 8 };

/// A pixel rectangle being filtered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Region {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    Nearest,
    Bilinear,
    Triscan,
    Advanced,
    Biadaptive,
    Hq,
}

struct Image<'a> {
    pixels: &'a [u32],
    stride: usize,
    width: i32,
    height: i32,
}

impl Image<'_> {
    fn pixel(&self, x: i32, y: i32) -> u32 {
        self.pixels[y.clamp(0, self.height - 1) as usize * self.stride
            + x.clamp(0, self.width - 1) as usize]
    }
}

const YUV_MATRIX: [[i32; 3]; 3] = [
    [9794, -2764, 8192],
    [19224, -5428, -6853],
    [3749, 8192, -1339],
];

fn rgb(fmt: &Format, pixel: u32) -> [i32; 3] {
    [fmt.r, fmt.g, fmt.b].map(|shift| ((pixel >> shift) & 255) as i32)
}

fn cmp_yuv(fmt: &Format, a: u32, b: u32, tolerance: i32) -> bool {
    let a = rgb(fmt, a);
    let b = rgb(fmt, b);
    let mut delta = 0;
    for component in [0, 1, 2] {
        let mut sum = 0;
        for channel in 0..3 {
            sum += (YUV_MATRIX[channel][component] * (a[channel] - b[channel])) >> 14;
        }
        delta += sum.abs();
        if delta > tolerance {
            return false;
        }
    }
    true
}

fn blend_11(a: u32, b: u32) -> u32 {
    ((a & 0xfefefefe) >> 1) + ((b & 0xfefefefe) >> 1)
}

fn half(a: u32) -> u32 {
    (a & 0xfefefefe) >> 1
}

fn bilinear(p: [u32; 4]) -> [u32; 4] {
    let mut sum1331 = half(p[1]) + half(p[2]);
    let mut sum3113 = half(p[0]) + half(p[3]);
    let sum1111 = half(sum1331) + half(sum3113) + 0x01010101;
    sum1331 = half(half(sum1331) + half(sum1111) + 0x01010101);
    sum3113 = half(half(sum3113) + half(sum1111) + 0x01010101);
    [
        half(p[0]) + sum1331,
        half(p[1]) + sum3113,
        half(p[2]) + sum3113,
        half(p[3]) + sum1331,
    ]
}

fn triscan(image: &Image<'_>, fmt: &Format, x: i32, y: i32) -> [u32; 4] {
    let c = image.pixel(x, y);
    let n = image.pixel(x, y - 1);
    let s = image.pixel(x, y + 1);
    let w = image.pixel(x - 1, y);
    let e = image.pixel(x + 1, y);
    let close = |a, b| a == b || cmp_yuv(fmt, a, b, 100);
    let mut out = [c; 4];
    if !close(n, s) && !close(w, e) {
        for (i, (a, b)) in [(w, n), (e, n), (w, s), (e, s)].into_iter().enumerate() {
            if close(a, b) {
                out[i] = blend_11(a, b);
            }
        }
    }
    out
}

fn expand_rect(rect: &mut Region, expansion: i32, limits: &Region) {
    if rect.x - expansion >= limits.x {
        rect.w += expansion;
        rect.x -= expansion;
    } else {
        rect.w += rect.x - limits.x;
        rect.x = limits.x;
    }
    if rect.y - expansion >= limits.y {
        rect.h += expansion;
        rect.y -= expansion;
    } else {
        rect.h += rect.y - limits.y;
        rect.y = limits.y;
    }
    if rect.x + rect.w + expansion <= limits.w {
        rect.w += expansion;
    } else {
        rect.w = limits.w - rect.x;
    }
    if rect.y + rect.h + expansion <= limits.h {
        rect.h += expansion;
    } else {
        rect.h = limits.h - rect.y;
    }
}

/// Filter `region` of `image` into `output` (twice the size, `stride`
/// words a row), growing the region by the filter's reach first.
fn scale(
    image: &Image<'_>,
    fmt: &Format,
    output: &mut [u32],
    stride: usize,
    region: &mut Region,
    filter: Filter,
) {
    if filter != Filter::Nearest {
        expand_rect(
            region,
            if matches!(filter, Filter::Advanced | Filter::Biadaptive) {
                2
            } else {
                1
            },
            &Region {
                x: 0,
                y: 0,
                w: image.width,
                h: image.height,
            },
        );
    }
    for y in region.y..region.y + region.h {
        for x in region.x..region.x + region.w {
            let p = match filter {
                Filter::Nearest => [image.pixel(x, y); 4],
                Filter::Bilinear => bilinear([
                    image.pixel(x, y),
                    image.pixel(x + 1, y),
                    image.pixel(x, y + 1),
                    image.pixel(x + 1, y + 1),
                ]),
                Filter::Triscan => triscan(image, fmt, x, y),
                Filter::Advanced => advanced::advanced(image, fmt, x, y),
                Filter::Biadaptive => biadaptive::biadaptive(image, x, y),
                Filter::Hq => hq(image, fmt, x, y),
            };
            let offset = y as usize * 2 * stride + x as usize * 2;
            output[offset..offset + 2].copy_from_slice(&p[..2]);
            output[offset + stride..offset + stride + 2].copy_from_slice(&p[2..]);
        }
    }
}

#[cfg(test)]
mod tests;

fn blend_1111(a: u32, b: u32, c: u32, d: u32) -> u32 {
    ((a & 0xfcfcfcfc) >> 2)
        + ((b & 0xfcfcfcfc) >> 2)
        + ((c & 0xfcfcfcfc) >> 2)
        + ((d & 0xfcfcfcfc) >> 2)
}

fn blend_31(a: u32, b: u32) -> u32 {
    half(a) + ((a & 0xfcfcfcfc) >> 2) + ((b & 0xfcfcfcfc) >> 2)
}

fn blend_211(a: u32, b: u32, c: u32) -> u32 {
    half(a) + ((b & 0xfcfcfcfc) >> 2) + ((c & 0xfcfcfcfc) >> 2)
}

fn blend_521(a: u32, b: u32, c: u32) -> u32 {
    half(a)
        + ((a & 0xf8f8f8f8) >> 3)
        + ((b & 0xfcfcfcfc) >> 2)
        + ((c & 0xf8f8f8f8) >> 3)
        + 0x02020202
}

fn blend_611(a: u32, b: u32, c: u32) -> u32 {
    half(a)
        + ((a & 0xfcfcfcfc) >> 2)
        + ((b & 0xf8f8f8f8) >> 3)
        + ((c & 0xf8f8f8f8) >> 3)
        + 0x02020202
}

fn blend_233(a: u32, b: u32, c: u32) -> u32 {
    ((a & 0xfcfcfcfc) >> 2)
        + ((b & 0xfcfcfcfc) >> 2)
        + ((b & 0xf8f8f8f8) >> 3)
        + ((c & 0xfcfcfcfc) >> 2)
        + ((c & 0xf8f8f8f8) >> 3)
        + 0x02020202
}

fn blend_e11(a: u32, b: u32, c: u32) -> u32 {
    half(a)
        + ((a & 0xfcfcfcfc) >> 2)
        + ((a & 0xf8f8f8f8) >> 3)
        + ((b & 0xf0f0f0f0) >> 4)
        + ((c & 0xf0f0f0f0) >> 4)
        + 0x03030303
}

fn rgb_delta(fmt: &Format, a: u32, b: u32) -> i32 {
    let a = rgb(fmt, a);
    let b = rgb(fmt, b);
    (0..3).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

fn intensity(fmt: &Format, p: u32) -> i32 {
    rgb(fmt, p)
        .into_iter()
        .enumerate()
        .map(|(i, c)| (YUV_MATRIX[i][0] * c) >> 14)
        .sum()
}

fn yuv_table() -> &'static [u32] {
    static TABLE: OnceLock<Box<[u32]>> = OnceLock::new();
    TABLE.get_or_init(|| {
        (0..0x8000)
            .map(|i| {
                let channels = [(i >> 10) & 31, (i >> 5) & 31, i & 31].map(|c| (c << 3) | (c >> 3));
                let component =
                    |j| (0..3).map(|k| channels[k] * YUV_MATRIX[k][j]).sum::<i32>() >> 15;
                ((component(0) << 16) | ((64 + component(1)) << 8) | (64 + component(2))) as u32
            })
            .collect()
    })
}

fn to_yuv(fmt: &Format, p: u32) -> u32 {
    let index = (((p >> (fmt.r + 3)) & 31) << 10)
        | (((p >> (fmt.g + 3)) & 31) << 5)
        | ((p >> (fmt.b + 3)) & 31);
    yuv_table()[index as usize]
}

fn diff_yuv(a: u32, b: u32) -> bool {
    let delta = |mask| ((a & mask) as i32 - (b & mask) as i32).abs();
    let ret = ((0x40 << 16) - delta(0xff0000))
        | ((0x12 << 8) - delta(0x00ff00))
        | (0x0c - delta(0x0000ff));
    ret < 0
}

fn hq(image: &Image<'_>, fmt: &Format, x: i32, y: i32) -> [u32; 4] {
    let mut pix = [0; 10];
    for dy in -1..=1 {
        for dx in -1..=1 {
            pix[((dy + 1) * 3 + dx + 2) as usize] = image.pixel(x + dx, y + dy);
        }
    }
    let yuv = pix.map(|p| to_yuv(fmt, p));
    let mut pattern = 0u8;
    for (bit, i) in [1, 2, 3, 4, 6, 7, 8, 9].into_iter().enumerate() {
        if diff_yuv(yuv[5], yuv[i]) {
            pattern |= 1 << bit;
        }
    }
    let edges = [(4, 2), (2, 6), (8, 4), (6, 8)].map(|(a, b)| diff_yuv(yuv[a], yuv[b]));
    hq_rules::rules(pattern, edges, &pix)
}

/// The 2x filter a set of `TFB_GFXFLAGS_SCALE_*` flags selects; the
/// first flag in this order wins, nearest neighbour otherwise.
fn filter_for(flags: i32) -> Filter {
    [
        (1 << 3, Filter::Bilinear),
        (1 << 4, Filter::Biadaptive),
        (1 << 5, Filter::Advanced),
        (1 << 6, Filter::Triscan),
        (1 << 7, Filter::Hq),
    ]
    .into_iter()
    .find(|(flag, _)| flags & flag == *flag)
    .map_or(Filter::Nearest, |(_, filter)| filter)
}

/// Render a whole screen canvas through the 2x filter `flags` select into
/// the renderer's RGBX byte buffer (four bytes a pixel at twice the size).
pub(crate) fn scale_screen(src: &Canvas, flags: i32, buffer: &mut [u8]) {
    let image = Image {
        pixels: src.words(),
        stride: src.width() as usize,
        width: src.width(),
        height: src.height(),
    };
    let stride = image.stride * 2;
    let mut pixels = vec![0u32; stride * image.height as usize * 2];
    assert_eq!(buffer.len(), pixels.len() * 4);
    let mut region = Region {
        x: 0,
        y: 0,
        w: image.width,
        h: image.height,
    };
    scale(
        &image,
        &SCREEN_FORMAT,
        &mut pixels,
        stride,
        &mut region,
        filter_for(flags),
    );
    for (pixel, bytes) in pixels.into_iter().zip(buffer.as_chunks_mut::<4>().0) {
        *bytes = pixel.to_ne_bytes();
    }
}
