//! Integer 2x screen filters, ported from the SDL software scaler family.
//!
//! Packed-channel truncation and the order of fixed-point YUV operations are
//! intentional: changing either changes pixels produced by the original game.

use sdl2_sys::{SDL_PixelFormat, SDL_Rect, SDL_Surface};

#[derive(Clone, Copy)]
enum Filter {
    Nearest,
    Bilinear,
    Triscan,
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
            1,
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
