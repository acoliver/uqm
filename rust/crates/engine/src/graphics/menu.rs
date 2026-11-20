//! Drawing through the selected context, for menus written in the C style
//! of "select a context, then draw".
use super::context::{self, ContextId, ContextLease};
use super::drawable::{ContextFrame, FrameId};
use super::font::{FontId, FontLease};
use super::geometry::{Color, Point, Rect, TextAlign};
use super::state;
use std::ffi::CStr;

/// The selected context; panics when none is selected.
fn current() -> ContextLease {
    ContextLease::retain(context::selected().unwrap_or_else(|| panic!("no context selected")))
}

/// C: `SetContext`: select `context`, returning the previous selection.
pub fn select(context: Option<ContextId>) -> Option<ContextId> {
    context::select(context)
}

/// The selected context, if any.
pub fn selected() -> Option<ContextId> {
    context::selected()
}

pub fn clip() -> Rect {
    current().clip_rect()
}

/// C: `SetContextClipRect`, where `None` clears the clip.
pub fn set_clip_or_clear(rect: Option<Rect>) {
    state::with(|g| g.set_clip(rect));
}

/// C: `SetContextDrawMode`: returns the previous mode.
pub fn draw_mode(mode: super::geometry::DrawMode) -> super::geometry::DrawMode {
    state::with(|g| g.set_draw_mode(mode))
}

/// C: `SetContextFGFrame` returning the previous frame.
pub fn set_frame_returning(frame: FrameId) -> FrameId {
    let previous = current().target().frame();
    set_frame(frame);
    previous
}

pub fn set_clip(rect: Rect) {
    current().set_clip(rect);
}

/// C: `SetContextFGFrame` on the selected context.
pub fn set_frame(frame: FrameId) {
    current().set_frame(ContextFrame::retain(frame));
}

/// C: `SetContextOrigin` on the selected context; returns the previous one.
pub fn set_origin(origin: Point) -> Point {
    current().target().replace_hotspot(origin)
}

/// C: `SetContextForeGroundColor`; returns the previous colour.
pub fn foreground(color: Color) -> Color {
    let context = current();
    let previous = context.foreground();
    context.set_foreground(color);
    previous
}

/// C: `SetContextBackGroundColor`; returns the previous colour.
pub fn background(color: Color) -> Color {
    current().background(color)
}

pub fn selected_font() -> Option<FontLease> {
    current().font_lease()
}

/// C: `SetContextFont`; returns the previous font.
pub fn select_font(font: Option<FontLease>) -> Option<FontId> {
    let context = current();
    let previous = context.font_lease().map(|font| font.font());
    context.replace_font(font);
    previous
}

pub fn font(font: Option<FontId>) -> Option<FontId> {
    select_font(font.map(FontLease::retain))
}

pub fn selected_effect() -> ContextFrame {
    current().font_effect()
}

/// C: `SetContextFontEffect`; returns the previous effect frame.
pub fn select_effect(effect: ContextFrame) -> FrameId {
    let context = current();
    let previous = context.font_effect();
    context.set_font_effect(effect);
    previous.frame()
}

pub fn font_effect(frame: FrameId) -> FrameId {
    select_effect(ContextFrame::retain(frame))
}

pub fn font_leading() -> i16 {
    selected_font()
        .unwrap_or_else(|| panic!("selected menu context has no font"))
        .leading()
}

pub fn fill(rect: Rect) {
    let context = current();
    context.fill_rectangle(context.foreground(), rect);
}

pub fn outline(rect: Rect) {
    let context = current();
    context.draw_rectangle(context.foreground(), rect);
}

pub fn point(point: Point) {
    let context = current();
    context.draw_point(context.foreground(), point);
}

/// C: `DrawStamp` through the selected context; nothing is drawn when no
/// context or frame is selected.
pub fn stamp(frame: FrameId, origin: Point) {
    state::with(|g| g.draw_stamp(frame, origin));
}

pub fn filled_stamp(frame: FrameId, origin: Point) {
    let context = current();
    context.draw_filled_stamp(&ContextFrame::retain(frame), origin, context.foreground());
}

pub fn text(text: &CStr, baseline: Point, align: TextAlign, count: u16) {
    let context = current();
    context.draw_text_bounded(
        &selected_font().unwrap_or_else(|| panic!("selected menu context has no font")),
        text,
        baseline,
        align,
        context.foreground(),
        count,
    );
}

/// C: `SetAbsFrameIndex`.
pub fn frame(frame: FrameId, index: u16) -> FrameId {
    frame.at(index)
}

/// C: `SetRelFrameIndex`.
pub fn relative_frame(frame: FrameId, offset: i16) -> FrameId {
    frame.relative(offset)
}

/// C: `GetFrameCount`.
pub fn frame_count(frame: FrameId) -> u16 {
    frame.count()
}

/// C: `GetFrameIndex`.
pub fn frame_index(frame: FrameId) -> u16 {
    frame.index()
}

pub fn batch() {
    current();
    state::batch();
}

pub fn unbatch() {
    current();
    state::unbatch();
}

pub fn clear() {
    current().clear();
}

pub fn transition_source() {
    current().transition_source(None);
}

pub fn transition() {
    current();
    super::screen::screen_transition(None);
}

#[repr(u32)]
pub enum ScreenFade {
    Black = 252,
    Color = 253,
}

pub fn fade_screen(kind: ScreenFade, interval: i16) -> u32 {
    current().fade_screen(kind as u32, interval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::context::LoadedContext;
    use crate::graphics::font::{Font, LoadedFont};

    #[test]
    fn selected_font_survives_owner_release_and_null_selection_releases_its_lease() {
        let owner = LoadedContext::create(c"menu-font-retention");
        let context = owner.lease();
        let mut font = Font::default();
        font.leading = 17;
        let loaded = LoadedFont::adopt(font);
        let raw = loaded.font();
        context.set_font(&loaded.lease());
        context.select();
        drop(loaded);
        assert_eq!(font_leading(), 17);
        assert_eq!(super::font(None), Some(raw));
        assert!(std::panic::catch_unwind(|| FontLease::retain(raw)).is_err());
    }
}
