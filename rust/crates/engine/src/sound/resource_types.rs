//! Sound bank and music instance loading. The `SNDRES` and `MUSICRES`
//! resource types are registered by the resource system itself.

use std::ffi::{c_char, c_void};

use crate::resource::ffi_bridge::{res_DetachResource, res_GetResource};

/// Loads the sound bank behind `res` and takes ownership of it from the
/// resource system.
///
/// # Safety
/// `res` must be a NUL-terminated resource key.
#[no_mangle]
pub unsafe extern "C" fn LoadSoundInstance(res: *const c_char) -> *mut c_void {
    let data = res_GetResource(res);
    if !data.is_null() {
        res_DetachResource(res);
    }
    data
}

/// Loads the music behind `res` and takes ownership of it from the
/// resource system.
///
/// # Safety
/// `res` must be a NUL-terminated resource key.
#[no_mangle]
pub unsafe extern "C" fn LoadMusicInstance(res: *const c_char) -> *mut c_void {
    let data = res_GetResource(res);
    if !data.is_null() {
        res_DetachResource(res);
    }
    data
}
