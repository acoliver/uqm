use std::cell::RefCell;
use uqm_rust::engine::{
    battle::{Battle, Cx},
    observe::{Io, Unobserved},
    world::state_file::RANDGRPINFO_FILE,
    Game, StarId,
};
use uqm_rust::planets::generate::default::Npcs;
use uqm_rust::planets::solarsys::site::Site;

#[test]
fn solar_host_round_trips_group_order_and_records_in_its_own_state_file() {
    let mut game = Game::new();
    game.world.current_star = Some(StarId(7));
    game.world.data.game.game_clock.year_index = 2155;
    game.world.data.game.game_clock.month_index = 2;
    game.world.data.game.game_clock.day_index = 17;
    let initialize: fn(&mut uqm_rust::engine::World, u32) =
        uqm_rust::navigation::abi::init_group_info;
    initialize(&mut game.world, 1);
    {
        let mut fleets = game.world.avail_races();
        let id = fleets.alloc().unwrap();
        fleets.put(id);
    }
    for (number, point) in [(1, (-123, 456)), (2, (789, -321))] {
        let mut groups = game.world.ip_groups();
        let id = groups.alloc().unwrap();
        let group = groups.get_mut(id);
        group.group_id = number;
        group.group_counter = 0x1234;
        group.in_system = 1;
        group.task = 9;
        group.sys_loc = 2;
        group.dest_loc = 3;
        group.orbit_pos = 15;
        group.loc.x = point.0;
        group.loc.y = point.1;
        groups.put(id);
    }
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
    let mut site = Site::new(&owner, &io, StarId(7), 0, false, &mut pickup, None);
    site.save_ip_groups();
    assert_eq!(
        owner
            .borrow()
            .world
            .files
            .get_file(RANDGRPINFO_FILE)
            .unwrap()
            .length(),
        268 + 2 + 2 * 17
    );
    owner.borrow_mut().world.ip_groups().reinit();
    site.load_ip_groups();
    let cx = owner.borrow();
    let groups = cx.world.ip_groups_ref();
    assert_eq!(groups.count(), 2);
    let first = groups.head().unwrap();
    let second = groups.succ(first).unwrap();
    assert_eq!(groups.tail(), Some(second));
    for (id, number, point) in [(first, 1, (-123, 456)), (second, 2, (789, -321))] {
        let group = groups.get(id);
        assert_eq!(group.group_id, number);
        assert_eq!((group.loc.x, group.loc.y), point);
        assert_eq!(
            (group.group_counter, group.task, group.sys_loc),
            (0x1234, 9, 2)
        );
        assert_eq!(
            (group.dest_loc, group.orbit_pos, group.in_system),
            (3, 15, 1)
        );
    }
    assert_eq!(cx.world.npc_built_ships_ref().count(), 0);
    assert_eq!(cx.world.last_encounter_group, 0);
    let other = Game::new();
    assert_eq!(
        other
            .world
            .files
            .get_file(RANDGRPINFO_FILE)
            .unwrap()
            .length(),
        0
    );
}
