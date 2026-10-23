//! Legacy macro entry points backed by their Rust providers.
#![allow(non_snake_case)]

use crate::graphics::{native_abi::drawable_desc, resgfx::LoadGraphicInstance};
use crate::ships::native::abi::{ELEMENT, QUEUE, STARSHIP};
use crate::strings::string_table::{
    CaptureStringTable, ReleaseStringTable, StringTable, StringTableEntry,
};
use std::ffi::{c_char, c_void};

/// # Safety
/// The element must be live and exclusively borrowed on the game thread.
#[no_mangle]
pub unsafe extern "C" fn InitIntersectStartPoint(e: *mut ELEMENT) {
    (*e).IntersectControl.IntersectStamp.origin.x = (*e).current.location.x >> 2;
    (*e).IntersectControl.IntersectStamp.origin.y = (*e).current.location.y >> 2;
}

/// # Safety
/// The element must be live and exclusively borrowed on the game thread.
#[no_mangle]
pub unsafe extern "C" fn InitIntersectEndPoint(e: *mut ELEMENT) {
    (*e).IntersectControl.EndPoint.x = (*e).next.location.x >> 2;
    (*e).IntersectControl.EndPoint.y = (*e).next.location.y >> 2;
}

/// # Safety
/// Both pointers must be valid; the element's parent must remain live.
#[no_mangle]
pub unsafe extern "C" fn GetElementStarShip(e: *const ELEMENT, ship: *mut *mut STARSHIP) {
    *ship = (*e).pParent.cast();
}

/// # Safety
/// The element must be live and exclusively borrowed; ship is its parent.
#[no_mangle]
pub unsafe extern "C" fn SetElementStarShip(e: *mut ELEMENT, ship: *mut STARSHIP) {
    (*e).pParent = ship.cast();
}

#[no_mangle]
pub extern "C" fn rust_bridge_LockLink(_queue: *const QUEUE, link: *mut c_void) -> *mut c_void {
    link
}

#[no_mangle]
pub extern "C" fn rust_bridge_UnlockLink(_queue: *const QUEUE, _link: *mut c_void) {}

/// # Safety
/// Queue must point to an initialized queue.
#[no_mangle]
pub unsafe extern "C" fn rust_bridge_GetLinkSize(queue: *const QUEUE) -> u16 {
    (*queue).object_size
}

/// # Safety
/// Resource key must be a NUL-terminated string and the loader initialized.
#[no_mangle]
pub unsafe extern "C" fn LoadGraphic(key: *const c_char) -> *mut drawable_desc {
    LoadGraphicInstance(key).cast()
}

/// # Safety
/// Resource key must be a NUL-terminated string and the loader initialized.
#[no_mangle]
pub unsafe extern "C" fn LoadSound(key: *const c_char) -> *mut c_void {
    crate::sound::resource_types::LoadSoundInstance(key)
}

/// # Safety
/// Resource key must be a NUL-terminated string and the loader initialized.
#[no_mangle]
pub unsafe extern "C" fn LoadMusic(key: *const c_char) -> *mut c_void {
    crate::sound::resource_types::LoadMusicInstance(key)
}

/// # Safety
/// Resource key must be a NUL-terminated string and the loader initialized.
#[no_mangle]
pub unsafe extern "C" fn LoadStringTable(key: *const c_char) -> *mut StringTable {
    crate::strings::sresins::LoadStringTableInstance(key)
}

/// # Safety
/// Sound must be null or a live sound bank represented by a string table.
#[no_mangle]
pub unsafe extern "C" fn CaptureSound(sound: *mut StringTable) -> *mut StringTableEntry {
    CaptureStringTable(sound)
}

/// # Safety
/// Sound must be null or a live sound bank entry.
#[no_mangle]
pub unsafe extern "C" fn ReleaseSound(sound: *mut StringTableEntry) -> *mut StringTable {
    ReleaseStringTable(sound)
}

/// # Safety
/// Called on the game thread while the live game state is initialized.
#[no_mangle]
pub unsafe extern "C" fn get_current_activity() -> u16 {
    crate::navigation::globals::GlobData
        .Game_state
        .CurrentActivity
}
