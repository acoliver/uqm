use super::*;
use crate::host::Files;
use std::io::{Read, Seek, SeekFrom, Write};

#[test]
fn files_port_roundtrips_and_removes_bytes() {
    let root = concat!(env!("UQM_RUST_ROOT"), "/../tmp");
    std::fs::create_dir_all(root).unwrap();
    let temp = tempfile::tempdir_in(root).unwrap();
    let files = Vfs::default();
    let path = temp.path().join("slot.sav");
    let mut output = files.open_write(&path).unwrap();
    output.write_all(b"\0save\xff\n").unwrap();
    output.flush().unwrap();
    drop(output);
    assert_eq!(files.stat(&path).unwrap().len, 7);
    let mut input = files.open_read(&path).unwrap();
    assert_eq!(input.seek(SeekFrom::End(-2)).unwrap(), 5);
    let mut tail = Vec::new();
    input.read_to_end(&mut tail).unwrap();
    assert_eq!(tail, b"\xff\n");
    assert_eq!(files.list(temp.path()).unwrap(), ["slot.sav"]);
    files.remove(&path).unwrap();
    assert_eq!(
        files.open_read(&path).err().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
#[serial_test::serial]
fn retained_config_writes_ignore_later_content_mounts() {
    let mut stack = layered_open_tests::Stack::new();
    let config = stack.mount_dir(&[], UIO_MOUNT_TOP);
    // SAFETY: the fixture owns the repository and the directory is closed below.
    let directory = unsafe { uio_openDir(stack.repo, c"/".as_ptr(), 0) };
    // SAFETY: directory is live until the matching close.
    let files = unsafe { owned::directory(directory) }.unwrap();
    let content = stack.mount_dir(&[], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let path = Path::new("uqmsave.01");
    let mut output = files.open_write(path).unwrap();
    output.write_all(b"save bytes").unwrap();
    output.finish().unwrap();
    assert_eq!(std::fs::read(config.join(path)).unwrap(), b"save bytes");
    assert!(!content.join(path).exists());
    assert_eq!(files.read(path).unwrap(), b"save bytes");
    files.remove(path).unwrap();
    assert!(!config.join(path).exists());
    // SAFETY: release exactly the reference acquired above.
    unsafe { uio_closeDir(directory) };
}

#[test]
#[cfg(target_os = "linux")]
fn files_listing_preserves_lossy_native_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = concat!(env!("UQM_RUST_ROOT"), "/../tmp");
    std::fs::create_dir_all(root).unwrap();
    let temp = tempfile::tempdir_in(root).unwrap();
    let name = OsString::from_vec(b"ship-\xff.ogg".to_vec());
    std::fs::write(temp.path().join(name), b"sound").unwrap();
    assert_eq!(
        Vfs::default().list(temp.path()).unwrap(),
        ["ship-\u{fffd}.ogg"]
    );
}

#[test]
#[serial_test::serial]
fn retained_directory_listing_returns_empty_for_an_unmounted_missing_path() {
    let root = concat!(env!("UQM_RUST_ROOT"), "/../tmp");
    std::fs::create_dir_all(root).unwrap();
    let temp = tempfile::tempdir_in(root).unwrap();
    let missing = temp.path().join("missing");
    assert_eq!(
        Vfs::default().list(&missing).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    let stack = layered_open_tests::Stack::new();
    let path = std::ffi::CString::new(missing.to_str().unwrap()).unwrap();
    // SAFETY: the fixture and path outlive their directory and list handles.
    unsafe {
        let directory = uio_openDir(stack.repo, path.as_ptr(), 0);
        let list = uio_getDirList(directory, ptr::null(), ptr::null(), 0);
        assert!(!list.is_null());
        assert_eq!((*list).numNames, 0);
        uio_DirList_free(list);
        uio_closeDir(directory);
    }
}
