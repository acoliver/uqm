//! Directory-entry string tables, preserving virtual file system enumeration order.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use crate::io::{Dir, Match};
use crate::memory::rust_hmalloc;
use crate::strings::string_table::{AllocStringTable, StringTable};
use std::path::Path;

/// # Safety
/// `path` and `pattern` must be NUL-terminated. The path must name a directory.
pub unsafe fn LoadDirEntryTable(
    base: &Dir,
    path: *const c_char,
    pattern: *const c_char,
    kind: Match,
) -> *mut StringTable {
    let path = CStr::from_ptr(path).to_string_lossy();
    let pattern = CStr::from_ptr(pattern).to_string_lossy();
    let names = base.files_matching(Path::new(path.as_ref()), &pattern, kind);
    if names.is_empty() {
        return ptr::null_mut();
    }
    let table = AllocStringTable(names.len() as c_int, 0);
    for (index, name) in names.iter().enumerate() {
        let entry = &mut *(*table).strings.add(index);
        let target = rust_hmalloc(name.len() + 1).cast::<c_char>();
        ptr::copy_nonoverlapping(name.as_ptr().cast(), target, name.len());
        *target.add(name.len()) = 0;
        entry.data = target;
        entry.length = (name.len() + 1) as c_int;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::fixture::TempMount;
    use crate::strings::string_table::FreeStringTable;

    #[test]
    fn directory_table_is_empty_for_a_missing_directory() {
        let mount = TempMount::new();
        // SAFETY: both strings are terminated literals.
        let table = unsafe {
            LoadDirEntryTable(
                &mount.dir(),
                c"missing".as_ptr(),
                c"".as_ptr(),
                Match::Literal,
            )
        };
        assert!(table.is_null());
    }

    #[test]
    fn directory_table_excludes_hidden_names_and_nonfiles() {
        let mount = TempMount::new();
        for name in ["one.sav", "two.sav", ".hidden.sav", "other.txt"] {
            mount.write(name, b"data");
        }
        std::fs::create_dir(mount.path_of("dir.sav")).unwrap();
        let root = mount.dir();
        // SAFETY: the strings are terminated literals and the table is freed once.
        unsafe {
            let table = LoadDirEntryTable(&root, c"".as_ptr(), c".sav".as_ptr(), Match::Substring);
            assert!(!table.is_null());
            assert_eq!((*table).size, 2);
            let mut names = Vec::new();
            for i in 0..2 {
                let entry = &*(*table).strings.add(i);
                let name = CStr::from_ptr(entry.data);
                assert_eq!(entry.length as usize, name.to_bytes_with_nul().len());
                names.push(name.to_bytes().to_vec());
            }
            names.sort();
            assert_eq!(names, [b"one.sav".to_vec(), b"two.sav".to_vec()]);
            FreeStringTable(table);
            assert!(
                LoadDirEntryTable(&root, c"".as_ptr(), c"absent".as_ptr(), Match::Literal)
                    .is_null()
            );
        }
    }
}
