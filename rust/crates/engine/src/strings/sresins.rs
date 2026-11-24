//! Copies of whole `BINTAB` resource entries.

use std::ffi::CStr;

/// Copy one complete binary entry, retaining the table until the copy finishes.
pub fn load_binary_entry(
    content: &crate::resource::Content,
    resource: &CStr,
    index: u16,
) -> Result<Vec<u8>, crate::resource::AssetLoadError> {
    let table = super::owned_table::LoadedBinaryTable::load(content, resource)?;
    table.entry(index).map(<[u8]>::to_vec).ok_or_else(|| {
        let resolved = content
            .require(resource, crate::resource::AssetKind::Binary)
            .unwrap_or_else(|error| panic!("loaded binary definition disappeared: {error}"));
        resolved.load_error(
            resource,
            crate::resource::FileError::invalid(resolved.descriptor, "binary table has no entries"),
        )
    })
}

/// Copy every complete binary entry before releasing its table owner.
pub fn load_binary_entries(
    content: &crate::resource::Content,
    resource: &CStr,
) -> Result<Vec<Vec<u8>>, crate::resource::AssetLoadError> {
    Ok(
        super::owned_table::LoadedBinaryTable::load(content, resource)?
            .entries()
            .to_vec(),
    )
}
