//! Graphics resource registration, ported from resgfx.c (GPL-2.0-or-later).
use super::native_abi::*;
use std::ffi::{c_char, c_void};

extern "C" {
    fn _GetCelData(fp: *mut uio_Stream, length: u32) -> *mut c_void;
    fn _GetFontData(fp: *mut uio_Stream, length: u32) -> *mut c_void;
    fn _ReleaseCelData(handle: *mut c_void) -> BOOLEAN;
    fn _ReleaseFontData(handle: *mut c_void) -> BOOLEAN;
}
unsafe extern "C" fn cel_data(path: *const c_char, data: *mut RESOURCE_DATA) {
    (*data).ptr = LoadResourceFromPath(path, Some(_GetCelData));
}
unsafe extern "C" fn font_data(path: *const c_char, data: *mut RESOURCE_DATA) {
    (*data).ptr = LoadResourceFromPath(path, Some(_GetFontData));
}
/// # Safety
/// Must be called during serialized resource-system initialization.
#[no_mangle]
pub unsafe extern "C" fn InstallGraphicResTypes() -> BOOLEAN {
    InstallResTypeVectors(
        c"GFXRES".as_ptr(),
        Some(cel_data),
        Some(_ReleaseCelData),
        None,
    );
    InstallResTypeVectors(
        c"FONTRES".as_ptr(),
        Some(font_data),
        Some(_ReleaseFontData),
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
