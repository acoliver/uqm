//! Owned binary palettes used by retained menu animations.

use crate::strings::string_table::{self, StringTableEntry};
use std::{ffi::CStr, ptr::NonNull};

pub struct PaletteCycle {
    entry: NonNull<StringTableEntry>,
}
impl PaletteCycle {
    pub fn load(resource: &CStr) -> Option<Self> {
        let table = crate::resource::ffi_bridge::detach_resource_of_type(resource, "BINTAB")?;
        // SAFETY: the detached binary table transfers its entries and release authority.
        let entry = unsafe { string_table::CaptureStringTable(table.as_ptr().cast()) };
        Some(Self {
            entry: NonNull::new(entry).unwrap_or_else(|| panic!("palette table has no entries")),
        })
    }
    pub fn apply(&self) {
        // SAFETY: this owner keeps the table and its selected binary entry allocated.
        let entry = unsafe { self.entry.as_ref() };
        let length = usize::try_from(entry.length)
            .unwrap_or_else(|e| panic!("invalid palette entry length: {e}"));
        assert!(length >= 2, "palette range is truncated");
        // SAFETY: the binary loader owns exactly length bytes for this entry.
        let bytes = unsafe { std::slice::from_raw_parts(entry.data.cast::<u8>(), length) };
        assert!(bytes.len() >= 2, "palette range is truncated");
        assert!(bytes[0] <= bytes[1], "palette range is reversed");
        let required = 2 + (usize::from(bytes[1]) - usize::from(bytes[0]) + 1) * 256 * 3;
        assert!(bytes.len() >= required, "palette colors are truncated");
        // SAFETY: the entry contains its complete palette range, validated above.
        unsafe { super::cmap_abi::SetColorMap(bytes.as_ptr()) };
    }
    pub fn advance(&mut self) {
        // SAFETY: relative indexing stays inside this retained table and wraps at its ends.
        self.entry =
            NonNull::new(unsafe { string_table::SetRelStringTableIndex(self.entry.as_ptr(), 1) })
                .unwrap_or_else(|| panic!("palette cycle lost its entry"));
    }
}
impl Drop for PaletteCycle {
    fn drop(&mut self) {
        // SAFETY: this value has the detached table's exclusive release authority.
        unsafe {
            string_table::FreeStringTable(string_table::ReleaseStringTable(self.entry.as_ptr()))
        };
    }
}
