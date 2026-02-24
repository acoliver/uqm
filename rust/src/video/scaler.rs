//! Video frame scaler using Lanczos3 interpolation
//!
//! Provides high-quality upscaling for DukVid video frames using the
//! `fast_image_resize` crate with SIMD acceleration.
//!
//! # Example
//!
//! ```ignore
//! use uqm_rust::video::scaler::VideoScaler;
//!
//! let mut scaler = VideoScaler::new(280, 200, 1120, 800);
//! let upscaled = scaler.scale(&frame_data)?;
//! ```

use fast_image_resize::{
    images::{Image, ImageRef},
    FilterType, PixelType, ResizeOptions, Resizer,
};

/// Lanczos-based video scaler for direct window presentation
///
/// Extends the base VideoScaler with window-aware scaling to actual window dimensions.
#[derive(Debug)]
pub struct LanczosVideoScaler {
    base_scaler: VideoScaler,
    window_width: u32,
    window_height: u32,
}

impl LanczosVideoScaler {
    /// Creates a new window-aware scaler for video presentation
    ///
    /// # Arguments
    /// * `src_width` - Source video frame width
    /// * `src_height` - Source video frame height
    /// * `window_width` - Actual window width for presentation
    /// * `window_height` - Actual window height for presentation
    pub fn new(src_width: u32, src_height: u32, window_width: u32, window_height: u32) -> Self {
        // Calculate destination dimensions to maintain aspect ratio
        let src_aspect = src_width as f32 / src_height as f32;
        let window_aspect = window_width as f32 / window_height as f32;

        let (dst_width, dst_height) = if src_aspect > window_aspect {
            // Video is wider than window - fit to width
            (window_width, (window_width as f32 / src_aspect) as u32)
        } else {
            // Video is taller than window - fit to height
            ((window_height as f32 * src_aspect) as u32, window_height)
        };

        let base_scaler = VideoScaler::new(src_width, src_height, dst_width, dst_height);

        Self {
            base_scaler,
            window_width,
            window_height,
        }
    }

    /// Scale a video frame to window-appropriate size
    ///
    /// # Arguments
    /// * `src_pixels` - Source pixel data as u32 RGBA values
    ///
    /// # Returns
    /// Scaled pixel data as u32 RGBA values, or None on error
    pub fn scale(&mut self, src_pixels: &[u32]) -> Option<Vec<u32>> {
        self.base_scaler.scale(src_pixels)
    }

    /// Get the destination dimensions after maintaining aspect ratio
    pub fn dst_dimensions(&self) -> (u32, u32) {
        self.base_scaler.dst_dimensions()
    }

    /// Get the window dimensions
    pub fn window_dimensions(&self) -> (u32, u32) {
        (self.window_width, self.window_height)
    }
}

/// Video frame scaler using Lanczos3 interpolation
///
/// Reuses internal buffers for efficient repeated scaling of video frames.
pub struct VideoScaler {
    /// Source width
    src_width: u32,
    /// Source height
    src_height: u32,
    /// Destination width
    dst_width: u32,
    /// Destination height
    dst_height: u32,
    /// Reusable resizer instance
    resizer: Resizer,
    /// Destination image buffer
    dst_image: Image<'static>,
    /// Resize options (Lanczos3)
    options: ResizeOptions,
}

impl VideoScaler {
    /// Creates a new video scaler for the given dimensions
    ///
    /// # Arguments
    ///
    /// * `src_width` - Source frame width
    /// * `src_height` - Source frame height
    /// * `dst_width` - Destination frame width
    /// * `dst_height` - Destination frame height
    pub fn new(src_width: u32, src_height: u32, dst_width: u32, dst_height: u32) -> Self {
        // Create resizer
        let resizer = Resizer::new();

        // Pre-allocate destination buffer
        let dst_image = Image::new(
            dst_width.try_into().unwrap_or(1),
            dst_height.try_into().unwrap_or(1),
            PixelType::U8x4, // RGBA
        );

        // Configure Lanczos3 algorithm
        let options = ResizeOptions::new().resize_alg(fast_image_resize::ResizeAlg::Convolution(
            FilterType::Lanczos3,
        ));

        Self {
            src_width,
            src_height,
            dst_width,
            dst_height,
            resizer,
            dst_image,
            options,
        }
    }

    /// Scales a frame from RGBA u32 pixels to the destination size
    ///
    /// # Arguments
    ///
    /// * `src_pixels` - Source pixel data as u32 RGBA values
    ///
    /// # Returns
    ///
    /// Scaled pixel data as u32 RGBA values, or None on error
    pub fn scale(&mut self, src_pixels: &[u32]) -> Option<Vec<u32>> {
        let expected_len = (self.src_width * self.src_height) as usize;
        if src_pixels.len() != expected_len {
            return None;
        }

        // Convert u32 RGBA to bytes (fast_image_resize expects [u8])
        let src_bytes: Vec<u8> = src_pixels
            .iter()
            .flat_map(|&pixel| {
                // u32 is stored as RGBA in our format
                let r = (pixel & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = ((pixel >> 16) & 0xFF) as u8;
                let a = ((pixel >> 24) & 0xFF) as u8;
                [r, g, b, a]
            })
            .collect();

        // Create source image reference
        let src_image = ImageRef::new(
            self.src_width.try_into().ok()?,
            self.src_height.try_into().ok()?,
            &src_bytes,
            PixelType::U8x4,
        )
        .ok()?;

        // Perform the resize with Lanczos3
        self.resizer
            .resize(&src_image, &mut self.dst_image, &self.options)
            .ok()?;

        // Convert bytes back to u32 RGBA
        let dst_bytes = self.dst_image.buffer();
        let dst_pixels: Vec<u32> = dst_bytes
            .chunks_exact(4)
            .map(|chunk| {
                let r = chunk[0] as u32;
                let g = chunk[1] as u32;
                let b = chunk[2] as u32;
                let a = chunk[3] as u32;
                r | (g << 8) | (b << 16) | (a << 24)
            })
            .collect();

        Some(dst_pixels)
    }

    /// Returns the destination dimensions
    pub fn dst_dimensions(&self) -> (u32, u32) {
        (self.dst_width, self.dst_height)
    }

    /// Returns the source dimensions
    pub fn src_dimensions(&self) -> (u32, u32) {
        (self.src_width, self.src_height)
    }

    /// Returns the scale factor
    pub fn scale_factor(&self) -> f32 {
        self.dst_width as f32 / self.src_width as f32
    }
}

impl std::fmt::Debug for VideoScaler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoScaler")
            .field("src", &format!("{}x{}", self.src_width, self.src_height))
            .field("dst", &format!("{}x{}", self.dst_width, self.dst_height))
            .field("scale", &format!("{:.1}x", self.scale_factor()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaler_creation() {
        let scaler = VideoScaler::new(280, 200, 1120, 800);
        assert_eq!(scaler.src_dimensions(), (280, 200));
        assert_eq!(scaler.dst_dimensions(), (1120, 800));
        assert!((scaler.scale_factor() - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_scaler_scale_2x() {
        let mut scaler = VideoScaler::new(4, 4, 8, 8);

        // Create a simple 4x4 red image
        let src: Vec<u32> = vec![0xFF0000FF; 16]; // Red with full alpha

        let dst = scaler.scale(&src).unwrap();
        assert_eq!(dst.len(), 64); // 8x8

        // Should still be predominantly red
        for pixel in &dst {
            let r = pixel & 0xFF;
            assert!(r > 200, "Red channel should be high");
        }
    }

    #[test]
    fn test_scaler_wrong_input_size() {
        let mut scaler = VideoScaler::new(280, 200, 560, 400);

        // Wrong size input
        let src: Vec<u32> = vec![0; 100];
        assert!(scaler.scale(&src).is_none());
    }

    #[test]
    fn test_scaler_debug() {
        let scaler = VideoScaler::new(280, 200, 1120, 800);
        let debug_str = format!("{:?}", scaler);
        assert!(debug_str.contains("280x200"));
        assert!(debug_str.contains("1120x800"));
        assert!(debug_str.contains("4.0x"));
    }

    #[test]
    fn test_scaler_gradient() {
        let mut scaler = VideoScaler::new(4, 4, 8, 8);

        // Create a gradient (black to white diagonal)
        let mut src = Vec::with_capacity(16);
        for y in 0..4u32 {
            for x in 0..4u32 {
                let v = ((x + y) * 255 / 6) as u8;
                let pixel = (v as u32) | ((v as u32) << 8) | ((v as u32) << 16) | (0xFF << 24);
                src.push(pixel);
            }
        }

        let dst = scaler.scale(&src).unwrap();
        assert_eq!(dst.len(), 64);

        // Lanczos should produce smooth interpolation
        // Check that output has varied values (not all same)
        let unique: std::collections::HashSet<u32> = dst.iter().copied().collect();
        assert!(unique.len() > 4, "Should have interpolated values");
    }
}
