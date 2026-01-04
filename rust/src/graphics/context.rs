//! Graphics context management
//!
//! This module provides Rust abstractions for the graphics context
//! concepts from the original C code (libs/graphics/context.h).
//!
//! Key concepts:
//! - Context: Drawing state container (colors, clip rects, draw modes)
//! - ClipRect: Bounding region for drawing operations
//! - GraphicsStatus: System state flags
//! - DrawMode: Rendering mode (replace, additive, alpha)

use crate::graphics::drawable::{Extent, Point, Rect};
use anyhow::{Context as _, Result};
use std::num::NonZeroU32;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, RwLock};

/// Graphics drawing modes (from DrawKind)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawKind {
    /// Replace pixels entirely
    Replace,
    /// Additive blending
    Additive,
    /// Alpha blending
    Alpha,
}

impl From<u8> for DrawKind {
    fn from(value: u8) -> Self {
        match value {
            0 => DrawKind::Replace,
            1 => DrawKind::Additive,
            2 => DrawKind::Alpha,
            _ => DrawKind::Replace, // Default fallback
        }
    }
}

/// Drawing mode with blending factor
///
/// Combines a drawing kind with an optional factor for blend operations.
/// Matches the C DrawMode struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawMode {
    pub kind: DrawKind,
    pub factor: i16,
}

impl DrawMode {
    /// Create a new draw mode
    pub fn new(kind: DrawKind, factor: i16) -> Self {
        Self { kind, factor }
    }

    /// Create a replace mode (default)
    pub fn replace() -> Self {
        Self {
            kind: DrawKind::Replace,
            factor: 0,
        }
    }

    /// Create an additive mode with default factor (255 = 1:1 ratio)
    pub fn additive(factor: i16) -> Self {
        Self {
            kind: DrawKind::Additive,
            factor,
        }
    }

    /// Create an alpha mode (factor 0..255, where 255 = fully opaque)
    pub fn alpha(factor: i16) -> Self {
        Self {
            kind: DrawKind::Alpha,
            factor: factor.clamp(0, 255),
        }
    }

    /// Check if this mode uses blending
    pub fn is_blended(&self) -> bool {
        matches!(self.kind, DrawKind::Additive | DrawKind::Alpha)
    }
}

/// Default draw mode (replace)
impl Default for DrawMode {
    fn default() -> Self {
        Self::replace()
    }
}

/// Clip rectangle for bounding drawing operations
///
/// The clip rect defines a rectangular region in screen coordinates.
/// Drawing operations are clipped to this region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRect {
    /// Top-left corner of the clipping region
    pub origin: Point,
    /// Extent (width, height) of the clipping region
    pub extent: Extent,
}

impl ClipRect {
    /// Create a new clip rect
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self {
            origin: Point::new(x, y),
            extent: Extent::new(width, height),
        }
    }

    /// Create from a Rect
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            origin: rect.corner,
            extent: rect.extent,
        }
    }

    /// Create an unbounded clip (full screen)
    pub fn full_screen(width: u16, height: u16) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Get width
    pub fn width(&self) -> u16 {
        self.extent.width
    }

    /// Get height
    pub fn height(&self) -> u16 {
        self.extent.height
    }

    /// Get right boundary (exclusive)
    pub fn right(&self) -> i32 {
        self.origin.x as i32 + self.width() as i32
    }

    /// Get bottom boundary (exclusive)
    pub fn bottom(&self) -> i32 {
        self.origin.y as i32 + self.height() as i32
    }

    /// Check if a point is within the clip region (in screen coordinates)
    pub fn contains(&self, point: Point) -> bool {
        let px = point.x as i32;
        let py = point.y as i32;
        let ox = self.origin.x as i32;
        let oy = self.origin.y as i32;

        px >= ox && py >= oy && px < self.right() && py < self.bottom()
    }

    /// Convert a global point to clip-relative coordinates
    pub fn to_local(&self, point: Point) -> Option<Point> {
        if self.contains(point) {
            Some(Point::new(point.x - self.origin.x, point.y - self.origin.y))
        } else {
            None
        }
    }

    /// Get as Rect
    pub fn as_rect(&self) -> Rect {
        Rect::new(self.origin, self.width(), self.height())
    }

    /// Intersect with another clip rect
    pub fn intersect(&self, other: &ClipRect) -> Option<ClipRect> {
        let r1 = self.as_rect();
        let r2 = other.as_rect();
        r1.intersection(&r2).map(ClipRect::from_rect)
    }
}

/// Graphics system status flags
///
/// Represents the global graphics state from the C code (GRAPHICS_STATUS).
/// Note: This is internally represented as raw bits, not an enum variant,
/// because multiple flags can be set simultaneously (bit flags pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphicsStatus(u8);

impl GraphicsStatus {
    /// No flags set
    pub const INACTIVE: Self = Self(0b0000);

    /// All flags set
    pub const FULLY_ACTIVE: Self = Self(0b1111);

    /// Create from raw bits
    pub fn from_bits(value: u8) -> Self {
        Self(value & 0b1111)
    }

    /// Convert to raw bits
    pub fn to_bits(&self) -> u8 {
        self.0
    }

    /// Check if graphics system is active (bit 0)
    pub fn is_active(&self) -> bool {
        self.0 & 0b0001 != 0
    }

    /// Check if graphics system is visible (bit 1)
    pub fn is_visible(&self) -> bool {
        self.0 & 0b0010 != 0
    }

    /// Check if context is active (bit 2)
    pub fn is_context_active(&self) -> bool {
        self.0 & 0b0100 != 0
    }

    /// Check if drawable system is active (bit 3)
    pub fn is_drawable_active(&self) -> bool {
        self.0 & 0b1000 != 0
    }

    /// Check if full system is active (context AND drawable)
    pub fn is_fully_active(&self) -> bool {
        self.is_context_active() && self.is_drawable_active()
    }

    /// Set active flag (bit 0)
    #[must_use]
    pub fn with_active(mut self) -> Self {
        self.0 |= 0b0001;
        self
    }

    /// Set visible flag (bit 1, and implies active)
    #[must_use]
    pub fn with_visible(mut self) -> Self {
        self.0 |= 0b0011; // Set visible + active
        self
    }

    /// Set context active flag (bit 2, and implies active)
    #[must_use]
    pub fn with_context_active(mut self) -> Self {
        self.0 |= 0b0101; // Set context + active
        self
    }

    /// Set drawable active flag (bit 3, and implies active)
    #[must_use]
    pub fn with_drawable_active(mut self) -> Self {
        self.0 |= 0b1001; // Set drawable + active
        self
    }

    /// Clear all flags
    #[must_use]
    pub fn clear_all(mut self) -> Self {
        self.0 = 0;
        self
    }
}

impl From<u8> for GraphicsStatus {
    fn from(value: u8) -> Self {
        Self::from_bits(value)
    }
}

impl From<GraphicsStatus> for u8 {
    fn from(status: GraphicsStatus) -> Self {
        status.0
    }
}

impl Default for GraphicsStatus {
    fn default() -> Self {
        Self::INACTIVE
    }
}

/// Graphics context for drawing operations
///
/// Encapsulates all drawing state: colors, clip rects, draw modes, etc.
/// Matches the C CONTEXT_DESC structure.
///
/// Note: Font-related fields (Font, FontEffect, FontBacking) are omitted
/// as they're out of scope for Phase 2.
#[derive(Debug, Clone)]
pub struct Context {
    /// Context ID
    id: NonZeroU32,
    /// Foreground color (RGBA)
    fg_color: [u8; 4],
    /// Background color (RGBA)
    bg_color: [u8; 4],
    /// Current draw mode
    draw_mode: DrawMode,
    /// Clipping rectangle
    clip_rect: ClipRect,
    /// Origin offset for drawing
    origin: Point,
    /// Context flags
    flags: u8,
}

impl Context {
    /// Create a new context with default values
    pub fn new(id: NonZeroU32, width: u16, height: u16) -> Self {
        Self {
            id,
            fg_color: [255, 255, 255, 255], // White by default
            bg_color: [0, 0, 0, 255],       // Black by default
            draw_mode: DrawMode::replace(),
            clip_rect: ClipRect::full_screen(width, height),
            origin: Point::new(0, 0),
            flags: 0,
        }
    }

    /// Create a context with specified clip rect
    pub fn with_clip_rect(id: NonZeroU32, clip_rect: ClipRect) -> Self {
        Self {
            id,
            fg_color: [255, 255, 255, 255],
            bg_color: [0, 0, 0, 255],
            draw_mode: DrawMode::replace(),
            clip_rect,
            origin: Point::new(0, 0),
            flags: 0,
        }
    }

    /// Get context ID
    pub fn id(&self) -> u32 {
        self.id.get()
    }

    /// Get foreground color (RGBA)
    pub fn fg_color(&self) -> [u8; 4] {
        self.fg_color
    }

    /// Set foreground color (RGBA)
    pub fn set_fg_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.fg_color = [r, g, b, a];
    }

    /// Get background color (RGBA)
    pub fn bg_color(&self) -> [u8; 4] {
        self.bg_color
    }

    /// Set background color (RGBA)
    pub fn set_bg_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.bg_color = [r, g, b, a];
    }

    /// Get draw mode
    pub fn draw_mode(&self) -> DrawMode {
        self.draw_mode
    }

    /// Set draw mode
    pub fn set_draw_mode(&mut self, mode: DrawMode) {
        self.draw_mode = mode;
    }

    /// Get clip rect
    pub fn clip_rect(&self) -> ClipRect {
        self.clip_rect
    }

    /// Set clip rect
    pub fn set_clip_rect(&mut self, clip_rect: ClipRect) {
        self.clip_rect = clip_rect;
    }

    /// Get origin offset
    pub fn origin(&self) -> Point {
        self.origin
    }

    /// Set origin offset
    pub fn set_origin(&mut self, origin: Point) {
        self.origin = origin;
    }

    /// Get flags
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Set a flag
    pub fn set_flag(&mut self, flag: u8) {
        self.flags |= flag;
    }

    /// Unset a flag
    pub fn unset_flag(&mut self, flag: u8) {
        self.flags &= !flag;
    }

    /// Check if a flag is set
    pub fn has_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// Transform a point from context-relative to screen coordinates
    pub fn to_screen(&self, point: Point) -> Point {
        Point::new(
            point.x + self.origin.x + self.clip_rect.origin.x,
            point.y + self.origin.y + self.clip_rect.origin.y,
        )
    }

    /// Transform a point from screen to context-relative coordinates
    pub fn to_context(&self, screen_point: Point) -> Option<Point> {
        let rel_x = screen_point.x - self.origin.x - self.clip_rect.origin.x;
        let rel_y = screen_point.y - self.origin.y - self.clip_rect.origin.y;

        // Check if within clip rect
        if self.clip_rect.contains(screen_point) {
            Some(Point::new(rel_x, rel_y))
        } else {
            None
        }
    }

    /// Get the valid drawing rectangle
    pub fn valid_rect(&self) -> Rect {
        let origin = self.to_screen(Point::new(0, 0));
        Rect::from_xywh(
            origin.x,
            origin.y,
            self.clip_rect.width(),
            self.clip_rect.height(),
        )
    }
}

/// Context stack for managing multiple contexts
///
/// Provides thread-safe context switching and management.
/// Matches the C code's global context handling.
pub struct ContextStack {
    contexts: RwLock<Vec<Arc<Context>>>,
    current: RwLock<Option<Arc<Context>>>,
    next_id: RwLock<u32>,
}

impl ContextStack {
    /// Create a new context stack
    pub fn new() -> Self {
        Self {
            contexts: RwLock::new(Vec::new()),
            current: RwLock::new(None),
            next_id: RwLock::new(1),
        }
    }

    /// Allocate a new context
    pub fn create(&self, width: u16, height: u16) -> Result<Arc<Context>> {
        let id = {
            let mut next = self
                .next_id
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let id = *next;
            *next = id.wrapping_add(1);
            if *next == 0 {
                *next = 1;
            }
            NonZeroU32::new(id).context("Failed to allocate context ID")?
        };

        let context = Context::new(id, width, height);

        {
            let mut contexts = self
                .contexts
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let context_arc = Arc::new(context);
            contexts.push(Arc::clone(&context_arc));

            // Set as current if it's the first context
            let mut current = self
                .current
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            if current.is_none() {
                *current = Some(Arc::clone(&context_arc));
            }

            Ok(context_arc)
        }
    }

    /// Allocate a new context with custom clip rect
    pub fn create_with_clip(&self, clip_rect: ClipRect) -> Result<Arc<Context>> {
        let id = {
            let mut next = self
                .next_id
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let id = *next;
            *next = id.wrapping_add(1);
            if *next == 0 {
                *next = 1;
            }
            NonZeroU32::new(id).context("Failed to allocate context ID")?
        };

        let context = Context::with_clip_rect(id, clip_rect);

        {
            let mut contexts = self
                .contexts
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let context_arc = Arc::new(context);
            contexts.push(Arc::clone(&context_arc));

            // Set as current if it's the first context
            let mut current = self
                .current
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            if current.is_none() {
                *current = Some(Arc::clone(&context_arc));
            }

            Ok(context_arc)
        }
    }

    /// Switch to a different current context
    pub fn switch(&self, context: Arc<Context>) -> Result<()> {
        let mut current = self
            .current
            .write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        *current = Some(context);
        Ok(())
    }

    /// Get current context
    pub fn current(&self) -> Result<Arc<Context>> {
        self.current
            .read()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?
            .as_ref()
            .cloned()
            .context("No current context")
    }

    /// Destroy a context
    pub fn destroy(&self, id: u32) -> Result<bool> {
        let mut contexts = self
            .contexts
            .write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        // Find and remove the context
        let mut found = false;
        contexts.retain(|ctx| {
            if ctx.id() == id {
                found = true;
                false
            } else {
                true
            }
        });

        // If we destroyed the current context, clear it
        if found {
            let mut current = self
                .current
                .write()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            if let Some(curr) = current.as_ref() {
                if curr.id() == id {
                    *current = None;
                }
            }
        }

        Ok(found)
    }

    /// Count active contexts
    pub fn count(&self) -> usize {
        self.contexts.read().map(|c| c.len()).unwrap_or(0)
    }
}

impl Default for ContextStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Global graphics status manager
///
/// Provides thread-safe access to the graphics system status flags.
pub struct GraphicsStatusManager {
    status: AtomicU8,
}

impl GraphicsStatusManager {
    /// Create a new status manager
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(0),
        }
    }

    /// Get current status
    pub fn get(&self) -> GraphicsStatus {
        let value = self.status.load(std::sync::atomic::Ordering::Acquire);
        GraphicsStatus::from_bits(value)
    }

    /// Set status
    pub fn set(&self, status: GraphicsStatus) {
        self.status
            .store(status.to_bits(), std::sync::atomic::Ordering::Release);
    }

    /// Activate graphics (set bit 0)
    pub fn activate_graphics(&self) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value |= 0b0001;
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }

    /// Deactivate graphics (clear bit 0)
    pub fn deactivate_graphics(&self) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value &= !0b0001;
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }

    /// Set graphics visible (set bit 1, also sets bit 0)
    pub fn set_visible(&self, visible: bool) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value = if visible {
            value | 0b0011 // visible + active
        } else {
            value & !0b0010 // clear visible only
        };
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }

    /// Activate context (set bit 2, also ensures bit 0)
    pub fn activate_context(&self) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value |= 0b0101; // context + active
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }

    /// Deactivate context (clear bit 2)
    pub fn deactivate_context(&self) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value &= !0b0100;
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }

    /// Activate drawable (set bit 3, also ensures bit 0)
    pub fn activate_drawable(&self) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value |= 0b1001; // drawable + active
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }

    /// Deactivate drawable (clear bit 3)
    pub fn deactivate_drawable(&self) {
        let mut value = self.status.load(std::sync::atomic::Ordering::Acquire);
        value &= !0b1000;
        self.status
            .store(value & 0b1111, std::sync::atomic::Ordering::Release);
    }
}

impl Default for GraphicsStatusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_mode_creation() {
        let replace = DrawMode::replace();
        assert_eq!(replace.kind, DrawKind::Replace);
        assert_eq!(replace.factor, 0);
        assert!(!replace.is_blended());

        let additive = DrawMode::additive(255);
        assert_eq!(additive.kind, DrawKind::Additive);
        assert_eq!(additive.factor, 255);
        assert!(additive.is_blended());

        let alpha = DrawMode::alpha(128);
        assert_eq!(alpha.kind, DrawKind::Alpha);
        assert_eq!(alpha.factor, 128);
        assert!(alpha.is_blended());

        // Alpha mode clamps factor to 0..255
        let alpha_clamped = DrawMode::alpha(-10);
        assert_eq!(alpha_clamped.factor, 0);

        let alpha_clamped2 = DrawMode::alpha(500);
        assert_eq!(alpha_clamped2.factor, 255);
    }

    #[test]
    fn test_clip_rect_basic() {
        let clip = ClipRect::new(10, 10, 100, 50);

        assert_eq!(clip.origin.x, 10);
        assert_eq!(clip.origin.y, 10);
        assert_eq!(clip.width(), 100);
        assert_eq!(clip.height(), 50);

        assert!(clip.contains(Point::new(10, 10)));
        assert!(clip.contains(Point::new(50, 30)));
        assert!(!clip.contains(Point::new(111, 10)));
        assert!(!clip.contains(Point::new(10, 61)));
    }

    #[test]
    fn test_clip_rect_to_local() {
        let clip = ClipRect::new(10, 10, 100, 50);

        let local = clip.to_local(Point::new(20, 20));
        assert!(local.is_some());
        assert_eq!(local.unwrap().x, 10);
        assert_eq!(local.unwrap().y, 10);

        let outside = clip.to_local(Point::new(150, 20));
        assert!(outside.is_none());
    }

    #[test]
    fn test_clip_rect_intersect() {
        let c1 = ClipRect::new(0, 0, 100, 100);
        let c2 = ClipRect::new(50, 50, 100, 100);

        let intersection = c1.intersect(&c2);
        assert!(intersection.is_some());
        let rect = intersection.unwrap();
        assert_eq!(rect.origin.x, 50);
        assert_eq!(rect.origin.y, 50);
        assert_eq!(rect.width(), 50);
        assert_eq!(rect.height(), 50);

        let no_intersect = c1.intersect(&ClipRect::new(200, 200, 10, 10));
        assert!(no_intersect.is_none());
    }

    #[test]
    fn test_graphics_status() {
        let status = GraphicsStatus::INACTIVE;
        assert!(!status.is_active());
        assert!(!status.is_visible());
        assert!(!status.is_context_active());
        assert!(!status.is_drawable_active());
        assert!(!status.is_fully_active());

        let active = GraphicsStatus::INACTIVE.with_active();
        assert!(active.is_active());

        let visible = GraphicsStatus::INACTIVE.with_visible();
        assert!(visible.is_active());
        assert!(visible.is_visible());

        let ctx_active = GraphicsStatus::INACTIVE.with_context_active();
        assert!(ctx_active.is_active());
        assert!(ctx_active.is_context_active());

        let drw_active = GraphicsStatus::INACTIVE.with_drawable_active();
        assert!(drw_active.is_active());
        assert!(drw_active.is_drawable_active());

        // FULLY_ACTIVE combines all flags
        let fully = GraphicsStatus::FULLY_ACTIVE;
        assert!(fully.is_active());
        assert!(fully.is_context_active());
        assert!(fully.is_drawable_active());
        assert!(fully.is_fully_active());
    }

    #[test]
    fn test_graphics_status_conversion() {
        let status = GraphicsStatus::INACTIVE;
        let byte: u8 = status.into();
        assert_eq!(byte, 0);

        let restored = GraphicsStatus::from(byte);
        assert_eq!(restored.to_bits(), 0);
    }

    #[test]
    fn test_context_creation() {
        let id = NonZeroU32::new(1).unwrap();
        let ctx = Context::new(id, 640, 480);

        assert_eq!(ctx.id(), 1);
        assert_eq!(ctx.fg_color(), [255, 255, 255, 255]);
        assert_eq!(ctx.bg_color(), [0, 0, 0, 255]);
        assert_eq!(ctx.draw_mode(), DrawMode::replace());
        assert_eq!(ctx.origin(), Point::new(0, 0));
    }

    #[test]
    fn test_context_colors() {
        let id = NonZeroU32::new(1).unwrap();
        let mut ctx = Context::new(id, 640, 480);

        ctx.set_fg_color(255, 0, 0, 255);
        assert_eq!(ctx.fg_color(), [255, 0, 0, 255]);

        ctx.set_bg_color(0, 255, 0, 128);
        assert_eq!(ctx.bg_color(), [0, 255, 0, 128]);
    }

    #[test]
    fn test_context_origin_transformation() {
        let id = NonZeroU32::new(1).unwrap();
        let mut ctx = Context::new(id, 640, 480);
        ctx.set_origin(Point::new(100, 50));

        let context_point = Point::new(10, 20);
        let screen_point = ctx.to_screen(context_point);
        assert_eq!(screen_point.x, 110);
        assert_eq!(screen_point.y, 70);

        let back_to_context = ctx.to_context(screen_point);
        assert!(back_to_context.is_some());
        assert_eq!(back_to_context.unwrap().x, 10);
        assert_eq!(back_to_context.unwrap().y, 20);
    }

    #[test]
    fn test_context_flags() {
        let id = NonZeroU32::new(1).unwrap();
        let mut ctx = Context::new(id, 640, 480);

        assert!(!ctx.has_flag(0x01));
        ctx.set_flag(0x01);
        assert!(ctx.has_flag(0x01));
        ctx.unset_flag(0x01);
        assert!(!ctx.has_flag(0x01));
    }

    #[test]
    fn test_context_stack() {
        let stack = ContextStack::new();

        let ctx1 = stack.create(640, 480).unwrap();
        assert_eq!(ctx1.id(), 1);
        assert_eq!(stack.count(), 1);

        let ctx2 = stack.create(320, 200).unwrap();
        assert_eq!(ctx2.id(), 2);
        assert_eq!(stack.count(), 2);

        // First context should be current initially
        let current = stack.current().unwrap();
        assert_eq!(current.id(), 1);

        // Switch to second context
        stack.switch(Arc::clone(&ctx2)).unwrap();
        let current = stack.current().unwrap();
        assert_eq!(current.id(), 2);

        // Destroy first context
        let destroyed = stack.destroy(1).unwrap();
        assert!(destroyed);
        assert_eq!(stack.count(), 1);
    }

    #[test]
    fn test_graphics_status_manager() {
        let manager = GraphicsStatusManager::new();

        assert!(!manager.get().is_active());

        manager.activate_graphics();
        assert!(manager.get().is_active());

        manager.set_visible(true);
        assert!(manager.get().is_visible());

        manager.activate_context();
        assert!(manager.get().is_context_active());

        manager.activate_drawable();
        assert!(manager.get().is_drawable_active());
        assert!(manager.get().is_fully_active());

        manager.deactivate_graphics();
        assert!(!manager.get().is_active());
    }
}
