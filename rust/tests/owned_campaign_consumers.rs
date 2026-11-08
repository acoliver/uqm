use uqm_rust::app::bridge;
use uqm_rust::engine::{Game, World};

#[test]
#[serial_test::serial]
fn starbase_and_probe_setup_use_owned_campaign_storage() {
    let mut game = Game::new();
    {
        let mut fleets = game.world.avail_races();
        for species in 0..24 {
            let id = fleets.alloc().unwrap();
            let fleet = fleets.get_mut(id);
            fleet.species_id = species;
            fleet.allied_state = 1;
            fleet.crew_level = 12;
            fleet.max_crew = 18;
            fleet.max_energy = 20;
            fleets.put(id);
        }
    }
    // SAFETY: the serialized fixture owns its published game; queue operations do not enter UI.
    unsafe {
        bridge::with_game(&mut game, || {
            World::current().write(|world| {
                let fleets = world.avail_races_ref();
                assert_eq!(fleets.count(), 24);
                let template = *fleets.get(fleets.at(0).unwrap());
                let mut ships = world.built_ships();
                assert_eq!(ships.count(), 0);
                let mut captain = 0;
                let mut name = || {
                    captain += 1;
                    captain
                };
                for slot in [3, 7] {
                    let id = ships.clone_fragment(&template, 0, 1, &mut name).unwrap();
                    ships.get_mut(id).index = slot;
                }
                let first = ships.head().unwrap();
                let second = ships.succ(first).unwrap();
                assert_eq!([ships.get(first).index, ships.get(second).index], [3, 7]);
                for id in [first, second] {
                    assert_eq!(ships.get(id).crew_level, 1);
                    assert_eq!(ships.get(id).max_crew, 18);
                }
                ships.get_mut(first).crew_level = 5;
                assert_eq!(ships.get(first).crew_level, 5);
                ships.remove(first);
                ships.free(first);
                assert!(!ships.contains(first));
                for _ in 1..12 {
                    assert!(ships.clone_fragment(&template, 0, 1, &mut name).is_some());
                }
                assert_eq!(ships.count(), 12);
                assert!(ships.clone_fragment(&template, 0, 1, &mut name).is_none());
                assert_eq!(ships.count(), 12);
            });
            World::current().write(|world| {
                uqm_rust::comm::dispatch::prepare_automation_sol_probe_encounter(world).unwrap()
            });
            let first = World::current().read(|world| world.npc_built_ships_ref().head().unwrap());
            World::current().write(|world| {
                uqm_rust::comm::dispatch::prepare_automation_sol_probe_encounter(world).unwrap()
            });
            World::current().read(|world| {
                let ships = world.npc_built_ships_ref();
                assert!(!ships.contains(first));
                assert_eq!(ships.count(), 1);
                let probe = ships.get(ships.head().unwrap());
                assert_eq!(probe.race_id, 23);
                assert_eq!(probe.crew_level, 12);
                assert_eq!(probe.max_crew, 18);
            });
        });
    }
    assert_eq!(game.world.built_ships_ref().count(), 12);
}

#[test]
#[serial_test::serial]
fn clock_helpers_work_with_an_unpublished_world() {
    use uqm_rust::time::clock_bridge::{
        rust_clock_init, rust_clock_is_running, rust_clock_set_rate, rust_clock_tick,
        rust_clock_uninit,
    };
    let mut world = World::new();
    assert_ne!(rust_clock_init(&mut world), 0);
    assert!(world.clock.initialized());
    let calendar = &world.data.game.game_clock;
    assert_eq!(
        (
            calendar.year_index,
            calendar.month_index,
            calendar.day_index
        ),
        (2155, 2, 17)
    );
    assert_eq!(rust_clock_is_running(&world), 0);
    rust_clock_set_rate(&mut world, 2);
    assert_eq!(rust_clock_is_running(&world), 1);
    assert_eq!(world.data.game.game_clock.tick_count, 48);
    assert!(!rust_clock_tick(&mut world));
    rust_clock_set_rate(&mut world, 4);
    assert_eq!(world.data.game.game_clock.tick_count, 94);
    rust_clock_set_rate(&mut world, 0);
    assert_eq!(rust_clock_is_running(&world), 0);
    assert!(!rust_clock_tick(&mut world));
    rust_clock_set_rate(&mut world, 1);
    for _ in 0..23 {
        assert!(!rust_clock_tick(&mut world));
    }
    assert!(rust_clock_tick(&mut world));
    assert_eq!(world.data.game.game_clock.day_index, 18);
    let id = world.clock.events().alloc().unwrap();
    world.clock.events().put(id);
    assert_ne!(rust_clock_uninit(&mut world), 0);
    assert!(!world.clock.initialized());
    assert!(!world.clock.events_ref().contains(id));
    assert_eq!(world.data.game.game_clock.tick_count, 0);
    assert_eq!(world.data.game.game_clock.day_in_ticks, 0);
}

#[test]
#[serial_test::serial]
fn scene_star_lookup_uses_the_unpublished_world() {
    use uqm_rust::engine::world::{CStarDesc, CATALOG_LEN};
    let mut world = World::new();
    world.story = Some(uqm_rust::state::game_state::StoryState::new());
    // SAFETY: descriptors contain only integer fields; this fixture retains every catalog entry.
    let mut catalog: Box<[CStarDesc; CATALOG_LEN]> = Box::new(unsafe { std::mem::zeroed() });
    for star in catalog.iter_mut() {
        star.star_pt.y = i16::MAX;
    }
    catalog[0].star_pt.x = 1752;
    catalog[0].star_pt.y = 1450;
    world.set_catalog(std::borrow::Cow::Owned(*catalog));
    let result = uqm_rust::comm::dispatch::place_flagship_at_star(&mut world, (1752, 1450));
    assert_eq!(result, Ok(()));
    assert_eq!(world.star_pointer(), world.catalog().as_ptr().cast_mut());
}

#[test]
#[serial_test::serial]
fn rainbow_ordinal_reads_the_unpublished_world_star() {
    use uqm_rust::engine::world::{CStarDesc, CATALOG_LEN};
    let mut world = World::new();
    // SAFETY: catalog descriptors contain only integer fields.
    let mut catalog: Box<[CStarDesc; CATALOG_LEN]> = Box::new(unsafe { std::mem::zeroed() });
    catalog[0].index = 4;
    catalog[1].index = 9;
    world.set_catalog(std::borrow::Cow::Owned(*catalog));
    world.current_star = Some(uqm_rust::engine::StarId(2));
    let indexes = uqm_rust::planets::c_api::star_indexes_before_current(&world);
    assert_eq!(indexes, [4, 9]);
}
