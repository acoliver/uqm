use super::*;
use crate::strings::string_table::StringTableEntry;

#[test]
fn copied_binary_entries_keep_full_lengths_empty_entries_and_index_wrapping() {
    let mut large = vec![0x7b; 65537];
    let mut tail = vec![1, 2, 0, 4];
    let mut entries = [
        StringTableEntry {
            data: large.as_mut_ptr().cast(),
            length: large.len() as i32,
            index: 0,
            parent: std::ptr::null_mut(),
        },
        StringTableEntry {
            data: std::ptr::null_mut(),
            length: 0,
            index: 1,
            parent: std::ptr::null_mut(),
        },
        StringTableEntry {
            data: tail.as_mut_ptr().cast(),
            length: tail.len() as i32,
            index: 2,
            parent: std::ptr::null_mut(),
        },
    ];
    let mut table = StringTable {
        flags: 0,
        size: entries.len() as i32,
        strings: entries.as_mut_ptr(),
        name_index: std::ptr::null_mut(),
        owned_text: false,
    };
    // SAFETY: these fixture entries borrow their full recorded allocations until after copying.
    let (all, wrapped) = unsafe {
        (
            copy_binary_entries(&table, None),
            copy_binary_entries(&table, Some(5)),
        )
    };
    assert_eq!(all, [large.clone(), Vec::new(), tail.clone()]);
    assert_eq!(wrapped, [tail.clone()]);
    large.fill(0);
    tail.fill(0);
    assert_eq!(all[0].len(), 65537);
    assert_eq!(all[0][65536], 0x7b);
    assert_eq!(all[2], [1, 2, 0, 4]);
    table.size = 0;
    table.strings = std::ptr::null_mut();
    // SAFETY: an empty table has no entry storage to dereference, including a selected index.
    unsafe {
        assert!(copy_binary_entries(&table, None).is_empty());
        assert!(copy_binary_entries(&table, Some(0)).is_empty());
    }
}
