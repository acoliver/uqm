//! Graphics resource registration, ported from resgfx.c (GPL-2.0-or-later).
use super::native_abi::*;
use std::ffi::{c_char, c_void};

use super::gfxload::{get_cel_data, get_font_data, release_cel_data, release_font_data};
use crate::resource::ffi_types::ResourceData;
unsafe extern "C" fn cel_data(path: *const c_char, data: *mut ResourceData) {
    (*data).ptr = LoadResourceFromPath(path, Some(get_cel_data));
}
unsafe extern "C" fn font_data(path: *const c_char, data: *mut ResourceData) {
    (*data).ptr = LoadResourceFromPath(path, Some(get_font_data));
}
/// # Safety
/// Must be called during serialized resource-system initialization.
#[no_mangle]
pub unsafe extern "C" fn InstallGraphicResTypes() -> BOOLEAN {
    InstallResTypeVectors(
        c"GFXRES".as_ptr(),
        Some(cel_data),
        Some(release_cel_data),
        None,
    );
    InstallResTypeVectors(
        c"FONTRES".as_ptr(),
        Some(font_data),
        Some(release_font_data),
        None,
    );
    1
}
/// # Safety
/// Resource must be a registered C-string resource identifier.
#[no_mangle]
pub unsafe extern "C" fn LoadGraphicInstance(resource: *const c_char) -> *mut c_void {
    let data = res_GetResource(resource);
    if !data.is_null() {
        res_DetachResource(resource);
    }
    data
}
