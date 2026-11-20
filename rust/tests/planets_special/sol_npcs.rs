//! Sol's actual ABI against real table queues and in-memory group files.

use super::Harness;
use uqm_rust::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
use uqm_rust::engine::world::queues::IpGroup;
use uqm_rust::engine::StoryKey;
use uqm_rust::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;
use uqm_rust::planets::generate::special::Story;
use uqm_rust::state::game_state_keys::get_game_state;

pub fn check(h: &mut Harness) {
    let keys = [
        uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE,
        uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED,
        uqm_rust::engine::StoryKey::GLOBAL_FLAGS_AND_DATA,
    ];
    let story = keys.map(|key| crate::battle_context::with(|cx| cx.world.story_state(key)));
    let word = crate::battle_context::with(|cx| cx.world.word(StoryKey::URQUAN_PROBE_GRPOFFS0));
    let activity = crate::battle_context::with(|cx| cx.world.activity());
    let saved = crate::battle_context::with(|cx| {
        let saved = (
            std::mem::take(&mut cx.world.queues),
            cx.world.data.game.battle_group_ref,
        );
        let mut fleets = cx.world.avail_races();
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
        crate::battle_context::with(|cx| uqm_rust::time::clock_bridge::rust_clock_init(cx.world)),
        0
    );
    crate::battle_context::with(|cx| uqm_rust::navigation::groups::init_group_info(cx.world, 1));
    h.set_star(0, 0);
    h.state.sys.sun.num_planets = 9;
    let (saved_catalog, saved_star) =
        crate::battle_context::with(|cx| (*cx.world.catalog(), cx.world.current_star));
    crate::battle_context::with(|cx| {
        cx.world.set_catalog(std::borrow::Cow::Borrowed(
            &uqm_rust::planets::tables::STARMAP_ARRAY,
        ));
        let sol = cx
            .world
            .catalog()
            .iter()
            .position(|star| star.index == 1)
            .unwrap();
        cx.world.current_star = Some(uqm_rust::engine::StarId(sol));
    });
    crate::battle_context::with(|cx| cx.world.set_word(StoryKey::URQUAN_PROBE_GRPOFFS0, 0));
    crate::battle_context::with(|cx| {
        cx.world
            .set_story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 0)
    });
    crate::battle_context::with(|cx| {
        cx.world
            .set_story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED, 0)
    });
    assert!(crate::invoke_generation(&mut h.state, |system, host| {
        GENERATE_SOL_FUNCTIONS.init_npcs(system, host)
    }));
    let probe_ref = crate::battle_context::with(|cx| cx.world.data.game.battle_group_ref);
    assert_ne!(probe_ref, 0);
    assert_eq!(
        crate::battle_context::with(|cx| cx.world.word(StoryKey::URQUAN_PROBE_GRPOFFS0)),
        probe_ref
    );
    assert!(crate::battle_context::with(|cx| cx
        .world
        .npc_built_ships_ref()
        .head()
        .is_none()));
    let group = crate::battle_context::with(|cx| {
        let groups = cx.world.ip_groups_ref();
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
        crate::battle_context::with(|cx| {
            uqm_rust::state::ffi::with_state_files(cx.world, |files| {
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
        crate::battle_context::with(|cx| cx.world.data.game.battle_group_ref),
        probe_ref
    );
    assert_eq!(file(), saved_file);
    crate::battle_context::with(|cx| {
        let mut groups = cx.world.ip_groups();
        let head = groups
            .head()
            .unwrap_or_else(|| panic!("fixture group missing"));
        groups.get_mut(head).task = IpGroup::REFORM_GROUP;
    });
    assert!(crate::invoke_generation(&mut h.state, |system, host| {
        GENERATE_SOL_FUNCTIONS.uninit_npcs(system, host)
    }));
    assert!(crate::battle_context::with(|cx| cx
        .world
        .npc_built_ships_ref()
        .head()
        .is_none()));
    assert!(crate::invoke_generation(&mut h.state, |system, host| {
        GENERATE_SOL_FUNCTIONS.reinit_npcs(system, host)
    }));
    assert_eq!(
        crate::battle_context::with(|cx| cx.world.data.game.battle_group_ref),
        probe_ref
    );
    assert_eq!(
        crate::battle_context::with(|cx| cx.world.word(StoryKey::URQUAN_PROBE_GRPOFFS0)),
        probe_ref
    );
    let probe = crate::battle_context::with(|cx| {
        let groups = cx.world.ip_groups_ref();
        *groups.get(
            groups
                .head()
                .unwrap_or_else(|| panic!("fixture group missing")),
        )
    });
    assert_eq!(probe.task, IpGroup::FLEE | IpGroup::IGNORE_FLAGSHIP);
    assert_eq!(probe.dest_loc, 0);
    crate::battle_context::with(|cx| {
        cx.world
            .set_story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED, 1)
    });
    assert!(crate::invoke_generation(&mut h.state, |system, host| {
        GENERATE_SOL_FUNCTIONS.init_npcs(system, host)
    }));
    assert_eq!(
        crate::battle_context::with(|cx| cx.world.data.game.battle_group_ref),
        probe_ref
    );
    assert_eq!(file(), saved_file);

    for activity in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
        crate::battle_context::with(|cx| {
            cx.world.set_activity(activity);
            cx.world.encounter_group = 17;
            cx.world
                .set_story_state(uqm_rust::engine::StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
        });
        assert!(crate::invoke_generation(&mut h.state, |view, host| {
            GENERATE_SOL_FUNCTIONS.generate_orbital(
                view,
                uqm_rust::engine::world::generation::Body::Moon { planet: 2, moon: 0 },
                host,
            )
        }));
        assert_eq!(
            crate::battle_context::with(|cx| cx.world.activity()),
            activity | START_ENCOUNTER
        );
        assert_eq!(
            crate::battle_context::with(|cx| get_game_state(
                cx.world,
                uqm_rust::engine::StoryKey::GLOBAL_FLAGS_AND_DATA
            )),
            255
        );
        assert!(
            crate::battle_context::with(|cx| cx.world.ip_groups_ref().head().is_none())
                && crate::battle_context::with(|cx| cx
                    .world
                    .npc_built_ships_ref()
                    .head()
                    .is_none())
        );
        assert_eq!(
            crate::battle_context::with(|cx| cx.world.word(StoryKey::URQUAN_PROBE_GRPOFFS0)),
            probe_ref
        );
    }
    crate::battle_context::with(|cx| {
        cx.world.set_catalog(std::borrow::Cow::Owned(saved_catalog));
        cx.world.current_star = saved_star;
    });
    crate::battle_context::with(|cx| uqm_rust::navigation::groups::uninit_group_info(cx.world));
    assert_ne!(
        crate::battle_context::with(|cx| uqm_rust::time::clock_bridge::rust_clock_uninit(cx.world)),
        0
    );
    crate::battle_context::with(|cx| {
        cx.world.queues = saved.0;
        cx.world.data.game.battle_group_ref = saved.1;
    });
    for (key, value) in keys.into_iter().zip(story) {
        crate::battle_context::with(|cx| cx.world.set_story_state(key, value));
    }
    crate::battle_context::with(|cx| cx.world.set_word(StoryKey::URQUAN_PROBE_GRPOFFS0, word));
    crate::battle_context::with(|cx| cx.world.set_activity(activity));
    println!("Sol NPC ABI: real probe file creation/reload/persistence, queue init/uninit/reinit, starbase save/reset, abort/load bits retained");
}

use uqm_rust::planets::generate::SystemGenerator;
