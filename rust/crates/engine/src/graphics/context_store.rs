//! The context store: creation, selection, release and the per-context
//! drawing state the selected context carries (context.c).
use super::*;

impl Graphics {
    pub(crate) fn context(&self, context: ContextId) -> &Context {
        self.contexts
            .entries
            .get(&context.0)
            .unwrap_or_else(|| panic!("context {context:?} was released"))
    }

    pub(crate) fn context_mut(&mut self, context: ContextId) -> &mut Context {
        self.contexts
            .entries
            .get_mut(&context.0)
            .unwrap_or_else(|| panic!("context {context:?} was released"))
    }

    /// The selected context; panics when none is.
    pub(crate) fn current_context(&self) -> &Context {
        self.context(
            self.current
                .context
                .unwrap_or_else(|| panic!("no context selected")),
        )
    }

    pub(super) fn current_context_mut(&mut self) -> &mut Context {
        let context = self
            .current
            .context
            .unwrap_or_else(|| panic!("no context selected"));
        self.context_mut(context)
    }

    pub(crate) fn context_active(&self) -> bool {
        self.current.status & STATUS_CONTEXT != 0
    }

    /// C: `CreateContextAux`.
    pub(crate) fn create_context(&mut self, name: &'static CStr) -> ContextId {
        self.contexts.next += 1;
        let id = NonZeroU64::new(self.contexts.next).unwrap_or_else(|| unreachable!());
        self.contexts.entries.insert(
            id,
            Context {
                flags: 0,
                fg_color: FORE,
                bg_color: BACK,
                mode: MODE,
                fg_frame: FrameId::NULL,
                font: None,
                clip: Rect::default(),
                font_effect: FrameId::NULL,
                font_backing: None,
                backing_flags: 0,
                name,
                held_target: FrameId::NULL,
                held_font: None,
                held_effect: FrameId::NULL,
                leases: 1,
                owned: true,
            },
        );
        self.contexts.order.push(ContextId(id));
        ContextId(id)
    }

    /// C: `SetContext`: select `context`; returns the previous selection.
    pub(crate) fn set_context(&mut self, context: Option<ContextId>) -> Option<ContextId> {
        let last = self.current.context;
        if context != last {
            if let Some(last) = last {
                let status = self.current.status;
                let last = self.context_mut(last);
                last.flags &= !(u16::from(SAVED_STATUS) << 8);
                last.flags |= u16::from(status & SAVED_STATUS) << 8;
                self.current.status &= !STATUS_CONTEXT;
            }
            self.current.context = context;
            if let Some(context) = context {
                let selected = self.context(context);
                let (flags, color, frame, font) = (
                    selected.flags,
                    selected.fg_color,
                    selected.fg_frame,
                    selected.font,
                );
                let status = &mut self.current.status;
                *status |= STATUS_CONTEXT;
                *status &= !SAVED_STATUS;
                *status |= (flags >> 8) as u8;
                self.current.color = color;
                self.current.frame = frame;
                self.current.font = font;
            }
        }
        last
    }

    /// C: `SetContextFGFrame`.
    pub(crate) fn set_context_fg_frame(&mut self, frame: FrameId) -> FrameId {
        let last = self.current.frame;
        if frame != last {
            if !last.is_null() {
                self.current.status &= !STATUS_FRAME;
            }
            self.current.frame = frame;
            if !frame.is_null() {
                self.current.status |= STATUS_FRAME;
            }
            if self.context_active() {
                self.current_context_mut().fg_frame = frame;
            }
        }
        last
    }

    /// C: `SetContextFont`.
    pub(crate) fn set_context_font(&mut self, font: Option<FontId>) -> Option<FontId> {
        let last = std::mem::replace(&mut self.current.font, font);
        if self.context_active() {
            let context = self.current_context_mut();
            context.font = font;
            context.backing_flags |= BACKING_DIRTY;
        }
        last
    }

    /// C: `SetContextForeGroundColor`.
    pub(crate) fn set_foreground(&mut self, color: Color) -> Color {
        if !self.context_active() {
            return FORE;
        }
        let context = self.current_context_mut();
        let old = context.fg_color;
        if old != color {
            context.fg_color = color;
            if context.backing_flags & BACKING_IMAGE == 0 {
                context.backing_flags |= BACKING_DIRTY;
            }
        }
        self.current.color = color;
        old
    }

    /// C: `SetContextBackGroundColor`.
    pub(crate) fn set_background(&mut self, color: Color) -> Color {
        if !self.context_active() {
            return BACK;
        }
        std::mem::replace(&mut self.current_context_mut().bg_color, color)
    }

    /// C: `SetContextDrawMode`.
    pub(crate) fn set_draw_mode(&mut self, mode: DrawMode) -> DrawMode {
        if !self.context_active() {
            return MODE;
        }
        std::mem::replace(&mut self.current_context_mut().mode, mode)
    }

    /// C: `GetContextDrawMode`.
    #[cfg(all(feature = "test-support", not(test)))]
    pub(crate) fn draw_mode(&self) -> DrawMode {
        if self.context_active() {
            self.current_context().mode
        } else {
            MODE
        }
    }

    pub(super) fn foreground_rect(&self) -> Rect {
        Rect {
            corner: Point::default(),
            extent: if self.current.frame.is_null() {
                Extent::zero()
            } else {
                self.frame(self.current.frame).bounds
            },
        }
    }

    /// C: `SetContextClipRect`; `None` clears the clip. A clip equal to the
    /// frame's whole area is no clip.
    pub(crate) fn set_clip(&mut self, rect: Option<Rect>) -> bool {
        if !self.context_active() {
            return false;
        }
        let foreground = self.foreground_rect();
        let context = self.current_context_mut();
        match rect {
            Some(rect) if rect != foreground => context.clip = rect,
            _ => context.clip.extent.width = 0,
        }
        true
    }

    /// C: `GetContextClipRect`: the clip (or the whole frame) and whether
    /// an explicit clip is set.
    pub(crate) fn clip(&self) -> Option<(Rect, bool)> {
        if !self.context_active() {
            return None;
        }
        let clip = self.current_context().clip;
        Some(if clip.extent.width != 0 {
            (clip, true)
        } else {
            (self.foreground_rect(), false)
        })
    }

    /// C: `SetContextFontEffect`.
    pub(crate) fn set_font_effect(&mut self, effect: FrameId) -> FrameId {
        if !self.context_active() {
            return FrameId::NULL;
        }
        let context = self.current_context_mut();
        let old = context.font_effect;
        if effect != old {
            context.font_effect = effect;
            context.backing_flags |= BACKING_DIRTY;
            if effect.is_null() {
                context.backing_flags &= !BACKING_IMAGE;
            } else {
                context.backing_flags |= BACKING_IMAGE;
            }
        }
        old
    }

    /// C: `FixContextFontEffect`: rebuild the font backing image when the
    /// colour, effect or font changed.
    pub(crate) fn fix_font_effect(&mut self) {
        if !self.context_active() {
            return;
        }
        let context = self.current_context();
        if context.font_backing.is_some() && context.backing_flags & BACKING_DIRTY == 0 {
            return;
        }
        let Some(font) = self.current.font else {
            return;
        };
        let (width, height) = {
            let font = self.font(font);
            (font.leading_width as i16, font.leading as i16)
        };
        let (old, flags, effect, color) = {
            let context = self.current_context();
            (
                context.font_backing,
                context.backing_flags,
                context.font_effect,
                context.fg_color,
            )
        };
        if let Some(old) = old {
            self.enqueue(DrawCommand::DeleteImage(old));
        }
        let backing =
            self.images
                .insert(Image::for_screen(i32::from(width), i32::from(height), true));
        if flags & BACKING_IMAGE != 0 {
            let hot = self.frame(effect).hot_spot;
            let source = self
                .frame(effect)
                .image
                .unwrap_or_else(|| panic!("font effect frame has no image"));
            image::draw_into(&mut self.images, backing, |images, target| {
                image::draw(
                    images,
                    source,
                    image::Placement::at(-i32::from(hot.x), -i32::from(hot.y)),
                    None,
                    DrawMode::REPLACE,
                    target,
                );
            });
        } else {
            image::draw_into(&mut self.images, backing, |_, target| {
                image::draw_rect(
                    target,
                    Rect::new(0, 0, width, height),
                    color,
                    DrawMode::REPLACE,
                );
            });
        }
        let context = self.current_context_mut();
        context.font_backing = Some(backing);
        context.backing_flags &= !BACKING_DIRTY;
    }

    /// C: `CopyContextRect`: copy an area of the selected context's frame
    /// into a new drawable owned by the caller.
    pub(crate) fn copy_context_rect(&mut self, area: Option<Rect>) -> Option<DrawableId> {
        if !self.context_active() || self.current.frame.is_null() {
            return None;
        }
        let foreground = self.foreground_rect();
        let (mut rect, _) = self.clip()?;
        if let Some(area) = area {
            rect.corner.x = rect.corner.x.wrapping_add(area.corner.x);
            rect.corner.y = rect.corner.y.wrapping_add(area.corner.y);
            rect.extent = area.extent;
        }
        let mut clipped = rect;
        if crate::graphics::bbox::box_intersect(rect, foreground, &mut clipped) == 0 {
            return None;
        }
        let frame = self.current.frame;
        Some(if self.frame(frame).kind == FrameKind::Screen {
            self.load_display_pixmap(clipped)
        } else {
            self.copy_frame_rect(frame, clipped)
        })
    }

    /// C: `LoadDisplayPixmap` into a new drawable: queue a copy of a screen
    /// area into its image.
    pub(super) fn load_display_pixmap(&mut self, area: Rect) -> DrawableId {
        let drawable = self.create_drawable(2 | 4, area.extent.width, area.extent.height, 1);
        let frame = FrameId::new(drawable, 0);
        let image = self
            .frame(frame)
            .image
            .unwrap_or_else(|| unreachable!("a RAM pixmap has an image"));
        self.enqueue(DrawCommand::CopyToImage {
            image,
            rect: area,
            source: Screen::Main,
        });
        drawable
    }

    /// Hold an owned context for a new lease.
    pub(super) fn retain_context(&mut self, context: ContextId) {
        let entry = self
            .contexts
            .entries
            .get_mut(&context.0)
            .filter(|entry| entry.owned)
            .unwrap_or_else(|| panic!("context is unowned or retired: {context:?}"));
        entry.leases += 1;
    }

    /// Let go of a lease; the last one frees the context.
    pub(super) fn release_context(&mut self, context: ContextId) {
        let entry = self.context_mut(context);
        entry.leases -= 1;
        if entry.leases != 0 {
            return;
        }
        if self.current.context == Some(context) {
            self.set_context(None);
        }
        let entry = self
            .contexts
            .entries
            .remove(&context.0)
            .unwrap_or_else(|| unreachable!());
        self.contexts.order.retain(|&live| live != context);
        if let Some(backing) = entry.font_backing {
            self.enqueue(DrawCommand::DeleteImage(backing));
        }
        self.release_held_frame(entry.held_target);
        self.release_held_font(entry.held_font);
        self.release_held_frame(entry.held_effect);
    }

    /// Hold `frame` as what the lease API keeps for the context's target;
    /// returns the frame held before, for [`Self::release_frame`].
    pub(super) fn hold_target(&mut self, context: ContextId, frame: FrameId) -> FrameId {
        self.retain_frame(frame);
        std::mem::replace(&mut self.context_mut(context).held_target, frame)
    }

    pub(super) fn hold_font(&mut self, context: ContextId, font: Option<FontId>) -> Option<FontId> {
        if let Some(font) = font {
            self.retain_font(font);
        }
        std::mem::replace(&mut self.context_mut(context).held_font, font)
    }

    pub(super) fn hold_effect(&mut self, context: ContextId, frame: FrameId) -> FrameId {
        self.retain_frame(frame);
        std::mem::replace(&mut self.context_mut(context).held_effect, frame)
    }

    pub(super) fn release_held_frame(&mut self, frame: FrameId) {
        let released = self.release_frame(frame);
        self.enqueue_all(released);
    }

    pub(super) fn release_held_font(&mut self, font: Option<FontId>) {
        if let Some(font) = font {
            self.release_font(font);
        }
    }
}
