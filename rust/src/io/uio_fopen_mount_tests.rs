//! `uio_fopen` and `uio_open` have to see the same file on a mount stack,
//! because the stream is only a buffered view of the handle. These tests mix
//! writable and read-only directories with archives and check the file that
//! comes back, the file that gets written, and the order `uio_getDirList`
//! reports.

use std::ffi::{CStr, CString};
use std::io::Read;

use serial_test::serial;

use super::layered_open_tests::Stack;
use super::*;
use crate::io::name_filter::{MATCH_PREFIX, MATCH_REGEX, MATCH_SUBSTRING, MATCH_SUFFIX};

fn cstr(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn fopen(dir: *mut uio_DirHandle, name: &str, mode: &str) -> *mut uio_Stream {
    let (name, mode) = (cstr(name), cstr(mode));
    unsafe { uio_fopen(dir, name.as_ptr(), mode.as_ptr()) }
}

/// Everything the stream sees, or None with the errno of the refusal.
fn stream_contents(dir: *mut uio_DirHandle, name: &str) -> Result<Vec<u8>, c_int> {
    let stream = fopen(dir, name, "rb");
    if stream.is_null() {
        return Err(errno_value());
    }
    let mut bytes = Vec::new();
    unsafe {
        loop {
            let byte = uio_fgetc(stream);
            if byte < 0 {
                break;
            }
            bytes.push(byte as u8);
        }
        uio_fclose(stream);
    }
    Ok(bytes)
}

/// Everything the handle sees, or the errno of the refusal.
fn handle_contents(dir: *mut uio_DirHandle, name: &str) -> Result<Vec<u8>, c_int> {
    let name = cstr(name);
    unsafe {
        let handle = uio_open(dir, name.as_ptr(), 0, 0);
        if handle.is_null() {
            return Err(errno_value());
        }
        let mut bytes = Vec::new();
        (*handle).lock().unwrap().read_to_end(&mut bytes).unwrap();
        uio_close(handle);
        Ok(bytes)
    }
}

fn dir_at(stack: &Stack, virtual_path: &str) -> *mut uio_DirHandle {
    Box::into_raw(Box::new(uio_DirHandle {
        path: PathBuf::from(virtual_path),
        virtual_path: PathBuf::from(virtual_path),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: stack.repo,
        root_end: PathBuf::from("/"),
    }))
}

fn dir_names(stack: &Stack, path: &str, pattern: &str, match_type: c_int) -> Vec<String> {
    let (path, pattern) = (cstr(path), cstr(pattern));
    unsafe {
        let list = uio_getDirList(stack.root, path.as_ptr(), pattern.as_ptr(), match_type);
        assert!(!list.is_null());
        let names = (0..(*list).numNames as usize)
            .map(|i| {
                CStr::from_ptr(*(*list).names.add(i))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        uio_DirList_free(list);
        names
    }
}

#[test]
#[serial]
fn fopen_reads_from_the_mount_not_from_the_directory_handles_host_path() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("data.txt", b"mounted bytes")], UIO_MOUNT_TOP);

    assert_eq!(
        stream_contents(stack.root, "data.txt").as_deref(),
        Ok(&b"mounted bytes"[..]),
        "the root handle's host path is /, which holds no such file"
    );
}

#[test]
#[serial]
fn fopen_prefers_the_highest_priority_mount_like_open() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[
            ("shared.txt", b"from archive"),
            ("deep.txt", b"archive only"),
        ],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let archive = stack.first_mount_id();
    stack.mount_dir_above(
        &[("shared.txt", b"from directory"), ("top.txt", b"top only")],
        archive,
    );

    for name in ["shared.txt", "deep.txt", "top.txt", "absent.txt"] {
        assert_eq!(
            stream_contents(stack.root, name),
            handle_contents(stack.root, name),
            "{name}"
        );
    }
    assert_eq!(
        stream_contents(stack.root, "shared.txt").as_deref(),
        Ok(&b"from directory"[..])
    );
    assert_eq!(stream_contents(stack.root, "absent.txt"), Err(libc::ENOENT));
}

#[test]
#[serial]
fn fopen_falls_through_a_mount_that_lacks_the_file() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("upper.txt", b"u")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let top = stack.first_mount_id();
    stack.mount_zip(
        &[("lower.txt", b"l")],
        UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
        top,
    );

    assert_eq!(
        stream_contents(stack.root, "lower.txt").as_deref(),
        Ok(&b"l"[..])
    );
    assert_eq!(
        stream_contents(stack.root, "upper.txt").as_deref(),
        Ok(&b"u"[..])
    );
}

#[test]
#[serial]
fn fopen_resolves_relative_and_absolute_paths_in_the_virtual_tree() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("root.txt", b"at the root")], UIO_MOUNT_TOP);
    std::fs::create_dir(host.join("sub")).unwrap();
    std::fs::write(host.join("sub/leaf.txt"), b"leaf").unwrap();
    let sub = dir_at(&stack, "/sub");

    assert_eq!(
        stream_contents(sub, "leaf.txt").as_deref(),
        Ok(&b"leaf"[..])
    );
    assert_eq!(
        stream_contents(sub, "./leaf.txt").as_deref(),
        Ok(&b"leaf"[..])
    );
    assert_eq!(
        stream_contents(sub, "../root.txt").as_deref(),
        Ok(&b"at the root"[..])
    );
    assert_eq!(
        stream_contents(sub, "/root.txt").as_deref(),
        Ok(&b"at the root"[..])
    );
    assert_eq!(
        stream_contents(sub, "../../../root.txt").as_deref(),
        Ok(&b"at the root"[..]),
        "'..' stops at the virtual root"
    );
    assert_eq!(
        stream_contents(stack.root, "sub/leaf.txt").as_deref(),
        Ok(&b"leaf"[..])
    );
    unsafe { drop(Box::from_raw(sub)) };
}

#[test]
#[serial]
fn fopen_for_writing_creates_the_file_in_the_topmost_mount_only() {
    let mut stack = Stack::new();
    let lower = stack.mount_dir(&[], UIO_MOUNT_TOP);
    let upper = stack.mount_dir(&[], UIO_MOUNT_TOP);

    let stream = fopen(stack.root, "saved.txt", "w");
    assert!(!stream.is_null(), "errno {}", errno_value());
    let text = cstr("save data");
    unsafe {
        assert_eq!(uio_fputs(text.as_ptr(), stream), 0);
        uio_fclose(stream);
    }

    assert_eq!(
        std::fs::read(upper.join("saved.txt")).unwrap(),
        b"save data"
    );
    assert!(!lower.join("saved.txt").exists());
    assert_eq!(
        stream_contents(stack.root, "saved.txt").as_deref(),
        Ok(&b"save data"[..])
    );
}

#[test]
#[serial]
fn fopen_append_extends_the_file_in_the_mount() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("log.txt", b"one;")], UIO_MOUNT_TOP);

    let stream = fopen(stack.root, "log.txt", "a");
    assert!(!stream.is_null());
    let text = cstr("two;");
    unsafe {
        assert_eq!(uio_fputs(text.as_ptr(), stream), 0);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(host.join("log.txt")).unwrap(), b"one;two;");

    let stream = fopen(stack.root, "log.txt", "a+");
    assert!(!stream.is_null());
    unsafe {
        assert_eq!(uio_fputc(b'!' as c_int, stream), b'!' as c_int);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(host.join("log.txt")).unwrap(), b"one;two;!");
}

#[test]
#[serial]
fn fopen_read_write_updates_the_file_in_the_mount() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("rw.txt", b"0123456789")], UIO_MOUNT_TOP);

    let stream = fopen(stack.root, "rw.txt", "r+b");
    assert!(!stream.is_null());
    unsafe {
        assert_eq!(uio_fseek(stream, 4, SEEK_SET), 0);
        assert_eq!(uio_fputc(b'X' as c_int, stream), b'X' as c_int);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(host.join("rw.txt")).unwrap(), b"0123X56789");
}

#[test]
#[serial]
fn fopen_for_writing_refuses_a_read_only_mount_and_an_archive() {
    let mut stack = Stack::new();
    let locked = stack.mount_dir(&[("fixed.txt", b"keep")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);

    for mode in ["w", "a", "r+", "w+"] {
        assert!(fopen(stack.root, "fixed.txt", mode).is_null(), "{mode}");
        assert_eq!(errno_value(), libc::EACCES, "{mode}");
    }
    assert!(fopen(stack.root, "fresh.txt", "w").is_null());
    assert_eq!(errno_value(), libc::EACCES);
    assert!(!locked.join("fresh.txt").exists());
    assert_eq!(std::fs::read(locked.join("fixed.txt")).unwrap(), b"keep");
    assert_eq!(
        stream_contents(stack.root, "fixed.txt").as_deref(),
        Ok(&b"keep"[..])
    );

    let mut stack = Stack::new();
    stack.mount_zip(&[("packed.txt", b"zip")], UIO_MOUNT_TOP, 0);
    assert!(fopen(stack.root, "packed.txt", "w").is_null());
    assert_eq!(errno_value(), libc::EACCES, "an archive is never writable");
    assert_eq!(
        stream_contents(stack.root, "packed.txt").as_deref(),
        Ok(&b"zip"[..])
    );
}

#[test]
#[serial]
fn open_with_the_platform_flags_creates_truncates_and_appends() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("old.txt", b"previous contents")], UIO_MOUNT_TOP);
    let write = |name: &str, flags: c_int, data: &[u8]| unsafe {
        let name = cstr(name);
        let handle = uio_open(stack.root, name.as_ptr(), flags, 0o644);
        assert!(!handle.is_null(), "errno {}", errno_value());
        assert_eq!(
            uio_write(handle, data.as_ptr(), data.len()),
            data.len() as isize
        );
        uio_close(handle);
    };

    write("new.txt", libc::O_WRONLY | libc::O_CREAT, b"fresh");
    assert_eq!(std::fs::read(host.join("new.txt")).unwrap(), b"fresh");

    write(
        "old.txt",
        libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
        b"short",
    );
    assert_eq!(std::fs::read(host.join("old.txt")).unwrap(), b"short");

    write("old.txt", libc::O_WRONLY | libc::O_APPEND, b"+tail");
    assert_eq!(std::fs::read(host.join("old.txt")).unwrap(), b"short+tail");

    let name = cstr("old.txt");
    unsafe {
        let refused = uio_open(
            stack.root,
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            0o644,
        );
        assert!(refused.is_null());
        assert_eq!(errno_value(), libc::EEXIST);
    }
}

#[test]
#[serial]
fn fopen_mode_strings_follow_the_original_parser() {
    assert_eq!(open_flags_for_mode(b"r"), Some(libc::O_RDONLY));
    assert_eq!(open_flags_for_mode(b"rb"), Some(libc::O_RDONLY));
    assert_eq!(open_flags_for_mode(b"r+"), Some(libc::O_RDWR));
    assert_eq!(open_flags_for_mode(b"rb+"), Some(libc::O_RDWR));
    assert_eq!(open_flags_for_mode(b"r+b"), Some(libc::O_RDWR));
    assert_eq!(
        open_flags_for_mode(b"w"),
        Some(libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC)
    );
    assert_eq!(
        open_flags_for_mode(b"wt+"),
        Some(libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC)
    );
    assert_eq!(
        open_flags_for_mode(b"a"),
        Some(libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND)
    );
    assert_eq!(
        open_flags_for_mode(b"a+"),
        Some(libc::O_RDWR | libc::O_CREAT | libc::O_APPEND)
    );
    assert_eq!(
        open_flags_for_mode(b"rzz+"),
        Some(libc::O_RDONLY),
        "the rest is ignored"
    );
    assert_eq!(
        open_flags_for_mode(b"rbb+"),
        Some(libc::O_RDONLY),
        "only two modifiers count"
    );
    assert_eq!(open_flags_for_mode(b""), None);
    assert_eq!(open_flags_for_mode(b"x"), None);
    assert_eq!(open_flags_for_mode(b"br"), None);
}

#[test]
#[serial]
fn a_stream_remembers_how_it_was_opened() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("f.txt", b"x")], UIO_MOUNT_TOP);
    for (mode, expected) in [
        ("r", libc::O_RDONLY),
        ("r+", libc::O_RDWR),
        ("w", libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC),
        ("a", libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND),
    ] {
        let stream = fopen(stack.root, "f.txt", mode);
        assert!(!stream.is_null(), "{mode}");
        unsafe {
            assert_eq!((*stream).open_flags, expected, "{mode}");
            uio_fclose(stream);
        }
    }
}

#[test]
#[serial]
fn dir_lists_come_back_in_byte_order_across_mounts() {
    let mut stack = Stack::new();
    stack.mount_dir(
        &[
            ("zeta", b""),
            ("alpha", b""),
            ("Beta", b""),
            ("é.txt", b""),
            ("a.b", b""),
            ("a0", b""),
        ],
        UIO_MOUNT_TOP,
    );
    let top = stack.first_mount_id();
    stack.mount_zip(
        &[("mid", b""), ("alpha", b""), ("ÿ", b"")],
        UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
        top,
    );

    assert_eq!(
        dir_names(&stack, "", "", MATCH_PREFIX),
        ["Beta", "a.b", "a0", "alpha", "mid", "zeta", "é.txt", "ÿ"],
        "uppercase before lowercase, '.' before digits before letters, UTF-8 by its bytes, one 'alpha'"
    );
}

#[test]
#[serial]
fn dir_lists_filter_by_pattern_before_sorting() {
    let mut stack = Stack::new();
    stack.mount_dir(
        &[
            ("b.zip", b""),
            ("a.UQM", b""),
            ("c.txt", b""),
            ("A.zip", b""),
        ],
        UIO_MOUNT_TOP,
    );

    assert_eq!(
        dir_names(&stack, "", "\\.([zZ][iI][pP]|[uU][qQ][mM])$", MATCH_REGEX),
        ["A.zip", "a.UQM", "b.zip"]
    );
    assert_eq!(dir_names(&stack, "", ".txt", MATCH_SUFFIX), ["c.txt"]);
    assert_eq!(
        dir_names(&stack, "", "nothing", MATCH_SUBSTRING),
        Vec::<String>::new()
    );
}

#[test]
#[serial]
fn a_name_held_by_a_directory_above_and_a_file_below_is_listed_once() {
    let mut stack = Stack::new();
    let upper = stack.mount_dir(&[("other", b"")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    std::fs::create_dir(upper.join("same")).unwrap();
    std::fs::write(upper.join("same/inner.txt"), b"inner").unwrap();
    let top = stack.first_mount_id();
    stack.mount_zip(
        &[("same", b"a file"), ("zip-only", b"z")],
        UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
        top,
    );

    assert_eq!(
        dir_names(&stack, "", "", MATCH_PREFIX),
        ["other", "same", "zip-only"]
    );
    assert_eq!(
        dir_names(&stack, "same", "", MATCH_PREFIX),
        ["inner.txt"],
        "listing the name lists the directory above"
    );
    assert_eq!(
        stream_contents(stack.root, "same/inner.txt").as_deref(),
        Ok(&b"inner"[..])
    );
}

#[test]
#[serial]
fn stat_reports_why_it_failed_instead_of_not_found() {
    use std::os::unix::fs::PermissionsExt;

    let mut stack = Stack::new();
    let host = stack.mount_dir(&[("plain.txt", b"x")], UIO_MOUNT_TOP);
    let sealed = host.join("sealed");
    std::fs::create_dir(&sealed).unwrap();
    std::fs::write(sealed.join("inside.txt"), b"hidden").unwrap();
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o000)).unwrap();

    let stat_of = |name: &str| unsafe {
        let name = cstr(name);
        let mut info: stat = std::mem::zeroed();
        let rc = uio_stat(stack.root, name.as_ptr(), &mut info);
        (rc, errno_value())
    };
    let sealed_result = stat_of("sealed/inside.txt");
    let beyond_a_file = stat_of("plain.txt/child");
    let missing = stat_of("missing.txt");
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(sealed_result, (-1, libc::EACCES));
    assert_eq!(beyond_a_file, (-1, libc::ENOTDIR));
    assert_eq!(missing, (-1, libc::ENOENT));
}
