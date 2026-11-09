//! Registration of the string table resource types and instance loading.

#[cfg(test)]
#[path = "sresins/tests.rs"]
mod tests;

use std::ffi::{c_char, c_int, c_void, CStr};

use crate::resource::ffi_bridge::{
    res_DetachResource, res_GetResource, InstallResTypeVectors, LoadResourceFromPath,
};
use crate::resource::ffi_types::ResourceData;
use crate::strings::getstr::{_get_binary_table_data, _get_conversation_data, _get_string_data};
use crate::strings::string_table::StringTable;

/// Copy one complete binary entry, retaining the table until the copy finishes.
pub fn load_binary_entry(resource: &CStr, index: u16) -> Option<Vec<u8>> {
    super::owned_table::LoadedBinaryTable::load(resource)?
        .entry(index)
        .map(<[u8]>::to_vec)
}

/// Copy every complete binary entry before releasing its table owner.
pub fn load_binary_entries(resource: &CStr) -> Option<Vec<Vec<u8>>> {
    Some(
        super::owned_table::LoadedBinaryTable::load(resource)?
            .entries()
            .to_vec(),
    )
}

unsafe extern "C" fn get_string_table_file_data(
    pathname: *const c_char,
    resdata: *mut ResourceData,
) {
    (*resdata).ptr = LoadResourceFromPath(pathname, Some(_get_string_data));
}

unsafe extern "C" fn get_binary_table_file_data(
    pathname: *const c_char,
    resdata: *mut ResourceData,
) {
    (*resdata).ptr = LoadResourceFromPath(pathname, Some(_get_binary_table_data));
}

unsafe extern "C" fn free_string_table(data: *mut c_void) -> c_int {
    // SAFETY: the string-resource loader transfers its table to this release callback.
    unsafe { super::string_table::DestroyStringTable(data.cast()) }
}

/// Registers `STRTAB`, `BINTAB` and `CONVERSATION`.
pub(crate) fn install_resource_types() {
    // SAFETY: the type names are NUL-terminated literals and the callbacks
    // match the resource system's signatures.
    unsafe {
        InstallResTypeVectors(
            c"STRTAB".as_ptr(),
            Some(get_string_table_file_data),
            Some(free_string_table),
            None,
        );
        InstallResTypeVectors(
            c"BINTAB".as_ptr(),
            Some(get_binary_table_file_data),
            Some(free_string_table),
            None,
        );
        InstallResTypeVectors(
            c"CONVERSATION".as_ptr(),
            Some(_get_conversation_data),
            Some(free_string_table),
            None,
        );
    }
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
