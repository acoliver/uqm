//! Integer 2x screen filters, ported from the SDL software scaler family.
//!
//! Packed-channel truncation and the order of fixed-point YUV operations are
//! intentional: changing either changes pixels produced by the original game.

use sdl2_sys::{SDL_PixelFormat, SDL_Rect, SDL_Surface};
use std::sync::OnceLock;

mod advanced;
mod biadaptive;
mod hq_rules;

#[derive(Clone, Copy)]
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

fn rgb(fmt: &SDL_PixelFormat, pixel: u32) -> [i32; 3] {
    [fmt.Rshift, fmt.Gshift, fmt.Bshift].map(|shift| ((pixel >> shift) & 255) as i32)
}

fn cmp_yuv(fmt: &SDL_PixelFormat, a: u32, b: u32, tolerance: i32) -> bool {
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

fn triscan(image: &Image<'_>, fmt: &SDL_PixelFormat, x: i32, y: i32) -> [u32; 4] {
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

fn expand_rect(rect: &mut SDL_Rect, expansion: i32, limits: &SDL_Rect) {
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

// The SDL caller owns and locks both surfaces. They must be disjoint 32-bit
// buffers in the same format, with a destination twice the source dimensions.
unsafe fn scale(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
    filter: Filter,
) {
    let src = &*src;
    let dst = &mut *dst;
    let region = &mut *region;
    let fmt = &*dst.format;
    assert_eq!(fmt.BytesPerPixel, 4);
    assert_eq!((*src.format).BytesPerPixel, 4);
    assert!(dst.w >= src.w * 2 && dst.h >= src.h * 2);
    let image = Image {
        pixels: std::slice::from_raw_parts(src.pixels.cast(), (src.pitch * src.h / 4) as usize),
        stride: (src.pitch / 4) as usize,
        width: src.w,
        height: src.h,
    };
    let stride = (dst.pitch / 4) as usize;
    let output = std::slice::from_raw_parts_mut(dst.pixels.cast::<u32>(), stride * dst.h as usize);
    if !matches!(filter, Filter::Nearest) {
        expand_rect(
            region,
            if matches!(filter, Filter::Advanced | Filter::Biadaptive) {
                2
            } else {
                1
            },
            &SDL_Rect {
                x: 0,
                y: 0,
                w: src.w,
                h: src.h,
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
                Filter::Triscan => triscan(&image, fmt, x, y),
                Filter::Advanced => advanced::advanced(&image, fmt, x, y),
                Filter::Biadaptive => biadaptive::biadaptive(&image, x, y),
                Filter::Hq => hq(&image, fmt, x, y),
            };
            let offset = y as usize * 2 * stride + x as usize * 2;
            output[offset..offset + 2].copy_from_slice(&p[..2]);
            output[offset + stride..offset + stride + 2].copy_from_slice(&p[2..]);
        }
    }
}

/// # Safety
/// Surfaces must be disjoint, locked 32-bit SDL surfaces in the same format;
/// `dst` must have twice the source dimensions and `region` must be within `src`.
#[no_mangle]
pub unsafe extern "C" fn Scale_Nearest(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
) {
    scale(src, dst, region, Filter::Nearest);
}

/// # Safety
/// Same surface and region requirements as `Scale_Nearest`.
#[no_mangle]
pub unsafe extern "C" fn Scale_BilinearFilter(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
) {
    scale(src, dst, region, Filter::Bilinear);
}

/// # Safety
/// Same surface and region requirements as `Scale_Nearest`.
#[no_mangle]
pub unsafe extern "C" fn Scale_TriScanFilter(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
) {
    scale(src, dst, region, Filter::Triscan);
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

fn rgb_delta(fmt: &SDL_PixelFormat, a: u32, b: u32) -> i32 {
    let a = rgb(fmt, a);
    let b = rgb(fmt, b);
    (0..3).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

fn intensity(fmt: &SDL_PixelFormat, p: u32) -> i32 {
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

fn to_yuv(fmt: &SDL_PixelFormat, p: u32) -> u32 {
    let index = (((p >> (fmt.Rshift + 3)) & 31) << 10)
        | (((p >> (fmt.Gshift + 3)) & 31) << 5)
        | ((p >> (fmt.Bshift + 3)) & 31);
    yuv_table()[index as usize]
}

fn diff_yuv(a: u32, b: u32) -> bool {
    let delta = |mask| ((a & mask) as i32 - (b & mask) as i32).abs();
    let ret = ((0x40 << 16) - delta(0xff0000))
        | ((0x12 << 8) - delta(0x00ff00))
        | (0x0c - delta(0x0000ff));
    ret < 0
}

fn hq(image: &Image<'_>, fmt: &SDL_PixelFormat, x: i32, y: i32) -> [u32; 4] {
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

/// # Safety
/// Same surface and region requirements as `Scale_Nearest`.
#[no_mangle]
pub unsafe extern "C" fn Scale_BiAdaptFilter(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
) {
    scale(src, dst, region, Filter::Biadaptive);
}

/// # Safety
/// Same surface and region requirements as `Scale_Nearest`.
#[no_mangle]
pub unsafe extern "C" fn Scale_BiAdaptAdvFilter(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
) {
    scale(src, dst, region, Filter::Advanced);
}

/// # Safety
/// Same surface and region requirements as `Scale_Nearest`.
#[no_mangle]
pub unsafe extern "C" fn Scale_HqFilter(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    region: *mut SDL_Rect,
) {
    scale(src, dst, region, Filter::Hq);
}

pub type ScaleFunc = unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_Surface, *mut SDL_Rect);

#[repr(C)]
pub struct ScaleFuncDef {
    pub flag: i32,
    pub func: ScaleFunc,
}

#[no_mangle]
pub static Scale_C_Functions: [ScaleFuncDef; 6] = [
    ScaleFuncDef {
        flag: 1 << 3,
        func: Scale_BilinearFilter,
    },
    ScaleFuncDef {
        flag: 1 << 4,
        func: Scale_BiAdaptFilter,
    },
    ScaleFuncDef {
        flag: 1 << 5,
        func: Scale_BiAdaptAdvFilter,
    },
    ScaleFuncDef {
        flag: 1 << 6,
        func: Scale_TriScanFilter,
    },
    ScaleFuncDef {
        flag: 1 << 7,
        func: Scale_HqFilter,
    },
    ScaleFuncDef {
        flag: 0,
        func: Scale_Nearest,
    },
];

// The command-line parser still writes this setting. As with the former
// USE_RUST_GFX build, screen filters use the scalar integer implementation.
#[no_mangle]
pub static mut force_platform: libc::c_int = 0;

#[no_mangle]
pub extern "C" fn Scale_Init() {
    let _ = yuv_table();
}

#[no_mangle]
pub extern "C" fn Scale_PrepPlatform(
    flags: libc::c_int,
    _fmt: *const SDL_PixelFormat,
) -> ScaleFunc {
    Scale_C_Functions
        .iter()
        .find(|entry| flags & entry.flag == entry.flag)
        .expect("the scaler table ends with its default entry")
        .func
}

/// Render a complete SDL screen into the renderer's byte-oriented RGBX buffer.
///
/// # Safety
/// `src` must point to a live 32-bit SDL screen surface. The output buffer
/// must contain exactly four bytes per pixel at twice the source dimensions.
/// Source pixels must not alias `buffer` and must remain stable during the call.
pub(crate) unsafe fn scale_surface_to_rgbx(src: *mut SDL_Surface, flags: i32, buffer: &mut [u8]) {
    let source = &*src;
    let width = source.w * 2;
    let height = source.h * 2;
    let mut pixels = vec![0u32; (width * height) as usize];
    assert_eq!(buffer.len(), pixels.len() * 4);
    // This descriptor borrows aligned pixel storage and the source format;
    // neither is an SDL resource owned by the descriptor.
    let mut dst: SDL_Surface = std::mem::zeroed();
    dst.w = width;
    dst.h = height;
    dst.pitch = width * 4;
    dst.format = source.format;
    dst.pixels = pixels.as_mut_ptr().cast();
    let mut region = SDL_Rect {
        x: 0,
        y: 0,
        w: source.w,
        h: source.h,
    };
    Scale_PrepPlatform(flags, source.format)(src, &mut dst, &mut region);
    for (pixel, bytes) in pixels.into_iter().zip(buffer.as_chunks_mut::<4>().0) {
        *bytes = pixel.to_ne_bytes();
    }
}
