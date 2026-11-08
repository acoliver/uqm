//! Registration of the string table resource types and instance loading.

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
    use super::string_table::{
        CaptureStringTable, FreeStringTable, GetStringAddress, GetStringLengthBin,
        SetAbsStringTableIndex,
    };
    let table = crate::resource::ffi_bridge::detach_resource_of_type(resource, "BINTAB")?;
    // SAFETY: the binary loader transfers this table and all its entry allocations.
    let entry = unsafe { SetAbsStringTableIndex(CaptureStringTable(table.as_ptr().cast()), index) };
    // SAFETY: the detached table keeps its selected entry live.
    let length = usize::from(unsafe { GetStringLengthBin(entry) });
    let bytes = if length == 0 {
        Vec::new()
    } else {
        // SAFETY: a nonempty loaded binary entry contains at least its recorded u16 length.
        unsafe { std::slice::from_raw_parts(GetStringAddress(entry).cast(), length).to_vec() }
    };
    // SAFETY: the copied bytes no longer borrow the uniquely detached table.
    unsafe { FreeStringTable(table.as_ptr().cast()) };
    Some(bytes)
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
