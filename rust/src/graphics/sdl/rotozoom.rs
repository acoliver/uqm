//! SDL_gfx rotozoomer as modified by UQM. LGPL, (c) A. Schiffler.
//! Fixed-point steps and border samples deliberately match the C implementation.
use sdl2_sys::*;
use std::ptr;

const VALUE_LIMIT: f64 = 0.001;
type Pixel = [u8; 4];

fn interpolate(c00: Pixel, c01: Pixel, c10: Pixel, c11: Pixel, ex: i32, ey: i32) -> Pixel {
    std::array::from_fn(|i| {
        let t1 = ((((i32::from(c01[i]) - i32::from(c00[i])) * ex) >> 16) + i32::from(c00[i])) & 255;
        let t2 = ((((i32::from(c11[i]) - i32::from(c10[i])) * ex) >> 16) + i32::from(c10[i])) & 255;
        ((((t2 - t1) * ey) >> 16) + t1) as u8
    })
}

unsafe fn pixel(src: *mut SDL_Surface, x: i32, y: i32) -> Pixel {
    *(*src)
        .pixels
        .cast::<u8>()
        .offset((y * (*src).pitch + x * 4) as isize)
        .cast::<Pixel>()
}
unsafe fn put(dst: *mut SDL_Surface, x: i32, y: i32, value: Pixel) {
    *(*dst)
        .pixels
        .cast::<u8>()
        .offset((y * (*dst).pitch + x * 4) as isize)
        .cast::<Pixel>() = value;
}

fn rgba_steps(src: i32, dst: i32, smooth: bool) -> Vec<i32> {
    // C promotes the explicitly cast float dimensions to double for 65536.0.
    let step =
        (65536.0 * f64::from((src - i32::from(smooth)) as f32) / f64::from(dst as f32)) as i32;
    let mut cs = 0;
    (0..=dst)
        .map(|_| {
            let value = cs;
            cs = (cs & 65535) + step;
            value
        })
        .collect()
}

/// # Safety
/// Surfaces must contain 32-bit pixels, be locked if required, and not alias.
#[no_mangle]
pub unsafe extern "C" fn zoomSurfaceRGBA(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    smooth: i32,
) -> i32 {
    let sax = rgba_steps((*src).w, (*dst).w, smooth != 0);
    let say = rgba_steps((*src).h, (*dst).h, smooth != 0);
    let mut sy = 0;
    for y in 0..(*dst).h {
        let mut sx = 0;
        for x in 0..(*dst).w {
            let c = if smooth != 0 {
                interpolate(
                    pixel(src, sx, sy),
                    pixel(src, sx + 1, sy),
                    pixel(src, sx, sy + 1),
                    pixel(src, sx + 1, sy + 1),
                    sax[x as usize] & 65535,
                    say[y as usize] & 65535,
                )
            } else {
                pixel(src, sx, sy)
            };
            put(dst, x, y, c);
            sx += sax[x as usize + 1] >> 16;
        }
        sy += say[y as usize + 1] >> 16;
    }
    0
}

unsafe fn zoom_y(src: *mut SDL_Surface, dst: *mut SDL_Surface) {
    let sx = (65536.0 * f64::from((*src).w as f32) / f64::from((*dst).w as f32)) as u32;
    let sy = (65536.0 * f64::from((*src).h as f32) / f64::from((*dst).h as f32)) as u32;
    let mut csx = 0u32;
    let sax: Vec<_> = (0..(*dst).w)
        .map(|_| {
            csx += sx;
            let v = csx >> 16;
            csx &= 65535;
            v
        })
        .collect();
    let mut csy = 0u32;
    let say: Vec<_> = (0..(*dst).h)
        .map(|_| {
            csy += sy;
            let v = csy >> 16;
            csy &= 65535;
            v
        })
        .collect();
    let mut source_y = 0;
    for y in 0..(*dst).h {
        let mut source_x = 0;
        for x in 0..(*dst).w {
            *(*dst)
                .pixels
                .cast::<u8>()
                .offset((y * (*dst).pitch + x) as isize) = *(*src)
                .pixels
                .cast::<u8>()
                .offset((source_y * (*src).pitch + source_x) as isize);
            source_x += sax[x as usize] as i32;
        }
        source_y += say[y as usize] as i32;
    }
}

unsafe fn transform_rgba(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    center: (i32, i32),
    isin: i32,
    icos: i32,
    smooth: i32,
) {
    let (cx, cy) = center;
    let xd = ((*src).w - (*dst).w) << 15;
    let yd = ((*src).h - (*dst).h) << 15;
    let ax = (cx << 16) - icos * cx;
    let ay = (cy << 16) - isin * cx;
    let sw = (*src).w - 1;
    let sh = (*src).h - 1;
    for y in 0..(*dst).h {
        let dy = cy - y;
        let mut sdx = ax + isin * dy + xd;
        let mut sdy = ay - icos * dy + yd;
        for x in 0..(*dst).w {
            if smooth != 0 {
                let dx = sdx >> 16;
                let dy = sdy >> 16;
                if dx >= -1 && dy >= -1 && dx < (*src).w && dy < (*src).h {
                    let (c00, c01, c10, c11) = if dx >= 0 && dy >= 0 && dx < sw && dy < sh {
                        (
                            pixel(src, dx, dy),
                            pixel(src, dx + 1, dy),
                            pixel(src, dx, dy + 1),
                            pixel(src, dx + 1, dy + 1),
                        )
                    } else if (dx == sw || dx == -1) && (dy == sh || dy == -1) {
                        let c = pixel(src, dx.max(0), dy.max(0));
                        (c, c, c, c)
                    } else if dx == -1 {
                        let c = pixel(src, 0, dy);
                        (c, c, c, pixel(src, 0, dy + 1))
                    } else if dy == -1 {
                        let c = pixel(src, dx, 0);
                        (c, c, c, pixel(src, dx + 1, 0))
                    } else if dx == sw {
                        let c = pixel(src, dx, dy);
                        let next = pixel(src, dx, dy + 1);
                        (c, c, next, next)
                    } else {
                        let c = pixel(src, dx, dy);
                        let next = pixel(src, dx + 1, dy);
                        (c, next, next, next)
                    };
                    put(
                        dst,
                        x,
                        y,
                        interpolate(c00, c01, c10, c11, sdx & 65535, sdy & 65535),
                    );
                }
            } else {
                let dx = (sdx >> 16) as i16 as i32;
                let dy = (sdy >> 16) as i16 as i32;
                if dx >= 0 && dy >= 0 && dx < (*src).w && dy < (*src).h {
                    put(dst, x, y, pixel(src, dx, dy));
                }
            }
            sdx = sdx.wrapping_add(icos);
            sdy = sdy.wrapping_add(isin);
        }
    }
}

unsafe fn transform_y(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    cx: i32,
    cy: i32,
    isin: i32,
    icos: i32,
) {
    let xd = ((*src).w - (*dst).w) << 15;
    let yd = ((*src).h - (*dst).h) << 15;
    let ax = (cx << 16) - icos * cx;
    let ay = (cy << 16) - isin * cx;
    let mut key = 0;
    SDL_GetColorKey(src, &mut key);
    ptr::write_bytes(
        (*dst).pixels.cast::<u8>(),
        key as u8,
        ((*dst).pitch * (*dst).h) as usize,
    );
    for y in 0..(*dst).h {
        let dy = cy - y;
        let mut sdx = ax + isin * dy + xd;
        let mut sdy = ay - icos * dy + yd;
        for x in 0..(*dst).w {
            let dx = (sdx >> 16) as i16 as i32;
            let dy = (sdy >> 16) as i16 as i32;
            if dx >= 0 && dy >= 0 && dx < (*src).w && dy < (*src).h {
                *(*dst)
                    .pixels
                    .cast::<u8>()
                    .offset((y * (*dst).pitch + x) as isize) = *(*src)
                    .pixels
                    .cast::<u8>()
                    .offset((dy * (*src).pitch + dx) as isize);
            }
            sdx = sdx.wrapping_add(icos);
            sdy = sdy.wrapping_add(isin);
        }
    }
}

fn size_trig(width: i32, height: i32, angle: f64, zoom: f64) -> (i32, i32, f64, f64) {
    let radangle = angle * (std::f64::consts::PI / 180.0);
    let sin = radangle.sin() * zoom;
    let cos = radangle.cos() * zoom;
    let x = f64::from(width / 2);
    let y = f64::from(height / 2);
    let cx = cos * x;
    let cy = cos * y;
    let sx = sin * x;
    let sy = sin * y;
    (
        2 * ((cx.abs() + sy.abs()).ceil().max(1.0) as i32),
        2 * ((sx.abs() + cy.abs()).ceil().max(1.0) as i32),
        cos,
        sin,
    )
}
/// # Safety
/// Output pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn rotozoomSurfaceSize(
    width: i32,
    height: i32,
    angle: f64,
    zoom: f64,
    dstwidth: *mut i32,
    dstheight: *mut i32,
) {
    let (w, h, _, _) = size_trig(width, height, angle, zoom);
    *dstwidth = w;
    *dstheight = h;
}
/// # Safety
/// Output pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn zoomSurfaceSize(
    width: i32,
    height: i32,
    zoomx: f64,
    zoomy: f64,
    dstwidth: *mut i32,
    dstheight: *mut i32,
) {
    *dstwidth = ((f64::from(width) * zoomx.max(VALUE_LIMIT)) as i32).max(1);
    *dstheight = ((f64::from(height) * zoomy.max(VALUE_LIMIT)) as i32).max(1);
}
unsafe fn converted_source(src: *mut SDL_Surface) -> *mut SDL_Surface {
    if matches!((*(*src).format).BitsPerPixel, 8 | 32) {
        return src;
    }
    let converted = SDL_CreateRGBSurface(
        0,
        (*src).w,
        (*src).h,
        32,
        0x000000ff,
        0x0000ff00,
        0x00ff0000,
        0xff000000,
    );
    if !converted.is_null() {
        SDL_UpperBlit(src, ptr::null(), converted, ptr::null_mut());
    }
    converted
}
unsafe fn target(src: *mut SDL_Surface, width: i32, height: i32) -> *mut SDL_Surface {
    let f = &*(*src).format;
    if f.BitsPerPixel == 32 {
        SDL_CreateRGBSurface(0, width, height, 32, f.Rmask, f.Gmask, f.Bmask, f.Amask)
    } else {
        SDL_CreateRGBSurface(0, width, height, 8, 0, 0, 0, 0)
    }
}
unsafe fn copy_palette(src: *mut SDL_Surface, dst: *mut SDL_Surface) {
    let s = &*(*(*src).format).palette;
    let d = &mut *(*(*dst).format).palette;
    ptr::copy_nonoverlapping(s.colors, d.colors, s.ncolors as usize);
    d.ncolors = s.ncolors;
}
unsafe fn alpha(dst: *mut SDL_Surface) {
    if SDL_SetSurfaceBlendMode(dst, SDL_BlendMode::SDL_BLENDMODE_BLEND) == 0 {
        SDL_SetSurfaceAlphaMod(dst, 255);
    }
}

/// # Safety
/// Source must be null or a live SDL surface. Returned surface is owned by caller.
#[no_mangle]
pub unsafe extern "C" fn rotozoomSurface(
    src: *mut SDL_Surface,
    angle: f64,
    zoom: f64,
    smooth: i32,
) -> *mut SDL_Surface {
    if src.is_null() {
        return ptr::null_mut();
    }
    let rz_src = converted_source(src);
    if rz_src.is_null() {
        return ptr::null_mut();
    }
    let zoom = zoom.max(VALUE_LIMIT);
    let rotate = angle.abs() > VALUE_LIMIT;
    let (w, h, cos, sin) = if rotate {
        size_trig((*rz_src).w, (*rz_src).h, angle, zoom)
    } else {
        let (mut w, mut h) = (0, 0);
        zoomSurfaceSize((*rz_src).w, (*rz_src).h, zoom, zoom, &mut w, &mut h);
        (w, h, 0.0, 0.0)
    };
    let dst = target(rz_src, w, h);
    if !dst.is_null() {
        SDL_LockSurface(rz_src);
        if (*(*rz_src).format).BitsPerPixel == 32 {
            if rotate {
                let inv = 65536.0 / (zoom * zoom);
                transform_rgba(
                    rz_src,
                    dst,
                    (w / 2, h / 2),
                    (sin * inv) as i32,
                    (cos * inv) as i32,
                    smooth,
                );
            } else {
                zoomSurfaceRGBA(rz_src, dst, smooth);
            }
            alpha(dst);
        } else {
            let mut key = 0;
            SDL_GetColorKey(rz_src, &mut key);
            copy_palette(rz_src, dst);
            if rotate {
                let inv = 65536.0 / (zoom * zoom);
                transform_y(
                    rz_src,
                    dst,
                    w / 2,
                    h / 2,
                    (sin * inv) as i32,
                    (cos * inv) as i32,
                );
            } else {
                zoom_y(rz_src, dst);
            }
            SDL_SetSurfaceRLE(dst, 1);
            SDL_SetColorKey(dst, SDL_bool::SDL_TRUE as i32, key);
        }
        SDL_UnlockSurface(rz_src);
    }
    if rz_src != src {
        SDL_FreeSurface(rz_src);
    }
    dst
}

/// # Safety
/// Source must be null or live; destination must be null or live and not alias it.
#[no_mangle]
pub unsafe extern "C" fn rotateSurface(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
    angle: f64,
    smooth: i32,
) -> i32 {
    if src.is_null()
        || dst.is_null()
        || (*(*src).format).BitsPerPixel != (*(*dst).format).BitsPerPixel
    {
        return -1;
    }
    if angle.abs() <= VALUE_LIMIT {
        SDL_UpperBlit(src, ptr::null(), dst, ptr::null_mut());
        return 0;
    }
    let (w, h, cos, sin) = size_trig((*src).w, (*src).h, angle, 1.0);
    if (*dst).w < w || (*dst).h < h {
        return -1;
    }
    SDL_LockSurface(src);
    SDL_LockSurface(dst);
    if (*(*src).format).BitsPerPixel == 32 {
        transform_rgba(
            src,
            dst,
            (w / 2, h / 2),
            (sin * 65536.0) as i32,
            (cos * 65536.0) as i32,
            smooth,
        );
    } else {
        copy_palette(src, dst);
        transform_y(
            src,
            dst,
            w / 2,
            h / 2,
            (sin * 65536.0) as i32,
            (cos * 65536.0) as i32,
        );
    }
    SDL_UnlockSurface(dst);
    SDL_UnlockSurface(src);
    0
}

/// # Safety
/// Source must be null or live. Returned surface is owned by caller.
#[no_mangle]
pub unsafe extern "C" fn zoomSurface(
    src: *mut SDL_Surface,
    zoomx: f64,
    zoomy: f64,
    smooth: i32,
) -> *mut SDL_Surface {
    if src.is_null() {
        return ptr::null_mut();
    }
    let rz_src = converted_source(src);
    if rz_src.is_null() {
        return ptr::null_mut();
    }
    let (mut w, mut h) = (0, 0);
    zoomSurfaceSize((*rz_src).w, (*rz_src).h, zoomx, zoomy, &mut w, &mut h);
    let dst = target(rz_src, w, h);
    if !dst.is_null() {
        SDL_LockSurface(rz_src);
        if (*(*rz_src).format).BitsPerPixel == 32 {
            zoomSurfaceRGBA(rz_src, dst, smooth);
            alpha(dst);
        } else {
            let mut key = 0;
            SDL_GetColorKey(rz_src, &mut key);
            copy_palette(rz_src, dst);
            zoom_y(rz_src, dst);
            SDL_SetSurfaceRLE(dst, 0);
            SDL_SetColorKey(dst, SDL_bool::SDL_TRUE as i32, key);
        }
        SDL_UnlockSurface(rz_src);
    }
    if rz_src != src {
        SDL_FreeSurface(rz_src);
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolation_keeps_signed_shift_rounding() {
        assert_eq!(
            interpolate([255; 4], [0; 4], [20; 4], [80; 4], 32768, 32768),
            [88; 4]
        );
        assert_eq!(rgba_steps(3, 4, true), [0, 32768, 65536, 32768, 65536]);
        assert_eq!(size_trig(5, 7, 0.0, 1.0), (4, 6, 1.0, 0.0));
    }
    #[test]
    fn small_surface_zoom_fixture() {
        unsafe {
            let src = SDL_CreateRGBSurface(0, 2, 2, 32, 255, 65280, 16711680, 4278190080);
            for (x, y, c) in [(0, 0, 0), (1, 0, 100), (0, 1, 200), (1, 1, 240)] {
                put(src, x, y, [c; 4]);
            }
            let dst = zoomSurface(src, 2.0, 2.0, 0);
            assert_eq!(pixel(dst, 2, 3), [240; 4]);
            assert_eq!(pixel(dst, 1, 0), [0; 4]);
            SDL_FreeSurface(dst);
            let dst = zoomSurface(src, 1.0, 1.0, 1);
            assert_eq!(pixel(dst, 1, 1), [135; 4]);
            SDL_FreeSurface(dst);
            SDL_FreeSurface(src);
        }
    }
}
