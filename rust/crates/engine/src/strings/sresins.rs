//! Copies of whole `BINTAB` resource entries.

use std::ffi::CStr;

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
