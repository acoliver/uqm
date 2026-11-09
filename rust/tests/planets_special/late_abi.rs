//! Headless checks of the selected production tables and safely callable effects.

use super::Harness;
use uqm_rust::battle::gendef::*;
use uqm_rust::planets::generate::myc::Generator as GENERATE_MYCON_FUNCTIONS;
use uqm_rust::planets::generate::sam::Generator as GENERATE_SAMATRA_FUNCTIONS;
use uqm_rust::planets::generate::spa::Generator as GENERATE_SPATHI_FUNCTIONS;
use uqm_rust::planets::generate::surface::BIOLOGICAL_SCAN;
use uqm_rust::planets::generate::utw::Generator as GENERATE_UTWIG_FUNCTIONS;
use uqm_rust::planets::generate::vux::Generator as GENERATE_VUX_FUNCTIONS;
use uqm_rust::planets::generate::{generator, SystemGenerator};
use uqm_rust::state::game_state_keys::get_game_state;

pub fn check() {
    for (indexes, table, _overrides) in [
        (
            &[UTWIG_DEFINED, BOMB_DEFINED][..],
            (&GENERATE_UTWIG_FUNCTIONS as &dyn SystemGenerator),
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
            (&GENERATE_MYCON_FUNCTIONS as &dyn SystemGenerator),
            &[3, 6, 8, 9, 11][..],
        ),
        (
            &[VUX_DEFINED, MAIDENS_DEFINED, VUX_BEAST_DEFINED][..],
            (&GENERATE_VUX_FUNCTIONS as &dyn SystemGenerator),
            &[3, 6, 8, 9, 11, 12][..],
        ),
        (
            &[SAMATRA_DEFINED][..],
            (&GENERATE_SAMATRA_FUNCTIONS as &dyn SystemGenerator),
            &[0, 1, 3, 4, 6][..],
        ),
        (
            &[SPATHI_DEFINED][..],
            (&GENERATE_SPATHI_FUNCTIONS as &dyn SystemGenerator),
            &[3, 4, 6, 8, 9, 11, 12][..],
        ),
    ] {
        let mut routed = Harness::new();
        for &index in indexes {
            routed.star_index = index;
            for seed in [0, 1, 104729, u32::MAX] {
                assert_eq!(
                    routed.c_planets(generator(index), (0, 0, seed)),
                    routed.c_planets(table, (0, 0, seed)),
                    "selector index {index}"
                );
            }
        }
    }

    let mut harness = Harness::new();
    harness.star_index = SPATHI_DEFINED;
    harness.set_star(0, 0);
    harness.seed(104729);

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
        harness.state.sys.info.planet_info.scan_retrieve_mask[BIOLOGICAL_SCAN] = if retrieved == 32
        {
            u32::MAX
        } else {
            (1_u32 << retrieved) - 1
        };
        let before = format!("{:?}", crate::generation_data(&harness.state));
        assert!(unsafe {
            crate::invoke_generation(&mut harness.state, |view, host| {
                GENERATE_SPATHI_FUNCTIONS.pickup_life(
                    view,
                    uqm_rust::planets::generate::special::Body::Planet(0),
                    0,
                    host,
                )
            })
        });
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
        assert_eq!(
            format!("{:?}", crate::generation_data(&harness.state)),
            before
        );
        assert_eq!(harness.rng_position(), 104729);
    }

    uqm_rust::engine::World::current().write(|world| {
        (world).set_story_state(uqm_rust::engine::StoryKey::SPATHI_CREATURES_EXAMINED, 0)
    });
    assert!(unsafe {
        crate::invoke_generation(&mut harness.state, |view, host| {
            GENERATE_SPATHI_FUNCTIONS.pickup_life(
                view,
                uqm_rust::planets::generate::special::Body::Planet(1),
                u16::MAX,
                host,
            )
        })
    });
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
            crate::invoke_generation(&mut harness.state, |view, host| {
                GENERATE_MYCON_FUNCTIONS.generate_life(
                    view,
                    uqm_rust::planets::generate::special::Body::Planet(0),
                    u16::MAX,
                    &mut crate::planets_common::fresh_node(),
                    host,
                )
            })
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
    use uqm_rust::planets::generate::sam::GuardSite;
    extern "C" {
        static mut ScreenHeight: i32;
    }
    let (saved_queues, ids, first) = World::current().write(|world| {
        let saved = std::mem::take(&mut world.queues);
        let mut groups = world.ip_groups();
        let ids = std::array::from_fn::<_, 3, _>(|i| {
            let id = groups
                .alloc()
                .unwrap_or_else(|| panic!("fixture pool capacity"));
            groups.get_mut(id).group_id = i as u8 + 1;
            groups.get_mut(id).task = 8;
            groups.put(id);
            id
        });
        (saved, ids, groups.raw(ids[0]))
    });
    unsafe {
        let mut visited = Vec::new();
        crate::invoke_generation(&mut harness.state, |_, host| {
            GuardSite::for_each_ip_group(host, &mut |group| {
                visited.push(group.group_id);
                group.task &= !8;
            });
        });
        assert!(World::current().read(|world| ids.iter().all(|id| world
            .ip_groups_ref()
            .get(*id)
            .task
            == 0)));
        assert_eq!(visited, [1, 2, 3]);
        harness.star_index = BOMB_DEFINED;
        harness.set_star(0, 0);
        World::current()
            .write(|world| world.set_story_state(uqm_rust::engine::StoryKey::UTWIG_BOMB, 0));
        let before = format!("{:?}", crate::generation_data(&harness.state));
        assert!(crate::invoke_generation(
            &mut harness.state,
            |system, host| GENERATE_UTWIG_FUNCTIONS.init_npcs(system, host)
        ));
        World::current().write(|world| {
            let mut groups = world.ip_groups();
            assert!(groups.head().is_none() && groups.tail().is_none());
            assert!(ids.iter().all(|id| !groups.contains(*id)));
            let reused = groups
                .alloc()
                .unwrap_or_else(|| panic!("fixture pool capacity"));
            assert_eq!(groups.raw(reused), first);
            assert_ne!(reused, ids[0]);
            groups.free(reused);
        });
        assert_eq!(
            format!("{:?}", crate::generation_data(&harness.state)),
            before
        );
        assert_eq!(
            World::current()
                .read(|world| get_game_state(world, uqm_rust::engine::StoryKey::UTWIG_BOMB)),
            0
        );
        assert_eq!(harness.rng_position(), 104729);
        World::current().write(|world| world.queues = saved_queues);

        use uqm_rust::c_bindings::global_state::{EncounterGroup, POINT};
        let saved_race = World::current().read(|w| w.encounter_race);
        let saved_group = std::ptr::addr_of!(EncounterGroup).read();
        World::current().write(|w| w.encounter_race = 18);
        std::ptr::addr_of_mut!(EncounterGroup).write(7);
        crate::invoke_generation(&mut harness.state, |_, host| {
            GuardSite::reset_encounter(host)
        });
        assert_eq!(World::current().read(|w| w.encounter_race), -1);
        assert_eq!(std::ptr::addr_of!(EncounterGroup).read(), 0);
        World::current().write(|w| w.encounter_race = saved_race);
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
        let saved_sun = harness.state.sys.sun;
        let saved_planet = harness.state.sys.planets[4];
        // units.h and solarsys.c: SIS height = screen height - 13;
        // DISPLAY_TO_LOC = ((SIS width >> 1) - 8) >> 1.
        let half_width = i32::from(uqm_rust::planets::generate::consts::MAP_WIDTH) >> 1;
        let half_height = (ScreenHeight - 13) >> 1;
        let display_to_loc = (half_width - 8) >> 1;
        harness.state.sys.sun.radius = (display_to_loc * 10) as i16;
        harness.state.sys.planets[4].image.origin = POINT {
            x: (half_width + 12) as i16,
            y: (half_height - 9) as i16,
        };
        let outer = crate::invoke_generation(&mut harness.state, |_, host| {
            GuardSite::planet_outer_location(host, 4)
        });
        assert_eq!((outer.x, outer.y), (120, -90));
        crate::invoke_generation(&mut harness.state, |_, host| {
            GuardSite::set_and_transform_ip_location(host, outer)
        });
        let location =
            crate::invoke_generation(&mut harness.state, |_, host| GuardSite::ip_location(host));
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
        harness.state.sys.sun = saved_sun;
        harness.state.sys.planets[4] = saved_planet;
        println!("live adapter checks: guard traversal, bomb queue reset, encounter reset and coordinate ABI");
    }
}
