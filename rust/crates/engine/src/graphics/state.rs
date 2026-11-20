//! The graphics state of the game thread: screens, images, drawables,
//! contexts, fonts, colour maps, the draw command queue and the display it
//! is presented on.
//!
//! One value replaces the C globals of dcqueue.c, cmap.c, context.c,
//! drawable.c, gfx_common.c, sdl_common.c and bbox.c. Graphics belong to the
//! thread that draws (the game thread), so the state is thread-local. It is
//! reached through [`with`], which must not be re-entered: every function
//! here takes the state for one step and releases it before calling out
//! (callbacks, event pumping, presentation observers). Screen commands a step
//! queues are pushed after it releases the state, because a full queue is
//! flushed first.
use super::arena::Arena;
use super::bbox::BoundingBox;
use super::canvas::{Canvas, PixelFormat};
use super::colormap::{ColorMap, ColorMapTable, ColorMaps};
use super::context::{ContextId, Contexts};
use super::dcq::{DrawCommand, Queue, Screen};
use super::drawable::{Drawables, FrameId, FrameKind};
use super::font::{FontId, Fonts, Glyph};
use super::geometry::{Color, DrawMode, Point, Rect};
use super::image::{self, Image, ImageId};
use super::present::Presentation;
use std::cell::RefCell;
use std::sync::Arc;

/// `GSCALE_IDENTITY`.
pub const GSCALE_IDENTITY: i32 = 256;

/// The logical screen size the game draws at.
pub const SCREEN_WIDTH: i32 = 320;
pub const SCREEN_HEIGHT: i32 = 240;

/// `TFB_GFXFLAGS_*` bits used by the engine.
pub const GFXFLAGS_FULLSCREEN: i32 = 1;
pub const GFXFLAGS_SHOWFPS: i32 = 2;
pub const GFXFLAGS_SCANLINES: i32 = 4;

/// Video mode as configured (C: `GraphicsDriver`, `GfxFlags`,
/// `ScreenWidthActual`, `ScreenHeightActual`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VideoMode {
    pub driver: i32,
    pub flags: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub(crate) struct FlushState {
    pub last_fade: i32,
    pub last_transition: i32,
    pub rendered_frames: i32,
    pub last_time: u32,
    pub fps_counter: u32,
}

#[derive(Debug)]
pub(crate) struct SwapState {
    pub last_fade: i32,
    pub last_transition: i32,
    pub system_box: Option<Rect>,
}

/// C: `_GraphicsStatusFlags` bits.
pub(crate) const STATUS_ACTIVE: u8 = 1;
pub(crate) const STATUS_CONTEXT: u8 = 4;
pub(crate) const STATUS_FRAME: u8 = 8;

/// The current drawing selection (C: `_pCurContext`, `_GraphicsStatusFlags`,
/// `_CurFramePtr`, `_CurFontPtr` and `_locPrim.color`).
#[derive(Debug, Default)]
pub(crate) struct Selection {
    pub context: Option<ContextId>,
    pub status: u8,
    pub frame: FrameId,
    pub font: Option<FontId>,
    pub color: Color,
}

pub(crate) struct Graphics {
    pub images: Arena<Image>,
    pub drawables: Drawables,
    pub contexts: Contexts,
    pub fonts: Fonts,
    pub colormaps: ColorMaps,
    pub queue: Queue,
    /// Commands queued by the current step, pushed when it ends.
    pending: Vec<DrawCommand>,
    pub screens: Vec<Canvas>,
    pub bbox: BoundingBox,
    pub scale: i32,
    pub scale_mode: i32,
    pub transition_amount: i32,
    pub transition_clip: Rect,
    pub video: VideoMode,
    pub quit_posted: bool,
    pub game_active: bool,
    pub flush: FlushState,
    pub swap: SwapState,
    pub display: Option<Presentation>,
    pub current: Selection,
    /// The context flashes draw with, created on first use.
    pub flash_work: Option<ContextId>,
}

impl Graphics {
    fn new() -> Self {
        Self {
            images: Arena::new(),
            drawables: Drawables::default(),
            contexts: Contexts::default(),
            fonts: Fonts::default(),
            colormaps: ColorMaps::new(),
            queue: Queue::new(),
            pending: Vec::new(),
            screens: Vec::new(),
            bbox: BoundingBox::new(),
            scale: GSCALE_IDENTITY,
            scale_mode: 1,
            transition_amount: 255,
            transition_clip: Rect::default(),
            video: VideoMode::default(),
            quit_posted: false,
            game_active: true,
            flush: FlushState {
                last_fade: 255,
                last_transition: 255,
                rendered_frames: 0,
                last_time: 0,
                fps_counter: 0,
            },
            swap: SwapState {
                last_fade: 255,
                last_transition: 255,
                system_box: None,
            },
            display: None,
            current: Selection::default(),
            flash_work: None,
        }
    }

    /// Queue a screen command (C: `TFB_EnqueueDrawCommand`). A drawing
    /// command onto the screen frame first queues a scissor change when the
    /// selected context's clip differs from the last one queued.
    pub(crate) fn enqueue(&mut self, command: DrawCommand) {
        if command.draws()
            && self.current.frame.drawable().is_some()
            && self.frame(self.current.frame).kind == FrameKind::Screen
        {
            let current = self.queue.scissor;
            let change = match self.current.context {
                Some(context) => {
                    let clip = self.context(context).clip;
                    (clip != current).then(|| {
                        self.queue.scissor = clip;
                        clip
                    })
                }
                None if current.extent.width != 0 => {
                    self.queue.scissor.extent.width = 0;
                    Some(self.queue.scissor)
                }
                None => None,
            };
            if let Some(clip) = change {
                self.pending.push(if clip.extent.width != 0 {
                    DrawCommand::ScissorEnable(clip)
                } else {
                    DrawCommand::ScissorDisable
                });
            }
        }
        self.pending.push(command);
    }

    pub(crate) fn enqueue_all(&mut self, commands: Vec<DrawCommand>) {
        for command in commands {
            self.enqueue(command);
        }
    }

    pub(crate) fn screen(&mut self, screen: Screen) -> &mut Canvas {
        assert!(
            !self.screens.is_empty(),
            "graphics screens are not initialized"
        );
        &mut self.screens[screen.index()]
    }

    /// C: `TFB_GetColorMap`.
    pub(crate) fn color_map(&self, index: i32) -> Arc<ColorMap> {
        self.colormaps.get(index)
    }
}

thread_local! {
    static GRAPHICS: RefCell<Graphics> = RefCell::new(Graphics::new());
}

/// Run `step` on the graphics state, then push the screen commands it
/// queued.
pub(crate) fn with<R>(step: impl FnOnce(&mut Graphics) -> R) -> R {
    let (result, pending) = GRAPHICS.with(|cell| {
        let mut graphics = cell
            .try_borrow_mut()
            .unwrap_or_else(|_| panic!("graphics state re-entered"));
        let result = step(&mut graphics);
        (result, std::mem::take(&mut graphics.pending))
    });
    for command in pending {
        push(command);
    }
    result
}

/// Push commands released outside a step (lease drops).
pub(crate) fn enqueue_all(commands: Vec<DrawCommand>) {
    if !commands.is_empty() {
        with(|g| g.enqueue_all(commands));
    }
}

/// C: `TFB_DrawCommandQueue_Push`, flushing first when the queue is full.
fn push(command: DrawCommand) {
    while with(|g| g.queue.full()) {
        reset_batch();
        flush_ex(true);
    }
    with(|g| g.queue.push(command));
}

// ----- lifecycle -----------------------------------------------------------

/// Create the screens, queue and bounding box for `mode` and show them on
/// `display` (C: `TFB_InitGraphics` after the window exists).
pub(crate) fn init(mode: VideoMode, display: Option<Presentation>) {
    with(|g| {
        assert!(g.screens.is_empty(), "graphics already initialized");
        g.video = mode;
        g.screens = Screen::ALL
            .iter()
            .map(|_| Canvas::new(PixelFormat::Rgbx, SCREEN_WIDTH, SCREEN_HEIGHT))
            .collect();
        g.queue = Queue::new();
        g.bbox = BoundingBox::new();
        g.bbox.init(SCREEN_WIDTH, SCREEN_HEIGHT);
        g.display = display;
    });
}

/// Release the screens and the display (C: `TFB_UninitGraphics`).
pub(crate) fn uninit() -> Option<Presentation> {
    with(|g| {
        g.screens.clear();
        g.display.take()
    })
}

/// Screens without a display, for headless replay and tests. Panics when
/// graphics are already initialized.
pub fn init_headless(mode: VideoMode) {
    init(mode, None);
}

/// Release headless screens.
pub fn uninit_headless() {
    assert!(uninit().is_none(), "headless graphics had a display");
}

pub fn initialized() -> bool {
    with(|g| !g.screens.is_empty())
}

/// Fail fast when `what` runs before graphics are initialized on this thread.
pub fn require_initialized(what: &str) {
    assert!(initialized(), "{what} requires initialized graphics");
}

pub fn video_mode() -> VideoMode {
    with(|g| g.video)
}

pub fn quit_posted() -> bool {
    with(|g| g.quit_posted)
}

pub fn game_active() -> bool {
    with(|g| g.game_active)
}

/// The logical screen size (C: `ScreenWidth`, `ScreenHeight`).
pub fn screen_dimensions() -> (i32, i32) {
    (SCREEN_WIDTH, SCREEN_HEIGHT)
}

/// C: `TFB_DrawScreen_ReinitVideo`: queue a mode switch.
pub fn reinit_video(mode: VideoMode) {
    with(|g| g.enqueue(DrawCommand::ReinitVideo(mode)));
}

/// Draw directly onto a screen canvas, outside the queue (movie frames).
pub fn draw_on_screen<R>(screen: Screen, draw: impl FnOnce(&mut Canvas) -> R) -> R {
    with(|g| draw(g.screen(screen)))
}

// ----- scale and transition ------------------------------------------------

/// C: `SetGraphicScale`; zero means identity. Returns the previous scale.
pub fn set_graphic_scale(scale: i32) -> i32 {
    with(|g| {
        std::mem::replace(
            &mut g.scale,
            if scale == 0 { GSCALE_IDENTITY } else { scale },
        )
    })
}

pub fn graphic_scale() -> i32 {
    with(|g| g.scale)
}

/// C: `SetGraphicScaleMode`. Returns the previous mode.
pub fn set_graphic_scale_mode(mode: i32) -> i32 {
    assert!((1..=3).contains(&mode), "graphic scale mode {mode}");
    with(|g| std::mem::replace(&mut g.scale_mode, mode))
}

pub fn graphic_scale_mode() -> i32 {
    with(|g| g.scale_mode)
}

// ----- colour maps ---------------------------------------------------------

fn now() -> u32 {
    crate::time::counter_abi::GetTimeCounter()
}

/// C: `InitColorMaps`.
pub fn init_color_maps() {
    with(|g| g.colormaps.init());
}

/// C: `UninitColorMaps`.
pub fn uninit_color_maps() {
    with(|g| g.colormaps.uninit());
}

/// C: `SetColorMap`. A malformed table (game content) is reported and
/// leaves every map unchanged.
pub fn set_color_map(bytes: &[u8]) {
    match ColorMapTable::parse(bytes) {
        Ok(table) => with(|g| g.colormaps.set(table)),
        Err(error) => log::error!("SetColorMap: {error}"),
    }
}

/// C: `GetFadeAmount`.
pub fn fade_amount() -> i32 {
    let now = now();
    with(|g| g.colormaps.fade_amount(now))
}

/// C: `FadeScreen`: start a fade; returns the tick it ends.
pub fn fade_screen(kind: u32, interval: i16) -> u32 {
    let now = now();
    with(|g| {
        let quit = g.quit_posted;
        g.colormaps.fade_screen(kind, interval, now, quit)
    })
}

/// C: `XFormColorMap`: returns the tick the transform ends; a malformed
/// table is reported and starts nothing.
pub fn xform_color_map(bytes: &[u8], interval: i16) -> u32 {
    let table = match ColorMapTable::parse(bytes) {
        Ok(table) => table,
        Err(error) => {
            log::error!("XFormColorMap: {error}");
            return 0;
        }
    };
    let now = now();
    with(|g| {
        let quit = g.quit_posted;
        g.colormaps.xform(table, interval, now, quit)
    })
}

/// C: `XFormColorMap_step`.
pub fn step_color_map_xforms() -> bool {
    let now = now();
    with(|g| g.colormaps.step(now))
}

/// C: `FlushColorXForms`.
pub fn flush_color_xforms() {
    with(|g| g.colormaps.flush_xforms());
}

// ----- the draw command queue ----------------------------------------------

/// C: `TFB_BatchGraphics`.
pub fn batch() {
    with(|g| g.queue.batch());
}

/// C: `TFB_UnbatchGraphics`.
pub fn unbatch() {
    with(|g| g.queue.unbatch());
}

/// C: `TFB_GetBatchDepth`.
pub fn batch_depth() -> i32 {
    with(|g| g.queue.batch_depth())
}

/// Apply exactly `levels` batch corrections: positive ends batches,
/// negative begins them.
pub fn correct_batch_by(levels: i32) {
    for _ in 0..levels.max(0) {
        unbatch();
    }
    for _ in 0..(-levels).max(0) {
        batch();
    }
}

/// C: `TFB_DrawScreen_Callback`: run `callback` when the queue reaches it.
pub fn callback(callback: impl FnOnce() + 'static) {
    with(|g| g.enqueue(DrawCommand::Callback(Box::new(callback))));
}

/// C: `TFB_BatchReset`.
pub fn reset_batch() {
    with(|g| g.queue.reset_batch());
}

/// C: `TFB_FlushGraphics`: replay the queue and present.
pub fn flush() {
    flush_ex(false);
}

/// C: `FlushGraphics` / `TFB_DrawScreen_WaitForSignal`: replay the queue,
/// presenting unless batching is active.
pub fn wait_for_signal() {
    if batch_depth() > 0 {
        unbatch();
        flush_ex(true);
        batch();
    } else {
        flush();
    }
}

/// C: `TFB_FlushGraphicsEx`.
pub fn flush_ex(skip_swap: bool) {
    if with(|g| g.queue.visible() == 0) {
        let fade = fade_amount();
        let (swap, transition) = with(|g| {
            let transition = g.transition_amount;
            let last = &g.flush;
            let swap = !skip_swap
                && ((fade != 255 && fade != last.last_fade)
                    || (transition != 255 && transition != last.last_transition)
                    || (fade == 255 && last.last_fade != 255)
                    || (transition == 255 && last.last_transition != 255));
            (swap, transition)
        });
        if swap {
            super::present::swap_buffers(super::present::Redraw::Fading);
        } else {
            crate::threading::task_switch();
        }
        with(|g| {
            g.flush.last_fade = fade;
            g.flush.last_transition = transition;
        });
        return;
    }
    if with(|g| g.video.flags & GFXFLAGS_SHOWFPS != 0) {
        compute_fps();
    }
    with(|g| g.bbox.reset());
    while let Some(command) = with(|g| g.queue.pop()) {
        execute(command);
    }
    if !skip_swap {
        super::present::swap_buffers(super::present::Redraw::No);
    }
    with(|g| g.flush.rendered_frames += 1);
}

fn compute_fps() {
    let now = now();
    with(|g| {
        let f = &mut g.flush;
        let delta = now.wrapping_sub(f.last_time);
        f.last_time = now;
        f.fps_counter = f.fps_counter.wrapping_add(delta);
        if f.fps_counter > 840 / 100 {
            log::info!(
                "fps {:.2}, effective {:.2}",
                840.0f32 / delta as f32,
                840.0f32 * f.rendered_frames as f32 / f.fps_counter as f32
            );
            f.fps_counter = 0;
            f.rendered_frames = 0;
        }
    });
}

fn register_image(
    images: &Arena<Image>,
    bbox: &mut BoundingBox,
    id: ImageId,
    x: i32,
    y: i32,
    scale: i32,
) {
    let image = images.get(id);
    let (origin, extent) = if scale != 0 {
        (image.last_scale_hot_spot(), image.scaled_extent())
    } else {
        (image.normal_hs, image.canvas_extent())
    };
    bbox.register_rect(Rect {
        corner: Point::new(
            (x - i32::from(origin.x)) as i16,
            (y - i32::from(origin.y)) as i16,
        ),
        extent,
    });
}

fn execute(command: DrawCommand) {
    match command {
        DrawCommand::Callback(callback) => callback(),
        DrawCommand::ReinitVideo(mode) => super::present::reinit_video(mode),
        command => with(|g| execute_drawing(g, command)),
    }
}

fn execute_drawing(g: &mut Graphics, command: DrawCommand) {
    if matches!(
        command,
        DrawCommand::Image { .. } | DrawCommand::FilledImage { .. } | DrawCommand::FontChar { .. }
    ) {
        return execute_image(g, command);
    }
    let Graphics {
        images,
        screens,
        bbox,
        ..
    } = g;
    match command {
        DrawCommand::SetMipmap { image, mipmap, hot } => {
            image::set_mipmap(images, image, mipmap, hot);
        }
        DrawCommand::Line {
            points,
            color,
            mode,
            dest,
        } => {
            if dest == Screen::Main {
                bbox.register_point(points[0], points[1]);
                bbox.register_point(points[2], points[3]);
            }
            image::draw_line(&mut screens[dest.index()], points, color, mode);
        }
        DrawCommand::Rect {
            rect,
            color,
            mode,
            dest,
        } => {
            if dest == Screen::Main {
                bbox.register_rect(rect);
            }
            image::draw_rect(&mut screens[dest.index()], rect, color, mode);
        }
        DrawCommand::ScissorEnable(rect) => {
            screens[Screen::Main.index()].set_clip_rect(Some(pixel_rect(rect)));
            bbox.set_clip(Some(rect));
        }
        DrawCommand::ScissorDisable => {
            screens[Screen::Main.index()].set_clip_rect(None);
            bbox.set_clip(None);
        }
        DrawCommand::CopyToImage {
            image,
            rect,
            source,
        } => {
            image::copy_screen_into(images, image, &screens[source.index()], rect);
        }
        DrawCommand::Copy { rect, source, dest } => {
            if dest == Screen::Main {
                bbox.register_rect(rect);
            }
            let (from, to) = pair(screens, source, dest);
            image::copy_rect(from, rect, to, rect.corner);
        }
        DrawCommand::DeleteImage(id) => {
            images.remove(id);
        }
        DrawCommand::DeleteData(data) => drop(data),
        DrawCommand::Image { .. }
        | DrawCommand::FilledImage { .. }
        | DrawCommand::FontChar { .. } => {
            unreachable!("executed by execute_image")
        }
        DrawCommand::ReinitVideo(_) | DrawCommand::Callback(_) => {
            unreachable!("executed outside the graphics state")
        }
    }
}

/// A queued glyph, marking its box on the main screen.
fn draw_glyph(
    images: &mut Arena<Image>,
    screens: &mut [Canvas],
    bbox: &mut BoundingBox,
    (glyph, backing): (&Glyph, ImageId),
    (x, y): (i32, i32),
    mode: DrawMode,
    dest: Screen,
) {
    image::draw_font_char(
        images,
        glyph,
        backing,
        x,
        y,
        mode,
        &mut screens[dest.index()],
    );
    if dest == Screen::Main {
        bbox.register_rect(Rect {
            corner: Point::new(
                (x - i32::from(glyph.hot_spot.x)) as i16,
                (y - i32::from(glyph.hot_spot.y)) as i16,
            ),
            extent: glyph.extent,
        });
    }
}

/// Image, filled-image and glyph commands, which mark the main screen's
/// changed area by what they drew.
fn execute_image(g: &mut Graphics, command: DrawCommand) {
    let Graphics {
        images,
        screens,
        bbox,
        ..
    } = g;
    match command {
        DrawCommand::Image {
            image: id,
            x,
            y,
            colormap,
            mode,
            scale,
            scale_mode,
            dest,
        } => {
            image::draw(
                images,
                id,
                image::Placement {
                    x,
                    y,
                    scale,
                    scale_mode,
                },
                colormap,
                mode,
                &mut screens[dest.index()],
            );
            if dest == Screen::Main {
                register_image(images, bbox, id, x, y, scale);
            }
        }
        DrawCommand::FilledImage {
            image: id,
            x,
            y,
            color,
            mode,
            scale,
            scale_mode,
            dest,
        } => {
            image::draw_filled(
                images,
                id,
                image::Placement {
                    x,
                    y,
                    scale,
                    scale_mode,
                },
                color,
                mode,
                &mut screens[dest.index()],
            );
            if dest == Screen::Main {
                register_image(images, bbox, id, x, y, scale);
            }
        }
        DrawCommand::FontChar {
            glyph,
            backing,
            x,
            y,
            mode,
            dest,
        } => draw_glyph(images, screens, bbox, (&glyph, backing), (x, y), mode, dest),
        _ => unreachable!("not an image command"),
    }
}

fn pair(screens: &mut [Canvas], a: Screen, b: Screen) -> (&Canvas, &mut Canvas) {
    assert_ne!(a, b, "screen copied onto itself");
    let (a, b) = (a.index(), b.index());
    if a < b {
        let (low, high) = screens.split_at_mut(b);
        (&low[a], &mut high[0])
    } else {
        let (low, high) = screens.split_at_mut(a);
        (&high[0], &mut low[b])
    }
}

pub(crate) fn pixel_rect(rect: Rect) -> super::canvas::PixelRect {
    super::canvas::PixelRect::new(
        i32::from(rect.corner.x),
        i32::from(rect.corner.y),
        i32::from(rect.extent.width),
        i32::from(rect.extent.height),
    )
}

/// C: `TFB_PurgeDanglingGraphics`: drop queued drawing, releasing the images
/// it was asked to delete.
pub fn purge() {
    with(|g| {
        let deleted: Vec<ImageId> = g
            .queue
            .drain()
            .filter_map(|command| match command {
                DrawCommand::DeleteImage(id) => Some(id),
                _ => None,
            })
            .collect();
        for id in deleted {
            g.images.remove(id);
        }
    });
}

#[cfg(test)]
mod thread_tests {
    use super::super::{context::LoadedContext, drawable::LoadedDrawable};

    /// #891: graphics state belongs to the thread that uses it, so threads
    /// allocate and release without racing each other's registries. (The
    /// observation log is process-wide, hence the serial guard.)
    #[test]
    #[serial_test::serial]
    fn parallel_threads_allocate_and_release_independently() {
        let workers: Vec<_> = (0..4)
            .map(|worker| {
                std::thread::spawn(move || {
                    for _ in 0..50 {
                        let context = LoadedContext::create(c"parallel fixture");
                        let drawable = LoadedDrawable::create(0, 4, 4, 2)
                            .unwrap_or_else(|| panic!("worker {worker} drawable"));
                        let lease = drawable.lease();
                        drop(drawable);
                        assert!(lease.frame(1).is_some(), "a lease outlives the owner");
                        drop(lease);
                        drop(context);
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap_or_else(|_| panic!("worker"));
        }
    }
}
