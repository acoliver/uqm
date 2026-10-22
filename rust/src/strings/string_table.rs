//! C ABI for string tables (`libs/strlib.h`, `libs/strings/strintrn.h`).
//!
//! A table is one array of entries: `size` strings, followed by `size` names
//! (`HAS_NAMEINDEX`), `size` sound clip names (`HAS_SOUND_CLIPS`) and `size`
//! timestamps (`HAS_TIMESTAMP`), in that order. The loaders in C still fill
//! these structs in directly, so the layouts must match `strintrn.h`.

use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

use crate::memory::{rust_hfree, rust_hmalloc};
use crate::strings::unicode::utf8StringCountN;

pub const HAS_SOUND_CLIPS: u16 = 1 << 0;
pub const HAS_TIMESTAMP: u16 = 1 << 1;
pub const HAS_NAMEINDEX: u16 = 1 << 2;

#[repr(C)]
pub struct StringTableEntry {
    /// Not NUL-terminated beyond `length`; internal NULs are allowed.
    pub data: *mut c_char,
    pub length: c_int,
    pub index: c_int,
    pub parent: *mut StringTable,
}

#[repr(C)]
pub struct StringTable {
    pub flags: u16,
    pub size: c_int,
    pub strings: *mut StringTableEntry,
    /// `StringHashTable` from name to entry; owned by the loader.
    pub name_index: *mut c_void,
}

extern "C" {
    fn StringHashTable_find(table: *mut c_void, key: *const c_char) -> *mut c_void;
}

/// Number of entry arrays of `size` entries a table with `flags` holds.
fn array_count(flags: u16) -> usize {
    1 + usize::from(flags & HAS_NAMEINDEX != 0)
        + usize::from(flags & HAS_SOUND_CLIPS != 0)
        + usize::from(flags & HAS_TIMESTAMP != 0)
}

/// # Safety
/// Allocates with the game allocator; release with [`FreeStringTable`].
#[no_mangle]
pub unsafe extern "C" fn AllocStringTable(num_entries: c_int, flags: c_int) -> *mut StringTable {
    let table = rust_hmalloc(std::mem::size_of::<StringTable>()).cast::<StringTable>();
    let flags = flags as u16;
    let total = num_entries as usize * array_count(flags);
    let strings =
        rust_hmalloc(std::mem::size_of::<StringTableEntry>() * total).cast::<StringTableEntry>();
    for i in 0..total {
        strings.add(i).write(StringTableEntry {
            data: null_mut(),
            length: 0,
            index: i as c_int,
            parent: table,
        });
    }
    table.write(StringTable {
        flags,
        size: num_entries,
        strings,
        name_index: null_mut(),
    });
    table
}

/// Frees the table and the data of the first `size * (1 + clips + timestamps)`
/// entries, which is what the loaders allocate separately. With a name index
/// the name array sits inside that range and the last array is not walked.
///
/// # Safety
/// `table` must be NULL or come from [`AllocStringTable`].
#[no_mangle]
pub unsafe extern "C" fn FreeStringTable(table: *mut StringTable) {
    if table.is_null() {
        return;
    }
    let t = &*table;
    let owning_arrays =
        1 + usize::from(t.flags & HAS_SOUND_CLIPS != 0) + usize::from(t.flags & HAS_TIMESTAMP != 0);
    for i in 0..t.size as usize * owning_arrays {
        let data = (*t.strings.add(i)).data;
        if !data.is_null() {
            rust_hfree(data.cast());
        }
    }
    rust_hfree(t.strings.cast());
    rust_hfree(table.cast());
}

/// Returns `TRUE` (1).
///
/// # Safety
/// As for [`FreeStringTable`].
#[no_mangle]
pub unsafe extern "C" fn DestroyStringTable(table: *mut StringTable) -> c_int {
    FreeStringTable(table);
    1
}

/// First string of the table, or NULL for a missing or empty table.
///
/// # Safety
/// `table` must be NULL or a live table.
#[no_mangle]
pub unsafe extern "C" fn CaptureStringTable(table: *mut StringTable) -> *mut StringTableEntry {
    if !table.is_null() && (*table).size > 0 {
        (*table).strings
    } else {
        null_mut()
    }
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn ReleaseStringTable(string: *mut StringTableEntry) -> *mut StringTable {
    GetStringTable(string)
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringTable(string: *mut StringTableEntry) -> *mut StringTable {
    if string.is_null() {
        null_mut()
    } else {
        (*string).parent
    }
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringTableCount(string: *mut StringTableEntry) -> u16 {
    let table = GetStringTable(string);
    if table.is_null() {
        0
    } else {
        (*table).size as u16
    }
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringTableIndex(string: *mut StringTableEntry) -> u16 {
    if string.is_null() {
        0
    } else {
        (*string).index as u16
    }
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn SetAbsStringTableIndex(
    string: *mut StringTableEntry,
    index: u16,
) -> *mut StringTableEntry {
    let table = GetStringTable(string);
    if table.is_null() {
        return null_mut();
    }
    let t = &*table;
    t.strings.add((c_int::from(index) % t.size) as usize)
}

/// Moves by `offset` entries, wrapping around the table.
///
/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn SetRelStringTableIndex(
    string: *mut StringTableEntry,
    mut offset: i16,
) -> *mut StringTableEntry {
    let table = GetStringTable(string);
    if table.is_null() {
        return null_mut();
    }
    let t = &*table;
    while offset < 0 {
        // Stays in SIZE (int16) arithmetic, as the callers' offsets are.
        offset = (c_int::from(offset) + t.size) as i16;
    }
    let index = ((*string).index + c_int::from(offset)) % t.size;
    t.strings.add(index as u16 as usize)
}

/// Length in characters.
///
/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringLength(string: *mut StringTableEntry) -> u16 {
    if string.is_null() {
        return 0;
    }
    let data = (*string).data.cast::<u8>().cast_const();
    utf8StringCountN(data, data.wrapping_offset((*string).length as isize)) as u16
}

/// Length in bytes.
///
/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringLengthBin(string: *mut StringTableEntry) -> u16 {
    if string.is_null() {
        0
    } else {
        (*string).length as u16
    }
}

/// Data of the companion entry for `string` in the array selected by
/// `required_flag`, which follows the string array and any arrays of
/// `preceding_flags` the table has. NULL when the table lacks it or the entry
/// is empty.
unsafe fn companion_data(
    string: *mut StringTableEntry,
    required_flag: u16,
    preceding_flags: &[u16],
) -> *mut c_char {
    let table = GetStringTable(string);
    if table.is_null() {
        return null_mut();
    }
    let t = &*table;
    if t.flags & required_flag == 0 {
        return null_mut();
    }
    let preceding = preceding_flags
        .iter()
        .filter(|f| t.flags & **f != 0)
        .count();
    let entry = &*t
        .strings
        .add((*string).index as usize + t.size as usize * (1 + preceding));
    if entry.length == 0 {
        null_mut()
    } else {
        entry.data
    }
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringName(string: *mut StringTableEntry) -> *mut c_char {
    companion_data(string, HAS_NAMEINDEX, &[])
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringSoundClip(string: *mut StringTableEntry) -> *mut c_char {
    companion_data(string, HAS_SOUND_CLIPS, &[HAS_NAMEINDEX])
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringTimeStamp(string: *mut StringTableEntry) -> *mut c_char {
    companion_data(string, HAS_TIMESTAMP, &[HAS_NAMEINDEX, HAS_SOUND_CLIPS])
}

/// # Safety
/// `string` must be NULL or a live entry.
#[no_mangle]
pub unsafe extern "C" fn GetStringAddress(string: *mut StringTableEntry) -> *mut c_char {
    if string.is_null() {
        null_mut()
    } else {
        (*string).data
    }
}

/// # Safety
/// `table` must be live with a name index; `index` NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn GetStringByName(
    table: *mut StringTable,
    index: *const c_char,
) -> *mut StringTableEntry {
    StringHashTable_find((*table).name_index, index).cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn set(table: *mut StringTable, slot: usize, text: &[u8]) {
        let entry = &mut *(*table).strings.add(slot);
        let buf = rust_hmalloc(text.len()).cast::<u8>();
        std::ptr::copy_nonoverlapping(text.as_ptr(), buf, text.len());
        entry.data = buf.cast();
        entry.length = text.len() as c_int;
    }

    #[test]
    fn alloc_links_entries_to_parent_and_index() {
        unsafe {
            let flags = c_int::from(HAS_NAMEINDEX | HAS_SOUND_CLIPS | HAS_TIMESTAMP);
            let table = AllocStringTable(3, flags);
            assert_eq!((*table).size, 3);
            for i in 0..12 {
                let e = &*(*table).strings.add(i);
                assert_eq!((e.index, e.parent, e.length), (i as c_int, table, 0));
                assert!(e.data.is_null());
            }
            assert_eq!(DestroyStringTable(table), 1);
        }
    }

    #[test]
    fn navigation_wraps_in_both_directions() {
        unsafe {
            let table = AllocStringTable(4, 0);
            let first = CaptureStringTable(table);
            assert_eq!(GetStringTableCount(first), 4);
            assert_eq!(GetStringTableIndex(SetAbsStringTableIndex(first, 6)), 2);
            assert_eq!(GetStringTableIndex(SetRelStringTableIndex(first, -1)), 3);
            assert_eq!(GetStringTableIndex(SetRelStringTableIndex(first, -9)), 3);
            let third = SetAbsStringTableIndex(first, 2);
            assert_eq!(GetStringTableIndex(SetRelStringTableIndex(third, 3)), 1);
            assert_eq!(ReleaseStringTable(third), table);
            assert!(SetAbsStringTableIndex(null_mut(), 1).is_null());
            assert!(CaptureStringTable(null_mut()).is_null());
            FreeStringTable(table);
        }
    }

    #[test]
    fn companion_entries_follow_flag_order() {
        unsafe {
            let flags = c_int::from(HAS_NAMEINDEX | HAS_SOUND_CLIPS | HAS_TIMESTAMP);
            let table = AllocStringTable(2, flags);
            set(table, 1, "h\u{e9}llo".as_bytes());
            set(table, 3, b"NAME");
            set(table, 5, b"clip.ogg");
            set(table, 7, b"1,2");
            let s = SetAbsStringTableIndex(CaptureStringTable(table), 1);
            assert_eq!(GetStringLengthBin(s), 6);
            assert_eq!(GetStringLength(s), 5);
            assert_eq!(*GetStringAddress(s).cast::<u8>(), b'h');
            assert_eq!(*GetStringName(s).cast::<u8>(), b'N');
            assert_eq!(*GetStringSoundClip(s).cast::<u8>(), b'c');
            assert_eq!(*GetStringTimeStamp(s).cast::<u8>(), b'1');
            let empty = CaptureStringTable(table);
            assert!(GetStringName(empty).is_null());
            assert!(GetStringSoundClip(empty).is_null());
            assert!(GetStringTimeStamp(empty).is_null());
            FreeStringTable(table);
        }
    }

    #[test]
    fn missing_companions_are_null_without_flags() {
        unsafe {
            let table = AllocStringTable(1, c_int::from(HAS_TIMESTAMP));
            set(table, 1, b"ts");
            let s = CaptureStringTable(table);
            assert!(GetStringName(s).is_null());
            assert!(GetStringSoundClip(s).is_null());
            assert_eq!(*GetStringTimeStamp(s).cast::<u8>(), b't');
            assert!(GetStringTimeStamp(null_mut()).is_null());
            assert_eq!(GetStringLength(null_mut()), 0);
            FreeStringTable(table);
        }
    }
}
