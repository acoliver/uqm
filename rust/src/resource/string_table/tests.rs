//! The C `STRING_TABLE` layout and the owned table, checked against a
//! transcription of `FreeStringTable` and independent byte offsets.

use std::ffi::{c_int, c_void, CStr, CString};

use super::{OwnedStringTable, StringTable, StringTableEntry, StringTableError, MAX_ENTRIES};
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

/// The table for `names`, `None` when there are none; inside the size limit.
fn build_table(names: &[CString]) -> Option<OwnedStringTable> {
    OwnedStringTable::from_names(names).expect("entry count within the C limit")
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
    let table = build_table(&names(&expected))
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
    let table = build_table(&names(&[b"one.mle", b"two.mle"])).unwrap();
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
    assert!(build_table(&[]).is_none());
}

#[test]
fn a_table_nobody_took_is_freed_when_dropped() {
    drop(build_table(&names(&[b"one.mle"])).unwrap());
}

// --- representability and ownership ----------------------------------------

/// What a C reader sees of a table: `GetStringTableCount` returns the size as
/// a `COUNT` (`UWORD`), and `SetAbsStringTableIndex` indexes with one.
const C_COUNT_MAX: usize = u16::MAX as usize;

fn numbered_names(count: usize) -> Vec<CString> {
    (0..count)
        .map(|n| CString::new(format!("n{n}")).unwrap())
        .collect()
}

/// Walks a raw table the way the C readers do and returns each entry's bytes
/// (without the NUL), checking length, index and parent on the way.
///
/// # Safety
///
/// `table` must be a live table with `size` valid entries.
#[cfg(target_pointer_width = "64")]
unsafe fn walk(table: *mut StringTable) -> Vec<Vec<u8>> {
    let base = table.cast::<u8>();
    let size: c_int = read_at(base, 4);
    let strings: *mut u8 = read_at(base, 8);
    (0..size as usize)
        .map(|slot| {
            let entry = strings.add(slot * 24);
            let data: *const u8 = read_at(entry, 0);
            let length: c_int = read_at(entry, 8);
            assert_eq!(read_at::<c_int>(entry, 12), slot as c_int, "index");
            assert_eq!(read_at::<*mut u8>(entry, 16), table.cast::<u8>(), "parent");
            let stored = std::slice::from_raw_parts(data, length as usize);
            assert_eq!(stored.last(), Some(&0), "length counts the NUL");
            let body = &stored[..stored.len() - 1];
            assert!(!body.contains(&0), "no NUL before the terminator");
            body.to_vec()
        })
        .collect()
}

#[cfg(target_pointer_width = "64")]
#[test]
fn a_table_of_the_largest_c_count_keeps_every_entry() {
    let names = numbered_names(C_COUNT_MAX);
    let table = build_table(&names).expect("entries");
    assert_eq!(table.len(), C_COUNT_MAX);
    let raw = table.into_raw();
    unsafe {
        assert_eq!(read_at::<c_int>(raw.cast::<u8>(), 4), C_COUNT_MAX as c_int);
        let entries = walk(raw);
        assert_eq!(entries.len(), C_COUNT_MAX);
        for (entry, name) in entries.iter().zip(&names) {
            assert_eq!(entry.as_slice(), name.as_bytes());
        }
        c_free_string_table(raw);
    }
}

#[test]
fn more_names_than_a_c_count_can_hold_are_refused_not_wrapped() {
    let names = numbered_names(C_COUNT_MAX + 1);
    assert_eq!(
        OwnedStringTable::from_names(&names).err(),
        Some(StringTableError::TooManyEntries {
            count: C_COUNT_MAX + 1
        })
    );
    // A size of 65536 is what C would have read as 0.
    assert_eq!(C_COUNT_MAX as u16 as usize, C_COUNT_MAX);
    assert_eq!((C_COUNT_MAX + 1) as u16, 0);
}

#[test]
fn the_limit_is_the_c_count_range_not_a_project_threshold() {
    assert_eq!(MAX_ENTRIES, C_COUNT_MAX);
    assert!(OwnedStringTable::from_names(&numbered_names(MAX_ENTRIES)).is_ok());
    assert!(OwnedStringTable::from_names(&numbered_names(MAX_ENTRIES + 1)).is_err());
    assert_eq!(
        OwnedStringTable::from_names(&numbered_names(3 * MAX_ENTRIES)).err(),
        Some(StringTableError::TooManyEntries {
            count: 3 * MAX_ENTRIES
        })
    );
}

#[test]
fn the_refusal_says_how_many_entries_were_offered() {
    let error = StringTableError::TooManyEntries { count: 70_000 };
    assert_eq!(
        error.to_string(),
        "70000 entries exceed the 65535 a string table can address"
    );
}

#[cfg(target_pointer_width = "64")]
#[test]
fn entries_are_contiguous_with_distinct_stable_data_blocks() {
    let names = numbered_names(40);
    let raw = build_table(&names).unwrap().into_raw();
    unsafe {
        let strings: *mut u8 = read_at(raw.cast::<u8>(), 8);
        let pointers = |_: ()| -> Vec<*const u8> {
            (0..names.len())
                .map(|slot| read_at::<*const u8>(strings, slot * 24))
                .collect()
        };
        let first = pointers(());
        let mut sorted = first.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "every entry owns its block");
        assert!(first.iter().all(|data| !data.is_null()));
        assert_eq!(walk(raw).len(), names.len());
        assert_eq!(pointers(()), first, "reading never moves a block");
        c_free_string_table(raw);
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn an_empty_name_is_one_nul_byte() {
    let raw = build_table(&names(&[b"", b"x"])).unwrap().into_raw();
    unsafe {
        let strings: *mut u8 = read_at(raw.cast::<u8>(), 8);
        assert_eq!(read_at::<c_int>(strings, 8), 1, "length of the empty name");
        assert_eq!(walk(raw), vec![b"".to_vec(), b"x".to_vec()]);
        c_free_string_table(raw);
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn a_long_name_is_stored_whole_with_its_terminator() {
    let long = vec![b'q'; 4096];
    let raw = build_table(&[CString::new(long.clone()).unwrap()])
        .unwrap()
        .into_raw();
    unsafe {
        let strings: *mut u8 = read_at(raw.cast::<u8>(), 8);
        assert_eq!(read_at::<c_int>(strings, 8), 4097);
        assert_eq!(walk(raw), vec![long]);
        c_free_string_table(raw);
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn a_table_handed_to_c_survives_a_panic_until_c_releases_it() {
    let handed_out = std::cell::Cell::new(0usize);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = build_table(&names(&[b"keep.mle"])).unwrap().into_raw();
        handed_out.set(raw as usize);
        panic!("the Rust caller fails after handing the table to C");
    }));
    assert!(outcome.is_err());
    let raw = handed_out.get() as *mut StringTable;
    unsafe {
        assert_eq!(walk(raw), vec![b"keep.mle".to_vec()]);
        c_free_string_table(raw);
    }
}

#[test]
fn an_owned_table_is_freed_by_unwinding() {
    let outcome = std::panic::catch_unwind(|| {
        let table = build_table(&names(&[b"a", b"b"])).unwrap();
        assert_eq!(table.len(), 2);
        panic!("fails while the table is live");
    });
    assert!(outcome.is_err());
}

#[test]
fn names_cut_at_a_nul_are_stored_up_to_it() {
    let cut = CStr::from_bytes_until_nul(b"head\0tail\0")
        .unwrap()
        .to_owned();
    assert!(CString::new(&b"head\0tail"[..]).is_err());
    let raw = build_table(&[cut]).unwrap().into_raw();
    unsafe {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(walk(raw), vec![b"head".to_vec()]);
        c_free_string_table(raw);
    }
}

#[cfg(target_pointer_width = "64")]
mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Any sequence of byte-string names, including high bytes and the
        /// empty name, comes back from a C-style walk unchanged and in order.
        #[test]
        fn every_name_round_trips_through_the_c_layout(
            raw_names in proptest::collection::vec(
                proptest::collection::vec(1u8..=255, 0..48), 1..64),
        ) {
            let owned: Vec<CString> =
                raw_names.iter().map(|n| CString::new(n.clone()).unwrap()).collect();
            let raw = build_table(&owned).unwrap().into_raw();
            unsafe {
                prop_assert_eq!(walk(raw), raw_names);
                c_free_string_table(raw);
            }
        }
    }
}
