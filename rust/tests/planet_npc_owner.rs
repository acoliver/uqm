use uqm::engine::StoryKey;
use uqm_rust::engine::World;
use uqm_rust::planets::c_api::NpcWorld;
use uqm_rust::planets::generate::default::Npcs;
use uqm_rust::planets::generate::special::Story;

#[test]
fn npc_story_and_groups_use_the_supplied_world_without_a_published_context() {
    let mut world = World::new();
    uqm_rust::state::ffi::rust_init_game_state(&mut world);
    let mut groups = world.ip_groups();
    let first = groups.alloc().unwrap();
    groups.get_mut(first).group_id = 7;
    groups.put(first);
    let second = groups.alloc().unwrap();
    groups.get_mut(second).group_id = 8;
    groups.put(second);

    let mut host = NpcWorld::new(&mut world);
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
        uqm_rust::planets::generate::special::FleetGroups::head_ip_group(&mut host, &mut |group| {
            group.group_counter = 17
        },)
    );
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

    let mut world = World::new();
    let mut escorts = world.built_ships();
    let escort = escorts.alloc().unwrap();
    escorts.put(escort);
    let mut ships = world.npc_built_ships();
    let npc = ships.alloc().unwrap();
    ships.put(npc);
    let mut groups = world.ip_groups();
    let group = groups.alloc().unwrap();
    groups.put(group);

    {
        let mut host = NpcWorld::new(&mut world);
        assert_eq!(Queues::built_ship_count(&host), 1);
        assert_eq!(GuardSite::built_ship_count(&host), 1);
        GuardSite::set_battle_group_ref(&mut host, 0x1234_5678);
        assert_eq!(GuardSite::battle_group_ref(&host), 0x1234_5678);
        host.clear_battle_group_ref();
        assert_eq!(FleetGroups::battle_group_ref(&host), 0);
        GuardSite::reinit_built_queue(&mut host);
        assert_eq!(Queues::built_ship_count(&host), 0);
        GuardSite::clone_ship(&mut host, u8::MAX);
        assert_eq!(GuardSite::built_ship_count(&host), 0);
        Npcs::reinit_ip_group_q(&mut host);
        assert!(!FleetGroups::head_ip_group(&mut host, &mut |_| {
            panic!("an empty group queue must not visit a record")
        }));
        GuardSite::for_each_ip_group(&mut host, &mut |_| {
            panic!("an empty group queue must not visit a record")
        });
    }
    assert!(!world.npc_built_ships_ref().contains(npc));
    assert!(!world.ip_groups_ref().contains(group));
    assert!(world.built_ships_ref().contains(escort));
    assert_eq!(world.built_ships_ref().count(), 1);
}
