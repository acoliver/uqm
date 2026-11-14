//! File and directory operations (`stat`, `mkdir`, `rmdir`, `lseek`, `fstat`,
//! `unlink`, `copy_file`) through mount stacks of writable directories,
//! read-only directories and archives, checking results, errno and what ends
//! up on the host.

use std::ffi::CString;

use serial_test::serial;

use super::layered_open_tests::Stack;
use super::*;

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn c(path: &str) -> CString {
    CString::new(path).unwrap()
}

fn stat_of(stack: &Stack, path: &str) -> Option<stat> {
    let mut buf: stat = unsafe { std::mem::zeroed() };
    let path = c(path);
    let result = unsafe { uio_stat(stack.root, path.as_ptr(), &mut buf) };
    (result == 0).then_some(buf)
}

fn mode_kind(buf: &stat) -> libc::mode_t {
    buf.st_mode & libc::S_IFMT
}

fn open(stack: &Stack, path: &str, flags: c_int) -> *mut uio_Handle {
    let path = c(path);
    unsafe { uio_open(stack.root, path.as_ptr(), flags, 0o644) }
}

#[test]
#[serial]
fn stat_reports_size_and_kind_of_host_files_and_directories() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("five.txt", b"12345")], UIO_MOUNT_TOP);
    std::fs::create_dir(root.join("folder")).unwrap();

    let file = stat_of(&stack, "five.txt").expect("file");
    assert_eq!(file.st_size, 5);
    assert_eq!(mode_kind(&file), 0o100000, "regular file");
    assert_ne!(file.st_mode & 0o200, 0, "owner can write");

    let dir = stat_of(&stack, "folder").expect("directory");
    assert_eq!(mode_kind(&dir), 0o040000);
    assert!(stat_of(&stack, "absent").is_none());
    assert_eq!(errno_value(), libc::ENOENT);
}

#[test]
#[serial]
fn stat_marks_a_host_read_only_file_as_unwritable() {
    use std::os::unix::fs::PermissionsExt;
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("frozen.txt", b"ice")], UIO_MOUNT_TOP);
    std::fs::set_permissions(
        root.join("frozen.txt"),
        std::fs::Permissions::from_mode(0o444),
    )
    .unwrap();
    let file = stat_of(&stack, "frozen.txt").unwrap();
    assert_eq!(file.st_mode & 0o777, 0o444);
}

#[test]
#[serial]
fn stat_describes_archive_files_and_directories() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("data/blob.bin", b"0123456789")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let file = stat_of(&stack, "data/blob.bin").expect("entry");
    assert_eq!(file.st_size, 10);
    assert_eq!(file.st_mode, 0o100444);
    let dir = stat_of(&stack, "data").expect("directory");
    assert_eq!(dir.st_size, 0);
    assert_eq!(dir.st_mode, 0o040555);
    assert!(stat_of(&stack, "data/none.bin").is_none());
    assert_eq!(errno_value(), libc::ENOENT);
}

#[test]
#[serial]
fn stat_refuses_missing_arguments() {
    let stack = Stack::new();
    let mut buf: stat = unsafe { std::mem::zeroed() };
    let path = c("x");
    unsafe {
        assert_eq!(uio_stat(ptr::null_mut(), path.as_ptr(), &mut buf), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_stat(stack.root, path.as_ptr(), ptr::null_mut()), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_stat(stack.root, ptr::null(), &mut buf), -1);
        assert_eq!(errno_value(), libc::EINVAL);
    }
}

#[test]
#[serial]
fn fstat_reports_the_size_of_an_open_file() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("five.txt", b"12345")], UIO_MOUNT_TOP);
    let handle = open(&stack, "five.txt", 0);
    let mut buf: stat = unsafe { std::mem::zeroed() };
    unsafe {
        assert_eq!(uio_fstat(handle, &mut buf), 0);
        assert_eq!(buf.st_size, 5);
        assert_eq!(uio_fstat(ptr::null_mut(), &mut buf), -1);
        assert_eq!(uio_fstat(handle, ptr::null_mut()), -1);
        uio_close(handle);
    }
}

#[test]
#[serial]
fn writing_to_an_archive_entry_is_refused() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("fixed.txt", b"abc")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let handle = open(&stack, "fixed.txt", 0);
    assert!(!handle.is_null());
    unsafe {
        assert!(uio_write(handle, b"zzz".as_ptr(), 3) < 3);
        uio_close(handle);
    }
}

#[test]
#[serial]
fn unlinking_removes_a_writable_file_and_spares_a_read_only_mount() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("temp.txt", b"x")], UIO_MOUNT_TOP);
    unsafe {
        assert_eq!(uio_unlink(stack.root, c("temp.txt").as_ptr()), 0);
        assert!(!root.join("temp.txt").exists());
        assert_eq!(uio_unlink(stack.root, c("temp.txt").as_ptr()), -1);
        assert_eq!(errno_value(), libc::ENOENT);
    }

    let mut stack = Stack::new();
    let root = stack.mount_dir(&[("keep.txt", b"x")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    unsafe {
        assert_eq!(uio_unlink(stack.root, c("keep.txt").as_ptr()), -1);
        assert_eq!(errno_value(), libc::EACCES);
    }
    assert!(root.join("keep.txt").exists());
}
