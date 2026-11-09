//! Serialized drawing operations for menus with registered context and frame owners.

use super::{
    context_abi, context_abi::ContextFrame, context_abi::ContextLease, native_abi::*,
    native_font::FontLease,
};
use std::{ffi::CStr, ptr::NonNull};

fn current() -> ContextLease {
    super::ffi::native_allocations().assert_thread();
    // SAFETY: allocation admission enforces the serialized graphics thread.
    let context = unsafe { context_abi::_pCurContext };
    ContextLease::require(context)
}

pub fn select(context: CONTEXT) -> CONTEXT {
    super::ffi::native_allocations().assert_thread();
    // SAFETY: allocation admission enforces serialized context selection.
    let previous = unsafe { context_abi::_pCurContext };
    let _previous = NonNull::new(previous).map(ContextLease::retain);
    let next = NonNull::new(context).map(ContextLease::retain);
    // SAFETY: both optional contexts are registered and retained for the selection.
    unsafe { context_abi::SetContext(next.as_ref().map_or(std::ptr::null_mut(), |_| context)) };
    previous
}

pub fn clip() -> RECT {
    current().clip_rect()
}
pub fn set_clip(rect: RECT) {
    current().set_clip(rect);
}
pub fn foreground(color: Color) -> Color {
    let context = current();
    let previous = context.foreground();
    context.set_foreground(color);
    previous
}
pub fn background(color: Color) -> Color {
    let context = current();
    // SAFETY: the registered context remains selected on its owner thread.
    let previous = unsafe { context_abi::SetContextBackGroundColor(color) };
    drop(context);
    previous
}
pub fn selected_font() -> Option<FontLease> {
    current().font_lease()
}
pub fn select_font(font: Option<FontLease>) {
    current().replace_font(font);
}
pub fn font(font: FONT) -> FONT {
    let context = current();
    let previous = context.font_lease();
    let previous_raw = previous
        .as_ref()
        .map_or(std::ptr::null_mut(), FontLease::as_raw);
    context.replace_font(NonNull::new(font).map(FontLease::retain));
    previous_raw
}
pub fn selected_effect() -> ContextFrame {
    current().font_effect()
}
pub fn select_effect(effect: ContextFrame) {
    current().set_font_effect(effect);
}
pub fn font_effect(frame: FRAME) -> FRAME {
    let context = current();
    let previous = context.font_effect();
    context.set_font_effect(ContextFrame::retain(NonNull::new(frame)));
    previous.as_raw()
}
pub fn font_leading() -> i16 {
    selected_font()
        .unwrap_or_else(|| panic!("selected menu context has no font"))
        .leading()
}
pub fn fill(rect: RECT) {
    let context = current();
    context.fill_rectangle(context.foreground(), rect);
}
pub fn outline(rect: RECT) {
    let context = current();
    context.draw_rectangle(context.foreground(), rect);
}
pub fn point(point: POINT) {
    let context = current();
    context.draw_point(context.foreground(), point);
}
pub fn stamp(frame: FRAME, origin: POINT) {
    current().draw_stamp(&ContextFrame::retain(NonNull::new(frame)), origin);
}
pub fn filled_stamp(frame: FRAME, origin: POINT) {
    let context = current();
    context.draw_filled_stamp(
        &ContextFrame::retain(NonNull::new(frame)),
        origin,
        context.foreground(),
    );
}
pub fn text(text: &CStr, baseline: POINT, align: TEXT_ALIGN, count: u16) {
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
pub fn frame(frame: FRAME, index: u16) -> FRAME {
    if frame.is_null() {
        return frame;
    }
    ContextFrame::retain(NonNull::new(frame))
        .wrapped_frame(index)
        .as_raw()
}
pub fn relative_frame(frame: FRAME, offset: i16) -> FRAME {
    let lease = ContextFrame::retain(NonNull::new(frame));
    // SAFETY: the optional frame and its complete drawable are retained during indexing.
    unsafe { super::pixmap_abi::SetRelFrameIndex(lease.as_raw(), offset) }
}
pub fn frame_count(frame: FRAME) -> u16 {
    if frame.is_null() {
        return 0;
    }
    ContextFrame::retain(NonNull::new(frame)).frame_count()
}
pub fn frame_index(frame: FRAME) -> u16 {
    let lease = ContextFrame::retain(NonNull::new(frame));
    // SAFETY: the optional frame belongs to its retained drawable allocation.
    unsafe { super::pixmap_abi::GetFrameIndex(lease.as_raw().cast()) }
}
pub fn batch() {
    let context = current();
    // SAFETY: the selected context admits this graphics-thread batch operation.
    unsafe { super::gfx_common_abi::batch_graphics() };
    drop(context);
}
pub fn unbatch() {
    let context = current();
    // SAFETY: menu callers balance batch operations on this graphics thread.
    unsafe { super::gfx_common_abi::unbatch_graphics() };
    drop(context);
}
pub fn clear() {
    current().clear();
}
pub fn transition_source() {
    current().transition_source(None);
}
pub fn transition(kind: i32) {
    let context = current();
    // SAFETY: the live selected context admits a whole-screen transition.
    unsafe { super::gfx_common_abi::ScreenTransition(kind, std::ptr::null_mut()) };
    drop(context);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    #[serial_test::serial]
    fn selected_font_survives_owner_release_and_null_selection_releases_its_lease() {
        let owner = super::super::context_abi::LoadedContext::create(c"menu-font-retention");
        let context = owner.lease();
        // SAFETY: this serial fixture registers one newly allocated empty font.
        let raw = unsafe {
            let raw = NonNull::new(
                crate::memory::rust_hcalloc(std::mem::size_of::<font_desc>()).cast::<font_desc>(),
            )
            .unwrap();
            (*raw.as_ptr()).Leading = 17;
            super::super::native_font::register(raw);
            raw
        };
        let allocation = Rc::downgrade(
            super::super::ffi::native_allocations()
                .fonts
                .get(&(raw.as_ptr() as usize))
                .unwrap(),
        );
        let lease = FontLease::retain(raw);
        context.set_font(&lease);
        context.select();
        drop(lease);
        super::super::native_font::release(raw);
        assert_eq!(font_leading(), 17);
        assert_eq!(font(std::ptr::null_mut()), raw.as_ptr());
        assert!(allocation.upgrade().is_none());
    }
}

#[repr(u32)]
pub enum ScreenFade {
    Black = 252,
    Color = 253,
}

pub fn fade_screen(kind: ScreenFade, interval: i16) -> u32 {
    current().fade_screen(kind as u32, interval)
}
