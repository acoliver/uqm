//! Resource phases around the typed communication session.

#[cfg(not(test))]
use super::*;

#[cfg(not(test))]
pub(super) fn load_descriptor(cx: &mut Cx<'_>, comm_id: u32) -> bool {
    // SAFETY: dispatch runs synchronously with the initialized campaign published.
    cx.with_legacy(|| unsafe {
        let descriptor = rust_init_race_dispatch(comm_id);
        if !descriptor.is_null() {
            std::ptr::copy_nonoverlapping(
                descriptor.cast::<crate::comm::locdata::CLocData>(),
                &raw mut ffi::CommData,
                1,
            );
            set_comm_data(read_locdata(&*descriptor.cast()));
        }
        !descriptor.is_null()
    })
}

#[cfg(not(test))]
pub(super) fn finish_unhailed(cx: &mut Cx<'_>, has_descriptor: bool) {
    let active = cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0;
    // SAFETY: the descriptor's callbacks remain live until encounter teardown.
    cx.with_legacy(|| unsafe {
        if has_descriptor {
            if active {
                if let Some(callback) = ffi::CommData.post_encounter_func {
                    callback();
                }
            }
            if let Some(callback) = ffi::CommData.uninit_encounter_func {
                callback();
            }
        }
    });
}

#[cfg(not(test))]
pub(super) fn fight_if_selected(cx: &mut Cx<'_>) -> u16 {
    // SAFETY: battle and encounter loops still use the published campaign until U2-10a.
    cx.with_legacy(|| unsafe {
        let mut status = 0;
        let activity = crate::mainloop::ffi::get_current_activity().0;
        if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
            let npc = npc_built_ship_queue();
            if lobyte(activity) == IN_LAST_BATTLE
                && WorldData::current().read(|owner| owner.game.glob_flags) & CYBORG_ENABLED != 0
            {
                ReinitQueue(npc);
            }
            World::current()
                .write(|world| world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0));
            status = u16::from(
                World::current().read(|world| world.story_state(StoryKey::BATTLE_SEGUE)) != 0
                    && !get_head_link(npc).is_null(),
            );
            if status != 0 {
                ffi::BuildBattle(RPG_PLAYER_NUM);
                ffi::EncounterBattle();
            } else {
                World::current().write(|world| world.set_story_state(StoryKey::BATTLE_SEGUE, 0));
            }
        }
        status
    })
}

#[cfg(not(test))]
pub(super) unsafe fn update_fleet(h_encounter: HLink, npc_q: *mut Queue, status: u16) {
    // SAFETY: the caller retains these native resources on the initialized game thread.
    unsafe {
        // Update HSpace encounter info, ships left, etc.
        let enc_ptr = lock_encounter(h_encounter);
        let num_ships = CountLinks(npc_q);
        (*enc_ptr).num_ships = num_ships as u8;
        (*enc_ptr).flags |= ENCOUNTER_REFORMING;
        if status == 0 {
            (*enc_ptr).flags |= ONE_SHOT_ENCOUNTER;
        }

        for i in 0..num_ships {
            let h_star_ship = get_star_ship_from_index(npc_q, i as u8);
            if h_star_ship.is_null() {
                break;
            }
            let frag_ptr = lock_ship_frag(npc_q, h_star_ship);
            let bsi = &mut (*enc_ptr).ship_list[i as usize];
            bsi.race_id = (*frag_ptr).race_id;
            bsi.crew_level = (*frag_ptr).crew_level;
            bsi.max_crew = (*frag_ptr).max_crew;
            bsi.max_energy = (*frag_ptr).max_energy;
        }
    }
}
