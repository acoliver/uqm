// ==============================================================================
// Nearest Neighbor Scaler
// ==============================================================================

/// Nearest-neighbor scaling implementation
///
/// Fast but produces pixelated results when upscaling.
/// Corresponds to: `sc2/src/libs/graphics/sdl/nearest2x.c`
pub struct NearestScaler;

impl NearestScaler {
    /// Create a new nearest scaler
    pub fn new() -> Self {
        Self
    }

    /// Scale using nearest-neighbor interpolation
    fn scale_nearest(src: &Pixmap, params: ScaleParams) -> Result<Pixmap> {
        if params.scale <= 0 {
            return Err(ScaleError::InvalidScaleFactor {
                factor: params.scale,
            }
            .into());
        }

        let scale_factor = params.scale_factor();
        let dst_width = ((src.width() as f32) * scale_factor).round() as u32;
        let dst_height = ((src.height() as f32) * scale_factor).round() as u32;

        if dst_width == 0 || dst_height == 0 {
            return Err(ScaleError::InvalidDimensions.into());
        }

        let id =
            NonZeroU32::new(1).ok_or_else(|| anyhow::anyhow!("Failed to generate pixmap ID"))?;
        let mut dst = Pixmap::new(id, dst_width, dst_height, src.format())?;

        // Only support 32-bit RGBA format for now
        if src.format() != PixmapFormat::Rgba32 {
            return Err(ScaleError::FormatMismatch.into());
        }

        let src_data = src.data();
        let dst_data = dst.data_mut();
        let src_stride = src.bytes_per_row() as usize;
        let dst_stride = dst.bytes_per_row() as usize;

        // Nearest-neighbor: for each destination pixel, find nearest source pixel
        for dst_y in 0..dst_height {
            for dst_x in 0..dst_width {
                // Calculate source coordinates (centered sampling)
                let src_x = ((dst_x as f32 + 0.5) / scale_factor - 0.5).floor() as i32;
                let src_y = ((dst_y as f32 + 0.5) / scale_factor - 0.5).floor() as i32;

                // Clamp to valid range
                let src_x = src_x.max(0).min(src.width() as i32 - 1) as u32;
                let src_y = src_y.max(0).min(src.height() as i32 - 1) as u32;

                // Copy pixel
                let src_offset = (src_y as usize * src_stride + src_x as usize * 4) as usize;
                let dst_offset = ((dst_y as usize * dst_stride) + dst_x as usize * 4) as usize;

                unsafe {
                    std::ptr::copy_nonoverlapping(
                        src_data.as_ptr().add(src_offset),
                        dst_data.as_mut_ptr().add(dst_offset),
                        4,
                    );
                }
            }
        }

        dst.clear_dirty();
        Ok(dst)
    }
}

impl Scaler for NearestScaler {
    fn scale(&self, src: &Pixmap, params: ScaleParams) -> Result<Pixmap> {
        if params.mode != ScaleMode::Nearest {
            return Err(ScaleError::UnsupportedMode { mode: params.mode }.into());
        }
        Self::scale_nearest(src, params)
    }

    fn supports(&self, mode: ScaleMode) -> bool {
        mode == ScaleMode::Nearest || mode == ScaleMode::Step
    }
}

impl Default for NearestScaler {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// Bilinear Scaler
// ==============================================================================

/// Bilinear interpolation scaling implementation
///
/// Produces smoother results than nearest-neighbor.
/// Corresponds to: `sc2/src/libs/graphics/sdl/bilinear2x.c`
pub struct BilinearScaler;

impl BilinearScaler {
    /// Create a new bilinear scaler
    pub fn new() -> Self {
        Self
    }

    /// Helper function to blend colors
    #[inline]
    fn lerp(a: u8, b: u8, t: f32) -> u8 {
        let val = a as f32 + (b as f32 - a as f32) * t;
        val.round().clamp(0.0, 255.0) as u8
    }

    /// Scale using bilinear interpolation
    fn scale_bilinear(src: &Pixmap, params: ScaleParams) -> Result<Pixmap> {
        if params.scale <= 0 {
            return Err(ScaleError::InvalidScaleFactor {
                factor: params.scale,
            }
            .into());
        }

        let scale_factor = params.scale_factor();
        let dst_width = ((src.width() as f32) * scale_factor).round() as u32;
        let dst_height = ((src.height() as f32) * scale_factor).round() as u32;
        let src_width = src.width();
        let src_height = src.height();

        if dst_width == 0 || dst_height == 0 {
            return Err(ScaleError::InvalidDimensions.into());
        }

        let id =
            NonZeroU32::new(2).ok_or_else(|| anyhow::anyhow!("Failed to generate pixmap ID"))?;
        let mut dst = Pixmap::new(id, dst_width, dst_height, src.format())?;

        if src.format() != PixmapFormat::Rgba32 {
            return Err(ScaleError::FormatMismatch.into());
        }

        let src_data = src.data();
        let dst_data = dst.data_mut();
        let src_stride = src.bytes_per_row() as usize;
        let dst_stride = dst.bytes_per_row() as usize;

        for dst_y in 0..dst_height {
            for dst_x in 0..dst_width {
                let src_x = dst_x as f32 / scale_factor;
                let src_y = dst_y as f32 / scale_factor;

                let x0 = src_x.floor() as u32;
                let y0 = src_y.floor() as u32;
                let x1 = (x0 + 1).min(src_width - 1);
                let y1 = (y0 + 1).min(src_height - 1);

                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                let get_pixel = |x: u32, y: u32| -> [u8; 4] {
                    let offset = (y as usize * src_stride + x as usize * 4) as usize;
                    let p = unsafe { src_data.as_ptr().add(offset) };
                    unsafe { [*p, *p.add(1), *p.add(2), *p.add(3)] }
                };

                let p00 = get_pixel(x0, y0);
                let p10 = get_pixel(x1, y0);
                let p01 = get_pixel(x0, y1);
                let p11 = get_pixel(x1, y1);

                let r1 = Self::lerp(p00[0], p10[0], fx);
                let g1 = Self::lerp(p00[1], p10[1], fx);
                let b1 = Self::lerp(p00[2], p10[2], fx);
                let a1 = Self::lerp(p00[3], p10[3], fx);

                let r2 = Self::lerp(p01[0], p11[0], fx);
                let g2 = Self::lerp(p01[1], p11[1], fx);
                let b2 = Self::lerp(p01[2], p11[2], fx);
                let a2 = Self::lerp(p01[3], p11[3], fx);

                let r = Self::lerp(r1, r2, fy);
                let g = Self::lerp(g1, g2, fy);
                let b = Self::lerp(b1, b2, fy);
                let a = Self::lerp(a1, a2, fy);

                let dst_offset = ((dst_y as usize * dst_stride) + dst_x as usize * 4) as usize;
                unsafe {
                    let dst_ptr = dst_data.as_mut_ptr().add(dst_offset);
                    *dst_ptr = r;
                    *dst_ptr.add(1) = g;
                    *dst_ptr.add(2) = b;
                    *dst_ptr.add(3) = a;
                }
            }
        }

        dst.clear_dirty();
        Ok(dst)
    }
}

impl Scaler for BilinearScaler {
    fn scale(&self, src: &Pixmap, params: ScaleParams) -> Result<Pixmap> {
        if params.mode != ScaleMode::Bilinear {
            return Err(ScaleError::UnsupportedMode { mode: params.mode }.into());
        }
        Self::scale_bilinear(src, params)
    }

    fn supports(&self, mode: ScaleMode) -> bool {
        mode == ScaleMode::Bilinear
    }
}

impl Default for BilinearScaler {
    fn default() -> Self {
        Self::new()
    }
}
