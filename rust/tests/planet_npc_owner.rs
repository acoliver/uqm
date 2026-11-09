use std::cell::RefCell;
use uqm_rust::engine::{
    battle::{Battle, Cx},
    observe::{Io, Unobserved},
    Game, StoryKey,
};
use uqm_rust::planets::generate::default::Npcs;
use uqm_rust::planets::generate::special::Story;
use uqm_rust::planets::solarsys::site::Site;

fn with_host(game: &mut Game, run: impl FnOnce(&mut Site<'_, '_, '_>)) {
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
    let mut host = Site::new(
        &owner,
        &io,
        uqm_rust::engine::StarId(0),
        0,
        false,
        &mut pickup,
        None,
    );
    run(&mut host);
}

#[test]
fn npc_story_and_groups_use_the_supplied_world_without_a_published_context() {
    let mut game = Game::new();
    uqm_rust::state::ffi::rust_init_game_state(&mut game.world);
    let (first, second) = {
        let mut groups = game.world.ip_groups();
        let first = groups.alloc().unwrap();
        groups.get_mut(first).group_id = 7;
        groups.put(first);
        let second = groups.alloc().unwrap();
        groups.get_mut(second).group_id = 8;
        groups.put(second);
        (first, second)
    };
    with_host(&mut game, |host| {
        host.set_flag(StoryKey::YEHAT_CIVIL_WAR, 1);
        assert!(host.yehat_civil_war());
        let mut visited = Vec::new();
        host.for_each_ip_group(&mut |group| {
            visited.push(group.group_id);
            group.sys_loc = 3;
        });
        assert_eq!(visited, [7, 8]);
        assert_eq!(host.flag(StoryKey::YEHAT_CIVIL_WAR), 1);
        host.set_word(StoryKey::URQUAN_PROBE_GRPOFFS0, 0xdeca_fbad);
        assert_eq!(host.word(StoryKey::URQUAN_PROBE_GRPOFFS0), 0xdeca_fbad);
        assert!(
            uqm_rust::planets::generate::special::FleetGroups::head_ip_group(host, &mut |group| {
                group.group_counter = 17
            })
        );
    });
    let world = &game.world;
    assert_eq!(world.ip_groups_ref().get(first).group_counter, 17);
    assert_eq!(world.ip_groups_ref().get(second).group_counter, 0);
    assert_eq!(world.ip_groups_ref().get(first).sys_loc, 3);
    assert_eq!(world.ip_groups_ref().get(second).sys_loc, 3);
}

#[test]
fn npc_guard_queue_operations_preserve_escorts_and_invalidate_only_reset_ids() {
    use uqm_rust::planets::generate::sam::GuardSite;
    use uqm_rust::planets::generate::sol::orbital::Queues;
    use uqm_rust::planets::generate::special::FleetGroups;
    let mut game = Game::new();
    let (escort, npc, group) = {
        let world = &mut game.world;
        let mut escorts = world.built_ships();
        let escort = escorts.alloc().unwrap();
        escorts.put(escort);
        let mut ships = world.npc_built_ships();
        let npc = ships.alloc().unwrap();
        ships.put(npc);
        let mut groups = world.ip_groups();
        let group = groups.alloc().unwrap();
        groups.put(group);
        (escort, npc, group)
    };
    with_host(&mut game, |host| {
        assert_eq!(Queues::built_ship_count(host), 1);
        assert_eq!(GuardSite::built_ship_count(host), 1);
        GuardSite::set_battle_group_ref(host, 0x1234_5678);
        assert_eq!(GuardSite::battle_group_ref(host), 0x1234_5678);
        host.clear_battle_group_ref();
        assert_eq!(FleetGroups::battle_group_ref(host), 0);
        GuardSite::reinit_built_queue(host);
        assert_eq!(Queues::built_ship_count(host), 0);
        GuardSite::clone_ship(host, u8::MAX);
        assert_eq!(GuardSite::built_ship_count(host), 0);
        Npcs::reinit_ip_group_q(host);
        assert!(!FleetGroups::head_ip_group(host, &mut |_| {
            panic!("an empty group queue must not visit a record")
        }));
        GuardSite::for_each_ip_group(host, &mut |_| {
            panic!("an empty group queue must not visit a record")
        });
    });
    let world = &game.world;
    assert!(!world.npc_built_ships_ref().contains(npc));
    assert!(!world.ip_groups_ref().contains(group));
    assert!(world.built_ships_ref().contains(escort));
    assert_eq!(world.built_ships_ref().count(), 1);
}
