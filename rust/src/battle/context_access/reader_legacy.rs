#[cfg(not(test))]
use crate::engine::battle::link_queue;
#[cfg(not(test))]
use crate::engine::battle::ShipId;
#[cfg(not(test))]
use crate::ships::native::abi::{COUNT, RPG_PLAYER_NUM, SHIP_FRAGMENT};
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
#[cfg(not(test))]
/// # Safety
/// The owner's fragment queue matches battle ship link order on the game thread.
pub(crate) unsafe fn update_ship_frag_crew(owner: ShipId) {
    // SAFETY: the caller retains the initialized operands on the published game thread.
    unsafe {
        let player = {
            // SAFETY: the typed owner identifies its initialized battle side.
            let ships = link_queue::ships(owner.side() as usize);
            ships.get(ships.link(owner)).player
        };
        let mut hf = crate::engine::WorldData::current()
            .read(|world| world.ship_fragments(player == RPG_PLAYER_NUM as i16).head);
        // SAFETY: this player was read from the live owner's battle record above.
        let ships = link_queue::ships(player as usize);
        let mut hs = ships.head();
        while let Some(id) = hs.filter(|_| !hf.is_null()) {
            let frag = hf.cast::<SHIP_FRAGMENT>();
            hs = ships.succ(id);
            hf = (*frag).succ;
            if ships.ship_id(id) == owner {
                assert_ne!((*frag).crew_level, COUNT::MAX);
                (*frag).crew_level = ships.get(id).crew;
                break;
            }
        }
    }
}
