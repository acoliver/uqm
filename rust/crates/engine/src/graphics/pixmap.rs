//! Pixmap and pixel data management
//!
//! This module provides abstractions for pixmap operations inspired by the
//! original C code. It manages pixel buffers for TFB_Image backends.
//!
//! Important notes for Phase 2 scope:
//! - This module does NOT touch SDL, FFI, DCQ, tfb_draw, cmap, or fonts
//! - It provides Rust-native structures for pixel data management
//! - Actual rendering integration will be done in a later phase (tfb_draw module)
//!
//! Key concepts:
//! - Pixmap: Raw pixel data buffer with format information
//! - PixmapFormat: Description of pixel layout (RGBA, BGRA, etc.)
//! - Pixmap operations: Blitting, copying, clipping

use crate::graphics::frame::{Point, Rect};
use anyhow::{Context, Result};
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};

// ==============================================================================
// Pixmap Format
// ==============================================================================

/// Pixel format enumeration
///
/// Describes the byte layout and channel ordering for pixel data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixmapFormat {
    /// 8-bit paletted format (indexed color)
    Indexed8,
    /// 16-bit RGB565 format
    Rgb565,
    /// 16-bit RGBA5551 format
    Rgba5551,
    /// 24-bit RGB format
    Rgb24,
    /// 32-bit RGBA format (most common)
    Rgba32,
    /// 32-bit BGRA format (common on Windows)
    Bgra32,
    /// 32-bit ARGB format (common on macOS)
    Argb32,
    /// 32-bit ABGR format
    Abgr32,
}

impl PixmapFormat {
    /// Get the number of bytes per pixel
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            PixmapFormat::Indexed8 => 1,
            PixmapFormat::Rgb565 => 2,
            PixmapFormat::Rgba5551 => 2,
            PixmapFormat::Rgb24 => 3,
            PixmapFormat::Rgba32 => 4,
            PixmapFormat::Bgra32 => 4,
            PixmapFormat::Argb32 => 4,
            PixmapFormat::Abgr32 => 4,
        }
    }

    /// Check if format has alpha channel
    pub fn has_alpha(&self) -> bool {
        matches!(
            self,
            PixmapFormat::Rgba5551
                | PixmapFormat::Rgba32
                | PixmapFormat::Bgra32
                | PixmapFormat::Argb32
                | PixmapFormat::Abgr32
        )
    }
}

/// Pixel layout description with bit masks
///
/// This provides detailed information about channels for use in software rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixmapLayout {
    /// Number of bits per pixel
    pub bits_per_pixel: u32,
    /// Pixels per stride (number of pixels per row)
    pub stride: u32,
    /// Red channel bitmask
    pub rmask: u32,
    /// Green channel bitmask
    pub gmask: u32,
    /// Blue channel bitmask
    pub bmask: u32,
    /// Alpha channel bitmask
    pub amask: u32,
    /// Red channel shift
    pub rshift: u8,
    /// Green channel shift
    pub gshift: u8,
    /// Blue channel shift
    pub bshift: u8,
    /// Alpha channel shift
    pub ashift: u8,
}

impl PixmapLayout {
    /// Create a pixmap layout from format with default masks
    pub fn from_format(format: PixmapFormat, width: u32) -> Self {
        match format {
            PixmapFormat::Indexed8 => Self {
                bits_per_pixel: 8,
                stride: width,
                rmask: 0,
                gmask: 0,
                bmask: 0,
                amask: 0,
                rshift: 0,
                gshift: 0,
                bshift: 0,
                ashift: 0,
            },
            PixmapFormat::Rgb565 => Self {
                bits_per_pixel: 16,
                stride: width,
                rmask: 0xF800,
                gmask: 0x07E0,
                bmask: 0x001F,
                amask: 0x0000,
                rshift: 11,
                gshift: 5,
                bshift: 0,
                ashift: 0,
            },
            PixmapFormat::Rgba5551 => Self {
                bits_per_pixel: 16,
                stride: width,
                rmask: 0x7C00,
                gmask: 0x03E0,
                bmask: 0x001F,
                amask: 0x8000,
                rshift: 10,
                gshift: 5,
                bshift: 0,
                ashift: 15,
            },
            PixmapFormat::Rgb24 => Self {
                bits_per_pixel: 24,
                stride: width,
                rmask: 0xFF0000,
                gmask: 0x00FF00,
                bmask: 0x0000FF,
                amask: 0x000000,
                rshift: 16,
                gshift: 8,
                bshift: 0,
                ashift: 0,
            },
            PixmapFormat::Rgba32 => Self {
                bits_per_pixel: 32,
                stride: width,
                rmask: 0x000000FF,
                gmask: 0x0000FF00,
                bmask: 0x00FF0000,
                amask: 0xFF000000,
                rshift: 0,
                gshift: 8,
                bshift: 16,
                ashift: 24,
            },
            PixmapFormat::Bgra32 => Self {
                bits_per_pixel: 32,
                stride: width,
                rmask: 0x00FF0000,
                gmask: 0x0000FF00,
                bmask: 0x000000FF,
                amask: 0xFF000000,
                rshift: 16,
                gshift: 8,
                bshift: 0,
                ashift: 24,
            },
            PixmapFormat::Argb32 => Self {
                bits_per_pixel: 32,
                stride: width,
                rmask: 0x00FF0000,
                gmask: 0x0000FF00,
                bmask: 0x000000FF,
                amask: 0xFF000000,
                rshift: 16,
                gshift: 8,
                bshift: 0,
                ashift: 24,
            },
            PixmapFormat::Abgr32 => Self {
                bits_per_pixel: 32,
                stride: width,
                rmask: 0x000000FF,
                gmask: 0x0000FF00,
                bmask: 0x00FF0000,
                amask: 0xFF000000,
                rshift: 0,
                gshift: 8,
                bshift: 16,
                ashift: 24,
            },
        }
    }

    /// Calculate bytes per row
    pub fn bytes_per_row(&self) -> u32 {
        (self.stride * self.bits_per_pixel).div_ceil(8)
    }
}

// ==============================================================================
// Pixmap
// ==============================================================================

/// Errors related to pixmap operations
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PixmapError {
    #[error("Invalid pixmap dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Region out of bounds: {rect:?} in {width}x{height}")]
    RegionOutOfBounds { rect: Rect, width: u32, height: u32 },

    #[error("Format mismatch: expected {expected:?}, got {actual:?}")]
    FormatMismatch {
        expected: PixmapFormat,
        actual: PixmapFormat,
    },

    #[error("Data access error")]
    DataAccessError,
}

/// Raw pixel data buffer
///
/// A Pixmap represents a block of pixel data with a specific format.
/// This is the underlying data structure for TFB_Image's NormalImg.
#[derive(Debug, Clone)]
pub struct Pixmap {
    /// Unique identifier
    id: NonZeroU32,
    /// Pixel format
    format: PixmapFormat,
    /// Pixel layout information
    layout: PixmapLayout,
    /// Image width in pixels
    width: u32,
    /// Image height in pixels
    height: u32,
    /// Raw pixel data (row-major order)
    data: Vec<u8>,
    /// Dirty flag for cache management
    dirty: bool,
}

impl Pixmap {
    /// Create a new pixmap with the specified dimensions and format
    pub fn new(id: NonZeroU32, width: u32, height: u32, format: PixmapFormat) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(PixmapError::InvalidDimensions { width, height }.into());
        }

        let layout = PixmapLayout::from_format(format, width);
        let bytes_per_row = layout.bytes_per_row();
        let total_bytes = bytes_per_row * height;

        Ok(Self {
            id,
            format,
            layout,
            width,
            height,
            data: vec![0; total_bytes as usize],
            dirty: true,
        })
    }

    /// Get pixmap ID
    pub fn id(&self) -> u32 {
        self.id.get()
    }

    /// Get pixmap format
    pub fn format(&self) -> PixmapFormat {
        self.format
    }

    /// Get pixmap layout
    pub fn layout(&self) -> PixmapLayout {
        self.layout
    }

    /// Get width in pixels
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get height in pixels
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get bytes per row
    pub fn bytes_per_row(&self) -> u32 {
        self.layout.bytes_per_row()
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> u32 {
        self.format.bytes_per_pixel()
    }

    /// Check if pixmap has alpha channel
    pub fn has_alpha(&self) -> bool {
        self.format.has_alpha()
    }

    /// Mark pixmap as dirty (needs update)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Check if pixmap is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get raw pixel data (read-only)
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get raw pixel data (mutable)
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.mark_dirty();
        &mut self.data
    }

    /// Get pixel pointer for a specific row
    pub fn row_ptr(&self, y: u32) -> Result<*const u8> {
        if y >= self.height {
            return Err(PixmapError::RegionOutOfBounds {
                rect: Rect::from_xywh(0, y as i16, self.width as u16, 1),
                width: self.width,
                height: self.height,
            }
            .into());
        }
        let offset = (y * self.bytes_per_row()) as usize;
        Ok(unsafe { self.data.as_ptr().add(offset) })
    }

    /// Get mutable pixel pointer for a specific row
    pub fn row_ptr_mut(&mut self, y: u32) -> Result<*mut u8> {
        if y >= self.height {
            return Err(PixmapError::RegionOutOfBounds {
                rect: Rect::from_xywh(0, y as i16, self.width as u16, 1),
                width: self.width,
                height: self.height,
            }
            .into());
        }
        self.mark_dirty();
        let offset = (y * self.bytes_per_row()) as usize;
        Ok(unsafe { self.data.as_mut_ptr().add(offset) })
    }

    /// Fill entire pixmap with a color (for non-indexed formats)
    pub fn fill(&mut self, color: u32) -> Result<()> {
        if matches!(self.format, PixmapFormat::Indexed8) {
            return Err(anyhow::anyhow!("Cannot fill indexed pixmap with color"));
        }

        // Each pixel takes the low bytes of the color; a 24-bit pixel is three
        // bytes, so a fixed four-byte stride would smear the color across pixels.
        let bytes_per_pixel = self.bytes_per_pixel() as usize;
        let color_bytes = color.to_le_bytes();
        for pixel in self.data_mut().chunks_exact_mut(bytes_per_pixel) {
            pixel.copy_from_slice(&color_bytes[..bytes_per_pixel]);
        }
        Ok(())
    }

    /// Copy a region from another pixmap
    pub fn copy_region(&mut self, src: &Pixmap, src_rect: Rect, dst_point: Point) -> Result<()> {
        // Validate source region
        let src_rect_valid = src_rect.corner.x >= 0
            && src_rect.corner.y >= 0
            && (src_rect.corner.x as u32 + src_rect.width() as u32) <= src.width()
            && (src_rect.corner.y as u32 + src_rect.height() as u32) <= src.height();

        if !src_rect_valid {
            return Err(PixmapError::RegionOutOfBounds {
                rect: src_rect,
                width: src.width(),
                height: src.height(),
            }
            .into());
        }

        // Validate destination
        if dst_point.x < 0
            || dst_point.y < 0
            || (dst_point.x as u32 + src_rect.width() as u32) > self.width()
            || (dst_point.y as u32 + src_rect.height() as u32) > self.height()
        {
            return Err(PixmapError::RegionOutOfBounds {
                rect: Rect::from_xywh(
                    dst_point.x,
                    dst_point.y,
                    src_rect.width(),
                    src_rect.height(),
                ),
                width: self.width(),
                height: self.height(),
            }
            .into());
        }

        // For now, only support same-format copying
        if self.format != src.format {
            return Err(PixmapError::FormatMismatch {
                expected: self.format,
                actual: src.format,
            }
            .into());
        }

        // Copy row by row
        let src_bytes_per_pixel = src.bytes_per_pixel() as usize;
        let dst_bytes_per_pixel = self.bytes_per_pixel() as usize;
        let row_bytes = src_rect.width() as usize * src_bytes_per_pixel;

        for y in 0..src_rect.height() {
            let src_y = (src_rect.corner.y + y as i16) as u32;
            let dst_y = (dst_point.y + y as i16) as u32;
            let src_offset = (src_y * src.bytes_per_row()
                + src_rect.corner.x as u32 * src_bytes_per_pixel as u32)
                as usize;
            let dst_offset = (dst_y * self.bytes_per_row()
                + dst_point.x as u32 * dst_bytes_per_pixel as u32)
                as usize;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    src.data().as_ptr().add(src_offset),
                    self.data_mut().as_mut_ptr().add(dst_offset),
                    row_bytes,
                );
            }
        }

        self.mark_dirty();
        Ok(())
    }

    /// Get a sub-region as a new pixmap
    pub fn extract_region(&self, rect: Rect) -> Result<Pixmap> {
        // Validate region
        if rect.corner.x < 0
            || rect.corner.y < 0
            || (rect.corner.x as u32 + rect.width() as u32) > self.width()
            || (rect.corner.y as u32 + rect.height() as u32) > self.height()
        {
            return Err(PixmapError::RegionOutOfBounds {
                rect,
                width: self.width(),
                height: self.height(),
            }
            .into());
        }

        let id = NonZeroU32::new(self.id.get() + 1)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate pixmap ID"))?;
        let mut region = Pixmap::new(id, rect.width() as u32, rect.height() as u32, self.format)?;

        let src_bytes_per_pixel = self.bytes_per_pixel() as usize;
        let row_bytes = rect.width() as usize * src_bytes_per_pixel;

        for y in 0..rect.height() {
            let src_y = (rect.corner.y + y as i16) as u32;
            let src_offset = (src_y * self.bytes_per_row()
                + rect.corner.x as u32 * src_bytes_per_pixel as u32)
                as usize;
            let dst_offset = (y as u32 * region.bytes_per_row()) as usize;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.data().as_ptr().add(src_offset),
                    region.data_mut().as_mut_ptr().add(dst_offset),
                    row_bytes,
                );
            }
        }

        Ok(region)
    }
}

// ==============================================================================
// Pixmap Registry
// ==============================================================================

/// Registry for managing pixmaps
#[derive(Debug, Default)]
pub struct PixmapRegistry {
    /// Pixmaps storage
    pixmaps: RwLock<std::collections::HashMap<NonZeroU32, Pixmap>>,
    /// Next pixmap ID
    next_id: RwLock<u32>,
}

impl PixmapRegistry {
    /// Create a new pixmap registry
    pub fn new() -> Self {
        Self {
            pixmaps: RwLock::new(std::collections::HashMap::new()),
            next_id: RwLock::new(1),
        }
    }

    /// Allocate a new pixmap
    pub fn allocate(&self, width: u32, height: u32, format: PixmapFormat) -> Result<u32> {
        let id = {
            let mut next = self.next_id.write().unwrap();
            let id = *next;
            *next = id.wrapping_add(1);
            if *next == 0 {
                *next = 1;
            }
            NonZeroU32::new(id).ok_or_else(|| anyhow::anyhow!("Failed to allocate pixmap ID"))?
        };

        let pixmap = Pixmap::new(id, width, height, format)?;

        let mut registry = self.pixmaps.write().unwrap();
        registry.insert(id, pixmap);
        Ok(id.get())
    }

    /// Get a pixmap by ID
    pub fn get(&self, id: u32) -> Result<Arc<Pixmap>> {
        let id = NonZeroU32::new(id).context("Invalid pixmap ID: 0")?;
        let registry = self.pixmaps.read().unwrap();
        registry
            .get(&id)
            .map(|p| Arc::new(p.clone()))
            .context("Pixmap not found")
    }

    /// Get a mutable pixmap by ID
    pub fn get_mut(
        &self,
        id: u32,
    ) -> Result<std::sync::RwLockWriteGuard<'_, std::collections::HashMap<NonZeroU32, Pixmap>>>
    {
        let _id = NonZeroU32::new(id).context("Invalid pixmap ID: 0")?;
        self.pixmaps
            .write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))
    }

    /// Release a pixmap
    pub fn release(&self, id: u32) -> Result<()> {
        let id = NonZeroU32::new(id).context("Invalid pixmap ID: 0")?;
        let mut registry = self.pixmaps.write().unwrap();
        registry.remove(&id).map(|_| ()).context("Pixmap not found")
    }

    /// Count active pixmaps
    pub fn count(&self) -> usize {
        self.pixmaps.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixmap_format_bytes_per_pixel() {
        assert_eq!(PixmapFormat::Indexed8.bytes_per_pixel(), 1);
        assert_eq!(PixmapFormat::Rgb565.bytes_per_pixel(), 2);
        assert_eq!(PixmapFormat::Rgba5551.bytes_per_pixel(), 2);
        assert_eq!(PixmapFormat::Rgb24.bytes_per_pixel(), 3);
        assert_eq!(PixmapFormat::Rgba32.bytes_per_pixel(), 4);
        assert_eq!(PixmapFormat::Bgra32.bytes_per_pixel(), 4);
    }

    #[test]
    fn test_pixmap_format_has_alpha() {
        assert!(!PixmapFormat::Indexed8.has_alpha());
        assert!(!PixmapFormat::Rgb565.has_alpha());
        assert!(PixmapFormat::Rgba5551.has_alpha());
        assert!(!PixmapFormat::Rgb24.has_alpha());
        assert!(PixmapFormat::Rgba32.has_alpha());
        assert!(PixmapFormat::Bgra32.has_alpha());
    }

    #[test]
    fn test_pixmap_creation() {
        let id = NonZeroU32::new(1).unwrap();
        let pixmap = Pixmap::new(id, 320, 200, PixmapFormat::Rgba32).unwrap();

        assert_eq!(pixmap.id(), 1);
        assert_eq!(pixmap.width(), 320);
        assert_eq!(pixmap.height(), 200);
        assert_eq!(pixmap.format(), PixmapFormat::Rgba32);
        assert!(pixmap.has_alpha());
        assert!(pixmap.is_dirty());
    }

    #[test]
    fn test_pixmap_invalid_dimensions() {
        let id = NonZeroU32::new(1).unwrap();
        let result = Pixmap::new(id, 0, 100, PixmapFormat::Rgba32);
        assert!(result.is_err());

        let result = Pixmap::new(id, 100, 0, PixmapFormat::Rgba32);
        assert!(result.is_err());
    }

    #[test]
    fn test_pixmap_layout() {
        let layout = PixmapLayout::from_format(PixmapFormat::Rgba32, 320);

        assert_eq!(layout.bits_per_pixel, 32);
        assert_eq!(layout.stride, 320);
        assert_eq!(layout.rmask, 0x000000FF);
        assert_eq!(layout.gmask, 0x0000FF00);
        assert_eq!(layout.bmask, 0x00FF0000);
        assert_eq!(layout.amask, 0xFF000000);
    }

    #[test]
    fn test_pixmap_fill() {
        let id = NonZeroU32::new(1).unwrap();
        let mut pixmap = Pixmap::new(id, 10, 10, PixmapFormat::Rgba32).unwrap();

        pixmap.fill(0xFF00FF00).unwrap();
        assert!(pixmap.is_dirty());

        pixmap.clear_dirty();
        assert!(!pixmap.is_dirty());
    }

    #[test]
    fn test_pixmap_registry() {
        let registry = PixmapRegistry::new();

        let id = registry.allocate(64, 64, PixmapFormat::Rgba32).unwrap();
        assert_eq!(registry.count(), 1);

        let pixmap = registry.get(id).unwrap();
        assert_eq!(pixmap.width(), 64);
        assert_eq!(pixmap.height(), 64);

        registry.release(id).unwrap();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_pixmap_copy_region() {
        let id1 = NonZeroU32::new(1).unwrap();
        let id2 = NonZeroU32::new(2).unwrap();

        let mut src = Pixmap::new(id1, 100, 100, PixmapFormat::Rgba32).unwrap();
        let mut dst = Pixmap::new(id2, 100, 100, PixmapFormat::Rgba32).unwrap();

        src.fill(0xFFFF0000).unwrap();

        let src_rect = Rect::from_xywh(10, 10, 50, 50);
        let dst_point = Point::new(25, 25);

        dst.copy_region(&src, src_rect, dst_point).unwrap();

        // Clear dirty flag
        dst.clear_dirty();
        assert!(!dst.is_dirty());
    }

    #[test]
    fn test_pixmap_extract_region() {
        let id = NonZeroU32::new(1).unwrap();
        let pixmap = Pixmap::new(id, 100, 100, PixmapFormat::Rgba32).unwrap();

        let rect = Rect::from_xywh(10, 20, 30, 40);
        let region = pixmap.extract_region(rect).unwrap();

        assert_eq!(region.width(), 30);
        assert_eq!(region.height(), 40);
        assert_eq!(region.format(), PixmapFormat::Rgba32);
    }

    #[test]
    fn test_region_out_of_bounds() {
        let id = NonZeroU32::new(1).unwrap();
        let pixmap = Pixmap::new(id, 100, 100, PixmapFormat::Rgba32).unwrap();

        // Region partially outside
        let rect = Rect::from_xywh(90, 90, 20, 20);
        let result = pixmap.extract_region(rect);
        assert!(result.is_err());

        // Region completely outside
        let rect = Rect::from_xywh(150, 150, 10, 10);
        let result = pixmap.extract_region(rect);
        assert!(result.is_err());
    }

    const ALL_FORMATS: [PixmapFormat; 8] = [
        PixmapFormat::Indexed8,
        PixmapFormat::Rgb565,
        PixmapFormat::Rgba5551,
        PixmapFormat::Rgb24,
        PixmapFormat::Rgba32,
        PixmapFormat::Bgra32,
        PixmapFormat::Argb32,
        PixmapFormat::Abgr32,
    ];

    fn pixmap(width: u32, height: u32, format: PixmapFormat) -> Pixmap {
        Pixmap::new(NonZeroU32::new(1).unwrap(), width, height, format).unwrap()
    }

    #[test]
    fn layouts_describe_disjoint_contiguous_channels_that_fit_the_pixel() {
        for format in ALL_FORMATS {
            let layout = PixmapLayout::from_format(format, 7);
            assert_eq!(
                layout.bits_per_pixel,
                format.bytes_per_pixel() * 8,
                "{format:?}"
            );
            assert_eq!(layout.stride, 7);
            assert_eq!(layout.bytes_per_row(), 7 * format.bytes_per_pixel());

            let masks = [layout.rmask, layout.gmask, layout.bmask, layout.amask];
            let shifts = [layout.rshift, layout.gshift, layout.bshift, layout.ashift];
            let mut seen = 0_u32;
            for (mask, shift) in masks.into_iter().zip(shifts) {
                if mask == 0 {
                    continue;
                }
                assert_eq!(seen & mask, 0, "{format:?} channels overlap");
                seen |= mask;
                // The shift lands the channel at bit zero as a run of ones.
                let channel = mask >> shift;
                assert_eq!(
                    mask,
                    channel << shift,
                    "{format:?} shift misplaces its mask"
                );
                assert_eq!(
                    channel & (channel + 1),
                    0,
                    "{format:?} channel is not contiguous"
                );
                assert!(
                    mask < (1_u64 << layout.bits_per_pixel) as u32 || layout.bits_per_pixel == 32
                );
            }
            assert_eq!(layout.amask != 0, format.has_alpha(), "{format:?}");
            if format == PixmapFormat::Indexed8 {
                assert_eq!(seen, 0, "palette pixels carry no channel masks");
            }
        }
    }

    #[test]
    fn rows_round_up_to_whole_bytes() {
        let mut layout = PixmapLayout::from_format(PixmapFormat::Rgb565, 3);
        assert_eq!(layout.bytes_per_row(), 6);
        layout.bits_per_pixel = 12;
        assert_eq!(layout.bytes_per_row(), 5, "36 bits need five bytes");
    }

    #[test]
    fn filling_writes_every_pixel_with_the_low_bytes_of_the_color() {
        let color = 0x44_33_22_11_u32;
        for format in ALL_FORMATS {
            let mut pixmap = pixmap(3, 2, format);
            let bytes_per_pixel = format.bytes_per_pixel() as usize;
            if format == PixmapFormat::Indexed8 {
                assert!(
                    pixmap.fill(color).is_err(),
                    "a palette index is not a color"
                );
                continue;
            }
            pixmap.fill(color).unwrap();
            let expected: Vec<u8> = color.to_le_bytes()[..bytes_per_pixel].to_vec();
            for (index, pixel) in pixmap.data().chunks_exact(bytes_per_pixel).enumerate() {
                assert_eq!(pixel, expected.as_slice(), "{format:?} pixel {index}");
            }
            assert_eq!(pixmap.data().len(), 6 * bytes_per_pixel, "{format:?}");
            assert!(pixmap.is_dirty());
        }
    }

    #[test]
    fn row_pointers_address_whole_rows_and_refuse_rows_past_the_end() {
        let mut pixmap = pixmap(4, 3, PixmapFormat::Rgba32);
        let base = pixmap.data().as_ptr();
        assert_eq!(pixmap.row_ptr(0).unwrap(), base);
        assert_eq!(pixmap.row_ptr(2).unwrap(), unsafe { base.add(2 * 16) });
        let error = pixmap.row_ptr(3).unwrap_err();
        assert!(
            error.to_string().contains("Region out of bounds"),
            "{error}"
        );

        pixmap.clear_dirty();
        let row = pixmap.row_ptr_mut(1).unwrap();
        unsafe { row.write(0xAB) };
        assert_eq!(pixmap.data()[16], 0xAB);
        assert!(
            pixmap.is_dirty(),
            "taking a writable row marks the pixmap dirty"
        );
        assert!(pixmap.row_ptr_mut(3).is_err());
    }

    #[test]
    fn copying_a_region_moves_exactly_those_pixels() {
        let mut src = pixmap(4, 4, PixmapFormat::Rgba32);
        for (index, byte) in src.data_mut().iter_mut().enumerate() {
            *byte = index as u8;
        }
        let mut dst = pixmap(4, 4, PixmapFormat::Rgba32);
        dst.copy_region(&src, Rect::from_xywh(1, 1, 2, 2), Point::new(0, 2))
            .unwrap();

        let pixel = |pixmap: &Pixmap, x: usize, y: usize| -> [u8; 4] {
            let at = y * 16 + x * 4;
            pixmap.data()[at..at + 4].try_into().unwrap()
        };
        assert_eq!(pixel(&dst, 0, 2), pixel(&src, 1, 1));
        assert_eq!(pixel(&dst, 1, 2), pixel(&src, 2, 1));
        assert_eq!(pixel(&dst, 0, 3), pixel(&src, 1, 2));
        assert_eq!(pixel(&dst, 1, 3), pixel(&src, 2, 2));
        let untouched = dst
            .data()
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                let (x, y) = ((index % 16) / 4, index / 16);
                !(x < 2 && y >= 2)
            })
            .all(|(_, byte)| *byte == 0);
        assert!(untouched, "pixels outside the destination block stay empty");
    }

    #[test]
    fn copying_refuses_regions_outside_either_pixmap_and_mixed_formats() {
        let src = pixmap(4, 4, PixmapFormat::Rgba32);
        let mut dst = pixmap(4, 4, PixmapFormat::Rgba32);
        let whole = Rect::from_xywh(0, 0, 4, 4);

        for (rect, point) in [
            (Rect::from_xywh(-1, 0, 2, 2), Point::new(0, 0)),
            (Rect::from_xywh(0, -1, 2, 2), Point::new(0, 0)),
            (Rect::from_xywh(3, 0, 2, 2), Point::new(0, 0)),
            (Rect::from_xywh(0, 3, 2, 2), Point::new(0, 0)),
            (whole, Point::new(-1, 0)),
            (whole, Point::new(0, -1)),
            (whole, Point::new(1, 0)),
            (whole, Point::new(0, 1)),
        ] {
            let error = dst.copy_region(&src, rect, point).unwrap_err();
            assert!(
                error.to_string().contains("Region out of bounds"),
                "{rect:?} -> {point:?}: {error}"
            );
        }

        let other_format = pixmap(4, 4, PixmapFormat::Rgb565);
        let error = dst
            .copy_region(&other_format, Rect::from_xywh(0, 0, 2, 2), Point::new(0, 0))
            .unwrap_err();
        assert!(error.to_string().contains("Format mismatch"), "{error}");
        assert!(dst.data().iter().all(|byte| *byte == 0));
    }

    #[test]
    fn an_extracted_region_holds_the_source_pixels_under_a_new_id() {
        let mut source = pixmap(5, 4, PixmapFormat::Rgb565);
        for (index, byte) in source.data_mut().iter_mut().enumerate() {
            *byte = index as u8;
        }
        let region = source.extract_region(Rect::from_xywh(1, 1, 2, 2)).unwrap();
        assert_eq!((region.width(), region.height()), (2, 2));
        assert_eq!(region.id(), source.id() + 1);
        // Source rows are 10 bytes; the region starts at pixel (1, 1) = byte 12.
        assert_eq!(region.data(), [12, 13, 14, 15, 22, 23, 24, 25]);
    }

    #[test]
    fn a_pixmap_needs_pixels() {
        for (width, height) in [(0, 4), (4, 0), (0, 0)] {
            let error = Pixmap::new(
                NonZeroU32::new(1).unwrap(),
                width,
                height,
                PixmapFormat::Rgba32,
            )
            .unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("Invalid pixmap dimensions: {width}x{height}")
            );
        }
    }

    #[test]
    fn the_registry_hands_out_sequential_ids_and_rejects_unknown_ones() {
        let registry = PixmapRegistry::new();
        let first = registry.allocate(2, 2, PixmapFormat::Rgba32).unwrap();
        let second = registry.allocate(3, 3, PixmapFormat::Rgb565).unwrap();
        assert_eq!((first, second), (1, 2));
        assert_eq!(registry.get(second).unwrap().width(), 3);

        assert!(registry.allocate(0, 2, PixmapFormat::Rgba32).is_err());
        assert_eq!(
            registry.count(),
            2,
            "a refused allocation leaves nothing behind"
        );

        let error = registry.get(0).unwrap_err();
        assert!(
            error.to_string().contains("Invalid pixmap ID: 0"),
            "{error}"
        );
        assert!(registry
            .get(99)
            .unwrap_err()
            .to_string()
            .contains("not found"));
        assert!(registry.get_mut(0).is_err());
        assert!(registry.release(0).is_err());
        assert!(registry.release(99).is_err());

        registry.release(first).unwrap();
        assert!(registry.get(first).is_err());
        assert!(
            registry.release(first).is_err(),
            "a pixmap is released once"
        );
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn the_registry_returns_copies_so_callers_cannot_alter_stored_pixels() {
        let registry = PixmapRegistry::new();
        let id = registry.allocate(2, 1, PixmapFormat::Rgba32).unwrap();
        let handle = registry.get(id).unwrap();
        let mut local = (*handle).clone();
        local.fill(0xFFFF_FFFF).unwrap();
        assert!(registry
            .get(id)
            .unwrap()
            .data()
            .iter()
            .all(|byte| *byte == 0));
    }
}
