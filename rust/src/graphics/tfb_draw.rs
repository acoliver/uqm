//! TFB Draw System
//!
//! This module provides the core drawing primitives for TFB (The Final Battle).
//! It implements the drawing commands for images, canvases, and screens.
//!
//! # Architecture
//!
//! The draw system is organized into three main layers:
//!
//! 1. **Image Layer (TFImage)**: Wraps a pixel canvas with metadata (hotspots,
//!    scaled versions, mipmap tracking, dirty flag).
//!
//! 2. **Canvas Layer (Canvas)**: Represents a raw pixel buffer with clipping
//!    support and direct pixel access.
//!
//! # Scope (Phase 2)
//!
//! - Core data structures: TFImage, Canvas
//! - Primitive drawing APIs: line, rect, image, fontchar, copy, scissor
//! - State/lifecycle tests (no actual rendering or SDL)
//!
//! # Out of Scope
//!
//! - DCQ integration (handled by dcqueue module)
//! - Color map handling
//! - Font parsing
//! - Scaling algorithms
//! - SDL drivers
//! - FFI bindings

use std::fmt;
use std::sync::{Arc, Mutex};
use std::u64;

use crate::graphics::dcqueue::{Color, DrawMode, Extent, FontCharRef, Point, Rect};
use log;
use crate::graphics::font::FontPage;
use crate::graphics::gfx_common::ScaleMode;

/// Unique identifier for canvas resources.
pub type CanvasId = u64;

/// Hot spot offset for image positioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HotSpot {
    pub x: i32,
    pub y: i32,
}

impl HotSpot {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn origin() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn is_origin(&self) -> bool {
        self.x == 0 && self.y == 0
    }
}

/// Canvas clipping configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScissorRect {
    pub rect: Option<Rect>,
}

impl ScissorRect {
    pub const fn new(rect: Option<Rect>) -> Self {
        Self { rect }
    }

    pub const fn disabled() -> Self {
        Self { rect: None }
    }

    pub const fn enabled(rect: Rect) -> Self {
        Self { rect: Some(rect) }
    }

    pub fn is_enabled(&self) -> bool {
        self.rect.is_some()
    }
}

/// Canvas pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasPixelFormat {
    Rgba,
    Rgb,
    Paletted,
}

/// Canvas format information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasFormat {
    pub kind: CanvasPixelFormat,
    pub bits_per_pixel: i32,
    pub bytes_per_pixel: i32,
    pub has_alpha: bool,
}

impl CanvasFormat {
    pub const fn rgba() -> Self {
        Self {
            kind: CanvasPixelFormat::Rgba,
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            has_alpha: true,
        }
    }

    pub const fn rgb() -> Self {
        Self {
            kind: CanvasPixelFormat::Rgb,
            bits_per_pixel: 24,
            bytes_per_pixel: 3,
            has_alpha: false,
        }
    }

    pub const fn paletted() -> Self {
        Self {
            kind: CanvasPixelFormat::Paletted,
            bits_per_pixel: 8,
            bytes_per_pixel: 1,
            has_alpha: true,
        }
    }

    pub const fn is_paletted(self) -> bool {
        matches!(self.kind, CanvasPixelFormat::Paletted)
    }
}

impl Default for CanvasFormat {
    fn default() -> Self {
        Self::rgba()
    }
}

/// Internal canvas state.
#[derive(Debug)]
struct CanvasInner {
    id: CanvasId,
    extent: Extent,
    format: CanvasFormat,
    scissor: ScissorRect,
    locked: bool,
    generation: u64,
    pixels: Vec<u8>,
    palette: Option<[Color; 256]>,
    transparent_index: Option<u8>,
}

impl CanvasInner {
    fn new(extent: Extent, format: CanvasFormat) -> Self {
        let pixel_count = (extent.width * extent.height) as usize;
        let bytes_per_pixel = format.bytes_per_pixel as usize;
        let pixels = vec![0u8; pixel_count * bytes_per_pixel];

        Self {
            id: Self::next_id(),
            extent,
            format,
            scissor: ScissorRect::disabled(),
            locked: false,
            generation: 0,
            pixels,
            palette: if format.is_paletted() {
                Some([Color::new(0, 0, 0, 255); 256])
            } else {
                None
            },
            transparent_index: None,
        }
    }

    fn next_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn lock(&mut self) -> Result<(), CanvasError> {
        if self.locked {
            return Err(CanvasError::AlreadyLocked);
        }
        self.locked = true;
        Ok(())
    }

    fn unlock(&mut self) -> Result<(), CanvasError> {
        if !self.locked {
            return Err(CanvasError::NotLocked);
        }
        self.locked = false;
        self.generation += 1;
        Ok(())
    }

    fn scissor(&self) -> ScissorRect {
        self.scissor
    }

    fn set_scissor(&mut self, scissor: ScissorRect) {
        self.scissor = scissor;
    }

    fn extent(&self) -> Extent {
        self.extent
    }

    fn format(&self) -> CanvasFormat {
        self.format
    }

    fn id(&self) -> CanvasId {

        self.id
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

/// A canvas representing a pixel buffer.
#[derive(Clone)]
pub struct Canvas {
    inner: Arc<Mutex<CanvasInner>>,
}

fn default_palette() -> [Color; 256] {
    [Color::new(0, 0, 0, 255); 256]
}

fn paletted_to_rgba(canvas: &Canvas) -> Result<Canvas, CanvasError> {
    if !canvas.is_paletted() {
        return Err(CanvasError::FormatMismatch);
    }
    let extent = canvas.extent();
    let mut rgba = Canvas::new_rgba(extent.width, extent.height);
    let palette = canvas.palette().unwrap_or_else(default_palette);
    let transparent = canvas.transparent_index();
    let src_pixels = canvas.pixels();
    rgba.with_pixels_mut(|dst_pixels| {
        for (index, chunk) in dst_pixels.chunks_exact_mut(4).enumerate() {
            let idx = src_pixels.get(index).copied().unwrap_or(0) as usize;
            let mut color = palette[idx];
            if Some(idx as u8) == transparent {
                color.a = 0;
            }
            chunk[0] = color.r;
            chunk[1] = color.g;
            chunk[2] = color.b;
            chunk[3] = color.a;
        }
    })?;
    Ok(rgba)
}

fn rgba_to_paletted(canvas: &Canvas, palette: [Color; 256]) -> Result<Canvas, CanvasError> {
    if canvas.is_paletted() {
        return Err(CanvasError::FormatMismatch);
    }
    let extent = canvas.extent();
    let mut paletted = Canvas::new_paletted(extent.width, extent.height, palette);
    let src_pixels = canvas.pixels();
    paletted.with_pixels_mut(|dst_pixels| {
        for (idx, chunk) in src_pixels.chunks_exact(4).enumerate() {
            let mut best_index = 0usize;
            let mut best_score = i32::MAX;
            for (palette_index, color) in palette.iter().enumerate() {
                let dr = chunk[0] as i32 - color.r as i32;
                let dg = chunk[1] as i32 - color.g as i32;
                let db = chunk[2] as i32 - color.b as i32;
                let score = dr * dr + dg * dg + db * db;
                if score < best_score {
                    best_score = score;
                    best_index = palette_index;
                    if score == 0 {
                        break;
                    }
                }
            }
            if let Some(slot) = dst_pixels.get_mut(idx) {
                *slot = best_index as u8;
            }
        }
    })?;
    Ok(paletted)
}

fn ensure_canvas_truecolor(canvas: &Canvas) -> Result<Canvas, CanvasError> {
    if canvas.is_paletted() {
        paletted_to_rgba(canvas)
    } else {
        Ok(canvas.clone())
    }
}

pub fn convert_canvas_format(
    canvas: &Canvas,
    target: CanvasFormat,
    palette: Option<[Color; 256]>,
) -> Result<Canvas, CanvasError> {
    if canvas.format() == target {
        return Ok(canvas.clone());
    }

    if target.is_paletted() {
        let palette = palette.unwrap_or_else(default_palette);
        return rgba_to_paletted(canvas, palette);
    }

    if canvas.is_paletted() {
        return paletted_to_rgba(canvas);
    }

    let extent = canvas.extent();
    let mut converted = Canvas::new(extent, target);
    let src_pixels = canvas.pixels();
    let src_bpp = canvas.format().bytes_per_pixel as usize;
    let dst_bpp = target.bytes_per_pixel as usize;
    converted.with_pixels_mut(|dst_pixels| {
        for (idx, chunk) in dst_pixels.chunks_exact_mut(dst_bpp).enumerate() {
            let src_offset = idx * src_bpp;
            if src_offset + src_bpp <= src_pixels.len() {
                chunk.copy_from_slice(&src_pixels[src_offset..src_offset + dst_bpp.min(src_bpp)]);
                if dst_bpp > src_bpp {
                    for extra in &mut chunk[src_bpp..] {
                        *extra = 255;
                    }
                }
            }
        }
    })?;
    Ok(converted)
}



impl Canvas {
    pub fn new(extent: Extent, format: CanvasFormat) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CanvasInner::new(extent, format))),
        }
    }

    pub fn new_rgba(width: i32, height: i32) -> Self {
        Self::new(Extent::new(width, height), CanvasFormat::rgba())
    }

    pub fn new_rgb(width: i32, height: i32) -> Self {
        Self::new(Extent::new(width, height), CanvasFormat::rgb())
    }

    pub fn new_paletted(width: i32, height: i32, palette: [Color; 256]) -> Self {
        let mut canvas = Self::new(Extent::new(width, height), CanvasFormat::paletted());
        canvas.set_palette(palette);
        canvas
    }

    pub fn new_for_screen(width: i32, height: i32) -> Self {
        Self::new_rgba(width, height)
    }

    pub fn lock(&self) -> Result<(), CanvasError> {
        self.inner.lock().unwrap().lock()
    }

    pub fn unlock(&self) -> Result<(), CanvasError> {
        self.inner.lock().unwrap().unlock()
    }

    pub fn is_locked(&self) -> bool {
        self.inner.lock().unwrap().locked
    }

    pub fn extent(&self) -> Extent {
        self.inner.lock().unwrap().extent()
    }

    pub fn width(&self) -> i32 {
        self.extent().width
    }

    pub fn height(&self) -> i32 {
        self.extent().height
    }

    pub fn with_locked_pixels<F, R>(&self, f: F) -> Result<R, CanvasError>
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut inner = self.inner.lock().unwrap();
        if !inner.locked {
            return Err(CanvasError::NotLocked);
        }
        Ok(f(inner.pixels_mut()))
    }

    pub fn with_pixels_mut<F, R>(&mut self, f: F) -> Result<R, CanvasError>
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut inner = self.inner.lock().unwrap();
        if inner.locked {
            return Err(CanvasError::AlreadyLocked);
        }
        Ok(f(inner.pixels_mut()))
    }

    pub fn format(&self) -> CanvasFormat {
        self.inner.lock().unwrap().format()
    }

    pub fn scissor(&self) -> ScissorRect {
        self.inner.lock().unwrap().scissor()
    }

    pub fn set_scissor(&self, scissor: ScissorRect) {
        self.inner.lock().unwrap().set_scissor(scissor);
    }

    pub fn enable_scissor(&self, rect: Rect) {
        self.set_scissor(ScissorRect::enabled(rect));
    }

    pub fn disable_scissor(&self) {
        self.set_scissor(ScissorRect::disabled());
    }

    pub fn pixels(&self) -> Vec<u8> {
        self.inner.lock().unwrap().pixels().to_vec()
    }

    pub fn pixels_mut(&mut self) -> Vec<u8> {
        self.inner.lock().unwrap().pixels_mut().to_vec()
    }

    pub fn is_paletted(&self) -> bool {
        self.format().is_paletted()
    }

    pub fn palette(&self) -> Option<[Color; 256]> {
        self.inner.lock().unwrap().palette
    }

    pub fn set_palette(&mut self, palette: [Color; 256]) {
        let mut inner = self.inner.lock().unwrap();
        inner.palette = Some(palette);
    }

    pub fn transparent_index(&self) -> Option<u8> {
        self.inner.lock().unwrap().transparent_index
    }

    pub fn set_transparent_index(&mut self, index: Option<u8>) {
        let mut inner = self.inner.lock().unwrap();
        inner.transparent_index = index;
    }

    pub fn copy_rect(
        &self,
        source: &Canvas,
        src_rect: Rect,
        dst_pt: Point,
    ) -> Result<(), CanvasError> {
        let _dst_extent = self.extent();
        let src_extent = source.extent();

        if src_rect.corner.x < 0
            || src_rect.corner.y < 0
            || src_rect.corner.x + src_rect.extent.width > src_extent.width
            || src_rect.corner.y + src_rect.extent.height > src_extent.height
        {
            return Err(CanvasError::InvalidRect);
        }

        if dst_pt.x < 0 || dst_pt.y < 0 {
            return Err(CanvasError::InvalidPoint);
        }

        Ok(())
    }
}

impl fmt::Debug for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        f.debug_struct("Canvas")
            .field("id", &inner.id())
            .field("extent", &inner.extent())
            .field("format", &inner.format())
            .field("locked", &inner.locked)
            .field("scissor_enabled", &inner.scissor().is_enabled())
            .field("generation", &inner.generation())
            .finish()
    }
}

/// TFImage - A wrapped canvas with image metadata.
#[derive(Clone)]
pub struct TFImage {
    normal: Arc<Mutex<Option<Canvas>>>,
    scaled: Arc<Mutex<Option<Canvas>>>,
    mipmap: Arc<Mutex<Option<Canvas>>>,
    filled: Arc<Mutex<Option<Canvas>>>,
    normal_hs: Arc<Mutex<HotSpot>>,
    mipmap_hs: Arc<Mutex<HotSpot>>,
    last_scale: Arc<Mutex<i32>>,
    last_scale_type: Arc<Mutex<Option<ScaleMode>>>,
    dirty: Arc<Mutex<bool>>,
    frames: Arc<Mutex<Vec<Arc<TFImage>>>>,
    frame_index: Arc<Mutex<usize>>,
}

impl TFImage {
    pub fn new(canvas: Canvas) -> Self {
        Self {
            normal: Arc::new(Mutex::new(Some(canvas))),
            scaled: Arc::new(Mutex::new(None)),
            mipmap: Arc::new(Mutex::new(None)),
            filled: Arc::new(Mutex::new(None)),
            normal_hs: Arc::new(Mutex::new(HotSpot::origin())),
            mipmap_hs: Arc::new(Mutex::new(HotSpot::origin())),
            last_scale: Arc::new(Mutex::new(0)),
            last_scale_type: Arc::new(Mutex::new(None)),
            dirty: Arc::new(Mutex::new(false)),
            frames: Arc::new(Mutex::new(Vec::new())),
            frame_index: Arc::new(Mutex::new(0)),
        }
    }

    pub fn new_rgba(width: i32, height: i32) -> Self {
        Self::new(Canvas::new_rgba(width, height))
    }

    pub fn new_paletted(width: i32, height: i32, palette: [Color; 256]) -> Self {
        Self::new(Canvas::new_paletted(width, height, palette))
    }

    pub fn new_for_screen(width: i32, height: i32, with_alpha: bool) -> Self {
        let format = if with_alpha {
            CanvasFormat::rgba()
        } else {
            CanvasFormat::rgb()
        };
        Self::new(Canvas::new(Extent::new(width, height), format))
    }

    pub fn from_canvas(canvas: Canvas) -> Self {
        Self::new(canvas)
    }

    pub fn normal(&self) -> Option<Canvas> {
        self.normal.lock().unwrap().clone()
    }

    pub fn scaled(&self) -> Option<Canvas> {
        self.scaled.lock().unwrap().clone()
    }

    pub fn mipmap(&self) -> Option<Canvas> {
        self.mipmap.lock().unwrap().clone()
    }

    pub fn filled(&self) -> Option<Canvas> {
        self.filled.lock().unwrap().clone()
    }

    pub fn normal_hot_spot(&self) -> HotSpot {
        *self.normal_hs.lock().unwrap()
    }

    pub fn set_normal_hot_spot(&self, hs: HotSpot) {
        *self.normal_hs.lock().unwrap() = hs;
    }

    pub fn mipmap_hot_spot(&self) -> HotSpot {
        *self.mipmap_hs.lock().unwrap()
    }

    pub fn set_mipmap_hot_spot(&self, hs: HotSpot) {
        *self.mipmap_hs.lock().unwrap() = hs;
    }

    pub fn extent(&self) -> Option<Extent> {
        self.normal.lock().unwrap().as_ref().map(|c| c.extent())
    }

    pub fn width(&self) -> Option<i32> {
        self.extent().map(|e| e.width)
    }

    pub fn height(&self) -> Option<i32> {
        self.extent().map(|e| e.height)
    }

    pub fn is_dirty(&self) -> bool {
        *self.dirty.lock().unwrap()
    }

    pub fn mark_dirty(&self) {
        *self.dirty.lock().unwrap() = true;
    }

    pub fn mark_clean(&self) {
        *self.dirty.lock().unwrap() = false;
    }

    pub fn set_scaled(&self, canvas: Option<Canvas>) {
        *self.scaled.lock().unwrap() = canvas;
    }

    pub fn add_frame(&self, frame: Arc<TFImage>) {
        self.frames.lock().unwrap().push(frame);
    }

    pub fn frame_count(&self) -> usize {
        self.frames.lock().unwrap().len() + 1
    }

    pub fn set_frame_index(&self, index: usize) {
        let mut frame_index = self.frame_index.lock().unwrap();
        let count = self.frame_count();
        *frame_index = index % count;
    }

    pub fn frame_index(&self) -> usize {
        *self.frame_index.lock().unwrap()
    }

    pub fn current_frame(&self) -> Option<Arc<TFImage>> {
        let frames = self.frames.lock().unwrap();
        let index = self.frame_index();
        if index == 0 {
            None
        } else {
            frames.get(index - 1).cloned()
        }
    }

    pub fn set_mipmap(&self, canvas: Option<Canvas>, hs: HotSpot) {
        if let Some(ref mipmap) = canvas {
            if let Some(ref normal) = *self.normal.lock().unwrap() {
                if normal.is_paletted()
                    && mipmap.is_paletted()
                    && normal.transparent_index() != mipmap.transparent_index()
                {
                    *self.mipmap.lock().unwrap() = None;
                    return;
                }
            }
        }
        *self.mipmap.lock().unwrap() = canvas;
        *self.mipmap_hs.lock().unwrap() = hs;
    }

    pub fn set_filled(&self, canvas: Option<Canvas>) {
        *self.filled.lock().unwrap() = canvas;
    }

    pub fn scaling_cache_valid(&self, scale: i32, scale_type: ScaleMode) -> bool {
        if self.is_dirty() {
            return false;
        }
        let cached_scale = *self.last_scale.lock().unwrap();
        let cached_type = *self.last_scale_type.lock().unwrap();
        cached_scale == scale && cached_type == Some(scale_type)
    }

    pub fn update_scaling_cache(&self, scale: i32, scale_type: ScaleMode) {
        *self.last_scale.lock().unwrap() = scale;
        *self.last_scale_type.lock().unwrap() = Some(scale_type);
    }

    pub fn invalidate_scaling_cache(&self) {
        *self.last_scale.lock().unwrap() = 0;
        *self.last_scale_type.lock().unwrap() = None;
    }

    pub fn delete(&self) {
        *self.normal.lock().unwrap() = None;
        *self.scaled.lock().unwrap() = None;
        *self.mipmap.lock().unwrap() = None;
        *self.filled.lock().unwrap() = None;
        self.invalidate_scaling_cache();
        self.mark_clean();
    }

    pub fn set_palette_from(&self, palette: [Color; 256]) -> Result<(), TFImageError> {
        let mut guard = self.normal.lock().unwrap();
        let Some(canvas) = guard.as_mut() else {
            return Err(TFImageError::NoPrimaryCanvas);
        };
        if !canvas.is_paletted() {
            return Err(TFImageError::InvalidPaletteConversion);
        }
        canvas.set_palette(palette);
        Ok(())
    }

    pub fn fix_scaling(&self, scale: i32, scale_mode: ScaleMode) -> Result<(), TFImageError> {
        let scale = if scale == 0 { 256 } else { scale };
        if self.scaling_cache_valid(scale, scale_mode) {
            return Ok(());
        }

        let scaled = rescale_image(self, scale, scale_mode)?;
        self.set_scaled(Some(scaled));
        self.update_scaling_cache(scale, scale_mode);
        self.mark_clean();
        Ok(())
    }
}

impl fmt::Debug for TFImage {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TFImage")
            .field("extent", &self.extent())
            .field("normal_hs", &self.normal_hot_spot())
            .field("mipmap_hs", &self.mipmap_hot_spot())
            .field("dirty", &self.is_dirty())
            .field("has_scaled", &self.scaled().is_some())
            .field("has_mipmap", &self.mipmap().is_some())
            .field("has_filled", &self.filled().is_some())
            .field("last_scale", &*self.last_scale.lock().unwrap())
            .field("last_scale_type", &*self.last_scale_type.lock().unwrap())
            .field("frames", &self.frame_count())
            .field("frame_index", &self.frame_index())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasError {
    AlreadyLocked,
    NotLocked,
    InvalidRect,
    InvalidPoint,
    InvalidOperation(String),
    FormatMismatch,
}

impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyLocked => write!(f, "Canvas is already locked"),
            Self::NotLocked => write!(f, "Canvas is not locked"),
            Self::InvalidRect => write!(f, "Invalid rectangle specification"),
            Self::InvalidPoint => write!(f, "Invalid point specification"),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::FormatMismatch => write!(f, "Format mismatch"),
        }
    }
}

impl std::error::Error for CanvasError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TFImageError {
    Canvas(CanvasError),
    NoPrimaryCanvas,
    InvalidMipmap,
    InvalidPaletteConversion,
}

impl fmt::Display for TFImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canvas(err) => write!(f, "Canvas error: {}", err),
            Self::NoPrimaryCanvas => write!(f, "TFImage has no primary canvas"),
            Self::InvalidMipmap => write!(f, "Mipmap invalid: colormap restriction violated"),
            Self::InvalidPaletteConversion => write!(f, "Palette conversion requires paletted canvas"),
        }
    }
}

impl std::error::Error for TFImageError {}

impl From<CanvasError> for TFImageError {
    fn from(err: CanvasError) -> Self {
        Self::Canvas(err)
    }
}

/// Helper function to check canvas validity.
fn check_canvas(canvas: &Canvas) -> Result<(), CanvasError> {
    let inner = canvas.inner.lock().unwrap();
    if inner.pixels().is_empty() {
        return Err(CanvasError::InvalidRect);
    }
    let extent = inner.extent();
    if extent.width <= 0 || extent.height <= 0 {
        return Err(CanvasError::InvalidRect);
    }
    Ok(())
}

/// Check if a point is within the current scissor rectangle.
///
/// Returns true if the point should be drawn, false if it's clipped.
fn is_in_scissor(canvas: &Canvas, x: i32, y: i32) -> bool {
    let scissor = canvas.scissor();
    if let Some(rect) = scissor.rect {
        let sc_x = rect.corner.x;
        let sc_y = rect.corner.y;
        let sc_w = rect.extent.width as i32;
        let sc_h = rect.extent.height as i32;
        let max_x = sc_x + sc_w;
        let max_y = sc_y + sc_h;

        if sc_x <= 0 && sc_y <= 0 {
            x >= 0 && x < max_x && y >= 0 && y < max_y
        } else {
            x >= sc_x && x < max_x && y >= sc_y && y < max_y
        }
    } else {
        true  // No scissor active
    }
}

/// Draw a line between two points.
///
/// This forwards to the canvas primitive implementation.
pub fn draw_line(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Color,
    mode: DrawMode,
) -> Result<(), CanvasError> {
    canvas.draw_line(x1, y1, x2, y2, color, mode)
}

/// Draw a rectangle outline between two corners.
///
/// Uses four draw_line calls to create a 1-pixel outline.
///
/// Note: DrawMode is currently not respected and will be ignored.
/// Future implementation may blend colors based on mode.
pub fn draw_rect(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Color,
    mode: DrawMode,
) -> Result<(), CanvasError> {
    canvas.draw_line(x1, y1, x2, y1, color, mode)?;
    canvas.draw_line(x2, y1, x2, y2, color, mode)?;
    canvas.draw_line(x2, y2, x1, y2, color, mode)?;
    canvas.draw_line(x1, y2, x1, y1, color, mode)?;
    Ok(())
}

/// Fill a rectangle between two corners with solid color.
///
/// Efficient row-by-row fill using direct pixel access with scissor support.
pub fn fill_rect(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Color,
) -> Result<(), CanvasError> {
    check_canvas(canvas)?;
    
    let width = canvas.width();
    let height = canvas.height();
    let bytes_per_pixel = canvas.format().bytes_per_pixel as usize;
    
    // Compute unclamped bounds first
    let x_unclamped_start = x1.min(x2);
    let x_unclamped_end = x1.max(x2);
    let y_unclamped_start = y1.min(y2);
    let y_unclamped_end = y1.max(y2);
    
    // Early exit if entirely outside canvas
    if x_unclamped_end < 0 || x_unclamped_start >= width ||
       y_unclamped_end < 0 || y_unclamped_start >= height {
        return Ok(());
    }
    
    // Clamp to canvas bounds
    let x_start = x_unclamped_start.max(0);
    let x_end = x_unclamped_end.min(width - 1);
    let y_start = y_unclamped_start.max(0);
    let y_end = y_unclamped_end.min(height - 1);
    
    let color_bytes = [color.r, color.g, color.b, color.a];
    
    // Get scissor rect before entering closure to avoid borrow conflict
    let scissor_opt = canvas.scissor().rect;
    
    // Fill row by row with scissor check
    canvas.with_pixels_mut(|pixels| {
        for y in y_start..=y_end {
            // Check if this row is in scissor (or if scissor is disabled)
            let row_in_scissor = if let Some(ref rect) = scissor_opt {
                let sc_y = rect.corner.y;
                let sc_h = rect.extent.height as i32;
                y >= sc_y && y < sc_y + sc_h
            } else {
                true
            };
            
            if !row_in_scissor {
                continue;
            }
            
            for x in x_start..=x_end {
                // Check scissor before setting pixel
                let in_scissor = if let Some(ref rect) = scissor_opt {
                    let sc_x = rect.corner.x;
                    let sc_y = rect.corner.y;
                    let sc_w = rect.extent.width as i32;
                    let sc_h = rect.extent.height as i32;
                    x >= sc_x && x < sc_x + sc_w && y >= sc_y && y < sc_y + sc_h
                } else {
                    true
                };
                
                if in_scissor {
                    let offset = (y * width + x) as usize * bytes_per_pixel;
                    
                    // Write color respecting format
                    for i in 0..bytes_per_pixel {
                        if offset + i < pixels.len() {
                            pixels[offset + i] = color_bytes[i];
                        }
                    }
                }
            }
        }
        Ok(())
    })?
}

/// Copy pixels from source canvas to destination canvas.
///
/// Copies a rectangular region from src_canvas to dst_canvas at the given
/// destination position. This is used for blitting sprites, copying between
/// screens, and other image operations.
///
/// Parameters:
/// - `dst`: Destination canvas
/// - `src`: Source canvas
/// - `dst_x,dst_y`: Destination position on dst canvas
/// - `src_x,src_y`: Source position on src canvas (default: 0,0)
/// - `width,height`: Size to copy (default: entire source)
///
/// Returns:
/// - `Ok(())` - Copy completed successfully
/// - `Err(CanvasError)` - Copy failed (invalid canvas, mismatched formats, etc.)
///
/// Notes:
/// - Clipping is applied - areas outside either canvas are skipped
/// - Scissor clipping is applied on destination canvas
/// - Canvas formats must match (e.g., both RGBA)
/// - No blending performed - direct pixel copy
pub fn copy_canvas(
    dst: &mut Canvas,
    src: &Canvas,
    dst_x: i32,
    dst_y: i32,
    src_x: i32,
    src_y: i32,
    width: i32,
    height: i32,
) -> Result<(), CanvasError> {
    check_canvas(dst)?;
    check_canvas(src)?;
    // Verify formats match (or convert)
    if dst.format() != src.format() {
        return Err(CanvasError::FormatMismatch);
    }

    let dst_width = dst.width();
    let dst_height = dst.height();
    let src_width = src.width();
    let src_height = src.height();
    let bytes_per_pixel = dst.format().bytes_per_pixel as usize;

    let copy_width = if width <= 0 { src_width } else { width };
    let copy_height = if height <= 0 { src_height } else { height };

    let copy_w = copy_width;
    let copy_h = copy_height;

    if copy_w <= 0 || copy_h <= 0 {
        return Ok(());
    }

    let src_start_x = src_x;
    let src_start_y = src_y;
    let src_end_x = src_x + copy_w;
    let src_end_y = src_y + copy_h;
    let dst_start_x = dst_x;
    let dst_start_y = dst_y;
    let dst_end_x = dst_x + copy_w;
    let dst_end_y = dst_y + copy_h;

    let src_clip_x1 = src_start_x.max(0).min(src_width);
    let src_clip_y1 = src_start_y.max(0).min(src_height);
    let src_clip_x2 = src_end_x.max(0).min(src_width);
    let src_clip_y2 = src_end_y.max(0).min(src_height);
    let dst_clip_x1 = dst_start_x.max(0).min(dst_width);
    let dst_clip_y1 = dst_start_y.max(0).min(dst_height);
    let dst_clip_x2 = dst_end_x.max(0).min(dst_width);
    let dst_clip_y2 = dst_end_y.max(0).min(dst_height);

    let copy_w = (src_clip_x2 - src_clip_x1).min(dst_clip_x2 - dst_clip_x1);
    let copy_h = (src_clip_y2 - src_clip_y1).min(dst_clip_y2 - dst_clip_y1);

    if copy_w <= 0 || copy_h <= 0 {
        return Ok(());
    }

    let dst_start_x = dst_clip_x1;
    let dst_start_y = dst_clip_y1;
    let src_start_x = src_clip_x1;
    let src_start_y = src_clip_y1;

    let scissor_opt = dst.scissor().rect;
    let src_pixels = src.pixels();

    dst.with_pixels_mut(|dst_pixels| {
        for y in 0..copy_h {
            let src_y = src_start_y + y;
            let dst_y = dst_start_y + y;

            for x in 0..copy_w {
                let src_x = src_start_x + x;
                let dst_x = dst_start_x + x;

                let in_scissor = if let Some(ref rect) = scissor_opt {
                    let sc_x = rect.corner.x;
                    let sc_y = rect.corner.y;
                    let sc_w = rect.extent.width as i32;
                    let sc_h = rect.extent.height as i32;
                    dst_x >= sc_x && dst_x < sc_x + sc_w && dst_y >= sc_y && dst_y < sc_y + sc_h
                } else {
                    true
                };

                if in_scissor {
                    let src_row_offset = (src_y * src_width + src_x) as usize * bytes_per_pixel;
                    let dst_row_offset = (dst_y * dst_width + dst_x) as usize * bytes_per_pixel;

                    for i in 0..bytes_per_pixel {
                        dst_pixels[dst_row_offset + i] = src_pixels[src_row_offset + i];
                    }
                }
            }
        }
    })?;

    Ok(())
}

/// Flags for draw_image() operations
pub mod draw_image_flags {
    /// No special flags
    pub const NONE: u32 = 0;
    
    /// Flip image horizontally
    pub const FLIP_X: u32 = 0x01;
    
    /// Flip image vertically
    pub const FLIP_Y: u32 = 0x02;
    
    /// Rotate 90 degrees clockwise
    pub const ROTATE_90: u32 = 0x04;
    
    /// Rotate 180 degrees
    pub const ROTATE_180: u32 = 0x08;
    
    /// Rotate 270 degrees clockwise
    pub const ROTATE_270: u32 = 0x0C;
    
    /// Apply color map to image (not yet implemented)
    pub const COLORMAP: u32 = 0x10;
    
    /// Mask for rotation bits
    pub const ROTATE_MASK: u32 = 0x0C;
}

/// Image drawing flags (partially implemented).
///
/// Currently, all flags are recognized but not fully applied to rendering:
/// - `FLIP_HORIZONTAL`, `FLIP_VERTICAL`: Recognized but not applied
/// - `ROTATE_90`/`ROTATE_180`/`ROTATE_270`: Recognized but not applied
/// - `USE_COLORMAP`: Recognized but not applied (colors rendered normally)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageFlags(u32);

impl ImageFlags {
    pub const NONE: u32 = 0;
    pub const FLIP_HORIZONTAL: u32 = 1 << 0;
    pub const FLIP_VERTICAL: u32 = 1 << 1;
    pub const ROTATE_90: u32 = 1 << 2;
    pub const ROTATE_180: u32 = 1 << 3;
    pub const ROTATE_270: u32 = 1 << 4;
    pub const USE_COLORMAP: u32 = 1 << 5;

    #[must_use]
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn flip_horizontal(self) -> bool {
        self.0 & Self::FLIP_HORIZONTAL != 0
    }

    #[must_use]
    pub const fn flip_vertical(self) -> bool {
        self.0 & Self::FLIP_VERTICAL != 0
    }

    #[must_use]
    pub const fn rotation(self) -> Rotation {
        if self.0 & Self::ROTATE_90 != 0 {
            Rotation::Degrees90
        } else if self.0 & Self::ROTATE_180 != 0 {
            Rotation::Degrees180
        } else if self.0 & Self::ROTATE_270 != 0 {
            Rotation::Degrees270
        } else {
            Rotation::None
        }
    }

    #[must_use]
    pub const fn use_colormap(self) -> bool {
        self.0 & Self::USE_COLORMAP != 0
    }
}

/// Rotation angle for image drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,
    Degrees90,
    Degrees180,
    Degrees270,
}

/// Draw an image to a canvas.
///
/// Blits the primary (normal) canvas from a TFImage to the destination
fn rescale_image(
    image: &TFImage,
    scale: i32,
    scale_mode: ScaleMode,
) -> Result<Canvas, CanvasError> {
    let source = image
        .normal()
        .ok_or_else(|| CanvasError::InvalidOperation("TFImage has no primary canvas".to_string()))?;
    let source = ensure_canvas_truecolor(&source)?;

    let scaled_extent = scaled_extent_for_canvas(&source, image.normal_hot_spot(), scale, scale_mode)?;
    let mut target = Canvas::new(scaled_extent, CanvasFormat::rgba());

    match scale_mode {
        ScaleMode::Nearest | ScaleMode::Step => {
            rescale_nearest(&source, &mut target)?;
        }
        ScaleMode::Bilinear => {
            rescale_bilinear(&source, &mut target)?;
        }
        ScaleMode::Trilinear => {
            let mipmap = image.mipmap().ok_or(CanvasError::InvalidOperation(
                "Trilinear scaling requires mipmap".to_string(),
            ))?;
            let mipmap = ensure_canvas_truecolor(&mipmap)?;
            rescale_trilinear(&source, &mipmap, &mut target)?;
        }
    }

    Ok(target)
}

fn scaled_extent_for_canvas(
    canvas: &Canvas,
    hotspot: HotSpot,
    scale: i32,
    scale_mode: ScaleMode,
) -> Result<Extent, CanvasError> {
    if scale <= 0 {
        return Err(CanvasError::InvalidOperation("Scale factor must be > 0".to_string()));
    }
    let width = canvas.width();
    let height = canvas.height();
    if width <= 0 || height <= 0 {
        return Err(CanvasError::InvalidRect);
    }
    let scaled_w = ((width as i64 * scale as i64) + 255) / 256;
    let scaled_h = ((height as i64 * scale as i64) + 255) / 256;
    let mut scaled_w = scaled_w.max(1) as i32;
    let mut scaled_h = scaled_h.max(1) as i32;

    if scale_mode != ScaleMode::Nearest {
        let hs_x = (hotspot.x * scale) & 0xFF;
        let hs_y = (hotspot.y * scale) & 0xFF;
        if hs_x != 0 {
            scaled_w += 1;
        }
        if hs_y != 0 {
            scaled_h += 1;
        }
    }

    Ok(Extent::new(scaled_w, scaled_h))
}

fn rescale_nearest(src: &Canvas, dst: &mut Canvas) -> Result<(), CanvasError> {
    check_canvas(src)?;
    check_canvas(dst)?;
    if src.format() != dst.format() {
        return Err(CanvasError::FormatMismatch);
    }

    let src_width = src.width();
    let src_height = src.height();
    let dst_width = dst.width();
    let dst_height = dst.height();
    let bytes_per_pixel = src.format().bytes_per_pixel as usize;

    let src_pixels = src.pixels();
    dst.with_pixels_mut(|dst_pixels| {
        for y in 0..dst_height {
            let src_y = (y * src_height) / dst_height;
            for x in 0..dst_width {
                let src_x = (x * src_width) / dst_width;
                let src_offset = (src_y * src_width + src_x) as usize * bytes_per_pixel;
                let dst_offset = (y * dst_width + x) as usize * bytes_per_pixel;
                dst_pixels[dst_offset..dst_offset + bytes_per_pixel]
                    .copy_from_slice(&src_pixels[src_offset..src_offset + bytes_per_pixel]);
            }
        }
    })?;
    Ok(())
}

fn rescale_bilinear(src: &Canvas, dst: &mut Canvas) -> Result<(), CanvasError> {
    check_canvas(src)?;
    check_canvas(dst)?;
    if src.format() != dst.format() {
        return Err(CanvasError::FormatMismatch);
    }

    let src_width = src.width().max(1);
    let src_height = src.height().max(1);
    let dst_width = dst.width().max(1);
    let dst_height = dst.height().max(1);
    let bytes_per_pixel = src.format().bytes_per_pixel as usize;

    let src_pixels = src.pixels();
    dst.with_pixels_mut(|dst_pixels| {
        for y in 0..dst_height {
            let src_y = y as f32 * (src_height - 1) as f32 / (dst_height - 1).max(1) as f32;
            let y0 = src_y.floor() as i32;
            let y1 = (y0 + 1).min(src_height - 1);
            let fy = src_y - y0 as f32;
            for x in 0..dst_width {
                let src_x = x as f32 * (src_width - 1) as f32 / (dst_width - 1).max(1) as f32;
                let x0 = src_x.floor() as i32;
                let x1 = (x0 + 1).min(src_width - 1);
                let fx = src_x - x0 as f32;
                let offsets = [
                    ((y0 * src_width + x0) as usize * bytes_per_pixel),
                    ((y0 * src_width + x1) as usize * bytes_per_pixel),
                    ((y1 * src_width + x0) as usize * bytes_per_pixel),
                    ((y1 * src_width + x1) as usize * bytes_per_pixel),
                ];
                let dst_offset = (y * dst_width + x) as usize * bytes_per_pixel;
                for channel in 0..bytes_per_pixel {
                    let p00 = src_pixels[offsets[0] + channel] as f32;
                    let p10 = src_pixels[offsets[1] + channel] as f32;
                    let p01 = src_pixels[offsets[2] + channel] as f32;
                    let p11 = src_pixels[offsets[3] + channel] as f32;
                    let top = p00 + (p10 - p00) * fx;
                    let bottom = p01 + (p11 - p01) * fx;
                    let value = top + (bottom - top) * fy;
                    dst_pixels[dst_offset + channel] = value.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    })?;
    Ok(())
}

fn rescale_trilinear(
    src: &Canvas,
    mipmap: &Canvas,
    dst: &mut Canvas,
) -> Result<(), CanvasError> {
    check_canvas(src)?;
    check_canvas(mipmap)?;
    check_canvas(dst)?;
    if src.format() != dst.format() || mipmap.format() != dst.format() {
        return Err(CanvasError::FormatMismatch);
    }

    let mut dst_from_src = Canvas::new(dst.extent(), dst.format());
    let mut dst_from_mipmap = Canvas::new(dst.extent(), dst.format());
    rescale_bilinear(src, &mut dst_from_src)?;
    rescale_bilinear(mipmap, &mut dst_from_mipmap)?;

    let src_pixels = dst_from_src.pixels();
    let mip_pixels = dst_from_mipmap.pixels();
    dst.with_pixels_mut(|dst_pixels| {
        for i in 0..dst_pixels.len() {
            let src_val = src_pixels[i] as f32;
            let mip_val = mip_pixels[i] as f32;
            let blended = (src_val + mip_val) * 0.5;
            dst_pixels[i] = blended.round().clamp(0.0, 255.0) as u8;
        }
    })?;
    Ok(())
}

pub fn draw_scaled_image(
    canvas: &mut Canvas,
    image: &TFImage,
    x: i32,
    y: i32,
    scale: i32,
    scale_mode: ScaleMode,
    flags: u32,
) -> Result<(), CanvasError> {
    check_canvas(canvas)?;

    let image_flags = ImageFlags::new(flags);
    if image_flags.bits() != 0 {
        log::warn!(
            "draw_image flags not fully implemented: 0x{:x} (flip_h={}, flip_v={}, rotation={:?}, colormap={})",
            image_flags.bits(),
            image_flags.flip_horizontal(),
            image_flags.flip_vertical(),
            image_flags.rotation(),
            image_flags.use_colormap()
        );
    }

    let draw_scale = if scale == 0 { 256 } else { scale };
    let active_image = image.current_frame();
    let active_image_ref = active_image.as_deref().unwrap_or(image);

    let (source_canvas, source_hotspot) = if draw_scale != 256 {
        if active_image_ref.scaling_cache_valid(draw_scale, scale_mode) {
            if let Some(cached) = active_image_ref.scaled() {
                let hotspot = active_image_ref.normal_hot_spot();
                (cached, hotspot)
            } else {
                (
                    active_image_ref.normal().ok_or_else(|| {
                        CanvasError::InvalidOperation("TFImage has no primary canvas".to_string())
                    })?,
                    active_image_ref.normal_hot_spot(),
                )
            }
        } else {
            let scaled = rescale_image(active_image_ref, draw_scale, scale_mode)?;
            active_image_ref.set_scaled(Some(scaled.clone()));
            active_image_ref.update_scaling_cache(draw_scale, scale_mode);
            active_image_ref.mark_clean();
            (scaled, active_image_ref.normal_hot_spot())
        }
    } else {
        (
            active_image_ref.normal().ok_or_else(|| {
                CanvasError::InvalidOperation("TFImage has no primary canvas".to_string())
            })?,
            active_image_ref.normal_hot_spot(),
        )
    };

    let draw_x = x - source_hotspot.x;
    let draw_y = y - source_hotspot.y;

    copy_canvas(canvas, &source_canvas, draw_x, draw_y, 0, 0, -1, -1)
}

pub fn draw_filled_image(
    canvas: &mut Canvas,
    image: &TFImage,
    x: i32,
    y: i32,
    fill_color: Color,
    scale: i32,
    scale_mode: ScaleMode,
    flags: u32,
) -> Result<(), CanvasError> {
    let _ = flags;
    let draw_scale = if scale == 0 { 256 } else { scale };
    let active_image = image.current_frame();
    let active_image_ref = active_image.as_deref().unwrap_or(image);

    let source = active_image_ref
        .normal()
        .ok_or_else(|| CanvasError::InvalidOperation("TFImage has no primary canvas".to_string()))?;
    let source = ensure_canvas_truecolor(&source)?;

    let extent = if draw_scale == 256 {
        source.extent()
    } else {
        scaled_extent_for_canvas(&source, active_image_ref.normal_hot_spot(), draw_scale, scale_mode)?
    };

    let mut filled = Canvas::new(extent, CanvasFormat::rgba());
    fill_rect(&mut filled, 0, 0, extent.width - 1, extent.height - 1, fill_color)?;

    if draw_scale == 256 {
        copy_canvas(&mut filled, &source, 0, 0, 0, 0, -1, -1)?;
    } else {
        let scaled = rescale_image(active_image_ref, draw_scale, scale_mode)?;
        copy_canvas(&mut filled, &scaled, 0, 0, 0, 0, -1, -1)?;
    }

    active_image_ref.set_filled(Some(filled.clone()));
    let draw_x = x - active_image_ref.normal_hot_spot().x;
    let draw_y = y - active_image_ref.normal_hot_spot().y;
    copy_canvas(canvas, &filled, draw_x, draw_y, 0, 0, -1, -1)
}



/// canvas at the specified position. The image has a hot spot offset
/// for positioning sprites correctly.
///
/// Parameters:
/// - `canvas`: Destination canvas
/// - `image`: TFImage to draw
/// - `x,y`: Draw position on canvas (before applying hotspot)
/// - `flags`: Drawing flags from `draw_image_flags` module
///            (flip, rotate, color map - not all implemented yet)
///
/// Returns:
/// - `Ok(())` - Image drawn successfully
/// - `Err(CanvasError)` - Drawing failed
///
/// Notes:
/// - Hot spot offset is applied (image positioned at x - hs_x, y - hs_y)
/// - Multi-frame support: TFImage currently supports a single frame (the normal canvas)
/// - Color maps: Not yet implemented (see flags parameter)
///
/// Future enhancements may include:
/// - Multi-frame animation support
/// - Flip/rotate transformations
/// - Color map application
pub fn draw_image(
    canvas: &mut Canvas,
    image: &TFImage,
    x: i32,
    y: i32,
    flags: u32,
) -> Result<(), CanvasError> {
    draw_scaled_image(canvas, image, x, y, 256, ScaleMode::Nearest, flags)
}


/// Draw a single font character to a canvas.
///
/// Renders a character from a font page with alpha blending support.
/// The character bitmap data is transferred to the canvas, applying
/// the character's alpha channel for transparency.
///
/// # Parameters
///
/// - `canvas`: Destination canvas to draw to
/// - `fg_color`: Foreground color for the character
/// - `page`: Font page containing the character data
/// - `char_index`: Index of the character within the page
/// - `x`: X position for drawing (baseline position)
/// - `y`: Y position for drawing (baseline position)
/// - `use_pixmap`: If true, render with higher quality (currently unused)
///
/// # Returns
///
/// - `Ok(width)` - Returns the character's display width
/// - `Err(CanvasError)` - Drawing failed
///
/// # Notes
///
/// - Character bitmaps are stored as alpha-only data in `TFChar.data`
/// - The alpha channel is applied to the `fg_color` for each pixel
/// - Hot spot offsets are applied to position the glyph correctly
/// - Transparent pixels (alpha = 0) preserve the canvas background
/// - Clipping is performed based on canvas bounds and scissor region
pub fn draw_fontchar(
    canvas: &mut Canvas,
    fg_color: Color,
    page: &FontPage,
    char_code: u32,
    x: i32,
    y: i32,
    use_pixmap: bool,
) -> Result<usize, CanvasError> {
    check_canvas(canvas)?;
    
    // Get character descriptor from page (char_code is the Unicode code point)
    let tf_char = page.get_char(char_code)
        .ok_or_else(|| CanvasError::InvalidOperation(
            format!("Character code 0x{:04X} not found in font page", char_code)
        ))?;
    
    // Check if we have data to render
    let data = tf_char.data.as_ref().ok_or_else(|| CanvasError::InvalidOperation(
        "Character has no bitmap data".to_string()
    ))?;
    
    let extent_width = tf_char.extent.width as usize;
    let extent_height = tf_char.extent.height as usize;
    let disp_width = tf_char.disp.width as usize;
    let disp_height = tf_char.disp.height as usize;
    let pitch = tf_char.pitch as usize;
    
    // Calculate drawing position applying hot spot offset
    let draw_x = x - tf_char.hotspot.x as i32;
    let draw_y = y - tf_char.hotspot.y as i32;
    
    // Get canvas properties
    let canvas_width = canvas.width() as usize;
    let canvas_height = canvas.height() as usize;
    let bytes_per_pixel = canvas.format().bytes_per_pixel as usize;
    
    // Early exit if character has no dimensions or is off canvas
    if extent_width == 0 || extent_height == 0 || disp_width == 0 || disp_height == 0 {
        return Ok(disp_width);
    }
    
    // Get scissor rect
    let scissor_rect = canvas.scissor().rect;
    
    // Transfer alpha channel to destination pixels
    canvas.with_pixels_mut(|pixels| {
        let fg_bytes = [fg_color.r, fg_color.g, fg_color.b, fg_color.a];
        
        // Iterate through character bitmap
        for char_y in 0..disp_height {
            for char_x in 0..disp_width {
                let src_offset = char_y * pitch + char_x;
                
                if src_offset >= data.len() {
                    continue;
                }
                
                // Get glyph alpha from character bitmap
                let glyph_alpha = data[src_offset] as i32;
                
                // Skip fully transparent pixels
                if glyph_alpha == 0 {
                    continue;
                }
                
                // Calculate effective alpha combining glyph and foreground color
                // This allows semi-transparent text colors to work correctly
                let effective_alpha = (glyph_alpha * fg_color.a as i32) / 255;
                let alpha = effective_alpha.clamp(0, 255);
                
                // Skip if effective alpha is zero
                if alpha == 0 {
                    continue;
                }
                
                // Calculate destination position
                let dst_x = draw_x + char_x as i32;
                let dst_y = draw_y + char_y as i32;
                
                // Check canvas bounds
                if dst_x < 0 || dst_x >= canvas_width as i32 ||
                   dst_y < 0 || dst_y >= canvas_height as i32 {
                    continue;
                }
                
                // Check scissor clip (if enabled)
                if let Some(ref scissor) = scissor_rect {
                    let sc_x = scissor.corner.x;
                    let sc_y = scissor.corner.y;
                    let sc_w = scissor.extent.width as i32;
                    let sc_h = scissor.extent.height as i32;
                    
                    if dst_x < sc_x || dst_x >= sc_x + sc_w ||
                       dst_y < sc_y || dst_y >= sc_y + sc_h {
                        continue;
                    }
                }
                
                // Calculate destination pixel offset
                let dst_offset = (dst_y as usize * canvas_width + dst_x as usize) * bytes_per_pixel;
                
                // Apply color with alpha blending
                if bytes_per_pixel >= 4 {
                    // RGBA format: blend RGB channels with effective alpha,
                    // then blend alpha channel properly
                    let alpha_factor = alpha;
                    let inv_alpha = 255 - alpha_factor;
                    
                    // Blend RGB channels (0, 1, 2)
                    for i in 0..3 {
                        if dst_offset + i < pixels.len() {
                            let fg_val = fg_bytes[i] as i32;
                            let dst_val = pixels[dst_offset + i] as i32;
                            let blended = (fg_val * alpha_factor + dst_val * inv_alpha) / 255;
                            pixels[dst_offset + i] = blended as u8;
                        }
                    }
                    
                    // Blend alpha channel (3) using proper alpha compositing
                    // Standard alpha blend: result = src + dst * (1 - src_alpha)
                    if dst_offset + 3 < pixels.len() {
                        let dst_a = pixels[dst_offset + 3] as i32;
                        let result_alpha = alpha as i32 + (dst_a * inv_alpha) / 255;
                        pixels[dst_offset + 3] = result_alpha.min(255).max(0) as u8;
                    }
                } else if bytes_per_pixel == 3 {
                    // RGB without alpha channel - use effective alpha for all channels
                    let alpha_factor = alpha;
                    let inv_alpha = 255 - alpha_factor;
                    
                    for i in 0..3 {
                        if dst_offset + i < pixels.len() {
                            let fg_val = fg_bytes[i] as i32;
                            let dst_val = pixels[dst_offset + i] as i32;
                            let blended = (fg_val * alpha_factor + dst_val * inv_alpha) / 255;
                            pixels[dst_offset + i] = blended as u8;
                        }
                    }
                }
            }
        }
    })?;
    
    // Note: use_pixmap parameter is reserved for future high-quality rendering
    let _ = use_pixmap;
    
    Ok(disp_width)
}

fn draw_fontchar_from_ref(
    canvas: &mut Canvas,
    fontchar: FontCharRef,
    backing: Option<&TFImage>,
    x: i32,
    y: i32,
    mode: DrawMode,
) -> Result<(), CanvasError> {
    if let Some(backing) = backing {
        let hs = backing.normal_hot_spot();
        let draw_x = x - hs.x;
        let draw_y = y - hs.y;
        let backing_canvas = backing.normal().ok_or_else(|| {
            CanvasError::InvalidOperation("Backing image missing primary canvas".to_string())
        })?;
        copy_canvas(canvas, &backing_canvas, draw_x, draw_y, 0, 0, -1, -1)?;
    }

    let page = crate::graphics::global_state()
        .lock()
        .map_err(|_| CanvasError::InvalidOperation("Graphics state lock poisoned".to_string()))?
        .render_context()
        .read()
        .map_err(|_| CanvasError::InvalidOperation("Render context lock poisoned".to_string()))?
        .get_font_page(fontchar.page_id)
        .ok_or_else(|| {
            CanvasError::InvalidOperation(format!(
                "Font page {} not found",
                fontchar.page_id
            ))
        })?;

    draw_fontchar(
        canvas,
        Color::new(255, 255, 255, 255),
        &page,
        fontchar.char_code,
        x,
        y,
        false,
    )?;
    let _ = mode;
    Ok(())
}


pub trait CanvasPrimitive {
    fn draw_line(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color,
        mode: DrawMode,
    ) -> Result<(), CanvasError>;

    fn draw_rect(&self, rect: Rect, color: Color, mode: DrawMode) -> Result<(), CanvasError>;

    fn draw_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        scale: i32,
        scale_mode: ScaleMode,
        mode: DrawMode,
        flags: u32,
    ) -> Result<(), CanvasError>;

    fn draw_filled_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        fill_color: Color,
        scale: i32,
        scale_mode: ScaleMode,
        mode: DrawMode,
        flags: u32,
    ) -> Result<(), CanvasError>;

    fn draw_fontchar(
        &self,
        fontchar: FontCharRef,
        backing: Option<&TFImage>,
        x: i32,
        y: i32,
        mode: DrawMode,
    ) -> Result<(), CanvasError>;

    fn copy_rect(&self, source: &Canvas, src_rect: Rect, dst_pt: Point) -> Result<(), CanvasError>;
}

impl CanvasPrimitive for Canvas {
    fn draw_line(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color,
        _mode: DrawMode,
    ) -> Result<(), CanvasError> {
        check_canvas(self)?;

        let width = self.width();
        let height = self.height();
        let bytes_per_pixel = self.format().bytes_per_pixel;

        // Bresenham's line algorithm with i64 to prevent overflow
        let dx = (x2 as i64 - x1 as i64).abs();
        let dy = (y2 as i64 - y1 as i64).abs();

        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };

        let mut err = dx - dy;
        let mut x = x1;
        let mut y = y1;
        let color_bytes = [color.r, color.g, color.b, color.a];

        loop {
            // Check bounds AND scissor
            if x >= 0 && x < width && y >= 0 && y < height && is_in_scissor(self, x, y) {
                let offset = (y * width + x) as usize * bytes_per_pixel as usize;

                let mut inner = self.inner.lock().unwrap();
                let pixels = inner.pixels_mut();

                // Write color respecting format
                for i in 0..bytes_per_pixel.min(color_bytes.len() as i32) as usize {
                    if offset + i < pixels.len() {
                        pixels[offset + i] = color_bytes[i];
                    }
                }
            }

            if x == x2 && y == y2 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }

        Ok(())
    }

    fn draw_rect(&self, rect: Rect, color: Color, mode: DrawMode) -> Result<(), CanvasError> {
        let mut canvas = self.clone();
        let x1 = rect.corner.x;
        let y1 = rect.corner.y;
        let x2 = rect.corner.x + rect.extent.width - 1;
        let y2 = rect.corner.y + rect.extent.height - 1;
        draw_rect(&mut canvas, x1, y1, x2, y2, color, mode)
    }

    fn draw_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        scale: i32,
        scale_mode: ScaleMode,
        _mode: DrawMode,
        flags: u32,
    ) -> Result<(), CanvasError> {
        let mut canvas = self.clone();
        draw_scaled_image(&mut canvas, image, x, y, scale, scale_mode, flags)
    }

    fn draw_filled_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        fill_color: Color,
        scale: i32,
        scale_mode: ScaleMode,
        _mode: DrawMode,
        flags: u32,
    ) -> Result<(), CanvasError> {
        let mut canvas = self.clone();
        draw_filled_image(&mut canvas, image, x, y, fill_color, scale, scale_mode, flags)
    }

    fn draw_fontchar(
        &self,
        fontchar: FontCharRef,
        backing: Option<&TFImage>,
        x: i32,
        y: i32,
        mode: DrawMode,
    ) -> Result<(), CanvasError> {
        let mut canvas = self.clone();
        draw_fontchar_from_ref(&mut canvas, fontchar, backing, x, y, mode)
    }

    fn copy_rect(&self, source: &Canvas, src_rect: Rect, dst_pt: Point) -> Result<(), CanvasError> {
        self.copy_rect(source, src_rect, dst_pt)
    }
}

pub trait ImagePrimitive {
    fn draw_line(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color,
        mode: DrawMode,
    ) -> Result<(), TFImageError>;

    fn draw_rect(&self, rect: Rect, color: Color, mode: DrawMode) -> Result<(), TFImageError>;

    fn draw_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        scale: i32,
        scale_mode: ScaleMode,
        mode: DrawMode,
        flags: u32,
    ) -> Result<(), TFImageError>;

    fn draw_filled_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        fill_color: Color,
        scale: i32,
        scale_mode: ScaleMode,
        mode: DrawMode,
        flags: u32,
    ) -> Result<(), TFImageError>;

    fn draw_fontchar(
        &self,
        fontchar: FontCharRef,
        backing: Option<&TFImage>,
        x: i32,
        y: i32,
        mode: DrawMode,
    ) -> Result<(), TFImageError>;

    fn copy_rect(
        &self,
        source: &TFImage,
        src_rect: Rect,
        dst_pt: Point,
    ) -> Result<(), TFImageError>;
}

impl ImagePrimitive for TFImage {
    fn draw_line(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color,
        mode: DrawMode,
    ) -> Result<(), TFImageError> {
        let canvas = self.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        canvas.draw_line(x1, y1, x2, y2, color, mode)?;
        self.mark_dirty();
        Ok(())
    }

    fn draw_rect(&self, rect: Rect, color: Color, mode: DrawMode) -> Result<(), TFImageError> {
        let canvas = self.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        canvas.draw_rect(rect, color, mode)?;
        self.mark_dirty();
        Ok(())
    }

    fn draw_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        scale: i32,
        scale_mode: ScaleMode,
        mode: DrawMode,
        flags: u32,
    ) -> Result<(), TFImageError> {
        let canvas = self.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        canvas.draw_image(image, x, y, scale, scale_mode, mode, flags)?;
        self.mark_dirty();
        Ok(())
    }

    fn draw_filled_image(
        &self,
        image: &TFImage,
        x: i32,
        y: i32,
        fill_color: Color,
        scale: i32,
        scale_mode: ScaleMode,
        mode: DrawMode,
        flags: u32,
    ) -> Result<(), TFImageError> {
        let canvas = self.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        canvas.draw_filled_image(image, x, y, fill_color, scale, scale_mode, mode, flags)?;
        self.mark_dirty();
        Ok(())
    }

    fn draw_fontchar(
        &self,
        fontchar: FontCharRef,
        backing: Option<&TFImage>,
        x: i32,
        y: i32,
        mode: DrawMode,
    ) -> Result<(), TFImageError> {
        let canvas = self.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        canvas.draw_fontchar(fontchar, backing, x, y, mode)?;
        self.mark_dirty();
        Ok(())
    }

    fn copy_rect(
        &self,
        source: &TFImage,
        src_rect: Rect,
        dst_pt: Point,
    ) -> Result<(), TFImageError> {
        let target_canvas = self.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        let source_canvas = source.normal().ok_or(TFImageError::NoPrimaryCanvas)?;
        target_canvas.copy_rect(&source_canvas, src_rect, dst_pt)?;
        self.mark_dirty();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_creation() {
        let canvas = Canvas::new_rgba(100, 50);
        assert_eq!(canvas.width(), 100);
        assert_eq!(canvas.height(), 50);
        assert_eq!(canvas.format().bytes_per_pixel, 4);
    }

    #[test]
    fn test_canvas_lock_unlock() {
        let canvas = Canvas::new_rgba(100, 50);
        assert!(!canvas.is_locked());
        assert!(canvas.lock().is_ok());
        assert!(canvas.is_locked());
        assert!(canvas.unlock().is_ok());
        assert!(!canvas.is_locked());
    }

    #[test]
    fn test_draw_horizontal_line() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let red = Color::rgb(255, 0, 0);
        let mode = DrawMode::Normal;

        canvas.draw_line(2, 5, 7, 5, red, mode).unwrap();

        let pixels = canvas.pixels();
        // Check that pixels (2,5) through (7,5) are red
        for x in 2..=7 {
            let offset = (5 * 10 + x) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
            assert_eq!(pixels[offset + 1], 0); // G
            assert_eq!(pixels[offset + 2], 0); // B
            assert_eq!(pixels[offset + 3], 255); // A
        }
    }

    #[test]
    fn test_draw_vertical_line() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let green = Color::rgb(0, 255, 0);
        let mode = DrawMode::Normal;

        canvas.draw_line(5, 2, 5, 7, green, mode).unwrap();

        let pixels = canvas.pixels();
        // Check that pixels (5,2) through (5,7) are green
        for y in 2..=7 {
            let offset = (y * 10 + 5) as usize * 4;
            assert_eq!(pixels[offset], 0); // R
            assert_eq!(pixels[offset + 1], 255); // G
            assert_eq!(pixels[offset + 2], 0); // B
            assert_eq!(pixels[offset + 3], 255); // A
        }
    }

    #[test]
    fn test_draw_diagonal_line() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let blue = Color::rgb(0, 0, 255);
        let mode = DrawMode::Normal;

        canvas.draw_line(2, 2, 7, 7, blue, mode).unwrap();

        let pixels = canvas.pixels();
        // Check that diagonal pixels (2,2) through (7,7) are blue
        for i in 0..6 {
            let x = 2 + i;
            let y = 2 + i;
            let offset = (y * 10 + x) as usize * 4;
            assert_eq!(pixels[offset], 0); // R
            assert_eq!(pixels[offset + 1], 0); // G
            assert_eq!(pixels[offset + 2], 255); // B
            assert_eq!(pixels[offset + 3], 255); // A
        }
    }

    #[test]
    fn test_draw_line_partial_clip() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let white = Color::rgb(255, 255, 255);
        let mode = DrawMode::Normal;

        // Line that starts outside and ends inside
        canvas.draw_line(-5, 5, 5, 5, white, mode).unwrap();

        let pixels = canvas.pixels();
        // Only pixels from (0,5) to (5,5) should be drawn
        for x in 0..=5 {
            let offset = (5 * 10 + x) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
            assert_eq!(pixels[offset + 1], 255); // G
            assert_eq!(pixels[offset + 2], 255); // B
            assert_eq!(pixels[offset + 3], 255); // A
        }
    }

    #[test]
    fn test_double_lock_error() {
        let canvas = Canvas::new_rgba(100, 50);
        assert!(canvas.lock().is_ok());
        let result = canvas.lock();
        assert!(matches!(result, Err(CanvasError::AlreadyLocked)));
    }
    #[test]
    fn test_tfimage_multi_frame_draw() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut frame_a = Canvas::new_rgba(4, 4);
        let mut frame_b = Canvas::new_rgba(4, 4);

        fill_rect(&mut frame_a, 0, 0, 3, 3, Color::rgb(255, 0, 0)).unwrap();
        fill_rect(&mut frame_b, 0, 0, 3, 3, Color::rgb(0, 0, 255)).unwrap();

        let base = TFImage::new(frame_a);
        let second = Arc::new(TFImage::new(frame_b));
        base.add_frame(Arc::clone(&second));

        base.set_frame_index(0);
        draw_image(&mut dst, &base, 0, 0, 0).unwrap();
        let pixels = dst.pixels();
        assert_eq!(pixels[0], 255);

        base.set_frame_index(1);
        draw_image(&mut dst, &base, 5, 0, 0).unwrap();
        let pixels = dst.pixels();
        let offset = (0 * dst.width() + 5) as usize * 4;
        assert_eq!(pixels[offset + 2], 255);
    }



    #[test]
    fn test_unlock_without_lock_error() {
        let canvas = Canvas::new_rgba(100, 50);
        let result = canvas.unlock();
        assert!(matches!(result, Err(CanvasError::NotLocked)));
    }

    #[test]
    fn test_tfimage_creation() {
        let canvas = Canvas::new_rgba(100, 50);
        let image = TFImage::new(canvas);
        assert_eq!(image.width(), Some(100));
        assert_eq!(image.height(), Some(50));
        assert_eq!(image.frame_count(), 1);
        assert!(!image.is_dirty());
    }

    #[test]
    fn test_tfimage_dirty_flag() {
        let canvas = Canvas::new_rgba(100, 50);
        let image = TFImage::new(canvas);
        assert!(!image.is_dirty());
        image.mark_dirty();
        assert!(image.is_dirty());
        image.mark_clean();
        assert!(!image.is_dirty());
    }

    #[test]
    fn test_hotspot() {
        let hs = HotSpot::new(10, 20);
        assert_eq!(hs.x, 10);
        assert_eq!(hs.y, 20);
        assert!(!hs.is_origin());

        let origin = HotSpot::origin();
        assert_eq!(origin.x, 0);
        assert_eq!(origin.y, 0);
        assert!(origin.is_origin());
    }
    #[test]
    fn test_paletted_to_rgba_conversion() {
        let mut palette = default_palette();
        palette[1] = Color::new(255, 0, 0, 255);
        palette[2] = Color::new(0, 255, 0, 255);

        let mut paletted = Canvas::new_paletted(2, 1, palette);
        paletted.set_transparent_index(Some(2));
        paletted
            .with_pixels_mut(|pixels| {
                pixels[0] = 1;
                pixels[1] = 2;
                Ok(())
            })
            .unwrap();

        let rgba = paletted_to_rgba(&paletted).unwrap();
        let pixels = rgba.pixels();
        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(&pixels[4..8], &[0, 255, 0, 0]);
    }

    #[test]
    fn test_convert_canvas_format_to_paletted() {
        let mut rgba = Canvas::new_rgba(1, 1);
        rgba
            .with_pixels_mut(|pixels| {
                pixels[0] = 255;
                pixels[1] = 0;
                pixels[2] = 0;
                pixels[3] = 255;
                Ok(())
            })
            .unwrap();

        let mut palette = default_palette();
        palette[5] = Color::new(255, 0, 0, 255);

        let paletted = convert_canvas_format(&rgba, CanvasFormat::paletted(), Some(palette)).unwrap();
        assert!(paletted.is_paletted());
        assert_eq!(paletted.pixels()[0], 5);
    }

    #[test]
    fn test_trilinear_scaling_blends_mipmap() {
        let mut base = Canvas::new_rgba(2, 2);
        let mut mip = Canvas::new_rgba(2, 2);
        fill_rect(&mut base, 0, 0, 1, 1, Color::rgb(255, 0, 0)).unwrap();
        fill_rect(&mut mip, 0, 0, 1, 1, Color::rgb(0, 0, 255)).unwrap();

        let image = TFImage::new(base);
        image.set_mipmap(Some(mip), HotSpot::origin());

        let scaled = rescale_image(&image, 256, ScaleMode::Trilinear).unwrap();
        let pixels = scaled.pixels();
        assert_eq!(pixels[0], 128);
        assert_eq!(pixels[2], 128);
    }

    #[test]
    fn test_fix_scaling_respects_dirty_flag() {
        let mut canvas = Canvas::new_rgba(2, 2);
        fill_rect(&mut canvas, 0, 0, 1, 1, Color::rgb(255, 0, 0)).unwrap();
        let image = TFImage::new(canvas);
        image.mark_dirty();
        image.fix_scaling(128, ScaleMode::Nearest).unwrap();
        assert!(!image.is_dirty());
        assert!(image.scaled().is_some());
    }



    #[test]
    fn test_image_primitive_delegates_to_canvas() {
        let canvas = Canvas::new_rgba(10, 10);
        let mut image = TFImage::new(canvas);
        let red = Color::rgb(255, 0, 0);
        let mode = DrawMode::Normal;

        image.draw_line(2, 5, 7, 5, red, mode).unwrap();

        assert!(image.is_dirty());
        let canvas = image.normal().unwrap();
        let pixels = canvas.pixels();

        // Check that the line was drawn on the underlying canvas
        for x in 2..=7 {
            let offset = (5 * 10 + x) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
        }
    }
    #[test]
    fn test_draw_rect() {
        let mut canvas = Canvas::new_rgba(20, 20);
        let red = Color::rgb(255, 0, 0);
        let mode = DrawMode::Normal;

        // Draw a rectangle from (5,5) to (15,15)
        draw_rect(&mut canvas, 5, 5, 15, 15, red, mode).unwrap();

        let pixels = canvas.pixels();
        let width = canvas.width();

        // Check top edge (y=5, x=5 to 15)
        for x in 5..=15 {
            let offset = (5 * width + x) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
        }

        // Check bottom edge (y=15, x=5 to 15)
        for x in 5..=15 {
            let offset = (15 * width + x) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
        }

        // Check left edge (x=5, y=5 to 15)
        for y in 5..=15 {
            let offset = (y * width + 5) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
        }

        // Check right edge (x=15, y=5 to 15)
        for y in 5..=15 {
            let offset = (y * width + 15) as usize * 4;
            assert_eq!(pixels[offset], 255); // R
        }
    }

    #[test]
    fn test_draw_rect_swapped_coords() {
        let mut canvas = Canvas::new_rgba(20, 20);
        let green = Color::rgb(0, 255, 0);
        let mode = DrawMode::Normal;

        // Draw a rectangle with swapped coordinates (x2 < x1, y2 < y1)
        draw_rect(&mut canvas, 15, 15, 5, 5, green, mode).unwrap();

        let pixels = canvas.pixels();
        let width = canvas.width();

        // Should still draw the same rectangle
        for x in 5..=15 {
            // Check corners are green
            let top_offset = (5 * width + x) as usize * 4;
            let bottom_offset = (15 * width + x) as usize * 4;
            assert_eq!(pixels[top_offset], 0); // R
            assert_eq!(pixels[top_offset + 1], 255); // G
            assert_eq!(pixels[bottom_offset], 0); // R
            assert_eq!(pixels[bottom_offset + 1], 255); // G
        }
    }

    #[test]
    fn test_draw_rect_outside_canvas() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let blue = Color::rgb(0, 0, 255);
        let mode = DrawMode::Normal;

        // Draw a rectangle completely outside canvas bounds
        draw_rect(&mut canvas, 15, 15, 25, 25, blue, mode).unwrap();

        let pixels = canvas.pixels();

        // All pixels should remain black
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 0); // R
        }
    }

    #[test]
    fn test_fill_rect_small() {
        let mut canvas = Canvas::new_rgba(20, 20);
        let blue = Color::rgb(0, 0, 255);

        // Fill a small rectangle from (5,5) to (10,10)
        fill_rect(&mut canvas, 5, 5, 10, 10, blue).unwrap();

        let pixels = canvas.pixels();
        let width = canvas.width();

        // Check all pixels in the filled area are blue
        for y in 5..=10 {
            for x in 5..=10 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 0); // R
                assert_eq!(pixels[offset + 1], 0); // G
                assert_eq!(pixels[offset + 2], 255); // B
                assert_eq!(pixels[offset + 3], 255); // A
            }
        }
    }

    #[test]
    fn test_fill_rect_large() {
        let mut canvas = Canvas::new_rgba(20, 20);
        let white = Color::rgb(255, 255, 255);

        // Fill a large portion of the canvas
        fill_rect(&mut canvas, 2, 2, 18, 18, white).unwrap();

        let pixels = canvas.pixels();
        let width = canvas.width();

        // Check filled area
        for y in 2..=18 {
            for x in 2..=18 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 255); // R
                assert_eq!(pixels[offset + 1], 255); // G
                assert_eq!(pixels[offset + 2], 255); // B
            }
        }

        // Check border is still black
        for y in 0..1 {
            for x in 0..20 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 0); // R
            }
        }
    }

    #[test]
    fn test_fill_rect_partial_clip() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let yellow = Color::rgb(255, 255, 0);

        // Fill a rectangle that extends beyond canvas bounds
        fill_rect(&mut canvas, 5, 5, 15, 15, yellow).unwrap();

        let pixels = canvas.pixels();
        let width = canvas.width();

        // Check only pixels within bounds were filled
        for y in 5..=9 {
            for x in 5..=9 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 255); // R
                assert_eq!(pixels[offset + 1], 255); // G
                assert_eq!(pixels[offset + 2], 0); // B
            }
        }

        // Pixels outside the fill area should be black
        for y in 0..4 {
            for x in 0..10 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 0); // R
            }
        }
    }

    #[test]
    fn test_fill_rect_edge_cases() {
        let mut canvas = Canvas::new_rgba(10, 10);
        let magenta = Color::rgb(255, 0, 255);

        // Fill entire canvas
        fill_rect(&mut canvas, 0, 0, 9, 9, magenta).unwrap();

        let pixels = canvas.pixels();
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 255); // R
            assert_eq!(pixels[i + 2], 255); // B
        }
    }

    #[test]
    fn test_fill_rect_entirely_outside() {
        let mut canvas = Canvas::new_rgba(100, 100);
        let red = Color::rgb(255, 0, 0);
        
        // Rectangle entirely to the left of canvas
        fill_rect(&mut canvas, -100, 10, -10, 90, red).unwrap();
        
        // Verify no pixels changed (all should still be black/transparent)
        let pixels = canvas.pixels();
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 0, "Pixel at {} should be 0 (black)", i);
            assert_eq!(pixels[i + 1], 0);
            assert_eq!(pixels[i + 2], 0);
            assert_eq!(pixels[i + 3], 0);
        }
        
        // Rectangle entirely to the right of canvas
        fill_rect(&mut canvas, 110, 10, 200, 90, red).unwrap();
        
        // Still no changes
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 0);
        }
        
        // Rectangle entirely above canvas
        fill_rect(&mut canvas, 10, -100, 90, -10, red).unwrap();
        
        // Still no changes
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 0);
        }
        
        // Rectangle entirely below canvas
        fill_rect(&mut canvas, 10, 110, 90, 200, red).unwrap();
        
        // Still no changes
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 0);
        }
    }

    #[test]
    fn test_copy_canvas_same_size() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Fill source with red
        fill_rect(&mut src, 0, 0, 9, 9, Color::rgb(255, 0, 0)).unwrap();
        
        // Copy entire source to destination
        copy_canvas(&mut dst, &src, 0, 0, 0, 0, -1, -1).unwrap();
        
        let dst_pixels = dst.pixels();
        
        // Verify all pixels were copied
        for i in (0..dst_pixels.len()).step_by(4) {
            assert_eq!(dst_pixels[i], 255); // R
            assert_eq!(dst_pixels[i + 1], 0); // G
            assert_eq!(dst_pixels[i + 2], 0); // B
            assert_eq!(dst_pixels[i + 3], 255); // A
        }
    }

    #[test]
    fn test_copy_canvas_to_offset() {
        let mut dst = Canvas::new_rgba(20, 20);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Fill source with green
        fill_rect(&mut src, 0, 0, 9, 9, Color::rgb(0, 255, 0)).unwrap();
        
        // Copy source to specific position in destination
        copy_canvas(&mut dst, &src, 5, 5, 0, 0, -1, -1).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify pixels in the copied area are green
        for y in 5..15 {
            for x in 5..15 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 0); // R
                assert_eq!(dst_pixels[offset + 1], 255); // G
                assert_eq!(dst_pixels[offset + 2], 0); // B
            }
        }
        
        // Verify pixels outside the copied area are black
        // Check top-left area
        let offset = (0 * dst_width + 0) as usize * 4;
        assert_eq!(dst_pixels[offset], 0); // R
    }

    #[test]
    fn test_copy_canvas_clip_source() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src = Canvas::new_rgba(20, 20);
        
        // Fill entire source with blue
        fill_rect(&mut src, 0, 0, 19, 19, Color::rgb(0, 0, 255)).unwrap();
        
        // Copy only partial rect from source (10x10 from position (5,5))
        copy_canvas(&mut dst, &src, 0, 0, 5, 5, 10, 10).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify entire destination is blue (we copied exactly 10x10)
        for y in 0..10 {
            for x in 0..10 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 0); // R
                assert_eq!(dst_pixels[offset + 1], 0); // G
                assert_eq!(dst_pixels[offset + 2], 255); // B
            }
        }
    }

    #[test]
    fn test_copy_canvas_clip_destination() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src = Canvas::new_rgba(20, 20);
        
        // Fill source with yellow
        fill_rect(&mut src, 0, 0, 19, 19, Color::rgb(255, 255, 0)).unwrap();
        
        // Copy larger source (20x20) to smaller destination starting at negative offset
        copy_canvas(&mut dst, &src, -5, -5, 0, 0, 20, 20).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify only pixels within destination bounds were copied
        // The copy starts at (-5, -5), so we expect pixels (0,0) to (14,14) from source
        // which would map to positions (5,5) to (9,9) in destination
        for y in 0..10 {
            for x in 0..10 {
                let offset = (y * dst_width + x) as usize * 4;
                if x >= 5 && y >= 5 {
                    // Pixels that should have been copied
                    assert_eq!(dst_pixels[offset], 255); // R
                    assert_eq!(dst_pixels[offset + 1], 255); // G
                    assert_eq!(dst_pixels[offset + 2], 0); // B
                } else {
                    // Pixels that remain black
                    assert_eq!(dst_pixels[offset], 0); // R
                }
            }
        }
    }

    #[test]
    fn test_copy_canvas_partial_overlap() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Fill destination with black (default)
        // Fill source with magenta
        fill_rect(&mut src, 0, 0, 9, 9, Color::rgb(255, 0, 255)).unwrap();
        
        // Copy partial rect from source to partial position in destination
        copy_canvas(&mut dst, &src, 2, 2, 0, 0, 6, 6).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify copied area is magenta
        for y in 2..8 {
            for x in 2..8 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 255); // R
                assert_eq!(dst_pixels[offset + 1], 0); // G
                assert_eq!(dst_pixels[offset + 2], 255); // B
            }
        }
        
        // Verify uncopied area is black
        let offset = (0 * dst_width + 0) as usize * 4;
        assert_eq!(dst_pixels[offset], 0); // R
    }

    #[test]
    fn test_copy_canvas_entirely_outside() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Fill source with cyan
        fill_rect(&mut src, 0, 0, 9, 9, Color::rgb(0, 255, 255)).unwrap();
        
        // Try to copy entirely outside destination bounds
        copy_canvas(&mut dst, &src, 20, 20, 0, 0, 10, 10).unwrap();
        
        let dst_pixels = dst.pixels();
        
        // Verify no pixels were modified
        for i in (0..dst_pixels.len()).step_by(4) {
            assert_eq!(dst_pixels[i], 0); // R
            assert_eq!(dst_pixels[i + 1], 0); // G
            assert_eq!(dst_pixels[i + 2], 0); // B
            assert_eq!(dst_pixels[i + 3], 0); // A
        }
    }

    #[test]
    fn test_copy_canvas_default_params_entire_source() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Fill source with white
        fill_rect(&mut src, 0, 0, 9, 9, Color::rgb(255, 255, 255)).unwrap();
        
        // Copy with default parameters (width=0, height=0 means copy entire source)
        copy_canvas(&mut dst, &src, 0, 0, 0, 0, 0, 0).unwrap();
        
        let dst_pixels = dst.pixels();
        
        // Verify entire source was copied
        for i in (0..dst_pixels.len()).step_by(4) {
            assert_eq!(dst_pixels[i], 255); // R
            assert_eq!(dst_pixels[i + 1], 255); // G
            assert_eq!(dst_pixels[i + 2], 255); // B
        }
    }

    #[test]
    fn test_copy_canvas_format_mismatch() {
        let mut dst = Canvas::new(Extent::new(10, 10), CanvasFormat::rgba());
        let src = Canvas::new(Extent::new(10, 10), CanvasFormat::rgb());
        
        // Should fail due to format mismatch
        let result = copy_canvas(&mut dst, &src, 0, 0, 0, 0, -1, -1);
        assert!(matches!(result, Err(CanvasError::InvalidOperation(_))));
    }

    #[test]
    fn test_copy_canvas_large_to_small() {
        let mut dst = Canvas::new_rgba(5, 5);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Fill source with varied colors
        for y in 0..10 {
            for x in 0..10 {
                let color = Color::rgb(x as u8 * 25, y as u8 * 25, 128);
                let mut temp_canvas = Canvas::new_rgba(1, 1);
                fill_rect(&mut temp_canvas, 0, 0, 0, 0, color).unwrap();
                let temp_pixels = temp_canvas.pixels();
                src.with_pixels_mut(|src_pixels| {
                    let offset = (y * 10 + x) as usize * 4;
                    src_pixels[offset..offset + 4].copy_from_slice(&temp_pixels[..4]);
                }).unwrap();
            }
        }
        
        // Copy larger source to smaller destination
        copy_canvas(&mut dst, &src, 0, 0, 0, 0, 10, 10).unwrap();
        
        // Should only copy 5x5 pixels (destination size)
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify top-left 5x5 pixels from source were copied
        for y in 0..5 {
            for x in 0..5 {
                let offset = (y * dst_width + x) as usize * 4;
                // Verify pixel values correspond to source
                assert!(dst_pixels[offset] == x as u8 * 25); // R
                assert!(dst_pixels[offset + 1] == y as u8 * 25); // G
            }
        }
    }


    #[test]
    fn test_scissor_clips_line() {
        let mut canvas = Canvas::new_rgba(20, 20);
        
        // Set a scissor rect from (5,5) to (15,15)
        let scissor_rect = Rect::from_parts(Point::new(5, 5), Extent::new(10, 10));
        canvas.set_scissor(ScissorRect::enabled(scissor_rect));

        
        let blue = Color::rgb(0, 0, 255);
        let mode = DrawMode::Normal;
        
        // Draw diagonal line from (0,0) to (20,20)
        // Only pixels in the scissor should be drawn
        canvas.draw_line(0, 0, 20, 20, blue, mode).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Check diagonal pixels inside scissor are blue
        for i in 5..15 {
            let offset = (i * width + i) as usize * 4;
            assert_eq!(pixels[offset], 0, "R at ({}, {})", i, i);  // R should be 0 for blue
            assert_eq!(pixels[offset + 1], 0, "G at ({}, {})", i, i);  // G should be 0 for blue
            assert_eq!(pixels[offset + 2], 255, "B at ({}, {})", i, i);  // B should be 255 for blue
        }
        
        // Check diagonal pixels outside scissor are black
        for i in 0..5 {
            let offset = (i * width + i) as usize * 4;
            assert_eq!(pixels[offset], 0, "Pixel at ({}, {}) outside scissor should be black", i, i);
        }
        
        for i in 15..20 {
            let offset = (i * width + i) as usize * 4;
            assert_eq!(pixels[offset], 0, "Pixel at ({}, {}) outside scissor should be black", i, i);
        }
    }

    #[test]
    fn test_disable_scissor() {
        let mut canvas = Canvas::new_rgba(20, 20);
        
        // Set a scissor rect
        let scissor_rect = Rect::from_parts(Point::new(5, 5), Extent::new(10, 10));
        canvas.set_scissor(ScissorRect::enabled(scissor_rect));

        
        // Fill a larger rect from (0,0) to (20,20)
        fill_rect(&mut canvas, 0, 0, 20, 20, Color::rgb(255, 0, 0)).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Check pixels inside scissor are red
        for y in 5..15 {
            for x in 5..15 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 255, "Pixel in scissor at ({}, {}) should be red", x, y);
            }
        }
        
        // Check pixels outside scissor are black
        for y in 0..20 {
            for x in 0..20 {
                let offset = (y * width + x) as usize * 4;
                if x < 5 || x >= 15 || y < 5 || y >= 15 {
                    assert_eq!(pixels[offset], 0, "Pixel outside scissor at ({}, {}) should be black", x, y);
                }
            }
        }
    }

    #[test]
    fn test_scissor_partial_fill_rect() {
        let mut canvas = Canvas::new_rgba(20, 20);
        
        // Set a scissor rect from (10,10) to (15,15)
        let scissor_rect = Rect::from_parts(Point::new(10, 10), Extent::new(5, 5));
        canvas.set_scissor(ScissorRect::enabled(scissor_rect));
        
        // Fill a rect from (5,5) to (20,20)
        fill_rect(&mut canvas, 5, 5, 20, 20, Color::rgb(0, 255, 0)).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Check pixels in intersection of fill rect and scissor are green
        for y in 10..15 {
            for x in 10..15 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 0, "R");
                assert_eq!(pixels[offset + 1], 255, "G at ({}, {})", x, y);
                assert_eq!(pixels[offset + 2], 0, "B");
            }
        }
        
        // Check pixels in fill rect but outside scissor are black
        let offset = (5 * width + 5) as usize * 4;
        assert_eq!(pixels[offset], 0, "Pixel at (5,5) should be black");
    }

    #[test]
    fn test_scissor_clips_copy_canvas() {
        let mut dst = Canvas::new_rgba(20, 20);
        let mut src = Canvas::new_rgba(10, 10);
        
        // Set scissor on destination from (5,5) to (15,15)
        let scissor_rect = Rect::from_parts(Point::new(5, 5), Extent::new(10, 10));
        dst.set_scissor(ScissorRect::enabled(scissor_rect));
        
        // Fill source with blue
        fill_rect(&mut src, 0, 0, 10, 10, Color::rgb(0, 0, 255)).unwrap();
        
        // Copy source to destination at (0,0)
        copy_canvas(&mut dst, &src, 0, 0, 0, 0, -1, -1).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Check pixels inside scissor are blue (where copy intersects scissor)
        for y in 5..10 {
            for x in 5..10 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 0, "R at ({}, {})", x, y);
                assert_eq!(dst_pixels[offset + 1], 0, "G at ({}, {})", x, y);
                assert_eq!(dst_pixels[offset + 2], 255, "B at ({}, {})", x, y);
            }
        }
        
        // Check pixels in source region but outside scissor are black
        for y in 0..5 {
            for x in 0..10 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 0, "Pixel at ({}, {}) should be black", x, y);
            }
        }
    }


    #[test]
    fn test_scissor_edge_cases() {
        let mut canvas = Canvas::new_rgba(20, 20);
        
        // Scissor at canvas origin
        let scissor_rect = Rect::from_parts(Point::new(5, 5), Extent::new(5, 5));
        canvas.set_scissor(ScissorRect::enabled(scissor_rect));
        
        assert!(is_in_scissor(&canvas, 0, 0));
        assert!(is_in_scissor(&canvas, 4, 4));
        assert!(!is_in_scissor(&canvas, 5, 5));
        assert!(!is_in_scissor(&canvas, 0, 5));
        assert!(!is_in_scissor(&canvas, 5, 0));
        
        // Scissor at canvas edge
        let scissor_rect = Rect::from_parts(Point::new(15, 15), Extent::new(5, 5));
        canvas.set_scissor(ScissorRect::enabled(scissor_rect));
        
        assert!(is_in_scissor(&canvas, 15, 15));
        assert!(is_in_scissor(&canvas, 19, 19));
        assert!(!is_in_scissor(&canvas, 14, 15));
        assert!(!is_in_scissor(&canvas, 15, 14));
    }

    #[test]
    fn test_draw_fontchar_missing_char() {
        let mut canvas = Canvas::new_rgba(100, 100);
        let page = FontPage::new(0x0000, 0x0020, 10);
        let red = Color::rgb(255, 0, 0);
        
        // Try to draw character that doesn't exist
        let result = draw_fontchar(&mut canvas, red, &page, 0x0020, 10, 10, false);
        assert!(matches!(result, Err(CanvasError::InvalidOperation(_))));
    }

    #[test]
    fn test_draw_fontchar_empty_char() {
        let mut canvas = Canvas::new_rgba(100, 100);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let red = Color::rgb(255, 0, 0);
        
        // Add a character with no data
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(8, 12),
            disp: crate::graphics::font::Extent::new(8, 12),
            hotspot: crate::graphics::font::Point::new(0, 10),
            data: None,
            pitch: 8,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Try to draw character with no bitmap data
        let result = draw_fontchar(&mut canvas, red, &page, 0x0020, 10, 10, false);
        assert!(matches!(result, Err(CanvasError::InvalidOperation(_))));
    }

    #[test]
    fn test_draw_fontchar_zero_dimensions() {
        let mut canvas = Canvas::new_rgba(100, 100);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let red = Color::rgb(255, 0, 0);
        
        // Add a character with zero dimensions
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(0, 0),
            disp: crate::graphics::font::Extent::new(0, 0),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(std::sync::Arc::new([])),
            pitch: 0,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Should return 0 width without error
        let result = draw_fontchar(&mut canvas, red, &page, 0x0020, 10, 10, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_draw_fontchar_simple_opaque() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(100, 100);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let red = Color::rgb(255, 0, 0);
        
        // Create a simple 3x3 character with full opacity
        let data: Vec<u8> = vec![255; 9]; // All pixels fully opaque
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(3, 3),
            disp: crate::graphics::font::Extent::new(3, 3),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(Arc::from(data)),
            pitch: 3,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Draw character at (10, 10)
        let result = draw_fontchar(&mut canvas, red, &page, 0x0020, 10, 10, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
        
        // Check that pixels (10,10) through (12,12) are red
        let pixels = canvas.pixels();
        let width = canvas.width();
        for y in 10..13 {
            for x in 10..13 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 255, "R at ({},{})", x, y);
                assert_eq!(pixels[offset + 1], 0, "G at ({},{})", x, y);
                assert_eq!(pixels[offset + 2], 0, "B at ({},{})", x, y);
            }
        }
    }

    #[test]
    fn test_draw_fontchar_with_hotspot() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(100, 100);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let green = Color::rgb(0, 255, 0);
        
        // Create a 2x2 character with hotspot offset
        let data: Vec<u8> = vec![255, 255, 255, 255]; // All pixels fully opaque
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(2, 2),
            disp: crate::graphics::font::Extent::new(2, 2),
            hotspot: crate::graphics::font::Point::new(1, 1),
            data: Some(Arc::from(data)),
            pitch: 2,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Draw character at (20, 20) - should appear at (19, 19) due to hotspot
        let result = draw_fontchar(&mut canvas, green, &page, 0x0020, 20, 20, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        
        // Check that pixels (19,19) through (20,20) are green
        let pixels = canvas.pixels();
        let width = canvas.width();
        for y in 19..21 {
            for x in 19..21 {
                let offset = (y * width + x) as usize * 4;
                assert_eq!(pixels[offset], 0, "R at ({},{})", x, y);
                // Allow some tolerance from blending
                assert!(pixels[offset + 1] > 200, "G at ({},{})", x, y);
                assert_eq!(pixels[offset + 2], 0, "B at ({},{})", x, y);
            }
        }
    }

    #[test]
    fn test_draw_fontchar_alpha_blending() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(100, 100);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let blue = Color::rgb(0, 0, 255);
        
        // Fill background with white first
        fill_rect(&mut canvas, 0, 0, 99, 99, Color::rgb(255, 255, 255)).unwrap();
        
        // Create a 2x2 character with varying alpha values
        // Top-left: 0 (transparent), Top-right: 85, Bottom-left: 170, Bottom-right: 255
        let data: Vec<u8> = vec![0, 85, 170, 255];
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(2, 2),
            disp: crate::graphics::font::Extent::new(2, 2),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(Arc::from(data)),
            pitch: 2,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Draw character over white background
        draw_fontchar(&mut canvas, blue, &page, 0x0020, 50, 50, false).unwrap();
        
        // Check alpha blending results
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Top-left (0 alpha) should still be white
        let offset = (50 * width + 50) as usize * 4;
        assert_eq!(pixels[offset], 255, "R at (50,50) should be white");
        assert_eq!(pixels[offset + 1], 255, "G at (50,50) should be white");
        assert_eq!(pixels[offset + 2], 255, "B at (50,50) should be white");
        
        // Top-right (85/255 alpha) should be light blue (blended)
        let offset = (50 * width + 51) as usize * 4;
        // Should have more red/green than pure blue due to white background
        assert!(pixels[offset] > 100, "R at (50,51)");
        assert!(pixels[offset + 1] > 100, "G at (50,51)");
        assert!(pixels[offset + 2] > 150, "B at (50,51)");
        
        // Bottom-left (170/255 alpha) should be more blue
        let offset = (51 * width + 50) as usize * 4;
        assert!(pixels[offset] < 100, "R at (51,50)");
        assert!(pixels[offset + 1] < 100, "G at (51,50)");
        assert!(pixels[offset + 2] > 200, "B at (51,50)");
        
        // Bottom-right (255 alpha) should be pure blue
        let offset = (51 * width + 51) as usize * 4;
        assert_eq!(pixels[offset], 0, "R at (51,51)");
        assert_eq!(pixels[offset + 1], 0, "G at (51,51)");
        assert_eq!(pixels[offset + 2], 255, "B at (51,51)");
    }

    #[test]
    fn test_draw_fontchar_scissor_clip() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(50, 50);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let magenta = Color::rgb(255, 0, 255);
        
        // Create a 10x10 fully opaque character
        let data: Vec<u8> = vec![255; 100];
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(10, 10),
            disp: crate::graphics::font::Extent::new(10, 10),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(Arc::from(data)),
            pitch: 10,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Set scissor rect from (10,10) to (20,20)
        let scissor_rect = Rect::from_parts(Point::new(10, 10), Extent::new(10, 10));
        canvas.set_scissor(ScissorRect::enabled(scissor_rect));
        
        // Draw character at (15, 15) - partially inside scissor
        draw_fontchar(&mut canvas, magenta, &page, 0x0020, 15, 15, false).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Check pixels inside scissor are magenta (where character overlaps)
        for y in 15..20 {
            for x in 15..20 {
                let offset = (y * width + x) as usize * 4;
                if x < 25 && y < 25 {  // Within character bounds
                    assert!(pixels[offset] > 200, "R at ({},{})", x, y);
                    assert!(pixels[offset + 2] > 200, "B at ({},{})", x, y);
                }
            }
        }
        
        // Check pixels outside scissor are still black
        let offset = (5 * width + 5) as usize * 4;
        assert_eq!(pixels[offset], 0, "Pixel outside scissor should be black");
    }

    #[test]
    fn test_draw_fontchar_canvas_bounds_clip() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(20, 20);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        let cyan = Color::rgb(0, 255, 255);
        
        // Create a 20x20 fully opaque character
        let data: Vec<u8> = vec![255; 400];
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(20, 20),
            disp: crate::graphics::font::Extent::new(20, 20),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(Arc::from(data)),
            pitch: 20,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Draw character at (-5, -5) - partially outside canvas
        draw_fontchar(&mut canvas, cyan, &page, 0x0020, -5, -5, false).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Check that pixels (0,0) through (14,14) are drawn (clipped portion)
        for y in 0..15 {
            for x in 0..15 {
                let offset = (y * width + x) as usize * 4;
                assert!(pixels[offset + 1] > 200, "G at ({},{})", x, y);
                assert!(pixels[offset + 2] > 200, "B at ({},{})", x, y);
            }
        }
        
        // Should not panic or access out of bounds
        assert_eq!(canvas.pixels().len(), 20 * 20 * 4);
    }

    #[test]
    fn test_draw_fontchar_semitransparent_fg() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(50, 50);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        
        // Start with white background
        fill_rect(&mut canvas, 0, 0, 49, 49, Color::rgb(255, 255, 255)).unwrap();
        
        // Create a 10x10 fully opaque character (glyph_alpha = 255)
        let data: Vec<u8> = vec![255; 100];
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(10, 10),
            disp: crate::graphics::font::Extent::new(10, 10),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(Arc::from(data)),
            pitch: 10,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Draw with semi-transparent foreground color (alpha = 128)
        // Effective alpha = (255 * 128) / 255 = 128
        let fg_color = Color { r: 255, g: 0, b: 0, a: 128 };
        draw_fontchar(&mut canvas, fg_color, &page, 0x0020, 10, 10, false).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Check that pixels in the character have been blended
        // With alpha=128, we expect red to be blended with white background
        // Result should be approximately: R = 255*0.5 + 255*0.5 = 255 (clamped)
        // This tests that effective alpha combining works correctly
        let offset = (12 * width + 12) as usize * 4;
        assert!(pixels[offset] > 200, "R should be high (blended with white)");
        assert!(pixels[offset + 1] < 200, "G should be reduced (blended with red)");
        assert!(pixels[offset + 2] < 200, "B should be reduced (blended with red)");
        
        // Check alpha channel - should use proper alpha blending
        // Effective alpha = 128, background alpha = 255
        // For alpha channel: 128 + 255*(255-128)/255 = 128 + 255*127/255 = 128 + 127 = 255
        assert_eq!(pixels[offset + 3], 255, "Alpha should be properly blended");
    }

    #[test]
    fn test_draw_fontchar_combined_alpha() {
        use std::sync::Arc;
        
        let mut canvas = Canvas::new_rgba(50, 50);
        let mut page = FontPage::new(0x0000, 0x0020, 10);
        
        // Create a 10x10 character with varying glyph alpha
        let mut data: Vec<u8> = Vec::with_capacity(100);
        for y in 0..10 {
            for x in 0..10 {
                // Create a gradient: left pixels have alpha 128, right pixels have alpha 255
                let alpha = if x < 5 { 128 } else { 255 };
                data.push(alpha);
            }
        }
        
        let tf_char = crate::graphics::font::TFChar {
            extent: crate::graphics::font::Extent::new(10, 10),
            disp: crate::graphics::font::Extent::new(10, 10),
            hotspot: crate::graphics::font::Point::new(0, 0),
            data: Some(Arc::from(data)),
            pitch: 10,
        };
        page.set_char(0x0020, tf_char).unwrap();
        
        // Draw with semi-transparent foreground color (alpha = 128)
        let fg_color = Color { r: 255, g: 0, b: 0, a: 128 };
        draw_fontchar(&mut canvas, fg_color, &page, 0x0020, 10, 10, false).unwrap();
        
        let pixels = canvas.pixels();
        let width = canvas.width();
        
        // Left side: glyph_alpha=128, fg_color.a=128
        // Effective alpha = (128 * 128) / 255 = 64
        let offset_left = (12 * width + 12) as usize * 4;
        let r_left = pixels[offset_left] as i32;
        
        // Right side: glyph_alpha=255, fg_color.a=128
        // Effective alpha = (255 * 128) / 255 = 128
        let offset_right = (12 * width + 17) as usize * 4;
        let r_right = pixels[offset_right] as i32;
        
        // Right side should have more red (higher effective alpha)
        assert!(r_right > r_left, 
                "Right side (alpha=128) should have more red than left side (alpha=64)");
    }

}

    #[test]
    fn test_draw_image_basic() {
        let mut dst = Canvas::new_rgba(20, 20);
        let mut src_canvas = Canvas::new_rgba(10, 10);
        
        // Fill source image with red
        fill_rect(&mut src_canvas, 0, 0, 9, 9, Color::rgb(255, 0, 0)).unwrap();
        
        // Create TFImage from source canvas
        let image = TFImage::new(src_canvas);
        
        // Draw image at position (5, 5)
        draw_image(&mut dst, &image, 5, 5, 0).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify image was drawn at correct position (0,0 to 9,9 in source maps to 5,5 to 14,14 in dst)
        for y in 5..15 {
            for x in 5..15 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 255); // R
                assert_eq!(dst_pixels[offset + 1], 0); // G
                assert_eq!(dst_pixels[offset + 2], 0); // B
                assert_eq!(dst_pixels[offset + 3], 255); // A
            }
        }
    }

    #[test]
    fn test_draw_image_with_hotspot() {
        let mut dst = Canvas::new_rgba(20, 20);
        let mut src_canvas = Canvas::new_rgba(10, 10);
        
        // Fill source image with green
        fill_rect(&mut src_canvas, 0, 0, 9, 9, Color::rgb(0, 255, 0)).unwrap();
        
        // Create TFImage with hot spot offset
        let image = TFImage::new(src_canvas);
        image.set_normal_hot_spot(HotSpot::new(2, 3));
        
        // Draw image at position (10, 10)
        // With hot spot (2, 3), image should be drawn at (8, 7)
        draw_image(&mut dst, &image, 10, 10, 0).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Verify image was drawn at (8, 7) after hot spot offset
        for y in 7..17 {
            for x in 8..18 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 0); // R
                assert_eq!(dst_pixels[offset + 1], 255); // G
                assert_eq!(dst_pixels[offset + 2], 0); // B
        }
    }

    #[test]
    fn test_draw_image_partial_clip() {
        let mut dst = Canvas::new_rgba(10, 10);
        let mut src_canvas = Canvas::new_rgba(10, 10);
        
        // Fill source image with blue
        fill_rect(&mut src_canvas, 0, 0, 9, 9, Color::rgb(0, 0, 255)).unwrap();
        
        let image = TFImage::new(src_canvas);
        
        // Draw image partially off the canvas (starts at -5, -5)
        draw_image(&mut dst, &image, -5, -5, 0).unwrap();
        
        let dst_pixels = dst.pixels();
        let dst_width = dst.width();
        
        // Only pixels within canvas bounds should be drawn
        // Source (5,5) to (9,9) should map to (0,0) to (4,4) in destination
        for y in 0..5 {
            for x in 0..5 {
                let offset = (y * dst_width + x) as usize * 4;
                assert_eq!(dst_pixels[offset], 0); // R
                assert_eq!(dst_pixels[offset + 1], 0); // G
                assert_eq!(dst_pixels[offset + 2], 255); // B
            }
        }
        
        // Pixels outside the copied area should remain black
        let offset = (7 * dst_width + 7) as usize * 4;
        assert_eq!(dst_pixels[offset], 0); // R
    }
}
