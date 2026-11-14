//! Mount bookkeeping and handle-level file calls on a real mount stack:
//! where a path resolves, what unmounting removes, and what read, write,
//! stat and unlink do to the files underneath.

use crate::host::Files;
use std::ffi::CString;
use std::io::{Read, Write};

use serial_test::serial;

use super::layered_open_tests::Stack;
use super::*;

fn cstr(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn mount_ptr(stack: &Stack, index: usize) -> *mut uio_MountHandle {
    let _ = stack;
    get_mount_registry().lock().unwrap().mounts[index].handle_ptr as *mut uio_MountHandle
}

fn read_mounted_zip(mount_point: &str, parent: &str) -> Option<Vec<u8>> {
    use std::io::Write;

    let stack = Stack::new();
    let host = tempfile::TempDir::new().unwrap();
    let archive_dir = host.path().join(parent.trim_start_matches('/'));
    std::fs::create_dir_all(&archive_dir).unwrap();
    let archive_path = archive_dir.join("packed.ani");
    let mut archive = zip::ZipWriter::new(std::fs::File::create(archive_path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    archive.start_file("packed.ani", options).unwrap();
    archive.write_all(b"archive animation contents").unwrap();
    archive.finish().unwrap();
    stack.mount(
        UIO_FSTYPE_STDIO,
        UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY,
        host.path(),
        ptr::null_mut(),
    );

    unsafe {
        let parent_path = cstr(parent);
        let dir = uio_openDir(stack.repo, parent_path.as_ptr(), 0);
        assert!(!dir.is_null());
        let point = cstr(mount_point);
        let name = cstr("packed.ani");
        let root = cstr("/");
        let mount = uio_mountDir(
            stack.repo,
            point.as_ptr(),
            UIO_FSTYPE_ZIP,
            dir,
            name.as_ptr(),
            root.as_ptr(),
            ptr::null_mut(),
            UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
            ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let data = owned::directory(dir)
            .unwrap()
            .read(Path::new("packed.ani"))
            .ok();
        assert_eq!(uio_unmountDir(mount), 0);
        uio_closeDir(dir);
        data
    }
}

#[test]
#[serial]
fn root_zip_mount_spellings_read_archive_entries() {
    for point in [".", "/", "", "./"] {
        for parent in [".", "/", ""] {
            assert_eq!(
                read_mounted_zip(point, parent).as_deref(),
                Some(b"archive animation contents".as_slice()),
                "mount={point:?}, parent={parent:?}"
            );
        }
    }
}

#[test]
#[serial]
fn relative_zip_mounts_and_directory_handles_read_archive_entries() {
    for point in ["animation", "/animation", "./animation/"] {
        for parent in ["animation", "/animation", "./animation/"] {
            assert_eq!(
                read_mounted_zip(point, parent).as_deref(),
                Some(b"archive animation contents".as_slice()),
                "mount={point:?}, parent={parent:?}"
            );
        }
    }
}

#[test]
#[serial]
fn unmounting_removes_the_files_it_provided() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let top = stack.first_mount_id();
    stack.mount_zip(&[("b.txt", b"b")], UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY, top);
    assert!(stack.read("a.txt").is_some() && stack.read("b.txt").is_some());

    let zip = mount_ptr(&stack, 1);
    assert_eq!(unsafe { uio_unmountDir(zip) }, 0);

    assert!(stack.read("a.txt").is_some());
    assert_eq!(stack.read("b.txt"), None);
    assert_eq!(get_mount_registry().lock().unwrap().mounts.len(), 1);
}

#[test]
#[serial]
fn unmounting_an_unknown_or_null_mount_fails() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let mut stranger = uio_MountHandle {
        repository: stack.repo,
        id: usize::MAX,
        fs_type: UIO_FSTYPE_STDIO,
    };
    unsafe {
        assert_eq!(uio_unmountDir(ptr::null_mut()), -1);
        assert_eq!(uio_unmountDir(&mut stranger), -1);
    }
    assert!(stack.read("a.txt").is_some(), "the real mount is untouched");
}

#[test]
#[serial]
fn transplanting_a_directory_requires_a_read_only_mount() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let source = Box::into_raw(Box::new(uio_DirHandle {
        path: host.clone(),
        virtual_path: PathBuf::from("/"),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: stack.repo,
        root_end: host,
    }));
    let point = cstr("/copy");
    unsafe {
        assert!(uio_transplantDir(
            point.as_ptr(),
            ptr::null_mut(),
            UIO_MOUNT_RDONLY,
            ptr::null_mut()
        )
        .is_null());
        assert!(
            uio_transplantDir(point.as_ptr(), source, 0, ptr::null_mut()).is_null(),
            "read-write transplants are refused"
        );
        assert!(uio_transplantDir(
            ptr::null(),
            source,
            UIO_MOUNT_RDONLY | UIO_MOUNT_TOP,
            ptr::null_mut()
        )
        .is_null());
        assert!(
            uio_transplantDir(
                point.as_ptr(),
                source,
                UIO_MOUNT_RDONLY | UIO_MOUNT_ABOVE,
                ptr::null_mut()
            )
            .is_null(),
            "ABOVE needs a relative mount"
        );
        let anchor = mount_ptr(&stack, 0);
        assert!(
            uio_transplantDir(
                point.as_ptr(),
                source,
                UIO_MOUNT_RDONLY | UIO_MOUNT_TOP,
                anchor
            )
            .is_null(),
            "TOP must not name a relative mount"
        );
        let _ = Box::from_raw(source);
    }
}

#[test]
#[serial]
fn a_transplanted_directory_exposes_its_files_at_the_new_mount_point() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(
        &[("a.txt", b"transplanted")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
    );
    let source = Box::into_raw(Box::new(uio_DirHandle {
        path: host.clone(),
        virtual_path: PathBuf::from("/"),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: stack.repo,
        root_end: host,
    }));
    let point = cstr("/copy");
    let mounted = unsafe {
        uio_transplantDir(
            point.as_ptr(),
            source,
            UIO_MOUNT_RDONLY | UIO_MOUNT_TOP,
            ptr::null_mut(),
        )
    };
    assert!(!mounted.is_null());
    assert_eq!(
        stack.read("copy/a.txt").as_deref(),
        Some(&b"transplanted"[..])
    );
    unsafe {
        let _ = Box::from_raw(source);
    }
}

#[test]
#[serial]
fn files_read_and_stat_the_underlying_file() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("data.bin", b"0123456789")], UIO_MOUNT_TOP);
    let files = unsafe { owned::directory(stack.root) }.unwrap();
    let mut input = files.open_read(Path::new("data.bin")).unwrap();
    let mut buf = [0; 4];
    assert_eq!(input.read(&mut buf).unwrap(), 4);
    assert_eq!(&buf, b"0123");
    assert_eq!(input.read(&mut buf).unwrap(), 4);
    assert_eq!(&buf, b"4567");
    assert_eq!(input.read(&mut buf).unwrap(), 2);
    assert_eq!(input.read(&mut buf).unwrap(), 0);
    let info = files.stat(Path::new("data.bin")).unwrap();
    assert_eq!(info.len, 10);
    assert!(info.is_file && !info.is_dir);
}

#[test]
#[serial]
fn writing_through_files_changes_the_file_on_disk() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[], UIO_MOUNT_TOP);
    let files = unsafe { owned::directory(stack.root) }.unwrap();
    let mut output = files.open_write(Path::new("new.bin")).unwrap();
    output.write_all(b"payload").unwrap();
    output.finish().unwrap();
    assert_eq!(std::fs::read(host.join("new.bin")).unwrap(), b"payload");
}

#[test]
#[serial]
fn unlink_removes_a_file_from_a_writable_mount() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("gone.txt", b"x"), ("kept.txt", b"y")], UIO_MOUNT_TOP);
    let files = unsafe { owned::directory(stack.root) }.unwrap();
    files.remove(Path::new("gone.txt")).unwrap();
    assert!(!host.join("gone.txt").exists());
    assert!(host.join("kept.txt").exists());
    assert_eq!(stack.read("gone.txt"), None);
}

#[test]
#[serial]
fn files_refuse_read_only_writes_and_removals() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("locked.txt", b"x")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let files = unsafe { owned::directory(stack.root) }.unwrap();
    assert_eq!(
        files.remove(Path::new("locked.txt")).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(host.join("locked.txt").exists());
    assert_eq!(
        files
            .open_write(Path::new("locked.txt"))
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        files.remove(Path::new("missing.txt")).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
#[serial]
fn files_stat_preserves_host_and_archive_size_and_kind() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("five.txt", b"12345")], UIO_MOUNT_TOP);
    std::fs::create_dir(root.join("folder")).unwrap();
    stack.mount_zip(
        &[("data/blob.bin", b"0123456789")],
        UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY,
        0,
    );
    let files = unsafe { owned::directory(stack.root) }.unwrap();
    let file = files.stat(Path::new("five.txt")).unwrap();
    assert_eq!(file.len, 5);
    assert!(file.is_file && !file.is_dir);
    let directory = files.stat(Path::new("folder")).unwrap();
    assert!(directory.is_dir && !directory.is_file);
    let archived = files.stat(Path::new("data/blob.bin")).unwrap();
    assert_eq!(archived.len, 10);
    assert!(archived.is_file && !archived.is_dir);
    let directory = files.stat(Path::new("data")).unwrap();
    assert_eq!(directory.len, 0);
    assert!(directory.is_dir && !directory.is_file);
    assert_eq!(
        files.stat(Path::new("data/none.bin")).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
#[serial]
fn an_archive_entry_copies_to_a_writable_directory_through_the_same_files_port() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("packed.txt", b"from the archive")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let root = stack.mount_dir(&[], UIO_MOUNT_TOP);
    let files = unsafe { owned::directory(stack.root) }.unwrap();
    let mut input = files.open_read(Path::new("packed.txt")).unwrap();
    let mut output = files.open_write(Path::new("out.txt")).unwrap();
    std::io::copy(&mut input, &mut output).unwrap();
    output.finish().unwrap();
    assert_eq!(
        std::fs::read(root.join("out.txt")).unwrap(),
        b"from the archive"
    );
    assert_eq!(
        files.read(Path::new("packed.txt")).unwrap(),
        b"from the archive"
    );
}
