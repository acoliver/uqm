//! Sol's actual ABI against real table queues and in-memory group files.

use super::Harness;
use uqm_rust::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
use uqm_rust::engine::StoryKey;
use uqm_rust::planets::c_api;
use uqm_rust::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;
use uqm_rust::planets::generate::special::Story;
use uqm_rust::state::game_state_keys::get_game_state;

pub fn check(h: &mut Harness) {
    let keys = [
        uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE,
        uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED,
        uqm_rust::engine::StoryKey::GLOBAL_FLAGS_AND_DATA,
    ];
    let story =
        keys.map(|key| uqm_rust::engine::World::current().read(|world| (world).story_state(key)));
    let word = uqm_rust::engine::World::current()
        .read(|world| world.word(StoryKey::URQUAN_PROBE_GRPOFFS0));
    let activity = uqm_rust::engine::World::current().read(|world| world.activity());
    unsafe {
        let saved = uqm_rust::engine::World::current().write(|world| {
            let saved = (
                std::mem::take(&mut world.queues),
                world.data.game.battle_group_ref,
            );
            let mut fleets = world.avail_races();
            for index in 0..24 {
                let id = fleets
                    .alloc()
                    .unwrap_or_else(|| panic!("fixture pool capacity"));
                let fleet = fleets.get_mut(id);
                fleet.species_id = index;
                fleet.crew_level = 12;
                fleet.max_crew = 12;
                fleet.max_energy = 20;
                fleets.put(id);
            }
            saved
        });
        assert_ne!(
            uqm_rust::engine::World::current().write(uqm_rust::time::clock_bridge::rust_clock_init),
            0
        );
        uqm_rust::engine::World::current().write(|world| c_api::init_group_info(world, 1));
        h.set_star(0, 0);
        h.state.sys.sun.num_planets = 9;
        let (saved_catalog, saved_star) =
            uqm_rust::engine::World::current().read(|world| (*world.catalog(), world.current_star));
        uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Borrowed(
                &uqm_rust::planets::tables::STARMAP_ARRAY,
            ));
            let sol = world
                .catalog()
                .iter()
                .position(|star| star.index == 1)
                .unwrap();
            world.current_star = Some(uqm_rust::engine::StarId(sol));
        });
        uqm_rust::engine::World::current()
            .write(|world| world.set_word(StoryKey::URQUAN_PROBE_GRPOFFS0, 0));
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 0)
        });
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED, 0)
        });
        assert!(crate::invoke_generation(&mut h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.init_npcs(system, host)
        }));
        let probe_ref =
            uqm_rust::engine::WorldData::current().read(|world| world.game.battle_group_ref);
        assert_ne!(probe_ref, 0);
        assert_eq!(
            uqm_rust::engine::World::current()
                .read(|world| world.word(StoryKey::URQUAN_PROBE_GRPOFFS0)),
            probe_ref
        );
        assert!(uqm_rust::engine::World::current()
            .read(|world| world.npc_built_ships_ref().head().is_none()));
        let group = uqm_rust::engine::World::current().read(|world| {
            let groups = world.ip_groups_ref();
            *groups.get(
                groups
                    .head()
                    .unwrap_or_else(|| panic!("fixture group missing")),
            )
        });
        assert_eq!(group.race_id, 23);
        assert_eq!((group.task, group.sys_loc, group.dest_loc), (0, 3, 3));
        assert_eq!((group.loc.x, group.loc.y, group.group_counter), (0, 0, 0));
        let file = || {
            uqm_rust::engine::World::current()
                .write(|world| {
                    uqm_rust::state::ffi::with_state_files(world, |files| {
                        files
                            .get_file(uqm_rust::engine::world::state_file::DEFGRPINFO_FILE)
                            .unwrap()
                            .contents()
                            .to_vec()
                    })
                })
                .unwrap()
        };
        let saved_file = file();
        assert!(saved_file.len() > probe_ref as usize);
        // Loading the saved reference does not create a second group record.
        assert!(crate::invoke_generation(&mut h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.init_npcs(system, host)
        }));
        assert_eq!(
            uqm_rust::engine::WorldData::current().read(|world| world.game.battle_group_ref),
            probe_ref
        );
        assert_eq!(file(), saved_file);
        uqm_rust::engine::World::current().write(|world| {
            let mut groups = world.ip_groups();
            let head = groups
                .head()
                .unwrap_or_else(|| panic!("fixture group missing"));
            groups.get_mut(head).task = uqm_rust::planets::generate::default::REFORM_GROUP;
        });
        assert!(crate::invoke_generation(&mut h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.uninit_npcs(system, host)
        }));
        assert!(uqm_rust::engine::World::current()
            .read(|world| world.npc_built_ships_ref().head().is_none()));
        assert!(crate::invoke_generation(&mut h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.reinit_npcs(system, host)
        }));
        assert_eq!(
            uqm_rust::engine::WorldData::current().read(|world| world.game.battle_group_ref),
            probe_ref
        );
        assert_eq!(
            uqm_rust::engine::World::current()
                .read(|world| world.word(StoryKey::URQUAN_PROBE_GRPOFFS0)),
            probe_ref
        );
        let probe = uqm_rust::engine::World::current().read(|world| {
            let groups = world.ip_groups_ref();
            *groups.get(
                groups
                    .head()
                    .unwrap_or_else(|| panic!("fixture group missing")),
            )
        });
        assert_eq!(
            probe.task,
            uqm_rust::planets::generate::default::FLEE
                | uqm_rust::planets::generate::default::IGNORE_FLAGSHIP
        );
        assert_eq!(probe.dest_loc, 0);
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED, 1)
        });
        assert!(crate::invoke_generation(&mut h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.init_npcs(system, host)
        }));
        assert_eq!(
            uqm_rust::engine::WorldData::current().read(|world| world.game.battle_group_ref),
            probe_ref
        );
        assert_eq!(file(), saved_file);

        let system = &raw mut *h.state;

        for activity in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
            uqm_rust::engine::World::current().write(|world| {
                world.set_activity(activity);
                world.encounter_group = 17;
                world.set_story_state(uqm_rust::engine::StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
            });
            assert!(crate::invoke_generation(&mut *system, |view, host| {
                GENERATE_SOL_FUNCTIONS.generate_orbital(
                    view,
                    uqm_rust::engine::world::generation::Body::Moon { planet: 2, moon: 0 },
                    host,
                )
            }));
            assert_eq!(
                uqm_rust::engine::World::current().read(|world| world.activity()),
                activity | START_ENCOUNTER
            );
            assert_eq!(
                uqm_rust::engine::World::current().read(|world| get_game_state(
                    world,
                    uqm_rust::engine::StoryKey::GLOBAL_FLAGS_AND_DATA
                )),
                255
            );
            assert!(
                uqm_rust::engine::World::current()
                    .read(|world| world.ip_groups_ref().head().is_none())
                    && uqm_rust::engine::World::current()
                        .read(|world| world.npc_built_ships_ref().head().is_none())
            );
            assert_eq!(
                uqm_rust::engine::World::current()
                    .read(|world| world.word(StoryKey::URQUAN_PROBE_GRPOFFS0)),
                probe_ref
            );
        }
        uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(saved_catalog));
            world.current_star = saved_star;
        });
        uqm_rust::engine::World::current().write(c_api::uninit_group_info);
        assert_ne!(
            uqm_rust::engine::World::current()
                .write(uqm_rust::time::clock_bridge::rust_clock_uninit),
            0
        );
        uqm_rust::engine::World::current().write(|world| {
            world.queues = saved.0;
            world.data.game.battle_group_ref = saved.1;
        });
    }
    for (key, value) in keys.into_iter().zip(story) {
        uqm_rust::engine::World::current().write(|world| (world).set_story_state(key, value));
    }
    uqm_rust::engine::World::current()
        .write(|world| world.set_word(StoryKey::URQUAN_PROBE_GRPOFFS0, word));
    uqm_rust::engine::World::current().write(|world| world.set_activity(activity));
    println!("Sol NPC ABI: real probe file creation/reload/persistence, queue init/uninit/reinit, starbase save/reset, abort/load bits retained");
}

use uqm_rust::planets::generate::SystemGenerator;
