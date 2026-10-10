//! Directories that exist only because of where things are mounted, and
//! directories a host refuses to read.
//!
//! The mount tree of the original knows a directory `/deep` when something is
//! mounted at `/deep/er`, whether or not any mount holds a `deep` of its own.
//! `uio_openDir`, `uio_stat`, `uio_access` and `uio_getDirList` have to agree
//! on that. The rules, from `uio_verifyPath`, `uio_stat`/`uio_statDir` and
//! `uio_getDirList`/`uio_getPathPhysicalDirs`:
//!
//! * a physical entry that a mount holds wins over the mount tree's
//!   directory: `stat` of a name held by a file is that file;
//! * a path through a file is ENOTDIR for `stat`, and ends the walk of
//!   `uio_getDirList` with an empty list, like the original;
//! * names compare by component, not by prefix (`/de` is not `/deep`);
//! * listings are in byte order and list every name once;
//! * a layer that holds the directory but cannot be read is an error the
//!   caller sees (NULL and the host's errno), not a layer that is left out.
//!
//! Fixtures are real temporary directories and ZIP files mounted through the
//! public UIO functions; the unreadable directory is a genuine `chmod 000`
//! that a drop guard restores before the temporary directory is removed.

use std::ffi::CStr;
use std::os::unix::fs::PermissionsExt;

use serial_test::serial;

use super::resolution_tests::{
    cstr, errno_value, mount_stdio, mount_zip, open_dir, write_zip, Fx, RDONLY_TOP,
};
use super::*;
use crate::io::name_filter::MATCH_SUBSTRING;

const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;
const S_IFREG: u32 = 0o100000;

fn names_of(
    dir: *mut uio_DirHandle,
    path: &str,
    pattern: &str,
    match_type: c_int,
) -> Result<Vec<Vec<u8>>, c_int> {
    let (path, pattern) = (cstr(path), cstr(pattern));
    unsafe {
        let list = uio_getDirList(dir, path.as_ptr(), pattern.as_ptr(), match_type);
        if list.is_null() {
            return Err(errno_value());
        }
        let names = (0..(*list).numNames as usize)
            .map(|i| CStr::from_ptr(*(*list).names.add(i)).to_bytes().to_vec())
            .collect();
        uio_DirList_free(list);
        Ok(names)
    }
}

fn list(dir: *mut uio_DirHandle, path: &str) -> Result<Vec<Vec<u8>>, c_int> {
    names_of(dir, path, "", MATCH_SUBSTRING)
}

fn expected(names: &[&str]) -> Result<Vec<Vec<u8>>, c_int> {
    Ok(names.iter().map(|name| name.as_bytes().to_vec()).collect())
}

/// The file type bits and size `uio_stat` reports, or its errno.
fn stat_of(dir: *mut uio_DirHandle, path: &str) -> Result<(u32, i64), c_int> {
    let path = cstr(path);
    unsafe {
        let mut info: stat = std::mem::zeroed();
        if uio_stat(dir, path.as_ptr(), &mut info) == 0 {
            Ok((info.st_mode as u32 & S_IFMT, info.st_size as i64))
        } else {
            Err(errno_value())
        }
    }
}

fn access_of(dir: *mut uio_DirHandle, path: &str, mode: c_int) -> Result<(), c_int> {
    let path = cstr(path);
    if unsafe { uio_access(dir, path.as_ptr(), mode) } == 0 {
        Ok(())
    } else {
        Err(errno_value())
    }
}

fn open_result(repo: *mut uio_Repository, path: &str) -> Result<(), c_int> {
    let handle = open_dir(repo, path);
    if handle.is_null() {
        Err(errno_value())
    } else {
        unsafe { uio_closeDir(handle) };
        Ok(())
    }
}

struct Stack {
    repo: *mut uio_Repository,
    root: *mut uio_DirHandle,
    deep_er: *mut uio_MountHandle,
    deep_other_third: *mut uio_MountHandle,
}

/// Fixture V of the report: a writable host at `/`, an archive below it, and
/// two hosts mounted deeper than any directory that exists in them.
fn stack(fx: &mut Fx) -> Stack {
    let repo = fx.repo();
    let h0 = fx.host(&[
        ("Zeta", b"z"),
        ("_u", b"u"),
        ("file.txt", b"f"),
        ("sub/inner.txt", b"i"),
        ("x", b"file named x"),
    ]);
    let zips = fx.host(&[]);
    write_zip(
        &zips.join("pack.zip"),
        &[("a/b/c.txt", b"c"), ("z.txt", b"z")],
    );
    let d = fx.host(&[("leaf.txt", b"leaf")]);
    let e = fx.host(&[]);
    assert!(!mount_stdio(repo, "/", &h0, UIO_MOUNT_TOP).is_null());
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    let src = open_dir(repo, "/src");
    assert!(!mount_zip(
        repo,
        "/",
        src,
        "pack.zip",
        UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY
    )
    .is_null());
    let deep_er = mount_stdio(repo, "/deep/er", &d, UIO_MOUNT_TOP);
    let deep_other_third = mount_stdio(repo, "/deep/other/third", &e, UIO_MOUNT_TOP);
    assert!(!deep_er.is_null() && !deep_other_third.is_null());
    Stack {
        repo,
        root: open_dir(repo, "/"),
        deep_er,
        deep_other_third,
    }
}

#[test]
#[serial]
fn stat_knows_directories_that_only_the_mount_tree_has() {
    let mut fx = Fx::new();
    let s = stack(&mut fx);
    for path in [
        "/deep",
        "/deep/other",
        "/deep/er",
        "/deep/other/third",
        "/a",
        "/a/b",
        "/sub",
    ] {
        let (kind, _) = stat_of(s.root, path).unwrap_or_else(|errno| panic!("{path}: {errno}"));
        assert_eq!(kind, S_IFDIR, "{path}");
    }
    for path in ["/de", "/deep/e", "/deepx", "/deep/other/thir", "/deep/zz"] {
        assert_eq!(stat_of(s.root, path), Err(libc::ENOENT), "{path}");
    }
    // A physical entry wins over the mount tree, and a path through a file is
    // not a missing entry.
    assert_eq!(stat_of(s.root, "/x"), Ok((S_IFREG, 12)));
    assert_eq!(stat_of(s.root, "/x/nothing"), Err(libc::ENOTDIR));
}

#[test]
#[serial]
fn a_directory_list_includes_mount_tree_children_in_byte_order() {
    let mut fx = Fx::new();
    let s = stack(&mut fx);
    assert_eq!(
        list(s.root, "/"),
        expected(&["Zeta", "_u", "a", "deep", "file.txt", "src", "sub", "x", "z.txt"])
    );
    assert_eq!(list(s.root, "/deep"), expected(&["er", "other"]));
    assert_eq!(list(s.root, "/deep/other"), expected(&["third"]));
    assert_eq!(list(s.root, "/deep/er"), expected(&["leaf.txt"]));
    assert_eq!(list(s.root, "/deep/other/third"), expected(&[]));
    assert_eq!(list(s.root, "/a"), expected(&["b"]));
    assert_eq!(list(s.root, "/a/b"), expected(&["c.txt"]));
    // The path runs through a file: the original ends the walk with success
    // and nothing to list.
    assert_eq!(list(s.root, "/x/q"), expected(&[]));
    // The filter selects mount-tree names like any other.
    assert_eq!(
        names_of(s.root, "/deep", "th", MATCH_SUBSTRING),
        expected(&["other"])
    );
}

#[test]
#[serial]
fn a_handle_inside_the_mount_tree_lists_and_stats_what_is_mounted_below_it() {
    let mut fx = Fx::new();
    let s = stack(&mut fx);
    let deep = open_dir(s.repo, "/deep");
    assert!(!deep.is_null());
    assert_eq!(list(deep, ""), expected(&["er", "other"]));
    assert_eq!(list(deep, "other"), expected(&["third"]));
    assert_eq!(list(deep, "er"), expected(&["leaf.txt"]));
    assert_eq!(stat_of(deep, "er").map(|(kind, _)| kind), Ok(S_IFDIR));
    assert_eq!(
        stat_of(deep, "other/third").map(|(kind, _)| kind),
        Ok(S_IFDIR)
    );
    assert_eq!(
        stat_of(deep, "../deep/other").map(|(kind, _)| kind),
        Ok(S_IFDIR)
    );
    assert_eq!(stat_of(deep, "er/leaf.txt"), Ok((S_IFREG, 4)));
}

#[test]
#[serial]
fn unmounting_and_remounting_changes_the_virtual_directories() {
    let mut fx = Fx::new();
    let s = stack(&mut fx);
    unsafe { assert_eq!(uio_unmountDir(s.deep_er), 0) };
    assert_eq!(list(s.root, "/deep"), expected(&["other"]));
    assert_eq!(stat_of(s.root, "/deep/er"), Err(libc::ENOENT));
    assert_eq!(stat_of(s.root, "/deep").map(|(kind, _)| kind), Ok(S_IFDIR));

    unsafe { assert_eq!(uio_unmountDir(s.deep_other_third), 0) };
    assert_eq!(stat_of(s.root, "/deep"), Err(libc::ENOENT));
    assert_eq!(open_result(s.repo, "/deep"), Err(libc::ENOENT));
    assert_eq!(
        list(s.root, "/"),
        expected(&["Zeta", "_u", "a", "file.txt", "src", "sub", "x", "z.txt"])
    );

    let again = fx.host(&[("again.txt", b"1")]);
    assert!(!mount_stdio(s.repo, "/deep/er", &again, UIO_MOUNT_TOP).is_null());
    assert_eq!(stat_of(s.root, "/deep").map(|(kind, _)| kind), Ok(S_IFDIR));
    assert_eq!(list(s.root, "/deep"), expected(&["er"]));
    assert_eq!(list(s.root, "/deep/er"), expected(&["again.txt"]));
    assert_eq!(open_result(s.repo, "/deep"), Ok(()));
}

#[test]
#[serial]
fn two_repositories_keep_their_own_virtual_directories() {
    let mut fx = Fx::new();
    let first = stack(&mut fx);
    let second = fx.repo();
    let host = fx.host(&[("only.txt", b"o")]);
    assert!(!mount_stdio(second, "/deep/zz", &host, UIO_MOUNT_TOP).is_null());
    let second_root = open_dir(second, "/");

    assert_eq!(list(second_root, "/"), expected(&["deep"]));
    assert_eq!(list(second_root, "/deep"), expected(&["zz"]));
    assert_eq!(stat_of(second_root, "/deep/er"), Err(libc::ENOENT));
    assert_eq!(
        stat_of(second_root, "/deep/zz").map(|(kind, _)| kind),
        Ok(S_IFDIR)
    );

    assert_eq!(list(first.root, "/deep"), expected(&["er", "other"]));
    assert_eq!(stat_of(first.root, "/deep/zz"), Err(libc::ENOENT));
}

#[test]
#[serial]
fn access_agrees_with_stat_about_virtual_directories() {
    let mut fx = Fx::new();
    let s = stack(&mut fx);
    assert_eq!(access_of(s.root, "/deep", F_OK), Ok(()));
    assert_eq!(access_of(s.root, "/deep", R_OK | X_OK), Ok(()));
    assert_eq!(access_of(s.root, "/de", F_OK), Err(libc::ENOENT));
    assert_eq!(access_of(s.root, "/deep/e", F_OK), Err(libc::ENOENT));
}

/// Puts the mode of a directory back when the test ends, however it ends, so
/// the temporary directory can be removed.
struct Restore(PathBuf);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

#[test]
#[serial]
fn a_directory_the_host_will_not_read_is_an_error_not_a_missing_layer() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let writable = fx.host(&[("open/inner.txt", b"i"), ("locked/hidden.txt", b"h")]);
    let lower = fx.host(&[("locked/shadow.txt", b"s")]);
    assert!(!mount_stdio(repo, "/", &lower, UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY).is_null());
    assert!(!mount_stdio(repo, "/", &writable, UIO_MOUNT_TOP).is_null());
    let root = open_dir(repo, "/");

    let locked = writable.join("locked");
    let _restore = Restore(locked.clone());
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        std::fs::read_dir(&locked).unwrap_err().raw_os_error(),
        Some(libc::EACCES),
        "the fixture must really be unreadable (not running as root)"
    );

    assert_eq!(list(root, "/"), expected(&["locked", "open"]));
    assert_eq!(list(root, "/open"), expected(&["inner.txt"]));
    assert_eq!(list(root, "/locked"), Err(libc::EACCES));
    assert_eq!(stat_of(root, "/locked").map(|(kind, _)| kind), Ok(S_IFDIR));
    assert_eq!(stat_of(root, "/locked/hidden.txt"), Err(libc::EACCES));

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        list(root, "/locked"),
        expected(&["hidden.txt", "shadow.txt"])
    );
}
