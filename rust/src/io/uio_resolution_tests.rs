//! A directory handle is a place in a repository's virtual tree. Every
//! operation on it, and every mount made from it, has to look that place up in
//! the mount stack as it is now: after an unmount or a remount at the same
//! virtual name, the handle reaches the new bytes, never the host directory
//! that was mounted when it was opened.
//!
//! The fixtures are real temporary directories and real ZIP files, driven only
//! through the public UIO functions, in the call order the game uses.

use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;

use proptest::prelude::*;
use serial_test::serial;
use tempfile::TempDir;

use super::*;
use crate::io::name_filter::MATCH_SUBSTRING;

pub(super) const RDONLY_TOP: c_int = UIO_MOUNT_RDONLY | UIO_MOUNT_TOP;

pub(super) fn cstr(text: &str) -> CString {
    CString::new(text).unwrap()
}

pub(super) fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// Owns the temporary hosts and repositories of one test and resets the
/// process-wide mount registry before and after it.
pub(super) struct Fx {
    hosts: Vec<TempDir>,
    repos: Vec<*mut uio_Repository>,
}

impl Fx {
    pub(super) fn new() -> Self {
        clear_registry();
        Fx {
            hosts: Vec::new(),
            repos: Vec::new(),
        }
    }

    pub(super) fn repo(&mut self) -> *mut uio_Repository {
        let repo = unsafe { uio_openRepository(0) };
        self.repos.push(repo);
        repo
    }

    /// A host directory holding `files`; names may contain `/`.
    pub(super) fn host(&mut self, files: &[(&str, &[u8])]) -> PathBuf {
        let dir = TempDir::new().unwrap();
        for (name, data) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, data).unwrap();
        }
        let path = dir.path().to_path_buf();
        self.hosts.push(dir);
        path
    }
}

impl Drop for Fx {
    fn drop(&mut self) {
        clear_registry();
        for repo in self.repos.drain(..) {
            drop(unsafe { Box::from_raw(repo) });
        }
    }
}

fn clear_registry() {
    if let Ok(mut registry) = get_mount_registry().lock() {
        registry.clear();
    }
}

pub(super) fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

pub(super) fn mount_stdio(
    repo: *mut uio_Repository,
    point: &str,
    host: &Path,
    flags: c_int,
) -> *mut uio_MountHandle {
    let (point, host) = (
        cstr(point),
        CString::new(host.as_os_str().as_bytes()).unwrap(),
    );
    unsafe {
        uio_mountDir(
            repo,
            point.as_ptr(),
            UIO_FSTYPE_STDIO,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            flags,
            ptr::null_mut(),
        )
    }
}

pub(super) fn mount_zip(
    repo: *mut uio_Repository,
    point: &str,
    source: *mut uio_DirHandle,
    name: &str,
    flags: c_int,
) -> *mut uio_MountHandle {
    let (point, name) = (cstr(point), cstr(name));
    unsafe {
        uio_mountDir(
            repo,
            point.as_ptr(),
            UIO_FSTYPE_ZIP,
            source,
            name.as_ptr(),
            c"/".as_ptr(),
            ptr::null_mut(),
            flags,
            ptr::null_mut(),
        )
    }
}

pub(super) fn open_dir(repo: *mut uio_Repository, path: &str) -> *mut uio_DirHandle {
    let path = cstr(path);
    unsafe { uio_openDir(repo, path.as_ptr(), 0) }
}

fn open_dir_relative(base: *mut uio_DirHandle, path: &str) -> *mut uio_DirHandle {
    let path = cstr(path);
    unsafe { uio_openDirRelative(base, path.as_ptr(), 0) }
}

/// The directory handle, or the errno of the refusal.
fn dir_or_errno(handle: *mut uio_DirHandle) -> Result<*mut uio_DirHandle, c_int> {
    if handle.is_null() {
        Err(errno_value())
    } else {
        Ok(handle)
    }
}

pub(super) fn read(dir: *mut uio_DirHandle, name: &str) -> Result<Vec<u8>, c_int> {
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

/// The mount handle and path `uio_get_file_location` reports, or the errno.
fn locate(dir: *mut uio_DirHandle, name: &str) -> Result<(*mut uio_MountHandle, PathBuf), c_int> {
    let name = cstr(name);
    let mut mount: *mut uio_MountHandle = ptr::null_mut();
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { uio_get_file_location(dir, name.as_ptr(), 0, &mut mount, &mut out) };
    if rc != 0 {
        let errno = errno_value();
        assert!(
            mount.is_null() && out.is_null(),
            "failure clears the outputs"
        );
        return Err(errno);
    }
    let path = PathBuf::from(std::ffi::OsStr::from_bytes(
        unsafe { CStr::from_ptr(out) }.to_bytes(),
    ));
    unsafe { libc::free(out.cast()) };
    Ok((mount, path))
}

fn stdio_access(
    dir: *mut uio_DirHandle,
    name: &str,
    temp: *mut uio_DirHandle,
) -> Result<(*mut uio_StdioAccessHandle, PathBuf), c_int> {
    let name = cstr(name);
    unsafe {
        let access = uio_getStdioAccess(dir, name.as_ptr(), 0, temp);
        if access.is_null() {
            return Err(errno_value());
        }
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(
            CStr::from_ptr(uio_StdioAccessHandle_getPath(access)).to_bytes(),
        ));
        Ok((access, path))
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn list_names(dir: *mut uio_DirHandle, path: &str) -> Vec<String> {
    let (path, pattern) = (cstr(path), cstr(""));
    unsafe {
        let list = uio_getDirList(dir, path.as_ptr(), pattern.as_ptr(), MATCH_SUBSTRING);
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

// ---------------------------------------------------------------------------
// F1: a handle follows the mount stack through unmount and remount
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn a_directory_handle_reaches_the_replacement_of_an_unmounted_mount() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let a = fx.host(&[("data.txt", b"A-bytes")]);
    let b = fx.host(&[("data.txt", b"B-bytes")]);
    let mount_a = mount_stdio(repo, "/", &a, UIO_MOUNT_TOP);
    assert!(!mount_a.is_null());
    let root = open_dir(repo, "/");
    assert!(!root.is_null());
    assert_eq!(read(root, "data.txt").unwrap(), b"A-bytes");

    assert_eq!(unsafe { uio_unmountDir(mount_a) }, 0);
    assert_eq!(read(root, "data.txt"), Err(libc::ENOENT));
    assert_eq!(locate(root, "data.txt").unwrap_err(), libc::ENOENT);

    let mount_b = mount_stdio(repo, "/", &b, UIO_MOUNT_TOP);
    assert!(!mount_b.is_null());
    assert_eq!(read(root, "data.txt").unwrap(), b"B-bytes");
    let (located_in, path) = locate(root, "data.txt").unwrap();
    assert_eq!(located_in, mount_b);
    assert!(same_file(&path, &b.join("data.txt")), "{path:?}");
    let (access, path) = stdio_access(root, "data.txt", ptr::null_mut()).unwrap();
    assert!(same_file(&path, &b.join("data.txt")), "{path:?}");
    unsafe { uio_releaseStdioAccess(access) };
}

#[test]
#[serial]
fn a_nested_handle_whose_directory_left_the_stack_fails_until_it_is_back() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let a = fx.host(&[("sub/x.txt", b"A")]);
    let b = fx.host(&[("sub/x.txt", b"B")]);
    let mount_a = mount_stdio(repo, "/", &a, UIO_MOUNT_TOP);
    let sub = open_dir(repo, "/sub");
    assert!(!sub.is_null());
    assert_eq!(read(sub, "x.txt").unwrap(), b"A");

    assert_eq!(unsafe { uio_unmountDir(mount_a) }, 0);
    // Nothing is left of "/sub": no reads, no locations, no children.
    assert_eq!(read(sub, "x.txt"), Err(libc::ENOENT));
    assert_eq!(locate(sub, "x.txt").unwrap_err(), libc::ENOENT);
    assert_eq!(
        dir_or_errno(open_dir_relative(sub, ".")),
        Err(libc::ENOENT),
        "a handle is not a placeholder for a directory that is gone"
    );

    assert!(!mount_stdio(repo, "/", &b, UIO_MOUNT_TOP).is_null());
    assert_eq!(read(sub, "x.txt").unwrap(), b"B");
    assert!(!open_dir_relative(sub, ".").is_null());
}

#[test]
#[serial]
fn unmounting_and_remounting_changes_which_mount_provides_a_file() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let lower = fx.host(&[("same.txt", b"lower")]);
    let upper = fx.host(&[("same.txt", b"upper")]);
    let lower_mount = mount_stdio(repo, "/", &lower, UIO_MOUNT_TOP);
    let upper_mount = mount_stdio(repo, "/", &upper, UIO_MOUNT_TOP);
    let root = open_dir(repo, "/");

    assert_eq!(locate(root, "same.txt").unwrap().0, upper_mount);
    assert_eq!(read(root, "same.txt").unwrap(), b"upper");

    assert_eq!(unsafe { uio_unmountDir(upper_mount) }, 0);
    assert_eq!(locate(root, "same.txt").unwrap().0, lower_mount);
    assert_eq!(read(root, "same.txt").unwrap(), b"lower");

    let again = mount_stdio(repo, "/", &upper, UIO_MOUNT_TOP);
    assert_eq!(locate(root, "same.txt").unwrap().0, again);
    assert_eq!(read(root, "same.txt").unwrap(), b"upper");
}

// ---------------------------------------------------------------------------
// F2: the archive named to uio_mountDir is found through the current tree
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn a_zip_source_is_found_in_the_current_tree_not_in_the_mount_it_was_opened_on() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let a = fx.host(&[]);
    let b = fx.host(&[]);
    write_zip(&a.join("packages/pack.zip"), &[("f.txt", b"ZIP-A")]);
    write_zip(&b.join("packages/pack.zip"), &[("f.txt", b"ZIP-B")]);
    let mount_a = mount_stdio(repo, "/", &a, RDONLY_TOP);
    let packages = open_dir(repo, "/packages");
    assert!(!packages.is_null());
    assert_eq!(unsafe { uio_unmountDir(mount_a) }, 0);
    assert!(!mount_stdio(repo, "/", &b, RDONLY_TOP).is_null());

    let zip_mount = mount_zip(repo, "/z", packages, "pack.zip", RDONLY_TOP);
    assert!(!zip_mount.is_null(), "errno {}", errno_value());
    let root = open_dir(repo, "/");
    assert_eq!(read(root, "z/f.txt").unwrap(), b"ZIP-B");
}

#[test]
#[serial]
fn a_zip_source_missing_from_the_current_tree_is_enoent_even_if_the_old_host_file_exists() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let a = fx.host(&[]);
    write_zip(&a.join("packages/pack.zip"), &[("f.txt", b"ZIP-A")]);
    let mount_a = mount_stdio(repo, "/", &a, RDONLY_TOP);
    let packages = open_dir(repo, "/packages");
    assert!(!packages.is_null());
    assert_eq!(unsafe { uio_unmountDir(mount_a) }, 0);
    let empty = fx.host(&[("packages/other.txt", b"x")]);
    assert!(!mount_stdio(repo, "/", &empty, RDONLY_TOP).is_null());

    assert!(mount_zip(repo, "/z", packages, "pack.zip", RDONLY_TOP).is_null());
    assert_eq!(errno_value(), libc::ENOENT);
}

// ---------------------------------------------------------------------------
// F3: transplant takes what the source directory is now
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn a_transplant_receives_the_current_bytes_of_a_nested_source() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let c1 = fx.host(&[("addons/x/shadow/hello.txt", b"OLD")]);
    let c2 = fx.host(&[("addons/x/shadow/hello.txt", b"NEW")]);
    let mount_c1 = mount_stdio(repo, "/", &c1, RDONLY_TOP);
    let root = open_dir(repo, "/");
    let shadow = open_dir_relative(root, "addons/x/shadow");
    assert!(!shadow.is_null(), "errno {}", errno_value());
    assert_eq!(unsafe { uio_unmountDir(mount_c1) }, 0);
    assert!(!mount_stdio(repo, "/", &c2, RDONLY_TOP).is_null());

    let point = cstr("/t");
    let transplanted =
        unsafe { uio_transplantDir(point.as_ptr(), shadow, RDONLY_TOP, ptr::null_mut()) };
    assert!(!transplanted.is_null(), "errno {}", errno_value());
    assert_eq!(read(root, "t/hello.txt").unwrap(), b"NEW");
}

#[test]
#[serial]
fn a_transplant_needs_a_read_only_physical_directory_at_the_source() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let writable = fx.host(&[("shadow/hello.txt", b"RW")]);
    assert!(!mount_stdio(repo, "/", &writable, UIO_MOUNT_TOP).is_null());
    let root = open_dir(repo, "/");
    let shadow = open_dir_relative(root, "shadow");
    assert!(!shadow.is_null());
    let point = cstr("/t");
    assert!(
        unsafe { uio_transplantDir(point.as_ptr(), shadow, RDONLY_TOP, ptr::null_mut()) }.is_null()
    );
    assert_eq!(errno_value(), libc::ENOENT);
}

#[test]
#[serial]
fn a_transplant_of_a_source_that_left_the_stack_is_enoent() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let c1 = fx.host(&[("shadow/hello.txt", b"OLD")]);
    let mount = mount_stdio(repo, "/", &c1, RDONLY_TOP);
    let root = open_dir(repo, "/");
    let shadow = open_dir_relative(root, "shadow");
    assert!(!shadow.is_null());
    assert_eq!(unsafe { uio_unmountDir(mount) }, 0);
    let point = cstr("/t");
    assert!(
        unsafe { uio_transplantDir(point.as_ptr(), shadow, RDONLY_TOP, ptr::null_mut()) }.is_null()
    );
    assert_eq!(errno_value(), libc::ENOENT);
    assert_eq!(read(root, "t/hello.txt"), Err(libc::ENOENT));
}

#[test]
#[serial]
fn a_transplant_of_an_archive_subdirectory_mounts_that_subtree() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[]);
    write_zip(
        &host.join("pack.zip"),
        &[("sub/inner.txt", b"in-zip"), ("top.txt", b"top")],
    );
    let source = mount_stdio(repo, "/src", &host, RDONLY_TOP);
    assert!(!source.is_null());
    let src = open_dir(repo, "/src");
    assert!(!mount_zip(repo, "/z", src, "pack.zip", RDONLY_TOP).is_null());
    let sub = open_dir(repo, "/z/sub");
    assert!(!sub.is_null(), "errno {}", errno_value());

    let point = cstr("/t");
    let transplanted =
        unsafe { uio_transplantDir(point.as_ptr(), sub, RDONLY_TOP, ptr::null_mut()) };
    assert!(!transplanted.is_null(), "errno {}", errno_value());
    let root = open_dir(repo, "/");
    assert_eq!(read(root, "t/inner.txt").unwrap(), b"in-zip");
    assert_eq!(read(root, "t/top.txt"), Err(libc::ENOENT));
}

// ---------------------------------------------------------------------------
// F4: file location of an archive entry is not a host path
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn an_archive_entry_is_located_inside_its_archive_and_a_loose_file_on_the_host() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let loose = fx.host(&[("a.txt", b"loose"), ("only_loose.txt", b"l")]);
    let zips = fx.host(&[]);
    write_zip(
        &zips.join("pack.zip"),
        &[("a.txt", b"zip"), ("only_zip.txt", b"z"), ("d/e.txt", b"e")],
    );
    let loose_mount = mount_stdio(repo, "/", &loose, RDONLY_TOP);
    let staging = mount_stdio(repo, "/src", &zips, RDONLY_TOP);
    assert!(!loose_mount.is_null() && !staging.is_null());
    let src = open_dir(repo, "/src");
    let zip_mount = mount_zip(repo, "/", src, "pack.zip", RDONLY_TOP);
    assert!(!zip_mount.is_null(), "errno {}", errno_value());
    let root = open_dir(repo, "/");

    let (mount, path) = locate(root, "a.txt").unwrap();
    assert_eq!(mount, zip_mount, "the archive is on top");
    assert_eq!(
        unsafe { uio_get_mount_file_system_type(mount) },
        UIO_FSTYPE_ZIP
    );
    assert_eq!(path, Path::new("/a.txt"));
    assert!(!path.exists() || !same_file(&path, &zips.join("pack.zip/a.txt")));
    assert_eq!(locate(root, "d/e.txt").unwrap().1, Path::new("/d/e.txt"));

    let (mount, path) = locate(root, "only_loose.txt").unwrap();
    assert_eq!(mount, loose_mount);
    assert!(same_file(&path, &loose.join("only_loose.txt")), "{path:?}");

    assert_eq!(locate(root, "d").unwrap_err(), libc::EISDIR);
    assert_eq!(locate(root, "absent").unwrap_err(), libc::ENOENT);
}

// ---------------------------------------------------------------------------
// F5: stdio access, its temporary directory, and what it needs of that mount
// ---------------------------------------------------------------------------

fn zip_and_temp_stack(fx: &mut Fx, temp_flags: c_int) -> (*mut uio_Repository, PathBuf) {
    let repo = fx.repo();
    let zips = fx.host(&[]);
    write_zip(&zips.join("pack.zip"), &[("f.bin", b"zip bytes")]);
    let temp = fx.host(&[]);
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    let src = open_dir(repo, "/src");
    assert!(!mount_zip(repo, "/", src, "pack.zip", RDONLY_TOP).is_null());
    assert!(!mount_stdio(repo, "/tmp", &temp, temp_flags).is_null());
    (repo, temp)
}

#[test]
#[serial]
fn stdio_access_unpacks_an_archive_entry_into_a_fresh_directory_of_the_temp_dir() {
    let mut fx = Fx::new();
    let (repo, temp) = zip_and_temp_stack(&mut fx, UIO_MOUNT_TOP);
    let root = open_dir(repo, "/");
    let temp_dir = open_dir(repo, "/tmp");
    assert!(!temp_dir.is_null());

    let (access, path) = stdio_access(root, "f.bin", temp_dir).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"zip bytes");
    assert_eq!(path.file_name().unwrap(), "f.bin");
    let generated = path.parent().unwrap();
    assert_eq!(generated.parent().unwrap(), temp, "{path:?}");
    let name = generated.file_name().unwrap().to_str().unwrap();
    assert!(
        name.len() == 8 && name.bytes().all(|b| b.is_ascii_hexdigit()),
        "{name}"
    );

    unsafe { uio_releaseStdioAccess(access) };
    assert!(!path.exists() && !generated.exists());
    assert!(
        temp.exists(),
        "the caller's temp directory is not ours to remove"
    );
}

#[test]
#[serial]
fn stdio_access_leaves_a_file_on_a_stdio_mount_where_it_is() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let loose = fx.host(&[("x.txt", b"x")]);
    assert!(!mount_stdio(repo, "/", &loose, UIO_MOUNT_TOP).is_null());
    let root = open_dir(repo, "/");
    let (access, path) = stdio_access(root, "x.txt", ptr::null_mut()).unwrap();
    assert!(same_file(&path, &loose.join("x.txt")));
    unsafe { uio_releaseStdioAccess(access) };
    assert!(
        loose.join("x.txt").exists(),
        "a direct path is never deleted"
    );
}

#[test]
#[serial]
fn stdio_access_refuses_a_temp_dir_on_a_read_only_mount() {
    let mut fx = Fx::new();
    let (repo, temp) = zip_and_temp_stack(&mut fx, RDONLY_TOP);
    let root = open_dir(repo, "/");
    let temp_dir = open_dir(repo, "/tmp");
    assert!(!temp_dir.is_null());
    assert_eq!(
        stdio_access(root, "f.bin", temp_dir).map(|_| ()),
        Err(libc::EPERM)
    );
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
}

#[test]
#[serial]
fn stdio_access_refuses_a_temp_dir_whose_mount_is_gone() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let zips = fx.host(&[]);
    write_zip(&zips.join("pack.zip"), &[("f.bin", b"zip bytes")]);
    let temp = fx.host(&[]);
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    let src = open_dir(repo, "/src");
    assert!(!mount_zip(repo, "/z", src, "pack.zip", RDONLY_TOP).is_null());
    let temp_mount = mount_stdio(repo, "/tmp", &temp, UIO_MOUNT_TOP);
    let temp_dir = open_dir(repo, "/tmp");
    assert!(!temp_dir.is_null());
    assert_eq!(unsafe { uio_unmountDir(temp_mount) }, 0);

    let root = open_dir(repo, "/");
    assert_eq!(
        stdio_access(root, "z/f.bin", temp_dir).map(|_| ()),
        Err(libc::ENOENT)
    );
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
}

// ---------------------------------------------------------------------------
// F6: repositories do not see each other's mounts
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn two_repositories_with_the_same_mount_point_stay_independent() {
    let mut fx = Fx::new();
    let (r1, r2) = (fx.repo(), fx.repo());
    let a = fx.host(&[("same.txt", b"A"), ("only_a.txt", b"a")]);
    let b = fx.host(&[("same.txt", b"BB"), ("only_b.txt", b"b")]);
    let mount_a = mount_stdio(r1, "/", &a, UIO_MOUNT_TOP);
    let mount_b = mount_stdio(r2, "/", &b, UIO_MOUNT_TOP);
    assert!(!mount_a.is_null() && !mount_b.is_null());
    let (dir_a, dir_b) = (open_dir(r1, "/"), open_dir(r2, "/"));

    assert_eq!(read(dir_a, "same.txt").unwrap(), b"A");
    assert_eq!(read(dir_b, "same.txt").unwrap(), b"BB");
    assert_eq!(read(dir_a, "only_b.txt"), Err(libc::ENOENT));
    assert_eq!(read(dir_b, "only_a.txt"), Err(libc::ENOENT));
    assert_eq!(locate(dir_a, "same.txt").unwrap().0, mount_a);
    assert_eq!(locate(dir_b, "same.txt").unwrap().0, mount_b);
    assert_eq!(list_names(dir_a, "/"), ["only_a.txt", "same.txt"]);
    assert_eq!(list_names(dir_b, "/"), ["only_b.txt", "same.txt"]);
    let name = cstr("same.txt");
    unsafe {
        let mut info: stat = std::mem::zeroed();
        assert_eq!(uio_stat(dir_a, name.as_ptr(), &mut info), 0);
        assert_eq!(info.st_size, 1);
        assert_eq!(uio_stat(dir_b, name.as_ptr(), &mut info), 0);
        assert_eq!(info.st_size, 2);
    }

    assert_eq!(unsafe { uio_unmountDir(mount_a) }, 0);
    assert_eq!(read(dir_a, "same.txt"), Err(libc::ENOENT));
    assert_eq!(read(dir_b, "same.txt").unwrap(), b"BB");
}

// ---------------------------------------------------------------------------
// F7: opening a directory verifies it, like uio_verifyPath
// ---------------------------------------------------------------------------

fn tree_stack(fx: &mut Fx) -> (*mut uio_Repository, PathBuf) {
    let repo = fx.repo();
    let host = fx.host(&[
        ("file.txt", b"f"),
        ("sub/inner.txt", b"i"),
        ("x", b"file named x"),
    ]);
    let zips = fx.host(&[]);
    write_zip(
        &zips.join("pack.zip"),
        &[("a/b/c.txt", b"c"), ("f", b"f"), ("x/y.txt", b"y")],
    );
    let deep = fx.host(&[]);
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());
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
    assert!(!mount_stdio(repo, "/deep/er", &deep, UIO_MOUNT_TOP).is_null());
    (repo, host)
}

#[test]
#[serial]
fn opening_a_directory_that_no_mount_has_is_enoent() {
    let mut fx = Fx::new();
    let (repo, host) = tree_stack(&mut fx);
    let host_text = host.to_str().unwrap().to_owned();
    for path in ["/missing", "/sub/nope", "/a/zz", host_text.as_str()] {
        assert_eq!(
            dir_or_errno(open_dir(repo, path)),
            Err(libc::ENOENT),
            "{path}"
        );
    }
    let root = open_dir(repo, "/");
    for path in ["missing", "sub/nope", "a/b/zz"] {
        assert_eq!(
            dir_or_errno(open_dir_relative(root, path)),
            Err(libc::ENOENT),
            "{path}"
        );
    }
}

#[test]
#[serial]
fn opening_a_file_as_a_directory_is_enotdir() {
    let mut fx = Fx::new();
    let (repo, _host) = tree_stack(&mut fx);
    for path in [
        "/file.txt",
        "/sub/inner.txt",
        "/a/b/c.txt",
        "/f/x",
        "/file.txt/x",
    ] {
        assert_eq!(
            dir_or_errno(open_dir(repo, path)),
            Err(libc::ENOTDIR),
            "{path}"
        );
    }
    // The host file "x" is on top of the archive directory "x/".
    assert_eq!(dir_or_errno(open_dir(repo, "/x")), Err(libc::ENOTDIR));
}

#[test]
#[serial]
fn directories_of_hosts_archives_and_mount_points_open() {
    let mut fx = Fx::new();
    let (repo, _host) = tree_stack(&mut fx);
    for path in [
        "/",
        "/sub",
        "/a",
        "/a/b",
        "/deep",
        "/deep/er",
        ".",
        "..",
        "/sub/../a/./b",
    ] {
        assert!(
            !open_dir(repo, path).is_null(),
            "{path}: errno {}",
            errno_value()
        );
    }
    let root = open_dir(repo, "/");
    let a = open_dir_relative(root, "a");
    assert!(!a.is_null());
    assert!(!open_dir_relative(a, "b").is_null());
    assert!(!open_dir_relative(a, "../sub").is_null());
    assert!(!open_dir_relative(a, "/deep").is_null());
}

#[test]
#[serial]
fn a_missing_directory_is_not_stat_able_either() {
    let mut fx = Fx::new();
    let (repo, _host) = tree_stack(&mut fx);
    let root = open_dir(repo, "/");
    let missing = cstr("a/zz");
    unsafe {
        let mut info: stat = std::mem::zeroed();
        assert_eq!(uio_stat(root, missing.as_ptr(), &mut info), -1);
        assert_eq!(errno_value(), libc::ENOENT);
    }
}

// ---------------------------------------------------------------------------
// F8: the call patterns of gfxload.c, options.c and dir_prep.rs
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn the_ani_loader_pattern_with_a_dot_directory() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let content = fx.host(&[]);
    write_zip(&content.join("x.ani"), &[("frame1.png", b"png")]);
    let zip_bytes = std::fs::read(content.join("x.ani")).unwrap();
    assert!(!mount_stdio(repo, "/", &content, RDONLY_TOP).is_null());

    let ani_dir = open_dir(repo, ".");
    assert!(!ani_dir.is_null(), "errno {}", errno_value());
    assert!(mount_zip(repo, ".", ani_dir, "x.ani", RDONLY_TOP).is_null());
    assert_eq!(
        errno_value(),
        libc::EINVAL,
        "'.' is not a valid mount point"
    );

    let (name, mode) = (cstr("x.ani"), cstr("r"));
    unsafe {
        let stream = uio_fopen(ani_dir, name.as_ptr(), mode.as_ptr());
        assert!(!stream.is_null(), "errno {}", errno_value());
        let mut bytes = vec![0u8; zip_bytes.len() + 8];
        let got = uio_fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), stream);
        assert_eq!(&bytes[..got], &zip_bytes[..]);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn the_ani_loader_pattern_with_a_subdirectory() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let content = fx.host(&[]);
    write_zip(&content.join("ani/x.ani"), &[("frame1.png", b"png")]);
    assert!(!mount_stdio(repo, "/", &content, RDONLY_TOP).is_null());

    let ani_dir = open_dir(repo, "ani");
    assert!(!ani_dir.is_null(), "errno {}", errno_value());
    let mount = mount_zip(repo, "ani", ani_dir, "x.ani", RDONLY_TOP);
    assert!(!mount.is_null(), "errno {}", errno_value());
    let root = open_dir(repo, "/");
    assert_eq!(read(root, "ani/frame1.png").unwrap(), b"png");
    assert_eq!(unsafe { uio_unmountDir(mount) }, 0);
    assert_eq!(read(root, "ani/frame1.png"), Err(libc::ENOENT));
}

#[test]
#[serial]
fn the_font_loader_pattern_mounts_at_the_full_resource_name() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let content = fx.host(&[]);
    write_zip(&content.join("fonts/f.fon"), &[("0041.png", b"A")]);
    assert!(!mount_stdio(repo, "/", &content, RDONLY_TOP).is_null());
    let font_dir = open_dir(repo, "fonts");
    assert!(!font_dir.is_null());
    assert!(!mount_zip(repo, "fonts/f.fon", font_dir, "f.fon", RDONLY_TOP).is_null());
    unsafe { uio_closeDir(font_dir) };
    let root = open_dir(repo, "/");
    assert_eq!(read(root, "fonts/f.fon/0041.png").unwrap(), b"A");
}

#[test]
#[serial]
fn the_content_mount_pattern_opens_packages_only_when_present() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let content = fx.host(&[("version", b"0.8")]);
    let content_mount = mount_stdio(repo, "/", &content, RDONLY_TOP);
    assert!(!content_mount.is_null());
    assert!(!open_dir(repo, "/").is_null());
    assert_eq!(dir_or_errno(open_dir(repo, "/packages")), Err(libc::ENOENT));

    write_zip(&content.join("packages/base.uqm"), &[("res/a.txt", b"pkg")]);
    let packages = open_dir(repo, "/packages");
    assert!(!packages.is_null(), "errno {}", errno_value());
    let (point, name) = (cstr("/"), cstr("base.uqm"));
    let below = unsafe {
        uio_mountDir(
            repo,
            point.as_ptr(),
            UIO_FSTYPE_ZIP,
            packages,
            name.as_ptr(),
            c"/".as_ptr(),
            ptr::null_mut(),
            UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
            content_mount,
        )
    };
    assert!(!below.is_null(), "errno {}", errno_value());
    let root = open_dir(repo, "/");
    assert_eq!(read(root, "res/a.txt").unwrap(), b"pkg");
    assert_eq!(read(root, "version").unwrap(), b"0.8");
}

// ---------------------------------------------------------------------------
// F9: byte names are never read as their replacement-character spelling
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn bytes_that_are_not_utf8_never_alias_a_replacement_character_name() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("caf\u{fffd}.txt", b"host")]);
    let zips = fx.host(&[]);
    write_zip(
        &zips.join("pack.zip"),
        &[
            ("caf\u{fffd}/in.txt", b"zip"),
            ("caf\u{fffd}.zip.txt", b"z"),
        ],
    );
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());
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
    let root = open_dir(repo, "/");

    assert_eq!(read(root, "caf\u{fffd}.txt").unwrap(), b"host");
    let raw_file = CString::new(&b"caf\xff.txt"[..]).unwrap();
    let raw_dir = CString::new(&b"caf\xff"[..]).unwrap();
    unsafe {
        let mut mount: *mut uio_MountHandle = ptr::null_mut();
        let mut out: *mut c_char = ptr::null_mut();
        assert_eq!(
            uio_get_file_location(root, raw_file.as_ptr(), 0, &mut mount, &mut out),
            -1
        );
        assert_eq!(errno_value(), libc::ENOENT);
        assert!(uio_openDirRelative(root, raw_dir.as_ptr(), 0).is_null());
        assert_eq!(errno_value(), libc::ENOENT);
        assert!(uio_getStdioAccess(root, raw_file.as_ptr(), 0, ptr::null_mut()).is_null());
        assert_eq!(errno_value(), libc::ENOENT);
    }
    assert!(!open_dir_relative(root, "caf\u{fffd}").is_null());
}

// ---------------------------------------------------------------------------
// F10: failures of the outside world are errors, not aborts
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn a_source_that_is_not_a_zip_archive_fails_with_eio() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("bogus.zip", b"this is not a zip archive")]);
    assert!(!mount_stdio(repo, "/", &host, RDONLY_TOP).is_null());
    let root = open_dir(repo, "/");
    assert!(mount_zip(repo, "/z", root, "bogus.zip", RDONLY_TOP).is_null());
    assert_eq!(errno_value(), libc::EIO);
}

#[test]
#[serial]
fn a_source_the_host_will_not_let_us_read_reports_the_host_error() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[]);
    write_zip(&host.join("locked.zip"), &[("f.txt", b"f")]);
    std::fs::set_permissions(
        host.join("locked.zip"),
        std::fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    assert!(!mount_stdio(repo, "/", &host, RDONLY_TOP).is_null());
    let root = open_dir(repo, "/");
    let mounted = mount_zip(repo, "/z", root, "locked.zip", RDONLY_TOP);
    let errno = errno_value();
    std::fs::set_permissions(
        host.join("locked.zip"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(mounted.is_null());
    assert_eq!(errno, libc::EACCES);
}

// ---------------------------------------------------------------------------
// F11: uio_mountDir's argument contract
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn mount_dir_refuses_inconsistent_sources_types_and_mount_points() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("p.zip", b"x")]);
    write_zip(&host.join("p.zip"), &[("f", b"f")]);
    assert!(!mount_stdio(repo, "/", &host, RDONLY_TOP).is_null());
    let root = open_dir(repo, "/");
    let (point, name, slash) = (cstr("/z"), cstr("p.zip"), cstr("/"));
    let call = |fs_type: c_int, point: &CStr, dir, name: *const c_char| unsafe {
        uio_mountDir(
            repo,
            point.as_ptr(),
            fs_type,
            dir,
            name,
            slash.as_ptr(),
            ptr::null_mut(),
            RDONLY_TOP,
            ptr::null_mut(),
        )
    };

    assert!(call(UIO_FSTYPE_ZIP, &point, root, ptr::null()).is_null());
    assert_eq!(errno_value(), libc::EINVAL, "sourceDir without sourcePath");
    assert!(call(UIO_FSTYPE_ZIP, &point, ptr::null_mut(), name.as_ptr()).is_null());
    assert_eq!(errno_value(), libc::EINVAL, "sourcePath without sourceDir");
    assert!(call(UIO_FSTYPE_STDIO, &point, root, name.as_ptr()).is_null());
    assert_eq!(
        errno_value(),
        libc::EINVAL,
        "stdio mounts take no source file"
    );
    assert!(call(99, &point, root, name.as_ptr()).is_null());
    assert_eq!(errno_value(), libc::ENODEV);
    for bad in [".", "a//b", "a/../b", "/z/./y"] {
        let bad_point = cstr(bad);
        assert!(call(UIO_FSTYPE_ZIP, &bad_point, root, name.as_ptr()).is_null());
        assert_eq!(errno_value(), libc::EINVAL, "{bad}");
    }
    for good in ["/", "addons", "/a/b"] {
        let good_point = cstr(good);
        assert!(
            !call(UIO_FSTYPE_ZIP, &good_point, root, name.as_ptr()).is_null(),
            "{good}"
        );
    }
}

// ---------------------------------------------------------------------------
// Properties: archive layouts and byte names against small models
// ---------------------------------------------------------------------------

/// Where `query` lands in an archive that holds exactly the files `files`
/// (no file is a directory of another), by the rules of `uio_verifyPath`.
#[derive(Debug, PartialEq)]
enum Expected {
    Directory,
    File,
    ThroughAFile,
    Missing,
}

fn expected_in_layout(files: &[Vec<&'static str>], query: &[&'static str]) -> Expected {
    if query.is_empty() {
        return Expected::Directory;
    }
    if files.iter().any(|file| file.as_slice() == query) {
        return Expected::File;
    }
    if files
        .iter()
        .any(|file| file.len() > query.len() && file.starts_with(query))
    {
        return Expected::Directory;
    }
    let through_a_file =
        (1..query.len()).any(|len| files.iter().any(|file| file.as_slice() == &query[..len]));
    if through_a_file {
        Expected::ThroughAFile
    } else {
        Expected::Missing
    }
}

fn layout_names() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("a"), Just("b"), Just("c")]
}

/// A set of archive files in which no file lies below another.
fn layout() -> impl Strategy<Value = Vec<Vec<&'static str>>> {
    prop::collection::btree_set(prop::collection::vec(layout_names(), 1..4), 0..6).prop_map(|all| {
        let all: Vec<Vec<&'static str>> = all.into_iter().collect();
        all.iter()
            .filter(|file| {
                !all.iter()
                    .any(|other| other.len() < file.len() && file.starts_with(other))
            })
            .cloned()
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Directories open, files do not, implicit directories of the archive
    /// exist, and location answers agree, for any layout and any query.
    #[test]
    #[serial]
    fn archive_layouts_resolve_like_the_model(
        files in layout(),
        query in prop::collection::vec(layout_names(), 0..5),
    ) {
        let mut fx = Fx::new();
        let repo = fx.repo();
        let host = fx.host(&[]);
        let entries: Vec<(String, &[u8])> =
            files.iter().map(|file| (file.join("/"), &b"x"[..])).collect();
        let entry_refs: Vec<(&str, &[u8])> =
            entries.iter().map(|(name, data)| (name.as_str(), *data)).collect();
        write_zip(&host.join("pack.zip"), &entry_refs);
        prop_assert!(!mount_stdio(repo, "/src", &host, RDONLY_TOP).is_null());
        let src = open_dir(repo, "/src");
        prop_assert!(!mount_zip(repo, "/", src, "pack.zip", UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY).is_null());
        let root = open_dir(repo, "/");

        let text = query.join("/");
        let opened = dir_or_errno(open_dir_relative(root, &text));
        let located = locate(root, &text).map(|(_, path)| path);
        match expected_in_layout(&files, &query) {
            Expected::Directory => {
                prop_assert!(opened.is_ok(), "{text:?}");
                if !query.is_empty() {
                    prop_assert_eq!(located, Err(libc::EISDIR));
                }
            }
            Expected::File => {
                prop_assert_eq!(opened, Err(libc::ENOTDIR));
                prop_assert_eq!(located, Ok(Path::new("/").join(&text)));
            }
            Expected::ThroughAFile => {
                prop_assert_eq!(opened, Err(libc::ENOTDIR));
                prop_assert_eq!(located, Err(libc::ENOTDIR));
            }
            Expected::Missing => {
                prop_assert_eq!(opened, Err(libc::ENOENT));
                prop_assert_eq!(located, Err(libc::ENOENT));
            }
        }
    }

    /// An archive entry is named by Unicode, so bytes find it exactly when
    /// they are that entry's UTF-8 and never through a lossy reading.
    #[test]
    #[serial]
    fn byte_names_find_an_archive_entry_only_by_its_own_bytes(
        raw in prop::collection::vec(
            (1u8..=255).prop_filter("no separators or dots", |b| !matches!(b, b'.' | b'/' | b'\\')),
            1..10,
        ),
    ) {
        let mut fx = Fx::new();
        let repo = fx.repo();
        let host = fx.host(&[]);
        let lossy = String::from_utf8_lossy(&raw).into_owned();
        write_zip(&host.join("pack.zip"), &[(lossy.as_str(), b"entry")]);
        prop_assert!(!mount_stdio(repo, "/src", &host, RDONLY_TOP).is_null());
        let src = open_dir(repo, "/src");
        prop_assert!(!mount_zip(repo, "/", src, "pack.zip", UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY).is_null());
        let root = open_dir(repo, "/");

        let name = CString::new(raw.clone()).unwrap();
        let mut mount: *mut uio_MountHandle = ptr::null_mut();
        let mut out: *mut c_char = ptr::null_mut();
        let rc = unsafe { uio_get_file_location(root, name.as_ptr(), 0, &mut mount, &mut out) };
        if std::str::from_utf8(&raw).is_ok() {
            prop_assert_eq!(rc, 0);
            let located = unsafe { CStr::from_ptr(out) }.to_bytes().to_vec();
            unsafe { libc::free(out.cast()) };
            let mut expected = b"/".to_vec();
            expected.extend_from_slice(&raw);
            prop_assert_eq!(located, expected);
        } else {
            prop_assert_eq!(rc, -1);
            prop_assert_eq!(errno_value(), libc::ENOENT);
            prop_assert!(out.is_null());
        }
    }
}

/// Real host names that are not UTF-8. The Apple file systems refuse to hold
/// them (EILSEQ), so there this only checks the refusal; the flow that follows
/// has run only where the file system takes such names, which this volume does
/// not, and is the Linux gap.
#[test]
#[serial]
fn host_names_that_are_not_utf8_resolve_by_their_bytes() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[]);
    let dir_name = std::ffi::OsStr::from_bytes(b"d\xff");
    if let Err(error) = std::fs::create_dir(host.join(dir_name)) {
        assert_eq!(error.raw_os_error(), Some(libc::EILSEQ), "{error}");
        if cfg!(not(target_vendor = "apple")) {
            panic!("only Apple file systems refuse byte names");
        }
        return;
    }
    std::fs::write(host.join(dir_name).join("f.txt"), b"bytes").unwrap();
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());
    let root = open_dir(repo, "/");
    let sub = CString::new(&b"d\xff"[..]).unwrap();
    let opened = unsafe { uio_openDirRelative(root, sub.as_ptr(), 0) };
    assert!(!opened.is_null(), "errno {}", errno_value());
    assert_eq!(read(opened, "f.txt").unwrap(), b"bytes");
    let (_, path) = locate(opened, "f.txt").unwrap();
    assert_eq!(path, host.join(dir_name).join("f.txt"));
}
