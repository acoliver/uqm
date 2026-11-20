//! A retained graphics view supplied by the running campaign.

use super::gfx::*;
use crate::engine::Kernel;
use crate::graphics::{
    context::ContextLease, drawable::ContextFrame, font::FontLease, menu as draw,
};
use crate::strings::owned_table::LoadedTextTable;
use std::{ffi::CString, rc::Rc};

pub struct Graphics {
    dimensions: (i32, i32),
    contexts: [Option<ContextLease>; 3],
    fonts: [Option<FontLease>; 3],
    off_screen: Option<ContextLease>,
    play: ContextFrame,
    gradient: ContextFrame,
    text: LoadedTextTable,
    highlight: Rc<crate::engine::kernel::SisHighlight>,
    pc_menu: bool,
    saved_fonts: std::cell::RefCell<Vec<(usize, Option<FontLease>)>>,
    saved_effects: std::cell::RefCell<Vec<(usize, ContextFrame)>>,
    saved_contexts: std::cell::RefCell<Vec<ContextLease>>,
    restored_context: std::cell::RefCell<Option<ContextLease>>,
}
impl Graphics {
    pub fn new(kernel: &Kernel, dimensions: (i32, i32)) -> Self {
        let fonts = [kernel.star_con_font, kernel.tiny_font, kernel.micro_font];
        Self {
            dimensions,
            saved_fonts: Default::default(),
            saved_effects: Default::default(),
            saved_contexts: Default::default(),
            restored_context: Default::default(),
            contexts: [
                kernel.screen_context,
                kernel.status_context,
                kernel.space_context,
            ]
            .map(|context| context.map(ContextLease::retain)),
            fonts: fonts.map(|font| font.map(FontLease::retain)),
            off_screen: kernel.off_screen_context.map(ContextLease::retain),
            play: ContextFrame::retain(kernel.play_frame),
            gradient: ContextFrame::retain(kernel.font_grad_frame),
            text: kernel.game_text_table(),
            highlight: kernel.sis_highlight.clone(),
            pc_menu: kernel.configuration.preferences().menu == 2,
        }
    }
    pub fn off_screen_context(&self) -> &ContextLease {
        self.off_screen
            .as_ref()
            .unwrap_or_else(|| panic!("off-screen graphics context is absent"))
    }
    fn context(&self, id: ContextId) -> &ContextLease {
        self.contexts[match id {
            ContextId::Screen => 0,
            ContextId::Status => 1,
            ContextId::Space => 2,
        }]
        .as_ref()
        .unwrap_or_else(|| panic!("menu context is absent"))
    }
}
fn color(c: Rgba) -> crate::graphics::geometry::Color {
    crate::graphics::geometry::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
fn rgba(c: crate::graphics::geometry::Color) -> Rgba {
    Rgba {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
fn point(p: Point) -> crate::graphics::geometry::Point {
    crate::graphics::geometry::Point { x: p.x, y: p.y }
}
fn rect(r: Rect) -> crate::graphics::geometry::Rect {
    crate::graphics::geometry::Rect {
        corner: point(r.corner),
        extent: crate::graphics::geometry::Extent {
            width: r.width,
            height: r.height,
        },
    }
}
fn rectangle(r: crate::graphics::geometry::Rect) -> Rect {
    Rect {
        corner: Point {
            x: r.corner.x,
            y: r.corner.y,
        },
        width: r.extent.width,
        height: r.extent.height,
    }
}
impl Gfx for Graphics {
    fn screen_size(&self) -> (i32, i32) {
        self.dimensions
    }
    fn set_context(&self, id: ContextId) -> ContextHandle {
        if let Some(previous) = self.context(id).selected_context() {
            self.saved_contexts.borrow_mut().push(previous);
        }
        ContextHandle(
            draw::select(Some(self.context(id).context())).map_or(0, |context| context.bits()),
        )
    }
    fn restore_context(&self, old: ContextHandle) {
        if old.0 == 0 {
            draw::select(None);
            self.restored_context.replace(None);
        } else {
            let mut saved = self.saved_contexts.borrow_mut();
            let index = saved
                .iter()
                .rposition(|context| context.context().bits() == old.0)
                .unwrap_or_else(|| panic!("menu context handle was not saved"));
            let context = saved.remove(index);
            context.select();
            self.restored_context.replace(Some(context));
        }
    }
    fn context_clip_rect(&self) -> Rect {
        rectangle(draw::clip())
    }
    fn set_fg_color(&self, c: Rgba) -> Rgba {
        rgba(draw::foreground(color(c)))
    }
    fn set_bg_color(&self, c: Rgba) -> Rgba {
        rgba(draw::background(color(c)))
    }
    fn set_font(&self, id: FontId) -> FontHandle {
        let previous = draw::selected_font();
        let old = draw::select_font(
            self.fonts[match id {
                FontId::StarCon => 0,
                FontId::Tiny => 1,
                FontId::Micro => 2,
            }]
            .clone(),
        )
        .map_or(0, |font| font.bits());
        self.saved_fonts.borrow_mut().push((old, previous));
        FontHandle(old)
    }
    fn restore_font(&self, old: FontHandle) {
        let mut saved = self.saved_fonts.borrow_mut();
        let index = saved
            .iter()
            .rposition(|(handle, _)| *handle == old.0)
            .unwrap_or_else(|| panic!("menu font handle was not saved"));
        draw::select_font(saved.remove(index).1);
    }
    fn set_font_effect(&self, index: Option<u16>) -> FontEffectHandle {
        let previous = draw::selected_effect();
        let effect = index.map_or_else(ContextFrame::default, |i| self.gradient.wrapped_frame(i));
        let old = draw::select_effect(effect).bits();
        self.saved_effects.borrow_mut().push((old, previous));
        FontEffectHandle(old)
    }
    fn restore_font_effect(&self, old: FontEffectHandle) {
        let mut saved = self.saved_effects.borrow_mut();
        let index = saved
            .iter()
            .rposition(|(handle, _)| *handle == old.0)
            .unwrap_or_else(|| panic!("menu font-effect handle was not saved"));
        draw::select_effect(saved.remove(index).1);
    }
    fn font_leading(&self) -> i16 {
        draw::font_leading()
    }
    fn set_context_clip_rect(&self, r: Rect) {
        draw::set_clip(rect(r));
    }
    fn fill_rect(&self, r: Rect) {
        draw::fill(rect(r));
    }
    fn draw_rect(&self, r: Rect) {
        draw::outline(rect(r));
    }
    fn draw_point(&self, p: Point) {
        draw::point(point(p));
    }
    fn draw_stamp(&self, s: Stamp) {
        draw::stamp(
            crate::graphics::drawable::FrameId::from_bits(s.frame),
            point(s.origin),
        );
    }
    fn draw_filled_stamp(&self, s: Stamp) {
        draw::filled_stamp(
            crate::graphics::drawable::FrameId::from_bits(s.frame),
            point(s.origin),
        );
    }
    fn draw_text(&self, t: &TextDraw) {
        let text =
            CString::new(t.text.as_str()).unwrap_or_else(|e| panic!("invalid menu text: {e}"));
        draw::text(
            &text,
            point(t.baseline),
            match t.align {
                Align::Left => crate::graphics::geometry::TextAlign::Left,
                Align::Center => crate::graphics::geometry::TextAlign::Center,
                Align::Right => crate::graphics::geometry::TextAlign::Right,
            },
            u16::MAX,
        );
    }
    fn abs_frame(&self, f: FrameId, i: u16) -> FrameId {
        draw::frame(crate::graphics::drawable::FrameId::from_bits(f), i).bits()
    }
    fn rel_frame(&self, f: FrameId, i: i16) -> FrameId {
        draw::relative_frame(crate::graphics::drawable::FrameId::from_bits(f), i).bits()
    }
    fn inc_frame(&self, f: FrameId) -> FrameId {
        self.rel_frame(f, 1)
    }
    fn dec_frame(&self, f: FrameId) -> FrameId {
        self.rel_frame(f, -1)
    }
    fn frame_count(&self, f: FrameId) -> u16 {
        draw::frame_count(crate::graphics::drawable::FrameId::from_bits(f))
    }
    fn frame_index(&self, f: FrameId) -> u16 {
        draw::frame_index(crate::graphics::drawable::FrameId::from_bits(f))
    }
    fn play_frame(&self) -> FrameId {
        self.play.frame().bits()
    }
    fn batch(&self) {
        draw::batch();
    }
    fn unbatch(&self) {
        draw::unbatch();
    }
    fn clear_drawable(&self) {
        draw::clear();
    }
    fn set_transition_source_none(&self) {
        draw::transition_source();
    }
    fn screen_transition(&self, _kind: i32) {
        draw::transition();
    }
    fn set_flash(&self, target: FlashTarget) {
        if matches!(target, FlashTarget::Off) || (target == FlashTarget::Menu3do && self.pc_menu) {
            *self.highlight.context.borrow_mut() = None;
            return;
        }
        let screen = self.context(ContextId::Screen);
        match target {
            FlashTarget::Menu3do | FlashTarget::MenuAny => {
                self.context(ContextId::Status).with_selected(|| {
                    let clip = self.context(ContextId::Status).clip_rect();
                    crate::ui::sis::set_flash_rect(
                        &self.highlight,
                        screen,
                        clip,
                        Some(crate::graphics::geometry::Rect {
                            corner: crate::graphics::geometry::Point {
                                x: (self.dimensions.0 - 60) as i16 - clip.corner.x,
                                y: (self.dimensions.1 - 56) as i16 - clip.corner.y,
                            },
                            extent: crate::graphics::geometry::Extent {
                                width: 56,
                                height: 53,
                            },
                        }),
                    );
                });
            }
            FlashTarget::Rect(r) => {
                crate::ui::sis::set_flash_rect(&self.highlight, screen, draw::clip(), Some(rect(r)))
            }
            FlashTarget::Off => unreachable!(),
        }
    }
    fn pre_update_flash(&self) {
        self.highlight.pre_update_flash_rect();
    }
    fn post_update_flash(&self) {
        self.highlight.post_update_flash_rect();
    }
    fn game_string(&self, index: i32) -> String {
        self.text
            .text(
                usize::try_from(index).unwrap_or_else(|e| panic!("invalid menu string index: {e}")),
            )
            .unwrap_or(c"")
            .to_string_lossy()
            .into_owned()
    }
}
