//! File and directory operations (`stat`, `mkdir`, `rmdir`, `lseek`, `fstat`,
//! `unlink`, `copyFile`) through mount stacks of writable directories,
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
fn directories_can_be_made_and_removed_on_a_writable_mount() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[], UIO_MOUNT_TOP);
    unsafe {
        assert_eq!(uio_mkdir(stack.root, c("saves").as_ptr(), 0o755), 0);
        assert!(root.join("saves").is_dir());
        assert_eq!(uio_mkdir(stack.root, c("saves").as_ptr(), 0o755), -1);
        assert_eq!(errno_value(), libc::EEXIST);
        assert_eq!(
            uio_mkdir(stack.root, c("a/b").as_ptr(), 0o755),
            -1,
            "parent is missing"
        );
        assert_eq!(errno_value(), libc::ENOENT);

        std::fs::write(root.join("saves/slot1"), b"x").unwrap();
        assert_eq!(uio_rmdir(stack.root, c("saves").as_ptr()), -1);
        assert_eq!(errno_value(), libc::ENOTEMPTY);
        std::fs::remove_file(root.join("saves/slot1")).unwrap();
        assert_eq!(uio_rmdir(stack.root, c("saves").as_ptr()), 0);
        assert!(!root.join("saves").exists());
        assert_eq!(uio_rmdir(stack.root, c("saves").as_ptr()), -1);
        assert_eq!(errno_value(), libc::ENOENT);
    }
}

#[test]
#[serial]
fn a_read_only_mount_refuses_directory_changes() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(&[], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    std::fs::create_dir(root.join("fixed")).unwrap();
    unsafe {
        assert_eq!(uio_mkdir(stack.root, c("new").as_ptr(), 0o755), -1);
        assert_eq!(errno_value(), libc::EACCES);
        assert_eq!(uio_rmdir(stack.root, c("fixed").as_ptr()), -1);
        assert_eq!(errno_value(), libc::EACCES);
    }
    assert!(root.join("fixed").is_dir());
    assert!(!root.join("new").exists());
}

#[test]
#[serial]
fn directory_changes_refuse_missing_arguments() {
    let stack = Stack::new();
    unsafe {
        assert_eq!(uio_mkdir(ptr::null_mut(), c("d").as_ptr(), 0o755), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_mkdir(stack.root, ptr::null(), 0o755), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_rmdir(ptr::null_mut(), c("d").as_ptr()), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_rmdir(stack.root, ptr::null()), -1);
        assert_eq!(errno_value(), libc::EINVAL);
    }
}

#[test]
#[serial]
fn seeking_a_handle_moves_the_read_position() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("digits.txt", b"0123456789")], UIO_MOUNT_TOP);
    let handle = open(&stack, "digits.txt", 0);
    assert!(!handle.is_null());
    let mut byte = [0u8; 1];
    unsafe {
        assert_eq!(uio_lseek(handle, 4, libc::SEEK_SET), 0);
        assert_eq!(uio_read(handle, byte.as_mut_ptr(), 1), 1);
        assert_eq!(byte[0], b'4');
        assert_eq!(uio_lseek(handle, 2, libc::SEEK_CUR), 0);
        assert_eq!(uio_read(handle, byte.as_mut_ptr(), 1), 1);
        assert_eq!(byte[0], b'7');
        assert_eq!(uio_lseek(handle, -1, libc::SEEK_END), 0);
        assert_eq!(uio_read(handle, byte.as_mut_ptr(), 1), 1);
        assert_eq!(byte[0], b'9');
        assert_eq!(uio_lseek(handle, 0, 99), -1, "unknown whence");
        assert_eq!(
            uio_lseek(handle, -100, libc::SEEK_CUR),
            -1,
            "before the start"
        );
        assert_eq!(uio_lseek(ptr::null_mut(), 0, libc::SEEK_SET), -1);
        uio_close(handle);
    }
}

#[test]
#[serial]
fn seeking_inside_an_archive_entry_works_like_a_file() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("digits.txt", b"0123456789")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let handle = open(&stack, "digits.txt", 0);
    assert!(!handle.is_null());
    let mut byte = [0u8; 1];
    unsafe {
        assert_eq!(uio_lseek(handle, 6, libc::SEEK_SET), 0);
        assert_eq!(uio_read(handle, byte.as_mut_ptr(), 1), 1);
        assert_eq!(byte[0], b'6');
        uio_close(handle);
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

#[test]
#[serial]
fn copying_a_file_duplicates_every_byte_across_chunk_boundaries() {
    let mut stack = Stack::new();
    let big: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
    let root = stack.mount_dir(&[("big.bin", &big)], UIO_MOUNT_TOP);
    unsafe {
        let result = uio_copyFile(
            stack.root,
            c("big.bin").as_ptr(),
            stack.root,
            c("copy.bin").as_ptr(),
        );
        assert_eq!(result, 0);
    }
    assert_eq!(std::fs::read(root.join("copy.bin")).unwrap(), big);
    assert_eq!(
        std::fs::read(root.join("big.bin")).unwrap(),
        big,
        "the source is untouched"
    );
}

#[test]
#[serial]
fn copying_never_overwrites_and_reports_a_missing_source() {
    let mut stack = Stack::new();
    let root = stack.mount_dir(
        &[("a.txt", b"source"), ("b.txt", b"existing")],
        UIO_MOUNT_TOP,
    );
    unsafe {
        assert_eq!(
            uio_copyFile(
                stack.root,
                c("a.txt").as_ptr(),
                stack.root,
                c("b.txt").as_ptr()
            ),
            -1
        );
        assert_eq!(errno_value(), libc::EEXIST);
        assert_eq!(
            uio_copyFile(
                stack.root,
                c("gone.txt").as_ptr(),
                stack.root,
                c("c.txt").as_ptr()
            ),
            -1
        );
        assert_eq!(errno_value(), libc::ENOENT);
    }
    assert_eq!(std::fs::read(root.join("b.txt")).unwrap(), b"existing");
    assert!(!root.join("c.txt").exists());
}

#[test]
#[serial]
fn an_archive_entry_can_be_copied_out_to_a_writable_directory() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("packed.txt", b"from the archive")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let root = stack.mount_dir(&[], UIO_MOUNT_TOP);
    unsafe {
        assert_eq!(
            uio_copyFile(
                stack.root,
                c("packed.txt").as_ptr(),
                stack.root,
                c("out.txt").as_ptr()
            ),
            0
        );
    }
    assert_eq!(
        std::fs::read(root.join("out.txt")).unwrap(),
        b"from the archive"
    );
}

#[test]
#[serial]
fn copying_refuses_missing_arguments() {
    let stack = Stack::new();
    unsafe {
        let a = c("a");
        assert_eq!(
            uio_copyFile(ptr::null_mut(), a.as_ptr(), stack.root, a.as_ptr()),
            -1
        );
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(
            uio_copyFile(stack.root, a.as_ptr(), ptr::null_mut(), a.as_ptr()),
            -1
        );
        assert_eq!(
            uio_copyFile(stack.root, ptr::null(), stack.root, a.as_ptr()),
            -1
        );
        assert_eq!(
            uio_copyFile(stack.root, a.as_ptr(), stack.root, ptr::null()),
            -1
        );
    }
}
