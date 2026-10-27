//! Integer pixel-alpha composition shared by all CPU architectures.
use sdl2::render::BlendMode;
use sdl2::surface::SurfaceRef;
use sdl2_sys::SDL_Rect;

fn pixel(source: u32, destination: u32) -> u32 {
    let alpha = source >> 24;
    match alpha {
        0 => destination,
        255 => source,
        _ => {
            let blend = |mask: u32| {
                let s = source & mask;
                let d = destination & mask;
                d.wrapping_add(s.wrapping_sub(d).wrapping_mul(alpha) >> 8) & mask
            };
            let a = alpha + (((destination >> 24) * (alpha ^ 255)) >> 8);
            blend(0xff00ff) | blend(0xff00) | (a << 24)
        }
    }
}

pub(super) fn blit(
    source: &SurfaceRef,
    source_rect: Option<SDL_Rect>,
    destination: &mut SurfaceRef,
    destination_rect: Option<&mut SDL_Rect>,
) -> bool {
    let source_format = source.pixel_format_enum().into_masks();
    let destination_format = destination.pixel_format_enum().into_masks();
    let (Ok(s), Ok(d)) = (source_format, destination_format) else {
        return false;
    };
    if s.bpp != 32
        || d.bpp != 32
        || s.amask != 0xff000000
        || (s.rmask, s.gmask, s.bmask) != (d.rmask, d.gmask, d.bmask)
        || source.blend_mode() != BlendMode::Blend
        || source.alpha_mod() != 255
        || source.color_mod() != sdl2::pixels::Color::RGB(255, 255, 255)
        || source.color_key().is_ok()
    {
        return false;
    }
    let src = source_rect.unwrap_or(SDL_Rect {
        x: 0,
        y: 0,
        w: source.width() as i32,
        h: source.height() as i32,
    });
    let dst = destination_rect.as_deref().copied().unwrap_or(SDL_Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    });
    let Some(clip) = destination.clip_rect() else {
        return true;
    };
    let left = 0_i64
        .max(-i64::from(src.x))
        .max(i64::from(clip.x()) - i64::from(dst.x));
    let top = 0_i64
        .max(-i64::from(src.y))
        .max(i64::from(clip.y()) - i64::from(dst.y));
    let right = i64::from(src.w)
        .min(i64::from(source.width()) - i64::from(src.x))
        .min(i64::from(clip.right()) - i64::from(dst.x));
    let bottom = i64::from(src.h)
        .min(i64::from(source.height()) - i64::from(src.y))
        .min(i64::from(clip.bottom()) - i64::from(dst.y));
    if let Some(rect) = destination_rect {
        rect.x = (i64::from(dst.x) + left) as i32;
        rect.y = (i64::from(dst.y) + top) as i32;
        rect.w = (right - left).max(0) as i32;
        rect.h = (bottom - top).max(0) as i32;
    }
    let source_pitch = source.pitch() as usize;
    let destination_pitch = destination.pitch() as usize;
    source.with_lock(|source_bytes| {
        destination.with_lock_mut(|destination_bytes| {
            for y in top..bottom {
                for x in left..right {
                    let si = (i64::from(src.y) + y) as usize * source_pitch
                        + (i64::from(src.x) + x) as usize * 4;
                    let di = (i64::from(dst.y) + y) as usize * destination_pitch
                        + (i64::from(dst.x) + x) as usize * 4;
                    let read = |bytes: &[u8], i| {
                        u32::from_ne_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
                    };
                    let color = pixel(read(source_bytes, si), read(destination_bytes, di));
                    destination_bytes[di..di + 4].copy_from_slice(&color.to_ne_bytes());
                }
            }
        })
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::pixels::PixelFormatEnum;
    use sdl2::surface::Surface;

    #[test]
    fn scalar_blend_preserves_transparency_opaque_copy_and_integer_rounding() {
        assert_eq!(pixel(0x00112233, 0x55443322), 0x55443322);
        assert_eq!(pixel(0xff112233, 0x55443322), 0xff112233);
        assert_eq!(pixel(0x80ffffff, 0), 0x807f7f7f);
        assert_eq!(pixel(0x80000000, 0xffffffff), 0xfe7f7f7f);
    }

    #[test]
    fn real_surfaces_apply_source_and_destination_clipping_without_touching_padding() {
        let mut source =
            Surface::new(3, 2, PixelFormatEnum::RGBA32).unwrap_or_else(|e| panic!("source: {e}"));
        let mut destination = Surface::new(3, 2, PixelFormatEnum::RGBA32)
            .unwrap_or_else(|e| panic!("destination: {e}"));
        source
            .set_blend_mode(BlendMode::Blend)
            .unwrap_or_else(|e| panic!("blend: {e}"));
        source.with_lock_mut(|bytes| {
            bytes
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .for_each(|p| *p = 0x80ffffff_u32.to_ne_bytes())
        });
        destination.with_lock_mut(|bytes| bytes.fill(0));
        destination.set_clip_rect(sdl2::rect::Rect::new(1, 0, 2, 2));
        let mut rect = SDL_Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        assert!(blit(
            &source,
            Some(SDL_Rect {
                x: -1,
                y: 0,
                w: 4,
                h: 2
            }),
            &mut destination,
            Some(&mut rect)
        ));
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (1, 0, 2, 2));
        destination.with_lock(|bytes| {
            assert_eq!(&bytes[..4], &[0; 4]);
            assert_eq!(&bytes[4..8], &0x807f7f7f_u32.to_ne_bytes());
        });
        source.set_alpha_mod(100);
        assert!(!blit(&source, None, &mut destination, None));
    }
}
