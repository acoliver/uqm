//! Exercise the production conversation renderer with registered fonts and pixel storage.

use super::Comm;
use crate::graphics::{
    context::ContextLease, drawable::ContextFrame, font::LoadedFont, geometry::Extent,
    geometry::Point, geometry::Rect,
};

pub(super) fn with_display(name: &str, call: impl FnOnce(&mut Comm, &ContextLease)) {
    if !uqm_engine::test_support::native_process::own_graphics_process(name) {
        return;
    }
    let mut game = crate::engine::Game::new();
    let fixture = uqm_engine::test_support::campaign_display::CampaignDisplay::install_prepared(
        &mut game,
        || {},
        |game| {
            let context = ContextLease::require(game.kernel.status_context);
            context.set_frame(ContextFrame::retain(game.kernel.screen));
            context.set_clip(Rect {
                corner: Point { x: 0, y: 0 },
                extent: Extent {
                    width: 320,
                    height: 240,
                },
            });
            let original = crate::graphics::font::FontLease::retain(game.kernel.tiny_font.unwrap());
            context.set_font(&original);
            let font = LoadedFont::load(c"fixture.tiny").unwrap();
            let mut comm = Comm::new();
            comm.init().unwrap();
            comm.presentation.space_context = Some(context.context());
            comm.presentation.tiny_font = game.kernel.tiny_font;
            comm.admit_conversation_names(&game.world).unwrap();
            let history = crate::graphics::font::FontLease::retain(game.kernel.tiny_font.unwrap());
            let display = super::sis_graphics::ConversationDisplay {
                space: context.clone(),
                player_font: font.lease(),
                history_font: history.clone(),
                game_strings: game.kernel.game_text_table(),
                subtitles: super::sis_graphics::SubtitleCache {
                    target: context.clone(),
                    context: context.clone(),
                    frame: context.target(),
                    font: history,
                },
            };
            comm.display.resources.as_mut().unwrap().display = Some(display);
            let activity = crate::graphics::drawable::LoadedDrawable::create(6, 8, 6, 10).unwrap();
            comm.presentation.activity_frame = activity.lease().frame(0).unwrap();
            comm.presentation.radar_context = Some(context.context());
            comm.presentation.off_screen_context = Some(context.context());
            call(&mut comm, &context);
            drop(comm.display.resources.as_mut().unwrap().display.take());
            context.flush();
            drop(font);
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
            let pixel = frame.pixel(Point { x, y });
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
            comm.presentation.space_context = None;
            comm.presentation.tiny_font = None;
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
            let original = uqm_engine::test_support::native_probe::context_font(context.context());
            comm.add_response(1, "A", None);
            super::sis_graphics::refresh_responses(comm, 0, 0, 1, 0);
            super::sis_graphics::feedback_player_phrase(comm, 0, Some(c"A"));
            assert_eq!(
                uqm_engine::test_support::native_probe::context_font(context.context()),
                original
            );
        },
    );
}

#[test]
#[serial_test::serial]
fn a_conversation_draws_with_one_font_and_frees_it_only_at_teardown() {
    with_display("comm::sis_graphics_font_tests::a_conversation_draws_with_one_font_and_frees_it_only_at_teardown", |comm, context| {
        let original = comm.conversation_display().player_font.clone();
        comm.add_response(1, "A", None);
        comm.add_response(2, "A A", None);
        super::sis_graphics::refresh_responses(comm, 0, 0, 2, 0);
        super::sis_graphics::feedback_player_phrase(comm, 0, Some(c"A A"));
        super::sis_graphics::refresh_responses(comm, 0, 0, 2, 1);
        assert!(comm.conversation_display().player_font.same_font(&original));
        assert!(has_text(context));
        super::sis_graphics::refresh_responses(comm, 0, 1, 2, 1);
        assert_eq!(comm.response_display, [1, 2, 1]);
    });
}
