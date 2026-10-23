//! Loading a string table straight from a file (`LoadStringTableFile`).

use std::ffi::{c_char, c_void};
use std::ptr::{addr_of_mut, null, null_mut};

use crate::resource::ffi_bridge::{
    _cur_resfile_name, res_CloseResFile, res_OpenResFile, LengthResFile,
};
use crate::strings::getstr::_get_string_data;
use crate::strings::string_table::StringTable;

/// Loads a `STRTAB`-format file. Returns NULL if the file cannot be opened
/// or another resource load is in progress.
///
/// # Safety
/// `dir` must be a UIO directory handle and `file_name` NUL-terminated and
/// valid until the load returns.
#[no_mangle]
pub unsafe extern "C" fn LoadStringTableFile(
    dir: *mut c_void,
    file_name: *const c_char,
) -> *mut StringTable {
    // No lock: loads happen on the main thread, as in the original.
    if !(*addr_of_mut!(_cur_resfile_name)).is_null() {
        return null_mut();
    }

    let fp = res_OpenResFile(dir, file_name, c"rb".as_ptr());
    if fp.is_null() {
        return null_mut();
    }
    *addr_of_mut!(_cur_resfile_name) = file_name;
    let table = _get_string_data(fp, LengthResFile(fp) as u32);
    *addr_of_mut!(_cur_resfile_name) = null();
    res_CloseResFile(fp);
    table.cast()
}
