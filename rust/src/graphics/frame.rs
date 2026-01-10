//! Frame management and drawing primitives
//!
//! This module provides Rust abstractions for frame concepts from the
//! original C code (sc2/src/libs/graphics/frame.c, pixmap.c).
//!
//! Key concepts:
//! - Frame: Individual drawable frame with bounds, hot spot, and pixel data
//! - Frame management: Navigation and index manipulation within drawables
//! - Drawing primitives: Points, lines, rectangles, stamps, text
//! - Context: Drawing state with clipping and valid rectangles
//!
//! Note: This module focuses on Frame management only. TFB_Image integration
//! and SDL rendering are handled in separate modules (per Phase 2 scope).

use crate::graphics::drawable::{Drawable, DrawableRegistry};
use crate::graphics::tfb_draw::TFImage;
use anyhow::{Context, Result};
use std::sync::{Arc, RwLock};

/// Re-export drawing types from drawable module
pub use crate::graphics::drawable::{Coord, Extent, HotSpot, Point, Rect};

// ==============================================================================
// Drawing Mode (from drawcmd.h)
//
// Note: This is FrameDrawMode (primitive drawing style), to distinguish from
// context::DrawMode (rendering mode with blending factors).
// ==============================================================================

/// Drawing mode for frame operations (primitive style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameDrawMode {
    /// Replace mode - overwrite destination pixels
    Replace = 0,
    /// Exclusive OR mode
    Xor = 1,
    /// Translucent mode - blend with alpha
    Translucent = 2,
}

/// Color type for drawing operations
pub type Color = u32;

/// Forward declaration of ScaleMode (defined in scaling.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleMode {
    Step = 0,
    Nearest = 1,
    Bilinear = 2,
    Trilinear = 3,
}

// ==============================================================================
// Frame
// ==============================================================================

/// Enhanced Frame wrapper that tracks parent drawable and cached image data
///
/// This extends the basic Frame from drawable.rs with state tracking for
/// frame operations and image data management.
#[derive(Debug, Clone)]
pub struct FrameHandle {
    /// The underlying frame
    frame: crate::graphics::drawable::Frame,
    /// Parent drawable ID
    parent_id: u32,
    /// Cached scale factor (for optimization)
    cached_scale: Option<i32>,
    /// Cached scale mode
    cached_scale_mode: Option<ScaleMode>,
}

impl FrameHandle {
    /// Create a new frame handle from a frame and parent drawable
    pub fn new(frame: crate::graphics::drawable::Frame, parent_id: u32) -> Self {
        Self {
            frame,
            parent_id,
            cached_scale: None,
            cached_scale_mode: None,
        }
    }

    /// Get frame index
    pub fn index(&self) -> usize {
        self.frame.index
    }

    /// Get frame width
    pub fn width(&self) -> u16 {
        self.frame.width()
    }

    /// Get frame height
    pub fn height(&self) -> u16 {
        self.frame.height()
    }

    /// Get frame bounds
    pub fn bounds(&self) -> Extent {
        self.frame.bounds
    }

    /// Get hot spot
    pub fn hot_spot(&self) -> HotSpot {
        self.frame.hot_spot
    }

    /// Set hot spot
    pub fn set_hot_spot(&mut self, hot_spot: HotSpot) {
        self.frame.set_hot_spot(hot_spot);
    }

    /// Get parent drawable ID
    pub fn parent_id(&self) -> u32 {
        self.parent_id
    }

    /// Get effective bounds with hot spot applied
    pub fn effective_rect(&self) -> Rect {
        self.frame.effective_rect()
    }

    /// Get cached scale factor
    pub fn cached_scale(&self) -> Option<i32> {
        self.cached_scale
    }

    /// Set cached scale factor
    pub fn set_cached_scale(&mut self, scale: i32) {
        self.cached_scale = Some(scale);
    }

    /// Clear cached scale
    pub fn clear_cached_scale(&mut self) {
        self.cached_scale = None;
    }

    /// Get cached scale mode
    pub fn cached_scale_mode(&self) -> Option<ScaleMode> {
        self.cached_scale_mode
    }

    /// Set cached scale mode
    pub fn set_cached_scale_mode(&mut self, mode: ScaleMode) {
        self.cached_scale_mode = Some(mode);
    }

    /// Clear cached scale mode
    pub fn clear_cached_scale_mode(&mut self) {
        self.cached_scale_mode = None;
    }

    /// Get optional image associated with the frame.
    pub fn image(&self) -> Option<Arc<TFImage>> {
        self.frame
            .image
            .as_ref()
            .map(|frame_image| Arc::clone(&frame_image.image))
    }

    /// Set image for the frame.
    pub fn set_image(&mut self, image: Arc<TFImage>) {
        self.frame.set_image(image);
    }
}

// ==============================================================================
// Frame Registry (Frame/Pixmap operations)
// ==============================================================================

/// Frame registry for managing frame operations
///
/// This handles frame navigation, index manipulation, and frame retrieval
/// from drawables, matching the functionality from pixmap.c.
#[derive(Debug, Default)]
pub struct FrameRegistry {
    /// Reference to drawable registry
    drawables: Arc<DrawableRegistry>,
    /// Frame handles cache
    frame_cache: RwLock<std::collections::HashMap<(u32, usize), FrameHandle>>,
}

impl FrameRegistry {
    /// Create a new frame registry
    pub fn new(drawables: Arc<DrawableRegistry>) -> Self {
        Self {
            drawables,
            frame_cache: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get parent drawable for a frame
    ///
    /// Corresponds to: `GetFrameParentDrawable (FRAME f)`
    pub fn get_parent_drawable(&self, frame_handle: &FrameHandle) -> Result<Arc<Drawable>> {
        self.drawables
            .get(frame_handle.parent_id())
            .context("Failed to get parent drawable")
    }

    /// Capture drawable to get its first frame
    ///
    /// Corresponds to: `FRAME CaptureDrawable (DRAWABLE DrawablePtr)`
    pub fn capture_drawable(&self, drawable_id: u32) -> Result<FrameHandle> {
        let drawable = self
            .drawables
            .get(drawable_id)
            .context("Failed to get drawable for capture")?;

        // Get frame 0 (first frame)
        let frame = drawable
            .get_frame(0)
            .context("Drawable has no frames")?
            .clone();

        Ok(FrameHandle::new(frame, drawable_id))
    }

    /// Release frame and return parent drawable ID
    ///
    /// Corresponds to: `DRAWABLE ReleaseDrawable (FRAME FramePtr)`
    pub fn release_drawable(&self, frame_handle: &FrameHandle) -> Result<u32> {
        Ok(frame_handle.parent_id())
    }

    /// Get frame count from a drawable
    ///
    /// Corresponds to: `COUNT GetFrameCount (FRAME FramePtr)`
    pub fn get_frame_count(&self, frame_handle: &FrameHandle) -> Result<usize> {
        let drawable = self
            .drawables
            .get(frame_handle.parent_id())
            .context("Failed to get parent drawable")?;

        Ok(drawable.frame_count())
    }

    /// Get frame index
    ///
    /// Corresponds to: `COUNT GetFrameIndex (FRAME FramePtr)`
    pub fn get_frame_index(&self, frame_handle: &FrameHandle) -> usize {
        frame_handle.index()
    }

    /// Set absolute frame index (with wraparound)
    ///
    /// Corresponds to: `FRAME SetAbsFrameIndex (FRAME FramePtr, COUNT FrameIndex)`
    pub fn set_abs_frame_index(&self, frame_handle: &mut FrameHandle, index: usize) -> Result<()> {
        let drawable = self
            .drawables
            .get(frame_handle.parent_id())
            .context("Failed to get parent drawable")?;

        let count = drawable.frame_count();
        if count == 0 {
            return Ok(());
        }

        // Wrap around using modulo
        let wrapped_index = index % count;
        let frame = drawable
            .get_frame(wrapped_index)
            .context("Failed to get frame at absolute index")?
            .clone();

        *frame_handle = FrameHandle::new(frame, frame_handle.parent_id());
        Ok(())
    }

    /// Set relative frame index (with wraparound)
    ///
    /// Corresponds to: `FRAME SetRelFrameIndex (FRAME FramePtr, SIZE FrameOffs)`
    pub fn set_rel_frame_index(&self, frame_handle: &mut FrameHandle, offset: i32) -> Result<()> {
        let drawable = self
            .drawables
            .get(frame_handle.parent_id())
            .context("Failed to get parent drawable")?;

        let count = drawable.frame_count();
        if count == 0 {
            return Ok(());
        }

        // Handle negative offset by adding multiples of count
        let current_index = frame_handle.index();
        let offset = if offset < 0 {
            // Compute positive equivalent
            let abs_offset = offset.unsigned_abs();
            let cycles = abs_offset / count as u32 + 1;
            offset + (cycles as i32) * (count as i32)
        } else {
            offset
        };

        let new_index = (current_index as i32 + offset) as usize % count;
        let frame = drawable
            .get_frame(new_index)
            .context("Failed to get frame at relative index")?
            .clone();

        *frame_handle = FrameHandle::new(frame, frame_handle.parent_id());
        Ok(())
    }

    /// Set frame index equal to another frame's index
    ///
    /// Corresponds to: `FRAME SetEquFrameIndex (FRAME DstFramePtr, FRAME SrcFramePtr)`
    pub fn set_equ_frame_index(
        &self,
        dst_handle: &mut FrameHandle,
        src_handle: &FrameHandle,
    ) -> Result<()> {
        let src_index = src_handle.index();
        self.set_abs_frame_index(dst_handle, src_index)
    }

    /// Increment frame index
    ///
    /// Corresponds to: `FRAME IncFrameIndex (FRAME FramePtr)`
    pub fn inc_frame_index(&self, frame_handle: &mut FrameHandle) -> Result<()> {
        self.set_rel_frame_index(frame_handle, 1)
    }

    /// Decrement frame index
    ///
    /// Corresponds to: `FRAME DecFrameIndex (FRAME FramePtr)`
    pub fn dec_frame_index(&self, frame_handle: &mut FrameHandle) -> Result<()> {
        self.set_rel_frame_index(frame_handle, -1)
    }

    /// Get a frame handle from a drawable
    pub fn get_frame(&self, drawable_id: u32, index: usize) -> Result<FrameHandle> {
        let drawable = self
            .drawables
            .get(drawable_id)
            .context("Failed to get drawable")?;

        let frame = drawable
            .get_frame(index)
            .context("Failed to get frame")?
            .clone();

        Ok(FrameHandle::new(frame, drawable_id))
    }

    /// Cache or retrieve a frame handle for repeated access.
    pub fn get_or_cache_frame(&self, drawable_id: u32, index: usize) -> Result<FrameHandle> {
        if let Some(handle) = self
            .frame_cache
            .read()
            .unwrap()
            .get(&(drawable_id, index))
            .cloned()
        {
            return Ok(handle);
        }

        let handle = self.get_frame(drawable_id, index)?;
        self.frame_cache
            .write()
            .unwrap()
            .insert((drawable_id, index), handle.clone());
        Ok(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::drawable::{DrawableFlags, DrawableType};

    #[test]
    fn test_frame_draw_mode_values() {
        assert_eq!(FrameDrawMode::Replace as u8, 0);
        assert_eq!(FrameDrawMode::Xor as u8, 1);
        assert_eq!(FrameDrawMode::Translucent as u8, 2);
    }

    #[test]
    fn test_frame_handle_creation() {
        let frame =
            crate::graphics::drawable::Frame::with_top_left_hotspot(0, DrawableType::Ram, 64, 32)
                .unwrap();
        let handle = FrameHandle::new(frame, 1);

        assert_eq!(handle.index(), 0);
        assert_eq!(handle.width(), 64);
        assert_eq!(handle.height(), 32);
        assert_eq!(handle.parent_id(), 1);
    }

    #[test]
    fn test_frame_handle_cache() {
        let frame =
            crate::graphics::drawable::Frame::with_top_left_hotspot(0, DrawableType::Ram, 64, 32)
                .unwrap();
        let mut handle = FrameHandle::new(frame, 1);

        assert!(handle.cached_scale().is_none());

        handle.set_cached_scale(512);
        assert_eq!(handle.cached_scale(), Some(512));

        handle.set_cached_scale_mode(ScaleMode::Bilinear);
        assert_eq!(handle.cached_scale_mode(), Some(ScaleMode::Bilinear));

        handle.clear_cached_scale();
        assert!(handle.cached_scale().is_none());
        assert_eq!(handle.cached_scale_mode(), Some(ScaleMode::Bilinear)); // unchanged
    }

    #[test]
    fn test_frame_registry_basic() {
        let drawables = Arc::new(DrawableRegistry::new());
        let registry = FrameRegistry::new(drawables.clone());

        let drawable_id = drawables
            .allocate_with_bounds(
                DrawableType::Ram,
                DrawableFlags::default(),
                3,
                Extent::new(8, 8),
            )
            .unwrap();

        let handle = registry.capture_drawable(drawable_id.get()).unwrap();
        let count = registry.get_frame_count(&handle).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_frame_registry_multi_frame_indices() {
        let drawables = Arc::new(DrawableRegistry::new());
        let registry = FrameRegistry::new(drawables.clone());

        let drawable_id = drawables
            .allocate_with_bounds(
                DrawableType::Ram,
                DrawableFlags::default(),
                3,
                Extent::new(8, 8),
            )
            .unwrap();

        let mut handle = registry.capture_drawable(drawable_id.get()).unwrap();
        assert_eq!(registry.get_frame_index(&handle), 0);

        registry.set_abs_frame_index(&mut handle, 2).unwrap();
        assert_eq!(registry.get_frame_index(&handle), 2);

        registry.set_rel_frame_index(&mut handle, 1).unwrap();
        assert_eq!(registry.get_frame_index(&handle), 0);
    }

    #[test]
    fn test_frame_handle_image_access() {
        let canvas = crate::graphics::tfb_draw::Canvas::new_rgba(4, 4);
        let image = Arc::new(TFImage::new(canvas));
        let frame = crate::graphics::drawable::Frame::from_image(
            0,
            DrawableType::Ram,
            Arc::clone(&image),
            HotSpot::top_left(),
        )
        .unwrap();
        let handle = FrameHandle::new(frame, 1);
        assert!(handle.image().is_some());
    }

    #[test]
    fn test_effective_rect() {
        let frame = crate::graphics::drawable::Frame::new(
            0,
            DrawableType::Ram,
            64,
            32,
            HotSpot::make(32, 16),
        )
        .unwrap();
        let handle = FrameHandle::new(frame, 1);

        let rect = handle.effective_rect();
        assert_eq!(rect.corner.x, -32);
        assert_eq!(rect.corner.y, -16);
        assert_eq!(rect.width(), 64);
        assert_eq!(rect.height(), 32);
    }
}
