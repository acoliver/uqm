//! Where a write goes in a stack of mounts, and what is copied up for it.
//!
//! The rules are `uio_getPhysicalAccess`, `uio_open`, `uio_mkdir`,
//! `uio_unlink`, `uio_rmdir`, `uio_rename` and `uio_getFileLocation` of the
//! original UIO, read from the legacy sources:
//!
//! * the covering mounts are walked in priority order; a directory at the full
//!   path ends the walk with EISDIR (EEXIST for mkdir); a file there is the
//!   entry; a mount that lacks it but holds its parent directory is a place
//!   the entry could be made; a path through a file ends the walk with ENOTDIR;
//! * the write location is the FIRST WRITABLE mount the walk reached, not the
//!   topmost mount: a read-only archive on top does not stop a write that a
//!   writable mount below it can take;
//! * the walk stops at the first mount that holds the entry, so a read-only
//!   mount holding the entry above the writable one means EPERM, and a
//!   writable mount holding it first means the file is updated where it is;
//! * an entry that lives in another mount than the write location is copied up
//!   (byte for byte, after the missing parent directories are made) unless
//!   the open truncates it; O_TRUNC then drops the flag, so without O_CREAT
//!   the open of the not-yet-copied file is ENOENT;
//! * EEXIST for O_CREAT|O_EXCL if the entry exists anywhere the walk reached,
//!   ENOENT for no O_CREAT on a missing entry or a parent no mount has, EPERM
//!   when no mount the walk reached is writable;
//! * unlink and rmdir visit EVERY mount that holds the parent directory, in
//!   order, and delete the entry in each: a directory for unlink is EISDIR, a
//!   file for rmdir ENOTDIR, a read-only mount holding the entry EROFS (after
//!   the earlier writable mounts were already cleared), nothing found ENOENT;
//! * rename reads the old path with read access and asks for a write location
//!   for the new path without making directories; different mounts are EXDEV.
//!
//! Fixtures are real temporary directories and real ZIP files mounted through
//! the public UIO functions. Each test names the mount that may change and
//! compares complete tree snapshots of every host, so "nothing else was
//! touched" and "no host fallback" are assertions, not hopes. Hosts and the
//! mount registry are torn down when the `Fx` drops.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::io::{Read, Seek, SeekFrom, Write};

use serial_test::serial;

use super::resolution_tests::{
    cstr, errno_value, mount_stdio, mount_zip, open_dir, read, write_zip, Fx, RDONLY_TOP,
};
use super::*;

type Tree = BTreeMap<String, Option<Vec<u8>>>;

/// Every directory (`None`) and file (its bytes) below `root`.
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

fn files(entries: &[(&str, &[u8])], dirs: &[&str]) -> Tree {
    let mut out: Tree = entries
        .iter()
        .map(|(name, data)| ((*name).to_owned(), Some(data.to_vec())))
        .collect();
    for dir in dirs {
        out.insert((*dir).to_owned(), None);
    }
    out
}

/// A repository with a writable host and a read-only archive; `zip_on_top`
/// picks which of the two is looked at first.
struct Stack {
    root: *mut uio_DirHandle,
    rw: PathBuf,
    zips: PathBuf,
    rw_mount: *mut uio_MountHandle,
}

fn stack(
    fx: &mut Fx,
    rw_files: &[(&str, &[u8])],
    zip_entries: &[(&str, &[u8])],
    zip_on_top: bool,
) -> Stack {
    let repo = fx.repo();
    let rw = fx.host(rw_files);
    let zips = fx.host(&[]);
    write_zip(&zips.join("pack.zip"), zip_entries);
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    let src = open_dir(repo, "/src");
    let mount_archive = || {
        assert!(!mount_zip(repo, "/", src, "pack.zip", RDONLY_TOP).is_null());
    };
    if !zip_on_top {
        mount_archive();
    }
    let rw_mount = mount_stdio(repo, "/", &rw, UIO_MOUNT_TOP);
    assert!(!rw_mount.is_null());
    if zip_on_top {
        mount_archive();
    }
    Stack {
        root: open_dir(repo, "/"),
        rw,
        zips,
        rw_mount,
    }
}

fn open_with(dir: *mut uio_DirHandle, name: &str, flags: c_int) -> Result<*mut uio_Handle, c_int> {
    let name = cstr(name);
    let handle = unsafe { uio_open(dir, name.as_ptr(), flags, 0o644) };
    if handle.is_null() {
        Err(errno_value())
    } else {
        Ok(handle)
    }
}

/// Opens with `flags`, writes `data` at the current position, closes.
fn write_with(dir: *mut uio_DirHandle, name: &str, flags: c_int, data: &[u8]) -> Result<(), c_int> {
    let handle = open_with(dir, name, flags)?;
    let written = unsafe { (*handle).lock().unwrap().write_all(data) };
    unsafe { uio_close(handle) };
    written.map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))
}

fn mkdir(dir: *mut uio_DirHandle, name: &str) -> Result<(), c_int> {
    let name = cstr(name);
    match unsafe { uio_mkdir(dir, name.as_ptr(), 0o755) } {
        0 => Ok(()),
        _ => Err(errno_value()),
    }
}

fn rmdir(dir: *mut uio_DirHandle, name: &str) -> Result<(), c_int> {
    let name = cstr(name);
    match unsafe { uio_rmdir(dir, name.as_ptr()) } {
        0 => Ok(()),
        _ => Err(errno_value()),
    }
}

fn unlink(dir: *mut uio_DirHandle, name: &str) -> Result<(), c_int> {
    let name = cstr(name);
    match unsafe { uio_unlink(dir, name.as_ptr()) } {
        0 => Ok(()),
        _ => Err(errno_value()),
    }
}

fn rename(dir: *mut uio_DirHandle, from: &str, to: &str) -> Result<(), c_int> {
    let (from, to) = (cstr(from), cstr(to));
    match unsafe { uio_rename(dir, from.as_ptr(), dir, to.as_ptr()) } {
        0 => Ok(()),
        _ => Err(errno_value()),
    }
}

fn location(
    dir: *mut uio_DirHandle,
    name: &str,
    flags: c_int,
) -> Result<(*mut uio_MountHandle, PathBuf), c_int> {
    let name = cstr(name);
    let mut mount: *mut uio_MountHandle = ptr::null_mut();
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { uio_get_file_location(dir, name.as_ptr(), flags, &mut mount, &mut out) };
    if rc != 0 {
        let errno = errno_value();
        assert!(mount.is_null() && out.is_null(), "failure clears outputs");
        return Err(errno);
    }
    let path = PathBuf::from(std::ffi::OsStr::from_bytes(
        unsafe { CStr::from_ptr(out) }.to_bytes(),
    ));
    unsafe { libc::free(out.cast()) };
    Ok((mount, path))
}

fn same_place(actual: &Path, expected: &Path) -> bool {
    let parent = |p: &Path| std::fs::canonicalize(p.parent().unwrap());
    matches!((parent(actual), parent(expected)), (Ok(a), Ok(b)) if a == b)
        && actual.file_name() == expected.file_name()
}

const CREATE: c_int = O_WRONLY | O_CREAT;
const CREATE_NEW: c_int = O_WRONLY | O_CREAT | O_EXCL;

// ---------------------------------------------------------------------------
// W1: an archive on top of a writable host. Hosts:
//   rw   = { only_rw.txt "RW-ONLY", shared.txt "RW-SHARED", saves/slot.sav "RW-SLOT", empty/ }
//   zip  = { only_zip.txt "ZIP-ONLY", shared.txt "ZIP-SHARED", zdir/inner.txt "Z" }
// ---------------------------------------------------------------------------

fn zip_over_rw(fx: &mut Fx) -> Stack {
    let s = stack(
        fx,
        &[
            ("only_rw.txt", b"RW-ONLY"),
            ("shared.txt", b"RW-SHARED"),
            ("saves/slot.sav", b"RW-SLOT"),
        ],
        &[
            ("only_zip.txt", b"ZIP-ONLY"),
            ("shared.txt", b"ZIP-SHARED"),
            ("zdir/inner.txt", b"Z"),
        ],
        true,
    );
    std::fs::create_dir(s.rw.join("empty")).unwrap();
    s
}

fn rw_start() -> Tree {
    files(
        &[
            ("only_rw.txt", b"RW-ONLY"),
            ("shared.txt", b"RW-SHARED"),
            ("saves/slot.sav", b"RW-SLOT"),
        ],
        &["saves", "empty"],
    )
}

#[test]
#[serial]
fn a_new_file_goes_to_the_writable_mount_below_a_read_only_archive() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    write_with(s.root, "new.txt", CREATE_NEW, b"NEW").unwrap();

    let mut expected = rw_start();
    expected.extend(files(&[("new.txt", b"NEW")], &[]));
    assert_eq!(tree(&s.rw), expected, "only the writable host changed");
    assert_eq!(tree(&s.zips), zips_before, "the archive file is untouched");
    assert_eq!(read(s.root, "new.txt").unwrap(), b"NEW");
    let (mount, path) = location(s.root, "new.txt", O_RDONLY).unwrap();
    assert_eq!(mount, s.rw_mount);
    assert!(same_place(&path, &s.rw.join("new.txt")), "{path:?}");
}

#[test]
#[serial]
fn fopen_for_writing_and_appending_uses_the_same_write_location() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let (name, w, a) = (cstr("made.txt"), cstr("w"), cstr("a"));
    unsafe {
        let stream = uio_fopen(s.root, name.as_ptr(), w.as_ptr());
        assert!(!stream.is_null(), "errno {}", errno_value());
        assert_eq!(uio_fputc(i32::from(b'A'), stream), i32::from(b'A'));
        uio_fclose(stream);
        let stream = uio_fopen(s.root, name.as_ptr(), a.as_ptr());
        assert!(!stream.is_null(), "errno {}", errno_value());
        assert_eq!(uio_fputc(i32::from(b'B'), stream), i32::from(b'B'));
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(s.rw.join("made.txt")).unwrap(), b"AB");
}

#[test]
#[serial]
fn a_file_only_the_writable_mount_holds_is_appended_truncated_and_updated_in_place() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);

    write_with(s.root, "only_rw.txt", O_WRONLY | O_APPEND, b"+A").unwrap();
    assert_eq!(
        std::fs::read(s.rw.join("only_rw.txt")).unwrap(),
        b"RW-ONLY+A"
    );

    let handle = open_with(s.root, "only_rw.txt", O_RDWR).unwrap();
    unsafe {
        let mut guard = (*handle).lock().unwrap();
        guard.seek(SeekFrom::Start(3)).unwrap();
        guard.write_all(b"x").unwrap();
        let mut head = [0u8; 4];
        guard.seek(SeekFrom::Start(0)).unwrap();
        guard.read_exact(&mut head).unwrap();
        assert_eq!(&head, b"RW-x");
    }
    unsafe { uio_close(handle) };
    assert_eq!(
        std::fs::read(s.rw.join("only_rw.txt")).unwrap(),
        b"RW-xNLY+A"
    );

    write_with(s.root, "only_rw.txt", O_WRONLY | O_TRUNC, b"T").unwrap();
    assert_eq!(std::fs::read(s.rw.join("only_rw.txt")).unwrap(), b"T");

    let mut expected = rw_start();
    expected.insert("only_rw.txt".into(), Some(b"T".to_vec()));
    assert_eq!(tree(&s.rw), expected);
    let (mount, path) = location(s.root, "only_rw.txt", O_WRONLY).unwrap();
    assert_eq!(mount, s.rw_mount, "the file stays where it is");
    assert!(same_place(&path, &s.rw.join("only_rw.txt")));
    assert_eq!(std::fs::read(&path).unwrap(), b"T", "no copy was made");
}

#[test]
#[serial]
fn create_new_refuses_an_entry_either_mount_holds_and_changes_nothing() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    for name in ["only_rw.txt", "only_zip.txt", "shared.txt"] {
        assert_eq!(
            write_with(s.root, name, CREATE_NEW, b"X"),
            Err(libc::EEXIST),
            "{name}"
        );
    }
    assert_eq!(tree(&s.rw), rw_start());
    assert_eq!(tree(&s.zips), zips_before);
}

#[test]
#[serial]
fn an_entry_the_archive_holds_above_the_writable_mount_cannot_be_written() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    for (name, flags) in [
        ("only_zip.txt", O_WRONLY),
        ("only_zip.txt", O_WRONLY | O_APPEND),
        ("only_zip.txt", CREATE | O_TRUNC),
        ("shared.txt", O_RDWR),
        ("shared.txt", CREATE | O_TRUNC),
    ] {
        assert_eq!(
            write_with(s.root, name, flags, b"X"),
            Err(libc::EPERM),
            "{name} {flags:#x}"
        );
    }
    assert_eq!(read(s.root, "shared.txt").unwrap(), b"ZIP-SHARED");
    assert_eq!(tree(&s.rw), rw_start(), "the shadowed copy was not reached");
    assert_eq!(tree(&s.zips), zips_before);
}

#[test]
#[serial]
fn a_path_whose_parent_no_mount_has_is_enoent_and_makes_nothing() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);

    assert_eq!(
        write_with(s.root, "nodir/x.txt", CREATE, b"X"),
        Err(libc::ENOENT)
    );
    assert_eq!(
        write_with(s.root, "a/b/c.txt", CREATE, b"X"),
        Err(libc::ENOENT)
    );
    assert_eq!(mkdir(s.root, "nodir/sub"), Err(libc::ENOENT));
    assert_eq!(tree(&s.rw), rw_start());
    for fallback in ["nodir", "a", "/nodir", "/a"] {
        assert!(
            !Path::new(fallback).exists(),
            "no host fallback at {fallback}"
        );
    }
}

#[test]
#[serial]
fn a_parent_only_the_archive_has_is_made_in_the_writable_mount_for_a_new_file() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    write_with(s.root, "zdir/new.txt", CREATE_NEW, b"N").unwrap();

    let mut expected = rw_start();
    expected.extend(files(&[("zdir/new.txt", b"N")], &["zdir"]));
    assert_eq!(tree(&s.rw), expected);
    assert_eq!(tree(&s.zips), zips_before);
    assert_eq!(read(s.root, "zdir/new.txt").unwrap(), b"N");
    assert_eq!(read(s.root, "zdir/inner.txt").unwrap(), b"Z");
    let (mount, path) = location(s.root, "zdir/new.txt", O_RDONLY).unwrap();
    assert_eq!(mount, s.rw_mount);
    assert!(same_place(&path, &s.rw.join("zdir/new.txt")));
}

#[test]
#[serial]
fn a_parent_only_the_writable_mount_has_takes_the_new_file_in_that_directory() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);

    write_with(s.root, "saves/new.sav", CREATE_NEW, b"S").unwrap();

    let mut expected = rw_start();
    expected.extend(files(&[("saves/new.sav", b"S")], &[]));
    assert_eq!(tree(&s.rw), expected);
}

#[test]
#[serial]
fn mkdir_takes_the_first_writable_mount_and_reports_existing_names() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    mkdir(s.root, "made").unwrap();
    mkdir(s.root, "zdir/sub").unwrap();
    for existing in [
        "zdir",
        "saves",
        "empty",
        "only_zip.txt",
        "only_rw.txt",
        "made",
    ] {
        assert_eq!(mkdir(s.root, existing), Err(libc::EEXIST), "{existing}");
    }

    let mut expected = rw_start();
    expected.extend(files(&[], &["made", "zdir", "zdir/sub"]));
    assert_eq!(tree(&s.rw), expected);
    assert_eq!(tree(&s.zips), zips_before);
}

#[test]
#[serial]
fn unlink_clears_every_mount_and_refuses_a_read_only_one() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    unlink(s.root, "only_rw.txt").unwrap();
    unlink(s.root, "saves/slot.sav").unwrap();
    assert_eq!(unlink(s.root, "only_rw.txt"), Err(libc::ENOENT));
    assert_eq!(unlink(s.root, "missing.txt"), Err(libc::ENOENT));
    assert_eq!(unlink(s.root, "only_zip.txt"), Err(libc::EROFS));
    assert_eq!(unlink(s.root, "shared.txt"), Err(libc::EROFS));
    assert_eq!(unlink(s.root, "saves"), Err(libc::EISDIR));
    assert_eq!(unlink(s.root, "zdir"), Err(libc::EISDIR));
    assert_eq!(unlink(s.root, "saves/"), Err(libc::ENOENT));

    let expected = files(&[("shared.txt", b"RW-SHARED")], &["saves", "empty"]);
    assert_eq!(tree(&s.rw), expected, "shared.txt is behind the archive's");
    assert_eq!(tree(&s.zips), zips_before);
}

#[test]
#[serial]
fn unlink_removes_the_writable_copy_before_the_read_only_one_is_refused() {
    let mut fx = Fx::new();
    // The writable host is on top here, so it is visited and cleared first.
    let s = stack(
        &mut fx,
        &[("dup.txt", b"RW-DUP")],
        &[("dup.txt", b"ZIP-DUP")],
        false,
    );
    let zips_before = tree(&s.zips);

    assert_eq!(unlink(s.root, "dup.txt"), Err(libc::EROFS));

    assert_eq!(tree(&s.rw), files(&[], &[]), "the writable copy is gone");
    assert_eq!(tree(&s.zips), zips_before);
    assert_eq!(read(s.root, "dup.txt").unwrap(), b"ZIP-DUP");
}

#[test]
#[serial]
fn rmdir_removes_empty_writable_directories_and_names_the_reason_otherwise() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    rmdir(s.root, "empty").unwrap();
    assert_eq!(rmdir(s.root, "empty"), Err(libc::ENOENT));
    assert_eq!(rmdir(s.root, "saves"), Err(libc::ENOTEMPTY));
    assert_eq!(rmdir(s.root, "zdir"), Err(libc::EROFS));
    assert_eq!(rmdir(s.root, "only_zip.txt"), Err(libc::ENOTDIR));
    assert_eq!(rmdir(s.root, "only_rw.txt"), Err(libc::ENOTDIR));

    let mut expected = rw_start();
    expected.remove("empty");
    assert_eq!(tree(&s.rw), expected);
    assert_eq!(tree(&s.zips), zips_before);
}

#[test]
#[serial]
fn rename_stays_inside_the_mount_that_holds_the_old_file() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);
    let zips_before = tree(&s.zips);

    rename(s.root, "only_rw.txt", "renamed.txt").unwrap();
    assert_eq!(
        rename(s.root, "only_zip.txt", "moved.txt"),
        Err(libc::EXDEV)
    );
    assert_eq!(
        rename(s.root, "renamed.txt", "shared.txt"),
        Err(libc::EEXIST)
    );
    assert_eq!(rename(s.root, "missing.txt", "x.txt"), Err(libc::ENOENT));
    // zdir exists only in the archive: rename makes no directories.
    assert_eq!(
        rename(s.root, "renamed.txt", "zdir/m.txt"),
        Err(libc::ENOENT)
    );
    assert_eq!(rename(s.root, "saves", "saves2"), Err(libc::EISDIR));

    let mut expected = rw_start();
    expected.remove("only_rw.txt");
    expected.insert("renamed.txt".into(), Some(b"RW-ONLY".to_vec()));
    assert_eq!(tree(&s.rw), expected);
    assert_eq!(tree(&s.zips), zips_before);
}

#[test]
#[serial]
fn file_location_for_writing_names_the_write_mount_and_makes_the_parents() {
    let mut fx = Fx::new();
    let s = zip_over_rw(&mut fx);

    let (mount, path) = location(s.root, "zdir/loc.txt", CREATE).unwrap();
    assert_eq!(mount, s.rw_mount);
    assert!(same_place(&path, &s.rw.join("zdir/loc.txt")), "{path:?}");
    let mut expected = rw_start();
    expected.extend(files(&[], &["zdir"]));
    assert_eq!(tree(&s.rw), expected, "parents made, the file is not");

    assert_eq!(
        location(s.root, "only_zip.txt", O_WRONLY).unwrap_err(),
        libc::EPERM
    );
    assert_eq!(
        location(s.root, "nodir/x", CREATE).unwrap_err(),
        libc::ENOENT
    );
    assert_eq!(
        location(s.root, "only_rw.txt", CREATE_NEW).unwrap_err(),
        libc::EEXIST
    );
    assert_eq!(
        location(s.root, "saves", O_WRONLY).unwrap_err(),
        libc::EISDIR
    );
}

// ---------------------------------------------------------------------------
// W2: a writable host on top of an archive: the first writable mount is the
// top one, and a file only the archive holds is copied up before it changes.
//   rw  = { }
//   zip = { cfg.ini "ZIP-CFG", cfg2.ini "0123456789", cfg3.ini "OLD",
//           cfg4.ini "Z4", sub/deep.cfg "ZIP-DEEP" }
// ---------------------------------------------------------------------------

fn rw_over_zip(fx: &mut Fx, rw_files: &[(&str, &[u8])]) -> Stack {
    stack(
        fx,
        rw_files,
        &[
            ("cfg.ini", b"ZIP-CFG"),
            ("cfg2.ini", b"0123456789"),
            ("cfg3.ini", b"OLD"),
            ("cfg4.ini", b"Z4"),
            ("sub/deep.cfg", b"ZIP-DEEP"),
        ],
        false,
    )
}

#[test]
#[serial]
fn appending_to_an_archive_file_copies_it_up_first() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[]);
    let zips_before = tree(&s.zips);

    write_with(s.root, "cfg.ini", O_WRONLY | O_APPEND, b"+MORE").unwrap();

    assert_eq!(
        tree(&s.rw),
        files(&[("cfg.ini", b"ZIP-CFG+MORE")], &[]),
        "the original bytes are kept, the addition follows"
    );
    assert_eq!(tree(&s.zips), zips_before);
    assert_eq!(read(s.root, "cfg.ini").unwrap(), b"ZIP-CFG+MORE");
}

#[test]
#[serial]
fn updating_in_the_middle_of_an_archive_file_keeps_the_rest_of_its_bytes() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[]);

    let handle = open_with(s.root, "cfg2.ini", O_RDWR).unwrap();
    unsafe {
        let mut guard = (*handle).lock().unwrap();
        guard.seek(SeekFrom::Start(4)).unwrap();
        guard.write_all(b"X").unwrap();
    }
    unsafe { uio_close(handle) };

    assert_eq!(tree(&s.rw), files(&[("cfg2.ini", b"0123X56789")], &[]));
    assert_eq!(read(s.root, "cfg2.ini").unwrap(), b"0123X56789");
}

#[test]
#[serial]
fn a_copy_up_makes_the_archive_directory_it_lives_in() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[]);

    write_with(s.root, "sub/deep.cfg", O_WRONLY | O_APPEND, b"!").unwrap();

    assert_eq!(
        tree(&s.rw),
        files(&[("sub/deep.cfg", b"ZIP-DEEP!")], &["sub"])
    );
}

#[test]
#[serial]
fn truncating_an_archive_file_creates_an_empty_copy_without_reading_the_original() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[]);

    write_with(s.root, "cfg3.ini", CREATE | O_TRUNC, b"N").unwrap();
    assert_eq!(tree(&s.rw), files(&[("cfg3.ini", b"N")], &[]));

    // Without O_CREAT the original's O_TRUNC is dropped and nothing is copied,
    // so the writable mount has no file to open: ENOENT, and nothing is made.
    let before = tree(&s.rw);
    assert_eq!(
        write_with(s.root, "cfg4.ini", O_WRONLY | O_TRUNC, b"N"),
        Err(libc::ENOENT)
    );
    assert_eq!(tree(&s.rw), before);
}

#[test]
#[serial]
fn create_new_refuses_an_archive_file_that_would_be_copied() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[]);

    assert_eq!(
        write_with(s.root, "cfg.ini", CREATE_NEW, b"X"),
        Err(libc::EEXIST)
    );
    assert_eq!(tree(&s.rw), files(&[], &[]), "no empty file was created");
}

#[test]
#[serial]
fn a_file_the_top_writable_mount_already_has_is_not_copied_over() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[("cfg.ini", b"RW-CFG")]);

    write_with(s.root, "cfg.ini", O_WRONLY | O_APPEND, b"+A").unwrap();

    assert_eq!(tree(&s.rw), files(&[("cfg.ini", b"RW-CFG+A")], &[]));
    assert_eq!(read(s.root, "cfg2.ini").unwrap(), b"0123456789");
}

#[test]
#[serial]
fn file_location_for_writing_performs_the_copy_up_and_reports_the_copy() {
    let mut fx = Fx::new();
    let s = rw_over_zip(&mut fx, &[]);

    let (mount, path) = location(s.root, "cfg.ini", O_WRONLY).unwrap();
    assert_eq!(mount, s.rw_mount);
    assert!(same_place(&path, &s.rw.join("cfg.ini")), "{path:?}");
    assert_eq!(std::fs::read(&path).unwrap(), b"ZIP-CFG");

    let (mount, path) = location(s.root, "cfg3.ini", CREATE | O_TRUNC).unwrap();
    assert_eq!(mount, s.rw_mount);
    assert!(same_place(&path, &s.rw.join("cfg3.ini")));
    assert!(!path.exists(), "a truncating location copies nothing");
}

#[test]
#[serial]
fn a_writable_mount_below_another_receives_a_copy_of_the_file_it_holds() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let low = fx.host(&[("low.txt", b"LOW")]);
    let top = fx.host(&[]);
    assert!(!mount_stdio(repo, "/", &low, UIO_MOUNT_TOP).is_null());
    let top_mount = mount_stdio(repo, "/", &top, UIO_MOUNT_TOP);
    assert!(!top_mount.is_null());
    let root = open_dir(repo, "/");

    write_with(root, "low.txt", O_WRONLY | O_APPEND, b"+TOP").unwrap();

    assert_eq!(tree(&top), files(&[("low.txt", b"LOW+TOP")], &[]));
    assert_eq!(
        tree(&low),
        files(&[("low.txt", b"LOW")], &[]),
        "the lower host is untouched"
    );
    assert_eq!(location(root, "low.txt", O_RDONLY).unwrap().0, top_mount);
}

// ---------------------------------------------------------------------------
// W3: no writable mount, or no mount at all
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn with_no_writable_mount_every_change_is_eperm_and_nothing_is_created() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let locked = fx.host(&[("locked.txt", b"L"), ("dir/inner.txt", b"I")]);
    let zips = fx.host(&[]);
    write_zip(&zips.join("pack.zip"), &[("z.txt", b"Z")]);
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    assert!(!mount_stdio(repo, "/", &locked, RDONLY_TOP).is_null());
    let src = open_dir(repo, "/src");
    assert!(!mount_zip(repo, "/", src, "pack.zip", RDONLY_TOP).is_null());
    let root = open_dir(repo, "/");
    let (locked_before, zips_before) = (tree(&locked), tree(&zips));

    assert_eq!(write_with(root, "new.txt", CREATE, b"X"), Err(libc::EPERM));
    assert_eq!(
        write_with(root, "dir/new.txt", CREATE, b"X"),
        Err(libc::EPERM)
    );
    assert_eq!(
        write_with(root, "locked.txt", O_WRONLY, b"X"),
        Err(libc::EPERM)
    );
    assert_eq!(write_with(root, "z.txt", O_RDWR, b"X"), Err(libc::EPERM));
    assert_eq!(mkdir(root, "newdir"), Err(libc::EPERM));
    assert_eq!(location(root, "new.txt", CREATE).unwrap_err(), libc::EPERM);
    assert_eq!(unlink(root, "locked.txt"), Err(libc::EROFS));
    assert_eq!(rmdir(root, "dir"), Err(libc::EROFS));
    assert_eq!(rename(root, "locked.txt", "moved.txt"), Err(libc::EPERM));

    assert_eq!(tree(&locked), locked_before);
    assert_eq!(tree(&zips), zips_before);
    for fallback in ["new.txt", "newdir", "/new.txt", "/newdir"] {
        assert!(
            !Path::new(fallback).exists(),
            "no host fallback at {fallback}"
        );
    }
}

#[test]
#[serial]
fn a_repository_without_mounts_has_nothing_to_write_to() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let root = open_dir(repo, "/");
    assert!(!root.is_null());

    assert_eq!(write_with(root, "new.txt", CREATE, b"X"), Err(libc::ENOENT));
    assert_eq!(mkdir(root, "newdir"), Err(libc::ENOENT));
    assert_eq!(unlink(root, "new.txt"), Err(libc::ENOENT));
    assert_eq!(rmdir(root, "newdir"), Err(libc::ENOENT));
    for fallback in ["new.txt", "newdir", "/new.txt", "/newdir"] {
        assert!(
            !Path::new(fallback).exists(),
            "no host fallback at {fallback}"
        );
    }
}

// ---------------------------------------------------------------------------
// W4: repositories do not share write locations
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn a_write_through_one_repository_never_lands_in_another_ones_mount() {
    let mut fx = Fx::new();
    let first = zip_over_rw(&mut fx);
    let (second_repo, second_host) = (fx.repo(), fx.host(&[("keep.txt", b"K")]));
    assert!(!mount_stdio(second_repo, "/", &second_host, UIO_MOUNT_TOP).is_null());
    let second_before = tree(&second_host);

    write_with(first.root, "new.txt", CREATE_NEW, b"FIRST").unwrap();

    assert_eq!(tree(&second_host), second_before);
    assert_eq!(std::fs::read(first.rw.join("new.txt")).unwrap(), b"FIRST");
    let second_root = open_dir(second_repo, "/");
    assert_eq!(read(second_root, "new.txt"), Err(libc::ENOENT));
}
