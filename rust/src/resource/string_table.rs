//! The C `STRING_TABLE` layout, and an owned table built in Rust.
//!
//! [`StringTable`] and [`StringTableEntry`] mirror `struct string_table` and
//! `struct string_table_entry` in `sc2/src/libs/strings/strintrn.h`. They are
//! the one Rust description of that ABI: the directory provider builds
//! tables with them, and the sound loader reads and fills tables that C's
//! `AllocStringTable` made. C consumers (`gfxload.c`, `loadmele.c`, the
//! sound and comm code) read and release a table with `CaptureStringTable`,
//! `SetAbsStringTableIndex`, `GetStringAddress` and `DestroyStringTable`, so
//! a table built here must be shaped exactly as `AllocStringTable` shapes it
//! and every block must come from the allocator that `FreeStringTable`
//! releases with.
//!
//! This is not [`super::string_bank`]: a string bank hands out strings from
//! append-only chunks and has no table, entry or parent pointers.
//!
//! Production builds define `USE_RUST_MEM`, which maps `HMalloc`/`HFree` to
//! `rust_hmalloc`/`rust_hfree`. Allocating here with those functions
//! therefore produces blocks the existing C `FreeStringTable` can free.

use std::ffi::{c_char, c_int, c_void, CString};
use std::mem::{offset_of, size_of, ManuallyDrop};
use std::ptr::{self, NonNull};

use crate::memory::{rust_hfree, rust_hmalloc};

/// `struct string_table_entry` (`STRING_TABLE_ENTRY_DESC`).
#[repr(C)]
pub struct StringTableEntry {
    /// NUL-terminated bytes; `length` counts the terminator.
    pub data: *mut c_char,
    pub length: c_int,
    pub index: c_int,
    pub parent: *mut StringTable,
}

/// `struct string_table` (`STRING_TABLE_DESC`).
#[repr(C)]
pub struct StringTable {
    pub flags: u16,
    /// Number of entries in `strings`.
    pub size: c_int,
    pub strings: *mut StringTableEntry,
    /// Null unless the table has `HAS_NAMEINDEX`, which Rust-built tables
    /// never set.
    pub name_index: *mut c_void,
}

// The C struct sizes on the LP64 targets the game ships for. A mismatch here
// would silently corrupt every C reader of these tables.
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(size_of::<StringTableEntry>() == 24);
    assert!(offset_of!(StringTableEntry, data) == 0);
    assert!(offset_of!(StringTableEntry, length) == 8);
    assert!(offset_of!(StringTableEntry, index) == 12);
    assert!(offset_of!(StringTableEntry, parent) == 16);
    assert!(size_of::<StringTable>() == 24);
    assert!(offset_of!(StringTable, flags) == 0);
    assert!(offset_of!(StringTable, size) == 4);
    assert!(offset_of!(StringTable, strings) == 8);
    assert!(offset_of!(StringTable, name_index) == 16);
};

/// A table of NUL-terminated names that frees itself unless ownership is
/// handed to C with [`OwnedStringTable::into_raw`].
pub struct OwnedStringTable {
    raw: NonNull<StringTable>,
}

impl OwnedStringTable {
    /// Builds a table holding one entry per name, in order.
    ///
    /// Returns `None` for an empty slice: C callers treat a null table, not
    /// an empty one, as "nothing found".
    pub fn from_names(names: &[CString]) -> Option<Self> {
        if names.is_empty() {
            return None;
        }
        let count = c_int::try_from(names.len()).expect("entry count fits the C int size field");
        // SAFETY: rust_hmalloc aborts instead of returning null, so every
        // pointer written below is a live allocation of the requested size.
        // Each field of the table and of every entry is written before the
        // table is exposed.
        unsafe {
            let table = rust_hmalloc(size_of::<StringTable>()).cast::<StringTable>();
            let strings = rust_hmalloc(size_of::<StringTableEntry>() * names.len())
                .cast::<StringTableEntry>();
            table.write(StringTable {
                flags: 0,
                size: count,
                strings,
                name_index: ptr::null_mut(),
            });
            for (slot, name) in names.iter().enumerate() {
                let bytes = name.as_bytes_with_nul();
                let data = rust_hmalloc(bytes.len()).cast::<u8>();
                ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
                strings.add(slot).write(StringTableEntry {
                    data: data.cast::<c_char>(),
                    length: c_int::try_from(bytes.len())
                        .expect("entry length fits the C int length field"),
                    index: c_int::try_from(slot).expect("slot fits the C int index field"),
                    parent: table,
                });
            }
            Some(OwnedStringTable {
                raw: NonNull::new_unchecked(table),
            })
        }
    }

    pub fn len(&self) -> usize {
        // SAFETY: `raw` points at the table built in `from_names`, which
        // lives until `self` is dropped or consumed.
        let size = unsafe { self.raw.as_ref().size };
        usize::try_from(size).expect("table size is positive")
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Hands the table to a C caller, who releases it with
    /// `DestroyStringTable`.
    pub fn into_raw(self) -> *mut StringTable {
        ManuallyDrop::new(self).raw.as_ptr()
    }
}

impl Drop for OwnedStringTable {
    fn drop(&mut self) {
        // SAFETY: `raw` came from `from_names` and has not been released.
        unsafe { free_table(self.raw.as_ptr()) }
    }
}

/// Releases a table the way `FreeStringTable` does for a table without
/// sound clips or timestamps (`flags == 0`).
///
/// # Safety
///
/// `table` must be a non-null table built by [`OwnedStringTable::from_names`]
/// and not yet released.
unsafe fn free_table(table: *mut StringTable) {
    let StringTable { size, strings, .. } = table.read();
    for slot in 0..usize::try_from(size).expect("table size is positive") {
        rust_hfree((*strings.add(slot)).data.cast::<c_void>());
    }
    rust_hfree(strings.cast::<c_void>());
    rust_hfree(table.cast::<c_void>());
}

#[cfg(test)]
pub(crate) mod tests;
