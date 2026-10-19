//! RGB32 additive rectangle core, not installed as a native canvas provider.
//!
//! Matches the supported `renderpixel_additive` arithmetic, including its
//! signed >>8 modulation and special factor 255. RGBA destinations have no
//! defined additive contract in gfxlib.h and are rejected, not reinterpreted.

use super::dcqueue::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AdditiveError {
    #[error("additive drawing requires RGB32 with disjoint eight-bit channels and no alpha")]
    UnsupportedFormat,
    #[error("invalid RGB32 buffer dimensions, pitch or length")]
    InvalidBuffer,
    #[error("additive factor must be in -32767..=32767")]
    InvalidFactor,
}

#[derive(Debug, Clone, Copy)]
pub struct Rgb32Format {
    shifts: [u32; 3],
}

impl Rgb32Format {
    /// Masks are R, G, B, A in native-endian pixel words.
    pub fn new(bytes_per_pixel: u8, masks: [u32; 4]) -> Result<Self, AdditiveError> {
        if bytes_per_pixel != 4 || masks[3] != 0 {
            return Err(AdditiveError::UnsupportedFormat);
        }
        let rgb = &masks[..3];
        if rgb.iter().any(|mask| {
            mask.count_ones() != 8
                || mask.trailing_zeros() % 8 != 0
                || *mask != 255u32 << mask.trailing_zeros()
        }) || (rgb[0] & rgb[1]) | (rgb[0] & rgb[2]) | (rgb[1] & rgb[2]) != 0
        {
            return Err(AdditiveError::UnsupportedFormat);
        }
        Ok(Self {
            shifts: [
                rgb[0].trailing_zeros(),
                rgb[1].trailing_zeros(),
                rgb[2].trailing_zeros(),
            ],
        })
    }

    fn add(self, pixel: u32, color: Color, factor: i16) -> u32 {
        self.shifts
            .into_iter()
            .zip([color.r, color.g, color.b])
            .fold(0, |packed, (shift, source)| {
                let dest = ((pixel >> shift) & 255) as i32;
                let source = i32::from(source);
                let delta = if factor == 255 {
                    source
                } else {
                    (source * i32::from(factor)) >> 8
                };
                packed | ((dest + delta).clamp(0, 255) as u32) << shift
            })
    }
}

/// A borrowed, already-locked RGB32 buffer. SDL ownership and locking remain
/// with the caller; no copy, allocation, registry handle or FFI export is added.
pub struct Rgb32CanvasMut<'a> {
    pixels: &'a mut [u8],
    width: i32,
    height: i32,
    pitch: usize,
    format: Rgb32Format,
}

impl<'a> Rgb32CanvasMut<'a> {
    pub fn new(
        pixels: &'a mut [u8],
        width: i32,
        height: i32,
        pitch: usize,
        format: Rgb32Format,
    ) -> Result<Self, AdditiveError> {
        let w = usize::try_from(width).map_err(|_| AdditiveError::InvalidBuffer)?;
        let h = usize::try_from(height).map_err(|_| AdditiveError::InvalidBuffer)?;
        let row = w.checked_mul(4).ok_or(AdditiveError::InvalidBuffer)?;
        let len = pitch.checked_mul(h).ok_or(AdditiveError::InvalidBuffer)?;
        if pitch < row || pixels.len() < len {
            return Err(AdditiveError::InvalidBuffer);
        }
        Ok(Self {
            pixels,
            width,
            height,
            pitch,
            format,
        })
    }

    /// Fill the intersection of the requested rectangle, clip and canvas.
    /// As in the supported native operation, Color.a is ignored and unused
    /// word bits are cleared only for pixels actually drawn.
    pub fn fill(
        &mut self,
        rect: Rect,
        clip: Rect,
        color: Color,
        factor: i16,
    ) -> Result<(), AdditiveError> {
        if factor == i16::MIN {
            return Err(AdditiveError::InvalidFactor);
        }
        if rect.extent.width <= 0
            || rect.extent.height <= 0
            || clip.extent.width <= 0
            || clip.extent.height <= 0
        {
            return Ok(());
        }
        let x0 = i64::from(rect.corner.x)
            .max(i64::from(clip.corner.x))
            .max(0);
        let y0 = i64::from(rect.corner.y)
            .max(i64::from(clip.corner.y))
            .max(0);
        let x1 = (i64::from(rect.corner.x) + i64::from(rect.extent.width))
            .min(i64::from(clip.corner.x) + i64::from(clip.extent.width))
            .min(i64::from(self.width));
        let y1 = (i64::from(rect.corner.y) + i64::from(rect.extent.height))
            .min(i64::from(clip.corner.y) + i64::from(clip.extent.height))
            .min(i64::from(self.height));
        for y in y0..y1 {
            for x in x0..x1 {
                let offset = y as usize * self.pitch + x as usize * 4;
                let bytes = &mut self.pixels[offset..offset + 4];
                let pixel = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                bytes.copy_from_slice(&self.format.add(pixel, color, factor).to_ne_bytes());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::dcqueue::{Extent, Point};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect::new(Point::new(x, y), Extent::new(w, h))
    }

    #[test]
    fn arithmetic_keeps_the_native_full_factor_and_signed_shift_rules() {
        let format = Rgb32Format::new(4, [0xff0000, 0xff00, 0xff, 0]).unwrap();
        let source = Color::new(255, 128, 1, 0);
        for (factor, expected) in [
            (255, 0xffe465),
            (254, 0xffe364),
            (0, 0x646464),
            (-1, 0x636363),
            (-255, 0x000063),
            (32767, 0xffffe3),
            (-32767, 0),
        ] {
            assert_eq!(
                format.add(0x646464, source, factor),
                expected,
                "factor {factor}"
            );
        }
    }

    #[test]
    fn unsupported_external_formats_are_errors() {
        for (bpp, masks) in [
            (1, [0, 0, 0, 0]),
            (2, [0xf800, 0x7e0, 0x1f, 0]),
            (3, [0xff0000, 0xff00, 0xff, 0]),
            (4, [0xff000000, 0xff0000, 0xff00, 0xff]),
            (4, [0xff, 0xff, 0xff0000, 0]),
            (4, [0xff0, 0xff0000, 0xff000000, 0]),
            (4, [0, 0xff00, 0xff0000, 0]),
        ] {
            assert_eq!(
                Rgb32Format::new(bpp, masks).unwrap_err(),
                AdditiveError::UnsupportedFormat
            );
        }
    }

    #[test]
    fn clipping_preserves_padding_and_outside_pixels_across_word_layouts() {
        for masks in [
            [0xff000000, 0xff0000, 0xff00, 0],
            [0xff, 0xff00, 0xff0000, 0],
        ] {
            let format = Rgb32Format::new(4, masks).unwrap();
            let mut pixels = [0x55; 40];
            Rgb32CanvasMut::new(&mut pixels, 3, 2, 20, format)
                .unwrap()
                .fill(
                    rect(-2, -2, 10, 10),
                    rect(1, 0, 1, 2),
                    Color::rgb(255, 0, 0),
                    255,
                )
                .unwrap();
            for y in 0..2 {
                let offset = y * 20 + 4;
                let expected = (masks[0] | (0x55555555 & (masks[1] | masks[2]))).to_ne_bytes();
                assert_eq!(&pixels[offset..offset + 4], &expected);
                assert!(pixels[y * 20..offset].iter().all(|b| *b == 0x55));
                assert!(pixels[offset + 4..(y + 1) * 20].iter().all(|b| *b == 0x55));
            }
        }
    }

    #[test]
    fn invalid_buffers_and_factors_fail_before_writing() {
        let format = Rgb32Format::new(4, [0xff0000, 0xff00, 0xff, 0]).unwrap();
        let mut bytes = [0; 16];
        for (w, h, pitch) in [(-1, 1, 4), (2, 2, 4), (2, 3, 8), (1, 2, usize::MAX)] {
            assert!(matches!(
                Rgb32CanvasMut::new(&mut bytes, w, h, pitch, format),
                Err(AdditiveError::InvalidBuffer)
            ));
        }
        let mut canvas = Rgb32CanvasMut::new(&mut bytes, 2, 2, 8, format).unwrap();
        assert_eq!(
            canvas.fill(
                rect(0, 0, 2, 2),
                rect(0, 0, 2, 2),
                Color::rgb(255, 0, 0),
                i16::MIN
            ),
            Err(AdditiveError::InvalidFactor)
        );
        assert_eq!(bytes, [0; 16]);
    }

    #[test]
    fn extreme_and_empty_rectangles_do_not_wrap() {
        let format = Rgb32Format::new(4, [0xff0000, 0xff00, 0xff, 0]).unwrap();
        let mut pixels = [0x55; 4];
        let mut canvas = Rgb32CanvasMut::new(&mut pixels, 1, 1, 4, format).unwrap();
        for area in [
            rect(i32::MAX, 0, i32::MAX, 1),
            rect(i32::MIN, 0, i32::MAX, 1),
            rect(0, 0, -1, 1),
        ] {
            canvas
                .fill(area, rect(0, 0, 1, 1), Color::rgb(255, 0, 0), 255)
                .unwrap();
        }
        assert_eq!(pixels, [0x55; 4]);
    }
}
