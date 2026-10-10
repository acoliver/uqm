//! Rows PT1-PT5 and CS1-CS2: path normalization, mount confinement, and case.

use serial_test::serial;
use tempfile::TempDir;

use super::declared::*;
use super::game::{write_files, write_zip};
use super::uio_api::{Dir, Mount, Repo, Session};

fn dir_with(files: &[(&str, &[u8])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), files);
    dir
}

/// A ZIP of `entries` mounted at "/" through a directory handle, as the
/// game mounts a package.
fn mount_case_zip(repo: &Repo, entries: Files) -> (TempDir, Dir) {
    let host = TempDir::new().unwrap();
    write_zip(&host.path().join("pack.zip"), entries);
    // The archive lies in the repository's `packages` directory, where the
    // game's content tree keeps it, and is mounted from there.
    let packages = repo.mount_stdio(
        "/packages",
        host.path(),
        MOUNT_TOP | MOUNT_RDONLY,
        Mount::NONE,
    );
    assert!(!packages.is_null());
    let source = repo.open_dir("/packages");
    let mount = repo.mount_zip(
        "/",
        &source,
        "pack.zip",
        MOUNT_BOTTOM | MOUNT_RDONLY,
        Mount::NONE,
    );
    assert!(!mount.is_null());
    (host, repo.open_dir("/"))
}

#[test]
#[serial]
fn pt1_dot_dotdot_and_double_slashes_normalize_inside_the_virtual_tree() {
    let _session = Session::begin();
    let repo = Repo::open();
    let host = dir_with(&[("c", b"ROOT-C"), ("a/c", b"AC")]);
    assert!(!repo
        .mount_stdio("/", host.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    let root = repo.open_dir("/");

    assert_eq!(root.read("a/./b//../c").unwrap(), b"AC");
    assert_eq!(root.read("/../../c").unwrap(), b"ROOT-C");
    assert_eq!(root.read("a/../a/c").unwrap(), b"AC");
    let sub = root.open_relative("a");
    assert_eq!(sub.read("../c").unwrap(), b"ROOT-C");
    assert_eq!(sub.read("../../../c").unwrap(), b"ROOT-C");
}

#[test]
#[serial]
fn pt2_dotdot_cannot_leave_a_mounted_directory() {
    let _session = Session::begin();
    let repo = Repo::open();
    let outer = TempDir::new().unwrap();
    write_files(
        outer.path(),
        &[
            ("outside.txt", b"SECRET"),
            ("mnt/in.txt", b"IN"),
            ("mnt/a/x", b"X"),
        ],
    );
    assert!(!repo
        .mount_stdio("/", &outer.path().join("mnt"), MOUNT_TOP, Mount::NONE)
        .is_null());
    let root = repo.open_dir("/");
    let sub = root.open_relative("a");

    assert_eq!(root.read("in.txt").unwrap(), b"IN");
    for (dir, path) in [
        (&root, "../outside.txt"),
        (&root, "a/../../outside.txt"),
        (&root, "/../outside.txt"),
        (&sub, "../../outside.txt"),
    ] {
        assert!(
            dir.read(path).is_err(),
            "{path} reached the host directory above the mount"
        );
    }
}

#[test]
#[serial]
fn pt3_mount_points_match_whole_path_components() {
    let _session = Session::begin();
    let repo = Repo::open();
    let host = dir_with(&[("voice.txt", b"V")]);
    assert!(!repo
        .mount_stdio("addons/aaa", host.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    let root = repo.open_dir("/");
    assert_eq!(root.read("addons/aaa/voice.txt").unwrap(), b"V");
    assert!(root.read("addons/aaax/voice.txt").is_err());
    assert!(root.read("addons/aa/voice.txt").is_err());
}

#[test]
#[serial]
fn pt4_missing_files_report_enoent() {
    let _session = Session::begin();
    let repo = Repo::open();
    let host = dir_with(&[("here.txt", b"H")]);
    assert!(!repo
        .mount_stdio("/", host.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    let root = repo.open_dir("/");
    assert_eq!(root.read("gone.txt").unwrap_err(), libc::ENOENT);
    assert_eq!(root.stat("gone.txt").unwrap_err(), libc::ENOENT);
    assert_eq!(root.stat("here.txt").unwrap().1, 1);
}

/// With no mount at "/", an absolute host path is not part of the virtual
/// tree: C answers ENOENT for any path no mount covers, so the file must not
/// be served from the host.
#[test]
#[serial]
fn pt5_a_path_no_mount_covers_is_not_served_from_the_host() {
    let _session = Session::begin();
    let repo = Repo::open();
    let mounted = dir_with(&[("m.txt", b"M")]);
    let unmounted = dir_with(&[("host-only.txt", b"HOST")]);
    assert!(!repo
        .mount_stdio("/game", mounted.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    let root = repo.open_dir("/");
    assert_eq!(root.read("game/m.txt").unwrap(), b"M");
    let host_file = unmounted.path().join("host-only.txt");
    let absolute = host_file.to_str().unwrap();
    assert!(
        root.read(absolute).is_err(),
        "{absolute} came from the host file system"
    );
}

#[test]
#[serial]
fn cs1_archive_names_that_differ_only_in_case_are_distinct_files() {
    let _session = Session::begin();
    let repo = Repo::open();
    let (_host, root) = mount_case_zip(&repo, CASE_ZIP);
    assert_eq!(root.read("Case/Readme.TXT").unwrap(), b"UPPER");
    assert_eq!(root.read("case/readme.txt").unwrap(), b"lower");
    assert_eq!(root.read("CASE/README.txt").unwrap(), b"SHOUT");
    for path in [
        "Case/readme.txt",
        "CASE/readme.txt",
        "case/Readme.TXT",
        "case/README.TXT",
    ] {
        assert!(root.read(path).is_err(), "{path} matched across case");
    }
    assert_eq!(root.list_all(), ["CASE", "Case", "case"]);
}

#[test]
#[serial]
fn cs2_listing_patterns_are_case_sensitive_unless_the_pattern_says_otherwise() {
    let _session = Session::begin();
    let repo = Repo::open();
    let (_host, root) = mount_case_zip(&repo, CASE_LISTING_ZIP);
    assert_eq!(
        root.list_all(),
        [
            "Addons",
            "B.RMP",
            "a.rmp",
            "addons",
            "c.Rmp",
            "d.rmpx",
            "e.rmp.bak"
        ]
    );
    assert_eq!(root.list(".rmp", MATCH_SUFFIX), ["a.rmp"]);
    assert_eq!(
        root.list(RMP_PATTERN, MATCH_REGEX),
        ["B.RMP", "a.rmp", "c.Rmp"]
    );
    assert_eq!(root.list("addons", MATCH_PREFIX), ["addons"]);
    assert_eq!(root.list("Addons", MATCH_LITERAL), ["Addons"]);
    assert!(root.list("ADDONS", MATCH_LITERAL).is_empty());
}
