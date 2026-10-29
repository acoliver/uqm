//! Existing picker, encounter and status ABI decoding at the record boundary.
#![cfg(not(test))]
use crate::engine::battle::{
    link_queue,
    record::{SHIP_INFO, STARSHIP},
};
use crate::graphics::native_abi::RECT;
use std::ffi::c_void;

/// # Safety
/// The ship is a live member of one battle-side pool on the game thread.
#[no_mangle]
pub unsafe extern "C" fn UpdateShipFragCrew(ship: *mut STARSHIP) {
    // SAFETY: no caller record borrow crosses the encounter writeback.
    unsafe {
        let id = link_queue::identify(ship as usize);
        crate::battle::encount_native::engine::update_ship_frag_crew(id);
    }
}
/// # Safety
/// Ship info, optional ship, and optional clip rectangle are live game-thread records.
#[no_mangle]
pub unsafe extern "C" fn InitShipStatus(
    info: *mut SHIP_INFO,
    ship: *mut STARSHIP,
    clip: *mut RECT,
) {
    // SAFETY: only the optional owner identity leaves this boundary.
    unsafe {
        let id = (!ship.is_null()).then(|| link_queue::identify(ship as usize));
        crate::ui::shipstat::init_ship_status(info, id, clip);
    }
}
/// # Safety
/// Last is null or a live ship in the selected player's pool; game thread only.
#[export_name = "GetEncounterStarShip"]
pub unsafe extern "C" fn get_encounter_star_ship(last: *mut STARSHIP, player: u16) -> *mut c_void {
    // SAFETY: the old ABI address is consumed at the picker boundary.
    unsafe {
        get_encounter_star_ship_with_picker(last, player, |player, result| {
            crate::supermelee::native::battle_picker::live::GetNextMeleeStarShip(player, result)
        })
    }
}
/// # Safety
/// Last is null or live; a successful picker must return a live selected ship address.
pub unsafe fn get_encounter_star_ship_with_picker(
    last: *mut STARSHIP,
    player: u16,
    pick: impl FnOnce(u16, *mut *mut c_void) -> u32,
) -> *mut c_void {
    // SAFETY: the Native picker retains only an identity while the blocking UI runs.
    unsafe { crate::pickship::ffi::encounter_ship(last.cast(), player, pick) }
}

/// # Safety
/// Input context and optional ship are initialized game-thread records.
pub(crate) unsafe fn computer_frame(
    context: *mut crate::battle::battle_controls::InputContext,
    ship: *mut c_void,
) -> u8 {
    // SAFETY: the vtable's common context prefix and opaque ship address preserve the ABI contract.
    unsafe { super::ai_legacy::computer_intelligence(context.cast(), ship.cast()) }
}
/// # Safety
/// Input context and optional ship are initialized game-thread records.
pub(crate) unsafe fn human_frame(
    context: *mut crate::battle::battle_controls::InputContext,
    ship: *mut c_void,
) -> u8 {
    // SAFETY: the vtable's common context prefix and opaque ship address preserve the ABI contract.
    unsafe { crate::battle::battle_native::frameInputHuman(context.cast(), ship.cast()) }
}
