//! Resource phases around the typed communication session.

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
    if has_descriptor {
        // SAFETY: the descriptor remains live until encounter teardown on this thread.
        let post = unsafe { ffi::CommData.post_encounter_func };
        if active {
            if let Some(callback) = post {
                cx.with_legacy(callback);
            }
        }
        // SAFETY: post-encounter dispatch has completed on the game thread.
        let uninit = unsafe { ffi::CommData.uninit_encounter_func };
        if let Some(callback) = uninit {
            cx.with_legacy(callback);
        }
    }
}

#[cfg(not(test))]
pub(super) fn fight_if_selected(cx: &mut Cx<'_>) -> u16 {
    let mut status = 0;
    let activity = cx.world.activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        let npc = cx.with_legacy(npc_built_ship_queue);
        if lobyte(activity) == IN_LAST_BATTLE && cx.world.data.game.glob_flags & CYBORG_ENABLED != 0
        {
            // SAFETY: encounter dispatch exclusively owns the live NPC queue.
            cx.with_legacy(|| unsafe { ReinitQueue(npc) });
        }
        cx.world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
        status = u16::from(
            cx.world.story_state(StoryKey::BATTLE_SEGUE) != 0
                && !cx.world.data.game.npc_built_ship_q.head.is_null(),
        );
        if status != 0 {
            // SAFETY: battle still consumes the published campaign until U2-10a.
            cx.with_legacy(|| unsafe {
                ffi::BuildBattle(RPG_PLAYER_NUM);
                ffi::EncounterBattle();
            });
        } else {
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
        }
    }
    status
}

pub(super) unsafe fn update_fleet(h_encounter: HLink, npc_q: *mut Queue, status: u16) {
    // SAFETY: the caller retains this encounter and NPC queue throughout the update.
    let (enc_ptr, num_ships) = unsafe { (lock_encounter(h_encounter), CountLinks(npc_q)) };
    // SAFETY: the encounter record is exclusively owned by this update.
    unsafe {
        (*enc_ptr).num_ships = num_ships as u8;
        (*enc_ptr).flags |= ENCOUNTER_REFORMING;
    }
    if status == 0 {
        // SAFETY: the same encounter record remains exclusively owned.
        unsafe { (*enc_ptr).flags |= ONE_SHOT_ENCOUNTER };
    }
    for i in 0..num_ships {
        // SAFETY: the queue remains live and i is bounded by its current link count.
        let h_star_ship = unsafe { get_star_ship_from_index(npc_q, i as u8) };
        if h_star_ship.is_null() {
            break;
        }
        // SAFETY: this handle belongs to the retained NPC queue and the encounter has matching slots.
        unsafe {
            let frag_ptr = lock_ship_frag(npc_q, h_star_ship);
            let bsi = &mut (*enc_ptr).ship_list[i as usize];
            bsi.race_id = (*frag_ptr).race_id;
            bsi.crew_level = (*frag_ptr).crew_level;
            bsi.max_crew = (*frag_ptr).max_crew;
            bsi.max_energy = (*frag_ptr).max_energy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::queue::{InitQueue, UninitQueue};

    #[test]
    fn surviving_ships_replace_the_encounter_fleet_in_queue_order() {
        let mut queue = Queue::default();
        // SAFETY: the fixture owns this queue and all records until teardown.
        unsafe {
            assert!(InitQueue(
                &mut queue,
                2,
                std::mem::size_of::<CShipFragment>() as u16
            ));
            for (race_id, crew_level, max_crew, max_energy) in [(3, 4, 5, 6), (7, 8, 9, 10)] {
                let link = AllocLink(&mut queue);
                assert!(!link.is_null());
                link.cast::<CShipFragment>().write(CShipFragment {
                    race_id,
                    crew_level,
                    max_crew,
                    max_energy,
                    ..Default::default()
                });
                PutQueue(&mut queue, link);
            }
            let mut encounter: CEncounter = std::mem::zeroed();
            encounter.flags = 1;
            update_fleet(std::ptr::from_mut(&mut encounter).cast(), &mut queue, 0);
            assert_eq!(encounter.num_ships, 2);
            assert_eq!(
                encounter.flags,
                1 | ENCOUNTER_REFORMING | ONE_SHOT_ENCOUNTER
            );
            let copied: Vec<_> = encounter.ship_list[..2]
                .iter()
                .map(|s| (s.race_id, s.crew_level, s.max_crew, s.max_energy))
                .collect();
            assert_eq!(copied, [(3, 4, 5, 6), (7, 8, 9, 10)]);
            encounter.flags = 1;
            ReinitQueue(&mut queue);
            update_fleet(std::ptr::from_mut(&mut encounter).cast(), &mut queue, 1);
            assert_eq!(encounter.num_ships, 0);
            assert_eq!(encounter.flags, 1 | ENCOUNTER_REFORMING);
            UninitQueue(&mut queue);
        }
    }
}
