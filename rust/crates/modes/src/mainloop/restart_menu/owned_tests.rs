use super::restart_ops::{RestartHost, RestartMenu, RestartMenuOps};
use crate::engine::{
    battle::{Battle, Cx},
    observe::{
        CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented, Unobserved,
    },
    Game, StoryKey,
};

struct NoLifecycle;
impl RestartHost for NoLifecycle {
    fn melee(&mut self, _: &mut Cx<'_>, _: &mut Io<'_>) {
        panic!("unexpected melee request");
    }
    fn credits(&mut self, _: &mut Cx<'_>, _: &mut Io<'_>, _: bool) {
        panic!("unexpected credits request");
    }
    fn free_game_data(&mut self, _: &mut Cx<'_>) {
        panic!("unexpected campaign teardown");
    }
}

#[test]
fn production_menu_borrows_unpublished_story_and_activity_owners_independently() {
    let mut first = Game::new();
    let mut second = Game::new();
    first.world.story = Some(crate::engine::world::story::StoryState::new());
    second.world.story = Some(crate::engine::world::story::StoryState::new());
    first.world.set_story_state(StoryKey::UTWIG_BOMB, 1);
    first.world.set_story_state(StoryKey::UTWIG_BOMB_ON_SHIP, 1);
    second
        .world
        .set_story_state(StoryKey::UTWIG_BOMB_ON_SHIP, 0);
    first.world.data.flagship.crew_enlisted = 0xffff;
    second.world.data.flagship.crew_enlisted = 17;
    let mut menu = RestartMenu::default();
    let mut host = NoLifecycle;
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    for (game, bomb, onboard, crew, activity) in [
        (&mut first, 1, 1, 0xffff, 0x4000),
        (&mut second, 0, 0, 17, 0x1004),
    ] {
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let mut ops = menu.operations(&mut cx, &mut io, &mut host);
        assert_eq!(ops.get_utwig_bomb(), bomb);
        assert_eq!(ops.get_utwig_bomb_on_ship(), onboard);
        assert_eq!(ops.get_crew_enlisted(), crew);
        ops.set_utwig_bomb_on_ship(0);
        ops.set_current_activity(activity);
        ops.set_last_activity(activity ^ 0x2000);
        ops.set_next_activity(activity ^ 0x1000);
        assert_eq!(ops.get_current_activity(), activity);
        assert_eq!(ops.get_last_activity(), activity ^ 0x2000);
    }
    assert_eq!(first.world.story_state(StoryKey::UTWIG_BOMB_ON_SHIP), 0);
    assert_eq!(second.world.story_state(StoryKey::UTWIG_BOMB_ON_SHIP), 0);
    assert_eq!(first.world.activity(), 0x4000);
    assert_eq!(second.world.activity(), 0x1004);
    assert_eq!(first.world.next_activity, 0x5000);
    assert_eq!(second.world.next_activity, 4);
}

#[test]
fn production_menu_controls_keep_owner_isolation_and_original_byte_narrowing() {
    let mut first = Game::new();
    let mut second = Game::new();
    first.kernel.player_control = [7, 8];
    second.kernel.player_control = [3, 4];
    let mut menu = RestartMenu::default();
    let mut host = NoLifecycle;
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    for (game, player, value) in [(&mut first, 1, 0x123), (&mut second, 0, 0x1ff)] {
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        menu.operations(&mut cx, &mut io, &mut host)
            .set_player_control(player, value);
    }
    assert_eq!(first.kernel.player_control, [7, 0x23]);
    assert_eq!(second.kernel.player_control, [0xff, 4]);
}

#[derive(Default)]
struct Admissions(Vec<InputView>);
impl Observer for Admissions {
    fn input_admitted(&mut self, view: &InputView, cx: &mut Cx<'_>) -> Injected {
        assert!(CampaignView::for_input(*view, || CampaignView::capture(cx)).is_none());
        self.0.push(*view);
        Injected {
            stop: matches!(view, InputView::ReplacesIntroduction),
            ..Default::default()
        }
    }
    fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
    fn presented(&mut self, _: &mut Presented) {}
}

#[test]
#[serial_test::serial]
fn production_menu_admits_the_supplied_observer_without_requiring_display_resources() {
    let mut game = Game::new();
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    let mut observer = Admissions::default();
    let mut io = Io {
        observer: &mut observer,
    };
    let mut host = NoLifecycle;
    let mut menu = RestartMenu::default();
    {
        let mut ops = menu.operations(&mut cx, &mut io, &mut host);
        assert!(ops.input(InputView::ReplacesIntroduction).stop);
        assert!(!ops.input(InputView::MainMenuReady).stop);
    }
    assert_eq!(
        observer.0,
        [InputView::ReplacesIntroduction, InputView::MainMenuReady]
    );
}

#[test]
#[serial_test::serial]
fn retained_production_menu_draws_centered_pixels_after_campaign_resources_are_dropped() {
    if !uqm_engine::test_support::native_process::own_graphics_process("mainloop::restart_menu::owned_tests::retained_production_menu_draws_centered_pixels_after_campaign_resources_are_dropped") {
        return;
    }
    use crate::graphics::{
        context::ContextLease, drawable::ContextFrame, geometry::Color, geometry::Extent,
        geometry::Point, geometry::Rect,
    };
    let mut game = Game::new();
    let mut menu = RestartMenu::default();
    let mut host = NoLifecycle;
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let fixture = uqm_engine::test_support::campaign_display::CampaignDisplay::install_prepared(
        &mut game,
        || {},
        |game| {
            let context = ContextLease::require(game.kernel.status_context);
            game.kernel.screen_context = game.kernel.status_context;
            context.set_frame(ContextFrame::retain(game.kernel.screen));
            context.set_clip(Rect {
                corner: Point { x: 0, y: 0 },
                extent: Extent {
                    width: 320,
                    height: 240,
                },
            });
            let mut battle = Battle::new();
            {
                let mut cx = Cx::new(
                    &mut battle,
                    &mut game.world,
                    &mut game.kernel,
                    crate::math::random,
                );
                let mut ops = menu.operations(&mut cx, &mut io, &mut host);
                ops.set_screen_context();
                ops.load_menu_graphic();
                ops.draw_restart_menu_graphic();
                ops.create_flash_overlay();
                ops.flash_set_merge_factors(-3, 3, 16);
                ops.flash_set_speed(315, 0, 315, 0);
                ops.flash_set_frame_time(52);
                ops.flash_set_state_fade_in(157);
                ops.draw_restart_menu_state(0);
                ops.flash_pause();
                ops.flash_continue();
                ops.flash_terminate();
            }
            context.flush();
            let target = context.target();
            let rgba = |point| {
                let Color { r, g, b, a } = target.pixel(point);
                (r, g, b, a)
            };
            assert_eq!(rgba(Point { x: 159, y: 119 }), (255, 255, 255, 255));
            assert_eq!(rgba(Point { x: 160, y: 119 }), (0, 0, 0, 255));
            assert!(
                (230..240)
                    .any(|y| (300..320).any(|x| rgba(Point { x, y }) == (255, 255, 255, 255))),
                "the version label must render with the retained tiny font"
            );
            drop(std::mem::replace(game, Game::new()));
            let mut cx = Cx::new(
                &mut battle,
                &mut game.world,
                &mut game.kernel,
                crate::math::random,
            );
            let mut ops = menu.operations(&mut cx, &mut io, &mut host);
            ops.batch_graphics();
            ops.draw_restart_menu_graphic();
            ops.unbatch_graphics();
            context.flush();
            assert_eq!(rgba(Point { x: 159, y: 119 }), (255, 255, 255, 255));
            assert!(
                (230..240)
                    .any(|y| (300..320).any(|x| rgba(Point { x, y }) == (255, 255, 255, 255))),
                "the version label must survive campaign font teardown"
            );
            ops.destroy_menu_graphic();
            assert!(!ops.has_menu_music());
            assert!(!ops.load_menu_music());
            ops.destroy_music();
        },
    )
    .unwrap();
    drop(menu);
    fixture.finish(game);
}
