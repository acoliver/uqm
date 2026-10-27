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

pub(super) fn fill_pixel(
    source: u32,
    mask: u32,
    shift: u8,
    color: u32,
    key: u32,
    alpha: u32,
) -> u32 {
    let coverage = source & mask;
    if coverage == 0 {
        key
    } else if alpha == 255 {
        coverage | color
    } else {
        ((((coverage >> shift) * alpha) >> 8) << shift) | color
    }
}

fn surface_pixel(source: u32, destination: u32, alpha: u8) -> u32 {
    if alpha == 128 {
        return ((((source & 0xfefefe) + (destination & 0xfefefe)) >> 1)
            + (source & destination & 0x010101))
            | 0xff000000;
    }
    pixel((source & 0xffffff) | (u32::from(alpha) << 24), destination) | 0xff000000
}

enum Composition {
    Pixel,
    Surface(u8),
    LowAlpha { key: Option<u32>, alpha_mask: u32 },
}

impl Composition {
    fn apply(&self, source: u32, destination: u32) -> u32 {
        match *self {
            Self::Pixel => pixel(source, destination),
            Self::Surface(alpha) => surface_pixel(source, destination, alpha),
            Self::LowAlpha { key, alpha_mask } => {
                if key == Some(source & 0xffffff00) {
                    return destination;
                }
                let alpha = (source & 255) as i32;
                if alpha == 0 {
                    return destination;
                }
                let mut result = 0;
                for shift in [8, 16, 24] {
                    let a = ((source >> shift) & 255) as i32;
                    let b = ((destination >> shift) & 255) as i32;
                    let channel = if key.is_some() {
                        a * alpha / 255 + (255 - alpha) * b / 255
                    } else {
                        b + (a - b) * alpha / 255
                    };
                    result |= (channel as u32) << shift;
                }
                let d = if alpha_mask == 0 {
                    255
                } else {
                    destination & 255
                };
                result | (alpha as u32 + d - alpha as u32 * d / 255) & alpha_mask
            }
        }
    }
}

fn supported(
    source: &SurfaceRef,
    destination: &SurfaceRef,
    override_alpha: Option<u8>,
) -> Option<Composition> {
    let s = source.pixel_format_enum().into_masks().ok()?;
    let d = destination.pixel_format_enum().into_masks().ok()?;
    if s.bpp != 32
        || d.bpp != 32
        || (s.rmask, s.gmask, s.bmask) != (d.rmask, d.gmask, d.bmask)
        || (override_alpha.is_none() && source.blend_mode() != BlendMode::Blend)
        || source.color_mod() != sdl2::pixels::Color::RGB(255, 255, 255)
    {
        return None;
    }
    let key = source
        .color_key()
        .ok()
        .map(|color| color.to_u32(&source.pixel_format()));
    match s.amask {
        0 if s.rmask | s.gmask | s.bmask == 0xffffff && key.is_none() => Some(
            Composition::Surface(override_alpha.unwrap_or_else(|| source.alpha_mod())),
        ),
        0xff000000 if source.alpha_mod() == 255 && key.is_none() => Some(Composition::Pixel),
        255 if source.alpha_mod() == 255 && s.rmask | s.gmask | s.bmask == 0xffffff00 => {
            Some(Composition::LowAlpha {
                key: key.map(|value| value & 0xffffff00),
                alpha_mask: d.amask,
            })
        }
        _ => None,
    }
}

pub(super) fn blit(
    source: &SurfaceRef,
    source_rect: Option<SDL_Rect>,
    destination: &mut SurfaceRef,
    destination_rect: Option<&mut SDL_Rect>,
    override_alpha: Option<u8>,
) -> bool {
    let Some(surface_alpha) = supported(source, destination, override_alpha) else {
        return false;
    };
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
                    let s = read(source_bytes, si);
                    let d = read(destination_bytes, di);
                    let color = surface_alpha.apply(s, d);
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
    fn native_low_byte_alpha_and_rgbx_destination_use_exact_scalar_components() {
        let mut source = Surface::new(1, 1, PixelFormatEnum::RGBA8888)
            .unwrap_or_else(|error| panic!("source: {error}"));
        let mut destination = Surface::new(1, 1, PixelFormatEnum::RGBX8888)
            .unwrap_or_else(|error| panic!("destination: {error}"));
        source
            .set_blend_mode(BlendMode::Blend)
            .unwrap_or_else(|error| panic!("mode: {error}"));
        source.with_lock_mut(|bytes| bytes[..4].copy_from_slice(&0x11223380_u32.to_ne_bytes()));
        destination
            .with_lock_mut(|bytes| bytes[..4].copy_from_slice(&0x44332200_u32.to_ne_bytes()));
        assert!(blit(&source, None, &mut destination, None, None));
        destination.with_lock(|bytes| assert_eq!(&bytes[..4], &0x2b2b2a00_u32.to_ne_bytes()));
    }

    #[test]
    fn surface_alpha_is_composed_without_architecture_selected_sdl_rounding() {
        let mut source = Surface::new(1, 1, PixelFormatEnum::RGB888)
            .unwrap_or_else(|error| panic!("source: {error}"));
        let mut destination = Surface::new(1, 1, PixelFormatEnum::ARGB8888)
            .unwrap_or_else(|error| panic!("destination: {error}"));
        source
            .set_blend_mode(BlendMode::Blend)
            .unwrap_or_else(|error| panic!("blend mode: {error}"));
        source.with_lock_mut(|bytes| bytes[..4].copy_from_slice(&0x00112233_u32.to_ne_bytes()));
        for (alpha, expected) in [(128, 0xff2a2a2a_u32), (127, 0xff2a2a2a)] {
            source.set_alpha_mod(alpha);
            destination
                .with_lock_mut(|bytes| bytes[..4].copy_from_slice(&0x00443322_u32.to_ne_bytes()));
            assert!(blit(&source, None, &mut destination, None, None));
            destination.with_lock(|bytes| assert_eq!(&bytes[..4], &expected.to_ne_bytes()));
        }
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
            Some(&mut rect),
            None
        ));
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (1, 0, 2, 2));
        destination.with_lock(|bytes| {
            assert_eq!(&bytes[..4], &[0; 4]);
            assert_eq!(&bytes[4..8], &0x807f7f7f_u32.to_ne_bytes());
        });
        source.set_alpha_mod(100);
        assert!(!blit(&source, None, &mut destination, None, None));
    }
}
