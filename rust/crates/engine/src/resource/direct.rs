//! Directory-entry string tables, preserving UIO enumeration order.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use crate::host::Files;
use crate::io::uio_bridge::uio_DirHandle;
use crate::io::uio_owned::directory;
use crate::memory::rust_hmalloc;
use crate::strings::string_table::{AllocStringTable, StringTable};
use std::path::Path;

/// # Safety
/// `base` must be a live UIO directory, and `path` and `pattern` must be
/// NUL-terminated. The path must name a directory, as required by the C API.
#[no_mangle]
pub unsafe extern "C" fn LoadDirEntryTable(
    base: *mut uio_DirHandle,
    path: *const c_char,
    pattern: *const c_char,
    match_type: c_int,
) -> *mut StringTable {
    let files = directory(base).expect("directory table base");
    let path = CStr::from_ptr(path).to_string_lossy();
    let path = Path::new(path.as_ref());
    let pattern = CStr::from_ptr(pattern).to_string_lossy();
    let names: Vec<_> = files
        .list(path)
        .expect("directory enumeration")
        .into_iter()
        .filter(|name| {
            !name.starts_with('.')
                && crate::io::uio_bridge::matches_pattern(name, &pattern, match_type)
                && files.stat(&path.join(name)).is_ok_and(|stat| stat.is_file)
        })
        .collect();
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
    use crate::io::uio_bridge::{
        uio_closeDir, uio_closeRepository, uio_mountDir, uio_openDir, uio_openRepository,
    };
    use crate::strings::string_table::FreeStringTable;
    use std::ffi::CString;

    #[test]
    #[serial_test::serial]
    fn directory_table_excludes_hidden_names_and_nonfiles() {
        std::fs::create_dir_all(concat!(env!("UQM_RUST_ROOT"), "/../tmp")).unwrap();
        let temp = tempfile::Builder::new()
            .prefix("direct-")
            .tempdir_in(concat!(env!("UQM_RUST_ROOT"), "/../tmp"))
            .unwrap();
        std::fs::write(temp.path().join("one.sav"), b"1").unwrap();
        std::fs::write(temp.path().join("two.sav"), b"2").unwrap();
        std::fs::write(temp.path().join(".hidden.sav"), b"3").unwrap();
        std::fs::write(temp.path().join("other.txt"), b"4").unwrap();
        std::fs::create_dir(temp.path().join("dir.sav")).unwrap();
        let host = CString::new(temp.path().to_str().unwrap()).unwrap();
        unsafe {
            let repo = uio_openRepository(0);
            assert!(!uio_mountDir(
                repo,
                c"/".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                4,
                ptr::null_mut()
            )
            .is_null());
            let root = uio_openDir(repo, c"/".as_ptr(), 0);
            let table = LoadDirEntryTable(root, c"".as_ptr(), c".sav".as_ptr(), 3);
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
            assert!(LoadDirEntryTable(root, c"".as_ptr(), c"absent".as_ptr(), 0).is_null());
            uio_closeDir(root);
            uio_closeRepository(repo);
        }
    }
}
