//! `uio_getStdioAccess` with write flags and `uio_access` with `W_OK`.
//!
//! The original `uio_getStdioAccess` asks `uio_getFileLocation` for the
//! location with the flags it was given. A location on a directory mount is
//! the answer; only a location on an archive is unpacked into a generated
//! directory, and a write location is never an archive (`zip_open` refuses
//! anything but `O_RDONLY`, and the write walk only picks writable mounts).
//! So for write flags the access is the write location after the preparation
//! `uio_open` does: the parent directories are made and an existing entry is
//! copied up unless the flags truncate it. Nothing is unpacked, no temporary
//! directory is needed, and releasing the handle leaves the file.
//!
//! The original `uio_access` is a stub that answers ENOSYS. The port keeps a
//! working `W_OK`: it answers whether a write to the path would succeed, using
//! the same mount choice as `uio_open`, and changes nothing while doing so.
//!
//! Fixtures are real hosts and a real ZIP file mounted through the public
//! functions; every refusal compares full tree snapshots.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::unix::fs::PermissionsExt;

use serial_test::serial;

use super::resolution_tests::{
    cstr, errno_value, mount_stdio, mount_zip, open_dir, read, write_zip, Fx, RDONLY_TOP,
};
use super::*;

type Tree = BTreeMap<String, Option<Vec<u8>>>;

fn tree(root: &Path) -> Tree {
    fn walk(dir: &Path, root: &Path, out: &mut Tree) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                out.insert(name, None);
                walk(&path, root, out);
            } else {
                out.insert(name, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Tree::new();
    walk(root, root, &mut out);
    out
}

struct Stack {
    root: *mut uio_DirHandle,
    rw: PathBuf,
    scratch: PathBuf,
    scratch_dir: *mut uio_DirHandle,
    pack: PathBuf,
}

/// A writable host (`rw_files`) and an archive holding `data/save.bin`
/// (bytes `saved`) and `data/other.bin`; `zip_on_top` says which is looked at
/// first. A second writable host is mounted at `/scratch`.
fn stack(fx: &mut Fx, rw_files: &[(&str, &[u8])], zip_on_top: bool) -> Stack {
    let repo = fx.repo();
    let rw = fx.host(rw_files);
    let zips = fx.host(&[]);
    let scratch = fx.host(&[]);
    write_zip(
        &zips.join("pack.zip"),
        &[("data/save.bin", b"saved"), ("data/other.bin", b"other")],
    );
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    assert!(!mount_stdio(repo, "/scratch", &scratch, UIO_MOUNT_TOP).is_null());
    let src = open_dir(repo, "/src");
    let mount_archive = |flags: c_int| {
        assert!(!mount_zip(repo, "/", src, "pack.zip", flags | UIO_MOUNT_RDONLY).is_null());
    };
    if zip_on_top {
        assert!(!mount_stdio(repo, "/", &rw, UIO_MOUNT_BOTTOM).is_null());
        mount_archive(UIO_MOUNT_TOP);
    } else {
        mount_archive(UIO_MOUNT_BOTTOM);
        assert!(!mount_stdio(repo, "/", &rw, UIO_MOUNT_TOP).is_null());
    }
    Stack {
        root: open_dir(repo, "/"),
        rw,
        scratch,
        scratch_dir: open_dir(repo, "/scratch"),
        pack: zips.join("pack.zip"),
    }
}

fn stdio_access(
    stack: &Stack,
    path: &str,
    flags: c_int,
    temp: *mut uio_DirHandle,
) -> Result<*mut uio_StdioAccessHandle, c_int> {
    let path = cstr(path);
    let handle = unsafe { uio_getStdioAccess(stack.root, path.as_ptr(), flags, temp) };
    if handle.is_null() {
        Err(errno_value())
    } else {
        Ok(handle)
    }
}

fn path_of(handle: *mut uio_StdioAccessHandle) -> PathBuf {
    unsafe {
        PathBuf::from(
            CStr::from_ptr(uio_StdioAccessHandle_getPath(handle))
                .to_str()
                .unwrap(),
        )
    }
}

#[test]
#[serial]
fn write_access_to_an_archive_entry_is_the_copy_in_the_writable_mount() {
    let mut fx = Fx::new();
    let s = stack(&mut fx, &[], false);
    let archive_before = std::fs::read(&s.pack).unwrap();
    let scratch_before = tree(&s.scratch);

    let handle = stdio_access(&s, "data/save.bin", O_RDWR, ptr::null_mut()).unwrap();
    let path = path_of(handle);
    assert_eq!(path, s.rw.join("data/save.bin"));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"saved",
        "the entry was copied up"
    );

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut file, b"+more").unwrap();
    drop(file);
    unsafe { uio_releaseStdioAccess(handle) };

    assert!(path.exists(), "releasing a write access keeps the file");
    assert_eq!(read(s.root, "data/save.bin"), Ok(b"saved+more".to_vec()));
    assert_eq!(
        std::fs::read(&s.pack).unwrap(),
        archive_before,
        "the archive is untouched"
    );
    assert_eq!(
        tree(&s.scratch),
        scratch_before,
        "nothing was unpacked into the temp dir"
    );
}

#[test]
#[serial]
fn write_access_to_a_file_the_writable_mount_holds_does_not_touch_it() {
    let mut fx = Fx::new();
    let s = stack(&mut fx, &[("data/save.bin", b"mine")], false);
    let before = tree(&s.rw);
    let handle = stdio_access(&s, "data/save.bin", O_RDWR, s.scratch_dir).unwrap();
    assert_eq!(path_of(handle), s.rw.join("data/save.bin"));
    assert_eq!(std::fs::read(path_of(handle)).unwrap(), b"mine");
    unsafe { uio_releaseStdioAccess(handle) };
    assert_eq!(tree(&s.rw), before);
    assert_eq!(tree(&s.scratch), BTreeMap::new());
}

#[test]
#[serial]
fn a_truncating_or_creating_access_copies_nothing_and_makes_only_the_parents() {
    let mut fx = Fx::new();
    let s = stack(&mut fx, &[], false);

    let handle = stdio_access(&s, "data/new.txt", O_WRONLY | O_CREAT, ptr::null_mut()).unwrap();
    assert_eq!(path_of(handle), s.rw.join("data/new.txt"));
    assert!(!path_of(handle).exists(), "the entry itself is not created");
    assert!(
        s.rw.join("data").is_dir(),
        "the parent was made for the writer"
    );
    unsafe { uio_releaseStdioAccess(handle) };

    let handle = stdio_access(
        &s,
        "data/save.bin",
        O_WRONLY | O_CREAT | O_TRUNC,
        ptr::null_mut(),
    )
    .unwrap();
    assert_eq!(path_of(handle), s.rw.join("data/save.bin"));
    assert!(
        !path_of(handle).exists(),
        "a truncating open copies nothing"
    );
    unsafe { uio_releaseStdioAccess(handle) };
}

#[test]
#[serial]
fn write_access_refusals_use_the_write_location_errnos_and_change_nothing() {
    let mut fx = Fx::new();
    // The archive holds the entry above the writable mount: EPERM.
    let above = stack(&mut fx, &[("keep.txt", b"k")], true);
    let rw_before = tree(&above.rw);
    assert_eq!(
        stdio_access(&above, "data/save.bin", O_RDWR, above.scratch_dir).err(),
        Some(libc::EPERM)
    );
    assert_eq!(tree(&above.rw), rw_before);
    assert_eq!(tree(&above.scratch), BTreeMap::new());

    // Only a read-only mount.
    let mut fx = Fx::new();
    let repo = fx.repo();
    let locked = fx.host(&[("a.txt", b"a")]);
    assert!(!mount_stdio(repo, "/", &locked, RDONLY_TOP).is_null());
    let only = Stack {
        root: open_dir(repo, "/"),
        rw: locked.clone(),
        scratch: locked.clone(),
        scratch_dir: ptr::null_mut(),
        pack: locked.join("a.txt"),
    };
    let before = tree(&locked);
    assert_eq!(
        stdio_access(&only, "a.txt", O_RDWR, ptr::null_mut()).err(),
        Some(libc::EPERM)
    );
    assert_eq!(
        stdio_access(&only, "new.txt", O_WRONLY | O_CREAT, ptr::null_mut()).err(),
        Some(libc::EPERM)
    );
    assert_eq!(tree(&locked), before);

    let mut fx = Fx::new();
    let s = stack(&mut fx, &[("data/mine.txt", b"m")], false);
    assert_eq!(
        stdio_access(
            &s,
            "data/mine.txt",
            O_RDWR | O_CREAT | O_EXCL,
            ptr::null_mut()
        )
        .err(),
        Some(libc::EEXIST)
    );
    assert_eq!(
        stdio_access(
            &s,
            "data/save.bin",
            O_RDWR | O_CREAT | O_EXCL,
            ptr::null_mut()
        )
        .err(),
        Some(libc::EEXIST)
    );
    assert_eq!(
        stdio_access(&s, "data", O_RDWR, ptr::null_mut()).err(),
        Some(libc::EISDIR)
    );
    assert_eq!(
        stdio_access(&s, "data/none.bin", O_RDWR, ptr::null_mut()).err(),
        Some(libc::ENOENT)
    );
    assert_eq!(
        stdio_access(&s, "nodir/none.bin", O_RDWR | O_CREAT, ptr::null_mut()).err(),
        Some(libc::ENOENT),
        "no mount has the parent directory"
    );
}

fn access_of(stack: &Stack, path: &str, mode: c_int) -> Result<(), c_int> {
    let path = cstr(path);
    if unsafe { uio_access(stack.root, path.as_ptr(), mode) } == 0 {
        Ok(())
    } else {
        Err(errno_value())
    }
}

#[test]
#[serial]
fn write_permission_asks_where_a_write_would_land() {
    let mut fx = Fx::new();
    // The entry is in an archive below the writable mount: a write would copy it up.
    let below = stack(&mut fx, &[("data/mine.txt", b"m")], false);
    let rw_before = tree(&below.rw);
    assert_eq!(access_of(&below, "data/save.bin", W_OK), Ok(()));
    assert_eq!(access_of(&below, "data/save.bin", R_OK | W_OK), Ok(()));
    assert_eq!(access_of(&below, "data/mine.txt", W_OK), Ok(()));
    assert_eq!(
        access_of(&below, "data", W_OK),
        Ok(()),
        "the writable mount holds data"
    );
    assert_eq!(access_of(&below, "data/none.bin", W_OK), Err(libc::ENOENT));
    assert_eq!(tree(&below.rw), rw_before, "asking changed nothing");

    // The archive is looked at first: the entry cannot be written.
    let mut fx = Fx::new();
    let above = stack(&mut fx, &[("data/mine.txt", b"m")], true);
    assert_eq!(access_of(&above, "data/save.bin", W_OK), Err(libc::EACCES));
    assert_eq!(access_of(&above, "data/save.bin", R_OK), Ok(()));
    assert_eq!(access_of(&above, "data/mine.txt", W_OK), Ok(()));
}

#[test]
#[serial]
fn write_permission_is_the_hosts_answer_for_a_file_and_a_directory_it_holds() {
    let mut fx = Fx::new();
    let s = stack(
        &mut fx,
        &[
            ("locked.txt", b"x"),
            ("odd.txt", b"x"),
            ("sub/in.txt", b"i"),
        ],
        false,
    );
    let mode = |name: &str, bits: u32| {
        std::fs::set_permissions(s.rw.join(name), std::fs::Permissions::from_mode(bits)).unwrap()
    };
    mode("locked.txt", 0o444);
    // Owner may only read; group and others could write. The host says no to
    // the owner, which a test of "any write bit" would get wrong.
    mode("odd.txt", 0o466);
    assert_eq!(access_of(&s, "locked.txt", W_OK), Err(libc::EACCES));
    assert_eq!(access_of(&s, "odd.txt", W_OK), Err(libc::EACCES));
    mode("odd.txt", 0o644);
    assert_eq!(access_of(&s, "odd.txt", W_OK), Ok(()));

    assert_eq!(access_of(&s, "sub", W_OK), Ok(()));
    mode("sub", 0o555);
    let result = access_of(&s, "sub", W_OK);
    mode("sub", 0o755);
    assert_eq!(result, Err(libc::EACCES));
}

#[test]
#[serial]
fn write_permission_on_directories_no_writable_mount_holds_is_refused() {
    let mut fx = Fx::new();
    let s = stack(&mut fx, &[], true);
    // `data` exists only in the archive; `/deep` only in the mount tree.
    assert_eq!(access_of(&s, "data", W_OK), Err(libc::EACCES));
    let deep = fx.host(&[]);
    let tmp = fx.repo();
    assert!(!mount_stdio(tmp, "/deep/er", &deep, UIO_MOUNT_TOP).is_null());
    let virtual_only = Stack {
        root: open_dir(tmp, "/"),
        rw: deep.clone(),
        scratch: deep.clone(),
        scratch_dir: ptr::null_mut(),
        pack: deep,
    };
    assert_eq!(access_of(&virtual_only, "/deep", F_OK), Ok(()));
    assert_eq!(access_of(&virtual_only, "/deep", W_OK), Err(libc::EACCES));
}
