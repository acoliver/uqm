//! The campaign clock entry points: scheduled events run on their day and
//! the status line is redrawn as days pass.

use super::{add_event, advance_days, game_clock_tick};
use uqm_engine::time::clock_abi::schedule_event;
use uqm_engine::{
    battle::{Battle, Cx},
    clock::ARILOU_UMGAH_CHECK,
    Game, StoryKey, World,
};

fn world() -> World {
    let mut world = World::new();
    world.story = Some(uqm_engine::world::story::StoryState::new());
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
    if !uqm_engine::test_support::native_process::own_graphics_process(concat!(
        module_path!(),
        "::supplied_day_passage_and_tick_dispatch_with_real_status_drawing"
    )) {
        return;
    }
    let mut game = Game::new();
    let display =
        uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game).unwrap();
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
    let context = uqm_engine::graphics::context::ContextLease::require(game.kernel.status_context);
    context.flush();
    let frame = uqm_engine::graphics::drawable::ContextFrame::retain(game.kernel.screen);
    let pixel = frame.pixel(uqm_engine::graphics::geometry::Point { x: 2, y: 130 });
    assert_eq!((pixel.r, pixel.g, pixel.b), (0, 66, 0));
    display.finish(game);
}

#[test]
fn scheduling_dispatches_immediate_events_even_when_the_pool_is_full() {
    let mut world = world();
    let mut random = || panic!("story event sampled RNG");
    let mut dispatch = super::dispatcher((320, 240), &mut random);
    assert!(schedule_event(&mut world, &mut dispatch, 0, 2, 27, 2156, 99).is_none());
    assert_eq!(world.clock.events_ref().count(), 0);
    for selector in 0..uqm_engine::clock::NUM_EVENTS {
        assert!(schedule_event(&mut world, &mut dispatch, 1, 0, 1, 0, selector as u8).is_some());
    }
    assert!(schedule_event(&mut world, &mut dispatch, 1, 0, 1, 0, 99).is_none());
    assert!(schedule_event(
        &mut world,
        &mut dispatch,
        1,
        0,
        0,
        0,
        ARILOU_UMGAH_CHECK as u8
    )
    .is_none());
    assert_eq!(world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
    assert_eq!(
        world.clock.events_ref().count(),
        uqm_engine::clock::NUM_EVENTS
    );
}
