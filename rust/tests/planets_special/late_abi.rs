//! Headless checks of the selected production tables and safely callable effects.

use super::Harness;
use uqm_rust::battle::gendef::*;
use uqm_rust::planets::generate::late_exports::*;
use uqm_rust::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};
use uqm_rust::planets::surface::BIOLOGICAL_SCAN;
use uqm_rust::state::game_state_keys::{get_game_state, set_game_state};

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
        set_game_state("SPATHI_CREATURES_ELIMINATED", 0);
        set_game_state("SPATHI_SHIELDED_SELVES", 0);
        set_game_state("SPATHI_CREATURES_EXAMINED", 0);
        harness.state.SysInfo.PlanetInfo.ScanRetrieveMask[BIOLOGICAL_SCAN] = if retrieved == 32 {
            u32::MAX
        } else {
            (1_u32 << retrieved) - 1
        };
        let before = format!("{:?}", harness.state);
        assert!(unsafe { pickup(&mut *harness.state, world, 0) });
        assert_eq!(get_game_state("SPATHI_CREATURES_EXAMINED"), 1);
        assert_eq!(
            get_game_state("SPATHI_CREATURES_ELIMINATED"),
            u32::from(retrieved == 31)
        );
        assert_eq!(format!("{:?}", harness.state), before);
        assert_eq!(harness.rng_position(), 104729);
    }
    harness.state.PlanetDesc[1].pPrevDesc = sun;
    let other = std::ptr::addr_of_mut!(harness.state.PlanetDesc[1]);
    set_game_state("SPATHI_CREATURES_EXAMINED", 0);
    assert!(unsafe { pickup(&mut *harness.state, other, u16::MAX) });
    assert_eq!(get_game_state("SPATHI_CREATURES_EXAMINED"), 0);

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
    use uqm_rust::c_bindings::global_state::IP_GROUP;
    use uqm_rust::collections::queue::Queue;
    use uqm_rust::planets::c_api::LiveGame;
    use uqm_rust::planets::c_api::GLOB_DATA as GlobData;
    use uqm_rust::planets::generate::sam::GuardSite;
    extern "C" {
        static mut ScreenHeight: i32;
    }
    let mut groups: [IP_GROUP; 3] = unsafe { std::mem::zeroed() };
    let first = groups.as_mut_ptr();
    for (i, group) in groups.iter_mut().enumerate() {
        group.group_id = i as u8 + 1;
        group.task = 8;
        group.succ = if i == 2 {
            std::ptr::null_mut()
        } else {
            unsafe { first.add(i + 1).cast() }
        };
    }
    let queue = Queue {
        head: first.cast(),
        tail: unsafe { first.add(2).cast() },
        pq_tab: first.cast(),
        free_list: std::ptr::null_mut(),
        object_size: std::mem::size_of::<IP_GROUP>() as u16,
        num_objects: 3,
    };
    // No references into GlobData or the group array cross either callback.
    unsafe {
        let ip = std::ptr::addr_of_mut!(GlobData.Game_state.ip_group_q);
        let npc = std::ptr::addr_of_mut!(GlobData.Game_state.npc_built_ship_q);
        let saved_ip = ip.read();
        let saved_npc = npc.read();
        ip.write(queue);
        npc.write(Queue::default());
        let mut visited = Vec::new();
        GuardSite::for_each_ip_group(&mut LiveGame, &mut |group| {
            visited.push(group.group_id);
            group.task &= !8;
        });
        assert_eq!(visited, [1, 2, 3]);
        assert!(groups.iter().all(|group| group.task == 0));
        harness.star_index = BOMB_DEFINED;
        harness.set_star(0, 0);
        set_game_state("UTWIG_BOMB", 0);
        let before = format!("{:?}", harness.state);
        assert!(GENERATE_UTWIG_FUNCTIONS.init_npcs.unwrap()(
            &mut *harness.state
        ));
        assert!(ip.read().head.is_null() && ip.read().tail.is_null());
        assert_eq!(ip.read().free_list, first.cast());
        assert_eq!(groups[0].succ, first.add(1).cast());
        assert_eq!(groups[1].succ, first.add(2).cast());
        assert!(groups[2].succ.is_null());
        assert_eq!(format!("{:?}", harness.state), before);
        assert_eq!(get_game_state("UTWIG_BOMB"), 0);
        assert_eq!(harness.rng_position(), 104729);
        ip.write(saved_ip);
        npc.write(saved_npc);

        use uqm_rust::c_bindings::global_state::{EncounterGroup, EncounterRace, POINT};
        let saved_race = std::ptr::addr_of!(EncounterRace).read();
        let saved_group = std::ptr::addr_of!(EncounterGroup).read();
        std::ptr::addr_of_mut!(EncounterRace).write(18);
        std::ptr::addr_of_mut!(EncounterGroup).write(7);
        GuardSite::reset_encounter(&mut LiveGame);
        assert_eq!(std::ptr::addr_of!(EncounterRace).read(), -1);
        assert_eq!(std::ptr::addr_of!(EncounterGroup).read(), 0);
        std::ptr::addr_of_mut!(EncounterRace).write(saved_race);
        std::ptr::addr_of_mut!(EncounterGroup).write(saved_group);

        let saved_location = std::ptr::addr_of!(GlobData.Game_state.ip_location).read();
        let saved_origin = std::ptr::addr_of!(GlobData.Game_state.ShipStamp.origin).read();
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
        let outer = GuardSite::planet_outer_location(&LiveGame, 4);
        assert_eq!((outer.x, outer.y), (120, -90));
        GuardSite::set_and_transform_ip_location(&mut LiveGame, outer);
        let location = GuardSite::ip_location(&LiveGame);
        assert_eq!((location.x, location.y), (120, -90));
        let origin = std::ptr::addr_of!(GlobData.Game_state.ShipStamp.origin).read();
        assert_eq!(
            (i32::from(origin.x), i32::from(origin.y)),
            (half_width + 12, half_height - 9)
        );
        std::ptr::addr_of_mut!(GlobData.Game_state.ip_location).write(saved_location);
        std::ptr::addr_of_mut!(GlobData.Game_state.ShipStamp.origin).write(saved_origin);
        harness.state.SunDesc[0] = saved_sun;
        harness.state.PlanetDesc[4] = saved_planet;
        println!("live adapter checks: guard traversal, bomb queue reset, encounter reset and coordinate ABI");
    }
}
