#[cfg(not(test))]
use crate::engine::battle::ShipId;
#[cfg(not(test))]
use crate::engine::battle::{
    link_queue,
    record::{SHIP_INFO, STARSHIP},
};
#[cfg(not(test))]
use crate::graphics::native_abi::RECT;
#[cfg(not(test))]
use crate::ships::native::abi::{COUNT, SHIP_FRAGMENT};
#[cfg(not(test))]
use std::ffi::c_void;

/// # Safety
/// The ship is a live member of one battle-side pool on the game thread.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn UpdateShipFragCrew(ship: *mut STARSHIP) {
    // SAFETY: no caller record borrow crosses the encounter writeback.

    let id = link_queue::identify(ship as usize);
    crate::battle::encount_native::engine::update_ship_frag_crew(id);
}
/// # Safety
/// Ship info, optional ship, and optional clip rectangle are live game-thread records.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn InitShipStatus(
    info: *mut SHIP_INFO,
    ship: *mut STARSHIP,
    clip: *mut RECT,
) {
    // SAFETY: only the optional owner identity leaves this boundary.

    let id = (!ship.is_null()).then(|| link_queue::identify(ship as usize));
    crate::ui::shipstat::init_ship_status(info, id, clip);
}
/// # Safety
/// Last is null or a live ship in the selected player's pool; game thread only.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn GetEncounterStarShip(last: *mut STARSHIP, player: u16) -> *mut c_void {
    // SAFETY: the old ABI address is consumed at the picker boundary.

    get_encounter_star_ship_with_picker(last, player, |player, result| {
        crate::supermelee::native::battle_picker::live::GetNextMeleeStarShip(player, result)
    })
}
/// # Safety
/// Last is null or live; a successful picker must return a live selected ship address.
#[cfg(not(test))]
pub unsafe fn get_encounter_star_ship_with_picker(
    last: *mut STARSHIP,
    player: u16,
    pick: impl FnOnce(u16, *mut *mut c_void) -> u32,
) -> *mut c_void {
    // SAFETY: the Native picker retains only an identity while the blocking UI runs.
    crate::pickship::ffi::encounter_ship(last.cast(), player, pick)
}

/// # Safety
/// Input context and optional ship are initialized game-thread records.
#[cfg(not(test))]
pub(crate) unsafe fn computer_frame(
    context: *mut crate::battle::battle_controls::InputContext,
    ship: *mut c_void,
) -> u8 {
    // SAFETY: the vtable's common context prefix and opaque ship address preserve the ABI contract.
    super::ai_legacy::computer_intelligence(context.cast(), ship.cast())
}
/// # Safety
/// Input context and optional ship are initialized game-thread records.
#[cfg(not(test))]
pub(crate) unsafe fn human_frame(
    context: *mut crate::battle::battle_controls::InputContext,
    ship: *mut c_void,
) -> u8 {
    // SAFETY: the vtable's common context prefix and opaque ship address preserve the ABI contract.
    crate::battle::battle_native::frameInputHuman(context.cast(), ship.cast())
}

#[cfg(not(test))]
pub use GetEncounterStarShip as get_encounter_star_ship;

#[cfg(not(test))]
/// # Safety
/// The owner's fragment queue matches battle ship link order on the game thread.
pub(crate) unsafe fn update_ship_frag_crew(owner: ShipId) {
    let player = {
        let ships = link_queue::ships(owner.side as usize);
        ships.get(ships.link(owner)).player
    };
    let frag_q = crate::battle::encount_native::engine::GetShipFragQueueForPlayer(player as COUNT);
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
