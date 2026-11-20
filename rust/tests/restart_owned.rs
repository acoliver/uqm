#![forbid(unsafe_code)]
use uqm_rust::{
    engine::{
        battle::{Battle, Cx},
        observe::{Io, Unobserved},
        Game,
    },
    mainloop::restart_menu::restart_ops::{RestartHost, RestartMenu, RestartMenuOps},
};

struct NoLifecycle;
impl RestartHost for NoLifecycle {
    fn melee(&mut self, _: &mut Cx<'_>, _: &mut Io<'_>) {
        panic!("unexpected melee");
    }
    fn credits(&mut self, _: &mut Cx<'_>, _: &mut Io<'_>, _: bool) {
        panic!("unexpected credits");
    }
    fn free_game_data(&mut self, _: &mut Cx<'_>) {
        panic!("unexpected cleanup");
    }
}

#[test]
fn player_control_writes_only_the_supplied_kernel_and_keeps_byte_truncation() {
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
            uqm_rust::math::random,
        );
        menu.operations(&mut cx, &mut io, &mut host)
            .set_player_control(player, value);
    }
    assert_eq!(first.kernel.player_control, [7, 0x23]);
    assert_eq!(second.kernel.player_control, [0xff, 4]);
}
