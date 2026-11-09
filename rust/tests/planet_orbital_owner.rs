use std::cell::RefCell;
use uqm_rust::engine::{
    battle::Battle, battle::Cx, observe::Io, observe::Unobserved, Game, StoryKey,
};
use uqm_rust::planets::c_api::{built_ship_count, built_ship_queue_has_ships};
use uqm_rust::planets::generate::special::Story;
use uqm_rust::planets::solarsys::site::Site;

#[test]
fn orbital_story_and_queue_hosts_share_the_caller_owner() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    let mut battle = Battle::new();
    let owner = RefCell::new(Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    ));
    let mut observer = Unobserved;
    let io = RefCell::new(Io {
        observer: &mut observer,
    });
    let mut pickup = uqm_rust::planet_side::special_effects::PickupContext::NoLander;
    let mut story_pickup = uqm_rust::planet_side::special_effects::PickupContext::NoLander;
    let host = Site::new(
        &owner,
        &io,
        uqm_rust::engine::StarId(0),
        0,
        false,
        &mut pickup,
        None,
    );
    let mut story = Site::new(
        &owner,
        &io,
        uqm_rust::engine::StarId(0),
        0,
        false,
        &mut story_pickup,
        None,
    );
    story.set_flag(StoryKey::YEHAT_CIVIL_WAR, 1);
    assert_eq!(host.flag(StoryKey::YEHAT_CIVIL_WAR), 1);
    story.set_word(StoryKey::URQUAN_PROBE_GRPOFFS0, 0x12345678);
    assert_eq!(host.word(StoryKey::URQUAN_PROBE_GRPOFFS0), 0x12345678);
    {
        let mut cx = owner.borrow_mut();
        let mut groups = cx.world.ip_groups();
        let id = groups.alloc().unwrap();
        groups.put(id);
    }
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
}
