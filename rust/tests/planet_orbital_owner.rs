use uqm_rust::engine::Game;
use uqm_rust::planets::c_api::{built_ship_count, built_ship_queue_has_ships};
use uqm_rust::planets::generate::special::Story;
#[test]
fn orbital_story_and_queue_hosts_share_the_caller_owner() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    // SAFETY: the serialized fixture retains its game and uses only synchronous owner handoffs.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::app::bridge::enter(|cx| {
                let owner = std::cell::RefCell::new(cx);
                let host = &owner;
                let mut story = &owner;
                story.set_flag("YEHAT_CIVIL_WAR", 1);
                assert_eq!(host.flag("YEHAT_CIVIL_WAR"), 1);
                story.set_word("URQUAN_PROBE_GRPOFFS0", 0x12345678);
                assert_eq!(host.word("URQUAN_PROBE_GRPOFFS0"), 0x12345678);
                {
                    let mut cx = owner.borrow_mut();
                    let mut groups = cx.world.ip_groups();
                    let id = groups.alloc().unwrap();
                    groups.put(id);
                };
                let npc = {
                    let mut cx = owner.borrow_mut();
                    let mut ships = cx.world.npc_built_ships();
                    let id = ships.alloc().unwrap();
                    ships.put(id);
                    id
                };
                assert_eq!(built_ship_count(owner.borrow().world), 1);
                assert!(built_ship_queue_has_ships(owner.borrow().world));
                owner.borrow_mut().world.npc_built_ships().reinit();
                owner.borrow_mut().world.ip_groups().reinit();
                let cx = owner.borrow();
                assert!(!cx.world.npc_built_ships_ref().contains(npc));
                assert!(cx.world.ip_groups_ref().head().is_none());
                assert_eq!(built_ship_count(cx.world), 0);
                assert!(!built_ship_queue_has_ships(cx.world));
            });
        });
    }
}
