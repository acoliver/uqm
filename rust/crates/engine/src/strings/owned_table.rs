//! Shared ownership of detached string tables and bounded entry views.

use super::string_table::{StringTable, StringTableEntry};
use std::{ffi::CStr, rc::Rc};

// The existing ABI observes only the header; Rust owns the entries and their bytes.
#[repr(C)]
pub(crate) struct TableAllocation {
    header: StringTable,
    entries: Box<[StringTableEntry]>,
    strings: Vec<Vec<u8>>,
    counts: Vec<u16>,
}

impl TableAllocation {
    fn new(mut strings: Vec<Vec<u8>>, counts: Vec<u16>) -> Rc<Self> {
        assert!(
            strings.len() <= i32::MAX as usize,
            "too many string entries"
        );
        let mut entries: Box<[_]> = strings
            .iter_mut()
            .enumerate()
            .map(|(index, bytes)| StringTableEntry {
                data: if bytes.is_empty() {
                    std::ptr::null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                },
                length: i32::try_from(bytes.len())
                    .unwrap_or_else(|error| panic!("string entry too long: {error}")),
                index: index as i32,
                parent: std::ptr::null_mut(),
            })
            .collect();
        let mut allocation = Rc::new(Self {
            header: StringTable {
                flags: 0,
                size: strings.len() as i32,
                strings: entries.as_mut_ptr(),
                name_index: std::ptr::null_mut(),
                owned_text: true,
            },
            entries,
            strings,
            counts,
        });
        let unique = Rc::get_mut(&mut allocation)
            .unwrap_or_else(|| panic!("new string allocation must be unique"));
        let parent = std::ptr::from_mut(&mut unique.header);
        for entry in &mut unique.entries {
            entry.parent = parent;
        }
        allocation
    }
}

#[derive(Clone)]
pub(super) struct LoadedBinaryTable(Rc<TableAllocation>);

impl LoadedBinaryTable {
    pub(super) fn from_entries(entries: Vec<Vec<u8>>) -> Self {
        let counts = vec![0; entries.len()];
        Self(TableAllocation::new(entries, counts))
    }

    pub(super) fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "BINTAB").map(|table| {
            // SAFETY: the binary loader transfers one Rc<TableAllocation>, header first.
            Self(unsafe { Rc::from_raw(table.as_ptr().cast()) })
        })
    }

    pub(super) fn entries(&self) -> &[Vec<u8>] {
        &self.0.strings
    }

    pub(super) fn entry(&self, index: u16) -> Option<&[u8]> {
        (!self.0.strings.is_empty())
            .then(|| self.0.strings[usize::from(index) % self.0.strings.len()].as_slice())
    }

    pub(super) fn into_raw(self) -> *mut StringTable {
        Rc::into_raw(self.0).cast_mut().cast()
    }
}

#[derive(Clone)]
pub struct LoadedTextTable(Rc<TableAllocation>);

impl std::fmt::Debug for LoadedTextTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedTextTable")
            .field("entries", &self.0.strings.len())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct TextEntry {
    table: LoadedTextTable,
    index: usize,
}
impl TextEntry {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn next_entry(&self) -> Self {
        Self {
            table: self.table.clone(),
            index: (self.index + 1) % self.table.0.entries.len(),
        }
    }

    pub fn record(&self) -> &StringTableEntry {
        &self.table.0.entries[self.index]
    }
    pub fn as_raw(&self) -> *mut StringTableEntry {
        self.table.as_raw().wrapping_add(self.index)
    }
    pub fn label(&self, index: u16) -> (&CStr, u16) {
        self.table.label(index)
    }

    pub fn current_label(&self) -> (&CStr, u16) {
        (
            self.table.text(self.index).unwrap_or(c""),
            self.table.0.counts[self.index],
        )
    }
}

impl LoadedTextTable {
    pub fn new(strings: Vec<std::ffi::CString>) -> Self {
        Self::from_strings(
            strings
                .into_iter()
                .map(|text| text.into_bytes_with_nul())
                .collect(),
        )
    }

    pub fn entry(&self, index: u16) -> Option<TextEntry> {
        (!self.0.entries.is_empty()).then(|| TextEntry {
            table: self.clone(),
            index: usize::from(index) % self.0.entries.len(),
        })
    }
    pub fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "STRTAB").map(|table| {
            // SAFETY: the plain text loader transfers one Rc<TableAllocation>, header first.
            Self(unsafe { Rc::from_raw(table.as_ptr().cast()) })
        })
    }

    pub(super) fn from_strings(strings: Vec<Vec<u8>>) -> Self {
        let counts = strings
            .iter()
            .map(|text| {
                if !text.is_empty() {
                    CStr::from_bytes_until_nul(text)
                        .unwrap_or_else(|error| panic!("loaded text is not terminated: {error}"));
                }
                super::unicode::count_utf8_bytes(text) as u16
            })
            .collect();
        Self(TableAllocation::new(strings, counts))
    }

    pub(crate) fn into_raw(self) -> *mut StringTable {
        Rc::into_raw(self.0).cast_mut().cast()
    }

    pub fn as_raw(&self) -> *mut StringTableEntry {
        self.0.entries.as_ptr().cast_mut()
    }

    pub fn len(&self) -> usize {
        self.0.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.strings.is_empty()
    }

    pub fn text(&self, index: usize) -> Option<&CStr> {
        let text = &self.0.strings[index];
        (!text.is_empty()).then(|| {
            CStr::from_bytes_until_nul(text)
                .unwrap_or_else(|error| panic!("validated text lost its terminator: {error}"))
        })
    }

    pub fn label(&self, index: u16) -> (&CStr, u16) {
        if self.0.strings.is_empty() {
            return (c"", 0);
        }
        let index = usize::from(index) % self.0.strings.len();
        (self.text(index).unwrap_or(c""), self.0.counts[index])
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(strings: Vec<Vec<u8>>) -> Self {
        Self::from_strings(strings)
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
    fn binary_views_retain_full_entries_and_native_release_preserves_the_owner() {
        let large = vec![0x7b; 65537];
        let owner = LoadedBinaryTable::from_entries(vec![large.clone(), Vec::new(), vec![1, 0, 4]]);
        assert_eq!(owner.entry(3), Some(large.as_slice()));
        assert_eq!(owner.entry(1), Some([].as_slice()));
        let raw = owner.clone().into_raw();
        // SAFETY: this handoff transfers one strong reference while owner retains another.
        unsafe {
            assert_eq!((*raw).size, 3);
            let entries = std::slice::from_raw_parts((*raw).strings, 3);
            assert_eq!(entries[0].length, 65537);
            assert_eq!(entries[0].parent, raw);
            assert_eq!(entries[2].index, 2);
            assert_eq!(
                std::slice::from_raw_parts(entries[2].data.cast::<u8>(), 3),
                [1, 0, 4]
            );
            super::super::string_table::FreeStringTable(raw);
        }
        assert_eq!(owner.entries(), [large, Vec::new(), vec![1, 0, 4]]);
        assert!(LoadedBinaryTable::from_entries(Vec::new())
            .entry(0)
            .is_none());
    }

    #[test]
    fn labels_wrap_and_keep_unicode_character_counts_after_native_release() {
        let owner = LoadedTextTable::from_strings(vec![
            b"ab\0cd\0".to_vec(),
            Vec::new(),
            "éclair\0".as_bytes().to_vec(),
        ]);
        let raw = owner.clone().into_raw();
        // SAFETY: the raw handoff owns one strong reference, independently of owner.
        unsafe { super::super::string_table::FreeStringTable(raw) };
        assert_eq!(owner.label(3), (c"ab", 2));
        assert_eq!(owner.label(1), (c"", 0));
        assert_eq!(owner.label(2), (c"éclair", 6));
        let selected = owner.entry(2).unwrap();
        drop(owner);
        assert_eq!(selected.current_label(), (c"éclair", 6));
        assert_eq!(selected.next_entry().current_label(), (c"ab", 2));
    }

    #[test]
    fn native_handoff_preserves_storage_until_the_abi_release() {
        let owner = LoadedTextTable::from_strings(vec![c"ab".to_bytes_with_nul().to_vec()]);
        let entries = owner.as_raw();
        let raw = owner.into_raw();
        // SAFETY: the loader handoff transfers one table reference and its owned entries.
        unsafe {
            assert_eq!((*raw).strings, entries);
            assert_eq!(CStr::from_ptr((*entries).data), c"ab");
            super::super::string_table::FreeStringTable(raw);
        }
    }
}
