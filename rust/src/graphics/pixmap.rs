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
        (self.stride * self.bits_per_pixel + 7) / 8
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

        match self.bytes_per_pixel() {
            4 => {
                let data32 = unsafe {
                    std::slice::from_raw_parts_mut(
                        self.data_mut().as_mut_ptr() as *mut u32,
                        self.data.len() / 4,
                    )
                };
                data32.fill(color);
            }
            2 => {
                let data16 = unsafe {
                    std::slice::from_raw_parts_mut(
                        self.data_mut().as_mut_ptr() as *mut u16,
                        self.data.len() / 2,
                    )
                };
                data16.fill(color as u16);
            }
            _ => {
                // Generic fill
                let color_bytes = color.to_le_bytes();
                for i in (0..self.data.len()).step_by(color_bytes.len()) {
                    for j in 0..color_bytes.len() {
                        if i + j < self.data.len() {
                            self.data_mut()[i + j] = color_bytes[j];
                        }
                    }
                }
            }
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
}
