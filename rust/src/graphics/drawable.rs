//! Drawable and Frame management
//!
//! This module provides Rust abstractions for the drawable and frame
//! concepts from the original C code (libs/graphics/drawable.h).
//!
//! Key concepts:
//! - DrawableType: ROM_DRAWABLE, RAM_DRAWABLE, SCREEN_DRAWABLE
//! - Drawable: Container for multiple frames with memory flags
//! - Frame: Individual drawable frame with bounds and hot spot

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};

use crate::graphics::tfb_draw::{Canvas, CanvasFormat, TFImage};

/// Drawable type classification (from DRAWABLE_TYPE)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawableType {
    /// Read-only memory (e.g., ROM or loaded assets)
    Rom,
    /// Read-write memory (e.g., created frames)
    Ram,
    /// Screen buffer
    Screen,
}

impl From<u16> for DrawableType {
    fn from(value: u16) -> Self {
        match value {
            0 => DrawableType::Rom,
            1 => DrawableType::Ram,
            2 => DrawableType::Screen,
            _ => DrawableType::Ram, // Default fallback
        }
    }
}

impl From<DrawableType> for u16 {
    fn from(dt: DrawableType) -> Self {
        match dt {
            DrawableType::Rom => 0,
            DrawableType::Ram => 1,
            DrawableType::Screen => 2,
        }
    }
}

/// Drawable creation flags (from CREATE_FLAGS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawableFlags {
    pub want_pixmap: bool,
    pub want_alpha: bool,
    /// Mapped to display flag (legacy compatibility)
    pub mapped_to_display: bool,
}

/// Represents a coordinate in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coord {
    pub value: i16,
}

impl Coord {
    pub fn new(value: i16) -> Self {
        Self { value }
    }

    /// Convert from COORD typedef (SWORD)
    pub fn from_sword(value: u16) -> Self {
        Self {
            value: value as i16,
        }
    }

    pub fn to_i32(&self) -> i32 {
        self.value as i32
    }
}

/// 2D Size (width, height)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub width: u16,
    pub height: u16,
}

impl Extent {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// Create from COORD-based SIZE typedef
    pub fn from_size(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> u32 {
        u32::from(self.width) * u32::from(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// 2D point (x, y)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub fn from_coords(x: Coord, y: Coord) -> Self {
        Self {
            x: x.value,
            y: y.value,
        }
    }

    pub fn offset(&self, dx: i16, dy: i16) -> Point {
        Point::new(self.x + dx, self.y + dy)
    }
}

/// Hot spot offset for drawing (anchor point)
///
/// The hot spot defines where the frame "originates" when drawing.
/// For example, (0,0) means top-left is the origin, while (-width/2, -height/2)
/// would center the frame on the draw point.
pub type HotSpot = Point;

/// Helper functions for creating hot spots
impl HotSpot {
    /// Create a hot spot directly
    pub fn make(x: i16, y: i16) -> Self {
        Point::new(x, y)
    }

    /// Top-left hot spot (default)
    pub fn top_left() -> Self {
        Point::new(0, 0)
    }

    /// Centered hot spot
    pub fn centered(extent: Extent) -> Self {
        Point::new((extent.width as i16) / 2, (extent.height as i16) / 2)
    }

    /// Bottom-right hot spot
    pub fn bottom_right(extent: Extent) -> Self {
        Point::new(extent.width as i16, extent.height as i16)
    }
}

/// Bounding rectangle (corner + extent)
///
/// This represents the geometric bounds of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub corner: Point,
    pub extent: Extent,
}

impl Rect {
    pub fn new(corner: Point, width: u16, height: u16) -> Self {
        Self {
            corner,
            extent: Extent::new(width, height),
        }
    }

    pub fn from_point_extent(corner: Point, extent: Extent) -> Self {
        Self { corner, extent }
    }

    pub fn from_xywh(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self {
            corner: Point::new(x, y),
            extent: Extent::new(width, height),
        }
    }

    pub fn width(&self) -> u16 {
        self.extent.width
    }

    pub fn height(&self) -> u16 {
        self.extent.height
    }

    /// Get the right edge x-coordinate (exclusive)
    pub fn right(&self) -> i32 {
        self.corner.x as i32 + self.extent.width as i32
    }

    /// Get the bottom edge y-coordinate (exclusive)
    pub fn bottom(&self) -> i32 {
        self.corner.y as i32 + self.extent.height as i32
    }

    /// Check if a point is within the rectangle
    pub fn contains(&self, point: Point) -> bool {
        (point.x as i32) >= (self.corner.x as i32)
            && (point.y as i32) >= (self.corner.y as i32)
            && (point.x as i32) < self.right()
            && (point.y as i32) < self.bottom()
    }

    /// Check if this rectangle intersects with another
    pub fn intersects(&self, other: &Rect) -> bool {
        self.right() > (other.corner.x as i32)
            && self.bottom() > (other.corner.y as i32)
            && (self.corner.x as i32) < other.right()
            && (self.corner.y as i32) < other.bottom()
    }

    /// Get the intersection of two rectangles (if any)
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x1 = self.corner.x.max(other.corner.x);
        let y1 = self.corner.y.max(other.corner.y);
        let x2 = self.right().min(other.right()) as i16;
        let y2 = self.bottom().min(other.bottom()) as i16;

        Some(Rect::new(
            Point::new(x1, y1),
            (x2 - x1) as u16,
            (y2 - y1) as u16,
        ))
    }
}

/// Errors related to Frame operations
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("Invalid frame index: {index} (max: {max_index})")]
    InvalidFrameIndex { index: usize, max_index: usize },

    #[error("Frame not initialized")]
    NotInitialized,

    #[error("Parent drawable already freed")]
    ParentFreed,

    #[error("Bounds size must be positive")]
    InvalidBounds,
}

/// Image data stored on a frame.
#[derive(Debug, Clone)]
pub struct FrameImage {
    pub image: Arc<TFImage>,
}

impl FrameImage {
    pub fn new(image: Arc<TFImage>) -> Self {
        Self { image }
    }
}


/// Frame representation
///
/// A Frame represents a single image frame within a Drawable.
/// It contains bounds, hot spot information, a reference to its parent,
/// and optional image data for RAM-backed drawables.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Unique frame identifier within its drawable
    pub index: usize,
    /// Drawable type (ROM, RAM, or SCREEN)
    pub frame_type: DrawableType,
    /// Bounding rectangle
    pub bounds: Extent,
    /// Hot spot offset
    pub hot_spot: HotSpot,
    /// Optional image data for this frame
    pub image: Option<FrameImage>,
    /// Reference to parent drawable
    pub parent: Option<NonZeroU32>,
}

impl Frame {
    /// Create a new frame
    pub fn new(
        index: usize,
        frame_type: DrawableType,
        width: u16,
        height: u16,
        hot_spot: HotSpot,
    ) -> Result<Self, FrameError> {
        if width == 0 || height == 0 {
            return Err(FrameError::InvalidBounds);
        }

        Ok(Self {
            index,
            frame_type,
            bounds: Extent::new(width, height),
            hot_spot,
            image: None,
            parent: None,
        })
    }

    /// Create a frame with top-left hot spot
    pub fn with_top_left_hotspot(
        index: usize,
        frame_type: DrawableType,
        width: u16,
        height: u16,
    ) -> Result<Self, FrameError> {
        Self::new(index, frame_type, width, height, HotSpot::top_left())
    }

    /// Create a frame backed by an existing image
    pub fn from_image(
        index: usize,
        frame_type: DrawableType,
        image: Arc<TFImage>,
        hot_spot: HotSpot,
    ) -> Result<Self, FrameError> {
        let extent = image
            .extent()
            .ok_or(FrameError::InvalidBounds)?;
        if extent.width <= 0 || extent.height <= 0 {
            return Err(FrameError::InvalidBounds);
        }

        let mut frame = Self::new(
            index,
            frame_type,
            extent.width as u16,
            extent.height as u16,
            hot_spot,
        )?;
        frame.set_image(image);
        Ok(frame)
    }

    /// Get frame width
    pub fn width(&self) -> u16 {
        self.bounds.width
    }

    /// Get frame height
    pub fn height(&self) -> u16 {
        self.bounds.height
    }

    /// Get frame bounds as a Rect at origin
    pub fn as_rect(&self) -> Rect {
        Rect::new(Point::new(0, 0), self.bounds.width, self.bounds.height)
    }

    /// Calculate the effective bounds with hot spot applied
    pub fn effective_rect(&self) -> Rect {
        Rect::new(
            Point::new(-self.hot_spot.x, -self.hot_spot.y),
            self.bounds.width,
            self.bounds.height,
        )
    }

    /// Set hot spot
    pub fn set_hot_spot(&mut self, hot_spot: HotSpot) {
        self.hot_spot = hot_spot;
    }

    /// Set parent drawable id
    pub fn set_parent(&mut self, parent_id: NonZeroU32) {
        self.parent = Some(parent_id);
    }

    /// Clear parent reference
    pub fn clear_parent(&mut self) {
        self.parent = None;
    }

    /// Attach an image to this frame
    pub fn set_image(&mut self, image: Arc<TFImage>) {
        self.image = Some(FrameImage::new(image));
    }
}

/// Drawable container
///
/// A Drawable holds a collection of frames, typically representing an
/// animated graphic or multi-state element. The original C code uses
/// this for managing sprite frames and image sequences.
#[derive(Debug, Clone)]
pub struct Drawable {
    /// Unique identifier for this drawable
    id: NonZeroU32,
    /// Drawable type
    drawable_type: DrawableType,
    /// Creation flags
    flags: DrawableFlags,
    /// Frames collection
    frames: Vec<Frame>,
    /// Maximum frame index (capacity)
    max_index: usize,
    /// Default frame size for newly allocated frames
    default_bounds: Extent,
}

impl Drawable {
    /// Create a new drawable with empty frame slots
    pub fn new(
        id: NonZeroU32,
        drawable_type: DrawableType,
        flags: DrawableFlags,
        num_frames: usize,
    ) -> Self {
        Self::with_bounds(id, drawable_type, flags, num_frames, Extent::new(0, 0))
    }

    /// Create a new drawable with preset frame bounds
    pub fn with_bounds(
        id: NonZeroU32,
        drawable_type: DrawableType,
        flags: DrawableFlags,
        num_frames: usize,
        bounds: Extent,
    ) -> Self {
        Self {
            id,
            drawable_type,
            flags,
            frames: Vec::with_capacity(num_frames),
            max_index: num_frames.saturating_sub(1),
            default_bounds: bounds,
        }
    }

    /// Add a frame to this drawable
    pub fn add_frame(&mut self, frame: Frame) -> Result<(), FrameError> {
        let frame_index = frame.index;
        if frame_index > self.max_index {
            return Err(FrameError::InvalidFrameIndex {
                index: frame_index,
                max_index: self.max_index,
            });
        }

        // Link frame to parent
        let mut linked_frame = frame;
        linked_frame.set_parent(self.id);

        self.ensure_frames_initialized();
        self.frames[frame_index] = linked_frame;
        Ok(())
    }

    /// Ensure all frame slots are initialized
    fn ensure_frames_initialized(&mut self) {
        if self.frames.len() == self.max_index + 1 {
            return;
        }

        let frame_type = self.drawable_type;
        let bounds = self.default_bounds;
        let flags = self.flags;

        while self.frames.len() <= self.max_index {
            let index = self.frames.len();
            let mut frame = Frame {
                index,
                frame_type,
                bounds,
                hot_spot: HotSpot::top_left(),
                image: None,
                parent: Some(self.id),
            };

            if matches!(frame_type, DrawableType::Ram) && bounds.width > 0 && bounds.height > 0 {
                let canvas = Canvas::new(
                    crate::graphics::dcqueue::Extent::new(
                        bounds.width as i32,
                        bounds.height as i32,
                    ),
                    if flags.want_alpha {
                        CanvasFormat::rgba()
                    } else {
                        CanvasFormat::rgb()
                    },
                );
                let image = Arc::new(TFImage::new(canvas));
                frame.set_image(image);
            }

            self.frames.push(frame);
        }
    }

    /// Get a frame by index
    pub fn get_frame(&self, index: usize) -> Result<&Frame, FrameError> {
        if index > self.max_index {
            return Err(FrameError::InvalidFrameIndex {
                index,
                max_index: self.max_index,
            });
        }

        if self.frames.is_empty() {
            return Err(FrameError::NotInitialized);
        }

        let frame = self.frames.get(index).ok_or(FrameError::NotInitialized)?;
        if frame.parent.is_none() {
            return Err(FrameError::NotInitialized);
        }

        Ok(frame)
    }

    /// Get a mutable frame by index
    pub fn get_frame_mut(&mut self, index: usize) -> Result<&mut Frame, FrameError> {
        if index > self.max_index {
            return Err(FrameError::InvalidFrameIndex {
                index,
                max_index: self.max_index,
            });
        }

        self.ensure_frames_initialized();
        let frame = self
            .frames
            .get_mut(index)
            .ok_or(FrameError::NotInitialized)?;
        if frame.parent.is_none() {
            return Err(FrameError::NotInitialized);
        }

        Ok(frame)
    }

    /// Get drawable ID
    pub fn id(&self) -> u32 {
        self.id.get()
    }

    /// Get drawable type
    pub fn drawable_type(&self) -> DrawableType {
        self.drawable_type
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.max_index + 1
    }

    /// Get total capacity (max index + 1)
    pub fn capacity(&self) -> usize {
        self.max_index + 1
    }

    /// Get flags
    pub fn flags(&self) -> DrawableFlags {
        self.flags
    }

    /// Check if drawable requires alpha channel
    pub fn has_alpha(&self) -> bool {
        self.flags.want_alpha
    }
}

/// Drawable registry for managing multiple drawables
///
/// This provides centralized management of all drawables in the system,
/// similar to how the C code uses global state combined with memory management.
#[derive(Debug, Default)]
pub struct DrawableRegistry {
    drawables: RwLock<HashMap<NonZeroU32, Drawable>>,
    next_id: RwLock<u32>,
}

impl DrawableRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            drawables: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
        }
    }

    /// Allocate a new drawable
    pub fn allocate(
        &self,
        ty: DrawableType,
        flags: DrawableFlags,
        num_frames: usize,
    ) -> Result<NonZeroU32> {
        self.allocate_with_bounds(ty, flags, num_frames, Extent::new(0, 0))
    }

    /// Allocate a new drawable with preset frame bounds
    pub fn allocate_with_bounds(
        &self,
        ty: DrawableType,
        flags: DrawableFlags,
        num_frames: usize,
        bounds: Extent,
    ) -> Result<NonZeroU32> {
        let id = {
            let mut next = self.next_id.write().unwrap();
            let id = *next;
            // Increment, skipping 0
            *next = id.wrapping_add(1);
            if *next == 0 {
                *next = 1;
            }
            NonZeroU32::new(id).context("Failed to allocate drawable ID")?
        };

        let drawable = Drawable::with_bounds(id, ty, flags, num_frames, bounds);

        let mut registry = self.drawables.write().unwrap();
        registry.insert(id, drawable);

        Ok(id)
    }

    /// Get a drawable by ID
    pub fn get(&self, id: u32) -> Result<Arc<Drawable>> {
        let id = NonZeroU32::new(id).context("Invalid drawable ID: 0")?;
        let mut registry = self.drawables.write().unwrap();
        let drawable = registry.get_mut(&id).context("Drawable not found")?;
        drawable.ensure_frames_initialized();

        // Clone via Arc for safe concurrent access
        Ok(Arc::new(drawable.clone()))
    }

    /// Release a drawable
    pub fn release(&self, id: u32) -> Result<()> {
        let id = NonZeroU32::new(id).context("Invalid drawable ID: 0")?;
        let mut registry = self.drawables.write().unwrap();
        registry
            .remove(&id)
            .map(|_| ())
            .context("Drawable not found")
    }

    /// Get frame from a drawable
    pub fn get_frame(&self, drawable_id: u32, frame_index: usize) -> Result<Frame> {
        let id = NonZeroU32::new(drawable_id).context("Invalid drawable ID: 0")?;
        let mut registry = self.drawables.write().unwrap();
        let drawable = registry.get_mut(&id).context("Drawable not found")?;
        drawable.ensure_frames_initialized();
        Ok(drawable.get_frame(frame_index)?.clone())
    }

    /// Count active drawables
    pub fn count(&self) -> usize {
        self.drawables.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawable_type_conversion() {
        assert_eq!(DrawableType::from(0u16), DrawableType::Rom);
        assert_eq!(DrawableType::from(1u16), DrawableType::Ram);
        assert_eq!(DrawableType::from(2u16), DrawableType::Screen);
        assert_eq!(DrawableType::from(99u16), DrawableType::Ram); // fallback

        assert_eq!(u16::from(DrawableType::Rom), 0);
        assert_eq!(u16::from(DrawableType::Ram), 1);
        assert_eq!(u16::from(DrawableType::Screen), 2);
    }

    #[test]
    fn test_extent_creation() {
        let extent = Extent::new(320, 200);
        assert_eq!(extent.width, 320);
        assert_eq!(extent.height, 200);
        assert_eq!(extent.area(), 320 * 200);
        assert!(!extent.is_empty());

        let empty = Extent::new(0, 100);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_point_operations() {
        let p1 = Point::new(10, 20);
        let p2 = p1.offset(5, -3);
        assert_eq!(p2.x, 15);
        assert_eq!(p2.y, 17);
    }

    #[test]
    fn test_hot_spot_variants() {
        let extent = Extent::new(64, 32);

        let top_left = HotSpot::top_left();
        assert_eq!(top_left.x, 0);
        assert_eq!(top_left.y, 0);

        let centered = HotSpot::centered(extent);
        assert_eq!(centered.x, 32);
        assert_eq!(centered.y, 16);

        let bottom_right = HotSpot::bottom_right(extent);
        assert_eq!(bottom_right.x, 64);
        assert_eq!(bottom_right.y, 32);
    }

    #[test]
    fn test_rect_operations() {
        let r1 = Rect::from_xywh(10, 10, 100, 50);

        assert_eq!(r1.width(), 100);
        assert_eq!(r1.height(), 50);
        assert_eq!(r1.right(), 110);
        assert_eq!(r1.bottom(), 60);

        assert!(r1.contains(Point::new(10, 10)));
        assert!(r1.contains(Point::new(50, 30)));
        assert!(!r1.contains(Point::new(110, 10)));
        assert!(!r1.contains(Point::new(10, 60)));

        let r2 = Rect::from_xywh(50, 30, 100, 50);
        assert!(r1.intersects(&r2));

        let rect_outside = Rect::from_xywh(200, 200, 10, 10);
        assert!(!r1.intersects(&rect_outside));
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::from_xywh(0, 0, 100, 100);
        let r2 = Rect::from_xywh(50, 50, 100, 100);

        let intersection = r1.intersection(&r2);
        assert!(intersection.is_some());
        let rect = intersection.unwrap();
        assert_eq!(rect.corner.x, 50);
        assert_eq!(rect.corner.y, 50);
        assert_eq!(rect.width(), 50);
        assert_eq!(rect.height(), 50);
    }

    #[test]
    fn test_frame_creation() {
        let frame = Frame::with_top_left_hotspot(0, DrawableType::Ram, 64, 64).unwrap();

        assert_eq!(frame.index, 0);
        assert_eq!(frame.width(), 64);
        assert_eq!(frame.height(), 64);
        assert_eq!(frame.hot_spot, HotSpot::top_left());
        assert_eq!(frame.frame_type, DrawableType::Ram);
    }

    #[test]
    fn test_frame_invalid_bounds() {
        let result = Frame::new(0, DrawableType::Ram, 0, 64, HotSpot::top_left());
        assert!(matches!(result, Err(FrameError::InvalidBounds)));

        let result = Frame::new(0, DrawableType::Ram, 64, 0, HotSpot::top_left());
        assert!(matches!(result, Err(FrameError::InvalidBounds)));
    }

    #[test]
    fn test_drawable_basic() {
        let id = NonZeroU32::new(1).unwrap();
        let flags = DrawableFlags::default();
        let mut drawable = Drawable::new(id, DrawableType::Ram, flags, 3);

        assert_eq!(drawable.id(), 1);
        assert_eq!(drawable.capacity(), 3);
        assert_eq!(drawable.frame_count(), 3);

        // Add a frame
        let frame = Frame::with_top_left_hotspot(0, DrawableType::Ram, 32, 32).unwrap();
        drawable.add_frame(frame).unwrap();

        assert_eq!(drawable.frame_count(), 3);

        // Retrieve the frame
        let retrieved = drawable.get_frame(0).unwrap();
        assert_eq!(retrieved.width(), 32);
        assert_eq!(retrieved.height(), 32);
    }

    #[test]
    fn test_drawable_invalid_frame_index() {
        let id = NonZeroU32::new(1).unwrap();
        let flags = DrawableFlags::default();
        let mut drawable = Drawable::new(id, DrawableType::Ram, flags, 2);

        // Try to add frame with index beyond capacity
        let frame = Frame::with_top_left_hotspot(5, DrawableType::Ram, 32, 32).unwrap();
        let result = drawable.add_frame(frame);
        assert!(matches!(result, Err(FrameError::InvalidFrameIndex { .. })));
    }

    #[test]
    fn test_drawable_registry() {
        let registry = DrawableRegistry::new();

        let id = registry
            .allocate(DrawableType::Ram, DrawableFlags::default(), 2)
            .unwrap();

        assert_eq!(registry.count(), 1);

        // Get and modify the drawable
        {
            let drawable_ref = registry.get(id.get()).unwrap();
            assert_eq!(drawable_ref.drawable_type(), DrawableType::Ram);
        }

        // Release
        registry.release(id.get()).unwrap();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_drawable_registry_frame_access() {
        let registry = DrawableRegistry::new();

        let id = registry
            .allocate_with_bounds(
                DrawableType::Ram,
                DrawableFlags::default(),
                2,
                Extent::new(16, 16),
            )
            .unwrap();

        let drawable_ref = registry.get(id.get()).unwrap();
        assert_eq!(drawable_ref.capacity(), 2);
        assert_eq!(drawable_ref.frame_count(), 2);
        let frame = drawable_ref.get_frame(1).unwrap();
        assert!(frame.image.is_some());
    }

    #[test]
    fn test_effective_rect() {
        let frame = Frame::new(0, DrawableType::Ram, 64, 32, HotSpot::make(32, 16)).unwrap();

        let rect = frame.effective_rect();
        assert_eq!(rect.corner.x, -32);
        assert_eq!(rect.corner.y, -16);
        assert_eq!(rect.width(), 64);
        assert_eq!(rect.height(), 32);
    }
}
