//! Legacy macro entry points backed by their Rust providers.
#![allow(non_snake_case)]

use crate::graphics::{native_abi::drawable_desc, resgfx::LoadGraphicInstance};
use crate::strings::string_table::{
    CaptureStringTable, ReleaseStringTable, StringTable, StringTableEntry,
};
use std::ffi::{c_char, c_void};

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
pub unsafe fn get_current_activity() -> u16 {
    crate::navigation::globals::GlobData
        .Game_state
        .CurrentActivity
}
