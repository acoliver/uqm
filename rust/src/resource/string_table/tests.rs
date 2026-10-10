//! The C `STRING_TABLE` layout and the owned table, checked against a
//! transcription of `FreeStringTable` and independent byte offsets.

use std::ffi::{c_int, c_void, CStr, CString};

use super::{OwnedStringTable, StringTable, StringTableEntry};
use crate::memory::rust_hfree;

/// Reads a value at a byte offset: the C layout of `strintrn.h` on an LP64
/// target, written out independently of the `repr(C)` structs.
#[cfg(target_pointer_width = "64")]
unsafe fn read_at<T: Copy>(base: *const u8, offset: usize) -> T {
    base.add(offset).cast::<T>().read_unaligned()
}

/// `FreeStringTable` from `strings.c`, transcribed. Releases with the same
/// allocator the C function uses under `USE_RUST_MEM`.
#[cfg(target_pointer_width = "64")]
pub(crate) unsafe fn c_free_string_table(table: *mut StringTable) {
    const HAS_SOUND_CLIPS: u16 = 1 << 0;
    const HAS_TIMESTAMP: u16 = 1 << 1;
    let base = table.cast::<u8>();
    let flags: u16 = read_at(base, 0);
    let size: c_int = read_at(base, 4);
    let strings: *mut u8 = read_at(base, 8);
    let mut multiplier = 1;
    if flags & HAS_SOUND_CLIPS != 0 {
        multiplier += 1;
    }
    if flags & HAS_TIMESTAMP != 0 {
        multiplier += 1;
    }
    for slot in 0..(size * multiplier) as usize {
        let data: *mut c_void = read_at(strings, slot * 24);
        if !data.is_null() {
            rust_hfree(data);
        }
    }
    rust_hfree(strings.cast());
    rust_hfree(table.cast());
}

fn names(list: &[&[u8]]) -> Vec<CString> {
    list.iter()
        .map(|name| CString::new(*name).unwrap())
        .collect()
}

#[cfg(target_pointer_width = "64")]
#[test]
fn the_table_has_the_layout_the_c_string_functions_read() {
    let expected: [&[u8]; 3] = [b"a.mle", b"b.mle", "\u{e9}-\u{df}.mle".as_bytes()];
    let table = OwnedStringTable::from_names(&names(&expected))
        .expect("three entries")
        .into_raw();
    unsafe {
        let base = table.cast::<u8>();
        assert_eq!(read_at::<u16>(base, 0), 0, "flags: no clips, no stamps");
        assert_eq!(read_at::<c_int>(base, 4), 3, "size");
        assert!(read_at::<*mut c_void>(base, 16).is_null(), "no name index");
        let strings: *mut u8 = read_at(base, 8);

        for (slot, name) in expected.iter().enumerate() {
            let entry = strings.add(slot * 24);
            let data: *const u8 = read_at(entry, 0);
            let length: c_int = read_at(entry, 8);
            assert_eq!(length as usize, name.len() + 1, "length counts the NUL");
            let stored = std::slice::from_raw_parts(data, name.len() + 1);
            assert_eq!(&stored[..name.len()], *name);
            assert_eq!(stored[name.len()], 0);
            assert_eq!(read_at::<c_int>(entry, 12), slot as c_int, "index");
            assert_eq!(read_at::<*mut u8>(entry, 16), table.cast::<u8>(), "parent");
        }
        c_free_string_table(table);
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn the_rust_view_of_the_layout_matches_the_table() {
    let table = OwnedStringTable::from_names(&names(&[b"one.mle", b"two.mle"])).unwrap();
    assert_eq!(table.len(), 2);
    assert!(!table.is_empty());
    let raw = table.into_raw();
    unsafe {
        let StringTable {
            flags,
            size,
            strings,
            name_index,
        } = raw.read();
        assert_eq!((flags, size), (0, 2));
        assert!(name_index.is_null());
        let StringTableEntry {
            data,
            length,
            index,
            parent,
        } = strings.add(1).read();
        assert_eq!(CStr::from_ptr(data).to_bytes(), b"two.mle");
        assert_eq!((length, index), (8, 1));
        assert_eq!(parent, raw);
        c_free_string_table(raw);
    }
}

#[test]
fn no_names_make_no_table() {
    assert!(OwnedStringTable::from_names(&[]).is_none());
}

#[test]
fn a_table_nobody_took_is_freed_when_dropped() {
    drop(OwnedStringTable::from_names(&names(&[b"one.mle"])).unwrap());
}
