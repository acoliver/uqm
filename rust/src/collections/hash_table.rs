//! C-compatible string-key hash tables retained by the transitional native ABI.
//!
//! The two exported families intentionally share the historical bucket layout
//! and resizing algorithm. `CharHashTable` owns a libc-allocated copy of each
//! key, while `StringHashTable` borrows key pointers from its caller. Values are
//! always caller-owned in both specializations. The callback fields are retained
//! only for ABI layout compatibility: these two historical macro specializations
//! ignore every supplied callback, including `free_value_function`.

use std::ffi::{c_char, c_int, c_void};
use std::mem;
use std::ptr;

type HashFunction = Option<unsafe extern "C" fn(*const c_char) -> u32>;
type EqualFunction = Option<unsafe extern "C" fn(*const c_char, *const c_char) -> bool>;
type CopyFunction = Option<unsafe extern "C" fn(*const c_char) -> *mut c_void>;
type FreeKeyFunction = Option<unsafe extern "C" fn(*mut c_char)>;
type FreeValueFunction = Option<unsafe extern "C" fn(*mut c_void)>;

#[repr(C)]
pub struct HashTable {
    pub hash_function: HashFunction,
    pub equal_function: EqualFunction,
    pub copy_function: CopyFunction,
    pub free_key_function: FreeKeyFunction,
    pub free_value_function: FreeValueFunction,
    pub min_fill_quotient: f64,
    pub max_fill_quotient: f64,
    pub min_size: u32,
    pub max_size: u32,
    pub size: u32,
    pub hash_mask: u32,
    pub entries: *mut *mut HashEntry,
    pub num_entries: u32,
    pub num_collisions: u32,
}

#[repr(C)]
pub struct HashEntry {
    pub hash: u32,
    pub key: *mut c_char,
    pub value: *mut c_void,
    pub next: *mut HashEntry,
}

#[repr(C)]
pub struct HashIterator {
    pub hash_table: *const HashTable,
    pub bucket_nr: u32,
    pub entry: *mut HashEntry,
}

#[derive(Clone, Copy)]
enum KeyOwnership {
    Copied,
    Borrowed,
}

#[derive(Clone, Copy)]
struct Callbacks {
    hash: HashFunction,
    equal: EqualFunction,
    copy: CopyFunction,
    free_key: FreeKeyFunction,
    free_value: FreeValueFunction,
}

unsafe fn allocate<T>() -> *mut T {
    let allocation = unsafe { libc::malloc(mem::size_of::<T>()) }.cast::<T>();
    if allocation.is_null() {
        std::process::abort();
    }
    allocation
}

unsafe fn allocate_buckets(size: u32) -> *mut *mut HashEntry {
    let count = match usize::try_from(size) {
        Ok(count) => count,
        Err(_) => std::process::abort(),
    };
    if count
        .checked_mul(mem::size_of::<*mut HashEntry>())
        .is_none()
    {
        std::process::abort();
    }
    let allocation =
        unsafe { libc::calloc(count, mem::size_of::<*mut HashEntry>()) }.cast::<*mut HashEntry>();
    if allocation.is_null() {
        std::process::abort();
    }
    allocation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstructorError {
    InvalidFillQuotient,
    BucketCountOverflow,
    AllocationSizeOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TableSizing {
    size: u32,
    min_size: u32,
    max_size: u32,
}

fn validate_sizing(
    initial_size: u32,
    min_fill_quotient: f64,
    max_fill_quotient: f64,
) -> Result<TableSizing, ConstructorError> {
    if !min_fill_quotient.is_finite()
        || !max_fill_quotient.is_finite()
        || max_fill_quotient <= 0.0
        || max_fill_quotient < min_fill_quotient
    {
        return Err(ConstructorError::InvalidFillQuotient);
    }
    let clamped = initial_size.max(4);
    let desired = ((f64::from(clamped)) / max_fill_quotient).ceil();
    if desired > f64::from(u32::MAX) {
        return Err(ConstructorError::BucketCountOverflow);
    }
    let desired = desired as u32;
    let size = desired
        .checked_next_power_of_two()
        .filter(|size| *size != 0)
        .ok_or(ConstructorError::BucketCountOverflow)?;
    let count = usize::try_from(size).map_err(|_| ConstructorError::AllocationSizeOverflow)?;
    count
        .checked_mul(mem::size_of::<*mut HashEntry>())
        .ok_or(ConstructorError::AllocationSizeOverflow)?;
    Ok(TableSizing {
        size,
        min_size: ((f64::from(size >> 1)) * min_fill_quotient).ceil() as u32,
        max_size: (f64::from(size) * max_fill_quotient).floor() as u32,
    })
}

unsafe fn setup(table: *mut HashTable, initial_size: u32) {
    let sizing = match validate_sizing(
        initial_size,
        unsafe { (*table).min_fill_quotient },
        unsafe { (*table).max_fill_quotient },
    ) {
        Ok(sizing) => sizing,
        Err(_) => std::process::abort(),
    };
    unsafe {
        (*table).size = sizing.size;
        (*table).hash_mask = sizing.size - 1;
        (*table).min_size = sizing.min_size;
        (*table).max_size = sizing.max_size;
        (*table).entries = allocate_buckets(sizing.size);
        (*table).num_entries = 0;
        (*table).num_collisions = 0;
    }
}

unsafe fn new_table(
    callbacks: Callbacks,
    initial_size: u32,
    min_fill_quotient: f64,
    max_fill_quotient: f64,
) -> *mut HashTable {
    if validate_sizing(initial_size, min_fill_quotient, max_fill_quotient).is_err() {
        std::process::abort();
    }
    let table = unsafe { allocate::<HashTable>() };
    unsafe {
        ptr::write(
            table,
            HashTable {
                hash_function: callbacks.hash,
                equal_function: callbacks.equal,
                copy_function: callbacks.copy,
                free_key_function: callbacks.free_key,
                free_value_function: callbacks.free_value,
                min_fill_quotient,
                max_fill_quotient,
                min_size: 0,
                max_size: 0,
                size: 0,
                hash_mask: 0,
                entries: ptr::null_mut(),
                num_entries: 0,
                num_collisions: 0,
            },
        );
        setup(table, initial_size);
    }
    table
}

unsafe fn hash_key(mut key: *const c_char) -> u32 {
    let mut hash = 0_u32;
    while unsafe { *key } != 0 {
        let byte = unsafe { *key } as i32 as u32;
        hash = (hash << 4) ^ (hash >> 28) ^ byte;
        key = unsafe { key.add(1) };
    }
    hash ^ (hash >> 10) ^ (hash >> 20)
}

unsafe fn keys_equal(left: *const c_char, right: *const c_char) -> bool {
    unsafe { libc::strcmp(left, right) == 0 }
}

unsafe fn copy_key(key: *const c_char, ownership: KeyOwnership) -> *mut c_char {
    match ownership {
        KeyOwnership::Borrowed => key.cast_mut(),
        KeyOwnership::Copied => {
            let length = unsafe { libc::strlen(key) } + 1;
            let copy = unsafe { libc::malloc(length) }.cast::<c_char>();
            if copy.is_null() {
                std::process::abort();
            }
            unsafe { ptr::copy_nonoverlapping(key, copy, length) };
            copy
        }
    }
}

unsafe fn free_key(key: *mut c_char, ownership: KeyOwnership) {
    if matches!(ownership, KeyOwnership::Copied) {
        unsafe { libc::free(key.cast()) };
    }
}

unsafe fn bucket(table: *mut HashTable, hash: u32) -> *mut *mut HashEntry {
    let index = hash & unsafe { (*table).hash_mask };
    unsafe { (*table).entries.add(index as usize) }
}

unsafe fn resize(table: *mut HashTable) {
    let old_entries = unsafe { (*table).entries };
    let old_size = unsafe { (*table).size };
    let mut remaining = unsafe { (*table).num_entries };
    let old_count = remaining;
    unsafe { setup(table, old_count) };
    unsafe { (*table).num_entries = old_count };

    let mut index = 0_usize;
    while index < old_size as usize && remaining > 0 {
        let mut entry = unsafe { *old_entries.add(index) };
        while !entry.is_null() {
            let next = unsafe { (*entry).next };
            let destination = unsafe { bucket(table, (*entry).hash) };
            if unsafe { !(*destination).is_null() } {
                unsafe { (*table).num_collisions += 1 };
            }
            unsafe {
                (*entry).next = *destination;
                *destination = entry;
            }
            remaining -= 1;
            entry = next;
        }
        index += 1;
    }
    if remaining != 0 {
        std::process::abort();
    }
    unsafe { libc::free(old_entries.cast()) };
}

unsafe fn add(
    table: *mut HashTable,
    key: *const c_char,
    value: *mut c_void,
    ownership: KeyOwnership,
) -> bool {
    let hash = unsafe { hash_key(key) };
    let location = unsafe { bucket(table, hash) };
    let mut entry = unsafe { *location };
    while !entry.is_null() {
        if unsafe { keys_equal(key, (*entry).key) } {
            return false;
        }
        entry = unsafe { (*entry).next };
    }

    if unsafe { !(*location).is_null() } {
        unsafe { (*table).num_collisions += 1 };
    }
    let new_entry = unsafe { allocate::<HashEntry>() };
    unsafe {
        ptr::write(
            new_entry,
            HashEntry {
                hash,
                key: copy_key(key, ownership),
                value,
                next: *location,
            },
        );
        *location = new_entry;
        (*table).num_entries += 1;
        if (*table).num_entries > (*table).max_size {
            resize(table);
        }
    }
    true
}

unsafe fn remove(table: *mut HashTable, key: *const c_char, ownership: KeyOwnership) -> bool {
    let hash = unsafe { hash_key(key) };
    let mut location = unsafe { bucket(table, hash) };
    loop {
        let entry = unsafe { *location };
        if entry.is_null() {
            return false;
        }
        if unsafe { keys_equal(key, (*entry).key) } {
            unsafe {
                *location = (*entry).next;
                free_key((*entry).key, ownership);
                libc::free(entry.cast());
                (*table).num_entries -= 1;
                if (*table).num_entries < (*table).min_size {
                    resize(table);
                }
            }
            return true;
        }
        location = unsafe { &mut (*entry).next };
    }
}

unsafe fn find(table: *mut HashTable, key: *const c_char) -> *mut c_void {
    let hash = unsafe { hash_key(key) };
    let mut entry = unsafe { *bucket(table, hash) };
    while !entry.is_null() {
        if unsafe { keys_equal(key, (*entry).key) } {
            return unsafe { (*entry).value };
        }
        entry = unsafe { (*entry).next };
    }
    ptr::null_mut()
}

unsafe fn delete_table(table: *mut HashTable, ownership: KeyOwnership) {
    let mut remaining = unsafe { (*table).num_entries };
    let table_size = unsafe { (*table).size };
    let mut bucket_ptr = unsafe { (*table).entries };
    let mut bucket_nr = 0_u32;
    while bucket_nr < table_size && remaining > 0 {
        let mut entry = unsafe { *bucket_ptr };
        while !entry.is_null() {
            let next = unsafe { (*entry).next };
            unsafe {
                free_key((*entry).key, ownership);
                libc::free(entry.cast());
            }
            remaining -= 1;
            entry = next;
        }
        bucket_ptr = unsafe { bucket_ptr.add(1) };
        bucket_nr += 1;
    }
    if remaining != 0 {
        std::process::abort();
    }
    unsafe {
        libc::free((*table).entries.cast());
        libc::free(table.cast());
    }
}

unsafe fn get_iterator(table: *const HashTable) -> *mut HashIterator {
    let iterator = unsafe { allocate::<HashIterator>() };
    let mut bucket_nr = 0_u32;
    while bucket_nr < unsafe { (*table).size } {
        let entry = unsafe { *(*table).entries.add(bucket_nr as usize) };
        if !entry.is_null() {
            unsafe {
                ptr::write(
                    iterator,
                    HashIterator {
                        hash_table: table,
                        bucket_nr,
                        entry,
                    },
                )
            };
            return iterator;
        }
        bucket_nr += 1;
    }
    unsafe {
        ptr::write(
            iterator,
            HashIterator {
                hash_table: table,
                bucket_nr,
                entry: ptr::null_mut(),
            },
        )
    };
    iterator
}

unsafe fn iterator_next(iterator: *mut HashIterator) -> *mut HashIterator {
    unsafe { (*iterator).entry = (*(*iterator).entry).next };
    if unsafe { !(*iterator).entry.is_null() } {
        return iterator;
    }
    let mut bucket_nr = unsafe { (*iterator).bucket_nr + 1 };
    while bucket_nr < unsafe { (*(*iterator).hash_table).size } {
        let entry = unsafe { *(*(*iterator).hash_table).entries.add(bucket_nr as usize) };
        if !entry.is_null() {
            unsafe {
                (*iterator).bucket_nr = bucket_nr;
                (*iterator).entry = entry;
            }
            return iterator;
        }
        bucket_nr += 1;
    }
    unsafe {
        (*iterator).bucket_nr = bucket_nr;
        (*iterator).entry = ptr::null_mut();
    }
    iterator
}

macro_rules! export_hash_table {
    (
        $ownership:expr,
        $new_fn:ident => $new_symbol:literal,
        $add_fn:ident => $add_symbol:literal,
        $remove_fn:ident => $remove_symbol:literal,
        $find_fn:ident => $find_symbol:literal,
        $count_fn:ident => $count_symbol:literal,
        $delete_fn:ident => $delete_symbol:literal,
        $get_iterator_fn:ident => $get_iterator_symbol:literal,
        $iterator_done_fn:ident => $iterator_done_symbol:literal,
        $iterator_key_fn:ident => $iterator_key_symbol:literal,
        $iterator_value_fn:ident => $iterator_value_symbol:literal,
        $iterator_next_fn:ident => $iterator_next_symbol:literal,
        $free_iterator_fn:ident => $free_iterator_symbol:literal
    ) => {
        /// Creates a hash table with the historical C layout and resize policy.
        ///
        /// # Safety
        /// **All callbacks, including `free_value`, are ignored.** They are retained solely for
        /// generic-header ABI layout parity and are never invoked by these historical macro
        /// specializations. Fill quotients must be finite,
        /// ordered, and in the supported `0.0 <= min <= max <= 1.0` range; the requested bucket
        /// count must fit the target allocation size. Violating constructor preconditions aborts.
        #[export_name = $new_symbol]
        pub unsafe extern "C" fn $new_fn(
            hash: HashFunction,
            equal: EqualFunction,
            copy: CopyFunction,
            free_key_callback: FreeKeyFunction,
            free_value: FreeValueFunction,
            initial_size: u32,
            min_fill_quotient: f64,
            max_fill_quotient: f64,
        ) -> *mut HashTable {
            unsafe {
                new_table(
                    Callbacks {
                        hash,
                        equal,
                        copy,
                        free_key: free_key_callback,
                        free_value,
                    },
                    initial_size,
                    min_fill_quotient,
                    max_fill_quotient,
                )
            }
        }

        /// Adds a key and caller-owned value if the key is absent.
        ///
        /// # Safety
        /// `table` must be live and `key` must point to a readable NUL-terminated string.
        #[export_name = $add_symbol]
        pub unsafe extern "C" fn $add_fn(
            table: *mut HashTable,
            key: *const c_char,
            value: *mut c_void,
        ) -> bool {
            unsafe { add(table, key, value, $ownership) }
        }

        /// Removes a key without freeing its caller-owned value.
        ///
        /// # Safety
        /// `table` must be live and `key` must point to a readable NUL-terminated string.
        #[export_name = $remove_symbol]
        pub unsafe extern "C" fn $remove_fn(table: *mut HashTable, key: *const c_char) -> bool {
            unsafe { remove(table, key, $ownership) }
        }

        /// Finds the caller-owned value associated with a key.
        ///
        /// # Safety
        /// `table` must be live and `key` must point to a readable NUL-terminated string.
        #[export_name = $find_symbol]
        pub unsafe extern "C" fn $find_fn(
            table: *mut HashTable,
            key: *const c_char,
        ) -> *mut c_void {
            unsafe { find(table, key) }
        }

        /// Returns the current number of entries.
        ///
        /// # Safety
        /// `table` must point to a live table.
        #[export_name = $count_symbol]
        pub unsafe extern "C" fn $count_fn(table: *const HashTable) -> u32 {
            unsafe { (*table).num_entries }
        }

        /// Deletes the table while leaving caller-owned values untouched.
        ///
        /// # Safety
        /// `table` must be live and must not be used again.
        #[export_name = $delete_symbol]
        pub unsafe extern "C" fn $delete_fn(table: *mut HashTable) {
            unsafe { delete_table(table, $ownership) }
        }

        /// Allocates an iterator at the first entry.
        ///
        /// # Safety
        /// `table` must remain live and unmodified while the iterator is used.
        #[export_name = $get_iterator_symbol]
        pub unsafe extern "C" fn $get_iterator_fn(table: *const HashTable) -> *mut HashIterator {
            unsafe { get_iterator(table) }
        }

        /// Reports whether an iterator is past the final entry.
        ///
        /// # Safety
        /// `iterator` and its table must remain live.
        #[export_name = $iterator_done_symbol]
        pub unsafe extern "C" fn $iterator_done_fn(iterator: *const HashIterator) -> c_int {
            unsafe { ((*iterator).bucket_nr >= (*(*iterator).hash_table).size).into() }
        }

        /// Returns the current key pointer.
        ///
        /// # Safety
        /// `iterator` must be live and not past the final entry.
        #[export_name = $iterator_key_symbol]
        pub unsafe extern "C" fn $iterator_key_fn(iterator: *mut HashIterator) -> *mut c_char {
            unsafe { (*(*iterator).entry).key }
        }

        /// Returns the current caller-owned value pointer.
        ///
        /// # Safety
        /// `iterator` must be live and not past the final entry.
        #[export_name = $iterator_value_symbol]
        pub unsafe extern "C" fn $iterator_value_fn(iterator: *mut HashIterator) -> *mut c_void {
            unsafe { (*(*iterator).entry).value }
        }

        /// Advances an iterator to the next entry.
        ///
        /// # Safety
        /// `iterator` must be live and not already past the final entry.
        #[export_name = $iterator_next_symbol]
        pub unsafe extern "C" fn $iterator_next_fn(
            iterator: *mut HashIterator,
        ) -> *mut HashIterator {
            unsafe { iterator_next(iterator) }
        }

        /// Frees an iterator allocation.
        ///
        /// # Safety
        /// `iterator` must be a live iterator and must not be used again.
        #[export_name = $free_iterator_symbol]
        pub unsafe extern "C" fn $free_iterator_fn(iterator: *mut HashIterator) {
            unsafe { libc::free(iterator.cast()) };
        }
    };
}

export_hash_table!(
    KeyOwnership::Copied,
    char_new => "CharHashTable_newHashTable",
    char_add => "CharHashTable_add",
    char_remove => "CharHashTable_remove",
    char_find => "CharHashTable_find",
    char_count => "CharHashTable_count",
    char_delete => "CharHashTable_deleteHashTable",
    char_get_iterator => "CharHashTable_getIterator",
    char_iterator_done => "CharHashTable_iteratorDone",
    char_iterator_key => "CharHashTable_iteratorKey",
    char_iterator_value => "CharHashTable_iteratorValue",
    char_iterator_next => "CharHashTable_iteratorNext",
    char_free_iterator => "CharHashTable_freeIterator"
);

export_hash_table!(
    KeyOwnership::Borrowed,
    string_new => "StringHashTable_newHashTable",
    string_add => "StringHashTable_add",
    string_remove => "StringHashTable_remove",
    string_find => "StringHashTable_find",
    string_count => "StringHashTable_count",
    string_delete => "StringHashTable_deleteHashTable",
    string_get_iterator => "StringHashTable_getIterator",
    string_iterator_done => "StringHashTable_iteratorDone",
    string_iterator_key => "StringHashTable_iteratorKey",
    string_iterator_value => "StringHashTable_iteratorValue",
    string_iterator_next => "StringHashTable_iteratorNext",
    string_free_iterator => "StringHashTable_freeIterator"
);

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeMap;
    use std::ffi::{CStr, CString};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn unexpected_hash(_key: *const c_char) -> u32 {
        CALLBACK_CALLS.fetch_add(1, Ordering::Relaxed);
        0
    }

    fn callbacks() -> Callbacks {
        Callbacks {
            hash: Some(unexpected_hash),
            equal: None,
            copy: None,
            free_key: None,
            free_value: None,
        }
    }

    #[test]
    fn historical_hash_preserves_target_c_char_promotion() {
        let key = [0x80_u8 as c_char, 0xff_u8 as c_char, 0];
        let expected = if c_char::MIN < 0 {
            0x0000_07f1
        } else {
            0x0000_08fd
        };
        assert_eq!(unsafe { hash_key(key.as_ptr()) }, expected);
    }

    #[test]
    fn constructor_validation_preserves_historical_c_behavior() {
        let (min, max) = (0.9, 0.8);
        assert_eq!(
            validate_sizing(4, min, max),
            Err(ConstructorError::InvalidFillQuotient)
        );
        for (min, max) in [
            (f64::NAN, 0.9),
            (0.85, f64::NAN),
            (f64::NEG_INFINITY, 0.9),
            (0.85, f64::INFINITY),
            (0.0, 0.0),
            (-1.0, -0.5),
        ] {
            assert_eq!(
                validate_sizing(4, min, max),
                Err(ConstructorError::InvalidFillQuotient)
            );
        }
        assert_eq!(validate_sizing(0, 0.0, 1.0).unwrap().size, 4);
        assert_eq!(
            validate_sizing(u32::MAX, 0.0, 1.0),
            Err(ConstructorError::BucketCountOverflow)
        );
        assert_eq!(
            validate_sizing(1_u32 << 31, 0.0, 0.5),
            Err(ConstructorError::BucketCountOverflow)
        );
    }

    #[test]
    fn exported_specializations_preserve_key_ownership_and_ignore_generic_callbacks() {
        CALLBACK_CALLS.store(0, Ordering::Relaxed);
        let key = CString::new("alpha").unwrap();
        let value = 17_usize as *mut c_void;
        unsafe {
            let copied = char_new(Some(unexpected_hash), None, None, None, None, 0, 0.85, 0.9);
            assert!(char_add(copied, key.as_ptr(), value));
            let iterator = char_get_iterator(copied);
            assert_ne!(char_iterator_key(iterator), key.as_ptr().cast_mut());
            assert_eq!(char_iterator_value(iterator), value);
            assert_eq!(char_find(copied, key.as_ptr()), value);
            char_free_iterator(iterator);
            char_delete(copied);

            let borrowed = string_new(Some(unexpected_hash), None, None, None, None, 0, 0.85, 0.9);
            assert!(string_add(borrowed, key.as_ptr(), value));
            let iterator = string_get_iterator(borrowed);
            assert_eq!(string_iterator_key(iterator), key.as_ptr().cast_mut());
            assert_eq!(string_iterator_value(iterator), value);
            string_free_iterator(iterator);
            string_delete(borrowed);
        }
        assert_eq!(CALLBACK_CALLS.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn iterator_visits_every_entry_after_growth_and_shrink() {
        let keys: Vec<_> = (0..80)
            .map(|index| CString::new(format!("key-{index}")).unwrap())
            .collect();
        unsafe {
            let table = new_table(callbacks(), 0, 0.85, 0.9);
            for (index, key) in keys.iter().enumerate() {
                assert!(add(
                    table,
                    key.as_ptr(),
                    (index + 1) as *mut c_void,
                    KeyOwnership::Borrowed,
                ));
            }
            for key in keys.iter().take(40) {
                assert!(remove(table, key.as_ptr(), KeyOwnership::Borrowed));
            }
            let iterator = get_iterator(table);
            let mut observed = BTreeMap::new();
            loop {
                if (*iterator).bucket_nr >= (*table).size {
                    break;
                }
                observed.insert(
                    CStr::from_ptr((*(*iterator).entry).key)
                        .to_string_lossy()
                        .into_owned(),
                    (*(*iterator).entry).value as usize,
                );
                iterator_next(iterator);
            }
            assert_eq!(observed.len(), 40);
            for index in 40..80 {
                assert_eq!(observed[&format!("key-{index}")], index + 1);
            }
            libc::free(iterator.cast());
            delete_table(table, KeyOwnership::Borrowed);
        }
    }

    proptest! {
        #[test]
        fn operations_match_a_map(
            operations in prop::collection::vec(("[a-z]{1,12}", any::<u16>(), any::<bool>()), 1..300)
        ) {
            let mut expected = BTreeMap::new();
            let keys: Vec<_> = operations
                .iter()
                .map(|(key, _, _)| CString::new(key.as_str()).unwrap())
                .collect();
            unsafe {
                let table = new_table(callbacks(), 0, 0.85, 0.9);
                for ((key, value, insert), c_key) in operations.iter().zip(&keys) {
                    if *insert {
                        let inserted = !expected.contains_key(key);
                        if inserted {
                            expected.insert(key.clone(), usize::from(*value) + 1);
                        }
                        prop_assert_eq!(
                            add(
                                table,
                                c_key.as_ptr(),
                                (usize::from(*value) + 1) as *mut c_void,
                                KeyOwnership::Copied,
                            ),
                            inserted,
                        );
                    } else {
                        prop_assert_eq!(
                            remove(table, c_key.as_ptr(), KeyOwnership::Copied),
                            expected.remove(key).is_some(),
                        );
                    }
                    prop_assert_eq!((*table).num_entries as usize, expected.len());
                    for (present, value) in &expected {
                        let query = CString::new(present.as_str()).unwrap();
                        prop_assert_eq!(find(table, query.as_ptr()) as usize, *value);
                    }
                }
                delete_table(table, KeyOwnership::Copied);
            }
        }
    }
}
