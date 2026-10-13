//! Fixture rows M* and C* of `tmp/issue38-port/dirs/fixture-spec.md`, run on
//! a real temporary directory. Paths are absolute so no test changes the
//! working directory of the shared test process.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::{Path, PathBuf};

use libc::{EACCES, EEXIST, EILSEQ, ENAMETOOLONG, ENOENT, ENOTDIR};

use super::mkdirhier::{create_directory, mkdirhier};
use super::test_support::{bytes_or_errno, resolved_tempdir, ModeGuard};

fn make(path: &Path) -> Result<(), i32> {
    bytes_or_errno(mkdirhier(path.as_os_str().as_bytes()))
}

fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().mode() & 0o7777
}

/// The mode plain `mkdir(path, 0777)` yields under this process's umask.
fn reference_mode(root: &Path) -> u32 {
    let reference = root.join("reference");
    create_directory(&reference, 0o777).unwrap();
    mode_of(&reference)
}

#[test]
fn empty_and_root_paths_succeed_without_touching_anything() {
    assert_eq!(bytes_or_errno(mkdirhier(b"")), Ok(())); // M01
    assert_eq!(bytes_or_errno(mkdirhier(b"/")), Ok(())); // M02
}

#[test]
fn existing_directory_is_left_alone() {
    let (_guard, root) = resolved_tempdir();
    std::fs::create_dir(root.join("d")).unwrap();
    std::fs::write(root.join("d/keep"), b"x").unwrap();
    assert_eq!(make(&root.join("d")), Ok(())); // M03
    assert_eq!(std::fs::read(root.join("d/keep")).unwrap(), b"x");
}

#[test]
fn creates_every_missing_component_with_mode_0777_under_the_umask() {
    let (_guard, root) = resolved_tempdir();
    let expected = reference_mode(&root);
    assert_eq!(make(&root.join("a/b/c")), Ok(())); // M04
    for part in ["a", "a/b", "a/b/c"] {
        assert_eq!(mode_of(&root.join(part)), expected, "{part}");
    }
}

#[test]
fn repeated_and_trailing_slashes_collapse() {
    let (_guard, root) = resolved_tempdir();
    let base = root.to_str().unwrap();
    assert_eq!(
        bytes_or_errno(mkdirhier(format!("{base}/a/b///").as_bytes())),
        Ok(())
    ); // M05
    assert!(root.join("a/b").is_dir());
    assert_eq!(
        bytes_or_errno(mkdirhier(format!("{base}//c///d").as_bytes())),
        Ok(())
    ); // M06
    assert!(root.join("c/d").is_dir());
}

#[test]
fn a_file_in_the_way_is_enotdir_and_nothing_is_created() {
    let (_guard, root) = resolved_tempdir();
    std::fs::write(root.join("file"), b"").unwrap();
    assert_eq!(make(&root.join("file/x")), Err(ENOTDIR)); // M07
    assert!(!root.join("file/x").exists());
    assert_eq!(make(&root.join("file")), Err(ENOTDIR)); // M08
}

#[test]
fn unwritable_parent_reports_the_os_error() {
    let (_guard, root) = resolved_tempdir();
    std::fs::create_dir(root.join("ro")).unwrap();
    let _restore = ModeGuard::set(&root.join("ro"), 0o500);
    let want = if unsafe { libc::geteuid() } == 0 {
        Ok(())
    } else {
        Err(EACCES)
    };
    assert_eq!(make(&root.join("ro/new")), want); // M09
}

#[test]
fn parents_made_before_a_failure_stay() {
    let (_guard, root) = resolved_tempdir();
    let too_long = "x".repeat(300);
    assert_eq!(make(&root.join("a").join(too_long)), Err(ENAMETOOLONG)); // M10
    assert!(root.join("a").is_dir());
}

#[test]
fn symlinks_are_followed_like_stat_does() {
    let (_guard, root) = resolved_tempdir();
    std::fs::create_dir(root.join("target")).unwrap();
    symlink(root.join("target"), root.join("link")).unwrap();
    assert_eq!(make(&root.join("link/x")), Ok(())); // M11
    assert!(root.join("target/x").is_dir());

    symlink(root.join("missing"), root.join("dangling")).unwrap();
    assert_eq!(make(&root.join("dangling")), Err(EEXIST)); // M12
}

#[test]
fn utf8_names_are_exact() {
    let (_guard, root) = resolved_tempdir();
    assert_eq!(make(&root.join("é/日本")), Ok(())); // M13
    let names: Vec<_> = std::fs::read_dir(root.join("é"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![OsStr::new("日本")]);
}

#[test]
fn non_utf8_names_are_passed_through_or_refused_by_the_filesystem() {
    let (_guard, root) = resolved_tempdir();
    let mut path = root.as_os_str().as_bytes().to_vec();
    path.extend_from_slice(b"/\xff\xfe");
    match bytes_or_errno(mkdirhier(&path)) {
        Ok(()) => {
            let names: Vec<_> = std::fs::read_dir(&root)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().as_bytes().to_vec())
                .collect();
            assert_eq!(names, vec![b"\xff\xfe".to_vec()]); // M14 (Linux)
        }
        // macOS APFS refuses names that are not valid UTF-8.
        Err(code) => assert_eq!(code, EILSEQ),
    }
    for entry in std::fs::read_dir(&root).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(
            !name
                .as_bytes()
                .windows(3)
                .any(|window| window == "\u{fffd}".as_bytes()),
            "no lossy replacement character in {name:?}"
        );
    }
}

#[test]
fn dot_dot_components_are_stat_ed_and_created_through() {
    let (_guard, root) = resolved_tempdir();
    std::fs::create_dir(root.join("a")).unwrap();
    assert_eq!(make(&root.join("a/../b")), Ok(())); // M15
    assert!(root.join("b").is_dir());

    assert_eq!(make(&root.join("new/../c")), Err(EEXIST)); // M15b
    assert!(root.join("new").is_dir());
    assert!(!root.join("c").exists());
}

#[test]
fn relative_paths_resolve_against_the_working_directory() {
    use super::test_support::in_child;
    let (_guard, root) = resolved_tempdir();
    let in_parent = in_child(
        module_path!(),
        "relative_paths_resolve_against_the_working_directory",
        &[],
        Some(&root),
        || {
            assert_eq!(bytes_or_errno(mkdirhier(b"x/y")), Ok(())); // M16
            assert!(Path::new("x/y").is_dir());
        },
    );
    if in_parent {
        assert!(root.join("x/y").is_dir());
    }
}

#[test]
fn create_directory_honours_mode_and_reports_errno() {
    let (_guard, root) = resolved_tempdir();
    let new = root.join("n");
    assert_eq!(bytes_or_errno(create_directory(&new, 0o700)), Ok(())); // C01
    assert_eq!(mode_of(&new) & 0o700, 0o700);
    assert_eq!(bytes_or_errno(create_directory(&new, 0o700)), Err(EEXIST)); // C02
    assert_eq!(
        bytes_or_errno(create_directory(&root.join("missing/n"), 0o700)),
        Err(ENOENT)
    ); // C03
    let long: PathBuf = root.join("y".repeat(300));
    assert_eq!(
        bytes_or_errno(create_directory(&long, 0o700)),
        Err(ENAMETOOLONG)
    ); // C04
    let none = root.join("z");
    assert_eq!(bytes_or_errno(create_directory(&none, 0)), Ok(())); // C05
    let _restore = ModeGuard::set(&none, 0);
    assert_eq!(mode_of(&none) & 0o777, 0);
}
