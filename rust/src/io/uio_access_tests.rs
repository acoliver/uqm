//! Permission checks (`uio_access`) and host-path access (`uio_get_stdio_access`)
//! against mount stacks that mix writable directories, read-only directories
//! and archives, checking the result code and the errno C callers read.

use std::ffi::{CStr, CString};

use serial_test::serial;
use tempfile::TempDir;

use super::layered_open_tests::Stack;
use super::*;

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn access(stack: &Stack, path: &str, mode: c_int) -> (c_int, c_int) {
    let path = CString::new(path).unwrap();
    let result = unsafe { uio_access(stack.root, path.as_ptr(), mode) };
    (result, if result == 0 { 0 } else { errno_value() })
}

fn stdio_access(
    stack: &Stack,
    path: &str,
    temp: *mut uio_DirHandle,
) -> (*mut uio_StdioAccessHandle, c_int) {
    let path = CString::new(path).unwrap();
    let handle = unsafe { uio_get_stdio_access(stack.root, path.as_ptr(), 0, temp) };
    (handle, if handle.is_null() { errno_value() } else { 0 })
}

/// A directory handle that stores extracted archive entries.
struct TempHandle {
    dir: TempDir,
    handle: *mut uio_DirHandle,
}

impl TempHandle {
    fn new(repo: *mut uio_Repository) -> Self {
        let dir = TempDir::new().unwrap();
        let handle = Box::into_raw(Box::new(uio_DirHandle {
            path: dir.path().to_path_buf(),
            virtual_path: dir.path().to_path_buf(),
            refcount: std::sync::atomic::AtomicI32::new(1),
            repository: repo,
            root_end: dir.path().to_path_buf(),
        }));
        TempHandle { dir, handle }
    }
}

impl Drop for TempHandle {
    fn drop(&mut self) {
        unsafe { drop(Box::from_raw(self.handle)) };
    }
}

#[test]
#[serial]
fn existing_paths_are_reachable_and_missing_ones_are_not() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("here.txt", b"x")], UIO_MOUNT_TOP);
    assert_eq!(access(&stack, "here.txt", F_OK), (0, 0));
    assert_eq!(access(&stack, "here.txt", R_OK), (0, 0));
    assert_eq!(access(&stack, "gone.txt", F_OK), (-1, libc::ENOENT));
    assert_eq!(access(&stack, "/", F_OK), (0, 0), "the mount root itself");
}

#[test]
#[serial]
fn access_refuses_malformed_requests() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("here.txt", b"x")], UIO_MOUNT_TOP);
    let path = CString::new("here.txt").unwrap();
    unsafe {
        assert_eq!(uio_access(ptr::null_mut(), path.as_ptr(), F_OK), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_access(stack.root, ptr::null(), F_OK), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(
            uio_access(stack.root, path.as_ptr(), 0x40),
            -1,
            "unknown mode bits"
        );
        assert_eq!(errno_value(), libc::EINVAL);
    }
}

#[test]
#[serial]
fn writing_is_refused_on_a_read_only_mount_and_allowed_elsewhere() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("config.txt", b"x")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    assert_eq!(access(&stack, "config.txt", W_OK), (-1, libc::EACCES));
    assert_eq!(access(&stack, "config.txt", R_OK), (0, 0));

    let mut stack = Stack::new();
    stack.mount_dir(&[("save.txt", b"x")], UIO_MOUNT_TOP);
    assert_eq!(access(&stack, "save.txt", W_OK), (0, 0));
    assert_eq!(access(&stack, "save.txt", R_OK | W_OK), (0, 0));
}

#[test]
#[serial]
fn a_file_the_host_marks_read_only_is_not_writable() {
    use std::os::unix::fs::PermissionsExt;
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("locked.txt", b"x")], UIO_MOUNT_TOP);
    let file = root.join("locked.txt");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).unwrap();
    assert_eq!(access(&stack, "locked.txt", W_OK), (-1, libc::EACCES));
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(access(&stack, "locked.txt", W_OK), (0, 0));
}

#[test]
#[serial]
fn execute_permission_follows_the_host_mode_and_directories_are_traversable() {
    use std::os::unix::fs::PermissionsExt;
    let mut stack = Stack::new();
    let root = stack.mount_dir(
        &[("tool.sh", b"#!/bin/sh"), ("data.txt", b"x")],
        UIO_MOUNT_TOP,
    );
    std::fs::create_dir(root.join("sub")).unwrap();
    std::fs::set_permissions(root.join("tool.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(
        root.join("data.txt"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(access(&stack, "tool.sh", X_OK), (0, 0));
    assert_eq!(access(&stack, "data.txt", X_OK), (-1, libc::EACCES));
    assert_eq!(access(&stack, "sub", X_OK), (0, 0));
}

#[test]
#[serial]
fn archive_entries_exist_and_are_read_only_but_directories_can_be_entered() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("top.txt", b"top"), ("folder/inner.txt", b"inner")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    assert_eq!(access(&stack, "top.txt", F_OK), (0, 0));
    assert_eq!(access(&stack, "folder/inner.txt", R_OK), (0, 0));
    assert_eq!(access(&stack, "missing.txt", F_OK), (-1, libc::ENOENT));
    assert_eq!(access(&stack, "top.txt", W_OK), (-1, libc::EACCES));
    assert_eq!(
        access(&stack, "top.txt", X_OK),
        (-1, libc::EACCES),
        "archive files are not programs"
    );
    assert_eq!(
        access(&stack, "folder", X_OK),
        (0, 0),
        "an archive directory can be entered"
    );
}

#[test]
#[serial]
fn a_loose_file_can_be_reached_by_path_on_the_host() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("loose.txt", b"loose")], UIO_MOUNT_TOP);
    let (handle, errno) = stdio_access(&stack, "loose.txt", ptr::null_mut());
    assert!(!handle.is_null(), "errno {errno}");
    unsafe {
        let path = CStr::from_ptr(uio_stdio_access_handle_get_path(handle))
            .to_str()
            .unwrap();
        assert_eq!(std::path::Path::new(path), root.join("loose.txt"));
        assert_eq!(std::fs::read(path).unwrap(), b"loose");
        uio_release_stdio_access(handle);
    }
    assert!(
        root.join("loose.txt").exists(),
        "releasing a direct path keeps the file"
    );
}

#[test]
#[serial]
fn host_path_access_refuses_directories_missing_files_and_unmounted_paths() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP);
    std::fs::create_dir(root.join("folder")).unwrap();
    assert_eq!(
        stdio_access(&stack, "folder", ptr::null_mut()),
        (ptr::null_mut(), libc::EISDIR)
    );
    assert_eq!(
        stdio_access(&stack, "nope.txt", ptr::null_mut()),
        (ptr::null_mut(), libc::ENOENT)
    );

    let empty = Stack::new();
    assert_eq!(
        stdio_access(&empty, "a.txt", ptr::null_mut()),
        (ptr::null_mut(), libc::ENOENT)
    );

    let path = CString::new("a.txt").unwrap();
    unsafe {
        assert!(uio_get_stdio_access(ptr::null_mut(), path.as_ptr(), 0, ptr::null_mut()).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_get_stdio_access(stack.root, ptr::null(), 0, ptr::null_mut()).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_stdio_access_handle_get_path(ptr::null_mut()).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        uio_release_stdio_access(ptr::null_mut());
    }
}

#[test]
#[serial]
fn an_archive_entry_is_copied_to_the_temp_directory_and_removed_on_release() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("data/level.bin", b"level bytes"), ("data/other.bin", b"o")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let temp = TempHandle::new(stack.repo);
    let (handle, errno) = stdio_access(&stack, "data/level.bin", temp.handle);
    assert!(!handle.is_null(), "errno {errno}");
    unsafe {
        let path = CStr::from_ptr(uio_stdio_access_handle_get_path(handle))
            .to_str()
            .unwrap();
        assert!(std::path::Path::new(path).starts_with(temp.dir.path()));
        assert_eq!(std::fs::read(path).unwrap(), b"level bytes");
        let copy = std::path::PathBuf::from(path);
        uio_release_stdio_access(handle);
        assert!(!copy.exists(), "the temporary copy is deleted on release");
    }
}

#[test]
#[serial]
fn an_archive_entry_cannot_be_reached_without_a_temp_directory() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("data/level.bin", b"x")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    assert_eq!(
        stdio_access(&stack, "data/level.bin", ptr::null_mut()),
        (ptr::null_mut(), libc::EINVAL)
    );
}

#[test]
#[serial]
fn archive_directories_and_missing_entries_are_not_files() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("data/level.bin", b"x")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let temp = TempHandle::new(stack.repo);
    assert_eq!(
        stdio_access(&stack, "data", temp.handle),
        (ptr::null_mut(), libc::EISDIR)
    );
    assert_eq!(
        stdio_access(&stack, "data/none.bin", temp.handle),
        (ptr::null_mut(), libc::ENOENT)
    );
}
