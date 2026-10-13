//! Mount bookkeeping and handle-level file calls on a real mount stack:
//! where a path resolves, what unmounting removes, and what read, write,
//! stat and unlink do to the files underneath.

use std::ffi::{CStr, CString};

use serial_test::serial;

use super::layered_open_tests::Stack;
use super::*;

fn cstr(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn locate(stack: &Stack, name: &str) -> Option<(*mut uio_MountHandle, String)> {
    let name = cstr(name);
    let mut mount: *mut uio_MountHandle = ptr::null_mut();
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { uio_get_file_location(stack.root, name.as_ptr(), 0, &mut mount, &mut out) };
    if rc != 0 {
        assert!(
            mount.is_null() && out.is_null(),
            "failure clears the outputs"
        );
        return None;
    }
    let text = unsafe { CStr::from_ptr(out) }
        .to_string_lossy()
        .into_owned();
    unsafe { libc::free(out as *mut libc::c_void) };
    Some((mount, text))
}

fn mount_ptr(stack: &Stack, index: usize) -> *mut uio_MountHandle {
    let _ = stack;
    get_mount_registry().lock().unwrap()[index].handle_ptr as *mut uio_MountHandle
}

#[test]
#[serial]
fn file_location_names_the_mount_and_the_host_path() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let (mount, path) = locate(&stack, "a.txt").expect("file is under the mount");
    assert_eq!(mount, mount_ptr(&stack, 0));
    assert_eq!(
        std::fs::canonicalize(&path).unwrap(),
        std::fs::canonicalize(host.join("a.txt")).unwrap()
    );
}

#[test]
#[serial]
fn file_location_outputs_are_optional() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let name = cstr("a.txt");
    let rc = unsafe {
        uio_get_file_location(
            stack.root,
            name.as_ptr(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0);
}

#[test]
#[serial]
fn file_location_outside_every_mount_is_enoent() {
    let stack = Stack::new();
    assert!(locate(&stack, "nowhere.txt").is_none());
    assert_eq!(errno_value(), libc::ENOENT);
    let name = cstr("x");
    let rc = unsafe {
        uio_get_file_location(
            ptr::null_mut(),
            name.as_ptr(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    assert_eq!(rc, -1);
}

#[test]
#[serial]
fn mounts_report_their_file_system_type() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let top = stack.first_mount_id();
    stack.mount_zip(&[("b", b"b")], UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY, top);
    unsafe {
        let stdio = mount_ptr(&stack, 0);
        let zip = mount_ptr(&stack, 1);
        assert_eq!(uio_get_mount_file_system_type(stdio), UIO_FSTYPE_STDIO);
        assert_eq!(uio_get_mount_file_system_type(zip), UIO_FSTYPE_ZIP);
        assert_eq!(uio_get_mount_file_system_type(ptr::null_mut()), 0);
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
    assert_eq!(get_mount_registry().lock().unwrap().len(), 1);
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
fn unmount_all_clears_only_the_given_repository() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let other_repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
    let other_host = tempfile::TempDir::new().unwrap();
    let other = register_mount(
        other_repo,
        Path::new("/other"),
        other_host.path().to_path_buf(),
        UIO_FSTYPE_STDIO,
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        ptr::null_mut(),
        true,
    );
    assert!(!other.is_null());

    assert_eq!(unsafe { uio_unmount_all_dirs(stack.repo) }, 0);

    let registry = get_mount_registry().lock().unwrap();
    assert_eq!(registry.len(), 1);
    assert_eq!(
        registry[0].mount_point,
        normalize_mount_point(Path::new("/other"))
    );
    drop(registry);
    unsafe {
        uio_unmountDir(other);
        let _ = Box::from_raw(other_repo);
    }
}

#[test]
#[serial]
fn transplanting_a_directory_requires_a_read_only_mount() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("a.txt", b"a")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let source = Box::into_raw(Box::new(uio_DirHandle {
        virtual_path: PathBuf::from("/"),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: stack.repo,
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
    stack.mount_dir(
        &[("a.txt", b"transplanted")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
    );
    let source = Box::into_raw(Box::new(uio_DirHandle {
        virtual_path: PathBuf::from("/"),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: stack.repo,
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
fn handle_calls_read_write_and_stat_the_underlying_file() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("data.bin", b"0123456789")], UIO_MOUNT_TOP);
    let name = cstr("data.bin");
    unsafe {
        let handle = uio_open(stack.root, name.as_ptr(), 0, 0);
        assert!(!handle.is_null());
        let mut buf = [0u8; 4];
        assert_eq!(uio_read(handle, buf.as_mut_ptr(), 4), 4);
        assert_eq!(&buf, b"0123");
        assert_eq!(uio_read(handle, buf.as_mut_ptr(), 4), 4);
        assert_eq!(&buf, b"4567");
        assert_eq!(
            uio_read(handle, buf.as_mut_ptr(), 4),
            2,
            "short read at the end"
        );
        assert_eq!(uio_read(handle, buf.as_mut_ptr(), 4), 0);

        let mut info: stat = std::mem::zeroed();
        assert_eq!(uio_fstat(handle, &mut info), 0);
        assert_eq!(info.st_size, 10);
        assert_eq!(info.st_mode & 0o170000, 0o100000, "regular file");
        assert_eq!(uio_close(handle), 0);
    }
}

#[test]
#[serial]
fn handle_calls_refuse_null_and_empty_requests() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("data.bin", b"abc")], UIO_MOUNT_TOP);
    let name = cstr("data.bin");
    unsafe {
        let handle = uio_open(stack.root, name.as_ptr(), 0, 0);
        let mut buf = [0u8; 4];
        let mut info: stat = std::mem::zeroed();
        assert_eq!(uio_read(ptr::null_mut(), buf.as_mut_ptr(), 4), -1);
        assert_eq!(uio_read(handle, ptr::null_mut(), 4), -1);
        assert_eq!(uio_read(handle, buf.as_mut_ptr(), 0), -1);
        assert_eq!(uio_write(ptr::null_mut(), buf.as_ptr(), 4), -1);
        assert_eq!(uio_write(handle, ptr::null(), 4), -1);
        assert_eq!(uio_write(handle, buf.as_ptr(), 0), -1);
        assert_eq!(uio_fstat(ptr::null_mut(), &mut info), -1);
        assert_eq!(uio_fstat(handle, ptr::null_mut()), -1);
        assert_eq!(uio_close(handle), 0);
        assert_eq!(uio_close(ptr::null_mut()), 0);
    }
}

#[test]
#[serial]
fn writing_through_a_handle_changes_the_file_on_disk() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[], UIO_MOUNT_TOP);
    let name = cstr("new.bin");
    unsafe {
        let handle = uio_open(stack.root, name.as_ptr(), O_WRONLY | O_CREAT, 0o644);
        assert!(!handle.is_null());
        let payload = b"payload";
        assert_eq!(uio_write(handle, payload.as_ptr(), payload.len()), 7);
        assert_eq!(uio_close(handle), 0);
    }
    assert_eq!(std::fs::read(host.join("new.bin")).unwrap(), b"payload");
}

#[test]
#[serial]
fn unlink_removes_a_file_from_a_writable_mount() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("gone.txt", b"x"), ("kept.txt", b"y")], UIO_MOUNT_TOP);
    let name = cstr("gone.txt");
    assert_eq!(unsafe { uio_unlink(stack.root, name.as_ptr()) }, 0);
    assert!(!host.join("gone.txt").exists());
    assert!(host.join("kept.txt").exists());
    assert_eq!(stack.read("gone.txt"), None);
}

#[test]
#[serial]
fn unlink_refuses_read_only_missing_and_malformed_requests() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("locked.txt", b"x")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let locked = cstr("locked.txt");
    let missing = cstr("missing.txt");
    unsafe {
        assert_eq!(uio_unlink(stack.root, locked.as_ptr()), -1);
        assert_eq!(errno_value(), libc::EROFS);
        assert!(host.join("locked.txt").exists());
        assert_eq!(uio_unlink(stack.root, missing.as_ptr()), -1);
        assert_eq!(uio_unlink(ptr::null_mut(), locked.as_ptr()), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_unlink(stack.root, ptr::null()), -1);
        assert_eq!(errno_value(), libc::EINVAL);
    }
}
