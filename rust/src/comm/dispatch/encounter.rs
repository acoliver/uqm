//! Resource phases around the typed communication session.

use super::*;

#[cfg(not(test))]
pub(super) fn load_descriptor(cx: &mut Cx<'_>, comm_id: u32) -> bool {
    // SAFETY: the returned descriptor is copied before any other encounter initializes.
    unsafe {
        let descriptor = rust_init_race_dispatch(cx, comm_id);
        if !descriptor.is_null() {
            std::ptr::copy_nonoverlapping(
                descriptor.cast::<crate::comm::locdata::CLocData>(),
                &raw mut ffi::CommData,
                1,
            );
            set_comm_data(read_locdata(&*descriptor.cast()));
        }
        !descriptor.is_null()
    }
}

#[cfg(not(test))]
pub(super) fn finish_unhailed(cx: &mut Cx<'_>, has_descriptor: bool) {
    let active = cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0;
    if has_descriptor {
        // SAFETY: the descriptor remains live until encounter teardown on this thread.
        let post = unsafe { ffi::CommData.post_encounter_func };
        if active {
            if let Some(callback) = post {
                callback(cx);
            }
        }
        // SAFETY: post-encounter dispatch has completed on the game thread.
        let uninit = unsafe { ffi::CommData.uninit_encounter_func };
        if let Some(callback) = uninit {
            callback(cx);
        }
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
            // SAFETY: the published campaign retains the fragment queue during construction.
            cx.with_legacy(|| unsafe { ffi::BuildBattle(RPG_PLAYER_NUM) });
            // SAFETY: encounter dispatch retains the initialized audio resources for this battle.
            unsafe { crate::battle::encount_native::encounter_battle(cx, io) };
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
