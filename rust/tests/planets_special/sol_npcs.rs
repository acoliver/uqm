//! Sol's actual ABI against real table queues and in-memory group files.

use super::Harness;
use uqm_rust::c_bindings::global_state::IP_GROUP;
use uqm_rust::collections::queue::{AllocLink, InitQueue, PutQueue, Queue, UninitQueue};
use uqm_rust::comm::dispatch::{
    CFleetInfo, CShipFragment, CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER,
};
use uqm_rust::planets::c_api::{self, GLOB_DATA};
use uqm_rust::planets::generate::sol_exports::GENERATE_SOL_FUNCTIONS;
use uqm_rust::planets::generate::special::{LiveStory, Story};
use uqm_rust::state::game_state_keys::{get_game_state, set_game_state};

pub fn check(h: &mut Harness) {
    let keys = [
        "CHMMR_BOMB_STATE",
        "PROBE_MESSAGE_DELIVERED",
        "GLOBAL_FLAGS_AND_DATA",
    ];
    let story = keys.map(get_game_state);
    let word = LiveStory.word("URQUAN_PROBE_GRPOFFS0");
    let activity = uqm_rust::mainloop::ffi::get_current_activity();
    unsafe {
        let ip = &raw mut GLOB_DATA.Game_state.ip_group_q;
        let built = &raw mut GLOB_DATA.Game_state.npc_built_ship_q;
        let fleets = &raw mut GLOB_DATA.Game_state.avail_race_q;
        let reference = &raw mut GLOB_DATA.Game_state.BattleGroupRef;
        let saved = (ip.read(), built.read(), fleets.read(), reference.read());
        ip.write(Queue::default());
        built.write(Queue::default());
        fleets.write(Queue::default());
        assert!(InitQueue(
            ip.cast(),
            4,
            std::mem::size_of::<IP_GROUP>() as u16
        ));
        assert!(InitQueue(
            built.cast(),
            4,
            std::mem::size_of::<CShipFragment>() as u16
        ));
        assert!(InitQueue(
            fleets.cast(),
            24,
            std::mem::size_of::<CFleetInfo>() as u16
        ));
        for index in 0..24 {
            let handle = AllocLink(fleets.cast());
            assert!(!handle.is_null());
            let fleet = handle.cast::<CFleetInfo>();
            fleet.write(std::mem::zeroed());
            (*fleet).species_id = index;
            (*fleet).crew_level = 12;
            (*fleet).max_crew = 12;
            (*fleet).max_energy = 20;
            PutQueue(fleets.cast(), handle);
        }
        assert_ne!(uqm_rust::time::clock_bridge::rust_clock_init(), 0);
        c_api::InitGroupInfo(1);
        h.set_star(0, 0);
        h.state.SunDesc[0].NumPlanets = 9;
        let saved_star = super::CurStarDescPtr;
        let saved_catalog = std::ptr::addr_of!(c_api::STAR_ARRAY).read();
        // The ordinary game restart installs this same Rust catalog.
        std::ptr::addr_of_mut!(c_api::STAR_ARRAY).write(
            std::ptr::addr_of!(uqm_rust::planets::tables::STARMAP_ARRAY)
                .cast_mut()
                .cast(),
        );
        let catalog = std::slice::from_raw_parts(
            c_api::STAR_ARRAY,
            uqm_rust::planets::tables::STAR_CATALOG_LEN,
        );
        let sol = catalog.iter().position(|star| star.index == 1).unwrap();
        super::CurStarDescPtr = c_api::STAR_ARRAY.add(sol).cast();
        LiveStory.set_word("URQUAN_PROBE_GRPOFFS0", 0);
        set_game_state("CHMMR_BOMB_STATE", 0);
        set_game_state("PROBE_MESSAGE_DELIVERED", 0);
        assert!(GENERATE_SOL_FUNCTIONS.init_npcs.unwrap()(&raw mut *h.state));
        let probe_ref = reference.read();
        assert_ne!(probe_ref, 0);
        assert_eq!(LiveStory.word("URQUAN_PROBE_GRPOFFS0"), probe_ref);
        assert!(built.read().head.is_null());
        let group = ip.read().head.cast::<IP_GROUP>();
        assert!(!group.is_null());
        assert_eq!((*group).race_id, 23);
        assert_eq!(
            ((*group).task, (*group).sys_loc, (*group).dest_loc),
            (0, 3, 3)
        );
        assert_eq!(
            ((*group).loc.x, (*group).loc.y, (*group).group_counter),
            (0, 0, 0)
        );
        let file = || {
            uqm_rust::state::ffi::with_state_files(|files| {
                files
                    .get_file(uqm_rust::state::state_file::DEFGRPINFO_FILE)
                    .unwrap()
                    .contents()
                    .to_vec()
            })
            .unwrap()
        };
        let saved_file = file();
        assert!(saved_file.len() > probe_ref as usize);
        // Loading the saved reference does not create a second group record.
        assert!(GENERATE_SOL_FUNCTIONS.init_npcs.unwrap()(&raw mut *h.state));
        assert_eq!(reference.read(), probe_ref);
        assert_eq!(file(), saved_file);
        let probe = ip.read().head.cast::<IP_GROUP>();
        (*probe).task = uqm_rust::planets::generate::default::REFORM_GROUP;
        assert!(GENERATE_SOL_FUNCTIONS.uninit_npcs.unwrap()(
            &raw mut *h.state
        ));
        assert!(built.read().head.is_null());
        assert!(GENERATE_SOL_FUNCTIONS.reinit_npcs.unwrap()(
            &raw mut *h.state
        ));
        assert_eq!(reference.read(), probe_ref);
        assert_eq!(LiveStory.word("URQUAN_PROBE_GRPOFFS0"), probe_ref);
        let probe = ip.read().head.cast::<IP_GROUP>();
        assert!(!probe.is_null());
        assert_eq!(
            (*probe).task,
            uqm_rust::planets::generate::default::FLEE
                | uqm_rust::planets::generate::default::IGNORE_FLAGSHIP
        );
        assert_eq!((*probe).dest_loc, 0);
        set_game_state("PROBE_MESSAGE_DELIVERED", 1);
        assert!(GENERATE_SOL_FUNCTIONS.init_npcs.unwrap()(&raw mut *h.state));
        assert_eq!(reference.read(), probe_ref);
        assert_eq!(file(), saved_file);

        let system = &raw mut *h.state;
        let parent = &raw mut (*system).PlanetDesc[2];
        (*parent).pPrevDesc = &raw mut (*system).SunDesc[0];
        let starbase = &raw mut (*system).MoonDesc[0];
        (*starbase).pPrevDesc = parent;
        for activity in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
            uqm_rust::mainloop::ffi::set_current_activity(
                uqm_rust::mainloop::types::ActivityValue(activity),
            );
            c_api::set_encounter_group(17);
            set_game_state("GLOBAL_FLAGS_AND_DATA", 0);
            assert!(GENERATE_SOL_FUNCTIONS.generate_orbital.unwrap()(
                system, starbase
            ));
            assert_eq!(
                uqm_rust::mainloop::ffi::get_current_activity().0,
                activity | START_ENCOUNTER
            );
            assert_eq!(get_game_state("GLOBAL_FLAGS_AND_DATA"), 255);
            assert!(ip.read().head.is_null() && built.read().head.is_null());
            assert_eq!(LiveStory.word("URQUAN_PROBE_GRPOFFS0"), probe_ref);
        }
        super::CurStarDescPtr = saved_star;
        std::ptr::addr_of_mut!(c_api::STAR_ARRAY).write(saved_catalog);
        c_api::UninitGroupInfo();
        assert_ne!(uqm_rust::time::clock_bridge::rust_clock_uninit(), 0);
        UninitQueue(ip.cast::<Queue>());
        UninitQueue(built.cast::<Queue>());
        UninitQueue(fleets.cast::<Queue>());
        ip.write(saved.0);
        built.write(saved.1);
        fleets.write(saved.2);
        reference.write(saved.3);
    }
    for (key, value) in keys.into_iter().zip(story) {
        set_game_state(key, value);
    }
    LiveStory.set_word("URQUAN_PROBE_GRPOFFS0", word);
    uqm_rust::mainloop::ffi::set_current_activity(activity);
    println!("Sol NPC ABI: real probe file creation/reload/persistence, queue init/uninit/reinit, starbase save/reset, abort/load bits retained");
}
