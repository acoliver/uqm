#[cfg(not(test))]
use crate::engine::battle::ShipId;
#[cfg(not(test))]
use crate::ships::native::abi::{COUNT, RPG_PLAYER_NUM};
#[cfg(not(test))]
use std::ffi::c_void;

#[cfg(not(test))]
/// # Safety
/// The game thread retains the initialized battle context and all resources used by this call.
pub unsafe fn get_encounter_star_ship(last: Option<ShipId>, player: u16) -> Option<ShipId> {
    // SAFETY: the game-thread picker retains its selected ship through this synchronous handoff.
    unsafe {
        get_encounter_star_ship_with_picker(last, player, |player, result| {
            crate::supermelee::native::battle_picker::live::GetNextMeleeStarShip(player, result)
        })
    }
}

/// # Safety
/// Last is null or live; a successful picker must return a live selected ship address.
#[cfg(not(test))]
pub unsafe fn get_encounter_star_ship_with_picker(
    last: Option<ShipId>,
    player: u16,
    pick: impl FnOnce(u16, *mut *mut c_void) -> u32,
) -> Option<ShipId> {
    // SAFETY: the Native picker retains only an identity while the blocking UI runs.
    unsafe {
        crate::pickship::ffi::encounter_ship(
            last,
            player,
            crate::battle::battle_native::engine::activity(),
            pick,
        )
    }
}

#[cfg(not(test))]
/// The owner's fragment queue matches battle ship link order in the supplied context.
pub(crate) fn update_ship_frag_crew(cx: &mut crate::engine::battle::Cx<'_>, owner: ShipId) {
    let player = cx.ship_state(owner).player;
    let ships = cx.kernel.ships(player as usize);
    let mut next = ships.head();
    let mut index = 0;
    while let Some(id) = next {
        if ships.ship_id(id) == owner {
            let crew = ships.get(id).crew;
            if player == RPG_PLAYER_NUM as i16 {
                write_fragment_crew(cx.world.built_ships(), index, crew);
            } else {
                write_fragment_crew(cx.world.npc_built_ships(), index, crew);
            }
            break;
        }
        next = ships.succ(id);
        index += 1;
    }
}

#[cfg(not(test))]
fn write_fragment_crew<const N: usize>(
    mut queue: crate::engine::battle::link_queue::LinkQueue<
        '_,
        crate::engine::world::queues::Fragment,
        N,
    >,
    index: usize,
    crew: u16,
) {
    if let Some(id) = queue.at(index) {
        let fragment = queue.get_mut(id);
        assert_ne!(fragment.crew_level, COUNT::MAX);
        fragment.crew_level = crew;
    }
}
