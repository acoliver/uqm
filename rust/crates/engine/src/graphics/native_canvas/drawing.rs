//! Primitive, image and font-character drawing onto canvases.
use super::lifecycle::TFB_DrawCanvas_Delete;
use super::surface::{
    cmap_colors, color_to_native, disable_surface_alpha_mod, format, get_color_key,
    has_surface_alpha_mod, rect_from, set_color_key, set_colors, set_surface_alpha_mod,
};
use super::transparency::TFB_DrawCanvas_IsPaletted;
use super::{DRAW_ALPHA, DRAW_REPLACE, GSCALE_IDENTITY, TFB_SCALE_BILINEAR, TFB_SCALE_TRILINEAR};
use crate::graphics::cmap_abi::TFB_ReturnColorMap;
use crate::graphics::native_abi::{
    Color, DrawMode, TFB_Canvas, TFB_Char, TFB_ColorMap, TFB_DrawImage_FixScaling, TFB_Image,
    POINT, RECT,
};
use crate::graphics::software_primitives::{blt_prim, fillrect_prim, line_prim, renderpixel_for};
use sdl2_sys::{
    SDL_Color, SDL_CreateRGBSurface, SDL_FillRect, SDL_LockSurface, SDL_MapRGBA, SDL_Rect,
    SDL_SetClipRect, SDL_Surface, SDL_UnlockSurface, SDL_UpperBlit,
};
use std::ffi::c_int;
use std::ptr;

use crate::threading::legacy_abi::LockMutex;

use crate::threading::legacy_abi::UnlockMutex;

const SDL_SWSURFACE: u32 = 0;

fn same_color(c1: Color, c2: Color) -> bool {
    c1.r == c2.r && c1.g == c2.g && c1.b == c2.b && c1.a == c2.a
}

/// Primitives may draw `DRAW_ALPHA` onto non-alpha surfaces via `Color.a`.
fn check_primitive_mode(alpha_mask: u32, color: &mut Color, mode: &mut DrawMode) {
    // Special case: We support DRAW_ALPHA mode to non-alpha surfaces
    // for primitives via Color.a
    if mode.kind == DRAW_REPLACE && alpha_mask == 0 && color.a != 0xff {
        mode.kind = DRAW_ALPHA;
        mode.factor = i16::from(color.a);
        color.a = 0xff;
    }
}

/// # Safety
/// Target must be a live SDL surface.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Line(
    x1: c_int,
    y1: c_int,
    x2: c_int,
    y2: c_int,
    mut color: Color,
    mut mode: DrawMode,
    target: TFB_Canvas,
) {
    let dst: *mut SDL_Surface = target.cast();

    check_primitive_mode(format(dst).Amask, &mut color, &mut mode);
    let sdl_color = SDL_MapRGBA(format(dst), color.r, color.g, color.b, color.a);

    let plot_fn = renderpixel_for(dst, u32::from(mode.kind));
    if plot_fn.is_none() {
        log::warn!(
            "ERROR: TFB_DrawCanvas_Line unsupported draw mode ({})",
            mode.kind
        );
        return;
    }

    SDL_LockSurface(dst);
    line_prim(
        x1,
        y1,
        x2,
        y2,
        sdl_color,
        plot_fn,
        i32::from(mode.factor),
        dst,
    );
    SDL_UnlockSurface(dst);
}

/// # Safety
/// Target must be a live SDL surface and rect readable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Rect(
    rect: *mut RECT,
    mut color: Color,
    mut mode: DrawMode,
    target: TFB_Canvas,
) {
    let dst: *mut SDL_Surface = target.cast();
    let fmt = format(dst);
    let sr = rect_from(
        (*rect).corner.x,
        (*rect).corner.y,
        (*rect).extent.width,
        (*rect).extent.height,
    );

    check_primitive_mode(format(dst).Amask, &mut color, &mut mode);
    let mut sdl_color = SDL_MapRGBA(fmt, color.r, color.g, color.b, color.a);

    if mode.kind == DRAW_REPLACE {
        // Standard SDL fillrect rendering
        let mut colorkey = 0u32;
        if fmt.Amask != 0 && get_color_key(dst, &mut colorkey) == 0 {
            // special case -- alpha surface with colorkey
            // colorkey rects are transparent
            if (sdl_color & !fmt.Amask) == (colorkey & !fmt.Amask) {
                sdl_color &= !fmt.Amask; // make transparent
            }
        }
        SDL_FillRect(dst, &sr, sdl_color);
    } else {
        // Custom fillrect rendering
        let plot_fn = renderpixel_for(dst, u32::from(mode.kind));
        if plot_fn.is_none() {
            log::warn!(
                "ERROR: TFB_DrawCanvas_Rect unsupported draw mode ({})",
                mode.kind
            );
            return;
        }

        SDL_LockSurface(dst);
        fillrect_prim(sr, sdl_color, plot_fn, i32::from(mode.factor), dst);
        SDL_UnlockSurface(dst);
    }
}

/// # Safety
/// Surfaces must be live; the rectangles are null or writable.
pub(super) unsafe fn blit(
    src: *mut SDL_Surface,
    mut src_r: *mut SDL_Rect,
    dst: *mut SDL_Surface,
    mut dst_r: *mut SDL_Rect,
    mode: DrawMode,
) {
    let surface_alpha = mode.kind == DRAW_ALPHA && format(src).Amask == 0;
    let override_alpha = surface_alpha.then_some(mode.factor as u8);
    if mode.kind == DRAW_REPLACE || surface_alpha {
        if src != dst {
            // SAFETY: distinct live canvases own disjoint pixel storage; rectangles
            // remain borrowed only for this synchronous blit.
            let (source, source_rect, destination, destination_rect) = unsafe {
                (
                    sdl2::surface::SurfaceRef::from_ll(src),
                    src_r.as_ref().copied(),
                    sdl2::surface::SurfaceRef::from_ll_mut(dst),
                    dst_r.as_mut(),
                )
            };
            if super::alpha::blit(
                source,
                source_rect,
                destination,
                destination_rect,
                override_alpha,
            ) {
                return;
            }
        }
        if surface_alpha {
            assert!(!has_surface_alpha_mod(src));
            set_surface_alpha_mod(src, mode.factor as u8);
        }
        SDL_UpperBlit(src, src_r, dst, dst_r);
        if surface_alpha {
            disable_surface_alpha_mod(src);
        }
    } else {
        // Custom blit
        let mut loc_src_r = SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        let mut loc_dst_r = loc_src_r;
        let plot_fn = renderpixel_for(dst, u32::from(mode.kind));
        if plot_fn.is_none() {
            log::warn!(
                "ERROR: TFB_DrawCanvas_Blit unsupported draw mode ({})",
                mode.kind
            );
            return;
        }

        if src_r.is_null() {
            // blit whole image; generate rect
            loc_src_r.w = (*src).w;
            loc_src_r.h = (*src).h;
            src_r = &mut loc_src_r;
        }

        if dst_r.is_null() {
            // blit to 0,0; generate rect
            loc_dst_r.w = (*dst).w;
            loc_dst_r.h = (*dst).h;
            dst_r = &mut loc_dst_r;
        }

        SDL_LockSurface(dst);
        blt_prim(src, *src_r, plot_fn, i32::from(mode.factor), dst, *dst_r);
        SDL_UnlockSurface(dst);
    }
}

/// The colormap, if any, must have been acquired via `TFB_GetColorMap`; it is
/// released here.
///
/// # Safety
/// Image and target must be live; the colormap is null or a held reference.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_Image(
    img: *mut TFB_Image,
    x: c_int,
    y: c_int,
    scale: c_int,
    scale_mode: c_int,
    cmap: *mut TFB_ColorMap,
    mode: DrawMode,
    target: TFB_Canvas,
) {
    if img.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_Image passed null image ptr");
        return;
    }

    LockMutex((*img).mutex);
    draw_image_locked(img, x, y, scale, scale_mode, cmap, mode, target.cast());
    UnlockMutex((*img).mutex);
}

#[allow(clippy::too_many_arguments, reason = "mirrors the C draw command")]
pub(super) unsafe fn draw_image_locked(
    img: *mut TFB_Image,
    x: i32,
    y: i32,
    scale: i32,
    mut scale_mode: i32,
    cmap: *mut TFB_ColorMap,
    mode: DrawMode,
    target: *mut SDL_Surface,
) {
    let mut src_rect = SDL_Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let mut target_rect = src_rect;
    let p_src_rect: *mut SDL_Rect;
    let surf: *mut SDL_Surface;

    let normal_pal = format((*img).NormalImg.cast()).palette;
    // only set the new palette if it changed
    if !normal_pal.is_null() && !cmap.is_null() && (*img).colormap_version != (*cmap).version {
        set_colors((*img).NormalImg.cast(), cmap_colors(cmap), 0, 256);
    }

    if scale != 0 && scale != GSCALE_IDENTITY {
        if scale_mode == TFB_SCALE_TRILINEAR && !(*img).MipmapImg.is_null() {
            // only set the new palette if it changed
            if TFB_DrawCanvas_IsPaletted((*img).MipmapImg) != 0
                && !cmap.is_null()
                && (*img).colormap_version != (*cmap).version
            {
                set_colors((*img).MipmapImg.cast(), cmap_colors(cmap), 0, 256);
            }
        } else if scale_mode == TFB_SCALE_TRILINEAR && (*img).MipmapImg.is_null() {
            // Do bilinear scaling instead when mipmap is unavailable
            scale_mode = TFB_SCALE_BILINEAR;
        }

        TFB_DrawImage_FixScaling(img, scale, scale_mode);
        surf = (*img).ScaledImg.cast();
        if TFB_DrawCanvas_IsPaletted(surf.cast()) != 0 {
            // We may only get a paletted scaled image if the source is
            // paletted. Currently, all scaling targets are truecolor.
            assert!(!normal_pal.is_null() && !(*normal_pal).colors.is_null());
            set_colors(surf, (*normal_pal).colors, 0, (*normal_pal).ncolors);
        }

        src_rect.w = i32::from((*img).extent.width);
        src_rect.h = i32::from((*img).extent.height);
        p_src_rect = &mut src_rect;

        target_rect.x = x - i32::from((*img).last_scale_hs.x);
        target_rect.y = y - i32::from((*img).last_scale_hs.y);
    } else {
        surf = (*img).NormalImg.cast();
        p_src_rect = ptr::null_mut();

        target_rect.x = x - i32::from((*img).NormalHs.x);
        target_rect.y = y - i32::from((*img).NormalHs.y);
    }

    if !cmap.is_null() {
        (*img).colormap_version = (*cmap).version;
        // TODO: Technically, this is not a proper place to release a
        //   colormap. As it stands now, the colormap must have been
        //   addrefed when passed to us.
        TFB_ReturnColorMap(cmap.cast());
    }

    blit(surf, p_src_rect, target, &mut target_rect, mode);
}

/// Assumes the source and destination surfaces are in the same format.
pub(super) unsafe fn fill(src: *mut SDL_Surface, mut fillcolor: u32, dst: *mut SDL_Surface) {
    let srcfmt = format(src);
    let dstfmt = format(dst);
    let width = (*src).w;
    let height = (*src).h;
    let bpp = i32::from(dstfmt.BytesPerPixel);
    let (sp, dp) = ((*src).pitch, (*dst).pitch);
    let (slen, dlen) = (sp / bpp, dp / bpp);
    let mut srckey = 0u32;
    let mut dstkey = 0u32; // 0 means alpha=0 too
    let amask = srcfmt.Amask;
    let alpha = (fillcolor & amask) >> srcfmt.Ashift;

    if srcfmt.BytesPerPixel != 4 || dstfmt.BytesPerPixel != 4 {
        log::warn!(
            "TFB_DrawCanvas_Fill: Unsupported surface formats: {} bytes/pixel source, {} bytes/pixel destination",
            srcfmt.BytesPerPixel,
            dstfmt.BytesPerPixel
        );
        return;
    }

    // Strip the alpha channel from fillcolor because we process the
    // alpha separately
    fillcolor &= !amask;

    SDL_LockSurface(src);
    SDL_LockSurface(dst);

    let src_p = (*src).pixels.cast::<u32>();
    let dst_p = (*dst).pixels.cast::<u32>();

    if dstkey == fillcolor {
        // color collision, must switch colorkey
        // new colorkey is grey (1/2,1/2,1/2)
        dstkey = SDL_MapRGBA(dstfmt, 127, 127, 127, 0);
    }

    if srcfmt.Amask != 0 {
        // alpha-based fill
        for y in 0..height as isize {
            for x in 0..width as isize {
                let s = src_p.offset(y * slen as isize + x);
                let d = dst_p.offset(y * dlen as isize + x);
                *d = super::alpha::fill_pixel(*s, amask, srcfmt.Ashift, fillcolor, dstkey, alpha);
            }
        }
    } else if get_color_key(src, &mut srckey) == 0 {
        // colorkey-based fill
        for y in 0..height as isize {
            for x in 0..width as isize {
                let p = *src_p.offset(y * slen as isize + x);
                *dst_p.offset(y * dlen as isize + x) = if p == srckey { dstkey } else { fillcolor };
            }
        }
    } else {
        log::warn!("TFB_DrawCanvas_Fill: Unsupported sourcesurface format");
    }

    SDL_UnlockSurface(dst);
    SDL_UnlockSurface(src);

    // save the colorkey (dynamic image -- not using RLE coding here)
    set_color_key(dst, dstkey, 0);
    // if the filled surface is RGBA, colorkey will only be used
    // when SDL_SRCALPHA flag is cleared. this allows us to blit
    // the surface in different ways to diff targets
}

/// # Safety
/// Image and target must be live.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_FilledImage(
    img: *mut TFB_Image,
    x: c_int,
    y: c_int,
    scale: c_int,
    scale_mode: c_int,
    mut color: Color,
    mut mode: DrawMode,
    target: TFB_Canvas,
) {
    let dst: *mut SDL_Surface = target.cast();

    if img.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_FilledImage passed null image ptr");
        return;
    }

    check_primitive_mode(format(dst).Amask, &mut color, &mut mode);

    LockMutex((*img).mutex);
    draw_filled_image_locked(img, x, y, scale, scale_mode, color, mode, dst);
    UnlockMutex((*img).mutex);
}

#[allow(clippy::too_many_arguments, reason = "mirrors the C draw command")]
pub(super) unsafe fn draw_filled_image_locked(
    img: *mut TFB_Image,
    x: i32,
    y: i32,
    scale: i32,
    mut scale_mode: i32,
    mut color: Color,
    mut mode: DrawMode,
    dst: *mut SDL_Surface,
) {
    let mut src_rect = SDL_Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let mut target_rect = src_rect;
    let p_src_rect: *mut SDL_Rect;
    let mut surf: *mut SDL_Surface;
    let mut force_fill = false;

    if scale != 0 && scale != GSCALE_IDENTITY {
        if scale_mode == TFB_SCALE_TRILINEAR {
            scale_mode = TFB_SCALE_BILINEAR; // no point in trilinear for filled images
        }

        if scale != (*img).last_scale || scale_mode != (*img).last_scale_type {
            force_fill = true;
        }

        TFB_DrawImage_FixScaling(img, scale, scale_mode);
        surf = (*img).ScaledImg.cast();
        src_rect.w = i32::from((*img).extent.width);
        src_rect.h = i32::from((*img).extent.height);
        p_src_rect = &mut src_rect;

        target_rect.x = x - i32::from((*img).last_scale_hs.x);
        target_rect.y = y - i32::from((*img).last_scale_hs.y);
    } else {
        if (*img).last_scale != 0 {
            // Make sure we remember that the last fill was from
            // an unscaled image
            force_fill = true;
            (*img).last_scale = 0;
        }

        surf = (*img).NormalImg.cast();
        p_src_rect = ptr::null_mut();

        target_rect.x = x - i32::from((*img).NormalHs.x);
        target_rect.y = y - i32::from((*img).NormalHs.y);
    }

    let palette = format(surf).palette;
    if !palette.is_null() {
        // set palette for fill-stamp
        // Calling tfb_set_colors() results in an expensive src -> dst
        // color-mapping operation for an SDL blit, following the call.
        // We want to avoid that as much as possible.

        // TODO: generate a 32bpp filled image?
        let mut colors = [SDL_Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }; 256];

        colors[0] = color_to_native(color);
        for i in 1..(*palette).ncolors as usize {
            colors[i] = colors[0];
        }

        set_colors(surf, colors.as_ptr(), 0, (*palette).ncolors);
        // reflect the change in *actual* image palette
        (*img).colormap_version -= 1;
    } else {
        // fill the non-transparent parts of the image with fillcolor
        let mut newfill: *mut SDL_Surface = (*img).FilledImg.cast();

        if !newfill.is_null() && ((*newfill).w < (*surf).w || (*newfill).h < (*surf).h) {
            TFB_DrawCanvas_Delete(newfill.cast());
            newfill = ptr::null_mut();
        }

        // prepare the filled image
        if newfill.is_null() {
            let surf_fmt = format(surf);
            newfill = SDL_CreateRGBSurface(
                SDL_SWSURFACE,
                (*surf).w,
                (*surf).h,
                i32::from(surf_fmt.BitsPerPixel),
                surf_fmt.Rmask,
                surf_fmt.Gmask,
                surf_fmt.Bmask,
                surf_fmt.Amask,
            );
            assert!(
                !newfill.is_null(),
                "TFB_DrawCanvas_FilledImage: out of surfaces"
            );
            force_fill = true;
        }
        let fillfmt = format(newfill);

        if force_fill || !same_color((*img).last_fill, color) {
            // image or fillcolor changed - regenerate
            if mode.kind == DRAW_ALPHA && fillfmt.Amask != 0 {
                // Per-pixel alpha and surface alpha will not work together
                // We have to handle DRAW_ALPHA differently by modulating
                // the surface alpha channel ourselves.
                color.a = mode.factor as u8;
                mode.kind = DRAW_REPLACE;
            } else {
                // Make sure we do not modulate the alpha channel
                color.a = 0xff;
            }
            let fill_color = SDL_MapRGBA(fillfmt, color.r, color.g, color.b, color.a);
            fill(surf, fill_color, newfill);
            // cache filled image if possible
            (*img).last_fill = color;
        }

        (*img).FilledImg = newfill.cast();
        surf = newfill;
    }

    blit(surf, p_src_rect, dst, &mut target_rect, mode);
}

/// # Safety
/// Font character, backing image and target must be live.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_FontChar(
    font_char: *mut TFB_Char,
    backing: *mut TFB_Image,
    x: c_int,
    y: c_int,
    mode: DrawMode,
    target: TFB_Canvas,
) {
    if font_char.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_FontChar passed null char ptr");
        return;
    }
    if backing.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_FontChar passed null backing ptr");
        return;
    }

    LockMutex((*backing).mutex);
    draw_font_char_locked(font_char, backing, x, y, mode, target.cast());
    UnlockMutex((*backing).mutex);
}

pub(super) unsafe fn draw_font_char_locked(
    font_char: *mut TFB_Char,
    backing: *mut TFB_Image,
    x: i32,
    y: i32,
    mut mode: DrawMode,
    dst: *mut SDL_Surface,
) {
    let w = i32::from((*font_char).extent.width);
    let h = i32::from((*font_char).extent.height);

    let surf: *mut SDL_Surface = (*backing).NormalImg.cast();
    if format(surf).BytesPerPixel != 4 || (*surf).w < w || (*surf).h < h {
        log::warn!(
            "ERROR: TFB_DrawCanvas_FontChar bad backing surface: {}x{}x{}; char: {}x{}",
            (*surf).w,
            (*surf).h,
            format(surf).BytesPerPixel,
            w,
            h
        );
        return;
    }

    let mut src_rect = SDL_Rect {
        x: 0,
        y: 0,
        w: i32::from((*font_char).extent.width),
        h: i32::from((*font_char).extent.height),
    };

    let mut target_rect = SDL_Rect {
        x: x - i32::from((*font_char).HotSpot.x),
        y: y - i32::from((*font_char).HotSpot.y),
        w: 0,
        h: 0,
    };

    // transfer the alpha channel to the backing surface
    SDL_LockSurface(surf);
    {
        let sskip = (*font_char).pitch as i32 - w;
        let dskip = ((*surf).pitch / 4) - w;
        let dmask = !format(surf).Amask;
        let ashift = u32::from(format(surf).Ashift);
        let mut src_p = (*font_char).data;
        let mut dst_p = (*surf).pixels.cast::<u32>();

        if mode.kind == DRAW_ALPHA {
            // Per-pixel alpha and surface alpha will not work together
            // We have to handle DRAW_ALPHA differently by modulating
            // the backing surface alpha channel ourselves.
            // The existing backing surface alpha channel is ignored.
            let alpha = mode.factor as i32;
            mode.kind = DRAW_REPLACE;

            for _ in 0..h {
                for _ in 0..w {
                    let p = *dst_p & dmask;
                    let mut a = u32::from(*src_p);

                    // we use >> 8 instead of / 255, and it does not handle
                    // alpha == 255 correctly
                    if alpha != 0xff {
                        // modulate the alpha channel
                        a = a.wrapping_mul(alpha as u32) >> 8;
                    }
                    *dst_p = p | (a << ashift);
                    src_p = src_p.add(1);
                    dst_p = dst_p.add(1);
                }
                src_p = src_p.wrapping_offset(sskip as isize);
                dst_p = dst_p.wrapping_offset(dskip as isize);
            }
        } else {
            // Transfer the alpha channel to the backing surface
            // DRAW_REPLACE + Color.a is NOT supported right now
            for _ in 0..h {
                for _ in 0..w {
                    *dst_p = (*dst_p & dmask) | (u32::from(*src_p) << ashift);
                    src_p = src_p.add(1);
                    dst_p = dst_p.add(1);
                }
                src_p = src_p.wrapping_offset(sskip as isize);
                dst_p = dst_p.wrapping_offset(dskip as isize);
            }
        }
    }
    SDL_UnlockSurface(surf);

    blit(surf, &mut src_rect, dst, &mut target_rect, mode);
}

/// # Safety
/// Both canvases must be null or live SDL surfaces and the rect readable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_CopyRect(
    source: TFB_Canvas,
    src_rect: *const RECT,
    target: TFB_Canvas,
    dst_pt: POINT,
) {
    if source.is_null() || target.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_CopyRect passed null canvas ptr");
        return;
    }

    let source_rect = rect_from(
        (*src_rect).corner.x,
        (*src_rect).corner.y,
        (*src_rect).extent.width,
        (*src_rect).extent.height,
    );

    // According to SDL docs, width and height are ignored, but
    // we'll set them anyway, just in case.
    let mut target_rect = rect_from(
        dst_pt.x,
        dst_pt.y,
        (*src_rect).extent.width,
        (*src_rect).extent.height,
    );

    SDL_UpperBlit(source.cast(), &source_rect, target.cast(), &mut target_rect);
}

/// # Safety
/// Canvas must be null or a live SDL surface and the rect null or readable.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCanvas_SetClipRect(canvas: TFB_Canvas, clip_rect: *const RECT) {
    if canvas.is_null() {
        log::warn!("ERROR: TFB_DrawCanvas_SetClipRect passed null canvas ptr");
        return;
    }

    if clip_rect.is_null() {
        // clipping disabled
        SDL_SetClipRect(canvas.cast(), ptr::null());
    } else {
        let r = rect_from(
            (*clip_rect).corner.x,
            (*clip_rect).corner.y,
            (*clip_rect).extent.width,
            (*clip_rect).extent.height,
        );
        SDL_SetClipRect(canvas.cast(), &r);
    }
}
