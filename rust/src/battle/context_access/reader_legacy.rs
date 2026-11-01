#[cfg(not(test))]
use crate::engine::battle::link_queue;
#[cfg(not(test))]
use crate::engine::battle::ShipId;
#[cfg(not(test))]
use crate::ships::native::abi::{COUNT, SHIP_FRAGMENT};
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

/// # Safety
/// Input context and optional ship are initialized game-thread records.
#[cfg(not(test))]
pub(crate) unsafe fn computer_frame(
    context: *mut crate::battle::battle_controls::InputContext,
    ship: *mut c_void,
) -> u8 {
    // SAFETY: the vtable's common context prefix and opaque ship address preserve the ABI contract.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let id = (!ship.is_null()).then(|| cx.ship_id(ship as usize));
            super::ai_legacy::computer_intelligence(cx, &*context.cast(), id)
        })
    }
}
/// # Safety
/// Input context and optional ship are initialized game-thread records.
#[cfg(not(test))]
pub(crate) unsafe fn human_frame(
    context: *mut crate::battle::battle_controls::InputContext,
    _ship: *mut c_void,
) -> u8 {
    // SAFETY: the vtable's common context prefix retains its live human input context.
    unsafe { crate::battle::battle_native::engine::frame_input_human(&*context.cast()) }
}

#[cfg(not(test))]
#[cfg(not(test))]
/// # Safety
/// The owner's fragment queue matches battle ship link order on the game thread.
pub(crate) unsafe fn update_ship_frag_crew(owner: ShipId) {
    // SAFETY: the caller retains the initialized operands on the published game thread.
    unsafe {
        let player = {
            let ships = link_queue::ships(owner.side as usize);
            ships.get(ships.link(owner)).player
        };
        let frag_q =
            crate::battle::encount_native::engine::GetShipFragQueueForPlayer(player as COUNT);
        let mut hf = (*frag_q).head;
        let ships = link_queue::ships(player as usize);
        let mut hs = ships.head();
        while let Some(id) = hs.filter(|_| !hf.is_null()) {
            let frag = hf.cast::<SHIP_FRAGMENT>();
            hs = ships.succ(id);
            hf = (*frag).succ;
            if ships.ship_id(id, player as u8) == owner {
                assert_ne!((*frag).crew_level, COUNT::MAX);
                (*frag).crew_level = ships.get(id).crew;
                break;
            }
        }
    }
}
