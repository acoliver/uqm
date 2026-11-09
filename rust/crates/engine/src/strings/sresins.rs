//! Registration of the string table resource types and instance loading.

#[cfg(test)]
#[path = "sresins/tests.rs"]
mod tests;

use std::ffi::{c_char, c_int, CStr};

use crate::resource::ffi_bridge::{
    res_DetachResource, res_GetResource, FreeResourceData, InstallResTypeVectors,
    LoadResourceFromPath,
};
use crate::resource::ffi_types::ResourceData;
use crate::strings::getstr::{_get_binary_table_data, _get_conversation_data, _get_string_data};
use crate::strings::string_table::StringTable;

/// Copy one binary-table entry and release the detached native table.
pub fn load_binary_entry(resource: &CStr, index: u16) -> Option<Vec<u8>> {
    load_binary(resource, Some(index))?.pop()
}

/// Copy every binary entry before releasing the detached native table.
pub fn load_binary_entries(resource: &CStr) -> Option<Vec<Vec<u8>>> {
    load_binary(resource, None)
}

fn load_binary(resource: &CStr, index: Option<u16>) -> Option<Vec<Vec<u8>>> {
    let table = crate::resource::ffi_bridge::detach_resource_of_type(resource, "BINTAB")?;
    // SAFETY: the binary loader transfers the table and its complete entry allocations.
    let bytes = unsafe { copy_binary_entries(&*table.as_ptr().cast::<StringTable>(), index) };
    // SAFETY: the copied bytes no longer borrow the uniquely detached table.
    unsafe { super::string_table::FreeStringTable(table.as_ptr().cast()) };
    Some(bytes)
}

unsafe fn copy_binary_entries(table: &StringTable, index: Option<u16>) -> Vec<Vec<u8>> {
    let count = usize::try_from(table.size)
        .unwrap_or_else(|error| panic!("binary table has an invalid entry count: {error}"));
    if count == 0 {
        return Vec::new();
    }
    // SAFETY: the detached loader table owns exactly `size` initialized entries.
    let entries = unsafe { std::slice::from_raw_parts(table.strings, count) };
    let range = index.map_or(0..count, |index| {
        let index = usize::from(index) % count;
        index..index + 1
    });
    range
        .map(|index| {
            let entry = &entries[index];
            // The ABI's u16 length query truncates large palettes.
            let length = usize::try_from(entry.length)
                .unwrap_or_else(|error| panic!("binary entry has an invalid length: {error}"));
            if length == 0 {
                Vec::new()
            } else {
                // SAFETY: the loader owns the entry's full recorded allocation until copying ends.
                unsafe { std::slice::from_raw_parts(entry.data.cast::<u8>(), length).to_vec() }
            }
        })
        .collect()
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
            Some(_get_conversation_data),
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
