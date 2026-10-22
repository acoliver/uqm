//! Registration of the string table resource types and instance loading.

use std::ffi::{c_char, c_int};

use crate::resource::ffi_bridge::{
    res_DetachResource, res_GetResource, FreeResourceData, InstallResTypeVectors,
    LoadResourceFromPath,
};
use crate::resource::ffi_types::ResourceData;
use crate::strings::getstr::{_GetBinaryTableData, _GetConversationData, _GetStringData};
use crate::strings::strings::StringTable;

unsafe extern "C" fn get_string_table_file_data(
    pathname: *const c_char,
    resdata: *mut ResourceData,
) {
    (*resdata).ptr = LoadResourceFromPath(pathname, Some(_GetStringData));
}

unsafe extern "C" fn get_binary_table_file_data(
    pathname: *const c_char,
    resdata: *mut ResourceData,
) {
    (*resdata).ptr = LoadResourceFromPath(pathname, Some(_GetBinaryTableData));
}

/// Registers `STRTAB`, `BINTAB` and `CONVERSATION`. Always returns TRUE.
#[no_mangle]
pub extern "C" fn InstallStringTableResType() -> c_int {
    // SAFETY: the type names are NUL-terminated literals and the callbacks
    // match the resource system's signatures.
    unsafe {
        InstallResTypeVectors(
            c"STRTAB".as_ptr(),
            Some(get_string_table_file_data),
            Some(FreeResourceData),
            None,
        );
        InstallResTypeVectors(
            c"BINTAB".as_ptr(),
            Some(get_binary_table_file_data),
            Some(FreeResourceData),
            None,
        );
        InstallResTypeVectors(
            c"CONVERSATION".as_ptr(),
            Some(_GetConversationData),
            Some(FreeResourceData),
            None,
        );
    }
    1
}

/// Loads the table behind `res` and takes ownership of it from the
/// resource system.
///
/// # Safety
/// `res` must be a NUL-terminated resource key.
#[no_mangle]
pub unsafe extern "C" fn LoadStringTableInstance(res: *const c_char) -> *mut StringTable {
    let data = res_GetResource(res);
    if !data.is_null() {
        res_DetachResource(res);
    }
    data.cast()
}
