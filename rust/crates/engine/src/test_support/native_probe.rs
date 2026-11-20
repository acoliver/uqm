//! Probes over the process graphics state for serialized fixtures.
//!
//! Every fixture that calls these is `#[serial]` and owns the screens, queue
//! and frames it probes. Frames and contexts come from the engine's own
//! lease and drawable APIs, never fabricated by a test.
use crate::graphics::context::ContextId;
use crate::graphics::drawable::{ContextFrame, FrameId, LoadedDrawable};
use crate::graphics::font::FontId;
use crate::graphics::geometry::{Color, DrawMode, HotSpot, Point};
use crate::graphics::state::{self, VideoMode};

/// Run every queued draw command.
pub fn flush_graphics() {
    state::flush();
}

/// Headless screens and an empty draw queue, released on drop.
pub struct DrawQueue(());

impl DrawQueue {
    pub fn init() -> Self {
        state::init_headless(VideoMode::default());
        Self(())
    }
}

impl Drop for DrawQueue {
    fn drop(&mut self) {
        state::uninit_headless();
    }
}

pub fn batch_graphics() {
    state::batch();
}

pub fn unbatch_graphics() {
    state::unbatch();
}

pub fn batch_depth() -> i32 {
    state::batch_depth()
}

/// Read a `width` by `height` RAM frame as colors. `None` for the null frame.
pub fn read_pixels(frame: FrameId, width: i32, height: i32) -> Option<Vec<Color>> {
    if frame.is_null() {
        return None;
    }
    let mut pixels = vec![Color::default(); width as usize * height as usize];
    ContextFrame::retain(frame).read_colors(&mut pixels, width, height);
    Some(pixels)
}

/// Overwrite a `width` by `height` RAM frame. `false` for the null frame.
pub fn write_pixels(frame: FrameId, pixels: &[Color], width: i32, height: i32) -> bool {
    if frame.is_null() {
        return false;
    }
    ContextFrame::retain(frame).write_colors(pixels, width, height);
    true
}

/// C: `GetFramePixel`.
pub fn frame_pixel(frame: FrameId, at: Point) -> Color {
    if frame.is_null() {
        return Color::rgba(0, 0, 0, 0);
    }
    ContextFrame::retain(frame).pixel(at)
}

/// The currently selected context.
pub fn selected_context() -> Option<ContextId> {
    crate::graphics::context::selected()
}

/// The font a context currently holds.
pub fn context_font(context: ContextId) -> Option<FontId> {
    state::with(|g| g.context(context).font)
}

/// Draw mode of the selected context.
pub fn selected_draw_mode() -> DrawMode {
    state::with(|g| g.draw_mode())
}

/// Set the graphic scale and return the previous one.
pub fn set_graphic_scale(scale: i32) -> i32 {
    state::set_graphic_scale(scale)
}

pub fn graphic_scale() -> i32 {
    state::graphic_scale()
}

/// The screen drawable and its frame, released on drop.
pub struct Display {
    drawable: LoadedDrawable,
}

impl Display {
    pub fn create() -> Self {
        Self {
            drawable: LoadedDrawable::display(0),
        }
    }

    pub fn frame(&self) -> FrameId {
        self.drawable.first_frame()
    }
}

/// The frame at `index` within the same drawable as `frame`.
pub fn frame_at(frame: FrameId, index: u16) -> FrameId {
    frame.at(index)
}

pub fn set_frame_hot_spot(frame: FrameId, hot: HotSpot) {
    frame.set_hot_spot(hot);
}

/// The number identifying the drawable a frame belongs to, stable across
/// conversations.
pub fn release_drawable(frame: FrameId) -> usize {
    frame.drawable().map_or(0, |drawable| drawable.bits())
}

/// Headless screens for fixtures that draw to the display, released on drop.
pub struct SdlSurfaces(DrawQueue);

impl SdlSurfaces {
    pub fn install(_width: u32, _height: u32) -> Result<Self, String> {
        Ok(Self(DrawQueue::init()))
    }
}

/// Install a packed color-map table (a two-byte range header, then 256 RGB entries per map).
pub fn set_color_map(table: &[u8]) {
    assert!(table.len() >= 2);
    let maps = usize::from(table[1] - table[0]) + 1;
    assert_eq!(table.len(), 2 + maps * 256 * 3);
    state::set_color_map(table);
}

/// C: `_GraphicsStatusFlags`, for fixtures that switch a status bit and
/// restore it.
pub fn graphics_status() -> u8 {
    state::with(|g| g.current.status)
}

pub fn set_graphics_status(status: u8) {
    state::with(|g| g.current.status = status);
}

/// Give `frame` a 4x4 indexed image whose pixels are all index 1, with no
/// transparent index, so collision tests see a solid shape.
pub fn make_opaque(frame: FrameId) {
    use crate::graphics::canvas::{Canvas, PixelFormat};
    use crate::graphics::image::Image;
    let mut canvas = Canvas::new(PixelFormat::Indexed, 4, 4);
    canvas.set_transparent_index(-1);
    canvas.write_indexes(&[1u8; 16], 4, 4);
    state::with(|g| {
        let image = g.images.insert(Image::new(canvas));
        let previous = g.frame_mut(frame).image.replace(image);
        if let Some(previous) = previous {
            g.images.remove(previous);
        }
    });
}

/// Run `inspect` with a loaded font's description.
pub fn with_font<R>(font: FontId, inspect: impl FnOnce(&crate::graphics::font::Font) -> R) -> R {
    state::with(|g| inspect(g.font(font)))
}

/// What a test reads of one frame: kind, index, bounds, hot spot, colour
/// map index, canvas transparent key and the first `rows` rows of indexes.
#[derive(Debug, PartialEq, Eq)]
pub struct FrameProbe {
    pub kind: Option<crate::graphics::drawable::FrameKind>,
    pub index: u16,
    pub bounds: crate::graphics::geometry::Extent,
    pub hot_spot: HotSpot,
    pub colormap_index: i32,
    pub key: Option<u32>,
}

pub fn probe_frame(frame: FrameId) -> FrameProbe {
    state::with(|g| {
        let record = g.frame(frame);
        let image = g.frame_image(frame);
        FrameProbe {
            kind: Some(record.kind),
            index: frame.index(),
            bounds: record.bounds,
            hot_spot: record.hot_spot,
            colormap_index: image.colormap_index,
            key: image.canvas().color_key(),
        }
    })
}

/// The row `y` of an indexed frame's pixels, `width` entries long.
pub fn frame_index_row(frame: FrameId, y: i32, width: i32) -> Vec<u8> {
    state::with(|g| {
        let canvas = g.frame_image(frame).canvas();
        (0..width).map(|x| canvas.pixel(x, y) as u8).collect()
    })
}

/// The palette of colour map `index` as the draw queue would apply it.
pub fn color_map_palette(index: i32) -> [crate::graphics::geometry::Color; 256] {
    state::with(|g| g.colormaps.get(index).palette)
}

/// Commands queued and commands visible to the renderer (C: the queue's
/// `FullSize` and `Size`).
pub fn queue_sizes() -> (usize, usize) {
    state::with(|g| (g.queue.len(), g.queue.visible()))
}

/// Frames rendered since the last FPS sample.
pub fn rendered_frames() -> i32 {
    state::with(|g| g.flush.rendered_frames)
}

/// C: `GetContextClipRect` on the selected context: the clip (the whole
/// frame when none is set) and whether one is set.
pub fn selected_clip() -> (crate::graphics::geometry::Rect, bool) {
    state::with(|g| g.clip().unwrap_or_else(|| panic!("no context selected")))
}

/// Adjust a loaded font, for fixtures that shape a one-glyph font.
pub fn with_font_mut<R>(
    font: FontId,
    edit: impl FnOnce(&mut crate::graphics::font::Font) -> R,
) -> R {
    state::with(|g| edit(g.font_mut(font)))
}

/// A context's foreground colour (C: `context->ForeGroundColor`).
pub fn context_foreground(context: ContextId) -> Color {
    state::with(|g| g.context(context).fg_color)
}

/// Mark `rect` of the main screen changed, as drawing would.
pub fn register_rect(rect: crate::graphics::geometry::Rect) {
    state::with(|g| g.bbox.register_rect(rect));
}

/// Live images and drawables, for fixtures that check every owner released
/// what it allocated.
pub fn allocations() -> (usize, usize) {
    state::with(|g| (g.images.len(), g.drawables.entries.len()))
}

/// A frame's bounds and whether it holds pixels.
pub fn frame_bounds(frame: FrameId) -> (crate::graphics::geometry::Extent, bool) {
    state::with(|g| {
        let record = g.frame(frame);
        (record.bounds, record.image.is_some())
    })
}
