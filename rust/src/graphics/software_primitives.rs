//! SDL software raster primitives, ported from primitives.c (GPL-2.0-or-later).
use sdl2_sys::{SDL_GetClipRect, SDL_GetColorKey, SDL_GetRGBA, SDL_MapRGBA, SDL_Rect, SDL_Surface};
use std::ptr;
pub type GetPixelFn = unsafe extern "C" fn(*mut SDL_Surface, i32, i32) -> u32;
pub type PutPixelFn = unsafe extern "C" fn(*mut SDL_Surface, i32, i32, u32);
pub type RenderPixelFn = unsafe extern "C" fn(*mut SDL_Surface, i32, i32, u32, i32);

unsafe extern "C" fn getpixel<const BPP: usize>(surface: *mut SDL_Surface, x: i32, y: i32) -> u32 {
    let p = (*surface)
        .pixels
        .cast::<u8>()
        .offset((y * (*surface).pitch + x * BPP as i32) as isize);
    match BPP {
        1 => u32::from(*p),
        2 => u32::from(ptr::read(p.cast::<u16>())),
        3 => {
            if cfg!(target_endian = "big") {
                u32::from(*p) << 16 | u32::from(*p.add(1)) << 8 | u32::from(*p.add(2))
            } else {
                u32::from(*p) | u32::from(*p.add(1)) << 8 | u32::from(*p.add(2)) << 16
            }
        }
        4 => ptr::read(p.cast::<u32>()),
        _ => unreachable!(),
    }
}
unsafe extern "C" fn putpixel<const BPP: usize>(
    surface: *mut SDL_Surface,
    x: i32,
    y: i32,
    pixel: u32,
) {
    let p = (*surface)
        .pixels
        .cast::<u8>()
        .offset((y * (*surface).pitch + x * BPP as i32) as isize);
    match BPP {
        1 => *p = pixel as u8,
        2 => ptr::write(p.cast::<u16>(), pixel as u16),
        3 => {
            let bytes = if cfg!(target_endian = "big") {
                [(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8]
            } else {
                [pixel as u8, (pixel >> 8) as u8, (pixel >> 16) as u8]
            };
            ptr::copy_nonoverlapping(bytes.as_ptr(), p, 3);
        }
        4 => ptr::write(p.cast::<u32>(), pixel),
        _ => unreachable!(),
    }
}
/// # Safety
/// Surface and its format must be live; returned function requires in-bounds coordinates.
pub unsafe extern "C" fn getpixel_for(surface: *mut SDL_Surface) -> Option<GetPixelFn> {
    match (*(*surface).format).BytesPerPixel {
        1 => Some(getpixel::<1>),
        2 => Some(getpixel::<2>),
        3 => Some(getpixel::<3>),
        4 => Some(getpixel::<4>),
        _ => None,
    }
}
/// # Safety
/// Surface and its format must be live; returned function requires in-bounds coordinates.
pub unsafe extern "C" fn putpixel_for(surface: *mut SDL_Surface) -> Option<PutPixelFn> {
    match (*(*surface).format).BytesPerPixel {
        1 => Some(putpixel::<1>),
        2 => Some(putpixel::<2>),
        3 => Some(putpixel::<3>),
        4 => Some(putpixel::<4>),
        _ => None,
    }
}
fn channel(dst: u8, src: u8, factor: i32, kind: u32) -> u8 {
    if kind == 1 {
        (i32::from(dst)
            + if factor == 255 {
                i32::from(src)
            } else {
                (i32::from(src) * factor) >> 8
            })
        .clamp(0, 255) as u8
    } else {
        ((((i32::from(src) - i32::from(dst)) * factor) >> 8) + i32::from(dst)) as u8
    }
}
unsafe extern "C" fn renderpixel<const KIND: u32>(
    surface: *mut SDL_Surface,
    x: i32,
    y: i32,
    pixel: u32,
    factor: i32,
) {
    if KIND == 0 || (KIND == 2 && factor == 255) {
        putpixel::<4>(surface, x, y, pixel);
        return;
    }
    let fmt = &*(*surface).format;
    let old = getpixel::<4>(surface, x, y);
    let mut result = 0;
    for shift in [fmt.Rshift, fmt.Gshift, fmt.Bshift] {
        result |= u32::from(channel(
            (old >> shift) as u8,
            (pixel >> shift) as u8,
            factor,
            KIND,
        )) << shift;
    }
    putpixel::<4>(surface, x, y, result);
}
/// # Safety
/// Surface must be live; returned function requires locked pixels and in-bounds coordinates.
#[no_mangle]
pub unsafe extern "C" fn renderpixel_for(
    surface: *mut SDL_Surface,
    kind: u32,
) -> Option<RenderPixelFn> {
    let fmt = &*(*surface).format;
    if fmt.BytesPerPixel != 4 || (fmt.Amask != 0 && kind != 0) {
        return None;
    }
    match kind {
        0 => Some(renderpixel::<0>),
        1 => Some(renderpixel::<1>),
        2 => Some(renderpixel::<2>),
        _ => None,
    }
}
fn code(x: f32, y: f32, xmin: f32, ymin: f32, xmax: f32, ymax: f32) -> u8 {
    let mut c = 0;
    if y > ymax {
        c |= 1;
    } else if y < ymin {
        c |= 2;
    }
    if x > xmax {
        c |= 4;
    } else if x < xmin {
        c |= 8;
    }
    c
}
/// # Safety
/// Endpoint pointers must be writable and rectangle readable.
pub unsafe extern "C" fn clip_line(
    lx0: *mut i32,
    ly0: *mut i32,
    lx1: *mut i32,
    ly1: *mut i32,
    rect: *const SDL_Rect,
) -> i32 {
    let r = &*rect;
    let (mut x0, mut y0, mut x1, mut y1) = (*lx0 as f32, *ly0 as f32, *lx1 as f32, *ly1 as f32);
    let (xmin, ymin) = (r.x as f32, r.y as f32);
    let (xmax, ymax) = (r.x as f32 + r.w as f32 - 1.0, r.y as f32 + r.h as f32 - 1.0);
    let mut c0 = code(x0, y0, xmin, ymin, xmax, ymax);
    let mut c1 = code(x1, y1, xmin, ymin, xmax, ymax);
    loop {
        if c0 | c1 == 0 {
            *lx0 = x0 as i32;
            *ly0 = y0 as i32;
            *lx1 = x1 as i32;
            *ly1 = y1 as i32;
            return 1;
        }
        if c0 & c1 != 0 {
            return 0;
        }
        let c = if c0 != 0 { c0 } else { c1 };
        let (x, y) = if c & 1 != 0 {
            (x0 + (x1 - x0) * (ymax - y0) / (y1 - y0), ymax)
        } else if c & 2 != 0 {
            (x0 + (x1 - x0) * (ymin - y0) / (y1 - y0), ymin)
        } else if c & 4 != 0 {
            (xmax, y0 + (y1 - y0) * (xmax - x0) / (x1 - x0))
        } else {
            (xmin, y0 + (y1 - y0) * (xmin - x0) / (x1 - x0))
        };
        if c == c0 {
            x0 = x;
            y0 = y;
            c0 = code(x0, y0, xmin, ymin, xmax, ymax);
        } else {
            x1 = x;
            y1 = y;
            c1 = code(x1, y1, xmin, ymin, xmax, ymax);
        }
    }
}
/// # Safety
/// Destination must be live and locked, callback must support its format.
#[allow(clippy::too_many_arguments, reason = "existing C ABI")]
pub unsafe extern "C" fn line_prim(
    mut x1: i32,
    mut y1: i32,
    mut x2: i32,
    mut y2: i32,
    color: u32,
    plot: Option<RenderPixelFn>,
    factor: i32,
    dst: *mut SDL_Surface,
) {
    let mut clip = std::mem::zeroed();
    SDL_GetClipRect(dst, &mut clip);
    if clip_line(&mut x1, &mut y1, &mut x2, &mut y2, &clip) == 0 {
        return;
    }
    let plot = plot.expect("valid pixel renderer");
    let (dx, dy) = (x2 - x1, y2 - y1);
    let (ax, ay, sx, sy) = (
        dx.abs() << 1,
        dy.abs() << 1,
        if dx < 0 { -1 } else { 1 },
        if dy < 0 { -1 } else { 1 },
    );
    let (mut x, mut y) = (x1, y1);
    if ax > ay {
        let mut d = ay - (ax >> 1);
        loop {
            plot(dst, x, y, color, factor);
            if x == x2 {
                return;
            }
            if d >= 0 {
                y += sy;
                d -= ax;
            }
            x += sx;
            d += ay;
        }
    } else {
        let mut d = ax - (ay >> 1);
        loop {
            plot(dst, x, y, color, factor);
            if y == y2 {
                return;
            }
            if d >= 0 {
                x += sx;
                d -= ay;
            }
            y += sy;
            d += ax;
        }
    }
}
/// # Safety
/// Rectangle must be writable and clipping rectangle readable.
pub unsafe extern "C" fn clip_rect(rect: *mut SDL_Rect, clip: *const SDL_Rect) -> i32 {
    let r = &mut *rect;
    let c = &*clip;
    let (mut w, mut h) = (r.w, r.h);
    let dx = c.x - r.x;
    if dx > 0 {
        w -= dx;
        r.x += dx;
    }
    let dx = r.x + w - c.x - c.w;
    if dx > 0 {
        w -= dx;
    }
    let dy = c.y - r.y;
    if dy > 0 {
        h -= dy;
        r.y += dy;
    }
    let dy = r.y + h - c.y - c.h;
    if dy > 0 {
        h -= dy;
    }
    if w <= 0 || h <= 0 {
        r.w = 0;
        r.h = 0;
        return 0;
    }
    r.w = w;
    r.h = h;
    1
}
/// # Safety
/// Destination must be live and locked, callback must support its format.
pub unsafe extern "C" fn fillrect_prim(
    mut rect: SDL_Rect,
    color: u32,
    plot: Option<RenderPixelFn>,
    factor: i32,
    dst: *mut SDL_Surface,
) {
    let mut clip = std::mem::zeroed();
    SDL_GetClipRect(dst, &mut clip);
    if clip_rect(&mut rect, &clip) == 0 {
        return;
    }
    let plot = plot.expect("valid pixel renderer");
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            plot(dst, x, y, color, factor);
        }
    }
}
/// # Safety
/// Source and destination must be writable, clip must be readable.
pub unsafe extern "C" fn clip_blt_rects(
    src: *mut SDL_Rect,
    dst: *mut SDL_Rect,
    clip: *const SDL_Rect,
) -> i32 {
    let (s, d, c) = (&mut *src, &mut *dst, &*clip);
    let (mut w, mut h) = (s.w, s.h);
    if s.x < 0 {
        w += s.x;
        d.x -= s.x;
        s.x = 0;
    }
    if s.y < 0 {
        h += s.y;
        d.y -= s.y;
        s.y = 0;
    }
    let dx = c.x - d.x;
    if dx > 0 {
        w -= dx;
        d.x += dx;
        s.x += dx;
    }
    let dx = d.x + w - c.x - c.w;
    if dx > 0 {
        w -= dx;
    }
    let dy = c.y - d.y;
    if dy > 0 {
        h -= dy;
        d.y += dy;
        s.y += dy;
    }
    let dy = d.y + h - c.y - c.h;
    if dy > 0 {
        h -= dy;
    }
    if w <= 0 || h <= 0 {
        s.w = 0;
        s.h = 0;
        return 0;
    }
    s.w = w;
    s.h = h;
    1
}
/// # Safety
/// Surfaces must be live and locked; callback must support destination format.
pub unsafe extern "C" fn blt_prim(
    src: *mut SDL_Surface,
    mut src_r: SDL_Rect,
    plot: Option<RenderPixelFn>,
    factor: i32,
    dst: *mut SDL_Surface,
    mut dst_r: SDL_Rect,
) {
    let (srcfmt, dstfmt) = ((*src).format, (*dst).format);
    let srcpal = (*srcfmt).palette;
    let (mut mask, mut key) = (0, u32::MAX);
    let get = getpixel_for(src).expect("supported source pixel format");
    let mut clip = std::mem::zeroed();
    SDL_GetClipRect(dst, &mut clip);
    if clip_blt_rects(&mut src_r, &mut dst_r, &clip) == 0 {
        return;
    }
    if src_r.x >= (*src).w || src_r.y >= (*src).h {
        return;
    }
    if src_r.x + src_r.w > (*src).w {
        src_r.w = (*src).w - src_r.x;
    }
    if src_r.y + src_r.h > (*src).h {
        src_r.h = (*src).h - src_r.y;
    }
    if (*srcfmt).Amask != 0 {
        mask = (*srcfmt).Amask;
        key = 0;
    } else if SDL_GetColorKey(src, &mut key) == 0 {
        mask = u32::MAX;
    }
    let plot = plot.expect("valid pixel renderer");
    for y in 0..src_r.h {
        for x in 0..src_r.w {
            let pixel = get(src, src_r.x + x, src_r.y + y);
            if if !srcpal.is_null() {
                pixel == key
            } else {
                pixel & mask == key
            } {
                continue;
            }
            let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0);
            SDL_GetRGBA(pixel, srcfmt, &mut r, &mut g, &mut b, &mut a);
            let pixel = SDL_MapRGBA(dstfmt, r, g, b, a);
            plot(dst, dst_r.x + x, dst_r.y + y, pixel, factor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2_sys::{SDL_CreateRGBSurface, SDL_FreeSurface, SDL_SetClipRect};
    #[test]
    fn small_canvas_lines_fills_and_blits_obey_clipping() {
        unsafe {
            let dst = SDL_CreateRGBSurface(0, 5, 4, 32, 0xff, 0xff00, 0xff0000, 0);
            let src = SDL_CreateRGBSurface(0, 3, 2, 32, 0xff, 0xff00, 0xff0000, 0);
            assert!(!dst.is_null() && !src.is_null());
            let clip = SDL_Rect {
                x: 1,
                y: 1,
                w: 3,
                h: 2,
            };
            SDL_SetClipRect(dst, &clip);
            let plot = renderpixel_for(dst, 0);
            line_prim(-2, 1, 6, 1, 5, plot, 0, dst);
            fillrect_prim(
                SDL_Rect {
                    x: 2,
                    y: 2,
                    w: 5,
                    h: 3,
                },
                7,
                plot,
                0,
                dst,
            );
            for y in 0..4 {
                for x in 0..5 {
                    let expected = if y == 1 && (1..=3).contains(&x) {
                        5
                    } else if y == 2 && (2..=3).contains(&x) {
                        7
                    } else {
                        0
                    };
                    assert_eq!(getpixel::<4>(dst, x, y), expected);
                }
            }
            putpixel::<4>(src, 1, 0, 0x123456);
            blt_prim(
                src,
                SDL_Rect {
                    x: 0,
                    y: 0,
                    w: 3,
                    h: 2,
                },
                plot,
                0,
                dst,
                SDL_Rect {
                    x: 0,
                    y: 1,
                    w: 3,
                    h: 2,
                },
            );
            assert_eq!(getpixel::<4>(dst, 1, 1) & 0xffffff, 0x123456);
            assert_eq!(getpixel::<4>(dst, 0, 1), 0);
            SDL_FreeSurface(src);
            SDL_FreeSurface(dst);
        }
    }
    #[test]
    fn additive_and_alpha_use_signed_shift_and_opaque_special_cases() {
        assert_eq!(channel(20, 200, 255, 1), 220);
        assert_eq!(channel(20, 200, 128, 1), 120);
        assert_eq!(channel(20, 200, -128, 1), 0);
        assert_eq!(channel(200, 20, 128, 2), 110);
        assert_eq!(channel(20, 200, 128, 2), 110);
    }
    #[test]
    fn line_clipping_truncates_float_intersections() {
        unsafe {
            let clip = SDL_Rect {
                x: 1,
                y: 1,
                w: 4,
                h: 4,
            };
            let (mut x0, mut y0, mut x1, mut y1) = (-3, 0, 8, 5);
            assert_eq!(clip_line(&mut x0, &mut y0, &mut x1, &mut y1, &clip), 1);
            assert_eq!((x0, y0, x1, y1), (1, 1, 4, 3));
            let mut src = SDL_Rect {
                x: -2,
                y: -1,
                w: 5,
                h: 4,
            };
            let mut dst = SDL_Rect {
                x: 0,
                y: 0,
                w: 5,
                h: 4,
            };
            assert_eq!(clip_blt_rects(&mut src, &mut dst, &clip), 1);
            assert_eq!(
                (src.x, src.y, src.w, src.h, dst.x, dst.y),
                (0, 0, 3, 3, 2, 1)
            );
        }
    }
}
