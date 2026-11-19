//! The campaign clock entry points: scheduled events run on their day and
//! the status line is redrawn as days pass.

use super::{add_event, advance_days, game_clock_tick};
use crate::engine::{
    battle::{Battle, Cx},
    clock::ARILOU_UMGAH_CHECK,
    Game, StoryKey, World,
};

fn world() -> World {
    let mut world = World::new();
    world.story = Some(crate::engine::world::story::StoryState::new());
    world.data.game.game_clock.year_index = 2156;
    world.data.game.game_clock.month_index = 2;
    world.data.game.game_clock.day_index = 28;
    world.clock.init();
    world
}

#[test]
fn zero_and_negative_day_passage_reset_ticks_without_drawing_or_dispatch() {
    let mut game = Game::new();
    game.world = world();
    game.world.data.game.game_clock.day_in_ticks = 24;
    game.world.data.game.game_clock.tick_count = 5;
    let mut battle = Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("clock RNG")
    });
    for days in [0, -1, i32::MIN] {
        advance_days(&mut cx, days);
        assert_eq!(cx.world.data.calendar_date(), (2, 28, 2156));
        assert_eq!(cx.world.data.game.game_clock.tick_count, 24);
        cx.world.data.game.game_clock.tick_count = 5;
    }
}

#[test]
#[serial_test::serial]
fn supplied_day_passage_and_tick_dispatch_with_real_status_drawing() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::supplied_day_passage_and_tick_dispatch_with_real_status_drawing"
    )) {
        return;
    }
    let mut game = Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    game.world = world();
    game.world.data.game.game_clock.day_in_ticks = 2;
    game.world.data.game.game_clock.tick_count = 1;
    let mut battle = Battle::new();
    {
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
            panic!("clock RNG")
        });
        add_event(&mut cx, 1, 0, 1, 0, ARILOU_UMGAH_CHECK as u8).unwrap();
        game_clock_tick(&mut cx);
        assert_eq!(cx.world.data.calendar_date(), (2, 29, 2156));
        assert_eq!(cx.world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
        assert_eq!(cx.world.data.game.game_clock.tick_count, 2);
        game_clock_tick(&mut cx);
        assert_eq!(cx.world.data.calendar_date(), (2, 29, 2156));
        assert_eq!(cx.world.data.game.game_clock.tick_count, 1);
        cx.world.set_story_state(StoryKey::ARILOU_CHECKED_UMGAH, 0);
        add_event(&mut cx, 1, 0, 1, 0, ARILOU_UMGAH_CHECK as u8).unwrap();
        advance_days(&mut cx, 2);
        assert_eq!(cx.world.data.calendar_date(), (3, 2, 2156));
        assert_eq!(cx.world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
        assert_eq!(cx.world.data.game.game_clock.tick_count, 2);
        assert_eq!(cx.world.clock.events_ref().count(), 0);
    }
    let context = crate::graphics::context_abi::ContextLease::require(game.kernel.status_context);
    context.flush();
    let frame = crate::graphics::context_abi::ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    ));
    let pixel = frame.pixel(crate::graphics::native_abi::POINT { x: 2, y: 130 });
    assert_eq!((pixel.r, pixel.g, pixel.b), (0, 66, 0));
    display.finish(game);
}
