//! Drawing through the selected context, ported from frame.c, tfb_prim.c,
//! the text half of font.c and DrawShadowedBox from widgets.c
//! (GPL-2.0-or-later).
//!
//! A drawing onto the screen frame is queued; a drawing onto an image frame
//! happens at once.
use super::batch::Primitive;
use super::dcq::{DrawCommand, Screen};
use super::drawable::{FrameId, FrameKind};
use super::font::{Font, Glyph, Text};
use super::geometry::{Color, DrawMode, Line, Point, Rect};
use super::image::{self, ImageId};
use super::state::{Graphics, GSCALE_IDENTITY, STATUS_CONTEXT, STATUS_FRAME};

fn scaled_extent(extent: i16, scale: i32) -> i32 {
    (i32::from(extent) * scale + (GSCALE_IDENTITY >> 1)) / GSCALE_IDENTITY
}

/// Where a primitive lands: the main screen or an image.
#[derive(Clone, Copy)]
enum Target {
    Screen,
    Image(ImageId),
}

impl Graphics {
    /// C: `_GraphicsStatusFlags & GRAPHICS_ACTIVE == GRAPHICS_ACTIVE`.
    fn graphics_active(&self) -> bool {
        let active = STATUS_CONTEXT | STATUS_FRAME;
        self.current.status & active == active
    }

    /// C: `GetContextValidRect`: the drawable area and the origin drawing
    /// is offset by, or `None` when the clip leaves nothing.
    pub(crate) fn valid_rect(&self) -> Option<(Rect, Point)> {
        let frame = self.frame(self.current.frame);
        let mut rect = Rect {
            corner: Point::default(),
            extent: frame.bounds,
        };
        let mut origin = frame.hot_spot;
        let clip = self.current_context().clip;
        if clip.extent.width != 0 {
            let whole = rect;
            if super::bbox::box_intersect(clip, whole, &mut rect) == 0 {
                return None;
            }
            origin.x = origin.x.wrapping_add(clip.corner.x);
            origin.y = origin.y.wrapping_add(clip.corner.y);
        }
        Some((rect, origin))
    }

    /// The origin of a drawing through the selected context, if it draws.
    fn drawing_origin(&self) -> Option<Point> {
        if !self.graphics_active() {
            return None;
        }
        self.valid_rect().map(|(_, origin)| origin)
    }

    fn target(&self) -> Target {
        let frame = self.frame(self.current.frame);
        if frame.kind == FrameKind::Screen {
            Target::Screen
        } else {
            Target::Image(
                frame
                    .image
                    .unwrap_or_else(|| panic!("drawing onto a frame without an image")),
            )
        }
    }

    fn prim_rect(&mut self, rect: Rect, color: Color, mode: DrawMode) {
        match self.target() {
            Target::Screen => self.enqueue(DrawCommand::Rect {
                rect,
                color,
                mode,
                dest: Screen::Main,
            }),
            Target::Image(target) => image::draw_into(&mut self.images, target, |_, canvas| {
                image::draw_rect(canvas, rect, color, mode);
            }),
        }
    }

    /// C: `TFB_Prim_Point`.
    fn prim_point(&mut self, point: Point, color: Color, mode: DrawMode, origin: Point) {
        let rect = Rect {
            corner: Point::new(
                point.x.wrapping_add(origin.x),
                point.y.wrapping_add(origin.y),
            ),
            extent: super::geometry::Extent::new(1, 1),
        };
        self.prim_rect(rect, color, mode);
    }

    /// C: `TFB_Prim_FillRect`.
    fn prim_fill_rect(&mut self, rect: Rect, color: Color, mode: DrawMode, origin: Point) {
        let mut r = rect;
        r.corner.x = r.corner.x.wrapping_add(origin.x);
        r.corner.y = r.corner.y.wrapping_add(origin.y);
        if self.scale != GSCALE_IDENTITY {
            r.extent.width = scaled_extent(r.extent.width, self.scale) as i16;
            r.extent.height = scaled_extent(r.extent.height, self.scale) as i16;
        }
        self.prim_rect(r, color, mode);
    }

    /// C: `TFB_Prim_Rect`: an outline of four filled arms.
    fn prim_outline(&mut self, rect: Rect, color: Color, mode: DrawMode, origin: Point) {
        let scale = self.scale;
        let mut arm = rect;
        arm.extent.height = 1;
        self.prim_fill_rect(arm, color, mode, origin);
        arm.extent.height = rect.extent.height;
        arm.extent.width = 1;
        self.prim_fill_rect(arm, color, mode, origin);
        arm.corner.x =
            (i32::from(arm.corner.x) + scaled_extent(rect.extent.width, scale) - 1) as i16;
        self.prim_fill_rect(arm, color, mode, origin);
        arm.corner.x = rect.corner.x;
        arm.corner.y =
            (i32::from(arm.corner.y) + scaled_extent(rect.extent.height, scale) - 1) as i16;
        arm.extent.width = rect.extent.width;
        arm.extent.height = 1;
        self.prim_fill_rect(arm, color, mode, origin);
    }

    /// C: `TFB_Prim_Line`.
    fn prim_line(&mut self, line: Line, color: Color, mode: DrawMode, origin: Point) {
        let points = [
            i32::from(line.first.x) + i32::from(origin.x),
            i32::from(line.first.y) + i32::from(origin.y),
            i32::from(line.second.x) + i32::from(origin.x),
            i32::from(line.second.y) + i32::from(origin.y),
        ];
        match self.target() {
            Target::Screen => self.enqueue(DrawCommand::Line {
                points,
                color,
                mode,
                dest: Screen::Main,
            }),
            Target::Image(target) => image::draw_into(&mut self.images, target, |_, canvas| {
                image::draw_line(canvas, points, color, mode);
            }),
        }
    }

    /// The image a stamp draws, with its hot spot installed (C: the
    /// `NormalHs` assignment of `TFB_Prim_Stamp`), or `None` with a warning.
    fn stamp_image(&mut self, frame: FrameId, what: &str) -> Option<ImageId> {
        if frame.is_null() {
            log::warn!("{what}: Tried to draw a NULL frame");
            return None;
        }
        let source = self.frame(frame);
        let hot = source.hot_spot;
        let Some(id) = source.image else {
            log::warn!("Non-existent image to {what}()");
            return None;
        };
        self.images.get_mut(id).normal_hs = hot;
        Some(id)
    }

    /// C: `TFB_Prim_Stamp`.
    fn prim_stamp(&mut self, frame: FrameId, at: Point, mode: DrawMode, origin: Point) {
        let Some(id) = self.stamp_image(frame, "TFB_Prim_Stamp") else {
            return;
        };
        let x = i32::from(at.x) + i32::from(origin.x);
        let y = i32::from(at.y) + i32::from(origin.y);
        let source = self.images.get(id);
        let colormap = (source.canvas().is_indexed() && source.colormap_index != -1)
            .then(|| self.color_map(source.colormap_index));
        let (scale, scale_mode) = (self.scale, self.scale_mode);
        match self.target() {
            Target::Screen => self.enqueue(DrawCommand::Image {
                image: id,
                x,
                y,
                colormap,
                mode,
                scale: if scale == GSCALE_IDENTITY { 0 } else { scale },
                scale_mode,
                dest: Screen::Main,
            }),
            Target::Image(target) => {
                assert_ne!(id, target, "image stamped onto itself");
                image::draw_into(&mut self.images, target, |images, canvas| {
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
                        canvas,
                    );
                });
            }
        }
    }

    /// C: `TFB_Prim_StampFill`.
    fn prim_stamp_fill(
        &mut self,
        frame: FrameId,
        at: Point,
        color: Color,
        mode: DrawMode,
        origin: Point,
    ) {
        let Some(id) = self.stamp_image(frame, "TFB_Prim_StampFill") else {
            return;
        };
        let x = i32::from(at.x) + i32::from(origin.x);
        let y = i32::from(at.y) + i32::from(origin.y);
        let (scale, scale_mode) = (self.scale, self.scale_mode);
        match self.target() {
            Target::Screen => self.enqueue(DrawCommand::FilledImage {
                image: id,
                x,
                y,
                color,
                mode,
                scale: if scale == GSCALE_IDENTITY { 0 } else { scale },
                scale_mode,
                dest: Screen::Main,
            }),
            Target::Image(target) => {
                assert_ne!(id, target, "image stamped onto itself");
                image::draw_into(&mut self.images, target, |images, canvas| {
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
                        canvas,
                    );
                });
            }
        }
    }

    /// C: `TFB_Prim_FontChar`.
    fn prim_font_char(
        &mut self,
        at: Point,
        glyph: Glyph,
        backing: ImageId,
        mode: DrawMode,
        origin: Point,
    ) {
        let x = i32::from(at.x) + i32::from(origin.x);
        let y = i32::from(at.y) + i32::from(origin.y);
        match self.target() {
            Target::Screen => self.enqueue(DrawCommand::FontChar {
                glyph,
                backing,
                x,
                y,
                mode,
                dest: Screen::Main,
            }),
            Target::Image(target) => {
                image::draw_into(&mut self.images, target, |images, canvas| {
                    image::draw_font_char(images, &glyph, backing, x, y, mode, canvas);
                });
            }
        }
    }

    fn clear_background(&mut self, clip: Rect) {
        let background = self.current_context().bg_color;
        self.prim_fill_rect(
            Rect {
                corner: Point::default(),
                extent: clip.extent,
            },
            background,
            DrawMode::REPLACE,
            clip.corner,
        );
    }

    /// C: `DrawBatch`: the primitives in order; `clear` first fills the
    /// drawable area with the background.
    pub(crate) fn draw_batch(&mut self, primitives: &[Primitive], clear: bool) {
        if !self.graphics_active() {
            return;
        }
        let Some((rect, origin)) = self.valid_rect() else {
            return;
        };
        let mode = self.current_context().mode;
        self.queue.batch();
        if clear {
            self.clear_background(rect);
        }
        for primitive in primitives {
            match *primitive {
                Primitive::Point(color, point) => self.prim_point(point, color, mode, origin),
                Primitive::Stamp(ref source, at) => {
                    self.prim_stamp(source.frame(), at, mode, origin);
                }
                Primitive::StampFill(color, ref source, at) => {
                    self.prim_stamp_fill(source.frame(), at, color, mode, origin);
                }
                Primitive::Line(color, line) => self.prim_line(line, color, mode, origin),
                Primitive::Rect(color, rect) => self.prim_outline(rect, color, mode, origin),
                Primitive::FillRect(color, rect) => {
                    self.prim_fill_rect(rect, color, mode, origin);
                }
                Primitive::Empty => {}
            }
        }
        self.queue.unbatch();
    }

    /// C: `ClearDrawable`.
    pub(crate) fn clear_drawable(&mut self) {
        if !self.graphics_active() {
            return;
        }
        if let Some((rect, _)) = self.valid_rect() {
            self.clear_background(rect);
        }
    }

    /// C: `DrawPoint`.
    pub(crate) fn draw_point(&mut self, point: Point) {
        if let Some(origin) = self.drawing_origin() {
            let (color, mode) = (self.current.color, self.current_context().mode);
            self.prim_point(point, color, mode, origin);
        }
    }

    /// C: `DrawRectangle`.
    pub(crate) fn draw_rectangle(&mut self, rect: Rect) {
        if let Some(origin) = self.drawing_origin() {
            let (color, mode) = (self.current.color, self.current_context().mode);
            self.prim_outline(rect, color, mode, origin);
        }
    }

    /// C: `DrawFilledRectangle`.
    pub(crate) fn draw_filled_rectangle(&mut self, rect: Rect) {
        if let Some(origin) = self.drawing_origin() {
            let (color, mode) = (self.current.color, self.current_context().mode);
            self.prim_fill_rect(rect, color, mode, origin);
        }
    }

    /// C: `DrawLine`.
    pub(crate) fn draw_line(&mut self, line: Line) {
        if let Some(origin) = self.drawing_origin() {
            let (color, mode) = (self.current.color, self.current_context().mode);
            self.prim_line(line, color, mode, origin);
        }
    }

    /// C: `DrawStamp`.
    pub(crate) fn draw_stamp(&mut self, frame: FrameId, at: Point) {
        if let Some(origin) = self.drawing_origin() {
            let mode = self.current_context().mode;
            self.prim_stamp(frame, at, mode, origin);
        }
    }

    /// C: `DrawFilledStamp`.
    pub(crate) fn draw_filled_stamp(&mut self, frame: FrameId, at: Point) {
        if let Some(origin) = self.drawing_origin() {
            let (color, mode) = (self.current.color, self.current_context().mode);
            self.prim_stamp_fill(frame, at, color, mode, origin);
        }
    }

    /// C: `TextRect` with the selected font.
    pub(crate) fn text_rect(&self, text: &mut Text<'_>) -> (bool, Rect) {
        Font::measure(self.current.font.map(|font| self.font(font)), text, None)
    }

    /// C: `_text_blt`.
    fn text_blt(&mut self, clip: Rect, text: &Text<'_>, origin: Point) {
        let Some(font) = self.current.font else {
            return;
        };
        let context = self.current_context();
        let mode = context.mode;
        let Some(backing) = context.font_backing else {
            return;
        };
        for (at, glyph) in self.font(font).layout(text, clip) {
            self.prim_font_char(at, glyph, backing, mode, origin);
        }
    }

    /// C: `font_DrawText`.
    pub(crate) fn draw_text(&mut self, mut text: Text<'_>) {
        self.fix_font_effect();
        let Some(origin) = self.drawing_origin() else {
            return;
        };
        let (visible, clip) = self.text_rect(&mut text);
        if visible {
            self.text_blt(clip, &text, origin);
        }
    }

    /// C: `font_DrawTracedText`: `trace` around the text, then the text.
    pub(crate) fn draw_traced_text(&mut self, mut text: Text<'_>, color: Color, trace: Color) {
        let old = self.set_foreground(trace);
        for (dx, dy) in [(-1, 0), (2, 0), (-1, -1), (0, 2)] {
            text.baseline.x = text.baseline.x.wrapping_add(dx);
            text.baseline.y = text.baseline.y.wrapping_add(dy);
            self.draw_text(text);
        }
        text.baseline.y = text.baseline.y.wrapping_sub(1);
        self.set_foreground(color);
        self.draw_text(text);
        self.set_foreground(old);
    }

    /// C: `DrawShadowedBox`.
    pub(crate) fn draw_shadowed_box(&mut self, r: Rect, bg: Color, dark: Color, medium: Color) {
        let mut t = r;
        self.queue.batch();
        t.corner.x = t.corner.x.wrapping_sub(2);
        t.corner.y = t.corner.y.wrapping_sub(2);
        t.extent.width = t.extent.width.wrapping_add(4);
        t.extent.height = t.extent.height.wrapping_add(4);
        let old = self.set_foreground(dark);
        self.draw_filled_rectangle(t);
        t.corner.x += 2;
        t.corner.y += 2;
        t.extent.width -= 2;
        t.extent.height -= 2;
        self.set_foreground(medium);
        self.draw_filled_rectangle(t);
        t.corner.x -= 1;
        t.corner.y = t.corner.y.wrapping_add(r.extent.height).wrapping_add(1);
        t.extent.height = 1;
        self.draw_filled_rectangle(t);
        t.corner.x = t.corner.x.wrapping_add(r.extent.width).wrapping_add(2);
        t.corner.y = t.corner.y.wrapping_sub(r.extent.height).wrapping_sub(2);
        t.extent.width = 1;
        self.draw_filled_rectangle(t);
        self.set_foreground(bg);
        self.draw_filled_rectangle(r);
        self.set_foreground(old);
        self.queue.unbatch();
    }

    /// C: `TFB_DrawScreen_Copy`; `None` copies the whole screen.
    pub(crate) fn enqueue_copy(&mut self, rect: Option<Rect>, source: Screen, dest: Screen) {
        let rect = rect.unwrap_or(Rect::new(
            0,
            0,
            super::state::SCREEN_WIDTH as i16,
            super::state::SCREEN_HEIGHT as i16,
        ));
        self.enqueue(DrawCommand::Copy { rect, source, dest });
    }
}
