//! Reads through a mount stack that mixes stdio directories and archives.
//! Each mount that covers a path is consulted in priority order, and a
//! mount that lacks the file lets the lookup continue to the next one.

use std::ffi::CString;
use std::io::Write;
use std::path::{Path, PathBuf};

use serial_test::serial;
use tempfile::TempDir;

use super::*;

pub(super) struct Stack {
    pub(super) repo: *mut uio_Repository,
    pub(super) root: *mut uio_DirHandle,
    _dirs: Vec<TempDir>,
}

impl Stack {
    pub(super) fn new() -> Self {
        if let Ok(mut registry) = get_mount_registry().lock() {
            registry.clear();
        }
        let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
        let root = Box::into_raw(Box::new(uio_DirHandle {
            path: PathBuf::from("/"),
            virtual_path: PathBuf::from("/"),
            refcount: std::sync::atomic::AtomicI32::new(1),
            repository: repo,
            root_end: PathBuf::from("/"),
        }));
        Stack {
            repo,
            root,
            _dirs: Vec::new(),
        }
    }

    pub(super) fn mount(
        &self,
        fs_type: c_int,
        flags: c_int,
        host: &Path,
        relative: *mut uio_MountHandle,
    ) {
        let handle = register_mount(
            self.repo,
            Path::new("/"),
            host.to_path_buf(),
            fs_type,
            flags,
            relative,
            true,
        );
        assert!(!handle.is_null(), "mount of {host:?} failed");
    }

    pub(super) fn mount_dir(&mut self, files: &[(&str, &[u8])], flags: c_int) -> PathBuf {
        let dir = TempDir::new().unwrap();
        for (name, data) in files {
            std::fs::write(dir.path().join(name), data).unwrap();
        }
        let path = dir.path().to_path_buf();
        self._dirs.push(dir);
        self.mount(UIO_FSTYPE_STDIO, flags, &path, ptr::null_mut());
        path
    }

    pub(super) fn mount_zip(&mut self, files: &[(&str, &[u8])], flags: c_int, relative: usize) {
        let dir = TempDir::new().unwrap();
        let zip_path = dir.path().join("pack.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        self._dirs.push(dir);
        let relative_handle = if relative == 0 {
            ptr::null_mut()
        } else {
            let registry = get_mount_registry().lock().unwrap();
            let entry = registry.iter().find(|m| m.id == relative).unwrap();
            entry.handle_ptr as *mut uio_MountHandle
        };
        self.mount(UIO_FSTYPE_ZIP, flags, &zip_path, relative_handle);
    }

    pub(super) fn first_mount_id(&self) -> usize {
        get_mount_registry().lock().unwrap()[0].id
    }

    pub(super) fn read(&self, name: &str) -> Option<Vec<u8>> {
        use std::io::Read;
        let name = CString::new(name).unwrap();
        unsafe {
            let handle = uio_open(self.root, name.as_ptr(), 0, 0);
            if handle.is_null() {
                return None;
            }
            let mut bytes = Vec::new();
            (*handle).lock().unwrap().read_to_end(&mut bytes).unwrap();
            uio_close(handle);
            Some(bytes)
        }
    }
}

/// A fresh repository whose only mount is `host`, on top at "/", and a handle
/// on "/" opened the way the game opens its root directory. Every file the
/// handle reaches comes through the mount registry, like in play.
pub(super) fn mount_host_root(host: &Path) -> (*mut uio_Repository, *mut uio_DirHandle) {
    if let Ok(mut registry) = get_mount_registry().lock() {
        registry.clear();
    }
    let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
    let mount = register_mount(
        repo,
        Path::new("/"),
        host.to_path_buf(),
        UIO_FSTYPE_STDIO,
        UIO_MOUNT_TOP,
        ptr::null_mut(),
        true,
    );
    assert!(!mount.is_null(), "mount of {host:?} failed");
    let root = Box::into_raw(Box::new(uio_DirHandle {
        path: host.to_path_buf(),
        virtual_path: PathBuf::from("/"),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: repo,
        root_end: PathBuf::from("/"),
    }));
    (repo, root)
}

/// Frees what `mount_host_root` made and drops its mount.
///
/// # Safety
///
/// Both pointers must come from one `mount_host_root` call and not be used again.
pub(super) unsafe fn release_host_root(repo: *mut uio_Repository, root: *mut uio_DirHandle) {
    drop(Box::from_raw(root));
    drop(Box::from_raw(repo));
    if let Ok(mut registry) = get_mount_registry().lock() {
        registry.clear();
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.root);
            let _ = Box::from_raw(self.repo);
        }
        if let Ok(mut registry) = get_mount_registry().lock() {
            registry.clear();
        }
    }
}

#[test]
#[serial]
fn a_file_missing_from_a_lower_priority_archive_is_read_from_the_directory_above() {
    let mut stack = Stack::new();
    stack.mount_dir(
        &[("loose.txt", b"loose data")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
    );
    let top = stack.first_mount_id();
    stack.mount_zip(
        &[("packed.txt", b"packed data")],
        UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
        top,
    );

    assert_eq!(stack.read("loose.txt").as_deref(), Some(&b"loose data"[..]));
    assert_eq!(
        stack.read("packed.txt").as_deref(),
        Some(&b"packed data"[..])
    );
}

#[test]
#[serial]
fn a_file_missing_from_the_top_archive_is_read_from_the_archive_below() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("upper.txt", b"upper")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let top = stack.first_mount_id();
    stack.mount_zip(
        &[("lower.txt", b"lower")],
        UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
        top,
    );

    assert_eq!(stack.read("upper.txt").as_deref(), Some(&b"upper"[..]));
    assert_eq!(stack.read("lower.txt").as_deref(), Some(&b"lower"[..]));
}

#[test]
#[serial]
fn the_highest_priority_mount_wins_when_several_hold_the_same_file() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("shared.txt", b"from archive")],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let archive = stack.first_mount_id();
    stack.mount_dir_above(&[("shared.txt", b"from directory")], archive);

    assert_eq!(
        stack.read("shared.txt").as_deref(),
        Some(&b"from directory"[..])
    );
}

#[test]
#[serial]
fn a_file_found_in_no_mount_cannot_be_opened() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("here.txt", b"x")], UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    let top = stack.first_mount_id();
    stack.mount_zip(
        &[("there.txt", b"y")],
        UIO_MOUNT_BELOW | UIO_MOUNT_RDONLY,
        top,
    );

    assert_eq!(stack.read("nowhere.txt"), None);
}

fn errno_of_failure() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// A root handle over a repository whose only mount sits at "/game", plus a
/// host directory that no mount covers.
struct UncoveredHost {
    stack: Stack,
    outside: TempDir,
}

impl UncoveredHost {
    fn new() -> Self {
        let stack = Stack::new();
        let game = TempDir::new().unwrap();
        let mount = register_mount(
            stack.repo,
            Path::new("/game"),
            game.path().to_path_buf(),
            UIO_FSTYPE_STDIO,
            UIO_MOUNT_TOP,
            ptr::null_mut(),
            true,
        );
        assert!(!mount.is_null());
        let mut stack = stack;
        stack._dirs.push(game);
        UncoveredHost {
            stack,
            outside: TempDir::new().unwrap(),
        }
    }

    fn outside(&self, name: &str) -> CString {
        CString::new(self.outside.path().join(name).to_str().unwrap()).unwrap()
    }
}

#[test]
#[serial]
fn mkdir_where_no_mount_covers_the_path_does_not_create_a_host_directory() {
    let world = UncoveredHost::new();
    let target = world.outside("made");
    set_errno(0);
    assert_eq!(
        unsafe { uio_mkdir(world.stack.root, target.as_ptr(), 0o755) },
        -1
    );
    assert_eq!(errno_of_failure(), libc::ENOENT);
    assert!(!world.outside.path().join("made").exists());
}

#[test]
#[serial]
fn rmdir_where_no_mount_covers_the_path_leaves_the_host_directory() {
    let world = UncoveredHost::new();
    std::fs::create_dir(world.outside.path().join("kept")).unwrap();
    let target = world.outside("kept");
    set_errno(0);
    assert_eq!(unsafe { uio_rmdir(world.stack.root, target.as_ptr()) }, -1);
    assert_eq!(errno_of_failure(), libc::ENOENT);
    assert!(world.outside.path().join("kept").is_dir());
}

#[test]
#[serial]
fn unlink_where_no_mount_covers_the_path_leaves_the_host_file() {
    let world = UncoveredHost::new();
    std::fs::write(world.outside.path().join("kept.txt"), b"keep").unwrap();
    let target = world.outside("kept.txt");
    set_errno(0);
    assert_eq!(unsafe { uio_unlink(world.stack.root, target.as_ptr()) }, -1);
    assert_eq!(errno_of_failure(), libc::ENOENT);
    assert_eq!(
        std::fs::read(world.outside.path().join("kept.txt")).unwrap(),
        b"keep"
    );
}

#[test]
#[serial]
fn creating_a_file_where_no_mount_covers_the_path_does_not_touch_the_host() {
    let world = UncoveredHost::new();
    let target = world.outside("created.txt");
    set_errno(0);
    let handle = unsafe { uio_open(world.stack.root, target.as_ptr(), O_WRONLY | O_CREAT, 0o644) };
    assert!(handle.is_null());
    assert_eq!(errno_of_failure(), libc::ENOENT);
    assert!(!world.outside.path().join("created.txt").exists());
}

#[test]
#[serial]
fn reading_where_no_mount_covers_the_path_does_not_reach_the_host() {
    let world = UncoveredHost::new();
    std::fs::write(world.outside.path().join("host.txt"), b"HOST").unwrap();
    let target = world.outside("host.txt");
    set_errno(0);
    let handle = unsafe { uio_open(world.stack.root, target.as_ptr(), 0, 0) };
    assert!(handle.is_null());
    assert_eq!(errno_of_failure(), libc::ENOENT);
}

#[test]
#[serial]
fn rename_in_a_nested_directory_handle_works_on_the_mounted_files() {
    let mut stack = Stack::new();
    let host = stack.mount_dir(&[], UIO_MOUNT_TOP);
    std::fs::create_dir(host.join("d")).unwrap();
    std::fs::write(host.join("d").join("a.txt"), b"moved").unwrap();
    let nested = unsafe { uio_openDirRelative(stack.root, c"d".as_ptr(), 0) };
    assert!(!nested.is_null());

    let renamed = unsafe { uio_rename(nested, c"a.txt".as_ptr(), nested, c"b.txt".as_ptr()) };
    assert_eq!(renamed, 0, "errno {}", errno_of_failure());
    assert!(!host.join("d").join("a.txt").exists());
    assert_eq!(
        std::fs::read(host.join("d").join("b.txt")).unwrap(),
        b"moved"
    );
    unsafe { uio_closeDir(nested) };
}

impl Stack {
    pub(super) fn mount_dir_above(&mut self, files: &[(&str, &[u8])], relative: usize) {
        let dir = TempDir::new().unwrap();
        for (name, data) in files {
            std::fs::write(dir.path().join(name), data).unwrap();
        }
        let path = dir.path().to_path_buf();
        self._dirs.push(dir);
        let relative_handle = {
            let registry = get_mount_registry().lock().unwrap();
            let entry = registry.iter().find(|m| m.id == relative).unwrap();
            entry.handle_ptr as *mut uio_MountHandle
        };
        self.mount(
            UIO_FSTYPE_STDIO,
            UIO_MOUNT_ABOVE | UIO_MOUNT_RDONLY,
            &path,
            relative_handle,
        );
    }
}
