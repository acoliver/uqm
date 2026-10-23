use std::collections::BTreeMap;
use std::ffi::{c_void, CStr, CString};
use uqm_rust::collections::hash_table::*;

#[test]
fn copied_keys_survive_their_sources_and_iterate_after_growth_and_removal() {
    let table = char_new(0, 0.85, 0.9);
    unsafe {
        for index in 0..80usize {
            let key = CString::new(format!("entry-{index}")).unwrap();
            assert!(char_add(table, key.as_ptr(), (index + 1) as *mut c_void));
            assert!(!char_add(table, key.as_ptr(), 999usize as *mut c_void));
        }
        assert_eq!(char_count(table), 80);
        for index in 0..60usize {
            let key = CString::new(format!("entry-{index}")).unwrap();
            assert!(char_remove(table, key.as_ptr()));
            assert!(!char_remove(table, key.as_ptr()));
            assert!(char_find(table, key.as_ptr()).is_null());
        }
        let mut seen = BTreeMap::new();
        let iterator = char_get_iterator(table);
        while char_iterator_done(iterator) == 0 {
            let key = CStr::from_ptr(char_iterator_key(iterator))
                .to_str()
                .unwrap();
            assert!(seen
                .insert(key.to_owned(), char_iterator_value(iterator) as usize)
                .is_none());
            assert_eq!(char_iterator_next(iterator), iterator);
        }
        assert_eq!(seen.len(), 20);
        for index in 60..80usize {
            let key = format!("entry-{index}");
            let query = CString::new(key.as_str()).unwrap();
            assert_eq!(seen[&key], index + 1);
            assert_eq!(char_find(table, query.as_ptr()) as usize, index + 1);
        }
        assert_eq!(char_count(table), 20);
        char_free_iterator(iterator);
        char_delete(table);
    }
}

#[test]
fn borrowed_keys_and_values_keep_their_identity_and_empty_iteration_terminates() {
    let key = c"borrowed-key";
    let mut value = 42u32;
    let table = string_new(0, 0.85, 0.9);
    unsafe {
        let empty = string_get_iterator(table);
        assert_ne!(string_iterator_done(empty), 0);
        string_free_iterator(empty);
        let pointer = std::ptr::addr_of_mut!(value).cast();
        assert!(string_add(table, key.as_ptr(), pointer));
        assert!(!string_add(table, key.as_ptr(), std::ptr::null_mut()));
        let iterator = string_get_iterator(table);
        assert_eq!(string_iterator_key(iterator), key.as_ptr().cast_mut());
        assert_eq!(string_iterator_value(iterator), pointer);
        assert_eq!(string_find(table, key.as_ptr()), pointer);
        string_free_iterator(iterator);
        assert_eq!(string_count(table), 1);
        assert!(string_remove(table, key.as_ptr()));
        assert_eq!(string_count(table), 0);
        string_delete(table);
    }
    assert_eq!(key.to_bytes(), b"borrowed-key");
    assert_eq!(value, 42);
}
