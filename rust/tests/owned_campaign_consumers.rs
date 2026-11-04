use uqm_rust::app::bridge;
use uqm_rust::engine::{Game, World};
use uqm_rust::starbase::cffi::CffiPlatform;
use uqm_rust::starbase::platform::{Platform, StarbaseError};

#[test]
#[serial_test::serial]
fn starbase_and_probe_setup_use_owned_campaign_storage() {
    let mut game = Game::new();
    {
        let mut fleets = game.world.avail_races();
        for species in 0..24 {
            let id = fleets.alloc().unwrap();
            let fleet = fleets.get_mut(id);
            fleet.SpeciesID = species;
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
            let platform = CffiPlatform;
            assert_eq!(platform.race_templates().unwrap().len(), 24);
            assert!(platform.escorts().unwrap().is_empty());
            assert!(platform.add_escort(0, 7).unwrap());
            assert!(platform.add_escort(0, 3).unwrap());
            assert!(matches!(
                platform.add_escort(0, 3),
                Err(StarbaseError::EscortQueue(_))
            ));
            let escorts = platform.escorts().unwrap();
            assert_eq!(
                escorts.iter().map(|ship| ship.slot).collect::<Vec<_>>(),
                [3, 7]
            );
            assert!(escorts
                .iter()
                .all(|ship| ship.crew_level == 1 && ship.max_crew == 18));
            platform.set_escort_crew(3, 5).unwrap();
            assert_eq!(platform.escorts().unwrap()[0].crew_level, 5);
            let removed = World::current().read(|world| world.built_ships_ref().head().unwrap());
            platform.remove_escort(3).unwrap();
            assert!(!World::current().read(|world| world.built_ships_ref().contains(removed)));
            assert!(matches!(
                platform.remove_escort(3),
                Err(StarbaseError::EscortSlot(3))
            ));
            for slot in 0..12 {
                if slot != 7 {
                    assert!(platform.add_escort(0, slot).unwrap());
                }
            }
            assert_eq!(platform.escorts().unwrap().len(), 12);
            assert!(!platform.add_escort(0, 12).unwrap());
            assert_eq!(platform.escorts().unwrap().len(), 12);
            uqm_rust::comm::dispatch::prepare_automation_sol_probe_encounter().unwrap();
            let first = World::current().read(|world| world.npc_built_ships_ref().head().unwrap());
            uqm_rust::comm::dispatch::prepare_automation_sol_probe_encounter().unwrap();
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
