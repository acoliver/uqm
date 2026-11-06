use uqm_rust::engine::Game;
use uqm_rust::mainloop::restart_menu::menu_abi::uqm_set_player_control;

#[test]
fn player_control_writes_only_the_supplied_kernel_and_keeps_byte_truncation() {
    let mut first = Game::new();
    let mut second = Game::new();
    first.kernel.player_control = [7, 8];
    second.kernel.player_control = [3, 4];
    uqm_set_player_control(&mut first.kernel, 1, 0x123);
    uqm_set_player_control(&mut second.kernel, 0, 0x1ff);
    assert_eq!(first.kernel.player_control, [7, 0x23]);
    assert_eq!(second.kernel.player_control, [0xff, 4]);
}
