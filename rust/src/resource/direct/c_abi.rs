//! C ABI form of `LoadDirEntryTable` (`reslib.h`).
//!
//! The function is deliberately not `#[no_mangle]`: `direct.c.o` still
//! exports the C symbol, and two exports of one name would fail the link.
//! Adding `#[no_mangle]` and dropping `direct.c.o` from the native archive
//! is the activation step; both belong to the native-ownership retirement.

use std::ffi::{c_char, c_int, CStr};
use std::ptr::{self, NonNull};

use crate::io::uio_bridge::uio_DirHandle;

use super::{load_dir_entry_table, MatchType};
use crate::resource::string_table::StringTable;

/// Reports a violated caller contract the way the C `assert` did, without
/// unwinding across the C boundary.
fn contract_violation(what: &str) -> ! {
    eprintln!("LoadDirEntryTable: {what}");
    std::process::abort()
}

/// Returns the table of regular files in `dir_handle`/`path` matching
/// `pattern`, or null when none match. The caller owns the table and
/// releases it with `DestroyStringTable`.
///
/// # Safety
///
/// - `dir_handle` must be a live handle from the UIO.
/// - `path` and `pattern` must be non-null, NUL-terminated strings.
/// - `match_type` must be a `match_MatchType` value.
///
/// A null pointer, an unknown `match_type`, or a directory the UIO cannot
/// open or list aborts the process, as the `assert`s in `direct.c` did.
#[allow(
    non_snake_case,
    reason = "the C symbol this function replaces is named LoadDirEntryTable"
)]
pub unsafe extern "C" fn LoadDirEntryTable(
    dir_handle: *mut uio_DirHandle,
    path: *const c_char,
    pattern: *const c_char,
    match_type: c_int,
) -> *mut StringTable {
    let base =
        NonNull::new(dir_handle).unwrap_or_else(|| contract_violation("directory handle is null"));
    if path.is_null() || pattern.is_null() {
        contract_violation("path or pattern is null");
    }
    let match_type =
        MatchType::from_c(match_type).unwrap_or_else(|| contract_violation("unknown match type"));
    match load_dir_entry_table(
        base,
        CStr::from_ptr(path),
        CStr::from_ptr(pattern),
        match_type,
    ) {
        Ok(Some(table)) => table.into_raw(),
        Ok(None) => ptr::null_mut(),
        Err(error) => contract_violation(&error.to_string()),
    }
}
