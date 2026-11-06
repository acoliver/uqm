//! Exclusive ownership and bounded text views of a detached text table.

use super::string_table::{StringTable, StringTableEntry};
use std::ffi::CStr;

// The existing ABI observes only the header; Rust owns the entries and their bytes.
#[repr(C)]
pub(crate) struct TextAllocation {
    header: StringTable,
    entries: Box<[StringTableEntry]>,
    strings: Vec<Vec<u8>>,
}

pub struct LoadedTextTable(Box<TextAllocation>);

impl LoadedTextTable {
    pub fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "STRTAB").map(|table| {
            // SAFETY: the plain text loader transfers one boxed TextAllocation with its header first.
            Self(unsafe { Box::from_raw(table.as_ptr().cast()) })
        })
    }

    pub(super) fn from_strings(mut strings: Vec<Vec<u8>>) -> Self {
        assert!(strings.len() <= i32::MAX as usize, "too many text entries");
        let mut entries: Box<[_]> = strings
            .iter_mut()
            .enumerate()
            .map(|(index, text)| {
                if !text.is_empty() {
                    CStr::from_bytes_until_nul(text)
                        .unwrap_or_else(|error| panic!("loaded text is not terminated: {error}"));
                }
                StringTableEntry {
                    data: if text.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        text.as_mut_ptr().cast()
                    },
                    length: i32::try_from(text.len())
                        .unwrap_or_else(|error| panic!("text entry too long: {error}")),
                    index: index as i32,
                    parent: std::ptr::null_mut(),
                }
            })
            .collect();
        let mut allocation = Box::new(TextAllocation {
            header: StringTable {
                flags: 0,
                size: strings.len() as i32,
                strings: entries.as_mut_ptr(),
                name_index: std::ptr::null_mut(),
                owned_text: true,
            },
            entries,
            strings,
        });
        let parent = std::ptr::from_mut(&mut allocation.header);
        for entry in &mut allocation.entries {
            entry.parent = parent;
        }
        Self(allocation)
    }

    pub(super) fn into_raw(self) -> *mut StringTable {
        Box::into_raw(self.0).cast()
    }

    pub fn as_raw(&self) -> *mut StringTableEntry {
        self.0.entries.as_ptr().cast_mut()
    }

    pub fn text(&self, index: usize) -> Option<&CStr> {
        let text = &self.0.strings[index];
        (!text.is_empty()).then(|| {
            CStr::from_bytes_until_nul(text)
                .unwrap_or_else(|error| panic!("validated text lost its terminator: {error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_views_use_entry_bounds_and_preserve_embedded_terminators() {
        let owner = LoadedTextTable::from_strings(vec![b"ab\0cd\0".to_vec(), Vec::new()]);
        assert_eq!(owner.text(0), Some(c"ab"));
        assert_eq!(owner.text(1), None);
        assert!(std::panic::catch_unwind(|| owner.text(2)).is_err());
        assert_eq!(owner.0.entries[0].length, 6);
        assert_eq!(
            owner.0.entries[0].parent,
            std::ptr::from_ref(&owner.0.header).cast_mut()
        );
    }

    #[test]
    fn native_handoff_preserves_storage_until_the_abi_release() {
        let owner = LoadedTextTable::from_strings(vec![c"ab".to_bytes_with_nul().to_vec()]);
        let entries = owner.as_raw();
        let raw = owner.into_raw();
        // SAFETY: the loader handoff transfers the sole boxed table and its owned entries.
        unsafe {
            assert_eq!((*raw).strings, entries);
            assert_eq!(CStr::from_ptr((*entries).data), c"ab");
            super::super::string_table::FreeStringTable(raw);
        }
    }
}
