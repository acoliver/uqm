//! Owned string tables: text (`STRTAB`) and binary (`BINTAB`) entries shared
//! by their owners and the entry views they hand out.

use std::{ffi::CStr, rc::Rc};

/// A table entry a game record names: which table (its identity, never
/// dereferenced) and which entry is current (C: a `STRING` pointer).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntryRef {
    table: usize,
    index: u16,
}

impl EntryRef {
    /// The entry `index` of the table identified by `table`.
    pub fn new(table: usize, index: u16) -> Self {
        Self { table, index }
    }

    pub fn table(self) -> usize {
        self.table
    }

    pub fn index(self) -> u16 {
        self.index
    }

    /// The same table with entry `index` current (C: `SetAbsStringTableIndex`).
    pub fn with_index(self, index: u16) -> Self {
        Self { index, ..self }
    }
}

struct TextData {
    strings: Vec<Vec<u8>>,
    counts: Vec<u16>,
}

#[derive(Clone)]
pub struct LoadedBinaryTable(Rc<Vec<Vec<u8>>>);

impl LoadedBinaryTable {
    pub(super) fn from_entries(entries: Vec<Vec<u8>>) -> Self {
        Self(Rc::new(entries))
    }

    pub(crate) fn load(resource: &CStr) -> Option<Self> {
        super::getstr::load_binary_table(&crate::resource::path_of(resource, "BINTAB")?)
    }

    pub(crate) fn entries(&self) -> &[Vec<u8>] {
        &self.0
    }

    pub(crate) fn entry(&self, index: u16) -> Option<&[u8]> {
        (!self.0.is_empty()).then(|| self.0[usize::from(index) % self.0.len()].as_slice())
    }
}

#[derive(Clone)]
pub struct LoadedTextTable(Rc<TextData>);

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
            index: (self.index + 1) % self.table.len(),
        }
    }

    /// The record form of this entry.
    pub fn entry_ref(&self) -> EntryRef {
        self.table.entry_ref(self.index as u16)
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
        (!self.is_empty()).then(|| TextEntry {
            table: self.clone(),
            index: usize::from(index) % self.len(),
        })
    }

    pub fn load(resource: &CStr) -> Option<Self> {
        Self::load_file(&crate::resource::path_of(resource, "STRTAB")?)
    }

    /// A `STRTAB`-format file in the content directory.
    pub fn load_file(path: &CStr) -> Option<Self> {
        super::getstr::load_text_table(&crate::io::content_dir()?, path)
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
        Self(Rc::new(TextData { strings, counts }))
    }

    /// This table with entry `index` current, as game records name it.
    pub fn entry_ref(&self, index: u16) -> EntryRef {
        EntryRef::new(Rc::as_ptr(&self.0) as usize, index)
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
        if self.is_empty() {
            return (c"", 0);
        }
        let index = usize::from(index) % self.len();
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
    }

    #[test]
    fn binary_views_retain_full_entries_and_wrap() {
        let large = vec![0x7b; 65537];
        let owner = LoadedBinaryTable::from_entries(vec![large.clone(), Vec::new(), vec![1, 0, 4]]);
        assert_eq!(owner.entry(3), Some(large.as_slice()));
        assert_eq!(owner.entry(1), Some([].as_slice()));
        assert_eq!(owner.entries(), [large, Vec::new(), vec![1, 0, 4]]);
        assert!(LoadedBinaryTable::from_entries(Vec::new())
            .entry(0)
            .is_none());
    }

    #[test]
    fn labels_wrap_and_keep_unicode_character_counts_after_the_owner_drops() {
        let owner = LoadedTextTable::from_strings(vec![
            b"ab\0cd\0".to_vec(),
            Vec::new(),
            "éclair\0".as_bytes().to_vec(),
        ]);
        assert_eq!(owner.label(3), (c"ab", 2));
        assert_eq!(owner.label(1), (c"", 0));
        assert_eq!(owner.label(2), (c"éclair", 6));
        let selected = owner.entry(2).unwrap();
        drop(owner);
        assert_eq!(selected.current_label(), (c"éclair", 6));
        assert_eq!(selected.next_entry().current_label(), (c"ab", 2));
    }

    /// #952: a string table loaded from the index belongs to its loader;
    /// dropping it frees its own storage and the resource loads again.
    #[test]
    #[serial_test::serial]
    fn indexed_string_tables_release_by_drop_and_load_again() {
        let mount = crate::io::fixture::TempMount::new().as_content();
        mount.write("text.rmp", b"test.strings = STRTAB:strings.txt\n");
        mount.write("strings.txt", b"#(first)\nhello\n#(second)\nworld\n");
        crate::resource::init();
        crate::resource::load_index(&mount.dir(), c"text.rmp", None);
        for _ in 0..2 {
            let table = LoadedTextTable::load(c"test.strings")
                .unwrap_or_else(|| panic!("indexed string table"));
            assert_eq!(table.text(1), Some(c"world"));
            let storage = Rc::downgrade(&table.0);
            drop(table);
            assert!(storage.upgrade().is_none(), "dropping frees the table");
        }
        assert!(crate::resource::remove("test.strings"));
    }

    #[test]
    fn entry_refs_name_one_table_and_the_current_entry() {
        let owner = LoadedTextTable::new(vec![c"a".to_owned(), c"b".to_owned()]);
        let other = LoadedTextTable::new(vec![c"a".to_owned()]);
        let first = owner.entry_ref(0);
        assert_eq!(owner.entry(1).unwrap().entry_ref(), first.with_index(1));
        assert_eq!(first.index(), 0);
        assert_ne!(first.table(), other.entry_ref(0).table());
        assert_eq!(owner.clone().entry_ref(0), first);
    }
}
