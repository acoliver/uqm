//! Font lifetime of the SIS communication window renderers.
//!
//! `font_DrawText` queues one draw command per glyph and each command keeps a
//! pointer into the context font. The queue is consumed later, at the frame
//! boundary, so a font that is destroyed after queuing but before the
//! consumer runs leaves commands reading freed glyph data. [`QueuedGfx`]
//! models that deferred consumer and records every such use after free.
//! The conversation owns its player font and frees it at teardown through
//! [`ConversationFont`], the same path `HailAlien` uses.

use super::locdata::{CColor, CRect};
use super::player_font::{ConversationFont, FontRelease};
use super::sis_graphics::{
    feedback_player_phrase_with, refresh_responses_with, CText, CommWindowGfx,
};
use std::ffi::{c_char, c_int, c_void, CStr};

const GAME_STRING: &CStr = c"You say:";

fn handle(index: usize) -> *mut c_void {
    (index + 1) as *mut c_void
}

struct QueuedGfx {
    /// `alive` flag per font; handle `n` is `fonts[n - 1]`.
    fonts: Vec<bool>,
    context_font: usize,
    conversation_font: *mut c_void,
    destroyed_fonts: Vec<usize>,
    queued_draw_fonts: Vec<usize>,
    responses: Vec<String>,
    violations: Vec<String>,
}

impl QueuedGfx {
    /// A conversation: the space context already uses `initial` and the
    /// conversation font is loaded and installed in `slot`.
    fn in_conversation(slot: &ConversationFont) -> (Self, *mut c_void) {
        let mut gfx = Self {
            fonts: Vec::new(),
            context_font: 0,
            conversation_font: std::ptr::null_mut(),
            destroyed_fonts: Vec::new(),
            queued_draw_fonts: Vec::new(),
            responses: Vec::new(),
            violations: Vec::new(),
        };
        let initial = gfx.new_font();
        gfx.context_font = initial as usize;
        gfx.conversation_font = gfx.new_font();
        slot.install(gfx.conversation_font).unwrap();
        (gfx, initial)
    }

    fn new_font(&mut self) -> *mut c_void {
        self.fonts.push(true);
        handle(self.fonts.len() - 1)
    }

    /// The frame-boundary consumer: every queued draw reads its font.
    fn consume_queued_draws(&mut self) {
        for font in std::mem::take(&mut self.queued_draw_fonts) {
            if !self.fonts[font - 1] {
                self.violations
                    .push(format!("queued draw read freed font {font}"));
            }
        }
    }
}

impl CommWindowGfx for QueuedGfx {
    fn current_activity(&self) -> u16 {
        0
    }
    fn space_context(&self) -> *mut c_void {
        handle(99)
    }
    fn set_context(&mut self, ctx: *mut c_void) -> *mut c_void {
        ctx
    }
    fn set_context_font(&mut self, font: *mut c_void) -> *mut c_void {
        std::mem::replace(&mut self.context_font, font as usize) as *mut c_void
    }
    fn set_foreground_color(&mut self, _color: CColor) {}
    fn font_leading(&mut self) -> i16 {
        8
    }
    fn batch(&mut self) {}
    fn unbatch(&mut self) {}
    fn fill_rect(&mut self, _rect: &mut CRect) {}
    fn draw_text(&mut self, _text: &mut CText) {
        assert!(
            self.context_font != 0 && self.fonts[self.context_font - 1],
            "text drawn with no live context font"
        );
        self.queued_draw_fonts.push(self.context_font);
    }
    fn line_within_width(
        &mut self,
        text: &mut CText,
        next: &mut *const c_char,
        _width: i16,
        _max_chars: u16,
    ) -> c_int {
        // SAFETY: the renderers pass NUL-terminated strings.
        let len = unsafe { CStr::from_ptr(text.p_str) }.to_bytes().len();
        text.char_count = len as u16;
        *next = unsafe { text.p_str.add(len) };
        1
    }
    fn game_string(&mut self, _id: i32) -> *const c_char {
        GAME_STRING.as_ptr()
    }
    fn response_text(&mut self, index: u8, buf: &mut [u8]) -> bool {
        let Some(text) = self.responses.get(usize::from(index)) else {
            return false;
        };
        buf[..text.len()].copy_from_slice(text.as_bytes());
        buf[text.len()] = 0;
        true
    }
    fn draw_scroll_indicator(&mut self, _frame_index: u16, _origin_y: i16) {}
    fn conversation_player_font(&mut self) -> *mut c_void {
        self.conversation_font
    }
}

impl FontRelease for QueuedGfx {
    fn drain_draw_queue(&mut self) {
        self.consume_queued_draws();
    }
    fn destroy_font(&mut self, font: *mut c_void) {
        let index = font as usize;
        if self.context_font == index {
            self.context_font = 0;
        }
        let borrowing = self
            .queued_draw_fonts
            .iter()
            .filter(|f| **f == index)
            .count();
        if borrowing != 0 {
            self.violations.push(format!(
                "font {index} freed while {borrowing} queued draws still borrow its glyphs"
            ));
        }
        self.fonts[index - 1] = false;
        self.destroyed_fonts.push(index);
    }
}

#[test]
fn refreshing_responses_keeps_the_font_alive_until_queued_draws_are_consumed() {
    let slot = ConversationFont::new();
    let (mut gfx, _initial) = QueuedGfx::in_conversation(&slot);
    gfx.responses = vec!["Hello.".into(), "Who are you?".into()];

    refresh_responses_with(&mut gfx, 0, 2, 0);
    gfx.consume_queued_draws();

    assert_eq!(gfx.violations, Vec::<String>::new());
    assert_eq!(gfx.destroyed_fonts, Vec::<usize>::new());
}

#[test]
fn player_phrase_feedback_keeps_the_font_alive_until_queued_draws_are_consumed() {
    let slot = ConversationFont::new();
    let (mut gfx, _initial) = QueuedGfx::in_conversation(&slot);

    // SAFETY: the phrase is a NUL-terminated literal.
    unsafe { feedback_player_phrase_with(&mut gfx, c"Hello.".as_ptr()) };
    gfx.consume_queued_draws();

    assert_eq!(gfx.violations, Vec::<String>::new());
    assert_eq!(gfx.destroyed_fonts, Vec::<usize>::new());
}

#[test]
fn rendering_restores_the_context_font_it_found() {
    let slot = ConversationFont::new();
    let (mut gfx, initial) = QueuedGfx::in_conversation(&slot);
    gfx.responses = vec!["Hello.".into()];

    refresh_responses_with(&mut gfx, 0, 1, 0);
    // SAFETY: the phrase is a NUL-terminated literal.
    unsafe { feedback_player_phrase_with(&mut gfx, c"Hello.".as_ptr()) };

    assert_eq!(gfx.context_font, initial as usize);
}

#[test]
fn a_conversation_draws_with_one_font_and_frees_it_only_at_teardown() {
    let slot = ConversationFont::new();
    let (mut gfx, _initial) = QueuedGfx::in_conversation(&slot);
    let conversation_font = gfx.conversation_font as usize;
    gfx.responses = vec!["Hello.".into(), "Who are you?".into()];

    refresh_responses_with(&mut gfx, 0, 2, 0);
    // SAFETY: the phrase is a NUL-terminated literal.
    unsafe { feedback_player_phrase_with(&mut gfx, c"Who are you?".as_ptr()) };
    refresh_responses_with(&mut gfx, 0, 2, 1);
    assert_eq!(
        gfx.fonts.len(),
        2,
        "renderers must not load fonts of their own"
    );
    assert!(gfx.destroyed_fonts.is_empty());

    // Teardown with draws still queued: the release drains them first.
    assert!(!gfx.queued_draw_fonts.is_empty());
    let mut teardown = gfx;
    slot.release(&mut teardown);

    assert_eq!(teardown.violations, Vec::<String>::new());
    assert_eq!(teardown.destroyed_fonts, [conversation_font]);
    assert!(teardown.queued_draw_fonts.is_empty());
}
