//! Rows LF1-LF4: open streams, unmount, remount, repository close, and the
//! resource index after its mount is gone.

use serial_test::serial;
use tempfile::TempDir;

use super::declared::*;
use super::game::write_files;
use super::res;
use super::uio_api::{Mount, Repo, Session};

fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed.wrapping_add((i % 251) as u8))
        .collect()
}

#[test]
#[serial]
fn lf1_open_stream_survives_the_unmount_of_its_mount() {
    let _session = Session::begin();
    let repo = Repo::open();
    let upper_bytes = pattern(20_000, 1);
    let lower = TempDir::new().unwrap();
    let upper = TempDir::new().unwrap();
    write_files(lower.path(), &[("x", b"LOWER")]);
    write_files(upper.path(), &[("x", &upper_bytes)]);
    let lower_mount = repo.mount_stdio("/", lower.path(), MOUNT_BOTTOM, Mount::NONE);
    let upper_mount = repo.mount_stdio("/", upper.path(), MOUNT_TOP, Mount::NONE);
    let root = repo.open_dir("/");

    let stream = root.open_stream("x").unwrap();
    assert_eq!(stream.read_exact_prefix(100), upper_bytes[..100]);
    assert_eq!(repo.unmount(upper_mount), 0);
    assert_eq!(stream.read_to_end(), upper_bytes[100..]);

    assert_eq!(root.read("x").unwrap(), b"LOWER");
    assert_eq!(repo.unmount(lower_mount), 0);
}

/// C answers ENOENT once the last mount is gone, even for a handle that was
/// opened while that mount was on top: the handle names a repository path, not
/// the mount's host directory.
#[test]
#[serial]
fn lf1b_nothing_is_served_after_the_last_mount_is_gone() {
    let _session = Session::begin();
    let repo = Repo::open();
    let only = TempDir::new().unwrap();
    write_files(only.path(), &[("x", b"ONLY")]);
    let mount = repo.mount_stdio("/", only.path(), MOUNT_TOP, Mount::NONE);
    let root = repo.open_dir("/");
    assert_eq!(root.read("x").unwrap(), b"ONLY");
    assert_eq!(repo.unmount(mount), 0);
    assert_eq!(root.read("x").unwrap_err(), libc::ENOENT);
}

#[test]
#[serial]
fn lf2_unmount_reveals_the_next_mount_and_remount_restores_the_top() {
    let _session = Session::begin();
    let repo = Repo::open();
    let lower = TempDir::new().unwrap();
    let upper = TempDir::new().unwrap();
    write_files(lower.path(), &[("x", b"LOWER")]);
    write_files(upper.path(), &[("x", b"UPPER")]);
    assert!(!repo
        .mount_stdio("/", lower.path(), MOUNT_BOTTOM, Mount::NONE)
        .is_null());
    let first = repo.mount_stdio("/", upper.path(), MOUNT_TOP, Mount::NONE);
    let root = repo.open_dir("/");
    assert_eq!(root.read("x").unwrap(), b"UPPER");
    assert_eq!(repo.unmount(first), 0);
    assert_eq!(root.read("x").unwrap(), b"LOWER");
    assert!(!repo
        .mount_stdio("/", upper.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    assert_eq!(root.read("x").unwrap(), b"UPPER");
}

#[test]
#[serial]
fn lf3_closing_a_repository_removes_only_its_own_mounts() {
    let _session = Session::begin();
    let first = Repo::open();
    let second = Repo::open();
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    write_files(a.path(), &[("x", b"A")]);
    write_files(b.path(), &[("y", b"B")]);
    assert!(!first
        .mount_stdio("/a", a.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    assert!(!second
        .mount_stdio("/b", b.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    let first_root = first.open_dir("/");
    let second_root = second.open_dir("/");
    assert_eq!(first_root.read("a/x").unwrap(), b"A");

    first.close();
    assert!(second_root.read("a/x").is_err());
    assert_eq!(second_root.read("b/y").unwrap(), b"B");
    drop(first_root);
}

#[test]
#[serial]
fn lf4_loaded_index_outlives_its_mount_and_uninit_empties_it() {
    let _session = Session::begin();
    let repo = Repo::open();
    let host = TempDir::new().unwrap();
    write_files(host.path(), &[("t.rmp", b"k = STRING:kept\nn = INT32:3\n")]);
    let mount = repo.mount_stdio("/", host.path(), MOUNT_TOP, Mount::NONE);
    let root = repo.open_dir("/");
    res::load(&root, "t.rmp", None);
    assert_eq!(repo.unmount(mount), 0);
    assert_eq!(res::text("k"), "kept");
    assert_eq!(res::integer("n"), 3);

    res::uninit();
    assert!(!res::has("k"));
    res::uninit();
    assert_eq!(res::text("k"), "");
}
