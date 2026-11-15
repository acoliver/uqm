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
impl Observer<Option<CampaignView>> for Admissions {
    fn input_admitted(&mut self, view: &InputView, world: &Option<CampaignView>) -> Injected {
        assert!(world.is_none());
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
