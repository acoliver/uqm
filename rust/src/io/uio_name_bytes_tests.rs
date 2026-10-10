//! File names are bytes. These tests keep names that are not valid UTF-8
//! intact through listing, `stat`, `open` and directory-list freeing, and keep
//! a byte path from being read as a different, valid-UTF-8 name.
//!
//! macOS file systems refuse a name that is not valid UTF-8 (EILSEQ), so the
//! tests that need such a file on disk check that refusal there and rely on
//! the allocation-level tests, which need no file, for the same byte handling.

use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;

use serial_test::serial;

use super::layered_open_tests::Stack;
use super::*;
use crate::io::name_filter::{MATCH_PREFIX, MATCH_REGEX};

const REPLACEMENT_CHARACTER: &str = "\u{fffd}";

fn raw(bytes: &[u8]) -> CString {
    CString::new(bytes).unwrap()
}

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// Creates a file named by `name` bytes. Returns false only where the host
/// file system cannot hold such a name, which it reports as EILSEQ.
fn create_host_file(dir: &Path, name: &[u8], content: &[u8]) -> bool {
    match std::fs::write(dir.join(OsStr::from_bytes(name)), content) {
        Ok(()) => true,
        Err(error) => {
            assert_eq!(error.raw_os_error(), Some(libc::EILSEQ), "{error}");
            if cfg!(not(target_vendor = "apple")) {
                panic!("only Apple file systems refuse byte names");
            }
            false
        }
    }
}

fn list(stack: &Stack, pattern: &[u8], match_type: c_int) -> Vec<Vec<u8>> {
    list_in(stack, b"", pattern, match_type)
}

fn list_in(stack: &Stack, path: &[u8], pattern: &[u8], match_type: c_int) -> Vec<Vec<u8>> {
    let (path, pattern) = (raw(path), raw(pattern));
    unsafe {
        let list = uio_getDirList(stack.root, path.as_ptr(), pattern.as_ptr(), match_type);
        assert!(!list.is_null());
        let names = read_list(list);
        uio_DirList_free(list);
        names
    }
}

unsafe fn read_list(list: *const uio_DirList) -> Vec<Vec<u8>> {
    (0..(*list).numNames as usize)
        .map(|i| CStr::from_ptr(*(*list).names.add(i)).to_bytes().to_vec())
        .collect()
}

fn stat_size(stack: &Stack, name: &[u8]) -> Result<i64, c_int> {
    let name = raw(name);
    let mut info: stat = unsafe { std::mem::zeroed() };
    let status = unsafe { uio_stat(stack.root, name.as_ptr(), &mut info) };
    if status == 0 {
        Ok(info.st_size)
    } else {
        Err(errno_value())
    }
}

fn read_file(stack: &Stack, name: &[u8]) -> Result<Vec<u8>, c_int> {
    let name = raw(name);
    unsafe {
        let handle = uio_open(stack.root, name.as_ptr(), 0, 0);
        if handle.is_null() {
            return Err(errno_value());
        }
        let mut bytes = Vec::new();
        (*handle).lock().unwrap().read_to_end(&mut bytes).unwrap();
        uio_close(handle);
        Ok(bytes)
    }
}

fn access_result(stack: &Stack, name: &[u8]) -> Result<(), c_int> {
    let name = raw(name);
    match unsafe { uio_access(stack.root, name.as_ptr(), F_OK) } {
        0 => Ok(()),
        _ => Err(errno_value()),
    }
}

#[test]
#[serial]
fn a_list_built_from_byte_names_reads_back_in_order_and_frees_exactly() {
    let names: BTreeSet<Vec<u8>> = [
        b"b".to_vec(),
        b"a\xff".to_vec(),
        "caf\u{e9}".as_bytes().to_vec(),
        b"caf\xe9".to_vec(),
        b"\xff\xfe".to_vec(),
        b"a".to_vec(),
    ]
    .into_iter()
    .collect();
    let list = dir_list_from_names(&names);
    unsafe {
        assert_eq!((*list).numNames as usize, names.len());
        assert_eq!(
            read_list(list),
            names.iter().cloned().collect::<Vec<_>>(),
            "byte order, not text order"
        );
        let first = *(*list).names;
        assert_eq!(first.cast_const(), (*list).buffer.cast_const());
        uio_DirList_free(list);
    }
}

#[test]
#[serial]
fn lists_of_every_small_size_and_one_large_one_free_without_a_registry() {
    for count in (0..20).chain([5000]) {
        let names: BTreeSet<Vec<u8>> = (0..count)
            .map(|i| format!("name-{i:05}\u{ff}").into_bytes())
            .collect();
        let list = dir_list_from_names(&names);
        assert!(!list.is_null(), "an empty list is still a list");
        unsafe {
            assert_eq!((*list).numNames as usize, count);
            if count == 0 {
                assert!((*list).names.is_null() && (*list).buffer.is_null());
            }
            uio_DirList_free(list);
        }
    }
}

#[test]
#[serial]
fn a_list_survives_the_subsystem_being_reinitialised_before_it_is_freed() {
    let names: BTreeSet<Vec<u8>> = [b"one".to_vec(), b"two\xff".to_vec()].into_iter().collect();
    let list = dir_list_from_names(&names);
    unsafe {
        uio_unInit();
        uio_init();
        assert_eq!(read_list(list), vec![b"one".to_vec(), b"two\xff".to_vec()]);
        uio_DirList_free(list);
    }
}

#[test]
#[serial]
fn a_byte_path_is_never_read_as_the_replacement_character_file() {
    let mut stack = Stack::new();
    let name = format!("{REPLACEMENT_CHARACTER}.dat");
    let host = stack.mount_dir(&[(&name, b"the replacement character file")], UIO_MOUNT_TOP);
    assert_eq!(stat_size(&stack, name.as_bytes()), Ok(30));
    assert_eq!(
        read_file(&stack, name.as_bytes()).as_deref(),
        Ok(&b"the replacement character file"[..])
    );

    let invalid = b"\xff.dat";
    assert_eq!(stat_size(&stack, invalid), Err(libc::ENOENT));
    assert_eq!(read_file(&stack, invalid), Err(libc::ENOENT));
    assert_eq!(access_result(&stack, invalid), Err(libc::ENOENT));
    assert!(host.join(&name).exists());
}

#[test]
#[serial]
fn an_archive_entry_is_not_found_by_bytes_that_only_look_like_its_name() {
    let mut stack = Stack::new();
    let name = format!("{REPLACEMENT_CHARACTER}.dat");
    stack.mount_zip(&[(&name, b"archived")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY, 0);
    assert_eq!(stat_size(&stack, name.as_bytes()), Ok(8));
    assert_eq!(
        read_file(&stack, name.as_bytes()).as_deref(),
        Ok(&b"archived"[..])
    );
    assert_eq!(access_result(&stack, name.as_bytes()), Ok(()));

    for lookup in [&b"\xff.dat"[..], b"\xff/inner", b"\xe9.dat"] {
        assert_eq!(stat_size(&stack, lookup), Err(libc::ENOENT), "{lookup:?}");
        assert_eq!(read_file(&stack, lookup), Err(libc::ENOENT), "{lookup:?}");
        assert_eq!(
            access_result(&stack, lookup),
            Err(libc::ENOENT),
            "{lookup:?}"
        );
    }
}

#[test]
#[serial]
fn a_pattern_that_is_not_utf8_selects_what_it_names_not_everything() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a.txt", b"a"), ("b.txt", b"b")], UIO_MOUNT_TOP);
    assert_eq!(
        list(&stack, b"", MATCH_PREFIX).len(),
        2,
        "empty selects all"
    );
    assert!(list(&stack, b"\xff", MATCH_PREFIX).is_empty());
    assert!(list(&stack, b"\xff", MATCH_REGEX).is_empty());
}

#[test]
#[serial]
fn archive_and_directory_names_merge_into_one_sorted_unique_list() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[
            ("caf\u{e9}.txt", b"zip"),
            ("pack.txt", b"zip"),
            ("zeta.txt", b"zip"),
        ],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    stack.mount_dir(
        &[("caf\u{e9}.txt", b"dir"), ("alpha.txt", b"dir")],
        UIO_MOUNT_TOP,
    );
    let expected: Vec<Vec<u8>> = ["alpha.txt", "caf\u{e9}.txt", "pack.txt", "zeta.txt"]
        .iter()
        .map(|name| name.as_bytes().to_vec())
        .collect();
    assert_eq!(list(&stack, b"", MATCH_PREFIX), expected);
    assert_eq!(
        read_file(&stack, "caf\u{e9}.txt".as_bytes()).as_deref(),
        Ok(&b"dir"[..]),
        "the higher mount provides the shared name"
    );
    assert_eq!(stat_size(&stack, b"pack.txt"), Ok(3));
}

#[test]
#[serial]
fn host_files_named_by_bytes_are_listed_statted_and_read_beside_an_archive() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("caf\u{e9}.txt", b"unicode name in the archive")],
        UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY,
        0,
    );
    let host = stack.mount_dir(&[], UIO_MOUNT_TOP);
    let latin1 = b"caf\xe9.txt";
    let lone_byte = b"\xff";
    let made_latin1 = create_host_file(&host, latin1, b"latin-1 name on the host");
    let made_lone = create_host_file(&host, lone_byte, b"x");

    let mut expected = vec!["caf\u{e9}.txt".as_bytes().to_vec()];
    if made_latin1 {
        expected.push(latin1.to_vec());
    }
    if made_lone {
        expected.push(lone_byte.to_vec());
    }
    expected.sort();
    assert_eq!(list(&stack, b"", MATCH_PREFIX), expected);

    if made_latin1 {
        assert_eq!(stat_size(&stack, latin1), Ok(24));
        assert_eq!(
            read_file(&stack, latin1).as_deref(),
            Ok(&b"latin-1 name on the host"[..])
        );
        assert_eq!(
            list(&stack, b"caf\xe9", MATCH_PREFIX),
            vec![latin1.to_vec()]
        );
    }
    if made_lone {
        assert_eq!(stat_size(&stack, lone_byte), Ok(1));
        assert_eq!(read_file(&stack, lone_byte).as_deref(), Ok(&b"x"[..]));
        assert_eq!(list(&stack, br"^\.?", MATCH_REGEX).len(), expected.len());
    }
    assert_eq!(
        read_file(&stack, "caf\u{e9}.txt".as_bytes()).as_deref(),
        Ok(&b"unicode name in the archive"[..]),
        "the archive's name is a different name from the host's bytes"
    );
}

#[test]
#[serial]
fn a_directory_named_by_other_bytes_is_not_the_archive_directory_it_resembles() {
    let mut stack = Stack::new();
    let inner = format!("{REPLACEMENT_CHARACTER}/inner.txt");
    stack.mount_zip(&[(&inner, b"x")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY, 0);
    assert_eq!(
        list_in(&stack, REPLACEMENT_CHARACTER.as_bytes(), b"", MATCH_PREFIX),
        vec![b"inner.txt".to_vec()]
    );
    assert!(list_in(&stack, b"\xff", b"", MATCH_PREFIX).is_empty());
}
