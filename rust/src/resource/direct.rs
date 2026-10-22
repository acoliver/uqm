//! Directory-entry string tables, preserving UIO enumeration order.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use crate::io::uio_bridge::{
    uio_DirHandle, uio_DirList_free, uio_closeDir, uio_getDirList, uio_openDirRelative, uio_stat,
};
use crate::memory::rust_hmalloc;
use crate::strings::string_table::{AllocStringTable, StringTable};

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
    let dir = uio_openDirRelative(base, path, 0);
    assert!(!dir.is_null());
    let list = uio_getDirList(dir, c"".as_ptr(), pattern, match_type);
    assert!(!list.is_null());
    if (*list).numNames == 0 {
        uio_closeDir(dir);
        uio_DirList_free(list);
        return ptr::null_mut();
    }
    let names = std::slice::from_raw_parts_mut((*list).names, (*list).numNames as usize);
    let mut count = 0;
    for name in names.iter_mut() {
        let mut stat = std::mem::MaybeUninit::uninit();
        if **name == b'.' as c_char
            || uio_stat(dir, *name, stat.as_mut_ptr()) == -1
            || stat.assume_init().st_mode & libc::S_IFMT != libc::S_IFREG
        {
            *name = ptr::null_mut();
        } else {
            count += 1;
        }
    }
    uio_closeDir(dir);
    if count == 0 {
        uio_DirList_free(list);
        return ptr::null_mut();
    }
    let table = AllocStringTable(count, 0);
    let mut entry = (*table).strings;
    for &name in names.iter().filter(|name| !name.is_null()) {
        let bytes = CStr::from_ptr(name).to_bytes_with_nul();
        let target = rust_hmalloc(bytes.len()).cast::<c_char>();
        ptr::copy_nonoverlapping(name, target, bytes.len());
        (*entry).data = target;
        (*entry).length = bytes.len() as c_int;
        entry = entry.add(1);
    }
    uio_DirList_free(list);
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::uio_bridge::{
        uio_closeRepository, uio_mountDir, uio_openDir, uio_openRepository,
    };
    use crate::strings::string_table::FreeStringTable;
    use std::ffi::CString;

    #[test]
    #[serial_test::serial]
    fn directory_table_excludes_hidden_names_and_nonfiles() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp")).unwrap();
        let temp = tempfile::Builder::new()
            .prefix("direct-")
            .tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp"))
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
