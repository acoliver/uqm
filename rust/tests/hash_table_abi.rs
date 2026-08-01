use std::ffi::{c_char, c_int, c_void};
use std::mem::{align_of, size_of};

use memoffset::offset_of;
use uqm_rust::collections::hash_table::{HashEntry, HashIterator, HashTable};

mod retained_headers {
    include!(concat!(env!("OUT_DIR"), "/hash_table_abi.rs"));
}

#[test]
fn retained_headers_declare_every_real_exported_signature() {
    use retained_headers as abi;

    let _: unsafe extern "C" fn(
        abi::CharHashTableHashFunction,
        abi::CharHashTableEqualFunction,
        abi::CharHashTableCopyFunction,
        abi::CharHashTableFreeKeyFunction,
        abi::CharHashTableFreeValueFunction,
        u32,
        f64,
        f64,
    ) -> *mut abi::CharHashTableHashTable = abi::CharHashTablenewHashTable;
    let _: unsafe extern "C" fn(
        *mut abi::CharHashTableHashTable,
        *const c_char,
        *mut c_void,
    ) -> bool = abi::CharHashTableadd;
    let _: unsafe extern "C" fn(*mut abi::CharHashTableHashTable, *const c_char) -> bool =
        abi::CharHashTableremove;
    let _: unsafe extern "C" fn(*mut abi::CharHashTableHashTable, *const c_char) -> *mut c_void =
        abi::CharHashTablefind;
    let _: unsafe extern "C" fn(*const abi::CharHashTableHashTable) -> u32 =
        abi::CharHashTablecount;
    let _: unsafe extern "C" fn(*mut abi::CharHashTableHashTable) =
        abi::CharHashTabledeleteHashTable;
    let _: unsafe extern "C" fn(
        *const abi::CharHashTableHashTable,
    ) -> *mut abi::CharHashTableIterator = abi::CharHashTablegetIterator;
    let _: unsafe extern "C" fn(*const abi::CharHashTableIterator) -> c_int =
        abi::CharHashTableiteratorDone;
    let _: unsafe extern "C" fn(*mut abi::CharHashTableIterator) -> *mut c_char =
        abi::CharHashTableiteratorKey;
    let _: unsafe extern "C" fn(*mut abi::CharHashTableIterator) -> *mut c_void =
        abi::CharHashTableiteratorValue;
    let _: unsafe extern "C" fn(
        *mut abi::CharHashTableIterator,
    ) -> *mut abi::CharHashTableIterator = abi::CharHashTableiteratorNext;
    let _: unsafe extern "C" fn(*mut abi::CharHashTableIterator) = abi::CharHashTablefreeIterator;

    let _: unsafe extern "C" fn(
        abi::StringHashTableHashFunction,
        abi::StringHashTableEqualFunction,
        abi::StringHashTableCopyFunction,
        abi::StringHashTableFreeKeyFunction,
        abi::StringHashTableFreeValueFunction,
        u32,
        f64,
        f64,
    ) -> *mut abi::StringHashTableHashTable = abi::StringHashTablenewHashTable;
    let _: unsafe extern "C" fn(
        *mut abi::StringHashTableHashTable,
        *const c_char,
        *mut abi::StringHashTableValue,
    ) -> bool = abi::StringHashTableadd;
    let _: unsafe extern "C" fn(*mut abi::StringHashTableHashTable, *const c_char) -> bool =
        abi::StringHashTableremove;
    let _: unsafe extern "C" fn(
        *mut abi::StringHashTableHashTable,
        *const c_char,
    ) -> *mut abi::StringHashTableValue = abi::StringHashTablefind;
    let _: unsafe extern "C" fn(*const abi::StringHashTableHashTable) -> u32 =
        abi::StringHashTablecount;
    let _: unsafe extern "C" fn(*mut abi::StringHashTableHashTable) =
        abi::StringHashTabledeleteHashTable;
    let _: unsafe extern "C" fn(
        *const abi::StringHashTableHashTable,
    ) -> *mut abi::StringHashTableIterator = abi::StringHashTablegetIterator;
    let _: unsafe extern "C" fn(*const abi::StringHashTableIterator) -> c_int =
        abi::StringHashTableiteratorDone;
    let _: unsafe extern "C" fn(*mut abi::StringHashTableIterator) -> *mut c_char =
        abi::StringHashTableiteratorKey;
    let _: unsafe extern "C" fn(
        *mut abi::StringHashTableIterator,
    ) -> *mut abi::StringHashTableValue = abi::StringHashTableiteratorValue;
    let _: unsafe extern "C" fn(
        *mut abi::StringHashTableIterator,
    ) -> *mut abi::StringHashTableIterator = abi::StringHashTableiteratorNext;
    let _: unsafe extern "C" fn(*mut abi::StringHashTableIterator) =
        abi::StringHashTablefreeIterator;
}

#[test]
fn retained_header_layout_matches_rust_on_supported_64_bit_targets() {
    use retained_headers as abi;

    assert_eq!(size_of::<*const c_void>(), 8);
    assert_eq!(size_of::<HashTable>(), 88);
    assert_eq!(align_of::<HashTable>(), 8);
    assert_eq!(
        size_of::<abi::CharHashTableHashTable>(),
        size_of::<HashTable>()
    );
    assert_eq!(
        align_of::<abi::CharHashTableHashTable>(),
        align_of::<HashTable>()
    );
    assert_eq!(
        size_of::<abi::StringHashTableHashTable>(),
        size_of::<HashTable>()
    );
    assert_eq!(
        align_of::<abi::StringHashTableHashTable>(),
        align_of::<HashTable>()
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashTable, hash_function),
        offset_of!(HashTable, hash_function)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashTable, min_fill_quotient),
        offset_of!(HashTable, min_fill_quotient)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashTable, entries),
        offset_of!(HashTable, entries)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashTable, num_entries),
        offset_of!(HashTable, num_entries)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashTable, num_collisions),
        offset_of!(HashTable, num_collisions)
    );

    assert_eq!(size_of::<HashEntry>(), 32);
    assert_eq!(align_of::<HashEntry>(), 8);
    assert_eq!(
        size_of::<abi::CharHashTableHashEntry>(),
        size_of::<HashEntry>()
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashEntry, hash),
        offset_of!(HashEntry, hash)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashEntry, key),
        offset_of!(HashEntry, key)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashEntry, value),
        offset_of!(HashEntry, value)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableHashEntry, next),
        offset_of!(HashEntry, next)
    );

    assert_eq!(size_of::<HashIterator>(), 24);
    assert_eq!(align_of::<HashIterator>(), 8);
    assert_eq!(
        size_of::<abi::CharHashTableIterator>(),
        size_of::<HashIterator>()
    );
    assert_eq!(
        offset_of!(abi::CharHashTableIterator, hash_table),
        offset_of!(HashIterator, hash_table)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableIterator, bucket_nr),
        offset_of!(HashIterator, bucket_nr)
    );
    assert_eq!(
        offset_of!(abi::CharHashTableIterator, entry),
        offset_of!(HashIterator, entry)
    );
}

#[test]
fn retained_header_declarations_link_to_and_execute_rust_exports() {
    unsafe {
        let char_table =
            retained_headers::CharHashTablenewHashTable(None, None, None, None, None, 0, 0.85, 0.9);
        let char_count = retained_headers::CharHashTablecount(char_table);
        retained_headers::CharHashTabledeleteHashTable(char_table);
        assert_eq!(char_count, 0);

        let string_table = retained_headers::StringHashTablenewHashTable(
            None, None, None, None, None, 0, 0.85, 0.9,
        );
        let string_count = retained_headers::StringHashTablecount(string_table);
        retained_headers::StringHashTabledeleteHashTable(string_table);
        assert_eq!(string_count, 0);
    }
}
