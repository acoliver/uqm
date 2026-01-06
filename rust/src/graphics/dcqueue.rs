//! Draw Command Queue (DCQ) implementation.

use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex, RwLock,
};

use log::{debug, warn};

use crate::graphics::gfx_common::ScaleMode;
use crate::graphics::render_context::{RenderContext, ScreenType};
use crate::graphics::tfb_draw::{Canvas, CanvasPrimitive, HotSpot, ScissorRect, TFImage};

/// Render destination screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Screen {
    Main = 0,
    Extra = 1,
    Transition = 2,
}

impl From<Screen> for ScreenType {
    fn from(screen: Screen) -> Self {
        match screen {
            Screen::Main => ScreenType::Main,
            Screen::Extra => ScreenType::Extra,
            Screen::Transition => ScreenType::Transition,
        }
    }
}

impl From<ScreenType> for Screen {
    fn from(screen: ScreenType) -> Self {
        match screen {
            ScreenType::Main => Screen::Main,
            ScreenType::Extra => Screen::Extra,
            ScreenType::Transition => Screen::Transition,
        }
    }
}

/// Draw mode for primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawMode {
    Normal,
    Blended,
}

/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 2D extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Extent {
    pub width: i32,
    pub height: i32,
}

impl Extent {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

/// Rectangle with corner and extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub corner: Point,
    pub extent: Extent,
}

impl Rect {
    pub const fn new(corner: Point, extent: Extent) -> Self {
        Self { corner, extent }
    }

    pub const fn from_parts(corner: Point, extent: Extent) -> Self {
        Self { corner, extent }
    }
}

/// Type-safe image handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImageRef(pub u32);

impl ImageRef {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn id(self) -> u32 {
        self.0
    }
}

impl From<u32> for ImageRef {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<ImageRef> for u32 {
    fn from(id: ImageRef) -> Self {
        id.0
    }
}

/// Type-safe color map handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorMapRef(pub u32);

impl ColorMapRef {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn id(self) -> u32 {
        self.0
    }
}

impl From<u32> for ColorMapRef {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<ColorMapRef> for u32 {
    fn from(id: ColorMapRef) -> Self {
        id.0
    }
}

/// Type-safe font character handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontCharRef {
    pub page_id: u32,
    pub char_code: u32,
}

impl FontCharRef {
    pub const fn new(page_id: u32, char_code: u32) -> Self {
        Self { page_id, char_code }
    }
}

/// Callback function for DCQ commands.
pub type CallbackFn = fn(u64);

/// DCQ command variants.
#[derive(Clone)]
pub enum DrawCommand {
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color,
        draw_mode: DrawMode,
        dest: Screen,
    },
    Rect {
        rect: Rect,
        color: Color,
        draw_mode: DrawMode,
        dest: Screen,
    },
    Image {
        image: ImageRef,
        x: i32,
        y: i32,
        dest: Screen,
        colormap: Option<ColorMapRef>,
        draw_mode: DrawMode,
        scale: i32,
        scale_mode: ScaleMode,
    },
    FilledImage {
        image: ImageRef,
        x: i32,
        y: i32,
        color: Color,
        dest: Screen,
        draw_mode: DrawMode,
        scale: i32,
        scale_mode: ScaleMode,
    },
    FontChar {
        fontchar: FontCharRef,
        backing: Option<ImageRef>,
        x: i32,
        y: i32,
        draw_mode: DrawMode,
        dest: Screen,
    },
    Copy {
        rect: Rect,
        src: Screen,
        dest: Screen,
    },
    CopyToImage {
        image: ImageRef,
        rect: Rect,
        src: Screen,
    },
    ScissorEnable {
        rect: Rect,
    },
    ScissorDisable,
    SetMipmap {
        image: ImageRef,
        mipmap: ImageRef,
        hotx: i32,
        hoty: i32,
    },
    DeleteImage {
        image: ImageRef,
    },
    DeleteData {
        data: u64,
    },
    SendSignal {
        signal: Arc<AtomicBool>,
    },
    ReinitVideo {
        driver: i32,
        flags: i32,
        width: i32,
        height: i32,
    },
    Callback {
        callback: CallbackFn,
        arg: u64,
    },
}

impl fmt::Debug for DrawCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line { .. } => f.write_str("DrawCommand::Line"),
            Self::Rect { .. } => f.write_str("DrawCommand::Rect"),
            Self::Image { .. } => f.write_str("DrawCommand::Image"),
            Self::FilledImage { .. } => f.write_str("DrawCommand::FilledImage"),
            Self::FontChar { .. } => f.write_str("DrawCommand::FontChar"),
            Self::Copy { .. } => f.write_str("DrawCommand::Copy"),
            Self::CopyToImage { .. } => f.write_str("DrawCommand::CopyToImage"),
            Self::ScissorEnable { .. } => f.write_str("DrawCommand::ScissorEnable"),
            Self::ScissorDisable => f.write_str("DrawCommand::ScissorDisable"),
            Self::SetMipmap { .. } => f.write_str("DrawCommand::SetMipmap"),
            Self::DeleteImage { .. } => f.write_str("DrawCommand::DeleteImage"),
            Self::DeleteData { .. } => f.write_str("DrawCommand::DeleteData"),
            Self::SendSignal { .. } => f.write_str("DrawCommand::SendSignal"),
            Self::ReinitVideo { .. } => f.write_str("DrawCommand::ReinitVideo"),
            Self::Callback { .. } => f.write_str("DrawCommand::Callback"),
        }
    }
}

/// DCQ configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DcqConfig {
    pub max_size: usize,
    pub force_slowdown_size: usize,
    pub force_break_size: usize,
    pub livelock_max: usize,
}

impl DcqConfig {
    pub const fn standard() -> Self {
        Self {
            max_size: 16_384,
            force_slowdown_size: 4_096,
            force_break_size: 16_384,
            livelock_max: 4_096,
        }
    }

    pub const fn debug() -> Self {
        Self {
            max_size: 512,
            force_slowdown_size: 128,
            force_break_size: 512,
            livelock_max: 256,
        }
    }

    pub fn validate(&self) -> Result<(), DcqError> {
        if self.max_size == 0 {
            return Err(DcqError::InvalidConfig);
        }
        if self.force_slowdown_size > self.max_size
            || self.force_break_size > self.max_size
            || self.livelock_max > self.max_size
        {
            return Err(DcqError::InvalidConfig);
        }
        Ok(())
    }
}

impl Default for DcqConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// DCQ statistics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DcqStats {
    pub size: usize,
    pub full_size: usize,
    pub max_size: usize,
    pub batching_depth: usize,
}

impl DcqStats {
    pub fn utilization(self) -> f32 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.full_size as f32 / self.max_size as f32) * 100.0
        }
    }
}

/// DCQ errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DcqError {
    QueueFull,
    InvalidConfig,
    WouldBlock,
}

impl fmt::Display for DcqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => write!(f, "DCQ is full"),
            Self::InvalidConfig => write!(f, "Invalid DCQ configuration"),
            Self::WouldBlock => write!(f, "Operation would block"),
        }
    }
}

impl std::error::Error for DcqError {}

struct Inner {
    buffer: Vec<Option<DrawCommand>>,
    front: usize,
    back: usize,
    insertion_point: usize,
    batching: usize,
    size: usize,
    full_size: usize,
}

impl Inner {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            front: 0,
            back: 0,
            insertion_point: 0,
            batching: 0,
            size: 0,
            full_size: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.buffer.len()
    }

    fn synchronize(&mut self) {
        if self.batching == 0 {
            self.back = self.insertion_point;
            self.size = self.full_size;
        }
    }

    fn push(&mut self, cmd: DrawCommand) {
        let capacity = self.capacity();
        self.buffer[self.insertion_point] = Some(cmd);
        self.insertion_point = (self.insertion_point + 1) % capacity;
        self.full_size += 1;
        if self.batching == 0 {
            self.back = self.insertion_point;
            self.size = self.size.saturating_add(1);
            self.full_size = self.size;
        }
    }

    fn pop(&mut self) -> Option<DrawCommand> {
        if self.size == 0 {
            return None;
        }
        let capacity = self.capacity();
        if self.front == self.back && self.size != capacity {
            self.size = 0;
            self.full_size = 0;
            self.front = 0;
            self.back = 0;
            self.insertion_point = 0;
            return None;
        }
        let cmd = self.buffer[self.front].take();
        self.front = (self.front + 1) % capacity;
        self.size = self.size.saturating_sub(1);
        if self.batching == 0 {
            self.full_size = self.full_size.saturating_sub(1);
        }
        cmd
    }

    fn clear(&mut self) {
        for slot in &mut self.buffer {
            *slot = None;
        }
        self.front = 0;
        self.back = 0;
        self.insertion_point = 0;
        self.batching = 0;
        self.size = 0;
        self.full_size = 0;
    }
}

fn ring_distance(front: usize, back: usize, capacity: usize) -> usize {
    if front <= back {
        back - front
    } else {
        back + capacity - front
    }
}

/// RAII guard for batching.
pub struct BatchGuard {
    queue: DrawCommandQueue,
    active: bool,
}

impl BatchGuard {
    fn new(queue: DrawCommandQueue) -> Self {
        Self { queue, active: true }
    }

    pub fn cancel(mut self) {
        self.active = false;
    }
}

impl Drop for BatchGuard {
    fn drop(&mut self) {
        if self.active {
            self.queue.unbatch();
        }
    }
}

/// Draw command queue.
#[derive(Clone)]
pub struct DrawCommandQueue {
    inner: Arc<Mutex<Inner>>,
    condvar: Arc<Condvar>,
    config: DcqConfig,
    render_context: Arc<RwLock<RenderContext>>,
}

impl DrawCommandQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(DcqConfig::standard(), Arc::new(RwLock::new(RenderContext::new())))
    }

    #[must_use]
    pub fn with_config(config: DcqConfig, render_context: Arc<RwLock<RenderContext>>) -> Self {
        config.validate().expect("invalid DCQ config");
        Self {
            inner: Arc::new(Mutex::new(Inner::new(config.max_size))),
            condvar: Arc::new(Condvar::new()),
            config,
            render_context,
        }
    }

    pub fn push(&self, cmd: DrawCommand) -> Result<(), DcqError> {
        let mut inner = self.inner.lock().unwrap();
        while inner.full_size >= self.config.max_size {
            inner = self.condvar.wait(inner).unwrap();
        }
        inner.push(cmd);
        Ok(())
    }

    pub fn try_push(&self, cmd: DrawCommand) -> Result<(), DcqError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.full_size >= self.config.max_size {
            return Err(DcqError::WouldBlock);
        }
        inner.push(cmd);
        Ok(())
    }

    pub fn pop(&self) -> Option<DrawCommand> {
        let mut inner = self.inner.lock().unwrap();
        let cmd = inner.pop();
        if cmd.is_some() {
            self.condvar.notify_all();
        }
        cmd
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.clear();
        self.condvar.notify_all();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().size
    }

    #[must_use]
    pub fn full_size(&self) -> usize {
        self.inner.lock().unwrap().full_size
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.full_size() >= self.config.max_size
    }

    #[must_use]
    pub fn stats(&self) -> DcqStats {
        let inner = self.inner.lock().unwrap();
        DcqStats {
            size: inner.size,
            full_size: inner.full_size,
            max_size: self.config.max_size,
            batching_depth: inner.batching,
        }
    }

    pub fn batch(&self) -> BatchGuard {
        let mut inner = self.inner.lock().unwrap();
        inner.batching += 1;
        BatchGuard::new(self.clone())
    }

    pub fn unbatch(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.batching > 0 {
            inner.batching -= 1;
        }
        inner.synchronize();
    }

    pub fn batch_reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.batching = 0;
        inner.synchronize();
    }

    pub fn lock_wait_space(&self, slots: usize) -> Result<(), DcqError> {
        if slots > self.config.max_size {
            return Err(DcqError::InvalidConfig);
        }
        let mut inner = self.inner.lock().unwrap();
        while inner.full_size + slots > self.config.max_size {
            inner = self.condvar.wait(inner).unwrap();
        }
        Ok(())
    }

    pub fn unlock(&self) {}

    pub fn process_commands(&self) -> Result<(), DcqError> {
        if self.full_size() > self.config.force_break_size {
            self.batch_reset();
        }

        let mut processed = 0usize;
        loop {
            let cmd = self.pop();
            let Some(cmd) = cmd else { break };
            self.handle_command(cmd);
            processed += 1;
            if processed > self.config.livelock_max {
                debug!("DCQ livelock deterrence triggered");
            }
        }
        Ok(())
    }

    fn handle_command(&self, cmd: DrawCommand) {
        match cmd {
            DrawCommand::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                draw_mode,
                dest,
            } => {
                if let Some(canvas) = self.get_screen_canvas(dest) {
                    let mut canvas = canvas.write().unwrap();
                    if let Err(err) = crate::graphics::tfb_draw::draw_line(
                        &mut canvas,
                        x1,
                        y1,
                        x2,
                        y2,
                        color,
                        draw_mode,
                    ) {
                        warn!("DCQ line draw failed: {}", err);
                    }
                }
            }
            DrawCommand::Rect {
                rect,
                color,
                draw_mode,
                dest,
            } => {
                if let Some(canvas) = self.get_screen_canvas(dest) {
                    let mut canvas = canvas.write().unwrap();
                    let x1 = rect.corner.x;
                    let y1 = rect.corner.y;
                    let x2 = rect.corner.x + rect.extent.width - 1;
                    let y2 = rect.corner.y + rect.extent.height - 1;
                    if let Err(err) = crate::graphics::tfb_draw::draw_rect(
                        &mut canvas,
                        x1,
                        y1,
                        x2,
                        y2,
                        color,
                        draw_mode,
                    ) {
                        warn!("DCQ rect draw failed: {}", err);
                    }
                }
            }
            DrawCommand::Image {
                image,
                x,
                y,
                dest,
                colormap: _,
                draw_mode: _,
                scale: _,
                scale_mode: _,
            } => {
                if let (Some(canvas), Some(image)) =
                    (self.get_screen_canvas(dest), self.get_image(image))
                {
                    let mut canvas = canvas.write().unwrap();
                    if let Err(err) = crate::graphics::tfb_draw::draw_image(&mut canvas, &image, x, y, 0)
                    {
                        warn!("DCQ image draw failed: {}", err);
                    }
                }
            }
            DrawCommand::FilledImage {
                image,
                x,
                y,
                color: _,
                dest,
                draw_mode: _,
                scale: _,
                scale_mode: _,
            } => {
                if let (Some(canvas), Some(image)) =
                    (self.get_screen_canvas(dest), self.get_image(image))
                {
                    let mut canvas = canvas.write().unwrap();
                    if let Err(err) = crate::graphics::tfb_draw::draw_image(&mut canvas, &image, x, y, 0)
                    {
                        warn!("DCQ filled image draw failed: {}", err);
                    }
                }
            }
            DrawCommand::FontChar {
                fontchar,
                backing,
                x,
                y,
                draw_mode,
                dest,
            } => {
                if let Some(canvas) = self.get_screen_canvas(dest) {
                    let mut canvas = canvas.write().unwrap();
                    let backing = backing.and_then(|id| self.get_image(id));
                    let backing_ref = backing.as_deref();
                    if let Err(err) = canvas.draw_fontchar(fontchar, backing_ref, x, y, draw_mode) {
                        warn!("DCQ font char draw failed: {}", err);
                    }
                }
            }
            DrawCommand::Copy { rect, src, dest } => {
                if let (Some(src_canvas), Some(dest_canvas)) =
                    (self.get_screen_canvas(src), self.get_screen_canvas(dest))
                {
                    let src_canvas = src_canvas.read().unwrap();
                    let mut dest_canvas = dest_canvas.write().unwrap();
                    let dst_pt = rect.corner;
                    if let Err(err) = crate::graphics::tfb_draw::copy_canvas(
                        &mut dest_canvas,
                        &src_canvas,
                        dst_pt.x,
                        dst_pt.y,
                        rect.corner.x,
                        rect.corner.y,
                        rect.extent.width,
                        rect.extent.height,
                    ) {
                        warn!("DCQ copy failed: {}", err);
                    }
                }
            }
            DrawCommand::CopyToImage { image, rect, src } => {
                if let (Some(src_canvas), Some(image)) =
                    (self.get_screen_canvas(src), self.get_image(image))
                {
                    let src_canvas = src_canvas.read().unwrap();
                    if let Some(mut image_canvas) = image.normal() {
                        if let Err(err) = crate::graphics::tfb_draw::copy_canvas(
                            &mut image_canvas,
                            &src_canvas,
                            0,
                            0,
                            rect.corner.x,
                            rect.corner.y,
                            rect.extent.width,
                            rect.extent.height,
                        ) {
                            warn!("DCQ copy to image failed: {}", err);
                        }
                        image.mark_dirty();
                    }
                }
            }
            DrawCommand::ScissorEnable { rect } => {
                if let Some(canvas) = self.get_screen_canvas(Screen::Main) {
                    let canvas = canvas.read().unwrap();
                    canvas.set_scissor(ScissorRect::enabled(rect));
                }
            }
            DrawCommand::ScissorDisable => {
                if let Some(canvas) = self.get_screen_canvas(Screen::Main) {
                    let canvas = canvas.read().unwrap();
                    canvas.set_scissor(ScissorRect::disabled());
                }
            }
            DrawCommand::SetMipmap {
                image,
                mipmap,
                hotx,
                hoty,
            } => {
                if let (Some(image), Some(mipmap)) =
                    (self.get_image(image), self.get_image(mipmap))
                {
                    let mipmap_canvas = mipmap.normal();
                    image.set_mipmap(mipmap_canvas, HotSpot::new(hotx, hoty));
                }
            }
            DrawCommand::DeleteImage { image } => {
                self.render_context.write().unwrap().remove_image(image.id());
            }
            DrawCommand::DeleteData { data } => {
                self.render_context.write().unwrap().remove_data_ptr(data);
            }
            DrawCommand::SendSignal { signal } => {
                signal.store(true, Ordering::Release);
            }
            DrawCommand::ReinitVideo { driver, flags, width, height } => {
                debug!(
                    "DCQ reinit video requested: driver={}, flags={}, {}x{}",
                    driver, flags, width, height
                );
            }
            DrawCommand::Callback { callback, arg } => {
                callback(arg);
            }
        }
    }

    fn get_screen_canvas(&self, screen: Screen) -> Option<Arc<RwLock<Canvas>>> {
        self.render_context.read().unwrap().get_screen(ScreenType::from(screen))
    }

    fn get_image(&self, image: ImageRef) -> Option<Arc<TFImage>> {
        self.render_context.read().unwrap().get_image(image.id())
    }
}

/// Execute a closure with scoped DCQ batching.
pub fn scoped_batch<F, R>(queue: &DrawCommandQueue, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = queue.batch();
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::thread;

    fn make_queue_with_capacity(capacity: usize) -> DrawCommandQueue {
        let config = DcqConfig {
            max_size: capacity,
            force_slowdown_size: capacity,
            force_break_size: capacity,
            livelock_max: capacity,
        };
        DrawCommandQueue::with_config(config, Arc::new(RwLock::new(RenderContext::new())))
    }

    fn basic_line_command() -> DrawCommand {
        DrawCommand::Line {
            x1: 0,
            y1: 0,
            x2: 1,
            y2: 1,
            color: Color::new(1, 2, 3, 4),
            draw_mode: DrawMode::Normal,
            dest: Screen::Main,
        }
    }

    #[test]
    fn config_standard_values() {
        let config = DcqConfig::standard();
        assert_eq!(config.max_size, 16_384);
        assert_eq!(config.force_slowdown_size, 4_096);
        assert_eq!(config.force_break_size, 16_384);
        assert_eq!(config.livelock_max, 4_096);
    }

    #[test]
    fn config_debug_values() {
        let config = DcqConfig::debug();
        assert_eq!(config.max_size, 512);
        assert_eq!(config.force_slowdown_size, 128);
        assert_eq!(config.force_break_size, 512);
        assert_eq!(config.livelock_max, 256);
    }

    #[test]
    fn config_validation_ok() {
        let config = DcqConfig::standard();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_validation_fail() {
        let config = DcqConfig {
            max_size: 0,
            force_slowdown_size: 0,
            force_break_size: 0,
            livelock_max: 0,
        };
        assert_eq!(config.validate(), Err(DcqError::InvalidConfig));
    }

    #[test]
    fn stats_utilization() {
        let stats = DcqStats {
            size: 1,
            full_size: 5,
            max_size: 10,
            batching_depth: 0,
        };
        assert_eq!(stats.utilization(), 50.0);
    }

    #[test]
    fn screen_to_screen_type() {
        assert_eq!(ScreenType::from(Screen::Main), ScreenType::Main);
        assert_eq!(ScreenType::from(Screen::Extra), ScreenType::Extra);
        assert_eq!(ScreenType::from(Screen::Transition), ScreenType::Transition);
    }

    #[test]
    fn screen_from_screen_type() {
        assert_eq!(Screen::from(ScreenType::Main), Screen::Main);
        assert_eq!(Screen::from(ScreenType::Extra), Screen::Extra);
        assert_eq!(Screen::from(ScreenType::Transition), Screen::Transition);
    }

    #[test]
    fn draw_mode_variants() {
        let mode = DrawMode::Normal;
        assert_eq!(mode, DrawMode::Normal);
    }

    #[test]
    fn color_new() {
        let color = Color::new(1, 2, 3, 4);
        assert_eq!(color, Color { r: 1, g: 2, b: 3, a: 4 });
    }

    #[test]
    fn point_new() {
        let point = Point::new(5, 6);
        assert_eq!(point, Point { x: 5, y: 6 });
    }

    #[test]
    fn extent_new() {
        let extent = Extent::new(7, 8);
        assert_eq!(extent, Extent { width: 7, height: 8 });
    }

    #[test]
    fn rect_new() {
        let rect = Rect::new(Point::new(1, 2), Extent::new(3, 4));
        assert_eq!(rect.corner.x, 1);
        assert_eq!(rect.extent.height, 4);
    }

    #[test]
    fn image_ref_roundtrip() {
        let id = ImageRef::new(42);
        assert_eq!(id.id(), 42);
    }

    #[test]
    fn colormap_ref_roundtrip() {
        let id = ColorMapRef::new(9);
        assert_eq!(id.id(), 9);
    }

    #[test]
    fn fontchar_ref_fields() {
        let id = FontCharRef::new(2, 99);
        assert_eq!(id.page_id, 2);
        assert_eq!(id.char_code, 99);
    }

    #[test]
    fn command_line_variant() {
        let cmd = basic_line_command();
        matches!(cmd, DrawCommand::Line { .. });
    }

    #[test]
    fn command_rect_variant() {
        let cmd = DrawCommand::Rect {
            rect: Rect::default(),
            color: Color::default(),
            draw_mode: DrawMode::Normal,
            dest: Screen::Main,
        };
        matches!(cmd, DrawCommand::Rect { .. });
    }

    #[test]
    fn command_image_variant() {
        let cmd = DrawCommand::Image {
            image: ImageRef::new(1),
            x: 0,
            y: 0,
            dest: Screen::Main,
            colormap: None,
            draw_mode: DrawMode::Normal,
            scale: 0,
            scale_mode: ScaleMode::Nearest,
        };
        matches!(cmd, DrawCommand::Image { .. });
    }

    #[test]
    fn command_filled_image_variant() {
        let cmd = DrawCommand::FilledImage {
            image: ImageRef::new(2),
            x: 1,
            y: 2,
            color: Color::default(),
            dest: Screen::Main,
            draw_mode: DrawMode::Normal,
            scale: 0,
            scale_mode: ScaleMode::Nearest,
        };
        matches!(cmd, DrawCommand::FilledImage { .. });
    }

    #[test]
    fn command_fontchar_variant() {
        let cmd = DrawCommand::FontChar {
            fontchar: FontCharRef::new(3, 65),
            backing: None,
            x: 0,
            y: 0,
            draw_mode: DrawMode::Normal,
            dest: Screen::Main,
        };
        matches!(cmd, DrawCommand::FontChar { .. });
    }

    #[test]
    fn command_copy_variant() {
        let cmd = DrawCommand::Copy {
            rect: Rect::default(),
            src: Screen::Main,
            dest: Screen::Extra,
        };
        matches!(cmd, DrawCommand::Copy { .. });
    }

    #[test]
    fn command_copy_to_image_variant() {
        let cmd = DrawCommand::CopyToImage {
            image: ImageRef::new(4),
            rect: Rect::default(),
            src: Screen::Main,
        };
        matches!(cmd, DrawCommand::CopyToImage { .. });
    }

    #[test]
    fn command_scissor_enable_variant() {
        let cmd = DrawCommand::ScissorEnable { rect: Rect::default() };
        matches!(cmd, DrawCommand::ScissorEnable { .. });
    }

    #[test]
    fn command_scissor_disable_variant() {
        let cmd = DrawCommand::ScissorDisable;
        matches!(cmd, DrawCommand::ScissorDisable);
    }

    #[test]
    fn command_set_mipmap_variant() {
        let cmd = DrawCommand::SetMipmap {
            image: ImageRef::new(1),
            mipmap: ImageRef::new(2),
            hotx: 0,
            hoty: 0,
        };
        matches!(cmd, DrawCommand::SetMipmap { .. });
    }

    #[test]
    fn command_delete_image_variant() {
        let cmd = DrawCommand::DeleteImage { image: ImageRef::new(1) };
        matches!(cmd, DrawCommand::DeleteImage { .. });
    }

    #[test]
    fn command_delete_data_variant() {
        let cmd = DrawCommand::DeleteData { data: 99 };
        matches!(cmd, DrawCommand::DeleteData { .. });
    }

    #[test]
    fn command_send_signal_variant() {
        let cmd = DrawCommand::SendSignal {
            signal: Arc::new(AtomicBool::new(false)),
        };
        matches!(cmd, DrawCommand::SendSignal { .. });
    }

    #[test]
    fn command_reinit_video_variant() {
        let cmd = DrawCommand::ReinitVideo {
            driver: 1,
            flags: 2,
            width: 320,
            height: 240,
        };
        matches!(cmd, DrawCommand::ReinitVideo { .. });
    }

    #[test]
    fn command_callback_variant() {
        let cmd = DrawCommand::Callback { callback: |_| {}, arg: 5 };
        matches!(cmd, DrawCommand::Callback { .. });
    }

    #[test]
    fn push_pop_fifo() {
        let queue = make_queue_with_capacity(8);
        queue.push(basic_line_command()).unwrap();
        queue.push(DrawCommand::ScissorDisable).unwrap();
        let first = queue.pop().unwrap();
        let second = queue.pop().unwrap();
        matches!(first, DrawCommand::Line { .. });
        matches!(second, DrawCommand::ScissorDisable);
    }

    #[test]
    fn try_push_would_block() {
        let queue = make_queue_with_capacity(1);
        queue.try_push(basic_line_command()).unwrap();
        assert_eq!(queue.try_push(DrawCommand::ScissorDisable), Err(DcqError::WouldBlock));
    }

    #[test]
    fn pop_empty_returns_none() {
        let queue = make_queue_with_capacity(4);
        assert!(queue.pop().is_none());
    }

    #[test]
    fn clear_resets_queue() {
        let queue = make_queue_with_capacity(4);
        queue.push(basic_line_command()).unwrap();
        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn len_and_full_size() {
        let queue = make_queue_with_capacity(4);
        queue.push(basic_line_command()).unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.full_size(), 1);
    }

    #[test]
    fn is_empty_and_full() {
        let queue = make_queue_with_capacity(1);
        assert!(queue.is_empty());
        queue.push(basic_line_command()).unwrap();
        assert!(queue.is_full());
    }

    #[test]
    fn ring_buffer_wraparound() {
        let queue = make_queue_with_capacity(3);
        queue.push(basic_line_command()).unwrap();
        queue.push(DrawCommand::ScissorDisable).unwrap();
        queue.pop();
        queue.pop();
        queue.push(DrawCommand::ScissorDisable).unwrap();
        queue.push(basic_line_command()).unwrap();
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn batching_hides_commands() {
        let queue = make_queue_with_capacity(4);
        let _guard = queue.batch();
        queue.push(basic_line_command()).unwrap();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.full_size(), 1);
    }

    #[test]
    fn unbatch_makes_visible() {
        let queue = make_queue_with_capacity(4);
        let guard = queue.batch();
        queue.push(basic_line_command()).unwrap();
        drop(guard);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn batch_reset_unbatches() {
        let queue = make_queue_with_capacity(4);
        let _guard = queue.batch();
        queue.push(basic_line_command()).unwrap();
        queue.batch_reset();
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn nested_batching_counts() {
        let queue = make_queue_with_capacity(4);
        let _g1 = queue.batch();
        let _g2 = queue.batch();
        assert_eq!(queue.stats().batching_depth, 2);
    }

    #[test]
    fn scoped_batch_executes() {
        let queue = make_queue_with_capacity(4);
        scoped_batch(&queue, || {
            queue.push(basic_line_command()).unwrap();
        });
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn batch_guard_drop_unbatch() {
        let queue = make_queue_with_capacity(4);
        {
            let _guard = queue.batch();
            queue.push(basic_line_command()).unwrap();
        }
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn lock_wait_space_succeeds() {
        let queue = make_queue_with_capacity(2);
        queue.lock_wait_space(1).unwrap();
    }

    #[test]
    fn lock_wait_space_invalid() {
        let queue = make_queue_with_capacity(2);
        assert_eq!(queue.lock_wait_space(3), Err(DcqError::InvalidConfig));
    }

    #[test]
    fn stats_update() {
        let queue = make_queue_with_capacity(2);
        queue.push(basic_line_command()).unwrap();
        let stats = queue.stats();
        assert_eq!(stats.size, 1);
        assert_eq!(stats.full_size, 1);
    }

    #[test]
    fn full_size_counts_batched() {
        let queue = make_queue_with_capacity(2);
        let _guard = queue.batch();
        queue.push(basic_line_command()).unwrap();
        assert_eq!(queue.full_size(), 1);
    }

    #[test]
    fn process_commands_empty_ok() {
        let queue = make_queue_with_capacity(4);
        assert!(queue.process_commands().is_ok());
    }

    #[test]
    fn process_commands_callback() {
        let queue = make_queue_with_capacity(4);
        let hit = Arc::new(AtomicUsize::new(0));
        let hit_clone = Arc::clone(&hit);
        fn callback(arg: u64) {
            let ptr = arg as *const AtomicUsize;
            unsafe {
                (*ptr).fetch_add(1, Ordering::SeqCst);
            }
        }
        let arg = Arc::into_raw(hit_clone) as u64;
        queue
            .push(DrawCommand::Callback { callback, arg })
            .unwrap();
        queue.process_commands().unwrap();
        unsafe {
            let _ = Arc::from_raw(arg as *const AtomicUsize);
        }
        assert_eq!(hit.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn thread_safe_push_pop() {
        let queue = make_queue_with_capacity(16);
        let producer = queue.clone();
        let handle = thread::spawn(move || {
            for _ in 0..5 {
                producer.push(basic_line_command()).unwrap();
            }
        });
        handle.join().unwrap();
        let mut count = 0;
        while queue.pop().is_some() {
            count += 1;
        }
        assert_eq!(count, 5);
    }
}
