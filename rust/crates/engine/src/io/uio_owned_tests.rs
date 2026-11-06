#[test]
#[serial_test::serial]
fn directory_entries_match_the_existing_provider_order_and_filter() {
    use crate::strings::string_table::*;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/uio-owned-tests");
    std::fs::create_dir_all(&root).unwrap();
    let scratch = tempfile::TempDir::new_in(root).unwrap();
    for name in ["zeta.mle", "Alpha.mle", ".hidden.mle", "other.txt"] {
        std::fs::write(scratch.path().join(name), b"data").unwrap();
    }
    std::fs::create_dir(scratch.path().join("folder.mle")).unwrap();
    let path = CString::new(scratch.path().to_str().unwrap()).unwrap();
    // SAFETY: the fixture retains the directory while both provider enumerations run.
    unsafe {
        let repository = uio_openRepository(0);
        let mount = uio_mountDir(
            repository,
            c"/".as_ptr(),
            1,
            std::ptr::null_mut(),
            std::ptr::null(),
            path.as_ptr(),
            std::ptr::null_mut(),
            4,
            std::ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let raw = uio_openDir(repository, c"/".as_ptr(), 0);
        let retained = Directory::retain(std::ptr::NonNull::new(raw).unwrap());
        let table =
            crate::resource::direct::LoadDirEntryTable(raw, c"".as_ptr(), c".mle".as_ptr(), 3);
        let original: Vec<_> = (0..GetStringTableCount(CaptureStringTable(table)))
            .map(|index| {
                std::ffi::CStr::from_ptr(GetStringAddress(SetAbsStringTableIndex(
                    CaptureStringTable(table),
                    index,
                )))
                .to_owned()
            })
            .collect();
        FreeStringTable(table);
        let names = retained.files_with_suffix(c".mle");
        assert_eq!(names, original);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&c"Alpha.mle".to_owned()));
        assert!(names.contains(&c"zeta.mle".to_owned()));
        uio_closeDir(raw);
        drop(retained);
        uio_unmountDir(mount);
        uio_closeRepository(repository);
    }
}

use super::*;
use crate::io::uio_owned::Directory;

#[test]
#[serial_test::serial]
fn retained_directory_and_stream_survive_mount_and_repository_release() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/uio-owned-tests");
    std::fs::create_dir_all(&root).unwrap();
    let scratch = tempfile::TempDir::new_in(root).unwrap();
    std::fs::write(scratch.path().join("fleet.mle"), b"retained").unwrap();
    let path = CString::new(scratch.path().to_str().unwrap()).unwrap();
    // SAFETY: the fixture owns the repository, mount and original directory reference.
    let (directory, mut input) = unsafe {
        let repository = uio_openRepository(0);
        let mount = uio_mountDir(
            repository,
            c"/".as_ptr(),
            1,
            std::ptr::null_mut(),
            std::ptr::null(),
            path.as_ptr(),
            std::ptr::null_mut(),
            4,
            std::ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let raw = uio_openDir(repository, c"/".as_ptr(), 0);
        let directory = Directory::retain(std::ptr::NonNull::new(raw).unwrap());
        let input = directory.open(c"fleet.mle").unwrap();
        assert_eq!(uio_unmountDir(mount), 0);
        uio_closeRepository(repository);
        uio_closeDir(raw);
        (directory, input)
    };
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"retained");
    bytes.clear();
    directory
        .open(c"fleet.mle")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"retained");
}
use std::ffi::CString;
use std::io::{Read, Write};

#[test]
#[serial_test::serial]
fn retained_directory_and_stream_outlive_the_source_handle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/uio-owned-tests");
    std::fs::create_dir_all(&root).unwrap();
    let scratch = tempfile::TempDir::new_in(root).unwrap();
    let path = CString::new(scratch.path().to_str().unwrap()).unwrap();
    // SAFETY: the fixture creates and releases its own UIO directory reference.
    let directory = unsafe {
        let raw = uio_openDir(std::ptr::null_mut(), path.as_ptr(), 0);
        let retained = Directory::retain(std::ptr::NonNull::new(raw).unwrap());
        uio_closeDir(raw);
        retained
    };
    let clone = directory.clone();
    let mut output = directory.create(c"fleet.mle").unwrap();
    drop(directory);
    output.write_all(b"legacy team bytes").unwrap();
    output.finish().unwrap();
    let mut input = clone.open(c"fleet.mle").unwrap();
    assert_eq!(input.len().unwrap(), 17);
    drop(clone);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"legacy team bytes");
    assert_eq!(input.read(&mut [0; 1]).unwrap(), 0);
}

#[test]
#[serial_test::serial]
fn directory_operations_preserve_errors_and_truncated_reads() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/uio-owned-tests");
    std::fs::create_dir_all(&root).unwrap();
    let scratch = tempfile::TempDir::new_in(root).unwrap();
    let path = CString::new(scratch.path().to_str().unwrap()).unwrap();
    // SAFETY: the fixture transfers its newly opened directory to the retained owner.
    let directory = unsafe {
        let raw = uio_openDir(std::ptr::null_mut(), path.as_ptr(), 0);
        let retained = Directory::retain(std::ptr::NonNull::new(raw).unwrap());
        uio_closeDir(raw);
        retained
    };
    assert!(directory.open(c"missing.mle").is_err());
    let mut output = directory.create(c"short.mle").unwrap();
    output.write_all(b"abc").unwrap();
    output.finish().unwrap();
    let mut input = directory.open(c"short.mle").unwrap();
    assert_eq!(
        input.read_exact(&mut [0; 4]).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    directory.remove(c"short.mle").unwrap();
    assert!(directory.open(c"short.mle").is_err());
}
