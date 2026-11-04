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
    host.set_flag("YEHAT_CIVIL_WAR", 1);
    assert!(host.yehat_civil_war());
    let mut visited = Vec::new();
    host.for_each_ip_group(&mut |group| {
        visited.push(group.group_id);
        group.sys_loc = 3;
    });
    assert_eq!(visited, [7, 8]);
    assert_eq!(host.flag("YEHAT_CIVIL_WAR"), 1);
    assert_eq!(world.ip_groups_ref().get(first).sys_loc, 3);
    assert_eq!(world.ip_groups_ref().get(second).sys_loc, 3);
}
