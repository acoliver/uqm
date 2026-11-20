//! Graphics contexts, ported from context.c (GPL-2.0-or-later).
//!
//! A context holds the drawing state (colours, draw mode, target frame,
//! font, clip). Drawing goes through the selected context, as in C; the
//! [`ContextLease`] API selects its context for each operation and leaves it
//! selected. A context lives while its owner ([`LoadedContext`]) or a lease
//! holds it.
use super::batch::NativeBatch;
use super::dcq::{DrawCommand, Screen};
use super::drawable::{ContextFrame, DrawableId, FrameId, FrameKind};
use super::font::{FontId, FontLease};
use super::geometry::{Color, DrawMode, Extent, Line, Point, Rect, TextAlign};
use super::image::{self, Image, ImageId};
use super::state::{self, Graphics, STATUS_ACTIVE, STATUS_CONTEXT, STATUS_FRAME};
use std::ffi::CStr;
use std::num::NonZeroU64;

const FORE: Color = Color::rgba(255, 255, 255, 255);
const BACK: Color = Color::rgba(0, 0, 0, 255);
const MODE: DrawMode = DrawMode::new(0, 255);

/// C: `FBK_DIRTY` and `FBK_IMAGE`.
const BACKING_DIRTY: u16 = 1;
const BACKING_IMAGE: u16 = 2;
/// The status bits a context saves while deselected.
const SAVED_STATUS: u8 = STATUS_ACTIVE | STATUS_FRAME;

/// C: `CONTEXT_DESC`.
#[derive(Debug)]
pub struct Context {
    flags: u16,
    pub(crate) fg_color: Color,
    pub(crate) bg_color: Color,
    pub(crate) mode: DrawMode,
    pub(crate) fg_frame: FrameId,
    pub(crate) font: Option<FontId>,
    pub(crate) clip: Rect,
    font_effect: FrameId,
    pub(crate) font_backing: Option<ImageId>,
    backing_flags: u16,
    name: &'static CStr,
    // What the lease API holds for this context (its frame, font and font
    // effect), released when the context goes.
    held_target: FrameId,
    held_font: Option<FontId>,
    held_effect: FrameId,
    leases: u32,
}

/// A copyable name for a context (C: `CONTEXT`). Names are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContextId(NonZeroU64);

impl ContextId {
    pub fn bits(self) -> usize {
        self.0.get() as usize
    }
}

/// The context registry.
#[derive(Debug, Default)]
pub(crate) struct Contexts {
    entries: std::collections::HashMap<NonZeroU64, Context>,
    /// Live contexts in creation order (C: the debug context list).
    order: Vec<ContextId>,
    next: u64,
}

#[path = "context_store.rs"]
mod store;

/// A held context. Every operation selects it first, as C code did with
/// `SetContext`, and leaves it selected.
#[derive(Debug, PartialEq, Eq)]
pub struct ContextLease(ContextId);

impl Clone for ContextLease {
    fn clone(&self) -> Self {
        state::with(|g| g.context_mut(self.0).leases += 1);
        Self(self.0)
    }
}

impl Drop for ContextLease {
    fn drop(&mut self) {
        state::with(|g| g.release_context(self.0));
    }
}

/// Unique release authority for a context retained by drawing leases.
#[derive(Debug)]
pub struct LoadedContext {
    context: ContextLease,
}

impl LoadedContext {
    pub fn create(name: &'static CStr) -> Self {
        Self {
            context: ContextLease(state::with(|g| g.create_context(name))),
        }
    }

    pub fn lease(&self) -> ContextLease {
        self.context.clone()
    }

    pub fn context(&self) -> ContextId {
        self.context.0
    }

    /// Hand ownership to game state that names the context by id (C: a
    /// `CONTEXT` field); [`Self::from_id`] takes it back.
    pub fn into_id(self) -> ContextId {
        // the owner's release moves to whoever calls `from_id`
        std::mem::ManuallyDrop::new(self).context.0
    }

    /// Take back ownership handed out by [`Self::into_id`].
    pub fn from_id(context: ContextId) -> Self {
        Self {
            context: ContextLease(context),
        }
    }
}

impl Drop for LoadedContext {
    fn drop(&mut self) {
        let context = self.context.0;
        state::with(|g| {
            if g.current.context == Some(context) {
                g.set_context(None);
            }
        });
    }
}

/// Undoes a scoped change when dropped.
struct Restore<F: FnMut()>(F);
impl<F: FnMut()> Drop for Restore<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// Ends a batch begun by [`ContextLease::batch`].
#[derive(Debug)]
pub struct BatchGuard(());
impl Drop for BatchGuard {
    fn drop(&mut self) {
        state::unbatch();
    }
}

/// Restarts a batch paused by [`ContextLease::pause_batch`].
#[derive(Debug)]
pub struct BatchPause(());
impl Drop for BatchPause {
    fn drop(&mut self) {
        state::batch();
    }
}

impl ContextLease {
    /// Hold a live context.
    pub fn retain(context: ContextId) -> Self {
        state::with(|g| g.retain_context(context));
        Self(context)
    }

    pub fn context(&self) -> ContextId {
        self.0
    }

    /// Hold a context the game state names; panics when it is absent.
    pub fn require(context: Option<ContextId>) -> Self {
        Self::retain(context.unwrap_or_else(|| panic!("required context is absent")))
    }

    /// Select this context, then run `step`.
    fn selected<R>(&self, step: impl FnOnce(&mut Graphics) -> R) -> R {
        state::with(|g| {
            g.set_context(Some(self.0));
            step(g)
        })
    }

    pub fn select(&self) {
        self.selected(|_| ());
    }

    /// The selected context, if any.
    pub fn selected_context(&self) -> Option<Self> {
        state::with(|g| g.current.context).map(Self::retain)
    }

    pub fn fade_screen(&self, kind: u32, interval: i16) -> u32 {
        state::fade_screen(kind, interval)
    }

    pub fn finish_color_transforms(&self) {
        state::flush_color_xforms();
    }

    pub fn transition_source(&self, rect: Option<Rect>) {
        super::screen::set_transition_source(rect);
    }

    pub fn transition(&self, rect: Option<Rect>) {
        super::screen::screen_transition(rect);
    }

    pub fn scale_mode(&self, mode: i32) {
        state::set_graphic_scale_mode(mode);
    }

    pub fn highlight(&self, rect: Rect) -> super::highlight::Highlight {
        super::highlight::Highlight::new(self.clone(), rect)
    }

    /// Select this context and hold its target frame.
    pub fn target(&self) -> ContextFrame {
        ContextFrame::retain(self.selected(|g| g.context(self.0).fg_frame))
    }

    pub fn with_selected<R>(&self, draw: impl FnOnce() -> R) -> R {
        let previous = state::with(|g| g.set_context(Some(self.0))).map(Self::retain);
        let _restore = Restore(move || {
            let previous = previous.as_ref().map(ContextLease::context);
            state::with(|g| g.set_context(previous));
        });
        draw()
    }

    pub fn batch(&self) -> BatchGuard {
        state::batch();
        BatchGuard(())
    }

    pub fn with_batch<R>(&self, draw: impl FnOnce() -> R) -> R {
        let _batch = self.batch();
        draw()
    }

    /// Select this context and batch for `draw`, ending the batch before
    /// restoring the previous selection.
    pub fn draw_batch<R>(&self, draw: impl FnOnce() -> R) -> R {
        self.with_selected(|| {
            let _batch = self.batch();
            draw()
        })
    }

    /// C: `TFB_GetBatchDepth`.
    pub fn batch_depth(&self) -> i32 {
        state::batch_depth()
    }

    /// Apply exactly `levels` batch corrections: positive ends batches,
    /// negative begins them.
    pub fn correct_batch_by(&self, levels: i32) {
        state::correct_batch_by(levels);
    }

    /// Replay the queue without presenting, then present unconditionally
    /// (`TFB_FlushGraphicsEx(1)` then `TFB_SwapBuffers(TFB_REDRAW_FADING)`).
    pub fn present_retained(&self) {
        state::flush_ex(true);
        super::present::swap_buffers(super::present::Redraw::Fading);
    }

    pub fn pause_batch(&self) -> BatchPause {
        state::unbatch();
        BatchPause(())
    }

    pub fn with_text_state<R>(&self, draw: impl FnOnce() -> R) -> R {
        self.with_text_mode(None, draw)
    }

    fn with_text_mode<R>(&self, mode: Option<DrawMode>, draw: impl FnOnce() -> R) -> R {
        self.with_selected(|| {
            let context = self.0;
            let (font, color, previous_mode) = state::with(|g| {
                let selected = g.context(context);
                let saved = (selected.font, selected.fg_color, selected.mode);
                if let Some(font) = saved.0 {
                    g.retain_font(font);
                }
                if let Some(mode) = mode {
                    g.set_draw_mode(mode);
                }
                saved
            });
            let _restore = Restore(move || {
                state::with(|g| {
                    g.set_context(Some(context));
                    let previous = g.hold_font(context, font);
                    g.set_context_font(font);
                    g.release_held_font(previous);
                    g.release_held_font(font);
                    g.set_foreground(color);
                    if mode.is_some() {
                        g.set_draw_mode(previous_mode);
                    }
                });
            });
            draw()
        })
    }

    pub fn set_background(&self, color: Color) {
        self.selected(|g| g.set_background(color));
    }

    pub fn background(&self, color: Color) -> Color {
        self.selected(|g| g.set_background(color))
    }

    pub fn foreground(&self) -> Color {
        state::with(|g| g.context(self.0).fg_color)
    }

    pub fn set_foreground(&self, color: Color) {
        self.selected(|g| g.set_foreground(color));
    }

    pub fn draw_mode(&self) -> DrawMode {
        state::with(|g| g.context(self.0).mode)
    }

    pub fn set_draw_mode(&self, mode: DrawMode) -> DrawMode {
        self.selected(|g| g.set_draw_mode(mode))
    }

    /// Make `target` this context's frame and hold it.
    pub fn set_frame(&self, target: ContextFrame) {
        let context = self.0;
        self.selected(|g| {
            g.set_context_fg_frame(target.frame());
            let previous = g.hold_target(context, target.frame());
            g.release_held_frame(previous);
        });
    }

    pub fn set_clip(&self, rect: Rect) {
        self.selected(|g| g.set_clip(Some(rect)));
    }

    fn clip_snapshot(&self) -> (Rect, bool) {
        self.selected(|g| {
            g.clip()
                .unwrap_or_else(|| unreachable!("context is selected"))
        })
    }

    pub fn clip_rect(&self) -> Rect {
        self.clip_snapshot().0
    }

    pub fn clear_clip(&self) {
        self.selected(|g| g.set_clip(None));
    }

    fn restore_clip(&self, previous: (Rect, bool)) -> impl Drop + '_ {
        Restore(move || {
            let rect = previous.1.then_some(previous.0);
            self.selected(|g| g.set_clip(rect));
        })
    }

    pub fn with_clip<R>(&self, clip: Rect, draw: impl FnOnce() -> R) -> R {
        let _restore = self.restore_clip(self.clip_snapshot());
        self.set_clip(clip);
        draw()
    }

    pub fn with_unclipped<R>(&self, draw: impl FnOnce() -> R) -> R {
        let _restore = self.restore_clip(self.clip_snapshot());
        self.clear_clip();
        draw()
    }

    pub fn with_origin<R>(&self, origin: Point, draw: impl FnOnce() -> R) -> R {
        let frame = self.target();
        let previous = frame.replace_hotspot(origin);
        let _restore = Restore(move || {
            frame.replace_hotspot(previous);
        });
        draw()
    }

    pub fn with_frame<R>(&self, target: ContextFrame, draw: impl FnOnce() -> R) -> R {
        self.with_selected(|| {
            let previous = self.target();
            let _restore = Restore(move || self.set_frame(previous.clone()));
            self.set_frame(target);
            draw()
        })
    }

    pub fn with_draw_mode<R>(&self, mode: DrawMode, draw: impl FnOnce() -> R) -> R {
        let previous = self.set_draw_mode(mode);
        let _restore = Restore(move || {
            self.set_draw_mode(previous);
        });
        draw()
    }

    pub fn with_scale<R>(&self, mode: i32, scale: i32, draw: impl FnOnce() -> R) -> R {
        self.select();
        let previous = (
            state::set_graphic_scale_mode(mode),
            state::set_graphic_scale(scale),
        );
        let _restore = Restore(move || {
            self.select();
            state::set_graphic_scale_mode(previous.0);
            state::set_graphic_scale(previous.1);
        });
        draw()
    }

    pub fn system_rect(&self, rect: Option<Rect>) {
        super::present::set_system_rect(rect);
    }

    pub fn screen_dimensions(&self) -> (i32, i32) {
        state::screen_dimensions()
    }

    pub fn save_screen_region(&self, rect: Rect) {
        self.selected(|g| g.enqueue_copy(Some(rect), Screen::Main, Screen::Extra));
    }

    pub fn restore_screen_region(&self, rect: Rect) {
        self.selected(|g| g.enqueue_copy(Some(rect), Screen::Extra, Screen::Main));
    }

    pub fn draw_line(&self, color: Color, first: Point, second: Point) {
        self.selected(|g| {
            g.set_foreground(color);
            g.draw_line(Line { first, second });
        });
    }

    /// Whether any of the line is inside `clip`.
    pub fn line_visible(&self, clip: Rect, first: Point, second: Point) -> bool {
        let mut line = super::bbox::BresenhamLine {
            first,
            second,
            abs_delta_x: 0,
            abs_delta_y: 0,
            error_term: 0,
            end_points_exchanged: false,
            intersect_code: 0,
        };
        super::bbox::clip_line(clip, &mut line) != 0
    }

    pub fn draw_point(&self, color: Color, point: Point) {
        self.selected(|g| {
            g.set_foreground(color);
            g.draw_point(point);
        });
    }

    pub fn draw_rectangle(&self, color: Color, rect: Rect) {
        self.selected(|g| {
            g.set_foreground(color);
            g.draw_rectangle(rect);
        });
    }

    pub fn fill_rectangle(&self, color: Color, rect: Rect) {
        self.selected(|g| {
            g.set_foreground(color);
            g.draw_filled_rectangle(rect);
        });
    }

    pub fn draw_stamp(&self, source: &ContextFrame, origin: Point) {
        self.selected(|g| g.draw_stamp(source.frame(), origin));
    }

    pub fn draw_scaled_stamp(&self, source: &ContextFrame, origin: Point, scale: i32, mode: i32) {
        assert!((1..=3).contains(&mode), "graphic scale mode {mode}");
        self.selected(|g| {
            let previous_mode = std::mem::replace(&mut g.scale_mode, mode);
            let previous_scale = std::mem::replace(
                &mut g.scale,
                if scale == 0 {
                    state::GSCALE_IDENTITY
                } else {
                    scale
                },
            );
            g.draw_stamp(source.frame(), origin);
            g.scale = previous_scale;
            g.scale_mode = previous_mode;
        });
    }

    pub fn draw_filled_stamp(&self, source: &ContextFrame, origin: Point, color: Color) {
        self.selected(|g| {
            g.set_foreground(color);
            g.draw_filled_stamp(source.frame(), origin);
        });
    }

    pub fn draw_filled_stamp_preserving_color(
        &self,
        source: &ContextFrame,
        origin: Point,
        color: Color,
    ) {
        self.selected(|g| {
            let previous = g.set_foreground(color);
            g.draw_filled_stamp(source.frame(), origin);
            g.set_foreground(previous);
        });
    }

    pub fn draw_shadowed_box(&self, rect: Rect, background: Color, dark: Color, medium: Color) {
        self.selected(|g| g.draw_shadowed_box(rect, background, dark, medium));
    }

    pub fn clear(&self) {
        self.selected(Graphics::clear_drawable);
    }

    pub fn flush(&self) {
        self.select();
        state::wait_for_signal();
    }

    /// Execute committed drawing without presenting a frame.
    pub fn drain(&self) {
        self.with_batch(|| self.flush());
    }

    pub fn replace_font(&self, font: Option<FontLease>) {
        let context = self.0;
        let font = font.as_ref().map(FontLease::font);
        self.selected(|g| {
            let previous = g.hold_font(context, font);
            g.set_context_font(font);
            g.release_held_font(previous);
        });
    }

    pub fn set_font(&self, font: &FontLease) {
        self.replace_font(Some(font.clone()));
    }

    pub fn font_lease(&self) -> Option<FontLease> {
        state::with(|g| g.context(self.0).font).map(FontLease::held)
    }

    pub fn font_effect(&self) -> ContextFrame {
        ContextFrame::retain(state::with(|g| g.context(self.0).font_effect))
    }

    pub fn set_font_effect(&self, effect: ContextFrame) {
        let context = self.0;
        self.selected(|g| {
            let previous = g.hold_effect(context, effect.frame());
            g.set_font_effect(effect.frame());
            g.release_held_frame(previous);
        });
    }

    pub fn with_font_effect<R>(
        &self,
        effect: Option<&ContextFrame>,
        draw: impl FnOnce() -> R,
    ) -> R {
        self.with_selected(|| {
            let previous = self.font_effect();
            self.set_font_effect(effect.cloned().unwrap_or_default());
            let _restore = Restore(move || self.set_font_effect(previous.clone()));
            draw()
        })
    }

    pub fn draw_text(
        &self,
        font: &FontLease,
        string: &CStr,
        baseline: Point,
        align: TextAlign,
        color: Color,
    ) {
        self.draw_text_bounded(font, string, baseline, align, color, u16::MAX);
    }

    pub fn draw_text_bounded(
        &self,
        font: &FontLease,
        string: &CStr,
        baseline: Point,
        align: TextAlign,
        color: Color,
        characters: u16,
    ) {
        let context = self.0;
        self.selected(|g| {
            let previous = g.hold_font(context, Some(font.font()));
            g.release_held_font(previous);
            g.set_context_font(Some(font.font()));
            g.set_foreground(color);
            g.draw_text(super::font::Text {
                baseline,
                bytes: string.to_bytes(),
                align,
                char_count: characters,
            });
        });
    }

    pub fn draw_traced_text(
        &self,
        font: &FontLease,
        string: &CStr,
        baseline: Point,
        align: TextAlign,
        color: Color,
        trace: Color,
    ) {
        let context = self.0;
        self.selected(|g| {
            let previous = g.hold_font(context, Some(font.font()));
            g.release_held_font(previous);
            g.set_context_font(Some(font.font()));
            g.draw_traced_text(
                super::font::Text::new(baseline, string.to_bytes(), align),
                color,
                trace,
            );
        });
    }

    pub fn draw_text_alpha(
        &self,
        font: &FontLease,
        string: &CStr,
        baseline: Point,
        align: TextAlign,
        color: Color,
    ) {
        self.with_text_mode(
            Some(DrawMode::new(
                super::canvas::raster::DRAW_ALPHA,
                i16::from(color.a),
            )),
            || self.draw_text(font, string, baseline, align, color),
        );
    }

    pub fn draw_primitives(&self, batch: &mut NativeBatch, scale: i32) {
        self.submit_batch(batch, Some(scale));
    }

    pub fn draw_preserving_scale(&self, batch: &mut NativeBatch) {
        self.submit_batch(batch, None);
    }

    fn submit_batch(&self, batch: &NativeBatch, scale: Option<i32>) {
        self.selected(|g| {
            if let Some(scale) = scale {
                g.scale = if scale == 0 {
                    state::GSCALE_IDENTITY
                } else {
                    scale
                };
            }
            g.draw_batch(batch.drawn(), false);
            if scale.is_some() {
                g.scale = state::GSCALE_IDENTITY;
            }
        });
    }

    /// Copy an area relative to the clip into a new drawable (C:
    /// `CopyContextRect`); the previous selection is restored.
    pub(crate) fn copy_rect(&self, area: Option<Rect>) -> Option<DrawableId> {
        self.with_selected(|| state::with(|g| g.copy_context_rect(area)))
    }

    #[cfg(feature = "debug-process")]
    pub fn diagnostic_contexts(&self) -> Vec<Self> {
        state::with(|g| g.contexts.order.clone())
            .into_iter()
            .map(Self::retain)
            .collect()
    }

    #[cfg(feature = "debug-process")]
    pub fn diagnostic_name(&self) -> std::ffi::CString {
        state::with(|g| g.context(self.0).name.to_owned())
    }
}

/// The selected context, if any (C: `_pCurContext`).
pub fn selected() -> Option<ContextId> {
    state::with(|g| g.current.context)
}

/// C: `SetContext`: select `context`, returning the previous selection.
pub fn select(context: Option<ContextId>) -> Option<ContextId> {
    state::with(|g| g.set_context(context))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_owners_release_once_and_leases_keep_each_context_alive() {
        let first = LoadedContext::create(c"first-owned-context");
        let second = LoadedContext::create(c"second-owned-context");
        let first_lease = first.lease();
        let second_lease = second.lease();
        let first_ref = first.context();
        first_lease.select();
        second_lease.select();
        drop(first);
        drop(ContextLease::retain(first_ref));
        drop(first_lease);
        assert!(std::panic::catch_unwind(|| ContextLease::retain(first_ref)).is_err());
        assert_eq!(selected(), Some(second.context()));
        drop(second);
        assert_eq!(selected(), None);
        drop(second_lease);
    }

    /// #961: a lease outliving its context's owner keeps the context
    /// selectable, by the lease and by the id the selection reports.
    #[test]
    fn a_retained_context_stays_selectable_after_its_owner_releases_it() {
        let owner = LoadedContext::create(c"retired-owner");
        let lease = owner.lease();
        let id = owner.context();
        drop(owner);
        lease.select();
        let reselected = lease
            .selected_context()
            .unwrap_or_else(|| panic!("the retained context is selected"));
        assert_eq!(reselected.context(), id);
        select(None);
        assert_eq!(select(Some(id)), None);
        assert_eq!(selected(), Some(id));
        drop(reselected);
        drop(lease);
        assert_eq!(selected(), None);
        assert!(std::panic::catch_unwind(|| ContextLease::retain(id)).is_err());
    }

    #[test]
    fn deselected_contexts_keep_their_status_and_colour() {
        let owner = LoadedContext::create(c"status");
        let context = owner.lease();
        context.set_foreground(Color::rgb(1, 2, 3));
        let frame = LoadedDrawable::create(0, 4, 4, 1).unwrap();
        context.set_frame(frame.lease().context_frame(0));
        assert_eq!(
            state::with(|g| g.current.status),
            STATUS_CONTEXT | STATUS_FRAME
        );
        select(None);
        assert_eq!(state::with(|g| g.current.status), STATUS_FRAME);
        context.select();
        assert_eq!(
            state::with(|g| (g.current.color, g.current.status)),
            (Color::rgb(1, 2, 3), STATUS_CONTEXT | STATUS_FRAME)
        );
    }

    #[test]
    fn a_clip_equal_to_the_whole_frame_is_no_clip() {
        let owner = LoadedContext::create(c"clip");
        let context = owner.lease();
        let frame = LoadedDrawable::create(0, 8, 6, 1).unwrap();
        context.set_frame(frame.lease().context_frame(0));
        context.set_clip(Rect::new(0, 0, 8, 6));
        assert_eq!(context.clip_snapshot(), (Rect::new(0, 0, 8, 6), false));
        context.with_clip(Rect::new(1, 1, 2, 2), || {
            assert_eq!(context.clip_snapshot(), (Rect::new(1, 1, 2, 2), true));
        });
        assert_eq!(context.clip_snapshot(), (Rect::new(0, 0, 8, 6), false));
    }

    use super::super::drawable::LoadedDrawable;
}
