//! Exercise the production conversation renderer with registered fonts and pixel storage.

use super::{player_font::FontRelease, Comm};
use crate::graphics::{
    context_abi::{ContextFrame, ContextLease},
    native_abi::{EXTENT, POINT, RECT},
    native_font::LoadedFont,
};

struct Release {
    context: ContextLease,
    font: Option<LoadedFont>,
    drained: bool,
}
impl FontRelease for Release {
    fn drain_draw_queue(&mut self) {
        self.context.flush();
        self.drained = true;
    }
    fn destroy_font(&mut self, font: *mut std::ffi::c_void) {
        assert!(self.drained);
        assert_eq!(font, self.font.as_ref().unwrap().as_raw().cast());
        drop(self.font.take());
    }
}

pub(super) fn with_display(name: &str, call: impl FnOnce(&mut Comm, &ContextLease)) {
    if !crate::native_test_process::own_graphics_process(name) {
        return;
    }
    let mut game = crate::engine::Game::new();
    let fixture = crate::campaign_display_test::CampaignDisplay::install_prepared(
        &mut game,
        || {},
        |game| {
            let context = ContextLease::require(game.kernel.status_context);
            context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
                game.kernel.screen,
            )));
            context.set_clip(RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 320,
                    height: 240,
                },
            });
            let original = crate::graphics::native_font::FontLease::retain(
                std::ptr::NonNull::new(game.kernel.tiny_font).unwrap(),
            );
            context.set_font(&original);
            let font = LoadedFont::load(c"fixture.tiny").unwrap();
            let mut comm = Comm::new();
            comm.presentation.space_context = context.as_raw();
            comm.presentation.tiny_font = game.kernel.tiny_font;
            comm.presentation.game_strings = game.kernel.game_strings;
            comm.player_font.install(font.as_raw().cast()).unwrap();
            comm.admit_conversation_names(&game.world).unwrap();
            comm.display.anim_context = context.as_raw().cast();
            comm.display.text_cache_context = context.as_raw();
            comm.display.text_cache_frame = game.kernel.screen.cast();
            comm.data.alien_font = game.kernel.tiny_font.cast();
            // SAFETY: the display fixture owns all registered handles and the Kernel text table.
            let display =
                unsafe { super::sis_graphics::ConversationDisplay::retain(&comm) }.unwrap();
            comm.display.resources.as_mut().unwrap().display = Some(display);
            let mut release = Release {
                context: context.clone(),
                font: Some(font),
                drained: false,
            };
            call(&mut comm, &context);
            drop(comm.display.resources.as_mut().unwrap().display.take());
            comm.player_font.release(&mut release);
            assert!(release.drained);
            assert!(release.font.is_none());
        },
    )
    .unwrap();
    fixture.finish(game);
}

fn has_text(context: &ContextLease) -> bool {
    context.flush();
    let frame = context.target();
    (122..227).any(|y| {
        (0..242).any(|x| {
            let pixel = frame.pixel(POINT { x, y });
            pixel.r != 0 || pixel.g != 0
        })
    })
}

#[test]
#[serial_test::serial]
fn refreshing_responses_keeps_the_font_alive_until_queued_draws_are_consumed() {
    with_display("comm::sis_graphics_font_tests::refreshing_responses_keeps_the_font_alive_until_queued_draws_are_consumed", |comm, context| {
        comm.add_response(1, "A", None);
        comm.add_response(2, "A A", None);
        super::sis_graphics::refresh_responses(comm, 0, 0, 2, 0);
        assert!(has_text(context));
        assert_eq!(comm.response_display, [0, 2, 0]);
    });
}

#[test]
#[serial_test::serial]
fn admitted_display_renders_without_recovering_raw_resources() {
    with_display(
        "comm::sis_graphics_font_tests::admitted_display_renders_without_recovering_raw_resources",
        |comm, context| {
            comm.presentation.space_context = std::ptr::null_mut();
            comm.presentation.tiny_font = std::ptr::null_mut();
            comm.presentation.game_strings = std::ptr::null_mut();
            comm.display.anim_context = std::ptr::null_mut();
            comm.display.text_cache_context = std::ptr::null_mut();
            comm.display.text_cache_frame = std::ptr::null_mut();
            comm.data.alien_font = std::ptr::null_mut();
            comm.add_response(1, "A A", None);
            super::sis_graphics::refresh_responses(comm, 0, 0, 1, 0);
            assert!(has_text(context));
            super::sis_graphics::feedback_player_phrase(comm, 0, Some(c"A"));
            assert!(has_text(context));
        },
    );
}

#[test]
#[serial_test::serial]
fn player_phrase_feedback_keeps_the_font_alive_until_queued_draws_are_consumed() {
    with_display("comm::sis_graphics_font_tests::player_phrase_feedback_keeps_the_font_alive_until_queued_draws_are_consumed", |comm, context| {
        super::sis_graphics::feedback_player_phrase(comm, 0, Some(c"A"));
        assert!(has_text(context));
        super::sis_graphics::feedback_player_phrase(comm, 0, None);
        assert!(!has_text(context));
    });
}

#[test]
#[serial_test::serial]
fn rendering_restores_the_context_font_it_found() {
    with_display(
        "comm::sis_graphics_font_tests::rendering_restores_the_context_font_it_found",
        |comm, context| {
            // SAFETY: this fixture retains the selected registered context throughout both draws.
            let original = unsafe { (*context.as_raw()).Font };
            comm.add_response(1, "A", None);
            super::sis_graphics::refresh_responses(comm, 0, 0, 1, 0);
            super::sis_graphics::feedback_player_phrase(comm, 0, Some(c"A"));
            // SAFETY: the context remains retained by the fixture.
            assert_eq!(unsafe { (*context.as_raw()).Font }, original);
        },
    );
}

#[test]
#[serial_test::serial]
fn a_conversation_draws_with_one_font_and_frees_it_only_at_teardown() {
    with_display("comm::sis_graphics_font_tests::a_conversation_draws_with_one_font_and_frees_it_only_at_teardown", |comm, context| {
        let original = comm.player_font.current();
        comm.add_response(1, "A", None);
        comm.add_response(2, "A A", None);
        super::sis_graphics::refresh_responses(comm, 0, 0, 2, 0);
        super::sis_graphics::feedback_player_phrase(comm, 0, Some(c"A A"));
        super::sis_graphics::refresh_responses(comm, 0, 0, 2, 1);
        assert_eq!(comm.player_font.current(), original);
        assert!(has_text(context));
        super::sis_graphics::refresh_responses(comm, 0, 1, 2, 1);
        assert_eq!(comm.response_display, [1, 2, 1]);
    });
}
