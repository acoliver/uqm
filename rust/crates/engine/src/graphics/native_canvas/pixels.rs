//! Pixel access, collision tests and bulk pixel transfers.
use super::surface::{format, get_color_key, FALSE, TRUE};
use super::transparency::TFB_DrawCanvas_IsPaletted;
use crate::graphics::native_abi::{Color, TFB_Canvas, BOOLEAN, BYTE, POINT, RECT};
use crate::graphics::software_primitives::{getpixel_for, putpixel_for};
use sdl2_sys::{SDL_GetRGBA, SDL_LockSurface, SDL_MapRGBA, SDL_Surface, SDL_UnlockSurface};
use std::ffi::c_int;
use std::ptr;

/// Produces an alpha-only image in internal BYTE[] format from the surface.
///
/// # Safety
/// Canvas must be null or live and `out_data` must hold `dataPitch * h` bytes.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetFontCharData(
    canvas: TFB_Canvas,
    out_data: *mut BYTE,
    data_pitch: u32,
) -> BOOLEAN {
    let surf: *mut SDL_Surface = canvas.cast();
    if surf.is_null() || out_data.is_null() {
        return FALSE;
    }
    let fmt = format(surf);

    SDL_LockSurface(surf);

    let getpix = getpixel_for(surf).expect("supported canvas pixel format");

    for y in 0..(*surf).h {
        let mut dst = out_data.add(data_pitch as usize * y as usize);

        for x in 0..(*surf).w {
            let p = getpix(surf, x, y);
            let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
            SDL_GetRGBA(p, fmt, &mut r, &mut g, &mut b, &mut a);

            if fmt.Amask == 0 {
                // produce alpha from intensity (Y component)
                // using a fast approximation
                // contributions to Y are: R=2, G=4, B=1
                a = ((u32::from(r) * 2 + u32::from(g) * 4 + u32::from(b)) / 7) as u8;
            }

            *dst = a;
            dst = dst.add(1);
        }
    }

    SDL_UnlockSurface(surf);

    TRUE
}

/// # Safety
/// Canvas must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetPixel(canvas: TFB_Canvas, x: c_int, y: c_int) -> Color {
    let surf: *mut SDL_Surface = canvas.cast();
    let mut c = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    if x < 0 || x >= (*surf).w || y < 0 || y >= (*surf).h {
        // outside bounds, return 0
        return c;
    }

    SDL_LockSurface(surf);

    let getpixel = getpixel_for(surf).expect("supported canvas pixel format");
    let pixel = getpixel(surf, x, y);
    SDL_GetRGBA(
        pixel,
        (*surf).format,
        &mut c.r,
        &mut c.g,
        &mut c.b,
        &mut c.a,
    );

    SDL_UnlockSurface(surf);

    c
}

/// Per-pixel collision test of two canvases over the intersection rectangle.
///
/// # Safety
/// Canvases must be live and the rectangle must lie within both at the given
/// origins.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Intersect(
    canvas1: TFB_Canvas,
    mut c1org: POINT,
    canvas2: TFB_Canvas,
    mut c2org: POINT,
    inter_rect: *const RECT,
) -> BOOLEAN {
    let mut ret = FALSE;
    let surf1: *mut SDL_Surface = canvas1.cast();
    let surf2: *mut SDL_Surface = canvas2.cast();

    SDL_LockSurface(surf1);
    SDL_LockSurface(surf2);

    let getpixel1 = getpixel_for(surf1).expect("supported canvas pixel format");
    let getpixel2 = getpixel_for(surf2).expect("supported canvas pixel format");

    let (s1mask, s1key) = collision_mask_and_key(surf1);
    let (s2mask, s2key) = collision_mask_and_key(surf2);

    // convert surface origins to pixel offsets within
    c1org.x = (i32::from((*inter_rect).corner.x) - i32::from(c1org.x)) as i16;
    c1org.y = (i32::from((*inter_rect).corner.y) - i32::from(c1org.y)) as i16;
    c2org.x = (i32::from((*inter_rect).corner.x) - i32::from(c2org.x)) as i16;
    c2org.y = (i32::from((*inter_rect).corner.y) - i32::from(c2org.y)) as i16;

    for y in 0..i32::from((*inter_rect).extent.height) {
        for x in 0..i32::from((*inter_rect).extent.width) {
            let p1 = getpixel1(surf1, x + i32::from(c1org.x), y + i32::from(c1org.y)) & s1mask;
            let p2 = getpixel2(surf2, x + i32::from(c2org.x), y + i32::from(c2org.y)) & s2mask;

            if p1 != s1key && p2 != s2key {
                // pixel collision
                ret = TRUE;
                break;
            }
        }
    }

    SDL_UnlockSurface(surf2);
    SDL_UnlockSurface(surf1);

    ret
}

unsafe fn collision_mask_and_key(surf: *mut SDL_Surface) -> (u32, u32) {
    let amask = format(surf).Amask;
    if amask != 0 {
        // use alpha transparency info
        // consider any not fully transparent pixel collidable
        (amask, 0)
    } else {
        // colorkey transparency
        let mut colorkey = 0u32;
        get_color_key(surf, &mut colorkey);
        let mask = !amask;
        (mask, colorkey & mask)
    }
}

/// Read/write the canvas pixels in a Color format understood by the core.
/// The pixels array is assumed to be at least width * height large; a wider or
/// taller array than the canvas only transfers the overlapping pixels.
unsafe fn transfer_colors(
    canvas: TFB_Canvas,
    write: bool,
    pixels: *mut Color,
    width: i32,
    height: i32,
) -> BOOLEAN {
    let surf: *mut SDL_Surface = canvas.cast();

    if canvas.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_TransferColors passed null canvas");
        return FALSE;
    }

    let fmt = (*surf).format;
    let getpix = getpixel_for(surf).expect("supported canvas pixel format");
    let putpix = putpixel_for(surf).expect("supported canvas pixel format");

    let w = width.min((*surf).w);
    let h = height.min((*surf).h);

    SDL_LockSurface(surf);

    // This could be done faster if we assumed 32bpp surfaces
    for y in 0..h {
        // pixels array pitch is width so as not to violate the interface
        let row = pixels.offset(y as isize * width as isize);

        for x in 0..w {
            let c = row.add(x as usize);
            if write {
                // writing from data to surface
                let p = SDL_MapRGBA(fmt, (*c).r, (*c).g, (*c).b, (*c).a);
                putpix(surf, x, y, p);
            } else {
                // reading from surface to data
                let p = getpix(surf, x, y);
                SDL_GetRGBA(p, fmt, &mut (*c).r, &mut (*c).g, &mut (*c).b, &mut (*c).a);
            }
        }
    }

    SDL_UnlockSurface(surf);

    TRUE
}

/// # Safety
/// Canvas must be null or live and `pixels` must hold width * height colors.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetPixelColors(
    canvas: TFB_Canvas,
    pixels: *mut Color,
    width: c_int,
    height: c_int,
) -> BOOLEAN {
    transfer_colors(canvas, false, pixels, width, height)
}

/// # Safety
/// Canvas must be null or live and `pixels` must hold width * height colors.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_SetPixelColors(
    canvas: TFB_Canvas,
    pixels: *const Color,
    width: c_int,
    height: c_int,
) -> BOOLEAN {
    // unconst pixels, but it is safe -- it will not be written to
    transfer_colors(canvas, true, pixels.cast_mut(), width, height)
}

/// Read/write the indexed canvas pixels as palette indexes.
unsafe fn transfer_indexes(
    canvas: TFB_Canvas,
    write: bool,
    data: *mut BYTE,
    width: i32,
    height: i32,
) -> BOOLEAN {
    let surf: *mut SDL_Surface = canvas.cast();

    if canvas.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_TransferIndexes passed null canvas");
        return FALSE;
    }
    let fmt = format(surf);
    if TFB_DrawCanvas_IsPaletted(canvas) == FALSE || fmt.BitsPerPixel != 8 {
        log::warn!(
            "ERROR: TFB_DrawCanvas_TransferIndexes unimplemeted function: not an 8bpp indexed canvas"
        );
        return FALSE;
    }

    let w = width.min((*surf).w);
    let h = height.min((*surf).h);

    SDL_LockSurface(surf);

    for y in 0..h {
        let surf_p = (*surf)
            .pixels
            .cast::<u8>()
            .offset(y as isize * (*surf).pitch as isize);
        // pixels array pitch is width so as not to violate the interface
        let data_p = data.offset(y as isize * width as isize);

        if write {
            // writing from data to surface
            ptr::copy_nonoverlapping(data_p, surf_p, w as usize);
        } else {
            // reading from surface to data
            ptr::copy_nonoverlapping(surf_p, data_p, w as usize);
        }
    }

    SDL_UnlockSurface(surf);

    TRUE
}

/// # Safety
/// Canvas must be null or live and `data` must hold width * height bytes.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_GetPixelIndexes(
    canvas: TFB_Canvas,
    data: *mut BYTE,
    width: c_int,
    height: c_int,
) -> BOOLEAN {
    transfer_indexes(canvas, false, data, width, height)
}

/// # Safety
/// Canvas must be null or live and `data` must hold width * height bytes.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_SetPixelIndexes(
    canvas: TFB_Canvas,
    data: *const BYTE,
    width: c_int,
    height: c_int,
) -> BOOLEAN {
    // unconst data, but it is safe -- it will not be written to
    transfer_indexes(canvas, true, data.cast_mut(), width, height)
}
