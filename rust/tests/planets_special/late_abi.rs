//! Headless checks of the selected production tables and safely callable effects.

use super::Harness;
use uqm_rust::battle::gendef::*;
use uqm_rust::planets::generate::late_exports::*;
use uqm_rust::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};
use uqm_rust::planets::surface::BIOLOGICAL_SCAN;
use uqm_rust::state::game_state_keys::get_game_state;

fn slots(table: &GenerateFunctions) -> [usize; 13] {
    [
        table.init_npcs.unwrap() as usize,
        table.reinit_npcs.unwrap() as usize,
        table.uninit_npcs.unwrap() as usize,
        table.generate_planets.unwrap() as usize,
        table.generate_moons.unwrap() as usize,
        table.generate_name.unwrap() as usize,
        table.generate_orbital.unwrap() as usize,
        table.generate_minerals.unwrap() as usize,
        table.generate_energy.unwrap() as usize,
        table.generate_life.unwrap() as usize,
        table.pickup_minerals.unwrap() as usize,
        table.pickup_energy.unwrap() as usize,
        table.pickup_life.unwrap() as usize,
    ]
}

pub fn check() {
    for (indexes, table, overrides) in [
        (
            &[UTWIG_DEFINED, BOMB_DEFINED][..],
            &GENERATE_UTWIG_FUNCTIONS,
            &[0, 3, 6, 8, 11][..],
        ),
        (
            &[
                MYCON_DEFINED,
                SUN_DEVICE_DEFINED,
                EGG_CASE0_DEFINED,
                EGG_CASE1_DEFINED,
                EGG_CASE2_DEFINED,
            ][..],
            &GENERATE_MYCON_FUNCTIONS,
            &[3, 6, 8, 9, 11][..],
        ),
        (
            &[VUX_DEFINED, MAIDENS_DEFINED, VUX_BEAST_DEFINED][..],
            &GENERATE_VUX_FUNCTIONS,
            &[3, 6, 8, 9, 11, 12][..],
        ),
        (
            &[SAMATRA_DEFINED][..],
            &GENERATE_SAMATRA_FUNCTIONS,
            &[0, 1, 3, 4, 6][..],
        ),
        (
            &[SPATHI_DEFINED][..],
            &GENERATE_SPATHI_FUNCTIONS,
            &[3, 4, 6, 8, 9, 11, 12][..],
        ),
    ] {
        let actual = slots(table);
        let defaults = slots(&DEFAULT_HOOKS);
        for &index in indexes {
            assert_eq!(
                getGenerateFunctions(index),
                std::ptr::from_ref(table).cast()
            );
        }
        for slot in 0..13 {
            if !overrides.contains(&slot) {
                assert_eq!(actual[slot], defaults[slot], "default slot {slot}");
            }
        }
    }

    let mut harness = Harness::new();
    harness.star_index = SPATHI_DEFINED;
    harness.set_star(0, 0);
    harness.seed(104729);
    let sun = std::ptr::addr_of_mut!(harness.state.SunDesc[0]);
    harness.state.PlanetDesc[0].pPrevDesc = sun;
    let world = std::ptr::addr_of_mut!(harness.state.PlanetDesc[0]);
    let pickup = GENERATE_SPATHI_FUNCTIONS.pickup_life.unwrap();
    for retrieved in 0..=32 {
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::SPATHI_CREATURES_ELIMINATED, 0)
        });
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::SPATHI_SHIELDED_SELVES, 0)
        });
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::SPATHI_CREATURES_EXAMINED, 0)
        });
        harness.state.SysInfo.PlanetInfo.ScanRetrieveMask[BIOLOGICAL_SCAN] = if retrieved == 32 {
            u32::MAX
        } else {
            (1_u32 << retrieved) - 1
        };
        let before = format!("{:?}", harness.state);
        assert!(unsafe { pickup(&mut *harness.state, world, 0) });
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| get_game_state(
                world,
                uqm_rust::engine::StoryKey::SPATHI_CREATURES_EXAMINED
            )),
            1
        );
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| get_game_state(
                world,
                uqm_rust::engine::StoryKey::SPATHI_CREATURES_ELIMINATED
            )),
            u32::from(retrieved == 31)
        );
        assert_eq!(format!("{:?}", harness.state), before);
        assert_eq!(harness.rng_position(), 104729);
    }
    harness.state.PlanetDesc[1].pPrevDesc = sun;
    let other = std::ptr::addr_of_mut!(harness.state.PlanetDesc[1]);
    uqm_rust::engine::World::current().write(|world| {
        (world).set_story_state(uqm_rust::engine::StoryKey::SPATHI_CREATURES_EXAMINED, 0)
    });
    assert!(unsafe { pickup(&mut *harness.state, other, u16::MAX) });
    assert_eq!(
        uqm_rust::engine::World::current().read(|world| get_game_state(
            world,
            uqm_rust::engine::StoryKey::SPATHI_CREATURES_EXAMINED
        )),
        0
    );

    // Mycon's life callback does not read any pointer or consume RNG.
    assert_eq!(
        unsafe {
            GENERATE_MYCON_FUNCTIONS.generate_life.unwrap()(
                std::ptr::null(),
                std::ptr::null(),
                u16::MAX,
                std::ptr::null_mut(),
            )
        },
        0
    );
    assert_eq!(harness.rng_position(), 104729);
    check_live_queues(&mut harness);
    println!(
        "production routing: 12 star indexes, 65 populated slots; Spathi pickup: 34 live ABI cases"
    );
}

fn check_live_queues(harness: &mut Harness) {
    use uqm_rust::engine::World;
    use uqm_rust::planets::c_api::NpcWorld;
    use uqm_rust::planets::generate::sam::GuardSite;
    extern "C" {
        static mut ScreenHeight: i32;
    }
    let (saved_queues, ids, first) = World::current().write(|world| {
        let saved = std::mem::take(&mut world.queues);
        let mut groups = world.ip_groups();
        let ids = std::array::from_fn::<_, 3, _>(|i| {
            let id = groups.alloc().unwrap();
            groups.get_mut(id).group_id = i as u8 + 1;
            groups.get_mut(id).task = 8;
            groups.put(id);
            id
        });
        (saved, ids, groups.raw(ids[0]))
    });
    unsafe {
        let mut visited = Vec::new();
        World::current().write(|world| {
            GuardSite::for_each_ip_group(&mut NpcWorld::new(world), &mut |group| {
                visited.push(group.group_id);
                group.task &= !8;
            });
            assert!(ids
                .iter()
                .all(|id| world.ip_groups_ref().get(*id).task == 0));
        });
        assert_eq!(visited, [1, 2, 3]);
        harness.star_index = BOMB_DEFINED;
        harness.set_star(0, 0);
        World::current()
            .write(|world| world.set_story_state(uqm_rust::engine::StoryKey::UTWIG_BOMB, 0));
        let before = format!("{:?}", harness.state);
        assert!(
            World::current().write(|world| GENERATE_UTWIG_FUNCTIONS.init_npcs.unwrap()(
                world,
                &mut *harness.state
            ))
        );
        World::current().write(|world| {
            let mut groups = world.ip_groups();
            assert!(groups.head().is_none() && groups.tail().is_none());
            assert!(ids.iter().all(|id| !groups.contains(*id)));
            let reused = groups.alloc().unwrap();
            assert_eq!(groups.raw(reused), first);
            assert_ne!(reused, ids[0]);
            groups.free(reused);
        });
        assert_eq!(format!("{:?}", harness.state), before);
        assert_eq!(
            World::current()
                .read(|world| get_game_state(world, uqm_rust::engine::StoryKey::UTWIG_BOMB)),
            0
        );
        assert_eq!(harness.rng_position(), 104729);
        World::current().write(|world| world.queues = saved_queues);

        use uqm_rust::c_bindings::global_state::{EncounterGroup, EncounterRace, POINT};
        let saved_race = std::ptr::addr_of!(EncounterRace).read();
        let saved_group = std::ptr::addr_of!(EncounterGroup).read();
        std::ptr::addr_of_mut!(EncounterRace).write(18);
        std::ptr::addr_of_mut!(EncounterGroup).write(7);
        World::current().write(|world| GuardSite::reset_encounter(&mut NpcWorld::new(world)));
        assert_eq!(std::ptr::addr_of!(EncounterRace).read(), -1);
        assert_eq!(std::ptr::addr_of!(EncounterGroup).read(), 0);
        std::ptr::addr_of_mut!(EncounterRace).write(saved_race);
        std::ptr::addr_of_mut!(EncounterGroup).write(saved_group);

        let saved_location =
            std::ptr::addr_of!((**uqm_rust::engine::WorldData::current()).game.ip_location).read();
        let saved_origin = std::ptr::addr_of!(
            (**uqm_rust::engine::WorldData::current())
                .game
                .ship_stamp
                .origin
        )
        .read();
        let saved_sun = harness.state.SunDesc[0];
        let saved_planet = harness.state.PlanetDesc[4];
        // units.h and solarsys.c: SIS height = screen height - 13;
        // DISPLAY_TO_LOC = ((SIS width >> 1) - 8) >> 1.
        let half_width = i32::from(uqm_rust::planets::consts::MAP_WIDTH) >> 1;
        let half_height = (ScreenHeight - 13) >> 1;
        let display_to_loc = (half_width - 8) >> 1;
        harness.state.SunDesc[0].radius = (display_to_loc * 10) as i16;
        harness.state.PlanetDesc[4].image.origin = POINT {
            x: (half_width + 12) as i16,
            y: (half_height - 9) as i16,
        };
        let outer = World::current()
            .write(|world| GuardSite::planet_outer_location(&NpcWorld::new(world), 4));
        assert_eq!((outer.x, outer.y), (120, -90));
        World::current().write(|world| {
            GuardSite::set_and_transform_ip_location(&mut NpcWorld::new(world), outer)
        });
        let location =
            World::current().write(|world| GuardSite::ip_location(&NpcWorld::new(world)));
        assert_eq!((location.x, location.y), (120, -90));
        let origin = std::ptr::addr_of!(
            (**uqm_rust::engine::WorldData::current())
                .game
                .ship_stamp
                .origin
        )
        .read();
        assert_eq!(
            (i32::from(origin.x), i32::from(origin.y)),
            (half_width + 12, half_height - 9)
        );
        std::ptr::addr_of_mut!((**uqm_rust::engine::WorldData::current()).game.ip_location)
            .write(saved_location);
        std::ptr::addr_of_mut!(
            (**uqm_rust::engine::WorldData::current())
                .game
                .ship_stamp
                .origin
        )
        .write(saved_origin);
        harness.state.SunDesc[0] = saved_sun;
        harness.state.PlanetDesc[4] = saved_planet;
        println!("live adapter checks: guard traversal, bomb queue reset, encounter reset and coordinate ABI");
    }
}
