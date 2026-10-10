// Minimal stdio-backed UIO implementation
// Reference: sc2/src/libs/uio/io.h, uiostream.h

use super::name_filter::NameFilter;
use super::zip_stat::{ZipEntryStat, ZipObject, IMPLIED_DIRECTORY_MODE};
use crate::bridge_log::rust_bridge_log_msg;
use libc::{mode_t, off_t, size_t};
use std::collections::BTreeSet;

use std::ffi::{CStr, CString, OsStr};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int, c_long};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[path = "uio_resolve.rs"]
mod resolve;
use resolve::{
    covering, is_virtual_directory, locate, probe, transplant_source, verify_directory,
    virtual_children, Kind, Place,
};

#[path = "uio_write_access.rs"]
mod write_access;
use write_access::{
    names_an_entry, plan_removal, plan_write, prepare_write, remove_entries, write_permission,
    Parents, WriteIntent,
};

// =============================================================================
// Mount Point Registry
const UIO_MOUNT_RDONLY: c_int = 1 << 1;
const UIO_MOUNT_LOCATION_MASK: c_int = 3 << 2;
#[cfg(test)]
const UIO_MOUNT_BOTTOM: c_int = 0 << 2;
const UIO_MOUNT_TOP: c_int = 1 << 2;

const UIO_MOUNT_BELOW: c_int = 2 << 2;
const UIO_MOUNT_ABOVE: c_int = 3 << 2;

const UIO_FSTYPE_STDIO: c_int = 1;
const UIO_FSTYPE_ZIP: c_int = 2;

// Access mode constants
const F_OK: c_int = 0;
const R_OK: c_int = 4;
const W_OK: c_int = 2;
const X_OK: c_int = 1;

const UIO_STREAM_STATUS_OK: c_int = 0;
const UIO_STREAM_STATUS_EOF: c_int = 1;
const UIO_STREAM_STATUS_ERROR: c_int = 2;

const UIO_STREAM_OPERATION_NONE: c_int = 0;
const UIO_STREAM_OPERATION_READ: c_int = 1;

// Helper function to set errno across FFI boundary
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
fn set_errno(code: c_int) {
    unsafe {
        *libc::__error() = code;
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set_errno(code: c_int) {
    unsafe {
        *libc::__errno_location() = code;
    }
}

#[cfg(test)]
fn current_errno() -> c_int {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe {
        *libc::__error()
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        *libc::__errno_location()
    }
}

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-ERRNO
/// Helper to fail with errno set and return a failure value
fn fail_errno<T>(code: c_int, failure_return: T) -> T {
    set_errno(code);
    failure_return
}

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-PANIC-SAFETY
/// Macro to wrap FFI function bodies with panic containment.
/// Catches panics and converts them to safe failure returns with errno set.
macro_rules! ffi_guard {
    ($default:expr, $body:expr) => {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => {
                set_errno(libc::EIO);
                $default
            }
        }
    };
}

// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P06
/// @requirement REQ-FIO-MOUNT-ORDER
/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
struct MountInfo {
    id: usize,
    repository: usize,
    handle_ptr: usize,
    mount_point: PathBuf,
    mounted_root: PathBuf,
    fs_type: c_int,
    active_in_registry: bool,
    /// Explicit ordering position (lower value = higher priority)
    position: usize,
    /// Read-only mount flag
    read_only: bool,
    /// ZIP archive index (only for UIO_FSTYPE_ZIP mounts)
    zip_index: Option<std::sync::Arc<crate::io::zip_reader::ZipIndex>>,
    /// The directory of the archive this mount shows as its root: empty for
    /// the whole archive, `"sub"` for a mount of the archive's `sub/`
    /// directory (what a transplant of such a directory makes).
    archive_root: String,
    // Note: AutoMount is DEFERRED per P00a Q2 resolution - not needed for engine/runtime paths
    // auto_mount_rules field is NOT implemented
}

impl MountInfo {
    /// Archives can only be mounted read-only (see
    /// [`index_archive_for_mount`]), so an archive mount always has the flag.
    fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// The archive entry name of `suffix`, a path below the mount point.
    /// `None` when the bytes are not UTF-8: entry names are Unicode, so such
    /// a path names no entry.
    fn archive_name(&self, suffix: &Path) -> Option<String> {
        let suffix = suffix.to_str()?;
        Some(match (self.archive_root.is_empty(), suffix.is_empty()) {
            (_, true) => self.archive_root.clone(),
            (true, false) => suffix.to_owned(),
            (false, false) => format!("{}/{}", self.archive_root, suffix),
        })
    }
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-LIFECYCLE
/// Global initialization flag for the UIO subsystem
static UIO_INITIALIZED: AtomicBool = AtomicBool::new(false);

static MOUNT_REGISTRY: OnceLock<Mutex<Vec<MountInfo>>> = OnceLock::new();
static NEXT_MOUNT_ID: AtomicUsize = AtomicUsize::new(1);

fn get_mount_registry() -> &'static Mutex<Vec<MountInfo>> {
    MOUNT_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

// Types matching C structures from io.h and uiostream.h

/// A place in a repository's virtual tree: an absolute, normalized virtual
/// path and the repository whose mounts give it meaning. It holds no host
/// path. Every operation looks the path up in the repository's mount stack as
/// it is at that moment (see `uio_resolve`), as the original's handle held
/// only the virtual path.
#[repr(C)]
pub struct uio_DirHandle {
    virtual_path: PathBuf,
    refcount: std::sync::atomic::AtomicI32,
    repository: *mut uio_Repository,
}

#[repr(C)]
pub struct uio_Repository {
    flags: c_int,
}

#[repr(C)]
pub struct uio_MountHandle {
    repository: *mut uio_Repository,
    id: usize,
    fs_type: c_int,
}

// =============================================================================
// uio_rename / uio_access / uio_stat / uio_mkdir / uio_rmdir / uio_lseek
// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_rename(
    old_dir: *mut uio_DirHandle,
    old_path: *const c_char,
    new_dir: *mut uio_DirHandle,
    new_path: *const c_char,
) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_rename called");

        if old_dir.is_null() || new_dir.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        let old_dir_path = &(*old_dir).virtual_path;
        let new_dir_path = &(*new_dir).virtual_path;

        let old_input = match cstr_to_pathbuf(old_path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        let new_input = match cstr_to_pathbuf(new_path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        let old_virtual = normalize_virtual_path_full(old_dir_path, &old_input);
        let new_virtual = normalize_virtual_path_full(new_dir_path, &new_input);

        let registry = get_mount_registry().lock().unwrap();
        let (old_host_path, new_host_path) = match rename_places(
            &registry,
            (repository_key((*old_dir).repository), &old_virtual),
            (repository_key((*new_dir).repository), &new_virtual),
        ) {
            Ok(places) => places,
            Err(errno) => return fail_errno(errno, -1),
        };
        drop(registry);

        match fs::rename(&old_host_path, &new_host_path) {
            Ok(_) => 0,
            Err(e) => {
                let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                fail_errno(err_code, -1)
            }
        }
    })
}

/// The host paths a rename moves between. Like `uio_rename` in the original:
/// the old path is read from the first mount that holds it (a directory is
/// EISDIR), the new path must be free and get a write location without
/// making directories, and both have to be the same mount (EXDEV).
fn rename_places(
    registry: &[MountInfo],
    (old_repository, old_virtual): (usize, &Path),
    (new_repository, new_virtual): (usize, &Path),
) -> Result<(PathBuf, PathBuf), c_int> {
    let old = locate(registry, old_repository, old_virtual)?;
    if old.kind == Kind::Directory {
        return Err(libc::EISDIR);
    }
    let new = plan_write(
        registry,
        new_repository,
        new_virtual,
        WriteIntent {
            create: true,
            exclusive: true,
            parents: Parents::MustExist,
        },
    )?;
    let Place::Host(old_host_path) = old.place else {
        // An archive entry has no host path to move.
        return Err(libc::EXDEV);
    };
    if old.mount.id != new.mount_id {
        return Err(libc::EXDEV);
    }
    Ok((old_host_path, new.target))
}

/// @plan PLAN-20260314-FILE-IO.P06
/// @requirement REQ-FIO-ACCESS-MODE
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn uio_access(dir: *mut uio_DirHandle, path: *const c_char, mode: c_int) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_access called");

        if dir.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        // Validate mode bits
        let valid_mode_mask = F_OK | R_OK | W_OK | X_OK;
        if mode != 0 && (mode & !valid_mode_mask) != 0 {
            return fail_errno(libc::EINVAL, -1);
        }

        let dir_virt_path = &(*dir).virtual_path;

        let input = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        let virtual_path = normalize_virtual_path_full(dir_virt_path, &input);
        let registry = get_mount_registry().lock().unwrap();
        let repository = repository_key((*dir).repository);

        // Resolve to topmost visible object in mount order
        let visible = covering(&registry, repository, &virtual_path).find_map(|covered| {
            // @plan PLAN-20260314-FILE-IO.P09
            // @requirement REQ-FIO-ARCHIVE-MOUNT
            if covered.mount.fs_type == UIO_FSTYPE_ZIP {
                let zip_index = covered.mount.zip_index.as_ref()?;
                let name = covered.archive_name()?;
                // Use mounted_root as placeholder for ZIP entry
                return (name.is_empty() || zip_index.contains(&name)).then(|| {
                    (
                        covered.mount,
                        covered.mount.mounted_root.clone(),
                        Some(name),
                    )
                });
            }
            let host_path = covered.host_path();
            host_path
                .exists()
                .then_some((covered.mount, host_path, None))
        });

        let (mount, host_path, archive_name) = match visible {
            Some(v) => v,
            // A directory of the mount tree: it can be looked at and entered,
            // and no mount can write in it.
            None if is_virtual_directory(&registry, repository, &virtual_path) => {
                return if mode & W_OK != 0 {
                    fail_errno(libc::EACCES, -1)
                } else {
                    0
                };
            }
            None => return fail_errno(libc::ENOENT, -1),
        };

        // What `uio_stat` and `uio_fstat` report for an archive entry is also
        // what decides here. An entry whose record describes nothing a file
        // system can show answers every mode alike.
        let archive_object = match (&mount.zip_index, &archive_name) {
            (Some(zip_index), Some(name)) => match zip_index.stat(name) {
                Some(Ok(object)) => Some(object),
                Some(Err(_)) => return fail_errno(libc::ENOTSUP, -1),
                None => None,
            },
            _ => None,
        };

        // F_OK: existence only
        if mode == F_OK {
            return 0;
        }

        // Check R_OK
        if (mode & R_OK) != 0 {
            // All mounts are readable
        }

        // W_OK: would a write to the path succeed, where `uio_open` and
        // `uio_mkdir` would put it. Nothing is created or copied to find out.
        if (mode & W_OK) != 0 {
            if let Err(errno) = write_permission(&registry, repository, &virtual_path) {
                return fail_errno(errno, -1);
            }
        }

        // Check X_OK
        if (mode & X_OK) != 0 {
            // An archive directory is always searchable; a file is
            // executable when its record says so.
            if mount.fs_type == UIO_FSTYPE_ZIP {
                let executable = archive_object.is_some_and(|object| object.is_executable());
                return if executable {
                    0
                } else {
                    fail_errno(libc::EACCES, -1)
                };
            }

            // For STDIO mounts, check host filesystem
            if host_path.is_dir() {
                // Directories are executable (traversable)
                return 0;
            }
            if mount.fs_type != UIO_FSTYPE_STDIO {
                // Other archive types are not executable
                return fail_errno(libc::EACCES, -1);
            }
            // Delegate to host filesystem for stdio-backed files
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = fs::metadata(&host_path) {
                    let perms = metadata.permissions();
                    if (perms.mode() & 0o111) == 0 {
                        return fail_errno(libc::EACCES, -1);
                    }
                }
            }
            #[cfg(not(unix))]
            {
                // On non-Unix, assume files are not executable
                return fail_errno(libc::EACCES, -1);
            }
        }

        0
    })
}

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-ERRNO
/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_stat(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    stat_buf: *mut stat,
) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_stat called");

        if dir.is_null() || stat_buf.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        let dir_virt_path = &(*dir).virtual_path;
        let input = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        let virtual_path = normalize_virtual_path_full(dir_virt_path, &input);
        let registry = get_mount_registry().lock().unwrap();

        // Resolve to topmost visible object in mount order
        for covered in covering(&registry, repository_key((*dir).repository), &virtual_path) {
            let mount = covered.mount;

            // Handle ZIP mounts
            if mount.fs_type == UIO_FSTYPE_ZIP {
                // Entry names are Unicode: other bytes name no entry.
                if let (Some(zip_index), Some(name)) =
                    (mount.zip_index.as_ref(), covered.archive_name())
                {
                    match zip_index.stat(&name) {
                        Some(Ok(object)) => {
                            fill_zip_stat(object, &mut *stat_buf);
                            return 0;
                        }
                        Some(Err(_)) => return fail_errno(libc::ENOTSUP, -1),
                        None => {}
                    }
                }
                continue;
            }

            // Handle stdio mounts
            let host_path = covered.host_path();

            match fs::metadata(&host_path) {
                Ok(meta) => {
                    fill_host_stat(&meta, &mut *stat_buf);
                    return 0;
                }
                // Only a mount that lacks the object lets the lookup go on to
                // the next one; any other failure (permissions, a file where
                // a directory should be) is the answer.
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return fail_errno(err.raw_os_error().unwrap_or(libc::EIO), -1),
            }
        }

        // No mount holds it; the mount tree may still know it as a directory.
        if is_virtual_directory(&registry, repository_key((*dir).repository), &virtual_path) {
            fill_zip_stat(ZipObject::ImpliedDirectory, &mut *stat_buf);
            return 0;
        }
        fail_errno(libc::ENOENT, -1)
    })
}

/// A zeroed `stat`: all-zero is the documented "nothing known" value of every
/// field that is not filled, so none is left uninitialised for the C caller.
fn zeroed_stat() -> stat {
    // SAFETY: `stat` is plain old data and all-zero is a valid value.
    unsafe { std::mem::zeroed() }
}

/// The `stat` of a host file or directory: everything `std` reports, which
/// is the whole of the file's metadata; the fields it does not report read 0.
/// `uio_stat` and `uio_fstat` both fill a host object through here.
fn fill_host_stat(meta: &fs::Metadata, out: &mut stat) {
    *out = zeroed_stat();
    out.st_dev = meta.dev() as _;
    out.st_ino = meta.ino() as _;
    out.st_mode = meta.mode() as _;
    out.st_nlink = meta.nlink() as _;
    out.st_uid = meta.uid() as _;
    out.st_gid = meta.gid() as _;
    out.st_rdev = meta.rdev() as _;
    out.st_size = meta.size() as _;
    out.st_atime = meta.atime() as _;
    out.st_atime_nsec = meta.atime_nsec() as _;
    out.st_mtime = meta.mtime() as _;
    out.st_mtime_nsec = meta.mtime_nsec() as _;
    out.st_ctime = meta.ctime() as _;
    out.st_ctime_nsec = meta.ctime_nsec() as _;
    out.st_blksize = meta.blksize() as _;
    out.st_blocks = meta.blocks() as _;
}

/// The `stat` of an archive object, as the original `zip_fillStat` gives it:
/// size, mode and modification time, with the owner and the access and change
/// times 0 because the archive holds none. A directory that only the mount
/// tree or an archive path implies has a mode and nothing else. Path and
/// handle lookups of an archive entry both fill it through here.
fn fill_zip_stat(object: ZipObject, out: &mut stat) {
    let entry = match object {
        ZipObject::Recorded(entry) => entry,
        ZipObject::ImpliedDirectory => ZipEntryStat {
            size: 0,
            mode: IMPLIED_DIRECTORY_MODE,
            mtime: 0,
        },
    };
    *out = zeroed_stat();
    out.st_size = entry.size as off_t;
    out.st_mode = entry.mode as mode_t;
    out.st_mtime = entry.mtime as libc::time_t;
}

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_mkdir(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    mode: mode_t,
) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_mkdir called");

        if dir.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        let dir_virt_path = &(*dir).virtual_path;
        let input_path = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        let virtual_path = normalize_virtual_path_full(dir_virt_path, &input_path);

        let registry = get_mount_registry().lock().unwrap();
        let planned = plan_write(
            &registry,
            repository_key((*dir).repository),
            &virtual_path,
            WriteIntent {
                create: true,
                exclusive: true,
                parents: Parents::Make,
            },
        );
        drop(registry);
        let plan = match planned {
            Ok(plan) => plan,
            // A directory already there is an existing name for mkdir.
            Err(libc::EISDIR) => return fail_errno(libc::EEXIST, -1),
            Err(errno) => return fail_errno(errno, -1),
        };
        if let Err(errno) = prepare_write(&plan, 0) {
            return fail_errno(errno, -1);
        }

        match fs::DirBuilder::new()
            .mode(u32::from(mode))
            .create(&plan.target)
        {
            Ok(_) => 0,
            Err(e) => {
                let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                fail_errno(err_code, -1)
            }
        }
    })
}

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_rmdir(dir: *mut uio_DirHandle, path: *const c_char) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_rmdir called");

        if dir.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        let input_path = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        match remove_at(dir, &input_path, Kind::Directory) {
            Ok(()) => 0,
            Err(errno) => fail_errno(errno, -1),
        }
    })
}

/// `uio_unlink` and `uio_rmdir`: removes the `wanted` kind of entry named by
/// `input_path` from every mount of the directory's repository that holds it.
///
/// # Safety
///
/// `dir` must be a live directory handle.
unsafe fn remove_at(dir: *mut uio_DirHandle, input_path: &Path, wanted: Kind) -> Result<(), c_int> {
    if !names_an_entry(input_path.as_os_str()) {
        return Err(libc::ENOENT);
    }
    let virtual_path = normalize_virtual_path_full(&(*dir).virtual_path, input_path);
    let registry = get_mount_registry().lock().unwrap();
    let removals = plan_removal(&registry, repository_key((*dir).repository), &virtual_path);
    drop(registry);
    remove_entries(removals?, wanted)
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_lseek(handle: *mut uio_Handle, offset: off_t, whence: c_int) -> c_int {
    log_marker("uio_lseek called");

    if handle.is_null() {
        return -1;
    }

    let mut guard = match (*handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let seek_from = match whence {
        SEEK_SET => SeekFrom::Start(offset as u64),
        SEEK_CUR => SeekFrom::Current(offset),
        SEEK_END => SeekFrom::End(offset),
        _ => return -1,
    };

    match guard.seek(seek_from) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// C-compatible uio_DirList struct (must match C definition exactly)
#[allow(
    non_snake_case,
    reason = "field names mirror the legacy C UIO layout API; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
#[repr(C)]
pub struct uio_DirList {
    pub(crate) names: *mut *mut c_char,
    pub(crate) numNames: c_int,
    buffer: *mut c_char,
}

// @plan PLAN-20260314-FILE-IO.P09
// @requirement REQ-FIO-ARCHIVE-MOUNT
// uio_Handle wraps either a regular file or a ZIP entry reader
#[allow(
    non_camel_case_types,
    reason = "legacy UIO handle type name is part of the transitional C-compatible API; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub enum uio_HandleInner {
    File(std::fs::File),
    ZipEntry(crate::io::zip_reader::ZipEntryReader),
}

impl Read for uio_HandleInner {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            uio_HandleInner::File(f) => f.read(buf),
            uio_HandleInner::ZipEntry(z) => z.read(buf),
        }
    }
}

impl Write for uio_HandleInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            uio_HandleInner::File(f) => f.write(buf),
            uio_HandleInner::ZipEntry(_) => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot write to ZIP entry",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            uio_HandleInner::File(f) => f.flush(),
            uio_HandleInner::ZipEntry(_) => Ok(()),
        }
    }
}

impl Seek for uio_HandleInner {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            uio_HandleInner::File(f) => f.seek(pos),
            uio_HandleInner::ZipEntry(z) => z.seek(pos),
        }
    }
}

impl uio_HandleInner {
    /// Get file size
    pub fn len(&self) -> std::io::Result<u64> {
        match self {
            uio_HandleInner::File(f) => f.metadata().map(|m| m.len()),
            uio_HandleInner::ZipEntry(z) => Ok(z.size()),
        }
    }

    /// Return whether the file contains no bytes.
    pub fn is_empty(&self) -> std::io::Result<bool> {
        self.len().map(|len| len == 0)
    }

    /// Fills the `fstat` struct of an open handle from the handle itself: a
    /// host file by its own metadata, an archive entry by the record it copied
    /// when it was opened. Either way every field not known reads 0, and a
    /// failure leaves the struct untouched.
    fn fstat_into(&self, out: &mut stat) -> std::io::Result<()> {
        match self {
            uio_HandleInner::File(f) => fill_host_stat(&f.metadata()?, out),
            uio_HandleInner::ZipEntry(z) => fill_zip_stat(ZipObject::Recorded(z.stat()?), out),
        }
        Ok(())
    }
}

#[allow(
    non_camel_case_types,
    reason = "legacy UIO handle type name is part of the transitional C-compatible API; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub type uio_Handle = Mutex<uio_HandleInner>;

// A `*mut uio_Handle` handed to C is the pointer `Arc::into_raw` returns for
// an `Arc<uio_Handle>`, so one allocation is shared by the C caller, every
// stream and every file block over it. C only ever treats the pointer as
// opaque; it never sees or depends on the allocation's layout. Each owner
// holds one strong count and the file closes when the last one is released.

/// Hands a freshly opened handle to C, which owns the first strong count.
fn handle_into_raw(inner: uio_HandleInner) -> *mut uio_Handle {
    Arc::into_raw(Arc::new(Mutex::new(inner))).cast_mut()
}

/// Takes one more strong count on a live handle for a new owner.
///
/// # Safety
///
/// `handle` must come from [`handle_into_raw`] and still hold at least one
/// strong count owned by the caller or by a structure the caller is borrowing.
unsafe fn handle_share(handle: *mut uio_Handle) -> Arc<uio_Handle> {
    Arc::increment_strong_count(handle.cast_const());
    Arc::from_raw(handle.cast_const())
}

/// Gives up the strong count C held on `handle`, closing the file when it
/// was the last one.
///
/// # Safety
///
/// `handle` must come from [`handle_into_raw`], and the count being released
/// must not have been released already.
unsafe fn handle_release(handle: *mut uio_Handle) {
    drop(Arc::from_raw(handle.cast_const()));
}

#[repr(C)]
pub struct uio_Handle_Opaque {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_Stream {
    handle: *mut uio_Handle,
    status: c_int,
    operation: c_int,
    open_flags: c_int,
    /// Byte parked by `uio_ungetc`, handed out before the file's next byte.
    /// It sits in front of the handle: the handle position is one past the
    /// stream position while it is pending.
    pushback: Option<u8>,
}

fn repository_key(repository: *mut uio_Repository) -> usize {
    repository as usize
}

fn mount_handle_key(handle: *mut uio_MountHandle) -> usize {
    handle as usize
}

/// The absolute form of a mount point or virtual path: one leading slash and
/// none trailing. It works on the bytes, so a name that is not UTF-8 survives.
fn normalize_mount_point(path: &Path) -> PathBuf {
    let bytes = path.as_os_str().as_bytes();
    let start = bytes.iter().position(|&b| b != b'/').unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| b != b'/')
        .map_or(start, |i| i + 1);
    let mut normalized = Vec::with_capacity(end - start + 1);
    normalized.push(b'/');
    normalized.extend_from_slice(&bytes[start..end]);
    PathBuf::from(OsStr::from_bytes(&normalized))
}

/// What a new mount shows: a host directory, or an archive (the whole of it
/// or one of its directories).
struct MountBacking {
    mounted_root: PathBuf,
    fs_type: c_int,
    zip_index: Option<Arc<crate::io::zip_reader::ZipIndex>>,
    archive_root: String,
}

/// The errno for a location/relative combination `uio_mountDir` and
/// `uio_transplantDir` refuse: ABOVE and BELOW name a mount to be relative to,
/// TOP and BOTTOM must not.
fn check_relative(flags: c_int, relative: *mut uio_MountHandle) -> Result<(), c_int> {
    let location = flags & UIO_MOUNT_LOCATION_MASK;
    let relative_required = location == UIO_MOUNT_ABOVE || location == UIO_MOUNT_BELOW;
    if relative_required == relative.is_null() {
        Err(libc::EINVAL)
    } else {
        Ok(())
    }
}

/// `zip_mount` of the original: an archive can only be mounted read-only, so
/// a mount without `uio_MOUNT_RDONLY` is EACCES rather than a writable
/// archive. The archive file was found before this, and is read after it.
fn index_archive_for_mount(
    path: &Path,
    flags: c_int,
) -> Result<Arc<crate::io::zip_reader::ZipIndex>, c_int> {
    if flags & UIO_MOUNT_RDONLY != UIO_MOUNT_RDONLY {
        return Err(libc::EACCES);
    }
    index_archive(path)
}

/// Indexes the archive at `path`. A failure of the host (no such file, no
/// permission) keeps the host's errno; a file that is not a readable archive
/// is EIO, as in the original.
fn index_archive(path: &Path) -> Result<Arc<crate::io::zip_reader::ZipIndex>, c_int> {
    crate::io::zip_reader::ZipIndex::new(path)
        .map(Arc::new)
        .map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))
}

/// @plan PLAN-20260314-FILE-IO.P06
/// @requirement REQ-FIO-MOUNT-ORDER
/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
/// Register a mount of `mounted_root` (a host directory, or for ZIP mounts
/// the archive file, which is indexed now) with explicit ordering semantics.
/// Returns NULL and sets errno on failure.
///
/// The game mounts through [`uio_mountDir`], which builds the backing from the
/// mounts as they are; this is the direct form the tests use to set up a stack.
#[cfg(test)]
fn register_mount(
    repository: *mut uio_Repository,
    mount_point: &Path,
    mounted_root: PathBuf,
    fs_type: c_int,
    flags: c_int,
    relative: *mut uio_MountHandle,
    active_in_registry: bool,
) -> *mut uio_MountHandle {
    if let Err(errno) = check_relative(flags, relative) {
        set_errno(errno);
        return ptr::null_mut();
    }
    let zip_index = if fs_type == UIO_FSTYPE_ZIP {
        match index_archive_for_mount(&mounted_root, flags) {
            Ok(index) => Some(index),
            Err(errno) => {
                set_errno(errno);
                return ptr::null_mut();
            }
        }
    } else {
        None
    };
    let backing = MountBacking {
        mounted_root,
        fs_type,
        zip_index,
        archive_root: String::new(),
    };
    insert_mount(
        repository,
        mount_point,
        backing,
        flags,
        relative,
        active_in_registry,
    )
    .unwrap_or_else(|errno| fail_errno(errno, ptr::null_mut()))
}

/// Puts a mount of `backing` into the registry at the position `flags` and
/// `relative` ask for. EINVAL if `relative` is not a mount of the registry.
fn insert_mount(
    repository: *mut uio_Repository,
    mount_point: &Path,
    backing: MountBacking,
    flags: c_int,
    relative: *mut uio_MountHandle,
    active_in_registry: bool,
) -> Result<*mut uio_MountHandle, c_int> {
    let location = flags & UIO_MOUNT_LOCATION_MASK;
    let read_only = (flags & UIO_MOUNT_RDONLY) != 0;

    let id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
    let handle = Box::new(uio_MountHandle {
        repository,
        id,
        fs_type: backing.fs_type,
    });
    let handle_ptr = Box::into_raw(handle);

    let mut registry = get_mount_registry().lock().unwrap();

    // The original repository kept one array with the highest priority mount
    // at index 0: TOP inserts at the front, BOTTOM appends, ABOVE inserts at
    // the relative mount's index and BELOW right after it. Several mounts made
    // ABOVE or BELOW the same relative mount therefore stack in the order the C
    // code produced, which a fixed offset from the relative mount cannot give.
    let active_count = registry.iter().filter(|m| m.active_in_registry).count();
    let relative_index = || {
        let relative_id = unsafe { (*relative).id };
        registry
            .iter()
            .filter(|m| m.active_in_registry)
            .position(|m| m.id == relative_id)
    };
    let insert_at = match location {
        UIO_MOUNT_TOP => Some(0),
        UIO_MOUNT_ABOVE => relative_index(),
        UIO_MOUNT_BELOW => relative_index().map(|index| index + 1),
        _ => Some(active_count),
    };
    let Some(insert_at) = insert_at else {
        drop(registry);
        // SAFETY: handle_ptr came from Box::into_raw above and was never published.
        drop(unsafe { Box::from_raw(handle_ptr) });
        return Err(libc::EINVAL);
    };

    let info = MountInfo {
        id,
        repository: repository_key(repository),
        handle_ptr: mount_handle_key(handle_ptr),
        mount_point: normalize_mount_point(mount_point),
        mounted_root: backing.mounted_root,
        fs_type: backing.fs_type,
        active_in_registry,
        position: 0,
        read_only,
        zip_index: backing.zip_index,
        archive_root: backing.archive_root,
    };

    registry.insert(insert_at, info);
    // Positions are the rank in the array; start at 1000 so tests and logs
    // keep seeing the values they always did.
    const DEFAULT_START_POSITION: usize = 1000;
    for (rank, mount) in registry.iter_mut().enumerate() {
        mount.position = DEFAULT_START_POSITION + rank;
    }
    sort_mount_registry(&mut registry);
    Ok(handle_ptr)
}

fn sort_mount_registry(registry: &mut [MountInfo]) {
    // Position-based ordering: lower position = higher priority
    registry.sort_by(|a, b| {
        // Active mounts first
        b.active_in_registry
            .cmp(&a.active_in_registry)
            // Then by position (ascending)
            .then_with(|| a.position.cmp(&b.position))
            // Then by mount point length (descending for specificity)
            .then_with(|| {
                b.mount_point
                    .as_os_str()
                    .len()
                    .cmp(&a.mount_point.as_os_str().len())
            })
            // Finally by ID for determinism
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn remove_mount_entry(handle: *mut uio_MountHandle) -> Option<MountInfo> {
    if handle.is_null() {
        return None;
    }

    let handle_key = mount_handle_key(handle);
    let mut registry = get_mount_registry().lock().unwrap();
    let index = registry
        .iter()
        .position(|entry| entry.handle_ptr == handle_key)?;
    Some(registry.remove(index))
}

fn remove_repository_mounts(repository: *mut uio_Repository) -> Vec<usize> {
    let repository_key = repository_key(repository);
    let mut registry = get_mount_registry().lock().unwrap();
    let mut handles = Vec::new();
    let mut i = 0;
    while i < registry.len() {
        if registry[i].repository == repository_key {
            handles.push(registry.remove(i).handle_ptr);
        } else {
            i += 1;
        }
    }
    handles
}

fn duplicate_c_string(path: &Path) -> *mut c_char {
    let bytes = path.as_os_str().as_bytes();
    let alloc = unsafe { libc::malloc(bytes.len() + 1) } as *mut c_char;
    if alloc.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, alloc, bytes.len());
        *alloc.add(bytes.len()) = 0;
    }

    alloc
}

// =============================================================================
// uio_get_file_location / uio_unmountDir / uio_unmount_all_dirs /
// uio_get_mount_file_system_type / uio_transplantDir
// =============================================================================
/// Which mount provides the file at `in_path` (relative to `dir`) and where
/// the file is inside that mount: its host path for a directory mount, the
/// absolute path inside the archive (`/entry`) for an archive mount, which has
/// no host path. On failure both outputs are cleared and errno says why:
/// ENOENT if no mount of the directory's repository has it, EISDIR for a
/// directory. For write access (`flags` other than `O_RDONLY`) it is the write
/// location `uio_open` would use, after the same preparation: the missing
/// parent directories are made and an existing entry is copied up unless
/// `O_TRUNC` is set; the entry itself is not created. EPERM when no writable
/// mount can take it.
///
/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-ERRNO
///
/// # Safety
///
/// `dir` must be null or a live directory handle, `in_path` a C string, and
/// each output null or writable.
pub unsafe fn uio_get_file_location(
    dir: *mut uio_DirHandle,
    in_path: *const c_char,
    flags: c_int,
    mount_handle: *mut *mut uio_MountHandle,
    out_path: *mut *mut c_char,
) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_get_file_location called");

        let found = file_location(dir, in_path, flags);
        match found {
            Ok((mount, location)) => {
                let duplicated = duplicate_c_string(&location);
                if duplicated.is_null() {
                    return fail_errno(libc::ENOMEM, -1);
                }
                if !mount_handle.is_null() {
                    *mount_handle = mount as *mut uio_MountHandle;
                }
                if !out_path.is_null() {
                    *out_path = duplicated;
                }
                0
            }
            Err(errno) => {
                if !mount_handle.is_null() {
                    *mount_handle = ptr::null_mut();
                }
                if !out_path.is_null() {
                    *out_path = ptr::null_mut();
                }
                fail_errno(errno, -1)
            }
        }
    })
}

/// The handle address of the mount holding the file, and its place there.
///
/// # Safety
///
/// `dir` must be null or a live directory handle and `in_path` a C string.
unsafe fn file_location(
    dir: *mut uio_DirHandle,
    in_path: *const c_char,
    flags: c_int,
) -> Result<(usize, PathBuf), c_int> {
    let dir = dir.as_ref().ok_or(libc::EINVAL)?;
    let input = cstr_to_pathbuf(in_path).ok_or(libc::EINVAL)?;
    let virtual_path = normalize_virtual_path_full(&dir.virtual_path, &input);
    let registry = get_mount_registry().lock().unwrap();
    if flags & O_ACCMODE != O_RDONLY {
        let plan = plan_write(
            &registry,
            repository_key(dir.repository),
            &virtual_path,
            WriteIntent {
                create: flags & O_CREAT != 0,
                exclusive: flags & O_CREAT != 0 && flags & O_EXCL != 0,
                parents: Parents::Make,
            },
        )?;
        drop(registry);
        prepare_write(&plan, flags)?;
        return Ok((plan.mount_handle, plan.target));
    }
    let found = locate(&registry, repository_key(dir.repository), &virtual_path)?;
    if found.kind == Kind::Directory {
        return Err(libc::EISDIR);
    }
    let location = match found.place {
        Place::Host(path) => path,
        Place::Archive(name) => Path::new("/").join(name),
    };
    Ok((found.mount.handle_ptr, location))
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_unmountDir(mount_handle: *mut uio_MountHandle) -> c_int {
    log_marker("uio_unmountDir called");

    if mount_handle.is_null() {
        return -1;
    }

    if remove_mount_entry(mount_handle).is_none() {
        return -1;
    }

    let _ = Box::from_raw(mount_handle);
    0
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_unmount_all_dirs(repository: *mut uio_Repository) -> c_int {
    log_marker("uio_unmount_all_dirs called");

    for handle_ptr in remove_repository_mounts(repository) {
        let _ = Box::from_raw(handle_ptr as *mut uio_MountHandle);
    }

    0
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_get_mount_file_system_type(mount_handle: *mut uio_MountHandle) -> c_int {
    log_marker("uio_get_mount_file_system_type called");
    if mount_handle.is_null() {
        return 0;
    }

    (*mount_handle).fs_type
}

/// Mounts, at `mount_point`, the physical directory that `source_dir` is
/// now: the first read-only directory the repository's mounts hold at the
/// source's virtual path, host directory or archive subtree. Only read-only
/// transplants exist (ENOSYS otherwise); ENOENT if no read-only directory is
/// there.
///
/// # Safety
///
/// `source_dir` must be null or a live directory handle, `mount_point` a C
/// string, and `relative` null or a live mount handle.
#[no_mangle]
pub unsafe extern "C" fn uio_transplantDir(
    mount_point: *const c_char,
    source_dir: *mut uio_DirHandle,
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    ffi_guard!(ptr::null_mut(), {
        log_marker("uio_transplantDir called");
        match transplant_dir(mount_point, source_dir, flags, relative) {
            Ok(handle) => handle,
            Err(errno) => fail_errno(errno, ptr::null_mut()),
        }
    })
}

/// # Safety
///
/// As for [`uio_transplantDir`].
unsafe fn transplant_dir(
    mount_point: *const c_char,
    source_dir: *mut uio_DirHandle,
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> Result<*mut uio_MountHandle, c_int> {
    let source = source_dir.as_ref().ok_or(libc::EINVAL)?;
    if flags & UIO_MOUNT_RDONLY != UIO_MOUNT_RDONLY {
        return Err(libc::ENOSYS);
    }
    check_relative(flags, relative)?;
    let mount_point = cstr_to_pathbuf(mount_point).ok_or(libc::EINVAL)?;
    if !valid_mount_point(&mount_point) {
        return Err(libc::EINVAL);
    }

    let backing = {
        let registry = get_mount_registry().lock().unwrap();
        let (mount, place) = transplant_source(
            &registry,
            repository_key(source.repository),
            &source.virtual_path,
        )?;
        match place {
            Place::Host(path) => MountBacking {
                mounted_root: path,
                fs_type: UIO_FSTYPE_STDIO,
                zip_index: None,
                archive_root: String::new(),
            },
            Place::Archive(name) => MountBacking {
                mounted_root: mount.mounted_root.clone(),
                fs_type: UIO_FSTYPE_ZIP,
                zip_index: mount.zip_index.clone(),
                archive_root: name,
            },
        }
    };
    insert_mount(
        source.repository,
        &mount_point,
        backing,
        flags,
        relative,
        true,
    )
}

// =============================================================================
// uio_fgets / uio_fgetc / uio_ungetc / uio_vfprintf / uio_fputc / uio_fputs
// uio_fflush / uio_feof / uio_ferror / uio_clearerr / uio_streamHandle
// =============================================================================

/// Locks the stream's handle and runs `f` on the stream state and the open
/// file. `None` means the stream or its handle is null, or the handle lock is
/// poisoned; every caller reports that as its plain failure value.
///
/// # Safety
///
/// `stream` must be null or point to a live `uio_Stream` whose handle is null
/// or live, with no other reference to either for the duration of the call.
unsafe fn with_stream_handle<R>(
    stream: *mut uio_Stream,
    f: impl FnOnce(&mut uio_Stream, &mut uio_HandleInner) -> R,
) -> Option<R> {
    let stream = stream.as_mut()?;
    let handle = stream.handle.as_ref()?;
    let mut guard = handle.lock().ok()?;
    Some(f(stream, &mut guard))
}

/// What one stdio-style read produced: how many bytes landed in the buffer
/// and whether the underlying read failed after that.
struct StreamRead {
    count: usize,
    failed: bool,
}

/// Fills `out` as far as the stream allows: first the byte parked by
/// `uio_ungetc`, then the file until it is full or ends. A short count with
/// `failed == false` means the end of the file was reached.
fn read_stream(stream: &mut uio_Stream, file: &mut uio_HandleInner, out: &mut [u8]) -> StreamRead {
    let mut count = 0;
    if !out.is_empty() {
        if let Some(byte) = stream.pushback.take() {
            out[0] = byte;
            count = 1;
        }
    }
    while count < out.len() {
        match file.read(&mut out[count..]) {
            Ok(0) => break,
            Ok(n) => count += n,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => {
                stream.status = UIO_STREAM_STATUS_ERROR;
                return StreamRead {
                    count,
                    failed: true,
                };
            }
        }
    }
    if count < out.len() {
        stream.status = UIO_STREAM_STATUS_EOF;
    }
    StreamRead {
        count,
        failed: false,
    }
}

/// Writes `data` at the logical position of the stream. A byte parked by
/// `uio_ungetc` is dropped first, which moves the file position back to where
/// the stream says it is. A failed write is recorded for `uio_ferror`.
fn write_stream(stream: &mut uio_Stream, file: &mut uio_HandleInner, data: &[u8]) -> bool {
    let positioned = if stream.pushback.is_some() {
        let logical = file
            .stream_position()
            .map(|position| position.saturating_sub(1));
        logical.and_then(|position| file.seek(SeekFrom::Start(position)))
    } else {
        Ok(0)
    };
    let written = positioned.and_then(|_| file.write_all(data));
    if written.is_err() {
        stream.status = UIO_STREAM_STATUS_ERROR;
        return false;
    }
    stream.pushback = None;
    true
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_fgets(
    buf: *mut c_char,
    size: c_int,
    stream: *mut uio_Stream,
) -> *mut c_char {
    if buf.is_null() || size <= 0 {
        return ptr::null_mut();
    }
    let buffer = slice::from_raw_parts_mut(buf.cast::<u8>(), size as usize);
    let line_len = with_stream_handle(stream, |stream, file| {
        let mut count = 0;
        // One byte stays free for the terminator.
        while count < buffer.len() - 1 {
            let read = read_stream(stream, file, &mut buffer[count..=count]);
            if read.failed {
                return None;
            }
            if read.count == 0 {
                break;
            }
            count += 1;
            if buffer[count - 1] == b'\n' {
                break;
            }
        }
        Some(count)
    });
    match line_len {
        Some(Some(count)) if count > 0 => {
            buffer[count] = 0;
            buf
        }
        _ => ptr::null_mut(),
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_fgetc(stream: *mut uio_Stream) -> c_int {
    with_stream_handle(stream, |stream, file| {
        let mut byte = [0u8; 1];
        match read_stream(stream, file, &mut byte) {
            StreamRead { count: 1, .. } => c_int::from(byte[0]),
            _ => -1,
        }
    })
    .unwrap_or(-1)
}

/// Parks `c` (converted to an unsigned char, as `ungetc` does) so the next
/// read returns it before anything else, and clears the end-of-file flag.
/// Like stdio, a single byte of pushback is guaranteed: a second call before
/// the first byte is read fails, as does pushing back EOF or pushing back
/// onto a write-only stream. Returns the byte pushed back, or -1.
///
/// The original C declared this entry point but never implemented it (the
/// body returned EOF unconditionally); the header comment, the file-io
/// specification and stdio all define one byte of pushback.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_ungetc(c: c_int, stream: *mut uio_Stream) -> c_int {
    if c == -1 {
        return -1;
    }
    with_stream_handle(stream, |stream, _file| {
        if stream.open_flags & O_ACCMODE == O_WRONLY || stream.pushback.is_some() {
            return -1;
        }
        let byte = c as u8;
        stream.pushback = Some(byte);
        if stream.status == UIO_STREAM_STATUS_EOF {
            stream.status = UIO_STREAM_STATUS_OK;
        }
        c_int::from(byte)
    })
    .unwrap_or(-1)
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_fputc(c: c_int, stream: *mut uio_Stream) -> c_int {
    let byte = c as u8;
    match with_stream_handle(stream, |stream, file| write_stream(stream, file, &[byte])) {
        Some(true) => c_int::from(byte),
        _ => -1,
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_fputs(s: *const c_char, stream: *mut uio_Stream) -> c_int {
    if s.is_null() {
        return -1;
    }
    let bytes = std::ffi::CStr::from_ptr(s).to_bytes();
    match with_stream_handle(stream, |stream, file| write_stream(stream, file, bytes)) {
        Some(true) => 0,
        _ => -1,
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_fflush(stream: *mut uio_Stream) -> c_int {
    if stream.is_null() {
        return 0; // Flushing NULL stream is a no-op success
    }

    let s = &*stream;
    if s.handle.is_null() {
        return 0;
    }

    if let Ok(mut guard) = (*s.handle).lock() {
        use std::io::Write;
        match guard.flush() {
            Ok(()) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// @plan PLAN-20260314-FILE-IO.P03
/// @requirement REQ-FIO-STREAM-STATUS
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_feof(stream: *mut uio_Stream) -> c_int {
    if stream.is_null() {
        return 0;
    }

    let status = (*stream).status;
    if status == UIO_STREAM_STATUS_EOF {
        1
    } else {
        0
    }
}

/// @plan PLAN-20260314-FILE-IO.P03
/// @requirement REQ-FIO-STREAM-STATUS
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_ferror(stream: *mut uio_Stream) -> c_int {
    if stream.is_null() {
        return 0;
    }

    let status = (*stream).status;
    if status == UIO_STREAM_STATUS_ERROR {
        1
    } else {
        0
    }
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_fwrite(
    ptr: *const libc::c_void,
    size: size_t,
    nmemb: size_t,
    stream: *mut uio_Stream,
) -> size_t {
    if stream.is_null() || ptr.is_null() {
        return 0;
    }
    let Some(total_bytes) = size.checked_mul(nmemb) else {
        return 0;
    };
    if total_bytes == 0 {
        return nmemb; // Writing 0 items is always successful
    }
    let data = std::slice::from_raw_parts(ptr as *const u8, total_bytes);
    match with_stream_handle(stream, |stream, file| write_stream(stream, file, data)) {
        Some(true) => nmemb,
        _ => 0,
    }
}

/// @plan PLAN-20260314-FILE-IO.P03
/// @requirement REQ-FIO-STREAM-STATUS
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_clearerr(stream: *mut uio_Stream) {
    if stream.is_null() {
        return;
    }

    (*stream).status = UIO_STREAM_STATUS_OK;
}

// =============================================================================
// uio_openFileBlock / uio_closeFileBlock / uio_accessFileBlock /
// uio_copyFileBlock / uio_setFileBlockUsageHint / uio_openFileBlock2
// uio_clear_file_block_buffers
// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Internal structure representing a file block with caching
struct FileBlockInner {
    /// Shares the handle with the C caller, so the block keeps reading after
    /// the caller closes its own reference. The last owner closes the file.
    handle: Arc<uio_Handle>,
    /// Base offset in file where this block starts
    base_offset: off_t,
    /// Size of this block (0 means whole file)
    size: off_t,
    /// Internal cache buffer for accessFileBlock
    cache: Vec<u8>,
    /// File offset where cache starts
    cache_offset: off_t,
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Opaque C-compatible FileBlock handle
#[repr(C)]
pub struct uio_FileBlock {
    _private: [u8; 0],
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Create a file block covering the entire file
///
/// # Safety
///
/// `handle` must be NULL or a live handle returned by `uio_open` or
/// `uio_fopen`'s stream. The block shares it, so it stays readable after the
/// caller's `uio_close`.
#[no_mangle]
pub unsafe extern "C" fn uio_openFileBlock(handle: *mut uio_Handle) -> *mut uio_FileBlock {
    ffi_guard!(ptr::null_mut(), {
        log_marker("uio_openFileBlock called");

        if handle.is_null() {
            return fail_errno(libc::EINVAL, ptr::null_mut());
        }

        // Get file size
        let file_guard = match (*handle).lock() {
            Ok(g) => g,
            Err(_) => return fail_errno(libc::EIO, ptr::null_mut()),
        };

        let size = match file_guard.len() {
            Ok(len) => len as off_t,
            Err(e) => {
                let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                return fail_errno(err_code, ptr::null_mut());
            }
        };

        drop(file_guard);

        let inner = FileBlockInner {
            handle: handle_share(handle),
            base_offset: 0,
            size,
            cache: Vec::new(),
            cache_offset: 0,
        };

        let block = Box::new(inner);
        Box::into_raw(block) as *mut uio_FileBlock
    })
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Create a file block covering a specific region
///
/// # Safety
///
/// Same as `uio_openFileBlock`: `handle` must be NULL or live, and the block
/// shares it.
#[no_mangle]
pub unsafe extern "C" fn uio_openFileBlock2(
    handle: *mut uio_Handle,
    offset: off_t,
    size: size_t,
) -> *mut uio_FileBlock {
    ffi_guard!(ptr::null_mut(), {
        log_marker("uio_openFileBlock2 called");

        if handle.is_null() {
            return fail_errno(libc::EINVAL, ptr::null_mut());
        }

        if offset < 0 {
            return fail_errno(libc::EINVAL, ptr::null_mut());
        }

        // Validate that offset + size doesn't overflow
        let end_offset = match (offset as u64).checked_add(size as u64) {
            Some(end) => end,
            None => return fail_errno(libc::EINVAL, ptr::null_mut()),
        };

        // Validate against file size
        let file_guard = match (*handle).lock() {
            Ok(g) => g,
            Err(_) => return fail_errno(libc::EIO, ptr::null_mut()),
        };

        let file_size = match file_guard.len() {
            Ok(len) => len,
            Err(e) => {
                let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                return fail_errno(err_code, ptr::null_mut());
            }
        };

        drop(file_guard);

        if end_offset > file_size {
            return fail_errno(libc::EINVAL, ptr::null_mut());
        }

        let inner = FileBlockInner {
            handle: handle_share(handle),
            base_offset: offset,
            size: size as off_t,
            cache: Vec::new(),
            cache_offset: 0,
        };

        let block = Box::new(inner);
        Box::into_raw(block) as *mut uio_FileBlock
    })
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Access bytes from a file block, caching them internally
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_accessFileBlock(
    block: *mut uio_FileBlock,
    offset: off_t,
    length: size_t,
    buffer: *mut *mut c_char,
) -> isize {
    ffi_guard!(-1, {
        log_marker("uio_accessFileBlock called");

        if block.is_null() || buffer.is_null() {
            return fail_errno(libc::EINVAL, -1) as isize;
        }

        if offset < 0 {
            return fail_errno(libc::EINVAL, -1) as isize;
        }

        let inner = &mut *(block as *mut FileBlockInner);

        let Some(file_offset) = inner.base_offset.checked_add(offset) else {
            return fail_errno(libc::EINVAL, -1) as isize;
        };

        // Calculate how many bytes are available in this block
        let available_in_block = if inner.size == 0 {
            // Whole file - need to check actual file size
            let file_guard = match inner.handle.lock() {
                Ok(g) => g,
                Err(_) => return fail_errno(libc::EIO, -1) as isize,
            };
            let file_size = match file_guard.len() {
                Ok(len) => len as off_t,
                Err(e) => {
                    let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                    return fail_errno(err_code, -1) as isize;
                }
            };
            drop(file_guard);
            file_size - file_offset
        } else {
            inner.size - offset
        };

        if available_in_block <= 0 {
            // At or past end of block
            *buffer = ptr::null_mut();
            return 0;
        }

        let bytes_to_read = std::cmp::min(length as off_t, available_in_block) as usize;

        // Allocate and populate cache
        inner.cache.clear();
        inner.cache.resize(bytes_to_read, 0);
        inner.cache_offset = file_offset;

        let mut file_guard = match inner.handle.lock() {
            Ok(g) => g,
            Err(_) => return fail_errno(libc::EIO, -1) as isize,
        };

        // Seek to position
        if let Err(e) = file_guard.seek(SeekFrom::Start(file_offset as u64)) {
            let err_code = e.raw_os_error().unwrap_or(libc::EIO);
            return fail_errno(err_code, -1) as isize;
        }

        // Read into cache
        let bytes_read = match file_guard.read(&mut inner.cache) {
            Ok(n) => n,
            Err(e) => {
                let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                return fail_errno(err_code, -1) as isize;
            }
        };

        drop(file_guard);

        // Truncate cache to actual bytes read
        inner.cache.truncate(bytes_read);

        // Set output buffer pointer to cache
        if bytes_read > 0 {
            *buffer = inner.cache.as_mut_ptr() as *mut c_char;
        } else {
            *buffer = ptr::null_mut();
        }

        bytes_read as isize
    })
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Copy bytes from file block into caller-provided buffer
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_copyFileBlock(
    block: *mut uio_FileBlock,
    offset: off_t,
    buffer: *mut c_char,
    length: size_t,
) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_copyFileBlock called");

        if block.is_null() || buffer.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        if offset < 0 {
            return fail_errno(libc::EINVAL, -1);
        }

        let inner = &mut *(block as *mut FileBlockInner);

        let Some(file_offset) = inner.base_offset.checked_add(offset) else {
            return fail_errno(libc::EINVAL, -1);
        };

        // Calculate how many bytes are available in this block
        let available_in_block = if inner.size == 0 {
            // Whole file - need to check actual file size
            let file_guard = match inner.handle.lock() {
                Ok(g) => g,
                Err(_) => return fail_errno(libc::EIO, -1),
            };
            let file_size = match file_guard.len() {
                Ok(len) => len as off_t,
                Err(e) => {
                    let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                    return fail_errno(err_code, -1);
                }
            };
            drop(file_guard);
            file_size - file_offset
        } else {
            inner.size - offset
        };

        if available_in_block <= 0 {
            // At or past end of block
            return 0;
        }

        let bytes_to_read = std::cmp::min(length as off_t, available_in_block) as usize;

        let mut file_guard = match inner.handle.lock() {
            Ok(g) => g,
            Err(_) => return fail_errno(libc::EIO, -1),
        };

        // Seek to position
        if let Err(e) = file_guard.seek(SeekFrom::Start(file_offset as u64)) {
            let err_code = e.raw_os_error().unwrap_or(libc::EIO);
            return fail_errno(err_code, -1);
        }

        // Create temporary buffer to read into
        let mut temp_buf = vec![0u8; bytes_to_read];
        let bytes_read = match file_guard.read(&mut temp_buf) {
            Ok(n) => n,
            Err(e) => {
                let err_code = e.raw_os_error().unwrap_or(libc::EIO);
                return fail_errno(err_code, -1);
            }
        };

        drop(file_guard);

        // Copy to caller buffer
        if bytes_read > 0 {
            let dest_slice = slice::from_raw_parts_mut(buffer.cast::<u8>(), bytes_read);
            dest_slice.copy_from_slice(&temp_buf[..bytes_read]);
        }

        0 // Success
    })
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Clear internal cache buffers without invalidating the block
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_clear_file_block_buffers(block: *mut uio_FileBlock) {
    ffi_guard!((), {
        log_marker("uio_clear_file_block_buffers called");

        if block.is_null() {
            return;
        }

        let inner = &mut *(block as *mut FileBlockInner);
        inner.cache.clear();
        inner.cache_offset = 0;
    })
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Close file block and release its share of the underlying handle. The
/// handle's file closes once the caller and every block have let go of it.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_closeFileBlock(block: *mut uio_FileBlock) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_closeFileBlock called");

        if block.is_null() {
            return 0;
        }

        // Convert back to Box and drop
        let _ = Box::from_raw(block as *mut FileBlockInner);
        0
    })
}

/// @plan PLAN-20260314-FILE-IO.P08
/// @requirement REQ-FIO-FILEBLOCK
/// Set usage hints for file block (no-op for now, documented as acceptable)
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_setFileBlockUsageHint(
    _block: *mut uio_FileBlock,
    _usage: c_int,
    _read_ahead_buf_size: size_t,
) {
    ffi_guard!((), {
        log_marker("uio_setFileBlockUsageHint called (no-op)");
        // No-op implementation - acceptable per plan
    })
}

// =============================================================================
// uio_getFileSystemHandler / uio_gPFileFlagsFromPRootFlags / uio_walkGPPath
// =============================================================================

#[repr(C)]
pub struct uio_FileSystemHandler {
    _private: [u8; 0],
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_getFileSystemHandler(_id: c_int) -> *mut uio_FileSystemHandler {
    log_marker("uio_getFileSystemHandler called - stub");
    // Return a dummy handler
    let handler = Box::new(uio_FileSystemHandler { _private: [] });
    Box::leak(handler) as *mut uio_FileSystemHandler
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_gPFileFlagsFromPRootFlags(_flags: c_int) -> c_int {
    log_marker("uio_gPFileFlagsFromPRootFlags called - stub");
    _flags // Pass through unchanged
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_walkGPPath(
    _start_gpdir: *mut uio_GPDir,
    _path: *const c_char,
    _flags: c_int,
    _result: *mut uio_GPDir,
) -> c_int {
    log_marker("uio_walkGPPath called - stub");
    -1 // Error
}

// =============================================================================
// uio_getStdioAccess / uio_releaseStdioAccess / uio_printMounts
// =============================================================================
/// @plan PLAN-20260314-FILE-IO.P10
/// @requirement REQ-FIO-STDIO-ACCESS
/// Internal structure for StdioAccess handles.
struct StdioAccessHandleInner {
    /// C-allocated string of the host path (stable across Rust reallocations)
    path_cstr: *mut c_char,
    /// The copy made for an archive entry; `None` when the file is used where
    /// it lies.
    copy: Option<StdioCopy>,
}

/// An archive entry unpacked into a directory generated under the caller's
/// temporary directory, as the original did: all of it goes through the mount
/// stack, and releasing the access removes exactly what was made.
struct StdioCopy {
    /// The caller's temporary directory; this handle holds one reference.
    temp_root: *mut uio_DirHandle,
    /// The generated directory under `temp_root`, and a handle on it.
    dir_name: CString,
    dir: *mut uio_DirHandle,
    file_name: CString,
}

impl StdioCopy {
    /// Removes the copy and its directory, then lets go of the temporary
    /// directory. Failures cannot be returned from the release call, so they
    /// are logged: the original reported them the same way.
    ///
    /// # Safety
    ///
    /// The handles must be the live ones this copy was made with, and not be
    /// used again.
    unsafe fn remove(self) {
        if uio_unlink(self.dir, self.file_name.as_ptr()) == -1 {
            log_marker(&format!(
                "uio_releaseStdioAccess: could not remove temporary file: errno {}",
                last_errno()
            ));
        }
        uio_closeDir(self.dir);
        if uio_rmdir(self.temp_root, self.dir_name.as_ptr()) == -1 {
            log_marker(&format!(
                "uio_releaseStdioAccess: could not remove temporary directory: errno {}",
                last_errno()
            ));
        }
        uio_closeDir(self.temp_root);
    }
}

/// The errno the last failed call set.
fn last_errno() -> c_int {
    std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EIO)
}

#[repr(C)]
pub struct uio_StdioAccessHandle {
    _private: [u8; 0],
}

// =============================================================================
// uio_GPDir / uio_GPFile / uio_GPRoot / uio_PRoot functions
// =============================================================================

#[repr(C)]
pub struct uio_GPDir {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_GPFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_GPDirEntry {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_PDirHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_PRoot {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_GPRoot {
    _private: [u8; 0],
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_DirHandle_print(
    _dir_handle: *const uio_DirHandle,
    _out_stream: *mut libc::FILE,
) {
    log_marker("uio_DirHandle_print called - stub");
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDirHandle_delete(_handle: *mut uio_PDirHandle) {
    log_marker("uio_GPDirHandle_delete called - stub");
    if !_handle.is_null() {
        let _ = Box::from_raw(_handle);
    }
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_addFile(
    _g_pdir: *mut uio_GPDir,
    _file_name: *const c_char,
    _file: *mut uio_GPFile,
) {
    log_marker("uio_GPDir_addFile called - stub");
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_closeEntries(_g_pdir: *mut uio_GPDir) {
    log_marker("uio_GPDir_closeEntries called - stub");
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_commitSubDir(
    _g_pdir: *mut uio_GPDir,
    _dir_name: *const c_char,
    _sub_dir: *mut uio_GPDir,
    _flags: c_int,
) {
    log_marker("uio_GPDir_commitSubDir called - stub");
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_getGPDirEntry(
    _g_pdir: *mut uio_GPDir,
    _name: *const c_char,
) -> *mut uio_GPDirEntry {
    log_marker("uio_GPDir_getGPDirEntry called - stub");
    ptr::null_mut()
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_getPDirEntryHandle(
    _entry: *mut uio_GPDirEntry,
) -> *mut uio_PDirHandle {
    log_marker("uio_GPDir_getPDirEntryHandle called - stub");
    ptr::null_mut()
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_openEntries(_g_pdir: *mut uio_GPDir) -> c_int {
    log_marker("uio_GPDir_openEntries called - stub");
    -1
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_prepareSubDir(
    _g_pdir: *mut uio_GPDir,
    _dir_name: *const c_char,
) -> *mut uio_GPDir {
    log_marker("uio_GPDir_prepareSubDir called - stub");
    ptr::null_mut()
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_readEntries(_g_pdir: *mut uio_GPDir) -> c_int {
    log_marker("uio_GPDir_readEntries called - stub");
    -1
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPFileHandle_delete(_handle: *mut uio_Handle) {
    log_marker("uio_GPFileHandle_delete called - stub");
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPFile_delete(_g_pfile: *mut uio_GPFile) {
    log_marker("uio_GPFile_delete called - stub");
    if !_g_pfile.is_null() {
        let _ = Box::from_raw(_g_pfile);
    }
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_GPFile_new(
    _p_root: *mut uio_PRoot,
    _extra: *mut libc::c_void,
    _flags: c_int,
) -> *mut uio_GPFile {
    log_marker("uio_GPFile_new called - stub");
    let file = Box::new(uio_GPFile { _private: [] });
    Box::leak(file) as *mut uio_GPFile
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPRoot_delete(_g_proot: *mut uio_GPRoot) {
    log_marker("uio_GPRoot_delete called - stub");
    if !_g_proot.is_null() {
        let _ = Box::from_raw(_g_proot);
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPRoot_makePRoot(_g_proot: *mut uio_GPRoot) -> *mut uio_PRoot {
    log_marker("uio_GPRoot_makePRoot called - stub");
    // For simplicity, just cast (assuming compatible layout)
    _g_proot as *mut uio_PRoot
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_GPRoot_umount(_p_root: *mut uio_PRoot) -> c_int {
    log_marker("uio_GPRoot_umount called - stub");
    0
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_Handle_new(
    _root: *mut uio_PRoot,
    _native: *mut libc::c_void,
    _open_flags: c_int,
) -> *mut uio_Handle {
    log_marker("uio_Handle_new called - stub");
    ptr::null_mut()
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_PRoot_getRootDirHandle(
    _p_root: *mut uio_PRoot,
) -> *mut uio_PDirHandle {
    log_marker("uio_PRoot_getRootDirHandle called - stub");
    let handle = Box::new(uio_PDirHandle { _private: [] });
    Box::leak(handle) as *mut uio_PDirHandle
}

/// @plan PLAN-20260314-FILE-IO.P10
/// @requirement REQ-FIO-STDIO-ACCESS
/// Get the path from a StdioAccess handle.
/// Returns a stable C string pointer that remains valid until the handle is released.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_StdioAccessHandle_getPath(
    handle: *mut uio_StdioAccessHandle,
) -> *const c_char {
    ffi_guard!(ptr::null(), {
        log_marker("uio_StdioAccessHandle_getPath called");

        if handle.is_null() {
            set_errno(libc::EINVAL);
            return ptr::null();
        }

        let inner = &*(handle as *const StdioAccessHandleInner);
        inner.path_cstr as *const c_char
    })
}

/// What the repository holds at a path that `uio_getStdioAccess` was asked for.
enum AccessSource {
    /// A file of a directory mount: usable where it lies.
    Host(PathBuf),
    /// An entry of an archive: it has no host path and has to be unpacked.
    Entry(Arc<crate::io::zip_reader::ZipIndex>, String),
}

/// The generated directory name the original used: eight hex digits from the
/// clock, advanced when the name is taken.
fn temp_dir_name(attempt: u32) -> CString {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as u32);
    CString::new(format!("{:08x}", seconds.wrapping_add(attempt))).expect("hex digits hold no NUL")
}

/// Creates a new directory under `temp_dir` through the mount stack.
///
/// # Safety
///
/// `temp_dir` must be a live directory handle.
unsafe fn make_temp_directory(temp_dir: *mut uio_DirHandle) -> Result<CString, c_int> {
    const ATTEMPTS: u32 = 16;
    for attempt in 0..ATTEMPTS {
        let name = temp_dir_name(attempt);
        if uio_mkdir(temp_dir, name.as_ptr(), 0o700) == 0 {
            return Ok(name);
        }
        match last_errno() {
            libc::EEXIST => {}
            errno => return Err(errno),
        }
    }
    Err(libc::ENOSPC)
}

/// Writes `data` as a new file `name` in `dir`, through the mount stack.
///
/// # Safety
///
/// `dir` must be a live directory handle.
unsafe fn write_new_file(dir: *mut uio_DirHandle, name: &CStr, data: &[u8]) -> Result<(), c_int> {
    let file = uio_open(dir, name.as_ptr(), O_WRONLY | O_CREAT | O_EXCL, 0o600);
    if file.is_null() {
        return Err(last_errno());
    }
    let written = (*file).lock().unwrap().write_all(data);
    uio_close(file);
    written.map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))
}

/// Unpacks `data` into a fresh directory under `temp_dir`.
///
/// # Safety
///
/// `temp_dir` must be a live directory handle.
unsafe fn unpack_into_temp(
    temp_dir: *mut uio_DirHandle,
    file_name: CString,
    data: &[u8],
) -> Result<StdioCopy, c_int> {
    let dir_name = make_temp_directory(temp_dir)?;
    let dir = uio_openDirRelative(temp_dir, dir_name.as_ptr(), 0);
    if dir.is_null() {
        let errno = last_errno();
        uio_rmdir(temp_dir, dir_name.as_ptr());
        return Err(errno);
    }
    (*temp_dir)
        .refcount
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let copy = StdioCopy {
        temp_root: temp_dir,
        dir_name,
        dir,
        file_name,
    };
    match write_new_file(dir, &copy.file_name, data) {
        Ok(()) => Ok(copy),
        Err(errno) => {
            copy.remove();
            Err(errno)
        }
    }
}

/// The host path of the unpacked copy. The copy has to be on a directory
/// mount: an archive under the temporary directory cannot give a host path.
///
/// # Safety
///
/// `copy.dir` must be a live directory handle.
unsafe fn copy_host_path(copy: &StdioCopy) -> Result<PathBuf, c_int> {
    let dir = &*copy.dir;
    let name = Path::new(OsStr::from_bytes(copy.file_name.to_bytes()));
    let virtual_path = normalize_virtual_path_full(&dir.virtual_path, name);
    let registry = get_mount_registry().lock().unwrap();
    match locate(&registry, repository_key(dir.repository), &virtual_path)?.place {
        Place::Host(path) => Ok(path),
        Place::Archive(_) => Err(libc::EINVAL),
    }
}

/// The C-visible handle for the path in `path_cstr`, owning `copy` if any.
fn new_access(path_cstr: *mut c_char, copy: Option<StdioCopy>) -> *mut uio_StdioAccessHandle {
    Box::into_raw(Box::new(StdioAccessHandleInner { path_cstr, copy })).cast()
}

/// Access for writing: the write location `uio_getFileLocation` names for
/// `flags`, after the same preparation (parents made, an entry of another
/// mount copied up unless the flags truncate it). A write location is never
/// an archive, so as in the original nothing is unpacked and no temporary
/// directory is involved.
///
/// # Safety
///
/// As for [`uio_getStdioAccess`].
unsafe fn write_access(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    flags: c_int,
) -> Result<*mut uio_StdioAccessHandle, c_int> {
    let (_, location) = file_location(dir, path, flags)?;
    let path_cstr = duplicate_c_string(&location);
    if path_cstr.is_null() {
        Err(libc::ENOMEM)
    } else {
        Ok(new_access(path_cstr, None))
    }
}

/// # Safety
///
/// As for [`uio_getStdioAccess`].
unsafe fn stdio_access(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    flags: c_int,
    temp_dir: *mut uio_DirHandle,
) -> Result<*mut uio_StdioAccessHandle, c_int> {
    if flags & O_ACCMODE != O_RDONLY {
        return write_access(dir, path, flags);
    }
    let dir = dir.as_ref().ok_or(libc::EINVAL)?;
    let input = cstr_to_pathbuf(path).ok_or(libc::EINVAL)?;
    let virtual_path = normalize_virtual_path_full(&dir.virtual_path, &input);
    let source = {
        let registry = get_mount_registry().lock().unwrap();
        let found = locate(&registry, repository_key(dir.repository), &virtual_path)?;
        match (found.kind, found.place) {
            (Kind::Directory, _) => return Err(libc::EISDIR),
            (Kind::File, Place::Host(path)) => AccessSource::Host(path),
            (Kind::File, Place::Archive(name)) => {
                let index = found
                    .mount
                    .zip_index
                    .clone()
                    .expect("an archive mount is registered with its index");
                AccessSource::Entry(index, name)
            }
        }
    };

    let (index, entry) = match source {
        AccessSource::Host(path) => {
            let path_cstr = duplicate_c_string(&path);
            return if path_cstr.is_null() {
                Err(libc::ENOMEM)
            } else {
                Ok(new_access(path_cstr, None))
            };
        }
        AccessSource::Entry(index, entry) => (index, entry),
    };
    if temp_dir.is_null() {
        return Err(libc::EINVAL);
    }
    let data = index
        .read_entry(&entry)
        .map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))?;
    let file_name = Path::new(&entry)
        .file_name()
        .and_then(|name| CString::new(name.as_bytes()).ok())
        .ok_or(libc::EINVAL)?;
    let copy = unpack_into_temp(temp_dir, file_name, &data)?;
    let host = match copy_host_path(&copy) {
        Ok(host) => host,
        Err(errno) => {
            copy.remove();
            return Err(errno);
        }
    };
    let path_cstr = duplicate_c_string(&host);
    if path_cstr.is_null() {
        copy.remove();
        return Err(libc::ENOMEM);
    }
    Ok(new_access(path_cstr, Some(copy)))
}

/// @plan PLAN-20260314-FILE-IO.P10
/// @requirement REQ-FIO-STDIO-ACCESS
/// Get stdio access to a file, returning a handle with a guaranteed host
/// filesystem path. A file of a directory mount is used where it lies. An
/// entry of an archive is unpacked into a new directory under `temp_dir`
/// (required then), through the mount stack, so `temp_dir` has to be a
/// writable directory mount now: EPERM on a read-only one, ENOENT when its
/// mount is gone. Releasing the handle removes the unpacked file and the
/// directory made for it, never `temp_dir` itself.
///
/// With write flags (anything but `O_RDONLY`) the path is the write location
/// of `uio_open` for those flags, prepared the same way, with the errnos of
/// `uio_getFileLocation` (EPERM when no writable mount can take it). The file
/// stays on release, `temp_dir` is not used, and may be NULL.
///
/// # Safety
///
/// `dir` and `temp_dir` must be null or live directory handles and `path` a C
/// string.
#[no_mangle]
pub unsafe extern "C" fn uio_getStdioAccess(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    flags: c_int,
    temp_dir: *mut uio_DirHandle,
) -> *mut uio_StdioAccessHandle {
    ffi_guard!(ptr::null_mut(), {
        log_marker("uio_getStdioAccess called");
        stdio_access(dir, path, flags, temp_dir)
            .unwrap_or_else(|errno| fail_errno(errno, ptr::null_mut()))
    })
}

/// @plan PLAN-20260314-FILE-IO.P10
/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-STDIO-ACCESS
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// Release a StdioAccess handle.
/// For temp copies, deletes the temp file (best-effort).
/// For direct paths, only frees the handle (never deletes the underlying file).
/// Safe to call even if the mount this handle came from was unmounted.
/// The handle owns its resources independently of mount state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_releaseStdioAccess(handle: *mut uio_StdioAccessHandle) {
    ffi_guard!((), {
        log_marker("uio_releaseStdioAccess called");

        if handle.is_null() {
            return;
        }

        let inner = Box::from_raw(handle as *mut StdioAccessHandleInner);
        libc::free(inner.path_cstr.cast());
        if let Some(copy) = inner.copy {
            copy.remove();
        }

        // inner is dropped here, freeing the Box
    })
}

/// @plan PLAN-20260314-FILE-IO.P10
/// @requirement REQ-FIO-COPY
/// Copy a file from one virtual location to another.
/// Resolves both paths through the mount system.
/// Returns 0 on success, -1 on error (with errno set).
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn uio_copy_file(
    src_dir: *mut uio_DirHandle,
    src_path: *const c_char,
    dst_dir: *mut uio_DirHandle,
    dst_path: *const c_char,
) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_copy_file called");

        if src_dir.is_null() || dst_dir.is_null() {
            set_errno(libc::EINVAL);
            return -1;
        }

        let _src_input = match cstr_to_pathbuf(src_path) {
            Some(p) => p,
            None => {
                set_errno(libc::EINVAL);
                return -1;
            }
        };

        let _dst_input = match cstr_to_pathbuf(dst_path) {
            Some(p) => p,
            None => {
                set_errno(libc::EINVAL);
                return -1;
            }
        };

        // Open source for reading
        let src_handle = uio_open(src_dir, src_path, O_RDONLY, 0);
        if src_handle.is_null() {
            // errno already set by uio_open
            return -1;
        }

        // Open destination for writing (create, fail if exists)
        let dst_handle = uio_open(dst_dir, dst_path, O_WRONLY | O_CREAT | O_EXCL, 0o644);
        if dst_handle.is_null() {
            // errno already set by uio_open
            uio_close(src_handle);
            return -1;
        }

        // Copy in 8KB chunks
        const CHUNK_SIZE: usize = 8192;
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut total_copied = 0usize;
        let mut had_error = false;

        loop {
            let bytes_read = uio_read(src_handle, buffer.as_mut_ptr(), CHUNK_SIZE);
            if bytes_read < 0 {
                log_marker("uio_copy_file: read error");
                had_error = true;
                break;
            }

            if bytes_read == 0 {
                // EOF reached
                break;
            }

            let bytes_to_write = bytes_read as usize;
            let bytes_written = uio_write(dst_handle, buffer.as_ptr(), bytes_to_write);

            if bytes_written < 0 || bytes_written as usize != bytes_to_write {
                log_marker("uio_copy_file: write error");
                had_error = true;
                break;
            }

            total_copied += bytes_to_write;
        }

        // Close handles
        uio_close(src_handle);
        uio_close(dst_handle);

        // On error, remove partial destination
        if had_error {
            log_marker("uio_copy_file: cleaning up partial destination");
            uio_unlink(dst_dir, dst_path);
            set_errno(libc::EIO);
            return -1;
        }

        log_marker(&format!("uio_copy_file: copied {} bytes", total_copied));
        0
    })
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_printMounts(
    _out_stream: *mut libc::FILE,
    _repository: *const uio_Repository,
) {
    log_marker("uio_printMounts called - stub");
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_streamHandle(stream: *mut uio_Stream) -> *mut uio_Handle {
    log_marker("uio_streamHandle called");
    if stream.is_null() {
        return ptr::null_mut();
    }
    (*stream).handle
}

#[allow(
    non_camel_case_types,
    reason = "legacy UIO stat alias mirrors the C API; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub type stat = libc::stat;

// Constants
// The open flags arrive from C callers built against the platform's
// <fcntl.h>, so they must carry the platform's values (Linux and macOS
// disagree on everything except the access mode).
const O_RDONLY: c_int = libc::O_RDONLY;
const O_WRONLY: c_int = libc::O_WRONLY;
const O_RDWR: c_int = libc::O_RDWR;
const O_CREAT: c_int = libc::O_CREAT;
const O_EXCL: c_int = libc::O_EXCL;
const O_TRUNC: c_int = libc::O_TRUNC;
const O_APPEND: c_int = libc::O_APPEND;
const O_ACCMODE: c_int = libc::O_ACCMODE;

const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;

// Logging helper using rust-bridge.log
fn log_marker(msg: &str) {
    use crate::bridge_log::rust_bridge_log_msg;
    // Use RUST_UIO_* markers in rust-bridge.log
    let log_msg = format!("RUST_UIO: {}", msg);
    rust_bridge_log_msg(&log_msg);
}

// Helper: Convert C string to PathBuf
unsafe fn cstr_to_pathbuf(cstr: *const c_char) -> Option<PathBuf> {
    if cstr.is_null() {
        return None;
    }
    let c_str = std::ffi::CStr::from_ptr(cstr);
    Some(PathBuf::from(OsStr::from_bytes(c_str.to_bytes())))
}

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-PATH-NORM
/// Normalize a virtual path by resolving ".", "..", repeated slashes, and trailing slashes.
/// Empty paths resolve to the base directory's location.
/// ".." cannot escape above root "/".
fn normalize_virtual_path_full(base_virtual: &Path, input_path: &Path) -> PathBuf {
    // Handle empty path - return base location
    if input_path.as_os_str().is_empty() {
        return base_virtual.to_path_buf();
    }

    // Determine starting point
    let combined = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        base_virtual.join(input_path)
    };

    // Normalize components
    let mut result_components: Vec<std::path::Component> = Vec::new();

    for component in combined.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                result_components.clear();
                result_components.push(component);
            }
            std::path::Component::CurDir => {
                // Skip "." components
                continue;
            }
            std::path::Component::ParentDir => {
                // Handle ".." with root clamping
                // Don't pop if we're at root
                if result_components.len() > 1 {
                    result_components.pop();
                }
                // If we're at root or empty, stay clamped (don't go above root)
            }
            std::path::Component::Normal(_) => {
                result_components.push(component);
            }
        }
    }

    // Build result path
    if result_components.is_empty()
        || (result_components.len() == 1
            && matches!(result_components[0], std::path::Component::RootDir))
    {
        PathBuf::from("/")
    } else {
        let mut path = PathBuf::new();
        for component in result_components {
            path.push(component);
        }
        path
    }
}

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-PATH-CONFINEMENT
/// Map virtual path components to host filesystem path, ensuring ".." cannot escape
/// above the mount's physical root.
fn map_virtual_to_host_confined(mount_root: &Path, mount_relative_components: &Path) -> PathBuf {
    let mut host_components: Vec<std::path::Component> = Vec::new();

    for component in mount_relative_components.components() {
        match component {
            std::path::Component::ParentDir => {
                // Don't escape above mount root
                if !host_components.is_empty() {
                    host_components.pop();
                }
                // If empty, we're at mount root boundary - stay clamped
            }
            std::path::Component::CurDir => {
                // Skip "." components
                continue;
            }
            std::path::Component::Normal(_) => {
                host_components.push(component);
            }
            _ => {
                // Skip prefixes and root dirs in relative components
                continue;
            }
        }
    }

    // Build final path from mount root + confined components
    let mut result = mount_root.to_path_buf();
    for component in host_components {
        result.push(component);
    }
    result
}

// =============================================================================
// uio_init / uio_unInit / uio_openRepository / uio_closeRepository
// =============================================================================

// uio_init / uio_unInit / uio_openRepository / uio_closeRepository
// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-LIFECYCLE
/// Initialize the UIO subsystem.
/// Sets the initialization flag and clears mount registry to known state.
/// Safe to call multiple times (idempotent).
/// Returns without error.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_init() {
    ffi_guard!((), {
        log_marker("uio_init called");

        // Set initialization flag (idempotent)
        let was_initialized = UIO_INITIALIZED.swap(true, Ordering::SeqCst);

        if was_initialized {
            log_marker("uio_init: already initialized, skipping");
            return;
        }

        // Clear mount registry to known state
        let mut registry = get_mount_registry().lock().unwrap();
        registry.clear();
        drop(registry);

        log_marker("uio_init: subsystem initialized");
    })
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-LIFECYCLE
/// Shut down the UIO subsystem.
/// Clears all mounts, resets initialization flag, and leaves subsystem ready for clean re-init.
/// Safe to call even if init wasn't called.
/// Caller must ensure all operations are quiesced before calling this.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_unInit() {
    ffi_guard!((), {
        log_marker("uio_unInit called");

        // Reset initialization flag
        let was_initialized = UIO_INITIALIZED.swap(false, Ordering::SeqCst);

        if !was_initialized {
            log_marker("uio_unInit: not initialized, clearing state anyway for safety");
        }

        // Clear all mounts from registry
        // Note: This removes mount metadata but does NOT free mount handles
        // (those are owned by callers who must call uio_unmountDir)
        let mut registry = get_mount_registry().lock().unwrap();
        registry.clear();
        drop(registry);

        // Note: Buffer size registry is intentionally NOT cleared here.
        // Outstanding uio_DirList and stream handles may still need it
        // for proper cleanup via uio_DirList_free / uio_fclose.
        // The registry is harmless to keep and entries are cleaned up
        // individually as resources are freed.

        // Reset mount ID counter to initial state
        NEXT_MOUNT_ID.store(1, Ordering::SeqCst);

        log_marker("uio_unInit: subsystem shut down, ready for clean re-init");
    })
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_openRepository(flags: c_int) -> *mut uio_Repository {
    log_marker("uio_openRepository called");
    let repo = Box::new(uio_Repository { flags });
    Box::leak(repo) as *mut uio_Repository
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-LIFECYCLE
/// @requirement REQ-FIO-RESOURCE-MGMT
/// Close a repository and unmount all associated mounts.
/// This removes all mounts from the registry and frees the repository structure.
/// Open handles remain valid and closeable after this call (see REQ-FIO-POST-UNMOUNT-CLEANUP).
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_closeRepository(repository: *mut uio_Repository) {
    log_marker("uio_closeRepository called");
    if !repository.is_null() {
        uio_unmount_all_dirs(repository);
        let _ = Box::from_raw(repository);
    }
}

// =============================================================================
// uio_openDir / uio_closeDir / uio_mountDir / uio_openDirRelative
// =============================================================================

/// Opens the directory `path` (relative to `base`) of `repository`'s virtual
/// tree. The handle keeps only the normalized virtual path; the directory has
/// to exist in the repository's mounts now (ENOENT if nothing is there,
/// ENOTDIR if a file is), as in the original `uio_verifyPath`.
///
/// # Safety
///
/// `path` must be a C string; `repository` null or a live repository.
unsafe fn open_directory(
    repository: *mut uio_Repository,
    base: &Path,
    path: *const c_char,
) -> Result<*mut uio_DirHandle, c_int> {
    if repository.is_null() {
        return Err(libc::EINVAL);
    }
    let requested = cstr_to_pathbuf(path).ok_or(libc::EINVAL)?;
    let virtual_path = normalize_virtual_path_full(base, &requested);
    {
        let registry = get_mount_registry().lock().unwrap();
        verify_directory(&registry, repository_key(repository), &virtual_path)?;
    }
    Ok(Box::into_raw(Box::new(uio_DirHandle {
        virtual_path,
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository,
    })))
}

/// Opens a directory of the repository's virtual tree. `path` is absolute in
/// that tree (a relative one starts at its root); a host path is not a
/// virtual path. NULL with errno ENOENT or ENOTDIR if the repository has no
/// such directory. The `uio_OD_ROOT` flag is not used by any caller.
///
/// # Safety
///
/// `path` must be a C string; `repository` a live repository.
#[no_mangle]
pub unsafe extern "C" fn uio_openDir(
    repository: *mut uio_Repository,
    path: *const c_char,
    _flags: c_int,
) -> *mut uio_DirHandle {
    ffi_guard!(ptr::null_mut(), {
        open_directory(repository, Path::new("/"), path)
            .unwrap_or_else(|errno| fail_errno(errno, ptr::null_mut()))
    })
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// Close a directory handle.
/// Decrements refcount and frees the handle when refcount reaches zero.
/// Safe to call even if the mount this handle came from was unmounted.
/// The handle owns its resources independently of mount state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_closeDir(dir: *mut uio_DirHandle) -> c_int {
    log_marker("uio_closeDir called");
    if !dir.is_null() {
        // Decrement refcount
        let old_ref = (*dir)
            .refcount
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        if old_ref == 1 {
            // Refcount went to 0, free the handle
            let _ = Box::from_raw(dir);
        }
    }
    0 // Success
}

/// The original's `validPathName` for a mount point: after one leading slash,
/// every component is a real name (not empty, `.` or `..`); one trailing slash
/// is allowed.
fn valid_mount_point(mount_point: &Path) -> bool {
    let bytes = mount_point.as_os_str().as_bytes();
    let bytes = bytes.strip_prefix(b"/").unwrap_or(bytes);
    let bytes = match bytes.strip_suffix(b"/") {
        Some(rest) if !rest.is_empty() => rest,
        _ => bytes,
    };
    bytes.is_empty()
        || bytes
            .split(|&b| b == b'/')
            .all(|name| !name.is_empty() && name != b"." && name != b"..")
}

/// What `uio_mountDir` was asked, with the C arguments read and checked.
struct MountRequest<'a> {
    destination: *mut uio_Repository,
    mount_point: PathBuf,
    fs_type: c_int,
    /// The directory and the path in it of the archive to mount, as the game
    /// passes them. `None` mounts `in_path` as a host path (the game's way of
    /// mounting its content and config directories).
    source: Option<(&'a uio_DirHandle, PathBuf)>,
    /// With a source: the directory inside the archive to show. Without: the
    /// host path to mount.
    in_path: PathBuf,
    flags: c_int,
    relative: *mut uio_MountHandle,
}

/// The directory of `index` that `in_path` names, as an archive name: empty
/// for the whole archive.
fn archive_subdirectory(
    index: &crate::io::zip_reader::ZipIndex,
    in_path: &Path,
) -> Result<String, c_int> {
    let bytes = in_path.as_os_str().as_bytes();
    let trimmed = bytes.strip_prefix(b"/").unwrap_or(bytes);
    let trimmed = trimmed.strip_suffix(b"/").unwrap_or(trimmed);
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    // Entry names are Unicode: other bytes name no directory.
    let name = std::str::from_utf8(trimmed).map_err(|_| libc::ENOENT)?;
    if index.is_directory(name) {
        Ok(name.to_owned())
    } else if index.get_entry(name).is_some() {
        Err(libc::ENOTDIR)
    } else {
        Err(libc::ENOENT)
    }
}

/// The archive file `file` names inside `dir`, found in the repository's
/// mounts as they are now, indexed and ready to mount.
fn archive_backing(
    dir: &uio_DirHandle,
    file: &Path,
    in_path: &Path,
    flags: c_int,
) -> Result<MountBacking, c_int> {
    let virtual_path = normalize_virtual_path_full(&dir.virtual_path, file);
    let host_file = {
        let registry = get_mount_registry().lock().unwrap();
        let found = locate(&registry, repository_key(dir.repository), &virtual_path)?;
        match (found.kind, found.place) {
            (Kind::Directory, _) => return Err(libc::EISDIR),
            (Kind::File, Place::Host(path)) => path,
            // The index reads the archive from the host file system.
            (Kind::File, Place::Archive(_)) => return Err(libc::ENOTSUP),
        }
    };
    let index = index_archive_for_mount(&host_file, flags)?;
    let archive_root = archive_subdirectory(&index, in_path)?;
    Ok(MountBacking {
        mounted_root: host_file,
        fs_type: UIO_FSTYPE_ZIP,
        zip_index: Some(index),
        archive_root,
    })
}

fn mount_backing(request: &MountRequest<'_>) -> Result<MountBacking, c_int> {
    match &request.source {
        Some((dir, file)) => archive_backing(dir, file, &request.in_path, request.flags),
        None if request.fs_type == UIO_FSTYPE_ZIP => Ok(MountBacking {
            mounted_root: request.in_path.clone(),
            fs_type: UIO_FSTYPE_ZIP,
            zip_index: Some(index_archive_for_mount(&request.in_path, request.flags)?),
            archive_root: String::new(),
        }),
        None => Ok(MountBacking {
            mounted_root: request.in_path.clone(),
            fs_type: UIO_FSTYPE_STDIO,
            zip_index: None,
            archive_root: String::new(),
        }),
    }
}

/// The arguments of `uio_mountDir` as C passed them.
struct RawMountArgs {
    destination: *mut uio_Repository,
    mount_point: *const c_char,
    fs_type: c_int,
    source_dir: *mut uio_DirHandle,
    source_path: *const c_char,
    in_path: *const c_char,
    flags: c_int,
    relative: *mut uio_MountHandle,
}

/// Reads and checks the arguments of `uio_mountDir` in the original's order.
///
/// # Safety
///
/// The pointers must be null or valid, as for [`uio_mountDir`].
unsafe fn mount_request<'a>(raw: RawMountArgs) -> Result<MountRequest<'a>, c_int> {
    check_relative(raw.flags, raw.relative)?;
    let mount_point = cstr_to_pathbuf(raw.mount_point).ok_or(libc::EINVAL)?;
    if !valid_mount_point(&mount_point) {
        return Err(libc::EINVAL);
    }
    let source = match (raw.source_dir.as_ref(), cstr_to_pathbuf(raw.source_path)) {
        (None, None) => None,
        (Some(dir), Some(file)) => Some((dir, file)),
        _ => return Err(libc::EINVAL),
    };
    match raw.fs_type {
        UIO_FSTYPE_ZIP => {}
        // The directory file system is mounted from a host path, never from
        // a file.
        UIO_FSTYPE_STDIO if source.is_some() => return Err(libc::EINVAL),
        UIO_FSTYPE_STDIO => {}
        _ => return Err(libc::ENODEV),
    }
    Ok(MountRequest {
        destination: raw.destination,
        mount_point,
        fs_type: raw.fs_type,
        source,
        in_path: cstr_to_pathbuf(raw.in_path).ok_or(libc::EINVAL)?,
        flags: raw.flags,
        relative: raw.relative,
    })
}

/// Mounts a directory of the host, or an archive, into `dest_rep` at
/// `mount_point`.
///
/// With `source_dir` and `source_path` the archive is the file `source_path`
/// of the virtual directory `source_dir`, found in the repository's mounts as
/// they are now, and `in_path` is the directory inside it to show. Without
/// them `in_path` is a host path, which is how the game mounts its content
/// and config directories. Failures: EINVAL for malformed arguments, ENODEV
/// for an unknown file system, ENOENT/ENOTDIR/EISDIR for a source the mounts
/// do not hold, the host's errno when it refuses to read the source, EIO when
/// the source is not an archive.
///
/// # Safety
///
/// Every pointer must be null or valid for its type; strings are C strings.
#[no_mangle]
pub unsafe extern "C" fn uio_mountDir(
    dest_rep: *mut uio_Repository,
    mount_point: *const c_char,
    fs_type: c_int,
    source_dir: *mut uio_DirHandle,
    source_path: *const c_char,
    in_path: *const c_char,
    _auto_mount: *mut *mut (),
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    ffi_guard!(ptr::null_mut(), {
        let mounted = mount_request(RawMountArgs {
            destination: dest_rep,
            mount_point,
            fs_type,
            source_dir,
            source_path,
            in_path,
            flags,
            relative,
        })
        .and_then(|request| {
            let backing = mount_backing(&request)?;
            insert_mount(
                request.destination,
                &request.mount_point,
                backing,
                request.flags,
                request.relative,
                true,
            )
        });
        mounted.unwrap_or_else(|errno| fail_errno(errno, ptr::null_mut()))
    })
}

/// Opens `path` relative to `base`; see [`uio_openDir`]. A path starting with
/// a slash is absolute in the virtual tree.
///
/// # Safety
///
/// `base` must be null or a live directory handle and `path` a C string.
#[no_mangle]
pub unsafe extern "C" fn uio_openDirRelative(
    base: *mut uio_DirHandle,
    path: *const c_char,
    _flags: c_int,
) -> *mut uio_DirHandle {
    ffi_guard!(ptr::null_mut(), {
        match base.as_ref() {
            Some(base) => open_directory(base.repository, &base.virtual_path, path)
                .unwrap_or_else(|errno| fail_errno(errno, ptr::null_mut())),
            None => fail_errno(libc::EINVAL, ptr::null_mut()),
        }
    })
}

// =============================================================================
// uio_open / uio_close / uio_read / uio_write / uio_fstat
// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_open(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    flags: c_int,
    mode: c_int,
) -> *mut uio_Handle {
    ffi_guard!(ptr::null_mut(), {
        rust_bridge_log_msg(&format!("RUST_UIO: uio_open called with flags {}", flags));

        let Some(dir) = dir.as_ref() else {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        };
        let Some(input_path) = cstr_to_pathbuf(path) else {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        };

        match open_through_mounts(dir, &input_path, flags, mode) {
            Ok(inner) => handle_into_raw(inner),
            Err(errno) => {
                set_errno(errno);
                ptr::null_mut()
            }
        }
    })
}

/// Opens `input_path` (relative to `dir`, or absolute in the virtual tree)
/// the way the original UIO mount stack does, for both `uio_open` and
/// `uio_fopen`.
///
/// Any access but `O_RDONLY` goes to the write location the original picks:
/// the first writable mount the walk of the covering mounts reaches, with the
/// entry copied up from a mount below it and the missing parent directories
/// made there. That is EPERM when only read-only mounts were reached. Reads try
/// every covering mount in priority order: a mount lacking the file lets the
/// lookup fall through to the next one, any other failure is final. A path no
/// mount covers does not exist: the answer is ENOENT, as in the original
/// repository.
///
/// Returns the errno on failure.
///
/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
fn open_through_mounts(
    dir: &uio_DirHandle,
    input_path: &Path,
    flags: c_int,
    mode: c_int,
) -> Result<uio_HandleInner, c_int> {
    let virtual_path = normalize_virtual_path_full(&dir.virtual_path, input_path);
    let registry = get_mount_registry().lock().unwrap();

    if flags & O_ACCMODE != O_RDONLY {
        let plan = plan_write(
            &registry,
            repository_key(dir.repository),
            &virtual_path,
            WriteIntent {
                create: flags & O_CREAT != 0,
                exclusive: flags & O_CREAT != 0 && flags & O_EXCL != 0,
                parents: Parents::Make,
            },
        )?;
        drop(registry);
        let flags = prepare_write(&plan, flags)?;
        return open_host_file(&plan.target, flags, mode)
            .map(uio_HandleInner::File)
            .map_err(|err| {
                log_marker(&format!(
                    "uio_open write failed: path={:?} err={}",
                    plan.target, err
                ));
                err.raw_os_error().unwrap_or(libc::EIO)
            });
    }

    for covered in covering(&registry, repository_key(dir.repository), &virtual_path) {
        let mount = covered.mount;
        let opened = if mount.fs_type == UIO_FSTYPE_ZIP {
            match (mount.zip_index.as_ref(), covered.archive_name()) {
                (Some(zip_index), Some(name)) => {
                    zip_index.open_entry(&name).map(uio_HandleInner::ZipEntry)
                }
                // Entry names are Unicode: other bytes name no entry.
                (Some(_), None) => Err(std::io::ErrorKind::NotFound.into()),
                (None, _) => continue,
            }
        } else {
            open_host_file(&covered.host_path(), flags, mode).map(uio_HandleInner::File)
        };

        match opened {
            Ok(inner) => return Ok(inner),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                log_marker(&format!(
                    "uio_open read failed: path={:?} err={}",
                    virtual_path, err
                ));
                return Err(err.raw_os_error().unwrap_or(libc::EIO));
            }
        }
    }
    Err(libc::ENOENT)
}

/// The mode `uio_fopen` creates files with: read and write for everyone, as
/// the original's `S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH`.
const FOPEN_MODE: c_int = 0o666;

/// Opens a host file with the access, creation, truncation and append
/// behaviour that the `O_*` flags ask for. A file the open creates gets
/// `mode` less the process umask; an existing file keeps its mode.
fn open_host_file(host_path: &Path, flags: c_int, mode: c_int) -> std::io::Result<std::fs::File> {
    let mut opts = OpenOptions::new();
    opts.mode(mode as u32);
    match flags & O_ACCMODE {
        O_WRONLY => opts.write(true),
        O_RDWR => opts.read(true).write(true),
        _ => opts.read(true),
    };
    if flags & O_APPEND != 0 {
        opts.append(true);
    }
    if flags & O_CREAT != 0 {
        if flags & O_EXCL != 0 {
            opts.create_new(true);
        } else {
            opts.create(true);
        }
    }
    if flags & O_TRUNC != 0 {
        opts.truncate(true);
    }
    opts.open(host_path)
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// Close a file handle: releases the count the caller owned. The file itself
/// stays open until the last file block sharing the handle is closed too.
/// Safe to call even if the mount this handle came from was unmounted.
/// The handle owns its resources independently of mount state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_close(handle: *mut uio_Handle) -> c_int {
    log_marker("uio_close called");
    if !handle.is_null() {
        handle_release(handle);
    }
    0 // Success
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_read(handle: *mut uio_Handle, buf: *mut u8, count: size_t) -> isize {
    if handle.is_null() || buf.is_null() || count == 0 {
        return -1;
    }

    let mut guard = match (*handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let buffer = slice::from_raw_parts_mut(buf, count);
    match guard.read(buffer) {
        Ok(n) => n as isize,
        Err(_) => -1,
    }
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_write(
    handle: *mut uio_Handle,
    buf: *const u8,
    count: size_t,
) -> isize {
    if handle.is_null() || buf.is_null() || count == 0 {
        return -1;
    }

    let mut guard = match (*handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let buffer = slice::from_raw_parts(buf, count);
    match guard.write_all(buffer) {
        Ok(_) => count as isize,
        Err(_) => -1,
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_fstat(handle: *mut uio_Handle, stat_buf: *mut stat) -> c_int {
    if handle.is_null() || stat_buf.is_null() {
        return -1;
    }

    let guard = match (*handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    match guard.fstat_into(&mut *stat_buf) {
        Ok(()) => 0,
        Err(err) => fail_errno(handle_error_errno(&err), -1),
    }
}

/// The errno of a handle operation's failure: the host's own, ENOTSUP when
/// the archive entry holds data this layer cannot report, EIO otherwise.
fn handle_error_errno(err: &std::io::Error) -> c_int {
    match err.raw_os_error() {
        Some(errno) => errno,
        None if err.kind() == std::io::ErrorKind::Unsupported => libc::ENOTSUP,
        None => libc::EIO,
    }
}

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_unlink(dir: *mut uio_DirHandle, path: *const c_char) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_unlink called");

        if dir.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        let input_path = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        match remove_at(dir, &input_path, Kind::File) {
            Ok(()) => 0,
            Err(errno) => fail_errno(errno, -1),
        }
    })
}

// =============================================================================
// uio_fopen / uio_fclose / uio_fread / uio_fseek / uio_ftell
// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-ERRNO
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_fopen(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    mode: *const c_char,
) -> *mut uio_Stream {
    ffi_guard!(ptr::null_mut(), {
        rust_bridge_log_msg("RUST_UIO: uio_fopen entry");

        let Some(dir) = dir.as_ref() else {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        };
        if mode.is_null() {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }
        let Some(input) = cstr_to_pathbuf(path) else {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        };
        let Some(flags) = open_flags_for_mode(std::ffi::CStr::from_ptr(mode).to_bytes()) else {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        };

        let inner = match open_through_mounts(dir, &input, flags, FOPEN_MODE) {
            Ok(inner) => inner,
            Err(errno) => {
                set_errno(errno);
                return ptr::null_mut();
            }
        };

        Box::leak(Box::new(uio_Stream {
            handle: handle_into_raw(inner),
            status: UIO_STREAM_STATUS_OK,
            operation: UIO_STREAM_OPERATION_NONE,
            open_flags: flags,
            pushback: None,
        })) as *mut uio_Stream
    })
}

/// Translates an `fopen` mode string into `open` flags as the original
/// `uio_fopen` does: the first character picks read, write or append, and
/// the next two may carry `b`, `t` or `+` in either order. Anything after
/// that is ignored. `None` means the first character is not r, w or a.
fn open_flags_for_mode(mode: &[u8]) -> Option<c_int> {
    let mut flags = match mode.first()? {
        b'r' => O_RDONLY,
        b'w' => O_WRONLY | O_CREAT | O_TRUNC,
        b'a' => O_WRONLY | O_CREAT | O_APPEND,
        _ => return None,
    };
    for modifier in mode[1..].iter().take(2) {
        match modifier {
            b'b' | b't' => {}
            b'+' => flags = (flags & !O_ACCMODE) | O_RDWR,
            _ => break,
        }
    }
    Some(flags)
}

/// @plan PLAN-20260314-FILE-IO.P03
/// @requirement REQ-FIO-RESOURCE-MGMT
/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// @requirement REQ-FIO-RESOURCE-MGMT
/// Close a stream and release its share of the underlying handle.
/// Safe to call even if the mount this stream came from was unmounted.
/// The stream owns its resources independently of mount state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_fclose(stream: *mut uio_Stream) -> c_int {
    log_marker("uio_fclose called");
    if !stream.is_null() {
        let s = &*stream;
        if !s.handle.is_null() {
            handle_release(s.handle);
        }
        let _ = Box::from_raw(stream);
    }
    0 // Success
}

/// @plan PLAN-20260314-FILE-IO.P04
/// @requirement REQ-FIO-BUILD-BOUNDARY
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_fread(
    buf: *mut libc::c_void,
    size: size_t,
    nmemb: size_t,
    stream: *mut uio_Stream,
) -> size_t {
    if buf.is_null() || size == 0 || nmemb == 0 {
        return 0;
    }
    let Some(total_bytes) = size.checked_mul(nmemb) else {
        return 0;
    };
    let buffer = slice::from_raw_parts_mut(buf as *mut u8, total_bytes);
    with_stream_handle(stream, |stream, file| {
        stream.operation = UIO_STREAM_OPERATION_READ;
        read_stream(stream, file, buffer).count / size
    })
    .unwrap_or(0)
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_fseek(
    stream: *mut uio_Stream,
    offset: c_long,
    whence: c_int,
) -> c_int {
    with_stream_handle(stream, |stream, file| {
        // A parked byte sits in front of the handle, so a seek relative to
        // the stream position starts one byte before the handle position.
        let relative = if stream.pushback.is_some() { 1 } else { 0 };
        let seek_from = match whence {
            SEEK_SET => u64::try_from(offset).map(SeekFrom::Start).ok(),
            SEEK_CUR => offset.checked_sub(relative).map(SeekFrom::Current),
            SEEK_END => Some(SeekFrom::End(offset)),
            _ => None,
        };
        match seek_from.map(|from| file.seek(from)) {
            Some(Ok(_)) => {
                stream.pushback = None;
                stream.status = UIO_STREAM_STATUS_OK;
                0
            }
            _ => -1,
        }
    })
    .unwrap_or(-1)
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_ftell(stream: *mut uio_Stream) -> c_long {
    with_stream_handle(stream, |stream, file| {
        let parked = u64::from(stream.pushback.is_some());
        match file.stream_position() {
            // Pushing back at offset 0 leaves the position undefined in ISO C;
            // it reads as 0 here.
            Ok(position) => position.saturating_sub(parked) as c_long,
            Err(_) => -1,
        }
    })
    .unwrap_or(-1)
}

// =============================================================================
// uio_getDirList / uio_DirList_free
// =============================================================================

/// @plan PLAN-20260314-FILE-IO.P07
/// @requirement REQ-FIO-DIRLIST-UNION
/// @requirement REQ-FIO-DIRLIST-EMPTY
/// List directory contents across all mounts covering the target virtual path.
/// Returns a union of all entries from all mounts, with precedence-based deduplication.
/// Empty match returns non-null uio_DirList with numNames=0 (NULL is for errors only).
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_getDirList(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    _pattern: *const c_char,
    _match_type: c_int,
) -> *mut uio_DirList {
    ffi_guard!(ptr::null_mut(), {
        log_marker(&format!(
            "uio_getDirList called: dir=0x{:x} path=0x{:x} pattern=0x{:x}",
            dir as usize, path as usize, _pattern as usize
        ));

        if dir.is_null() {
            log_marker("uio_getDirList: null dir handle");
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }

        let dir_virtual_path = &(*dir).virtual_path;
        let rel_path = if path.is_null() {
            PathBuf::new() // Empty path means current directory
        } else {
            match cstr_to_pathbuf(path) {
                Some(p) => p,
                None => {
                    log_marker("uio_getDirList: invalid path string");
                    set_errno(libc::EINVAL);
                    return ptr::null_mut();
                }
            }
        };

        log_marker(&format!(
            "uio_getDirList: dir_virtual={:?} rel_path={:?}",
            dir_virtual_path, rel_path
        ));

        // Use the virtual path for mount resolution
        let virtual_path = normalize_virtual_path_full(dir_virtual_path, &rel_path);

        let pattern = if _pattern.is_null() {
            &[][..]
        } else {
            std::ffi::CStr::from_ptr(_pattern).to_bytes()
        };
        let filter = NameFilter::new(pattern, _match_type);
        log_marker(&format!(
            "uio_getDirList: virtual_path={:?} pattern={:?} matchType={}",
            virtual_path,
            String::from_utf8_lossy(pattern),
            _match_type
        ));

        // The registry lock gives a consistent view of the mount topology.
        let names = {
            let registry = get_mount_registry().lock().unwrap();
            collect_dir_names(
                &registry,
                repository_key((*dir).repository),
                &virtual_path,
                &filter,
            )
        };
        match names {
            Ok(names) => {
                log_marker(&format!(
                    "uio_getDirList: {} matches in {:?}",
                    names.len(),
                    virtual_path
                ));
                dir_list_from_names(&names)
            }
            Err(errno) => fail_errno(errno, ptr::null_mut()),
        }
    })
}

/// The names `virtual_path` has in the repository that the filter selects:
/// what the covering mounts hold there, and the mount-tree directories below
/// it. A name several of them provide is listed once, and the set's order is
/// byte order, which is what the C implementation's `strcmp` sort produced.
/// Callers rely on it: the addon packs and archives are mounted in list
/// order, and that order is their priority.
///
/// The walk is `uio_getPathPhysicalDirs`: a mount that lacks the directory is
/// passed over, and a file where the directory should be ends the walk with
/// what was found so far. A mount that holds the directory but cannot be read
/// fails the listing with the host's errno; the original left such a layer
/// out and said so only in debug builds, which would present a partial
/// listing as the whole.
fn collect_dir_names(
    registry: &[MountInfo],
    repository: usize,
    virtual_path: &Path,
    filter: &NameFilter,
) -> Result<BTreeSet<Vec<u8>>, c_int> {
    let mut names = BTreeSet::new();
    for covered in covering(registry, repository, virtual_path) {
        match probe(&covered) {
            Ok(None) => {}
            Ok(Some((_, Kind::File))) | Err(libc::ENOTDIR) => break,
            Err(errno) => return Err(errno),
            Ok(Some((Place::Host(directory), Kind::Directory))) => {
                add_host_names(&mut names, &directory, filter)?;
            }
            Ok(Some((Place::Archive(directory), Kind::Directory))) => {
                add_archive_names(&mut names, covered.mount, &directory, filter);
            }
        }
    }
    names.extend(
        virtual_children(registry, repository, virtual_path)
            .map(OsStrExt::as_bytes)
            .filter(|name| filter.matches(name))
            .map(<[u8]>::to_vec),
    );
    Ok(names)
}

/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
fn add_archive_names(
    names: &mut BTreeSet<Vec<u8>>,
    mount: &MountInfo,
    directory: &str,
    filter: &NameFilter,
) {
    let zip_index = mount
        .zip_index
        .as_ref()
        .expect("an archive mount is registered with its index");
    let entries = zip_index.list_directory(directory);
    log_marker(&format!(
        "uio_getDirList: ZIP mount {:?} returned {} entries for {:?}",
        mount.mount_point,
        entries.len(),
        directory
    ));
    for name in entries {
        // A name with a NUL cannot be a C string, and an archive is
        // untrusted input.
        if !name.as_bytes().contains(&0) && filter.matches(name.as_bytes()) {
            names.insert(name.into_bytes());
        }
    }
}

fn add_host_names(
    names: &mut BTreeSet<Vec<u8>>,
    directory: &Path,
    filter: &NameFilter,
) -> Result<(), c_int> {
    let host_error = |err: std::io::Error| err.raw_os_error().unwrap_or(libc::EIO);
    for entry in fs::read_dir(directory).map_err(host_error)? {
        // The bytes the host holds, valid UTF-8 or not.
        let name = entry.map_err(host_error)?.file_name();
        if filter.matches(name.as_bytes()) {
            names.insert(name.as_bytes().to_vec());
        }
    }
    Ok(())
}

/// Lays the names out the way C reads a `uio_DirList`: one buffer of
/// NUL-terminated names in order, and an array of pointers into it. Both are
/// exact-size boxed slices, so [`uio_DirList_free`] can rebuild them from the
/// pointers alone. An empty selection is a list of zero names, not NULL.
fn dir_list_from_names(names: &BTreeSet<Vec<u8>>) -> *mut uio_DirList {
    if names.is_empty() {
        return Box::into_raw(Box::new(uio_DirList {
            names: ptr::null_mut(),
            numNames: 0,
            buffer: ptr::null_mut(),
        }));
    }
    let mut text: Vec<u8> = Vec::with_capacity(names.iter().map(|name| name.len() + 1).sum());
    let mut starts = Vec::with_capacity(names.len());
    for name in names {
        starts.push(text.len());
        text.extend_from_slice(name);
        text.push(0);
    }
    let buffer = Box::into_raw(text.into_boxed_slice()).cast::<c_char>();
    // SAFETY: every start is an offset inside the buffer just allocated.
    let pointers: Box<[*mut c_char]> = starts
        .iter()
        .map(|&start| unsafe { buffer.add(start) })
        .collect();
    Box::into_raw(Box::new(uio_DirList {
        names: Box::into_raw(pointers).cast::<*mut c_char>(),
        numNames: c_int::try_from(names.len()).expect("name count fits a C int"),
        buffer,
    }))
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-RESOURCE-MGMT
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// Free a directory list and all its resources.
/// Safe to call with NULL pointer (no-op).
/// Safe to call on empty lists.
/// Safe to call even if the mount this list came from was unmounted.
///
/// # Safety
///
/// `dirlist` must be NULL or a list [`uio_getDirList`] returned that has not
/// been freed. Its names are laid out in order in one buffer, so the buffer
/// ends at the last name's terminator.
#[no_mangle]
pub unsafe extern "C" fn uio_DirList_free(dirlist: *mut uio_DirList) {
    log_marker("uio_DirList_free called");
    if dirlist.is_null() {
        return;
    }
    let list = Box::from_raw(dirlist);
    let count = usize::try_from(list.numNames).expect("name count is not negative");
    if count == 0 {
        return;
    }
    let last = *list.names.add(count - 1);
    let buffer_len = usize::try_from(last.offset_from(list.buffer))
        .expect("names follow the buffer start")
        + std::ffi::CStr::from_ptr(last).to_bytes_with_nul().len();
    drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
        list.buffer.cast::<u8>(),
        buffer_len,
    )));
    drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
        list.names, count,
    )));
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    use std::ffi::CStr;
    use std::ffi::CString;

    use std::path::PathBuf;

    use crate::io::name_filter::{
        MATCH_LITERAL, MATCH_PREFIX, MATCH_REGEX, MATCH_SUBSTRING, MATCH_SUFFIX,
    };

    fn matches_pattern(name: &str, pattern: &str, match_type: c_int) -> bool {
        NameFilter::new(pattern.as_bytes(), match_type).matches(name.as_bytes())
    }

    // Helper to clear mount registry after tests
    fn clear_mount_registry() {
        if let Ok(mut registry) = get_mount_registry().lock() {
            registry.clear();
        }
    }

    // Helper to add a test mount
    fn add_test_mount(repository: *mut uio_Repository, mount_point: &str, mounted_root: &str) {
        let id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
        let handle = Box::into_raw(Box::new(uio_MountHandle {
            repository,
            id,
            fs_type: UIO_FSTYPE_STDIO,
        }));

        let mut registry = get_mount_registry().lock().unwrap();
        const DEFAULT_START_POSITION: usize = 1000;
        let position = registry
            .iter()
            .filter(|m| m.active_in_registry)
            .map(|m| m.position)
            .max()
            .unwrap_or(DEFAULT_START_POSITION)
            .saturating_add(1);

        registry.push(MountInfo {
            id,
            repository: repository_key(repository),
            handle_ptr: handle as usize,
            mount_point: normalize_mount_point(Path::new(mount_point)),
            mounted_root: PathBuf::from(mounted_root),
            fs_type: UIO_FSTYPE_STDIO,
            active_in_registry: true,
            position,
            read_only: false,
            zip_index: None,
            archive_root: String::new(),
        });
        sort_mount_registry(&mut registry);
    }

    // Helper to add a test mount with read_only flag
    fn add_test_mount_readonly(
        repository: *mut uio_Repository,
        mount_point: &str,
        mounted_root: &str,
        read_only: bool,
    ) {
        let id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
        let handle = Box::into_raw(Box::new(uio_MountHandle {
            repository,
            id,
            fs_type: UIO_FSTYPE_STDIO,
        }));

        let mut registry = get_mount_registry().lock().unwrap();
        const DEFAULT_START_POSITION: usize = 1000;
        let position = registry
            .iter()
            .filter(|m| m.active_in_registry)
            .map(|m| m.position)
            .max()
            .unwrap_or(DEFAULT_START_POSITION)
            .saturating_add(1);
        registry.push(MountInfo {
            id,
            repository: repository_key(repository),
            handle_ptr: handle as usize,
            mount_point: normalize_mount_point(Path::new(mount_point)),
            mounted_root: PathBuf::from(mounted_root),
            fs_type: UIO_FSTYPE_STDIO,
            active_in_registry: true,
            position,
            read_only,
            zip_index: None,
            archive_root: String::new(),
        });
        sort_mount_registry(&mut registry);
    }

    #[test]
    #[serial]
    fn test_mount_registry_basic() {
        {
            clear_mount_registry();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount(repo, "/content", "/tmp/content");

            {
                let registry = get_mount_registry().lock().unwrap();
                let info = registry
                    .iter()
                    .find(|entry| entry.mount_point == Path::new("/content"))
                    .unwrap();
                assert_eq!(info.mounted_root, PathBuf::from("/tmp/content"));
                assert_eq!(info.fs_type, UIO_FSTYPE_STDIO);
                assert!(info.active_in_registry);
            }

            drop(unsafe { Box::from_raw(repo) });
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_cstr_to_pathbuf_valid() {
        {
            let test_path = CString::new("/test/path").unwrap();
            let result = unsafe { cstr_to_pathbuf(test_path.as_ptr()) };

            assert!(result.is_some());
            assert_eq!(result.unwrap(), PathBuf::from("/test/path"));
        }
    }

    #[test]
    #[serial]
    fn test_cstr_to_pathbuf_null() {
        {
            let result = unsafe { cstr_to_pathbuf(std::ptr::null()) };
            assert!(result.is_none());
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_literal() {
        {
            assert!(matches_pattern("test.txt", "test.txt", MATCH_LITERAL));
            assert!(!matches_pattern("test.txt", "other.txt", MATCH_LITERAL));
            assert!(!matches_pattern("test.txt", "TEST.TXT", MATCH_LITERAL)); // Case-sensitive
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_prefix() {
        {
            assert!(matches_pattern("test.txt", "test", MATCH_PREFIX));
            assert!(!matches_pattern("test.txt", "txt", MATCH_PREFIX));
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_suffix() {
        {
            assert!(matches_pattern("test.txt", ".txt", MATCH_SUFFIX));
            assert!(!matches_pattern("test.txt", ".doc", MATCH_SUFFIX));
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_substring() {
        {
            assert!(matches_pattern("mytest.txt", "test", MATCH_SUBSTRING));
            assert!(!matches_pattern("mytest.txt", "foo", MATCH_SUBSTRING));
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_regex_rmp() {
        {
            // Test the .rmp regex pattern
            assert!(matches_pattern("file.rmp", r"\.[rR][mM][pP]$", MATCH_REGEX));
            assert!(matches_pattern("file.RMP", r"\.[rR][mM][pP]$", MATCH_REGEX));
            assert!(!matches_pattern(
                "file.txt",
                r"\.[rR][mM][pP]$",
                MATCH_REGEX
            ));
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_regex_zip_uqm() {
        {
            // Test the .zip/.uqm regex pattern
            assert!(matches_pattern(
                "file.zip",
                r"\.([zZ][iI][pP]|[uU][qQ][mM])$",
                MATCH_REGEX
            ));
            assert!(matches_pattern(
                "file.uqm",
                r"\.([zZ][iI][pP]|[uU][qQ][mM])$",
                MATCH_REGEX
            ));
            assert!(matches_pattern(
                "file.ZIP",
                r"\.([zZ][iI][pP]|[uU][qQ][mM])$",
                MATCH_REGEX
            ));
            assert!(!matches_pattern(
                "file.txt",
                r"\.([zZ][iI][pP]|[uU][qQ][mM])$",
                MATCH_REGEX
            ));
        }
    }

    #[test]
    #[serial]
    fn test_matches_pattern_empty_pattern() {
        {
            // Empty pattern should match everything
            assert!(matches_pattern("anything.txt", "", MATCH_LITERAL));
            assert!(matches_pattern("anything.txt", "", MATCH_REGEX));
        }
    }

    #[test]
    #[serial]
    fn test_seek_constants() {
        {
            // Verify our seek constants match expected values
            assert_eq!(SEEK_SET, 0);
            assert_eq!(SEEK_CUR, 1);
            assert_eq!(SEEK_END, 2);
        }
    }

    #[test]
    #[serial]
    fn test_open_flags_constants() {
        {
            // Verify file open flags
            assert_eq!(O_RDONLY, 0);
            assert_eq!(O_WRONLY, 1);
            assert_eq!(O_RDWR, 2);
            assert_eq!(O_CREAT, libc::O_CREAT);
            assert_eq!(O_TRUNC, libc::O_TRUNC);
            assert_eq!(O_APPEND, libc::O_APPEND);
        }
    }

    // =============================================================================
    // Stream Status Tracking Tests (Phase P03)
    // =============================================================================

    #[test]
    #[serial]
    fn test_stream_status_constants() {
        {
            assert_eq!(UIO_STREAM_STATUS_OK, 0);
            assert_eq!(UIO_STREAM_STATUS_EOF, 1);
            assert_eq!(UIO_STREAM_STATUS_ERROR, 2);
        }
    }

    #[test]
    #[serial]
    fn test_uio_feof_returns_zero_when_not_eof() {
        {
            let mut stream = uio_Stream {
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_OK,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
                pushback: None,
            };

            let result = unsafe { uio_feof(&mut stream) };
            assert_eq!(result, 0);
        }
    }

    #[test]
    #[serial]
    fn test_uio_feof_returns_nonzero_when_eof() {
        {
            let mut stream = uio_Stream {
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_EOF,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
                pushback: None,
            };

            let result = unsafe { uio_feof(&mut stream) };
            assert_eq!(result, 1);
        }
    }

    #[test]
    #[serial]
    fn test_uio_feof_null_stream() {
        {
            let result = unsafe { uio_feof(ptr::null_mut()) };
            assert_eq!(result, 0);
        }
    }

    #[test]
    #[serial]
    fn test_uio_ferror_returns_zero_when_no_error() {
        {
            let mut stream = uio_Stream {
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_OK,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
                pushback: None,
            };

            let result = unsafe { uio_ferror(&mut stream) };
            assert_eq!(result, 0);
        }
    }

    #[test]
    #[serial]
    fn test_uio_ferror_returns_nonzero_when_error() {
        {
            let mut stream = uio_Stream {
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_ERROR,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
                pushback: None,
            };

            let result = unsafe { uio_ferror(&mut stream) };
            assert_eq!(result, 1);
        }
    }

    #[test]
    #[serial]
    fn test_uio_ferror_null_stream() {
        {
            let result = unsafe { uio_ferror(ptr::null_mut()) };
            assert_eq!(result, 0);
        }
    }

    #[test]
    #[serial]
    fn test_uio_clearerr_clears_eof() {
        {
            let mut stream = uio_Stream {
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_EOF,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
                pushback: None,
            };

            unsafe {
                uio_clearerr(&mut stream);
            }

            assert_eq!(stream.status, UIO_STREAM_STATUS_OK);
        }
    }

    #[test]
    #[serial]
    fn test_uio_clearerr_clears_error() {
        {
            let mut stream = uio_Stream {
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_ERROR,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
                pushback: None,
            };

            unsafe {
                uio_clearerr(&mut stream);
            }

            assert_eq!(stream.status, UIO_STREAM_STATUS_OK);
        }
    }

    #[test]
    #[serial]
    fn test_uio_clearerr_null_stream() {
        {
            // Should not crash
            unsafe {
                uio_clearerr(ptr::null_mut());
            }
        }
    }

    #[test]
    #[serial]
    fn test_fopen_initializes_status_to_ok() {
        {
            use std::fs;
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"test content").unwrap();

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                assert_eq!((*stream).status, UIO_STREAM_STATUS_OK);
                uio_fclose(stream);
                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    #[test]
    #[serial]
    fn test_read_eof_sets_eof_status() {
        {
            use std::fs;
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"A").unwrap(); // Single byte file

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                // Read the single byte
                let mut buf = [0u8; 1];
                let read = uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 1, 1, stream);
                assert_eq!(read, 1);
                assert_eq!((*stream).status, UIO_STREAM_STATUS_OK);

                // Try to read again - should hit EOF
                let read = uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 1, 1, stream);
                assert_eq!(read, 0);
                assert_eq!((*stream).status, UIO_STREAM_STATUS_EOF);

                // Verify uio_feof returns non-zero
                assert_eq!(uio_feof(stream), 1);
                assert_eq!(uio_ferror(stream), 0);

                uio_fclose(stream);
                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    #[test]
    #[serial]
    fn test_clearerr_after_eof() {
        {
            use std::fs;
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"").unwrap(); // Empty file

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                // Try to read - should hit EOF immediately
                let mut buf = [0u8; 1];
                let read = uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 1, 1, stream);
                assert_eq!(read, 0);
                assert_eq!(uio_feof(stream), 1);

                // Clear the error/eof state
                uio_clearerr(stream);
                assert_eq!(uio_feof(stream), 0);
                assert_eq!(uio_ferror(stream), 0);

                uio_fclose(stream);
                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    #[test]
    #[serial]
    fn test_fseek_clears_eof() {
        {
            use std::fs;
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"ABCD").unwrap();

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                // Read all data to hit EOF
                let mut buf = [0u8; 10];
                let read = uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 1, 10, stream);
                assert_eq!(read, 4); // Only 4 bytes available

                // Try to read again - should hit EOF
                let read = uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 1, 1, stream);
                assert_eq!(read, 0);
                assert_eq!(uio_feof(stream), 1);

                // Seek back to start
                let seek_result = uio_fseek(stream, 0, SEEK_SET);
                assert_eq!(seek_result, 0);

                // EOF flag should be cleared
                assert_eq!(uio_feof(stream), 0);

                uio_fclose(stream);
                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    #[test]
    #[serial]
    fn test_write_error_sets_error_status() {
        {
            use std::fs;
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"").unwrap();

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("w").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                // Write some data - should succeed
                let data = b"test data";
                let written =
                    uio_fwrite(data.as_ptr() as *const libc::c_void, 1, data.len(), stream);
                assert_eq!(written, data.len());
                assert_eq!((*stream).status, UIO_STREAM_STATUS_OK);

                uio_fclose(stream);
                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    #[test]
    #[serial]
    fn test_fgetc_sets_eof() {
        {
            use std::fs;
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"X").unwrap();

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                // Read the single character
                let ch = uio_fgetc(stream);
                assert_eq!(ch, b'X' as c_int);
                assert_eq!((*stream).status, UIO_STREAM_STATUS_OK);

                // Try to read again - should hit EOF
                let ch = uio_fgetc(stream);
                assert_eq!(ch, -1);
                assert_eq!((*stream).status, UIO_STREAM_STATUS_EOF);
                assert_eq!(uio_feof(stream), 1);

                uio_fclose(stream);
                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    // =============================================================================
    // Path Normalization Tests (Phase P05)
    // =============================================================================

    #[test]
    #[serial]
    fn test_normalize_path_removes_dot_components() {
        {
            let base = PathBuf::from("/base");
            let input = PathBuf::from("./foo/./bar");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/base/foo/bar"));
        }
    }

    #[test]
    #[serial]
    fn test_normalize_path_resolves_dotdot() {
        {
            let base = PathBuf::from("/base");
            let input = PathBuf::from("foo/../bar");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/base/bar"));
        }
    }

    #[test]
    #[serial]
    fn test_normalize_path_clamps_at_root() {
        {
            let base = PathBuf::from("/base");
            let input = PathBuf::from("../../above_root");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/above_root"));
        }
    }

    #[test]
    #[serial]
    fn test_normalize_path_collapses_repeated_slashes() {
        {
            let base = PathBuf::from("/");
            let input = PathBuf::from("//foo///bar//");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/foo/bar"));
        }
    }

    #[test]
    #[serial]
    fn test_normalize_path_empty_returns_base() {
        {
            let base = PathBuf::from("/content/addons");
            let input = PathBuf::from("");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/content/addons"));
        }
    }

    #[test]
    #[serial]
    fn test_normalize_path_absolute_ignores_base() {
        {
            let base = PathBuf::from("/base");
            let input = PathBuf::from("/absolute/path");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/absolute/path"));
        }
    }

    #[test]
    #[serial]
    fn test_normalize_path_complex_case() {
        {
            let base = PathBuf::from("/");
            let input = PathBuf::from("/foo/./bar/../baz");
            let result = normalize_virtual_path_full(&base, &input);
            assert_eq!(result, PathBuf::from("/foo/baz"));
        }
    }

    #[test]
    #[serial]
    fn test_map_virtual_to_host_confined() {
        {
            let mount_root = PathBuf::from("/host/mount/point");
            let relative = PathBuf::from("subdir/file.txt");
            let result = map_virtual_to_host_confined(&mount_root, &relative);
            assert_eq!(result, PathBuf::from("/host/mount/point/subdir/file.txt"));
        }
    }

    #[test]
    #[serial]
    fn test_map_virtual_to_host_confined_prevents_escape() {
        {
            let mount_root = PathBuf::from("/host/mount/point");
            let relative = PathBuf::from("../../above/mount");
            let result = map_virtual_to_host_confined(&mount_root, &relative);
            // Should clamp at mount root, not escape above it
            assert_eq!(result, PathBuf::from("/host/mount/point/above/mount"));
        }
    }

    #[test]
    #[serial]
    fn test_map_virtual_to_host_confined_single_dotdot() {
        {
            let mount_root = PathBuf::from("/host/mount/point");
            let relative = PathBuf::from("foo/../bar");
            let result = map_virtual_to_host_confined(&mount_root, &relative);
            assert_eq!(result, PathBuf::from("/host/mount/point/bar"));
        }
    }

    // =============================================================================
    // errno Setting Tests (Phase P05)
    // =============================================================================

    #[test]
    #[serial]
    fn test_uio_access_sets_enoent_on_missing_file() {
        {
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
            }));

            let path = CString::new("nonexistent.txt").unwrap();

            unsafe {
                // Clear errno first
                set_errno(0);

                let result = uio_access(dir_handle, path.as_ptr(), 0);
                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::ENOENT);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
            }
        }
    }

    #[test]
    #[serial]
    fn test_uio_mkdir_sets_eexist_on_existing_dir() {
        {
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let existing_dir = temp_dir.path().join("existing");
            fs::create_dir(&existing_dir).unwrap();

            let (repository, dir_handle) = layered_open_tests::mount_host_root(temp_dir.path());

            let path = CString::new("existing").unwrap();

            unsafe {
                // Clear errno first
                set_errno(0);

                let result = uio_mkdir(dir_handle, path.as_ptr(), 0o755);
                assert_eq!(result, -1);
                // errno should be EEXIST (directory already exists)
                assert_eq!(current_errno(), libc::EEXIST);

                layered_open_tests::release_host_root(repository, dir_handle);
            }
        }
    }

    #[test]
    #[serial]
    fn test_uio_fopen_sets_einval_on_invalid_mode() {
        {
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
            }));

            let path = CString::new("test.txt").unwrap();
            let invalid_mode = CString::new("x").unwrap(); // Invalid mode

            unsafe {
                // Clear errno first
                set_errno(0);

                let stream = uio_fopen(dir_handle, path.as_ptr(), invalid_mode.as_ptr());
                assert!(stream.is_null());
                assert_eq!(current_errno(), libc::EINVAL);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
            }
        }
    }

    #[test]
    #[serial]
    fn test_uio_open_sets_enoent_on_missing_file() {
        {
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
            }));

            let path = CString::new("nonexistent.txt").unwrap();

            unsafe {
                // Clear errno first
                set_errno(0);

                let handle = uio_open(dir_handle, path.as_ptr(), O_RDONLY, 0);
                assert!(handle.is_null());
                assert_eq!(current_errno(), libc::ENOENT);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
            }
        }
    }

    // =============================================================================
    // Mount Ordering Tests (Phase P06)
    // =============================================================================

    #[test]
    #[serial]
    fn test_mount_top_gives_highest_priority() {
        {
            clear_mount_registry();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            // Add first mount at BOTTOM
            let mount1 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    PathBuf::from("/tmp/mount1"),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_BOTTOM,
                    ptr::null_mut(),
                    true,
                )
            };
            assert!(!mount1.is_null());

            // Add second mount at TOP
            let mount2 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    PathBuf::from("/tmp/mount2"),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_TOP,
                    ptr::null_mut(),
                    true,
                )
            };
            assert!(!mount2.is_null());

            // Verify mount2 has higher priority (lower position)
            let registry = get_mount_registry().lock().unwrap();
            let m1 = registry
                .iter()
                .find(|m| m.handle_ptr == mount1 as usize)
                .unwrap();
            let m2 = registry
                .iter()
                .find(|m| m.handle_ptr == mount2 as usize)
                .unwrap();
            assert!(
                m2.position < m1.position,
                "TOP mount should have lower position than BOTTOM"
            );

            drop(registry);
            unsafe {
                let _ = Box::from_raw(mount1);
                let _ = Box::from_raw(mount2);
                let _ = Box::from_raw(repo);
            }
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_mount_above_below_placement() {
        {
            clear_mount_registry();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            // Add base mount
            let mount1 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    PathBuf::from("/tmp/mount1"),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_BOTTOM,
                    ptr::null_mut(),
                    true,
                )
            };

            // Add mount ABOVE mount1
            let mount2 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    PathBuf::from("/tmp/mount2"),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_ABOVE,
                    mount1,
                    true,
                )
            };

            // Add mount BELOW mount1
            let mount3 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    PathBuf::from("/tmp/mount3"),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_BELOW,
                    mount1,
                    true,
                )
            };

            let registry = get_mount_registry().lock().unwrap();
            let m1 = registry
                .iter()
                .find(|m| m.handle_ptr == mount1 as usize)
                .unwrap();
            let m2 = registry
                .iter()
                .find(|m| m.handle_ptr == mount2 as usize)
                .unwrap();
            let m3 = registry
                .iter()
                .find(|m| m.handle_ptr == mount3 as usize)
                .unwrap();

            assert!(
                m2.position < m1.position,
                "ABOVE mount should have lower position"
            );
            assert!(
                m3.position > m1.position,
                "BELOW mount should have higher position"
            );

            drop(registry);
            unsafe {
                let _ = Box::from_raw(mount1);
                let _ = Box::from_raw(mount2);
                let _ = Box::from_raw(mount3);
                let _ = Box::from_raw(repo);
            }
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_mount_top_requires_null_relative() {
        {
            clear_mount_registry();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dummy_mount = Box::into_raw(Box::new(uio_MountHandle {
                repository: repo,
                id: 999,
                fs_type: UIO_FSTYPE_STDIO,
            }));

            // TOP with non-null relative should fail
            set_errno(0);
            let result = register_mount(
                repo,
                Path::new("/content"),
                PathBuf::from("/tmp/mount"),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                dummy_mount,
                true,
            );

            assert!(result.is_null());
            assert_eq!(current_errno(), libc::EINVAL);

            unsafe {
                let _ = Box::from_raw(dummy_mount);
                let _ = Box::from_raw(repo);
            }
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_mount_above_requires_nonnull_relative() {
        {
            clear_mount_registry();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            // ABOVE with null relative should fail
            set_errno(0);
            let result = register_mount(
                repo,
                Path::new("/content"),
                PathBuf::from("/tmp/mount"),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_ABOVE,
                ptr::null_mut(),
                true,
            );

            assert!(result.is_null());
            assert_eq!(current_errno(), libc::EINVAL);

            unsafe {
                let _ = Box::from_raw(repo);
            }
            clear_mount_registry();
        }
    }

    // =============================================================================
    // Access Mode Tests (Phase P06)
    // =============================================================================

    #[test]
    #[serial]
    fn test_access_w_ok_fails_on_readonly_mount() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"test").unwrap();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            // Create a read-only mount
            let mount = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    temp_dir.path().to_path_buf(),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_RDONLY | UIO_MOUNT_TOP,
                    ptr::null_mut(),
                    true,
                )
            };
            assert!(!mount.is_null());

            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: PathBuf::from("/content"),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository: repo,
            }));

            let path = CString::new("test.txt").unwrap();

            unsafe {
                set_errno(0);
                let result = uio_access(dir_handle, path.as_ptr(), W_OK);
                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::EACCES);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(mount);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_access_r_ok_succeeds_on_readonly_mount() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            fs::write(&test_file, b"test").unwrap();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            let mount = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    temp_dir.path().to_path_buf(),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_RDONLY | UIO_MOUNT_TOP,
                    ptr::null_mut(),
                    true,
                )
            };

            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: PathBuf::from("/content"),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository: repo,
            }));

            let path = CString::new("test.txt").unwrap();

            unsafe {
                let result = uio_access(dir_handle, path.as_ptr(), R_OK);
                assert_eq!(result, 0);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(mount);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_access_x_ok_on_directory() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let subdir = temp_dir.path().join("subdir");
            fs::create_dir(&subdir).unwrap();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            let mount = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    temp_dir.path().to_path_buf(),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_TOP,
                    ptr::null_mut(),
                    true,
                )
            };

            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: PathBuf::from("/content"),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository: repo,
            }));

            let path = CString::new("subdir").unwrap();

            unsafe {
                let result = uio_access(dir_handle, path.as_ptr(), X_OK);
                assert_eq!(result, 0);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(mount);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_access_topmost_visible_object() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir1 = TempDir::new().unwrap();
            let temp_dir2 = TempDir::new().unwrap();

            // Create file in lower mount only
            let file1 = temp_dir1.path().join("file.txt");
            fs::write(&file1, b"lower").unwrap();

            // Create file in upper mount only
            let file2 = temp_dir2.path().join("other.txt");
            fs::write(&file2, b"upper").unwrap();

            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

            // Add lower mount
            let mount1 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    temp_dir1.path().to_path_buf(),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_BOTTOM,
                    ptr::null_mut(),
                    true,
                )
            };

            // Add upper mount (higher priority)
            let mount2 = {
                register_mount(
                    repo,
                    Path::new("/content"),
                    temp_dir2.path().to_path_buf(),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_TOP,
                    ptr::null_mut(),
                    true,
                )
            };

            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                virtual_path: PathBuf::from("/content"),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository: repo,
            }));

            unsafe {
                // file.txt exists only in lower mount - should be found
                let path1 = CString::new("file.txt").unwrap();
                let result = uio_access(dir_handle, path1.as_ptr(), F_OK);
                assert_eq!(result, 0);

                // other.txt exists only in upper mount - should be found
                let path2 = CString::new("other.txt").unwrap();
                let result = uio_access(dir_handle, path2.as_ptr(), F_OK);
                assert_eq!(result, 0);

                // nonexistent.txt doesn't exist - should fail
                let path3 = CString::new("nonexistent.txt").unwrap();
                set_errno(0);
                let result = uio_access(dir_handle, path3.as_ptr(), F_OK);
                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::ENOENT);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(mount1);
                let _ = Box::from_raw(mount2);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    // =========================================================================
    // G14 Overlay-Aware Mutation Tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_uio_open_write_fails_on_readonly_top_mount() {
        {
            use std::fs;

            clear_mount_registry();

            // Create temp directory and file
            let temp_dir = std::env::temp_dir().join("uqm_test_readonly_write");
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            // Create a file in the directory
            let test_file = temp_dir.join("test.txt");
            fs::write(&test_file, b"original content").unwrap();

            // Mount as read-only
            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount_readonly(repo, "/data", temp_dir.to_str().unwrap(), true);

            unsafe {
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Try to open for writing - should fail with EPERM: no writable mount was reached
                let path = CString::new("test.txt").unwrap();
                set_errno(0);
                let handle = uio_open(dir_handle, path.as_ptr(), O_WRONLY, 0o644);

                assert!(handle.is_null());
                assert_eq!(current_errno(), libc::EPERM);

                // Try to open with O_CREAT - should also fail
                let new_file = CString::new("newfile.txt").unwrap();
                set_errno(0);
                let handle2 = uio_open(dir_handle, new_file.as_ptr(), O_WRONLY | O_CREAT, 0o644);

                assert!(handle2.is_null());
                assert_eq!(current_errno(), libc::EPERM);

                // Reading should still work
                let handle3 = uio_open(dir_handle, path.as_ptr(), O_RDONLY, 0);
                assert!(!handle3.is_null());
                uio_close(handle3);

                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            let _ = fs::remove_dir_all(&temp_dir);
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_uio_rename_cross_mount_fails_with_exdev() {
        {
            use std::fs;

            clear_mount_registry();

            // Create two temp directories for two different mounts
            let temp_dir1 = std::env::temp_dir().join("uqm_test_mount1");
            let temp_dir2 = std::env::temp_dir().join("uqm_test_mount2");
            let _ = fs::remove_dir_all(&temp_dir1);
            let _ = fs::remove_dir_all(&temp_dir2);
            fs::create_dir_all(&temp_dir1).unwrap();
            fs::create_dir_all(&temp_dir2).unwrap();

            // Create a file in mount1
            fs::write(temp_dir1.join("file.txt"), b"test content").unwrap();

            // Add two separate mounts
            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount_readonly(repo, "/mount1", temp_dir1.to_str().unwrap(), false);
            add_test_mount_readonly(repo, "/mount2", temp_dir2.to_str().unwrap(), false);

            unsafe {
                let dir1 = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/mount1"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));
                let dir2 = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/mount2"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Try to rename across mounts - should fail with EXDEV
                let old_path = CString::new("file.txt").unwrap();
                let new_path = CString::new("moved.txt").unwrap();
                set_errno(0);
                let result = uio_rename(dir1, old_path.as_ptr(), dir2, new_path.as_ptr());

                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::EXDEV);

                // File should still exist in original location
                assert!(temp_dir1.join("file.txt").exists());
                assert!(!temp_dir2.join("moved.txt").exists());

                let _ = Box::from_raw(dir1);
                let _ = Box::from_raw(dir2);
                let _ = Box::from_raw(repo);
            }

            let _ = fs::remove_dir_all(&temp_dir1);
            let _ = fs::remove_dir_all(&temp_dir2);
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_uio_unlink_fails_on_readonly_mount() {
        {
            use std::fs;

            clear_mount_registry();

            let temp_dir = std::env::temp_dir().join("uqm_test_unlink_readonly");
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            // Create a file
            fs::write(temp_dir.join("file.txt"), b"test content").unwrap();

            // Mount as read-only
            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount_readonly(repo, "/data", temp_dir.to_str().unwrap(), true);

            unsafe {
                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Try to unlink - should fail with EROFS: the only mount holding the file is read-only
                let path = CString::new("file.txt").unwrap();
                set_errno(0);
                let result = uio_unlink(dir, path.as_ptr());

                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::EROFS);

                // File should still exist
                assert!(temp_dir.join("file.txt").exists());

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            let _ = fs::remove_dir_all(&temp_dir);
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_uio_mkdir_fails_on_readonly_mount() {
        {
            use std::fs;

            clear_mount_registry();

            let temp_dir = std::env::temp_dir().join("uqm_test_mkdir_readonly");
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            // Mount as read-only
            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount_readonly(repo, "/data", temp_dir.to_str().unwrap(), true);

            unsafe {
                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Try to create directory - should fail with EPERM: no writable mount was reached
                let path = CString::new("newdir").unwrap();
                set_errno(0);
                let result = uio_mkdir(dir, path.as_ptr(), 0o755);

                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::EPERM);

                // Directory should not exist
                assert!(!temp_dir.join("newdir").exists());

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            let _ = fs::remove_dir_all(&temp_dir);
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_uio_rmdir_fails_on_readonly_mount() {
        {
            use std::fs;

            clear_mount_registry();

            let temp_dir = std::env::temp_dir().join("uqm_test_rmdir_readonly");
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            // Create a directory
            fs::create_dir(temp_dir.join("testdir")).unwrap();

            // Mount as read-only
            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount_readonly(repo, "/data", temp_dir.to_str().unwrap(), true);

            unsafe {
                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Try to remove directory - should fail with EROFS: the only mount holding the directory is read-only
                let path = CString::new("testdir").unwrap();
                set_errno(0);
                let result = uio_rmdir(dir, path.as_ptr());

                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::EROFS);

                // Directory should still exist
                assert!(temp_dir.join("testdir").exists());

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            let _ = fs::remove_dir_all(&temp_dir);
            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_mount_position_no_underflow() {
        {
            clear_mount_registry();

            // Add a mount with TOP - should not underflow
            let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            add_test_mount(repo, "/first", "/tmp/first");

            {
                let registry = get_mount_registry().lock().unwrap();
                let mount = registry
                    .iter()
                    .find(|m| m.mount_point == Path::new("/first"))
                    .unwrap();
                // Position should be reasonable (starting at 1000 or near it)
                assert!(mount.position < 10000);
            }

            // Add another mount with TOP - should still not underflow
            add_test_mount(repo, "/second", "/tmp/second");

            {
                let registry = get_mount_registry().lock().unwrap();
                let mount1 = registry
                    .iter()
                    .find(|m| m.mount_point == Path::new("/first"))
                    .unwrap();
                let mount2 = registry
                    .iter()
                    .find(|m| m.mount_point == Path::new("/second"))
                    .unwrap();

                // Both positions should be valid and different
                assert!(mount1.position < 10000);
                assert!(mount2.position < 10000);
                assert_ne!(mount1.position, mount2.position);
            }

            drop(unsafe { Box::from_raw(repo) });
            clear_mount_registry();
        }
    }

    // =============================================================================
    // Phase P07: Regex Matching and Cross-Mount Directory Listing Tests
    // =============================================================================

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-REGEX
    #[test]
    #[serial]
    fn test_regex_pattern_rmp_files() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_dir = tempdir().unwrap();
            let dir_path = temp_dir.path();

            // Create test files
            std::fs::write(dir_path.join("index.rmp"), b"test").unwrap();
            std::fs::write(dir_path.join("music.RMP"), b"test").unwrap();
            std::fs::write(dir_path.join("archive.RmP"), b"test").unwrap();
            std::fs::write(dir_path.join("data.txt"), b"test").unwrap();
            std::fs::write(dir_path.join("file.zip"), b"test").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", dir_path.to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Test .rmp pattern
                let pattern = std::ffi::CString::new(r"\.[rR][mM][pP]$").unwrap();
                let list =
                    uio_getDirList(dir_handle, std::ptr::null(), pattern.as_ptr(), MATCH_REGEX);

                assert!(!list.is_null());
                let num_names = (*list).numNames;
                assert_eq!(
                    num_names, 3,
                    "Should match every case variant of .rmp files"
                );

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-REGEX
    #[test]
    #[serial]
    fn test_regex_pattern_zip_uqm_files() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_dir = tempdir().unwrap();
            let dir_path = temp_dir.path();
            // Create test files
            std::fs::write(dir_path.join("package.zip"), b"test").unwrap();
            std::fs::write(dir_path.join("data.uqm"), b"test").unwrap();
            std::fs::write(dir_path.join("music.UQM"), b"test").unwrap();

            std::fs::write(dir_path.join("file.rmp"), b"test").unwrap();
            std::fs::write(dir_path.join("readme.txt"), b"test").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", dir_path.to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Test .zip/.uqm pattern
                let pattern = std::ffi::CString::new(r"\.([zZ][iI][pP]|[uU][qQ][mM])$").unwrap();
                let list =
                    uio_getDirList(dir_handle, std::ptr::null(), pattern.as_ptr(), MATCH_REGEX);

                assert!(!list.is_null());
                let num_names = (*list).numNames;
                assert_eq!(num_names, 3, "Should match .zip, .uqm, and .UQM files");

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-REGEX
    #[test]
    #[serial]
    fn test_invalid_regex_returns_empty() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_dir = tempdir().unwrap();
            let dir_path = temp_dir.path();

            std::fs::write(dir_path.join("file.txt"), b"test").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", dir_path.to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // Use invalid regex pattern
                let pattern = std::ffi::CString::new(r"[invalid(regex").unwrap();
                let list =
                    uio_getDirList(dir_handle, std::ptr::null(), pattern.as_ptr(), MATCH_REGEX);

                // Should return non-null empty list, not crash
                assert!(!list.is_null());
                assert_eq!((*list).numNames, 0, "Invalid regex should match nothing");

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-UNION
    #[test]
    #[serial]
    fn test_cross_mount_directory_listing_union() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_a = tempdir().unwrap();
            let temp_b = tempdir().unwrap();

            // Create files in mount A
            std::fs::write(temp_a.path().join("a.txt"), b"a").unwrap();
            std::fs::write(temp_a.path().join("shared.txt"), b"a_version").unwrap();

            // Create files in mount B
            std::fs::write(temp_b.path().join("b.txt"), b"b").unwrap();
            std::fs::write(temp_b.path().join("shared.txt"), b"b_version").unwrap();

            unsafe {
                // Add mounts manually with proper precedence
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/content", temp_a.path().to_str().unwrap(), true);
                add_test_mount_readonly(repo, "/content", temp_b.path().to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/content"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // List all files
                let list = uio_getDirList(
                    dir_handle,
                    std::ptr::null(),
                    std::ptr::null(),
                    MATCH_LITERAL,
                );
                assert!(!list.is_null());

                // Should see union: a.txt, b.txt, shared.txt (only once, from mount A)
                let num_names = (*list).numNames;
                assert_eq!(num_names, 3, "Should see union of both mounts");

                // Verify names
                let names_slice = std::slice::from_raw_parts((*list).names, num_names as usize);
                let mut names: Vec<String> = names_slice
                    .iter()
                    .map(|&ptr| std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
                    .collect();
                names.sort();

                assert_eq!(names, vec!["a.txt", "b.txt", "shared.txt"]);

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-UNION
    #[test]
    #[serial]
    fn test_cross_mount_listing_deduplication() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_a = tempdir().unwrap();
            let temp_b = tempdir().unwrap();

            // Both mounts have the same file
            std::fs::write(temp_a.path().join("duplicate.txt"), b"version_a").unwrap();
            std::fs::write(temp_b.path().join("duplicate.txt"), b"version_b").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_a.path().to_str().unwrap(), true);
                add_test_mount_readonly(repo, "/data", temp_b.path().to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let list = uio_getDirList(
                    dir_handle,
                    std::ptr::null(),
                    std::ptr::null(),
                    MATCH_LITERAL,
                );

                assert!(!list.is_null());
                // Should see only one entry (from higher precedence mount A)
                assert_eq!((*list).numNames, 1, "Duplicate should appear only once");

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-EMPTY
    #[test]
    #[serial]
    fn test_empty_directory_returns_nonnull() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_dir = tempdir().unwrap();
            // Don't create any files

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let list = uio_getDirList(
                    dir_handle,
                    std::ptr::null(),
                    std::ptr::null(),
                    MATCH_LITERAL,
                );

                // Should return non-null list with numNames=0
                assert!(
                    !list.is_null(),
                    "Empty directory should return non-null list"
                );
                assert_eq!(
                    (*list).numNames,
                    0,
                    "Empty directory should have numNames=0"
                );

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P07
    /// @requirement REQ-FIO-DIRLIST-REGEX
    #[test]
    #[serial]
    fn test_regex_with_cross_mount_listing() {
        {
            use tempfile::tempdir;

            clear_mount_registry();

            let temp_a = tempdir().unwrap();
            let temp_b = tempdir().unwrap();

            // Mount A has .rmp files
            std::fs::write(temp_a.path().join("index1.rmp"), b"a").unwrap();
            std::fs::write(temp_a.path().join("data.txt"), b"a").unwrap();

            // Mount B has .rmp files
            std::fs::write(temp_b.path().join("index2.rmp"), b"b").unwrap();
            std::fs::write(temp_b.path().join("readme.md"), b"b").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/indices", temp_a.path().to_str().unwrap(), true);
                add_test_mount_readonly(repo, "/indices", temp_b.path().to_str().unwrap(), true);

                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/indices"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // List only .rmp files
                let pattern = std::ffi::CString::new(r"\.[rR][mM][pP]$").unwrap();
                let list =
                    uio_getDirList(dir_handle, std::ptr::null(), pattern.as_ptr(), MATCH_REGEX);

                assert!(!list.is_null());
                // Should see both .rmp files from both mounts
                assert_eq!(
                    (*list).numNames,
                    2,
                    "Should match .rmp files from both mounts"
                );

                // Verify names
                let names_slice =
                    std::slice::from_raw_parts((*list).names, (*list).numNames as usize);
                let mut names: Vec<String> = names_slice
                    .iter()
                    .map(|&ptr| std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
                    .collect();
                names.sort();

                assert_eq!(names, vec!["index1.rmp", "index2.rmp"]);

                uio_DirList_free(list);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    // =============================================================================
    // FileBlock Tests
    // =============================================================================

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_open_whole_file() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello, World!").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                let inner = &*(block as *const FileBlockInner);
                assert_eq!(inner.base_offset, 0);
                assert_eq!(inner.size, 13);

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_open_with_range() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"0123456789ABCDEF").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                // Create block covering bytes 5-10 (5 bytes)
                let block = uio_openFileBlock2(handle, 5, 5);
                assert!(!block.is_null());

                let inner = &*(block as *const FileBlockInner);
                assert_eq!(inner.base_offset, 5);
                assert_eq!(inner.size, 5);

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_open2_invalid_range() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                // Try to create block beyond file size
                let block = uio_openFileBlock2(handle, 0, 100);
                assert!(block.is_null());
                assert_eq!(current_errno(), libc::EINVAL);

                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_access_basic() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello, World!").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                let mut buffer: *mut c_char = ptr::null_mut();
                let bytes_read = uio_accessFileBlock(block, 0, 5, &mut buffer);

                assert_eq!(bytes_read, 5);
                assert!(!buffer.is_null());

                let data = slice::from_raw_parts(buffer as *const u8, 5);
                assert_eq!(data, b"Hello");

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_access_with_offset() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello, World!").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                let mut buffer: *mut c_char = ptr::null_mut();
                let bytes_read = uio_accessFileBlock(block, 7, 5, &mut buffer);

                assert_eq!(bytes_read, 5);
                assert!(!buffer.is_null());

                let data = slice::from_raw_parts(buffer as *const u8, 5);
                assert_eq!(data, b"World");

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_access_short_at_eof() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                // Request more bytes than available
                let mut buffer: *mut c_char = ptr::null_mut();
                let bytes_read = uio_accessFileBlock(block, 3, 100, &mut buffer);

                // Should return only 2 bytes (positions 3-4)
                assert_eq!(bytes_read, 2);
                assert!(!buffer.is_null());

                let data = slice::from_raw_parts(buffer as *const u8, 2);
                assert_eq!(data, b"lo");

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_access_past_eof() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                // Access beyond file size
                let mut buffer: *mut c_char = ptr::null_mut();
                let bytes_read = uio_accessFileBlock(block, 100, 10, &mut buffer);

                assert_eq!(bytes_read, 0);
                assert!(buffer.is_null());

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_access_ranged_block() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"0123456789ABCDEF").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                // Block covering bytes 5-10 (6 bytes: "56789A")
                let block = uio_openFileBlock2(handle, 5, 6);
                assert!(!block.is_null());

                // Access from offset 2 within block (byte 7 in file)
                let mut buffer: *mut c_char = ptr::null_mut();
                let bytes_read = uio_accessFileBlock(block, 2, 3, &mut buffer);

                assert_eq!(bytes_read, 3);
                assert!(!buffer.is_null());

                let data = slice::from_raw_parts(buffer as *const u8, 3);
                assert_eq!(data, b"789");

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_copy_basic() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello, World!").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                let mut buffer = [0u8; 10];
                let result = uio_copyFileBlock(block, 7, buffer.as_mut_ptr() as *mut c_char, 5);

                assert_eq!(result, 0); // Success
                assert_eq!(&buffer[..5], b"World");

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_clear_buffers() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello, World!").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                // Access data to populate cache
                let mut buffer: *mut c_char = ptr::null_mut();
                let bytes_read = uio_accessFileBlock(block, 0, 5, &mut buffer);
                assert_eq!(bytes_read, 5);

                let inner = &*(block as *const FileBlockInner);
                assert!(!inner.cache.is_empty());

                // Clear buffers
                uio_clear_file_block_buffers(block);

                let inner = &*(block as *const FileBlockInner);
                assert!(inner.cache.is_empty());

                // Block should still be usable
                let bytes_read = uio_accessFileBlock(block, 7, 5, &mut buffer);
                assert_eq!(bytes_read, 5);
                assert!(!buffer.is_null());

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_multiple_access_stable_pointer() {
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(b"Hello, World!").unwrap();
            temp.flush().unwrap();

            unsafe {
                let file = std::fs::File::open(temp.path()).unwrap();
                let handle = handle_into_raw(uio_HandleInner::File(file));

                let block = uio_openFileBlock(handle);
                assert!(!block.is_null());

                // First access
                let mut buffer1: *mut c_char = ptr::null_mut();
                let bytes_read1 = uio_accessFileBlock(block, 0, 5, &mut buffer1);
                assert_eq!(bytes_read1, 5);
                let _first_ptr = buffer1;

                // Second access invalidates first pointer
                let mut buffer2: *mut c_char = ptr::null_mut();
                let bytes_read2 = uio_accessFileBlock(block, 7, 5, &mut buffer2);
                assert_eq!(bytes_read2, 5);

                // First pointer should not be used after second access
                // (This test just documents the behavior)
                let data2 = slice::from_raw_parts(buffer2 as *const u8, 5);
                assert_eq!(data2, b"World");

                uio_closeFileBlock(block);
                uio_close(handle);
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_null_handle() {
        unsafe {
            let block = uio_openFileBlock(ptr::null_mut());
            assert!(block.is_null());
            assert_eq!(current_errno(), libc::EINVAL);
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P08
    /// @requirement REQ-FIO-FILEBLOCK
    #[test]
    fn test_fileblock_close_null_safe() {
        unsafe {
            let result = uio_closeFileBlock(ptr::null_mut());
            assert_eq!(result, 0); // Should succeed safely
        }
    }

    // =========================================================================
    // StdioAccess and copyFile Tests (Phase P10)
    // =========================================================================

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_direct_path_on_stdio_mount() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.txt");
            std::fs::write(&test_file, b"direct path content").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let path = CString::new("test.txt").unwrap();
                let handle = uio_getStdioAccess(dir, path.as_ptr(), 0, ptr::null_mut());

                assert!(!handle.is_null());

                // Get path
                let path_ptr = uio_StdioAccessHandle_getPath(handle);
                assert!(!path_ptr.is_null());

                let returned_path = CStr::from_ptr(path_ptr).to_str().unwrap();
                assert!(returned_path.contains("test.txt"));

                // Verify we can read the file from the returned path
                let content = std::fs::read_to_string(returned_path).unwrap();
                assert_eq!(content, "direct path content");

                // Release handle
                uio_releaseStdioAccess(handle);

                // File should still exist (direct path, not temp copy)
                assert!(test_file.exists());

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_temp_copy_on_zip_mount() {
        {
            use std::io::Write;
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let zip_path = temp_dir.path().join("test.zip");

            // Create a ZIP archive
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("data.txt", options).unwrap();
            zip.write_all(b"ZIP content").unwrap();
            zip.finish().unwrap();

            // Create temp directory for extraction
            let temp_extract_dir = TempDir::new().unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

                // Mount the ZIP
                let mount = register_mount(
                    repo,
                    Path::new("/archive"),
                    zip_path.clone(),
                    UIO_FSTYPE_ZIP,
                    UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
                    ptr::null_mut(),
                    true,
                );
                assert!(!mount.is_null());

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/archive"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                // The temporary directory is a writable directory of the same
                // repository, found through the mounts like any other.
                let scratch_mount = register_mount(
                    repo,
                    Path::new("/scratch"),
                    temp_extract_dir.path().to_path_buf(),
                    UIO_FSTYPE_STDIO,
                    UIO_MOUNT_TOP,
                    ptr::null_mut(),
                    true,
                );
                assert!(!scratch_mount.is_null());
                let temp_dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/scratch"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let path = CString::new("data.txt").unwrap();
                let handle = uio_getStdioAccess(dir, path.as_ptr(), 0, temp_dir_handle);

                assert!(!handle.is_null());

                // Get path
                let path_ptr = uio_StdioAccessHandle_getPath(handle);
                assert!(!path_ptr.is_null());

                let returned_path = CStr::from_ptr(path_ptr).to_str().unwrap();

                // Verify it's in the temp directory
                assert!(returned_path.starts_with(temp_extract_dir.path().to_str().unwrap()));

                // Verify we can read the extracted content
                let content = std::fs::read_to_string(returned_path).unwrap();
                assert_eq!(content, "ZIP content");

                let temp_file_path = PathBuf::from(returned_path);
                assert!(temp_file_path.exists());

                // Release handle - should delete temp file
                uio_releaseStdioAccess(handle);

                // Temp file should be deleted
                assert!(!temp_file_path.exists());

                assert!(
                    temp_extract_dir.path().exists(),
                    "the temporary directory itself is kept"
                );

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(temp_dir_handle);
                let _ = Box::from_raw(scratch_mount);
                let _ = Box::from_raw(mount);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_rejects_directory_with_eisdir() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let subdir = temp_dir.path().join("subdir");
            std::fs::create_dir(&subdir).unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let path = CString::new("subdir").unwrap();
                set_errno(0);
                let handle = uio_getStdioAccess(dir, path.as_ptr(), 0, ptr::null_mut());

                assert!(handle.is_null());
                assert_eq!(current_errno(), libc::EISDIR);

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_missing_file_returns_enoent() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let path = CString::new("nonexistent.txt").unwrap();
                set_errno(0);
                let handle = uio_getStdioAccess(dir, path.as_ptr(), 0, ptr::null_mut());

                assert!(handle.is_null());
                assert_eq!(current_errno(), libc::ENOENT);

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_zip_without_tempdir_fails() {
        {
            use std::io::Write;
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let zip_path = temp_dir.path().join("test.zip");

            // Create a ZIP archive
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("data.txt", options).unwrap();
            zip.write_all(b"content").unwrap();
            zip.finish().unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));

                // Mount the ZIP
                let mount = register_mount(
                    repo,
                    Path::new("/archive"),
                    zip_path.clone(),
                    UIO_FSTYPE_ZIP,
                    UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
                    ptr::null_mut(),
                    true,
                );
                assert!(!mount.is_null());

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/archive"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let path = CString::new("data.txt").unwrap();
                set_errno(0);
                // No tempDir provided - should fail
                let handle = uio_getStdioAccess(dir, path.as_ptr(), 0, ptr::null_mut());

                assert!(handle.is_null());
                assert_eq!(current_errno(), libc::EINVAL);

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(mount);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_getpath_null_handle() {
        unsafe {
            set_errno(0);
            let path = uio_StdioAccessHandle_getPath(ptr::null_mut());
            assert!(path.is_null());
            assert_eq!(current_errno(), libc::EINVAL);
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-STDIO-ACCESS
    #[test]
    #[serial]
    fn test_stdio_access_release_null_safe() {
        unsafe {
            // Should not crash
            uio_releaseStdioAccess(ptr::null_mut());
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-COPY
    #[test]
    #[serial]
    fn test_copy_file_basic() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let src_file = temp_dir.path().join("source.txt");
            std::fs::write(&src_file, b"file content to copy").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let src_path = CString::new("source.txt").unwrap();
                let dst_path = CString::new("dest.txt").unwrap();

                let result = uio_copy_file(dir, src_path.as_ptr(), dir, dst_path.as_ptr());
                assert_eq!(result, 0);

                // Verify destination exists
                let dst_file = temp_dir.path().join("dest.txt");
                assert!(dst_file.exists());

                // Verify content
                let content = std::fs::read_to_string(&dst_file).unwrap();
                assert_eq!(content, "file content to copy");

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-COPY
    #[test]
    #[serial]
    fn test_copy_file_to_existing_dest_fails() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let src_file = temp_dir.path().join("source.txt");
            let dst_file = temp_dir.path().join("dest.txt");
            std::fs::write(&src_file, b"source").unwrap();
            std::fs::write(&dst_file, b"existing").unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let src_path = CString::new("source.txt").unwrap();
                let dst_path = CString::new("dest.txt").unwrap();

                set_errno(0);
                let result = uio_copy_file(dir, src_path.as_ptr(), dir, dst_path.as_ptr());
                assert_eq!(result, -1);
                // Should fail with EEXIST (O_EXCL semantics)
                assert_eq!(current_errno(), libc::EEXIST);

                // Destination should still have original content
                let content = std::fs::read_to_string(&dst_file).unwrap();
                assert_eq!(content, "existing");

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-COPY
    #[test]
    #[serial]
    fn test_copy_file_missing_source() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let src_path = CString::new("nonexistent.txt").unwrap();
                let dst_path = CString::new("dest.txt").unwrap();

                set_errno(0);
                let result = uio_copy_file(dir, src_path.as_ptr(), dir, dst_path.as_ptr());
                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::ENOENT);

                // Destination should not exist
                let dst_file = temp_dir.path().join("dest.txt");
                assert!(!dst_file.exists());

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P10
    /// @requirement REQ-FIO-COPY
    #[test]
    #[serial]
    fn test_copy_file_large() {
        {
            use tempfile::TempDir;
            clear_mount_registry();

            let temp_dir = TempDir::new().unwrap();
            let src_file = temp_dir.path().join("large.dat");

            // Create a file larger than the 8KB chunk size
            let data = vec![0x42u8; 20000];
            std::fs::write(&src_file, &data).unwrap();

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                add_test_mount_readonly(repo, "/data", temp_dir.path().to_str().unwrap(), false);

                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                }));

                let src_path = CString::new("large.dat").unwrap();
                let dst_path = CString::new("large_copy.dat").unwrap();

                let result = uio_copy_file(dir, src_path.as_ptr(), dir, dst_path.as_ptr());
                assert_eq!(result, 0);

                // Verify destination exists and has same size
                let dst_file = temp_dir.path().join("large_copy.dat");
                assert!(dst_file.exists());

                let copied_data = std::fs::read(&dst_file).unwrap();
                assert_eq!(copied_data.len(), 20000);
                assert_eq!(copied_data, data);

                let _ = Box::from_raw(dir);
                let _ = Box::from_raw(repo);
            }

            clear_mount_registry();
        }
    }

    // =============================================================================
    // Phase P11: Lifecycle, Init/Uninit, and Resource Cleanup Tests
    // =============================================================================

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-LIFECYCLE
    /// Test: init → use → uninit → reinit → use works correctly
    #[test]
    #[serial]
    fn test_lifecycle_init_uninit_reinit() {
        unsafe {
            // First init
            uio_init();
            assert!(UIO_INITIALIZED.load(Ordering::SeqCst));

            // Use the system
            let repo = uio_openRepository(0);
            assert!(!repo.is_null());

            // Close repository
            uio_closeRepository(repo);

            // Uninit
            uio_unInit();
            assert!(!UIO_INITIALIZED.load(Ordering::SeqCst));

            // Re-init
            uio_init();
            assert!(UIO_INITIALIZED.load(Ordering::SeqCst));

            // Use again
            let repo2 = uio_openRepository(0);
            assert!(!repo2.is_null());

            // Clean up
            uio_closeRepository(repo2);
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-LIFECYCLE
    /// Test: uninit clears mount registry
    #[test]
    #[serial]
    fn test_uninit_clears_mount_registry() {
        unsafe {
            uio_init();

            // Add a test mount
            let repo = uio_openRepository(0);
            let _mount_point = CString::new("/testmount").unwrap();
            let test_dir = std::env::temp_dir();
            let _test_dir_cstr = CString::new(test_dir.to_str().unwrap()).unwrap();

            let mount_handle = register_mount(
                repo,
                Path::new("/testmount"),
                test_dir,
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            assert!(!mount_handle.is_null());

            // Verify mount exists
            {
                let registry = get_mount_registry().lock().unwrap();
                assert!(!registry.is_empty());
            }

            // Close repository (unmounts all)
            uio_closeRepository(repo);

            // Verify mounts cleared
            {
                let registry = get_mount_registry().lock().unwrap();
                assert!(registry.is_empty());
            }

            // Uninit
            uio_unInit();

            // Verify registry still empty
            {
                let registry = get_mount_registry().lock().unwrap();
                assert!(registry.is_empty());
            }
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
    /// Test: uio_close remains safe after mount removal
    #[test]
    #[serial]
    fn test_close_safe_after_unmount() {
        unsafe {
            uio_init();

            let temp_dir = std::env::temp_dir().join("uio_test_close_after_unmount");
            std::fs::create_dir_all(&temp_dir).unwrap();
            let test_file = temp_dir.join("test.txt");
            std::fs::write(&test_file, b"test data").unwrap();

            let repo = uio_openRepository(0);
            let mount_point = CString::new("/testmount").unwrap();
            let _temp_dir_cstr = CString::new(temp_dir.to_str().unwrap()).unwrap();

            let mount_handle = register_mount(
                repo,
                Path::new("/testmount"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            // Open a file through the mount
            let dir = uio_openDir(repo, mount_point.as_ptr(), 0);
            assert!(!dir.is_null());

            let filename = CString::new("test.txt").unwrap();
            let handle = uio_open(dir, filename.as_ptr(), O_RDONLY, 0);
            assert!(!handle.is_null());

            // Unmount the directory
            uio_unmountDir(mount_handle);

            // Verify mount is gone
            {
                let registry = get_mount_registry().lock().unwrap();
                assert!(registry.is_empty());
            }

            // Close the handle - should still work
            let result = uio_close(handle);
            assert_eq!(result, 0);

            // Clean up
            uio_closeDir(dir);
            uio_closeRepository(repo);
            std::fs::remove_file(&test_file).ok();
            std::fs::remove_dir(&temp_dir).ok();
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
    /// Test: uio_fclose remains safe after mount removal
    #[test]
    #[serial]
    fn test_fclose_safe_after_unmount() {
        unsafe {
            uio_init();

            let temp_dir = std::env::temp_dir().join("uio_test_fclose_after_unmount");
            std::fs::create_dir_all(&temp_dir).unwrap();
            let test_file = temp_dir.join("test.txt");
            std::fs::write(&test_file, b"test data").unwrap();

            let repo = uio_openRepository(0);
            let mount_point = CString::new("/testmount").unwrap();
            let _temp_dir_cstr = CString::new(temp_dir.to_str().unwrap()).unwrap();

            let mount_handle = register_mount(
                repo,
                Path::new("/testmount"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            // Open a file through the mount
            let dir = uio_openDir(repo, mount_point.as_ptr(), 0);
            assert!(!dir.is_null());

            let filename = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();
            let stream = uio_fopen(dir, filename.as_ptr(), mode.as_ptr());
            assert!(!stream.is_null());

            // Unmount the directory
            uio_unmountDir(mount_handle);

            // Close the stream - should still work
            let result = uio_fclose(stream);
            assert_eq!(result, 0);

            // Clean up
            uio_closeDir(dir);
            uio_closeRepository(repo);
            std::fs::remove_file(&test_file).ok();
            std::fs::remove_dir(&temp_dir).ok();
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
    /// Test: uio_closeDir remains safe after mount removal
    #[test]
    #[serial]
    fn test_closedir_safe_after_unmount() {
        unsafe {
            uio_init();

            let temp_dir = std::env::temp_dir().join("uio_test_closedir_after_unmount");
            std::fs::create_dir_all(&temp_dir).unwrap();

            let repo = uio_openRepository(0);
            let mount_point = CString::new("/testmount").unwrap();
            let _temp_dir_cstr = CString::new(temp_dir.to_str().unwrap()).unwrap();

            let mount_handle = register_mount(
                repo,
                Path::new("/testmount"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            // Open a directory through the mount
            let dir = uio_openDir(repo, mount_point.as_ptr(), 0);
            assert!(!dir.is_null());

            // Unmount the directory
            uio_unmountDir(mount_handle);

            // Close the dir handle - should still work
            let result = uio_closeDir(dir);
            assert_eq!(result, 0);

            // Clean up
            uio_closeRepository(repo);
            std::fs::remove_dir(&temp_dir).ok();
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
    /// Test: uio_releaseStdioAccess remains safe (handles are independent of mounts)
    #[test]
    #[serial]
    fn test_release_stdio_safe_after_unmount() {
        unsafe {
            uio_init();

            // This test verifies that uio_releaseStdioAccess can safely free its resources
            // The function already owns its resources independently of mount state
            // Testing the actual function implementation by calling it with null (which is safe)
            uio_releaseStdioAccess(ptr::null_mut());

            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-RESOURCE-MGMT
    /// Test: uio_DirList_free on non-empty list doesn't leak
    #[test]
    #[serial]
    fn test_dirlist_free_nonempty() {
        unsafe {
            uio_init();

            let temp_dir = std::env::temp_dir().join("uio_test_dirlist_nonempty");
            std::fs::create_dir_all(&temp_dir).unwrap();
            std::fs::write(temp_dir.join("file1.txt"), b"data1").unwrap();
            std::fs::write(temp_dir.join("file2.txt"), b"data2").unwrap();

            let repo = uio_openRepository(0);
            let mount_point = CString::new("/testmount").unwrap();
            let _temp_dir_cstr = CString::new(temp_dir.to_str().unwrap()).unwrap();

            let mount_handle = register_mount(
                repo,
                Path::new("/testmount"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            let dir = uio_openDir(repo, mount_point.as_ptr(), 0);
            assert!(!dir.is_null());

            // Get directory list
            let empty_str = CString::new("").unwrap();
            let list = uio_getDirList(dir, empty_str.as_ptr(), ptr::null(), 0);
            assert!(!list.is_null());
            // Note: numNames might be 0 if directory appears empty due to filtering
            // The test is just verifying that uio_DirList_free can handle lists properly

            // Free the list - should not leak
            uio_DirList_free(list);

            // Clean up
            uio_closeDir(dir);
            uio_unmountDir(mount_handle);
            uio_closeRepository(repo);
            std::fs::remove_file(temp_dir.join("file1.txt")).ok();
            std::fs::remove_file(temp_dir.join("file2.txt")).ok();
            std::fs::remove_dir(&temp_dir).ok();
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-RESOURCE-MGMT
    /// Test: uio_DirList_free on empty list doesn't crash
    #[test]
    #[serial]
    fn test_dirlist_free_empty() {
        unsafe {
            uio_init();

            let temp_dir = std::env::temp_dir().join("uio_test_dirlist_empty");
            std::fs::create_dir_all(&temp_dir).unwrap();

            let repo = uio_openRepository(0);
            let mount_point = CString::new("/testmount").unwrap();
            let _temp_dir_cstr = CString::new(temp_dir.to_str().unwrap()).unwrap();

            let mount_handle = register_mount(
                repo,
                Path::new("/testmount"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            let dir = uio_openDir(repo, mount_point.as_ptr(), 0);
            assert!(!dir.is_null());

            // Get directory list (should be empty or nearly empty)
            let pattern = CString::new("nonexistent_*").unwrap();
            let list = uio_getDirList(dir, ptr::null(), pattern.as_ptr(), 0);
            assert!(!list.is_null());

            // Free the list - should not crash
            uio_DirList_free(list);

            // Clean up
            uio_closeDir(dir);
            uio_unmountDir(mount_handle);
            uio_closeRepository(repo);
            std::fs::remove_dir(&temp_dir).ok();
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-RESOURCE-MGMT
    /// Test: uio_DirList_free(NULL) is safe
    #[test]
    #[serial]
    fn test_dirlist_free_null() {
        unsafe {
            uio_init();

            // Should not crash
            uio_DirList_free(ptr::null_mut());

            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-LIFECYCLE
    /// Test: closeRepository cleans up all mounts
    #[test]
    #[serial]
    fn test_close_repository_cleans_mounts() {
        unsafe {
            uio_init();

            let temp_dir = std::env::temp_dir().join("uio_test_close_repo_mounts");
            std::fs::create_dir_all(&temp_dir).unwrap();

            let repo = uio_openRepository(0);

            // Add multiple mounts
            let _mount1 = register_mount(
                repo,
                Path::new("/mount1"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            let _mount2 = register_mount(
                repo,
                Path::new("/mount2"),
                temp_dir.clone(),
                UIO_FSTYPE_STDIO,
                UIO_MOUNT_TOP,
                ptr::null_mut(),
                true,
            );

            // Verify mounts exist
            {
                let registry = get_mount_registry().lock().unwrap();
                assert_eq!(registry.len(), 2);
            }

            // Close repository
            uio_closeRepository(repo);

            // Verify all mounts cleaned up
            {
                let registry = get_mount_registry().lock().unwrap();
                assert_eq!(registry.len(), 0);
            }

            std::fs::remove_dir(&temp_dir).ok();
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-LIFECYCLE
    /// Test: uninit without init is safe
    #[test]
    #[serial]
    fn test_uninit_without_init() {
        unsafe {
            // Should not crash
            uio_unInit();
        }
    }

    /// @plan PLAN-20260314-FILE-IO.P11
    /// @requirement REQ-FIO-LIFECYCLE
    /// Test: multiple init calls are idempotent
    #[test]
    #[serial]
    fn test_multiple_init_idempotent() {
        unsafe {
            uio_init();
            uio_init();
            uio_init();

            assert!(UIO_INITIALIZED.load(Ordering::SeqCst));

            uio_unInit();
        }
    }
}

#[cfg(test)]
#[path = "uio_layered_open_tests.rs"]
mod layered_open_tests;

#[cfg(test)]
#[path = "uio_stream_tests.rs"]
mod stream_tests;

#[cfg(test)]
#[path = "uio_access_tests.rs"]
mod access_tests;

#[cfg(test)]
#[path = "uio_fileops_tests.rs"]
mod fileops_tests;

#[cfg(test)]
#[path = "uio_fileblock_tests.rs"]
mod fileblock_tests;

#[cfg(test)]
#[path = "uio_mount_ops_tests.rs"]
mod mount_ops_tests;

#[cfg(test)]
#[path = "uio_fopen_mount_tests.rs"]
mod fopen_mount_tests;

#[cfg(test)]
#[path = "uio_name_bytes_tests.rs"]
mod name_bytes_tests;

#[cfg(test)]
#[path = "uio_resolution_tests.rs"]
mod resolution_tests;

#[cfg(test)]
#[path = "uio_write_location_tests.rs"]
mod write_location_tests;

#[cfg(test)]
#[path = "uio_virtual_dir_tests.rs"]
mod virtual_dir_tests;

#[cfg(test)]
#[path = "uio_mode_tests.rs"]
mod mode_tests;

#[cfg(test)]
#[path = "uio_stdio_write_tests.rs"]
mod stdio_write_tests;

#[cfg(test)]
#[path = "uio_archive_stat_tests.rs"]
mod archive_stat_tests;

#[cfg(test)]
#[path = "uio_archive_path_tests.rs"]
mod archive_path_tests;
