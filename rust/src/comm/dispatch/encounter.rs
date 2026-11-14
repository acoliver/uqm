//! Resource phases around the typed communication session.

use crate::comm::Comm;

use super::*;

pub(super) fn load_descriptor(comm: &mut Comm, cx: &mut Cx<'_>, comm_id: u32) {
    // SAFETY: dispatch owns the game thread and retains the descriptor resources through teardown.
    unsafe {
        let descriptor = race_descriptor(comm, cx, comm_id);
        comm.install_descriptor(descriptor);
    }
}

pub(super) fn finish_unhailed(comm: &mut Comm, cx: &mut Cx<'_>) {
    let active = cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0;
    let post = comm.data.post_encounter_func;
    if active {
        if let Some(callback) = post {
            callback(comm, cx);
        }
    }
    let uninit = comm.data.uninit_encounter_func;
    if let Some(callback) = uninit {
        callback(comm, cx);
    }
}

#[cfg(not(test))]
pub(super) fn fight_if_selected(cx: &mut Cx<'_>, io: &mut Io<'_>) -> u16 {
    let mut status = 0;
    let activity = cx.world.activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        if lobyte(activity) == IN_LAST_BATTLE && cx.world.data.game.glob_flags & CYBORG_ENABLED != 0
        {
            // SAFETY: encounter dispatch exclusively owns the live NPC queue.
            cx.world.npc_built_ships().reinit();
        }
        cx.world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
        status = u16::from(
            cx.world.story_state(StoryKey::BATTLE_SEGUE) != 0
                && cx.world.npc_built_ships_ref().head().is_some(),
        );
        if status != 0 {
            crate::battle::encount_native::build_battle(cx.world, cx.kernel, RPG_PLAYER_NUM);
            crate::battle::encount_native::encounter_battle(cx, io);
        } else {
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
        }
    }
    status
}

pub(super) fn update_fleet(
    world: &mut World,
    id: LinkId<crate::engine::world::queues::Encounter>,
    status: u16,
) {
    let queue = world.npc_built_ships_ref();
    let mut ships = [crate::engine::world::queues::BriefShipInfo::default(); MAX_HYPER_SHIPS];
    let mut count = 0;
    let mut next = queue.head();
    while let Some(current) = next {
        let fragment = queue.get(current);
        next = queue.succ(current);
        ships[count] = crate::engine::world::queues::BriefShipInfo {
            race_id: fragment.race_id,
            crew_level: fragment.crew_level,
            max_crew: fragment.max_crew,
            max_energy: fragment.max_energy,
        };
        count += 1;
    }
    let mut queue = world.encounters();
    let record = queue.get_mut(id);
    record.num_ships = count as u8;
    record.flags |= ENCOUNTER_REFORMING;
    if status == 0 {
        record.flags |= ONE_SHOT_ENCOUNTER;
    }
    record.ship_list[..count].copy_from_slice(&ships[..count]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn descriptor_install_runs_both_passes_and_keeps_the_selected_session() {
        crate::comm::with_test_context(|cx| {
            use crate::comm::{live, locdata::resource_key};
            cx.world.story = Some(crate::engine::world::story::StoryState::new());
            let mut comm = Comm::new();
            assert!(comm.race_scratch.spathi.is_none());
            cx.world
                .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 128);
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
            load_descriptor(&mut comm, cx, conv::SPATHI);
            assert!(comm.race_scratch.spathi.is_some());
            assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 1);
            assert_eq!(
                comm.data.definition.conversation_phrases_res,
                Some(c"comm.safeones.dialogue")
            );
            assert!(std::ptr::fn_addr_eq(
                comm.data.init_encounter_func.unwrap(),
                live::spahome::intro as fn(&mut Comm, &mut Cx<'_>)
            ));
            let view = comm.comm_data().unwrap();
            assert_eq!(
                view.conversation_phrases_res,
                resource_key(comm.data.definition.conversation_phrases_res)
            );
            assert_eq!(view.alien_text_width, comm.data.definition.alien_text_width);
            assert!(std::ptr::fn_addr_eq(
                view.post_encounter_func.unwrap(),
                comm.data.post_encounter_func.unwrap()
            ));
            assert!(std::ptr::fn_addr_eq(
                view.uninit_encounter_func.unwrap(),
                comm.data.uninit_encounter_func.unwrap()
            ));
            assert!(comm.data.alien_frame.is_null());
            assert!(comm.data.conversation_phrases.is_null());

            cx.world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
            cx.world.set_story_state(StoryKey::FOUND_PLUTO_SPATHI, 1);
            load_descriptor(&mut comm, cx, conv::SPATHI);
            assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 0);
            assert_eq!(
                comm.data.definition.conversation_phrases_res,
                Some(c"comm.spathi.dialogue")
            );
            assert_eq!(comm.data.definition.alien_song_flags & 1, 1);
            assert!(std::ptr::fn_addr_eq(
                comm.data.init_encounter_func.unwrap(),
                live::spathi::intro as fn(&mut Comm, &mut Cx<'_>)
            ));

            cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 1);
            load_descriptor(&mut comm, cx, conv::COMMANDER);
            assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 0);
            assert_eq!(
                comm.data.definition.conversation_phrases_res,
                Some(c"comm.starbase.dialogue")
            );
            assert!(std::ptr::fn_addr_eq(
                comm.data.init_encounter_func.unwrap(),
                live::starbas::intro as fn(&mut Comm, &mut Cx<'_>)
            ));
            assert_eq!(
                comm.comm_data().unwrap().conversation_phrases_res,
                resource_key(comm.data.definition.conversation_phrases_res)
            );
        });
    }

    #[test]
    fn unhailed_teardown_skips_post_on_abort_or_load_but_always_uninitializes() {
        fn post(_comm: &mut crate::comm::Comm, cx: &mut crate::engine::battle::Cx<'_>) {
            assert_eq!(
                cx.world
                    .story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE),
                0
            );
            cx.world
                .set_story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE, 1);
        }
        fn uninit(_comm: &mut crate::comm::Comm, cx: &mut crate::engine::battle::Cx<'_>) -> u16 {
            assert_eq!(
                cx.world
                    .story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE),
                u32::from(cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0)
            );
            cx.world
                .set_story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE, 2);
            0
        }
        crate::comm::with_test_context(|cx| {
            cx.world.story = Some(crate::engine::world::story::StoryState::new());
            for flags in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
                cx.world.set_activity(flags);
                cx.world
                    .set_story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE, 0);
                let mut comm = crate::comm::Comm::new();
                let data = crate::comm::locdata::SessionData {
                    post_encounter_func: Some(post),
                    uninit_encounter_func: Some(uninit),
                    ..crate::comm::locdata::SessionData::new(
                        crate::comm::races::chmmr::descriptor::descriptor(),
                    )
                };
                // SAFETY: the unplayed definition owns no loaded resources.
                unsafe { comm.install_descriptor(data) };
                finish_unhailed(&mut comm, cx);
                assert_eq!(
                    cx.world
                        .story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE),
                    2
                );
                comm.data.post_encounter_func = None;
                comm.data.uninit_encounter_func = None;
                finish_unhailed(&mut comm, cx);
                assert_eq!(
                    cx.world
                        .story_state(crate::engine::StoryKey::CHMMR_BOMB_STATE),
                    2
                );
            }
        });
    }

    #[test]
    fn surviving_ships_replace_the_encounter_fleet_in_queue_order() {
        let mut world = World::new();
        for (race_id, crew_level, max_crew, max_energy) in [(3, 4, 5, 6), (7, 8, 9, 10)] {
            let mut queue = world.npc_built_ships();
            let id = queue.alloc().unwrap();
            let ship = queue.get_mut(id);
            ship.race_id = race_id;
            ship.crew_level = crew_level;
            ship.max_crew = max_crew;
            ship.max_energy = max_energy;
            queue.put(id);
        }
        let id = world.encounters().alloc().unwrap();
        world.encounters().get_mut(id).flags = 1;
        update_fleet(&mut world, id, 0);
        let record = *world.encounters_ref().get(id);
        assert_eq!(record.num_ships, 2);
        assert_eq!(record.flags, 1 | ENCOUNTER_REFORMING | ONE_SHOT_ENCOUNTER);
        let copied: Vec<_> = record.ship_list[..2]
            .iter()
            .map(|s| (s.race_id, s.crew_level, s.max_crew, s.max_energy))
            .collect();
        assert_eq!(copied, [(3, 4, 5, 6), (7, 8, 9, 10)]);
        world.encounters().get_mut(id).flags = 1;
        world.npc_built_ships().reinit();
        update_fleet(&mut world, id, 1);
        let record = *world.encounters_ref().get(id);
        assert_eq!(record.num_ships, 0);
        assert_eq!(record.flags, 1 | ENCOUNTER_REFORMING);
    }
}
