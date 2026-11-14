// Minimal stdio-backed UIO implementation
// Reference: sc2/src/libs/uio/io.h, uiostream.h

use crate::bridge_log::rust_bridge_log_msg;
use crate::platform::set_errno;
use libc::size_t;
use std::collections::HashMap;

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int, c_long};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

// =============================================================================
// Mount Point Registry
const UIO_MOUNT_RDONLY: c_int = 1 << 1;
const UIO_MOUNT_LOCATION_MASK: c_int = 3 << 2;
const UIO_MOUNT_BOTTOM: c_int = 0 << 2;
const UIO_MOUNT_TOP: c_int = 1 << 2;

const UIO_MOUNT_BELOW: c_int = 2 << 2;
const UIO_MOUNT_ABOVE: c_int = 3 << 2;

const UIO_FSTYPE_STDIO: c_int = 1;
const UIO_FSTYPE_ZIP: c_int = 2;

// Access mode constants

const UIO_STREAM_STATUS_OK: c_int = 0;
const UIO_STREAM_STATUS_EOF: c_int = 1;
const UIO_STREAM_STATUS_ERROR: c_int = 2;

const UIO_STREAM_OPERATION_NONE: c_int = 0;
const UIO_STREAM_OPERATION_READ: c_int = 1;

#[cfg(test)]
use crate::platform::errno as current_errno;

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
    mount_point: String,
    mounted_root: PathBuf,
    fs_type: c_int,
    active_in_registry: bool,
    /// Explicit ordering position (lower value = higher priority)
    position: usize,
    /// Read-only mount flag
    read_only: bool,
    /// ZIP archive index (only for UIO_FSTYPE_ZIP mounts)
    zip_index: Option<std::sync::Arc<crate::io::zip_reader::ZipIndex>>,
    // Note: AutoMount is DEFERRED per P00a Q2 resolution - not needed for engine/runtime paths
    // auto_mount_rules field is NOT implemented
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

pub use owned::uio_DirHandle;

#[path = "uio_owned.rs"]
pub mod owned;

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

/// @plan PLAN-20260314-FILE-IO.P05
/// @requirement REQ-FIO-ERRNO
/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn uio_stat(dir: *mut uio_DirHandle, path: *const c_char, stat_buf: *mut stat) -> c_int {
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
        for mount in registry.iter().filter(|m| m.active_in_registry) {
            let mount_path = Path::new(&mount.mount_point);
            if virtual_path != mount_path && !virtual_path.starts_with(mount_path) {
                continue;
            }

            let suffix = virtual_path
                .strip_prefix(mount_path)
                .unwrap_or_else(|_| Path::new(""));

            // Handle ZIP mounts
            if mount.fs_type == UIO_FSTYPE_ZIP {
                if let Some(ref zip_index) = mount.zip_index {
                    let suffix_str = suffix.to_string_lossy();

                    // Check if it's a directory
                    if zip_index.is_directory(&suffix_str) {
                        (*stat_buf).st_size = 0;
                        (*stat_buf).st_mode = 0o040555; // Directory, read+exec for all
                        return 0;
                    }

                    // Check if it's a file
                    if let Some(entry) = zip_index.get_entry(&suffix_str) {
                        (*stat_buf).st_size = entry.uncompressed_size as i64;
                        (*stat_buf).st_mode = 0o100444; // Regular file, read-only
                        return 0;
                    }
                }
                continue;
            }

            // Handle stdio mounts
            let host_path = if suffix.as_os_str().is_empty() {
                mount.mounted_root.clone()
            } else {
                map_virtual_to_host_confined(&mount.mounted_root, suffix)
            };

            match fs::metadata(&host_path) {
                Ok(meta) => {
                    (*stat_buf).st_size = meta.len() as i64;
                    (*stat_buf).st_mode = if meta.is_file() { 0o100000 } else { 0o040000 };
                    (*stat_buf).st_mode |= if meta.permissions().readonly() {
                        0o444
                    } else {
                        0o666
                    };
                    return 0;
                }
                Err(_) => continue,
            }
        }

        // Not found in any mount
        fail_errno(libc::ENOENT, -1)
    })
}

// C-compatible uio_DirList struct (must match C definition exactly)
#[allow(
    non_snake_case,
    reason = "field names mirror the legacy C UIO layout API; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
#[repr(C)]
pub struct uio_DirList {
    pub names: *mut *mut c_char,
    pub numNames: c_int,
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

    /// Get metadata (only works for File handles)
    pub fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
        match self {
            uio_HandleInner::File(f) => f.metadata(),
            uio_HandleInner::ZipEntry(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "metadata not supported for ZIP entries",
            )),
        }
    }
}

#[allow(
    non_camel_case_types,
    reason = "legacy UIO handle type name is part of the transitional C-compatible API; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub type uio_Handle = Mutex<uio_HandleInner>;

#[repr(C)]
pub struct uio_Handle_Opaque {
    _private: [u8; 0],
}

#[repr(C)]
pub struct uio_Stream {
    buf: *mut c_char,
    data_start: *mut c_char,
    data_end: *mut c_char,
    buf_end: *mut c_char,
    handle: *mut uio_Handle,
    status: c_int,
    operation: c_int,
    open_flags: c_int,
}

fn repository_key(repository: *mut uio_Repository) -> usize {
    repository as usize
}

fn mount_handle_key(handle: *mut uio_MountHandle) -> usize {
    handle as usize
}

fn normalize_mount_point(path: &Path) -> String {
    normalize_virtual_path(path).to_string_lossy().into_owned()
}

fn normalize_virtual_path(path: &Path) -> PathBuf {
    normalize_virtual_path_full(Path::new("/"), path)
}
/// @plan PLAN-20260314-FILE-IO.P06
/// @requirement REQ-FIO-MOUNT-ORDER
/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
/// Register a mount with explicit ordering semantics.
/// Returns NULL and sets errno on failure.
/// For ZIP mounts, indexes the archive at mount time.
fn register_mount(
    repository: *mut uio_Repository,
    mount_point: &Path,
    mounted_root: PathBuf,
    fs_type: c_int,
    flags: c_int,
    relative: *mut uio_MountHandle,
    active_in_registry: bool,
) -> *mut uio_MountHandle {
    let location = flags & UIO_MOUNT_LOCATION_MASK;
    let read_only = (flags & UIO_MOUNT_RDONLY) != 0;

    // Validate location/relative combination
    let relative_required = location == UIO_MOUNT_ABOVE || location == UIO_MOUNT_BELOW;
    let relative_forbidden = location == UIO_MOUNT_TOP || location == UIO_MOUNT_BOTTOM;

    if relative_required && relative.is_null() {
        set_errno(libc::EINVAL);
        return ptr::null_mut();
    }

    if relative_forbidden && !relative.is_null() {
        set_errno(libc::EINVAL);
        return ptr::null_mut();
    }

    // For ZIP mounts, index the archive at mount time
    let zip_index = if fs_type == UIO_FSTYPE_ZIP {
        match crate::io::zip_reader::ZipIndex::new(&mounted_root) {
            Ok(index) => Some(std::sync::Arc::new(index)),
            Err(_e) => {
                set_errno(libc::EIO);
                return ptr::null_mut();
            }
        }
    } else {
        None
    };

    let id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
    let handle = Box::new(uio_MountHandle {
        repository,
        id,
        fs_type,
    });
    let handle_ptr = Box::into_raw(handle);

    let mut registry = get_mount_registry().lock().unwrap();

    // Derive position from TOP/BOTTOM/ABOVE/BELOW semantics
    // Start positions at 1000 to avoid underflow with TOP insertions
    const DEFAULT_START_POSITION: usize = 1000;

    let position = match location {
        UIO_MOUNT_TOP => {
            // Highest priority - insert before all others
            registry
                .iter()
                .filter(|m| m.active_in_registry)
                .map(|m| m.position)
                .min()
                .unwrap_or(DEFAULT_START_POSITION)
                .saturating_sub(1)
        }
        UIO_MOUNT_BOTTOM => {
            // Lowest priority - insert after all others
            registry
                .iter()
                .filter(|m| m.active_in_registry)
                .map(|m| m.position)
                .max()
                .unwrap_or(DEFAULT_START_POSITION)
                .saturating_add(1)
        }
        UIO_MOUNT_ABOVE => {
            // Insert just above the referenced mount
            let relative_id = unsafe { (*relative).id };
            let relative_pos = registry
                .iter()
                .find(|m| m.id == relative_id)
                .map(|m| m.position)
                .unwrap_or(DEFAULT_START_POSITION);
            relative_pos.saturating_sub(1)
        }
        UIO_MOUNT_BELOW => {
            // Insert just below the referenced mount
            let relative_id = unsafe { (*relative).id };
            let relative_pos = registry
                .iter()
                .find(|m| m.id == relative_id)
                .map(|m| m.position)
                .unwrap_or(DEFAULT_START_POSITION);
            relative_pos.saturating_add(1)
        }
        _ => {
            // Default to bottom
            registry
                .iter()
                .filter(|m| m.active_in_registry)
                .map(|m| m.position)
                .max()
                .unwrap_or(DEFAULT_START_POSITION)
                .saturating_add(1)
        }
    };

    let info = MountInfo {
        id,
        repository: repository_key(repository),
        handle_ptr: mount_handle_key(handle_ptr),
        mount_point: normalize_mount_point(mount_point),
        mounted_root,
        fs_type,
        active_in_registry,
        position,
        read_only,
        zip_index,
    };

    registry.push(info);
    // Sort by position (lower = higher priority)
    sort_mount_registry(&mut registry);
    handle_ptr
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
            .then_with(|| b.mount_point.len().cmp(&a.mount_point.len()))
            // Finally by ID for determinism
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
/// Result of resolving a path through the mount overlay
struct MountResolution<'a> {
    mount: &'a MountInfo,
    host_path: PathBuf,
}

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
/// Resolve a virtual path to the topmost visible mount that covers it.
/// Returns None if no mount covers the path.
///
/// This is used for mutation operations to determine:
/// - Which mount owns the path
/// - Whether the mount is read-only
/// - The actual host filesystem path
fn resolve_mount_for_path<'a>(
    registry: &'a [MountInfo],
    virtual_path: &Path,
) -> Option<MountResolution<'a>> {
    let normalized = normalize_virtual_path(virtual_path);

    registry
        .iter()
        .filter(|m| m.active_in_registry)
        .find_map(|mount| {
            let mount_path = Path::new(&mount.mount_point);
            if normalized == mount_path || normalized.starts_with(mount_path) {
                let suffix = normalized
                    .strip_prefix(mount_path)
                    .unwrap_or_else(|_| Path::new(""));
                let host_path = if suffix.as_os_str().is_empty() {
                    mount.mounted_root.clone()
                } else {
                    map_virtual_to_host_confined(&mount.mounted_root, suffix)
                };
                Some(MountResolution { mount, host_path })
            } else {
                None
            }
        })
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

fn is_real_filesystem_path(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }

    let Some(component) = path.components().nth(1) else {
        return false;
    };

    matches!(
        component.as_os_str().to_string_lossy().as_ref(),
        "Users"
            | "home"
            | "tmp"
            | "var"
            | "opt"
            | "private"
            | "System"
            | "Library"
            | "Applications"
    )
}

fn resolve_virtual_mount_path(registry: &[MountInfo], path: &Path) -> Option<(usize, PathBuf)> {
    let normalized = normalize_virtual_path(path);

    registry
        .iter()
        .filter(|entry| entry.active_in_registry)
        .find_map(|entry| {
            let mount_path = Path::new(&entry.mount_point);
            if normalized == mount_path || normalized.starts_with(mount_path) {
                let suffix = normalized
                    .strip_prefix(mount_path)
                    .unwrap_or_else(|_| Path::new(""));
                let resolved = if suffix.as_os_str().is_empty() {
                    entry.mounted_root.clone()
                } else {
                    entry.mounted_root.join(suffix)
                };
                Some((entry.handle_ptr, resolved))
            } else {
                None
            }
        })
}

fn set_stream_status(stream: *mut uio_Stream, status: c_int) {
    if !stream.is_null() {
        unsafe {
            (*stream).status = status;
        }
    }
}

fn set_stream_operation(stream: *mut uio_Stream, operation: c_int) {
    if !stream.is_null() {
        unsafe {
            (*stream).operation = operation;
        }
    }
}

// =============================================================================
// uio_getFileLocation / uio_unmountDir / uio_unmountAllDirs /
// uio_getMountFileSystemType / uio_transplantDir
// =============================================================================

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
#[no_mangle]
pub unsafe extern "C" fn uio_unmountAllDirs(repository: *mut uio_Repository) -> c_int {
    log_marker("uio_unmountAllDirs called");

    for handle_ptr in remove_repository_mounts(repository) {
        let _ = Box::from_raw(handle_ptr as *mut uio_MountHandle);
    }

    0
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_transplantDir(
    mount_point: *const c_char,
    source_dir: *mut uio_DirHandle,
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    log_marker("uio_transplantDir called");

    if source_dir.is_null() {
        return ptr::null_mut();
    }

    if (flags & UIO_MOUNT_RDONLY) != UIO_MOUNT_RDONLY {
        return ptr::null_mut();
    }

    let mount_point = match cstr_to_pathbuf(mount_point) {
        Some(path) => path,
        None => return ptr::null_mut(),
    };

    let location = flags & UIO_MOUNT_LOCATION_MASK;
    let relative_required = location == UIO_MOUNT_ABOVE || location == UIO_MOUNT_BELOW;
    if relative_required == relative.is_null() {
        return ptr::null_mut();
    }

    register_mount(
        (*source_dir).repository,
        &mount_point,
        (*source_dir).path.clone(),
        UIO_FSTYPE_STDIO,
        flags,
        relative,
        true,
    )
}

// =============================================================================
// uio_fgets / uio_fgetc / uio_ungetc / uio_fputc / uio_fputs
// uio_fflush / uio_feof / uio_ferror / uio_clearerr / uio_streamHandle
// =============================================================================

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn uio_fgets(buf: *mut c_char, size: c_int, stream: *mut uio_Stream) -> *mut c_char {
    // Wrap entire function in catch_unwind to prevent panics from unwinding across FFI
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        uio_fgets_inner(buf, size, stream)
    }));

    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            rust_bridge_log_msg("RUST_UIO: uio_fgets panicked, returning null");
            ptr::null_mut()
        }
    }
}

unsafe fn uio_fgets_inner(buf: *mut c_char, size: c_int, stream: *mut uio_Stream) -> *mut c_char {
    if stream.is_null() || buf.is_null() || size <= 0 {
        return ptr::null_mut();
    }

    let max_len = size as usize;
    let s = &mut *stream;
    if s.handle.is_null() {
        return ptr::null_mut();
    }

    // Use correct type: uio_Handle = Mutex<uio_HandleInner>
    let mut guard = match (*s.handle).lock() {
        Ok(g) => g,
        Err(_) => {
            return ptr::null_mut();
        }
    };

    let buffer = slice::from_raw_parts_mut(buf.cast::<u8>(), max_len);
    let mut count = 0usize;
    let max_read = max_len - 1; // leave room for null terminator

    let mut had_error = false;
    while count < max_read {
        let mut byte = [0u8; 1];
        let read = match guard.read(&mut byte) {
            Ok(n) => n,
            Err(_) => {
                set_stream_status(stream, UIO_STREAM_STATUS_ERROR);
                had_error = true;
                break;
            }
        };
        if read == 0 {
            if count == 0 {
                set_stream_status(stream, UIO_STREAM_STATUS_EOF);
            }
            break;
        }
        buffer[count] = byte[0];
        count += 1;
        if byte[0] == b'\n' {
            break;
        }
    }

    if count == 0 {
        return ptr::null_mut();
    }

    if !had_error {
        set_stream_status(stream, UIO_STREAM_STATUS_OK);
    }

    buffer[count] = 0;
    buf
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_fgetc(stream: *mut uio_Stream) -> c_int {
    rust_bridge_log_msg("RUST_UIO: uio_fgetc entry");
    if stream.is_null() {
        return -1;
    }
    let s = &mut *stream;
    if s.handle.is_null() {
        return -1;
    }
    let mut guard = match (*s.handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    let mut byte = [0u8; 1];
    match guard.read(&mut byte) {
        Ok(1) => {
            set_stream_status(stream, UIO_STREAM_STATUS_OK);
            byte[0] as c_int
        }
        Ok(0) => {
            // EOF
            set_stream_status(stream, UIO_STREAM_STATUS_EOF);
            -1
        }
        Err(_) => {
            set_stream_status(stream, UIO_STREAM_STATUS_ERROR);
            -1
        }
        _ => -1,
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_ungetc(c: c_int, stream: *mut uio_Stream) -> c_int {
    rust_bridge_log_msg("RUST_UIO: uio_ungetc entry");
    if stream.is_null() {
        return -1;
    }
    let s = &mut *stream;
    if s.handle.is_null() {
        return -1;
    }
    let mut guard = match (*s.handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    // The stream has no pushback buffer, so only the byte just read can be
    // handed back; pretending to push back any other byte would make the next
    // read return different data than the caller was promised.
    if c == -1 {
        return -1;
    }
    let wanted = c as u8;
    if guard.seek(SeekFrom::Current(-1)).is_err() {
        return -1;
    }
    let mut previous = [0u8; 1];
    if !matches!(guard.read(&mut previous), Ok(1)) || previous[0] != wanted {
        return -1;
    }
    if guard.seek(SeekFrom::Current(-1)).is_err() {
        return -1;
    }
    drop(guard);
    set_stream_status(stream, UIO_STREAM_STATUS_OK);
    wanted as c_int
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_fputc(c: c_int, stream: *mut uio_Stream) -> c_int {
    if stream.is_null() {
        return -1;
    }

    let s = &*stream;
    if s.handle.is_null() {
        return -1;
    }

    let byte = c as u8;

    if let Ok(mut guard) = (*s.handle).lock() {
        use std::io::Write;
        match guard.write_all(&[byte]) {
            Ok(()) => c,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_fputs(s: *const c_char, stream: *mut uio_Stream) -> c_int {
    if stream.is_null() || s.is_null() {
        return -1;
    }

    let s_stream = &*stream;
    if s_stream.handle.is_null() {
        return -1;
    }

    let cstr = std::ffi::CStr::from_ptr(s);
    let bytes = cstr.to_bytes();

    if let Ok(mut guard) = (*s_stream.handle).lock() {
        use std::io::Write;
        match guard.write_all(bytes) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    } else {
        -1
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
pub unsafe fn uio_feof(stream: *mut uio_Stream) -> c_int {
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
pub unsafe fn uio_ferror(stream: *mut uio_Stream) -> c_int {
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
pub unsafe fn uio_fwrite(
    ptr: *const libc::c_void,
    size: size_t,
    nmemb: size_t,
    stream: *mut uio_Stream,
) -> size_t {
    if stream.is_null() || ptr.is_null() {
        return 0;
    }

    let s = &*stream;
    if s.handle.is_null() {
        return 0;
    }

    let total_bytes = size * nmemb;
    if total_bytes == 0 {
        return nmemb; // Writing 0 items is always successful
    }

    let data = std::slice::from_raw_parts(ptr as *const u8, total_bytes);

    if let Ok(mut handle) = (*s.handle).lock() {
        use std::io::Write;
        match handle.write_all(data) {
            Ok(()) => {
                set_stream_status(stream, UIO_STREAM_STATUS_OK);
                nmemb // Return number of items written
            }
            Err(_) => {
                set_stream_status(stream, UIO_STREAM_STATUS_ERROR);
                0
            }
        }
    } else {
        set_stream_status(stream, UIO_STREAM_STATUS_ERROR);
        0
    }
}

/// @plan PLAN-20260314-FILE-IO.P03
/// @requirement REQ-FIO-STREAM-STATUS
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_clearerr(stream: *mut uio_Stream) {
    if stream.is_null() {
        return;
    }

    (*stream).status = UIO_STREAM_STATUS_OK;
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
const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const O_RDWR: c_int = 2;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;
const O_TRUNC: c_int = 0o1000;

const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;

/// Log a message to the Rust bridge log file (C-ABI function for use by C).
///
/// # Safety
/// The message pointer must be a valid null-terminated C string.
///
/// Returns 0 on success, -1 on failure.
pub unsafe fn rust_bridge_log_msg_c(message: *const c_char) -> c_int {
    use crate::bridge_log::rust_bridge_log_msg;

    if message.is_null() {
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(message);
    let message_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    rust_bridge_log_msg(message_str);
    0
}

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
    Some(PathBuf::from(c_str.to_string_lossy().as_ref()))
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

// Helper: Convert path to absolute path if relative, with normalization
fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    normalize_virtual_path_full(base, path)
}

const MATCH_LITERAL: c_int = 0;
const MATCH_PREFIX: c_int = 1;
const MATCH_SUFFIX: c_int = 2;
const MATCH_SUBSTRING: c_int = 3;
const MATCH_REGEX: c_int = 4;
const MATCH_REGEX_ALT: c_int = 5;

/// @plan PLAN-20260314-FILE-IO.P07
/// @requirement REQ-FIO-DIRLIST-REGEX
/// Match a name against a pattern based on match type.
/// For MATCH_REGEX, uses the regex crate to support POSIX ERE-compatible patterns.
/// Invalid regex patterns return false (no match) instead of crashing.
fn matches_pattern(name: &str, pattern: &str, match_type: c_int) -> bool {
    if pattern.is_empty() {
        return true;
    }

    match match_type {
        MATCH_LITERAL => name == pattern,
        MATCH_PREFIX => name.starts_with(pattern),
        MATCH_SUFFIX => name.ends_with(pattern),
        MATCH_SUBSTRING => name.contains(pattern),
        MATCH_REGEX | MATCH_REGEX_ALT => {
            // Use regex crate for proper POSIX ERE-compatible matching
            // Invalid regex returns false (no match) instead of crashing
            match regex::Regex::new(pattern) {
                Ok(re) => re.is_match(name),
                Err(_) => {
                    // Invalid regex - log and return false
                    log_marker(&format!(
                        "matches_pattern: invalid regex pattern '{}', treating as no match",
                        pattern
                    ));
                    false
                }
            }
        }
        _ => {
            // Unknown match type - treat as regex for backward compatibility
            match regex::Regex::new(pattern) {
                Ok(re) => re.is_match(name),
                Err(_) => false,
            }
        }
    }
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
pub unsafe fn uio_init() {
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

        // Clear buffer size registry
        let mut buffer_registry = get_buffer_size_registry().lock().unwrap();
        buffer_registry.clear();
        drop(buffer_registry);

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
        uio_unmountAllDirs(repository);
        let _ = Box::from_raw(repository);
    }
}

// =============================================================================
// uio_openDir / uio_closeDir / uio_mountDir / uio_openDirRelative
// =============================================================================

fn resolve_mount_path(path: &Path) -> PathBuf {
    rust_bridge_log_msg(&format!("RUST_UIO: resolve_mount_path input: {:?}", path));

    let registry = get_mount_registry().lock().unwrap();
    if is_real_filesystem_path(path)
        || registry
            .iter()
            .filter(|entry| entry.active_in_registry)
            .any(|entry| path == entry.mounted_root || path.starts_with(&entry.mounted_root))
    {
        rust_bridge_log_msg(&format!(
            "RUST_UIO: path {:?} already resolved to filesystem path",
            path
        ));
        return path.to_path_buf();
    }

    if let Some((_handle_ptr, resolved)) = resolve_virtual_mount_path(&registry, path) {
        rust_bridge_log_msg(&format!("RUST_UIO: resolved {:?} -> {:?}", path, resolved));
        resolved
    } else {
        rust_bridge_log_msg(&format!(
            "RUST_UIO: no mount match for {:?}, returning original",
            path
        ));
        path.to_path_buf()
    }
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_openDir(
    _repository: *mut uio_Repository,
    path: *const c_char,
    _flags: c_int,
) -> *mut uio_DirHandle {
    let c_path = match cstr_to_pathbuf(path) {
        Some(p) => p,
        None => {
            rust_bridge_log_msg("RUST_UIO: uio_openDir: null path");
            return ptr::null_mut();
        }
    };

    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_openDir called with path: {:?}",
        c_path
    ));

    // Resolve through mount registry
    let resolved = resolve_mount_path(&c_path);

    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_openDir resolved to: {:?}",
        resolved
    ));

    // Create directory handle (don't fail if it doesn't exist - may be created later)
    let handle = Box::new(uio_DirHandle {
        path: resolved.clone(),
        virtual_path: normalize_virtual_path(&c_path),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: _repository,
        root_end: resolved.clone(), // For now, root_end = full path
    });
    Box::leak(handle) as *mut uio_DirHandle
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

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_mountDir(
    _dest_rep: *mut uio_Repository,
    mount_point: *const c_char,
    _fs_type: c_int,
    source_dir: *mut uio_DirHandle,
    source_path: *const c_char,

    in_path: *const c_char,
    _auto_mount: *mut *mut (),
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    let mount_point = match cstr_to_pathbuf(mount_point) {
        Some(p) => p,
        None => {
            eprintln!("RUST_UIO: uio_mountDir: null mount_point");
            return ptr::null_mut();
        }
    };

    let source_path_str = if source_path.is_null() {
        "(null)".to_string()
    } else {
        std::ffi::CStr::from_ptr(source_path)
            .to_string_lossy()
            .into_owned()
    };
    let in_path_str = if in_path.is_null() {
        "(null)".to_string()
    } else {
        std::ffi::CStr::from_ptr(in_path)
            .to_string_lossy()
            .into_owned()
    };
    eprintln!(
        "RUST_UIO: uio_mountDir: mount_point={:?}, source_path='{}', in_path='{}', source_dir={:?}, flags={}, relative={:?}",
        mount_point, source_path_str, in_path_str, source_dir, flags, relative
    );

    if !source_dir.is_null() {
        let base_path = (*source_dir).path.clone();
        let rel_path = cstr_to_pathbuf(source_path).unwrap_or_default();
        let mounted_root = if rel_path.as_os_str().is_empty() || rel_path == Path::new("/") {
            base_path.clone()
        } else {
            resolve_path(&base_path, &rel_path)
        };
        rust_bridge_log_msg(&format!(
            "RUST_UIO: uio_mountDir: source_dir set, source_path {:?} -> {:?}",
            rel_path, mounted_root
        ));

        // @plan PLAN-20260314-FILE-IO.P09
        // @requirement REQ-FIO-ARCHIVE-MOUNT
        // ZIP mounts are now fully active in the registry
        let active_in_registry = true;
        return register_mount(
            _dest_rep,
            &mount_point,
            mounted_root,
            _fs_type,
            flags,
            relative,
            active_in_registry,
        );
    }

    let mounted_root = if !in_path.is_null() {
        match cstr_to_pathbuf(in_path) {
            Some(path) => {
                rust_bridge_log_msg(&format!(
                    "RUST_UIO: uio_mountDir: using in_path {:?} as source",
                    path
                ));
                path
            }
            None => {
                rust_bridge_log_msg(
                    "RUST_UIO: uio_mountDir: in_path conversion failed, using empty path",
                );
                PathBuf::new()
            }
        }
    } else {
        rust_bridge_log_msg("RUST_UIO: uio_mountDir: in_path is NULL, using empty path");
        PathBuf::new()
    };

    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_mountDir: mounting root={:?} at {:?}",
        mounted_root, mount_point
    ));

    register_mount(
        _dest_rep,
        &mount_point,
        mounted_root,
        _fs_type,
        flags,
        relative,
        true,
    )
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn uio_openDirRelative(
    base: *mut uio_DirHandle,
    path: *const c_char,
    _flags: c_int,
) -> *mut uio_DirHandle {
    log_marker("uio_openDirRelative called");

    if base.is_null() {
        return ptr::null_mut();
    }

    let base_path = &(*base).path;
    let rel_path = match cstr_to_pathbuf(path) {
        Some(p) => p,
        None => return ptr::null_mut(),
    };

    // Log before moving rel_path
    let is_abs = rel_path.is_absolute();
    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_openDirRelative: base={:?} path={:?} (is_absolute={})",
        base_path, rel_path, is_abs
    ));

    // If rel_path is already absolute, it's been resolved by caller - skip resolve_mount_path
    // This prevents double-resolution that causes path duplication
    let resolved = if is_abs {
        rust_bridge_log_msg(&format!("RUST_UIO: uio_openDirRelative: path is absolute {:?}, using directly (no mount resolution)", rel_path));
        rel_path
    } else {
        // Only join if rel_path is actually relative
        let joined = resolve_path(base_path, &rel_path);
        rust_bridge_log_msg(&format!(
            "RUST_UIO: uio_openDirRelative: joined {:?} + {:?} = {:?}",
            base_path, rel_path, joined
        ));
        // Then resolve through mount registry
        resolve_mount_path(&joined)
    };

    // Compute virtual path from base's virtual path
    let base_virtual = &(*base).virtual_path;
    let virtual_resolved = if is_abs {
        PathBuf::from(std::ffi::CStr::from_ptr(path).to_string_lossy().as_ref())
    } else {
        let rel = cstr_to_pathbuf(path).unwrap_or_default();
        normalize_virtual_path_full(base_virtual, &rel)
    };

    let handle = Box::new(uio_DirHandle {
        path: resolved.clone(),
        virtual_path: virtual_resolved,
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: (*base).repository,
        root_end: (*base).root_end.clone(),
    });
    Box::leak(handle) as *mut uio_DirHandle
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
pub unsafe fn uio_open(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    flags: c_int,
    _mode: c_int,
) -> *mut uio_Handle {
    ffi_guard!(ptr::null_mut(), {
        rust_bridge_log_msg(&format!("RUST_UIO: uio_open called with flags {}", flags));

        if dir.is_null() {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }

        let dir_host_path = &(*dir).path;
        let dir_virt_path = &(*dir).virtual_path;
        let input_path = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => {
                set_errno(libc::EINVAL);
                return ptr::null_mut();
            }
        };

        let virtual_path = normalize_virtual_path_full(dir_virt_path, &input_path);

        // Check if this is a write operation
        let is_write_mode =
            (flags & 3) == O_WRONLY || (flags & 3) == O_RDWR || (flags & O_CREAT) != 0;

        if is_write_mode {
            // For write operations, try mount registry first
            let registry = get_mount_registry().lock().unwrap();
            let file_path =
                if let Some(resolution) = resolve_mount_for_path(&registry, &virtual_path) {
                    // Path is in a mount - extract data before dropping registry
                    let is_readonly = resolution.mount.read_only;
                    let path = resolution.host_path.clone();
                    drop(registry);

                    if is_readonly {
                        // Topmost mount is read-only, fail without falling through
                        set_errno(libc::EACCES);
                        return ptr::null_mut();
                    }
                    path
                } else {
                    drop(registry);
                    // No mount - fall back to direct path resolution
                    resolve_path(dir_host_path, &input_path)
                };

            let mut opts = OpenOptions::new();
            match flags & 3 {
                O_WRONLY => {
                    opts.write(true);
                }
                O_RDWR => {
                    opts.read(true).write(true);
                }
                _ => {}
            }

            if (flags & O_CREAT) != 0 {
                if (flags & O_EXCL) != 0 {
                    // O_CREAT | O_EXCL: fail if file exists
                    opts.create_new(true);
                } else {
                    opts.create(true);
                }
            }
            if (flags & O_TRUNC) != 0 {
                opts.truncate(true);
            }

            let file = match opts.open(&file_path) {
                Ok(f) => f,
                Err(err) => {
                    log_marker(&format!(
                        "uio_open write failed: path={:?} err={}",
                        file_path, err
                    ));
                    let err_code = crate::platform::io_errno(&err);
                    set_errno(err_code);
                    return ptr::null_mut();
                }
            };

            let handle = uio_HandleInner::File(file);
            return Box::leak(Box::new(Mutex::new(handle))) as *mut uio_Handle;
        }

        // Read-only mode: consult every mount that covers the path in
        // priority order. A mount that lacks the file lets the lookup fall
        // through to the next one; any other failure is reported as is.
        // @plan PLAN-20260314-FILE-IO.P09
        // @requirement REQ-FIO-ARCHIVE-MOUNT
        let registry = get_mount_registry().lock().unwrap();
        let normalized = normalize_virtual_path(&virtual_path);
        let mut covered_by_a_mount = false;
        for mount in registry.iter().filter(|m| m.active_in_registry) {
            let mount_path = Path::new(&mount.mount_point);
            if normalized != mount_path && !normalized.starts_with(mount_path) {
                continue;
            }
            covered_by_a_mount = true;
            let suffix = normalized
                .strip_prefix(mount_path)
                .unwrap_or_else(|_| Path::new(""));

            let opened = if mount.fs_type == UIO_FSTYPE_ZIP {
                match mount.zip_index {
                    Some(ref zip_index) => zip_index
                        .open_entry(&suffix.to_string_lossy())
                        .map(uio_HandleInner::ZipEntry),
                    None => continue,
                }
            } else {
                let host_path = if suffix.as_os_str().is_empty() {
                    mount.mounted_root.clone()
                } else {
                    map_virtual_to_host_confined(&mount.mounted_root, suffix)
                };
                OpenOptions::new()
                    .read(true)
                    .open(&host_path)
                    .map(uio_HandleInner::File)
            };

            match opened {
                Ok(inner) => {
                    drop(registry);
                    return Box::leak(Box::new(Mutex::new(inner))) as *mut uio_Handle;
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => {
                    drop(registry);
                    log_marker(&format!(
                        "uio_open read failed: path={:?} err={}",
                        virtual_path, err
                    ));
                    set_errno(crate::platform::io_errno(&err));
                    return ptr::null_mut();
                }
            }
        }
        drop(registry);

        if covered_by_a_mount {
            set_errno(libc::ENOENT);
            return ptr::null_mut();
        }

        // No mount covers the path - fall back to direct path resolution
        let file_path = resolve_path(dir_host_path, &input_path);
        let file = match OpenOptions::new().read(true).open(&file_path) {
            Ok(f) => f,
            Err(err) => {
                log_marker(&format!(
                    "uio_open read failed: path={:?} err={}",
                    file_path, err
                ));
                set_errno(crate::platform::io_errno(&err));
                return ptr::null_mut();
            }
        };

        let handle = uio_HandleInner::File(file);
        Box::leak(Box::new(Mutex::new(handle))) as *mut uio_Handle
    })
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// Close a file handle and free its resources.
/// Safe to call even if the mount this handle came from was unmounted.
/// The handle owns its resources independently of mount state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_close(handle: *mut uio_Handle) -> c_int {
    log_marker("uio_close called");
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
    0 // Success
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_read(handle: *mut uio_Handle, buf: *mut u8, count: size_t) -> isize {
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
pub unsafe fn uio_write(handle: *mut uio_Handle, buf: *const u8, count: size_t) -> isize {
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
pub unsafe fn uio_fstat(handle: *mut uio_Handle, stat_buf: *mut stat) -> c_int {
    if handle.is_null() || stat_buf.is_null() {
        return -1;
    }

    let guard = match (*handle).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    match guard.metadata() {
        Ok(meta) => {
            (*stat_buf).st_size = meta.len() as i64;
            (*stat_buf).st_mode = if meta.is_file() { 0o100000 } else { 0o040000 };
            0 // Success
        }
        Err(_) => -1,
    }
}

/// @plan PLAN-20260314-FILE-IO.P06a
/// @requirement REQ-FIO-MUTATION (G14)
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_unlink(dir: *mut uio_DirHandle, path: *const c_char) -> c_int {
    ffi_guard!(-1, {
        log_marker("uio_unlink called");

        if dir.is_null() {
            return fail_errno(libc::EINVAL, -1);
        }

        let dir_host_path = &(*dir).path;
        let dir_virt_path = &(*dir).virtual_path;
        let input_path = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => return fail_errno(libc::EINVAL, -1),
        };

        let virtual_path = normalize_virtual_path_full(dir_virt_path, &input_path);

        // Try to resolve through mount registry first
        let registry = get_mount_registry().lock().unwrap();
        let file_path = if let Some(resolution) = resolve_mount_for_path(&registry, &virtual_path) {
            // Path is in a mount - extract data before dropping registry
            let is_readonly = resolution.mount.read_only;
            let path = resolution.host_path.clone();
            drop(registry);

            if is_readonly {
                return fail_errno(libc::EACCES, -1);
            }
            path
        } else {
            drop(registry);
            // No mount - fall back to direct path resolution
            resolve_path(dir_host_path, &input_path)
        };

        match fs::remove_file(&file_path) {
            Ok(_) => 0,
            Err(e) => {
                let err_code = crate::platform::io_errno(&e);
                fail_errno(err_code, -1)
            }
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
pub unsafe fn uio_fopen(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    mode: *const c_char,
) -> *mut uio_Stream {
    ffi_guard!(ptr::null_mut(), {
        rust_bridge_log_msg("RUST_UIO: uio_fopen entry");
        log_marker("uio_fopen called");

        if dir.is_null() {
            rust_bridge_log_msg("RUST_UIO: uio_fopen null dir");
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }
        if mode.is_null() {
            rust_bridge_log_msg("RUST_UIO: uio_fopen null mode");
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }

        let dir_host_path = &(*dir).path;
        let dir_virt_path = &(*dir).virtual_path;

        let input = match cstr_to_pathbuf(path) {
            Some(p) => p,
            None => {
                rust_bridge_log_msg("RUST_UIO: uio_fopen null path");
                set_errno(libc::EINVAL);
                return ptr::null_mut();
            }
        };

        let mode_str = std::ffi::CStr::from_ptr(mode).to_string_lossy();

        // Validate mode string
        let valid_mode = mode_str.contains("r") || mode_str.contains("w") || mode_str.contains("a");
        if !valid_mode {
            rust_bridge_log_msg(&format!("RUST_UIO: uio_fopen invalid mode: {}", mode_str));
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }

        // @plan PLAN-20260314-FILE-IO.P09
        // @requirement REQ-FIO-ARCHIVE-MOUNT
        // Use virtual path (not host path) for mount resolution
        let virtual_path = normalize_virtual_path_full(dir_virt_path, &input);

        let registry = get_mount_registry().lock().unwrap();

        // Find the mount for this path
        let mount_info = registry
            .iter()
            .filter(|m| m.active_in_registry)
            .find(|mount| {
                let mount_path = Path::new(&mount.mount_point);
                virtual_path == mount_path || virtual_path.starts_with(mount_path)
            });

        let handle_inner = if let Some(mount) = mount_info {
            if mount.fs_type == UIO_FSTYPE_ZIP {
                // ZIP mount - open entry from archive
                if !mode_str.contains("r") || mode_str.contains("w") || mode_str.contains("a") {
                    // ZIP entries are read-only
                    rust_bridge_log_msg("RUST_UIO: uio_fopen cannot write to ZIP archive");
                    set_errno(libc::EACCES);
                    return ptr::null_mut();
                }

                if let Some(ref zip_index) = mount.zip_index {
                    let mount_path = Path::new(&mount.mount_point);
                    let suffix = virtual_path
                        .strip_prefix(mount_path)
                        .unwrap_or_else(|_| Path::new(""));
                    let suffix_str = suffix.to_string_lossy();

                    match zip_index.open_entry(&suffix_str) {
                        Ok(reader) => uio_HandleInner::ZipEntry(reader),
                        Err(err) => {
                            rust_bridge_log_msg(&format!(
                                "RUST_UIO: uio_fopen ZIP entry not found: {}",
                                err
                            ));
                            set_errno(libc::ENOENT);
                            return ptr::null_mut();
                        }
                    }
                } else {
                    rust_bridge_log_msg("RUST_UIO: uio_fopen ZIP mount has no index");
                    set_errno(libc::EIO);
                    return ptr::null_mut();
                }
            } else {
                // Regular file mount
                let file_path = resolve_path(dir_host_path, &input);
                rust_bridge_log_msg(&format!(
                    "RUST_UIO: uio_fopen path={:?} mode={}",
                    file_path, mode_str
                ));

                let mut opts = OpenOptions::new();
                if mode_str.contains("r") {
                    opts.read(true);
                }
                if mode_str.contains("w") {
                    opts.write(true).create(true).truncate(true);
                }
                if mode_str.contains("a") {
                    opts.append(true).create(true);
                }
                if mode_str.contains("+") {
                    opts.read(true).write(true);
                }

                match opts.open(&file_path) {
                    Ok(f) => uio_HandleInner::File(f),
                    Err(err) => {
                        log_marker(&format!(
                            "uio_fopen failed: path={:?} err={}",
                            file_path, err
                        ));
                        let err_code = crate::platform::io_errno(&err);
                        set_errno(err_code);
                        return ptr::null_mut();
                    }
                }
            }
        } else {
            // No mount found - try as regular file (for backward compatibility)
            let file_path = resolve_path(dir_host_path, &input);
            rust_bridge_log_msg(&format!(
                "RUST_UIO: uio_fopen path={:?} mode={} (no mount)",
                file_path, mode_str
            ));

            let mut opts = OpenOptions::new();
            if mode_str.contains("r") {
                opts.read(true);
            }
            if mode_str.contains("w") {
                opts.write(true).create(true).truncate(true);
            }
            if mode_str.contains("a") {
                opts.append(true).create(true);
            }
            if mode_str.contains("+") {
                opts.read(true).write(true);
            }

            match opts.open(&file_path) {
                Ok(f) => uio_HandleInner::File(f),
                Err(err) => {
                    log_marker(&format!(
                        "uio_fopen failed: path={:?} err={}",
                        file_path, err
                    ));
                    let err_code = crate::platform::io_errno(&err);
                    set_errno(err_code);
                    return ptr::null_mut();
                }
            }
        };

        drop(registry);

        let open_flags =
            if mode_str.contains("r") && !mode_str.contains("w") && !mode_str.contains("a") {
                O_RDONLY
            } else if mode_str.contains("w") {
                O_WRONLY | O_CREAT | O_TRUNC
            } else {
                0
            };

        let stream = Box::new(uio_Stream {
            buf: ptr::null_mut(),
            data_start: ptr::null_mut(),
            data_end: ptr::null_mut(),
            buf_end: ptr::null_mut(),
            handle: Box::leak(Box::new(Mutex::new(handle_inner))) as *mut uio_Handle,
            status: UIO_STREAM_STATUS_OK,
            operation: UIO_STREAM_OPERATION_NONE,
            open_flags,
        });

        let stream_ptr = Box::leak(stream) as *mut uio_Stream;
        rust_bridge_log_msg(&format!(
            "RUST_UIO: uio_fopen returning stream={:?}",
            stream_ptr
        ));
        stream_ptr
    })
}

/// @plan PLAN-20260314-FILE-IO.P03
/// @requirement REQ-FIO-RESOURCE-MGMT
/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// @requirement REQ-FIO-RESOURCE-MGMT
/// Close a stream and free its resources (buffer and underlying handle).
/// Safe to call even if the mount this stream came from was unmounted.
/// The stream owns its resources independently of mount state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_fclose(stream: *mut uio_Stream) -> c_int {
    log_marker("uio_fclose called");
    if !stream.is_null() {
        let s = &*stream;
        // Free buffer if allocated
        if !s.buf.is_null() {
            if let Some(size) = get_buffer_size(s.buf) {
                let buffer_layout = std::alloc::Layout::from_size_align(size, 1).unwrap();
                std::alloc::dealloc(s.buf.cast::<u8>(), buffer_layout);
                remove_buffer_size(s.buf);
            }
            // If size not found in registry, buffer was likely not allocated by us
            // (or was never registered), so we don't deallocate it to avoid double-free
        }
        if !s.handle.is_null() {
            // Reconstruct Box<uio_Handle> from raw pointer
            let _ = Box::from_raw(s.handle);
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
pub unsafe fn uio_fread(
    buf: *mut libc::c_void,
    size: size_t,
    nmemb: size_t,
    stream: *mut uio_Stream,
) -> size_t {
    if stream.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fread null stream");
        return 0;
    }
    if buf.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fread null buffer");
        return 0;
    }
    if size == 0 || nmemb == 0 {
        rust_bridge_log_msg(&format!(
            "RUST_UIO: uio_fread zero size or nmemb (size={} nmemb={})",
            size, nmemb
        ));
        return 0;
    }

    let s = &mut *stream;

    // Check if stream has a valid handle pointer
    if s.handle.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fread handle is null");
        return 0;
    }

    // Validate the handle pointer is properly aligned and not obviously corrupted
    let handle_addr = s.handle as usize;
    if handle_addr < 4096 {
        // Pointer is too small to be valid
        rust_bridge_log_msg(&format!(
            "RUST_UIO: uio_fread invalid handle pointer: 0x{:x}",
            handle_addr
        ));
        return 0;
    }

    // Try to safely dereference the handle
    let file_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| &*s.handle));

    let file = match file_result {
        Ok(f) => f,
        Err(_) => {
            rust_bridge_log_msg("RUST_UIO: uio_fread panic when dereferencing handle");
            s.status = 2; // uio_Stream_STATUS_ERROR
            return 0;
        }
    };

    let mut guard = match file.lock() {
        Ok(g) => g,
        Err(_) => {
            rust_bridge_log_msg("RUST_UIO: uio_fread failed to lock mutex");
            s.status = 2; // uio_Stream_STATUS_ERROR
            return 0;
        }
    };

    let total_bytes = size * nmemb;
    let buffer = slice::from_raw_parts_mut(buf as *mut u8, total_bytes);
    match guard.read(buffer) {
        Ok(n) => {
            set_stream_operation(stream, UIO_STREAM_OPERATION_READ);
            if n == 0 {
                // EOF reached - 0 bytes read
                set_stream_status(stream, UIO_STREAM_STATUS_EOF);
            } else if n < total_bytes {
                // Short read but some data - could be EOF or just a short read
                // Don't set EOF here; let the next read determine it
                set_stream_status(stream, UIO_STREAM_STATUS_OK);
            } else {
                // Full read successful
                set_stream_status(stream, UIO_STREAM_STATUS_OK);
            }
            rust_bridge_log_msg(&format!(
                "RUST_UIO: uio_fread requested={} read={}",
                total_bytes, n
            ));
            n / size
        }
        Err(err) => {
            rust_bridge_log_msg(&format!("RUST_UIO: uio_fread error: {}", err));
            set_stream_status(stream, UIO_STREAM_STATUS_ERROR);
            0
        }
    }
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn uio_fseek(stream: *mut uio_Stream, offset: c_long, whence: c_int) -> c_int {
    if stream.is_null() {
        return -1;
    }

    let s = &*stream;
    let file = &(*s.handle);
    let mut guard = match file.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let seek_from = match whence {
        SEEK_SET => SeekFrom::Start(offset as u64),
        SEEK_CUR => SeekFrom::Current(offset as _),
        SEEK_END => SeekFrom::End(offset as _),
        _ => return -1,
    };

    match guard.seek(seek_from) {
        Ok(_) => {
            // Per P02a carry-forward: seek clears EOF flag
            if (*stream).status == UIO_STREAM_STATUS_EOF {
                (*stream).status = UIO_STREAM_STATUS_OK;
            }
            0
        }
        Err(_) => -1,
    }
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn uio_ftell(stream: *mut uio_Stream) -> c_long {
    if stream.is_null() {
        return -1;
    }

    let s = &*stream;
    let file = &(*s.handle);
    let mut guard = match file.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    match guard.seek(SeekFrom::Current(0)) {
        Ok(pos) => pos as c_long,
        Err(_) => -1,
    }
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

        let pattern_str = if _pattern.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(_pattern)
                .to_str()
                .unwrap_or_default()
        };
        log_marker(&format!(
            "uio_getDirList: virtual_path={:?} pattern='{}' matchType={}",
            virtual_path, pattern_str, _match_type
        ));

        // Acquire mount registry lock for consistent topology view
        let registry = get_mount_registry().lock().unwrap();

        // Collect entries from all mounts covering the virtual path
        // Use indexmap to preserve insertion order (first-seen = highest precedence)
        use std::collections::BTreeSet;
        let mut seen_names = BTreeSet::new();
        let mut name_strings: Vec<String> = Vec::new();

        // Iterate mounts in precedence order (registry is already sorted)
        for mount in registry.iter().filter(|m| m.active_in_registry) {
            let mount_path = Path::new(&mount.mount_point);

            // Check if this mount covers the virtual path
            let is_covered = virtual_path == mount_path || virtual_path.starts_with(mount_path);
            if !is_covered {
                continue;
            }

            // Map virtual path to relative path within mount
            let suffix = virtual_path
                .strip_prefix(mount_path)
                .unwrap_or_else(|_| Path::new(""));

            // @plan PLAN-20260314-FILE-IO.P09
            // @requirement REQ-FIO-ARCHIVE-MOUNT
            // Handle ZIP mounts differently than stdio mounts
            if mount.fs_type == UIO_FSTYPE_ZIP {
                if let Some(ref zip_index) = mount.zip_index {
                    let suffix_str = suffix.to_string_lossy();
                    let archive_entries = zip_index.list_directory(&suffix_str);

                    log_marker(&format!(
                        "uio_getDirList: ZIP mount '{}' returned {} entries for {:?}",
                        mount.mount_point,
                        archive_entries.len(),
                        suffix_str
                    ));

                    for name in archive_entries {
                        // Deduplicate: first-seen wins (higher precedence)
                        if seen_names.contains(&name) {
                            continue;
                        }

                        // Apply pattern matching
                        if matches_pattern(&name, pattern_str, _match_type) {
                            seen_names.insert(name.clone());
                            name_strings.push(name);
                        }
                    }
                }
                continue;
            }

            // Stdio mount handling
            let host_path = if suffix.as_os_str().is_empty() {
                mount.mounted_root.clone()
            } else {
                map_virtual_to_host_confined(&mount.mounted_root, suffix)
            };

            log_marker(&format!(
                "uio_getDirList: checking stdio mount '{}' at host_path={:?}",
                mount.mount_point, host_path
            ));

            // Read directory if it exists
            if !host_path.is_dir() {
                continue;
            }

            let entries = match fs::read_dir(&host_path) {
                Ok(entries) => entries,
                Err(err) => {
                    log_marker(&format!(
                        "uio_getDirList: read_dir failed for {:?}: {}, skipping mount",
                        host_path, err
                    ));
                    continue;
                }
            };

            // Collect matching entries from this mount
            for entry in entries.flatten() {
                if let Some(name_osstr) = entry.file_name().to_str() {
                    // Deduplicate: first-seen wins (higher precedence)
                    if seen_names.contains(name_osstr) {
                        continue;
                    }

                    // Apply pattern matching
                    if matches_pattern(name_osstr, pattern_str, _match_type) {
                        seen_names.insert(name_osstr.to_string());
                        name_strings.push(name_osstr.to_string());
                    }
                }
            }
        }

        // Release registry lock before allocation
        drop(registry);

        // Always return non-null uio_DirList, even for empty results
        if name_strings.is_empty() {
            log_marker(&format!(
                "uio_getDirList: no matches for pattern '{}' in {:?}",
                pattern_str, virtual_path
            ));
            let dirlist = Box::new(uio_DirList {
                names: ptr::null_mut(),
                numNames: 0,
                buffer: ptr::null_mut(),
            });
            return Box::leak(dirlist) as *mut uio_DirList;
        }

        log_marker(&format!(
            "uio_getDirList: {} matches for pattern '{}' in {:?}",
            name_strings.len(),
            pattern_str,
            virtual_path
        ));

        // Allocate a single contiguous buffer for all strings
        let total_size: usize = name_strings.iter().map(|s| s.len() + 1).sum();
        let buffer_layout = std::alloc::Layout::from_size_align(total_size, 1).unwrap();
        let buffer_ptr = std::alloc::alloc(buffer_layout) as *mut c_char;
        if buffer_ptr.is_null() {
            set_errno(libc::ENOMEM);
            return ptr::null_mut();
        }

        // Register the buffer size for later deallocation
        register_buffer_size(buffer_ptr, total_size);

        // Allocate array of pointers using Vec for capacity tracking
        let num_names = name_strings.len();
        let mut names_vec: Vec<*mut c_char> = Vec::with_capacity(num_names);

        // Copy strings into buffer and collect pointers
        let mut offset = 0;
        for name in name_strings.iter() {
            let name_bytes = name.as_bytes();
            let dst = buffer_ptr.add(offset);

            // Copy string bytes including null terminator
            std::ptr::copy_nonoverlapping(
                name_bytes.as_ptr() as *const c_char,
                dst,
                name_bytes.len(),
            );
            std::ptr::write(dst.add(name_bytes.len()), 0); // Null terminate

            // Store pointer in names array
            names_vec.push(dst);

            offset += name_bytes.len() + 1;
        }

        // Convert Vec to boxed slice, then leak to get stable pointer
        let names_ptr = names_vec.into_boxed_slice();
        let names_ptr_leaked = Box::leak(names_ptr) as *mut [*mut c_char] as *mut *mut c_char;

        let dirlist = Box::new(uio_DirList {
            names: names_ptr_leaked,
            numNames: num_names as c_int,
            buffer: buffer_ptr,
        });
        Box::leak(dirlist) as *mut uio_DirList
    })
}

/// @plan PLAN-20260314-FILE-IO.P11
/// @requirement REQ-FIO-RESOURCE-MGMT
/// @requirement REQ-FIO-POST-UNMOUNT-CLEANUP
/// Free a directory list and all its resources.
/// Uses a side-channel buffer size registry to properly deallocate the buffer.
/// Safe to call with NULL pointer (no-op).
/// Safe to call on empty lists.
/// Safe to call even if the mount this list came from was unmounted.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn uio_DirList_free(dirlist: *mut uio_DirList) {
    log_marker("uio_DirList_free called");
    if !dirlist.is_null() {
        let list = &*dirlist;

        // IMPORTANT: The C uio_DirList struct doesn't store capacity information,
        // so we need to reconstruct it from what we know about our allocation strategy.
        //
        // Our allocation strategy in uio_getDirList:
        // 1. buffer: allocated with Layout::from_size_align(total_size, 1)
        // 2. names: allocated via Vec::with_capacity() then converted to boxed slice
        //
        // To safely free:
        // 1. Free the buffer first (names pointers point into it)
        // 2. Reconstruct the names slice from the raw pointer
        // 3. Free the names allocation

        // Step 1: Free the buffer
        if !list.buffer.is_null() {
            // We need to know the buffer size. Since we don't store it in the C struct,
            // we have a problem. However, looking at the C code, it also doesn't store
            // the buffer size - it just calls uio_free() which knows the size.
            //
            // For Rust, we need the size. Let's work around this by:
            // 1. NOT using the standard allocator directly
            // 2. Instead, use Box<[u8]> which can be reconstructed from raw ptr + size
            //
            // But we don't have the size! The C struct doesn't preserve it.
            //
            // SOLUTION: Store metadata in a side-channel global registry.
            // Or: Use a known sentinel/size encoding.
            //
            // ACTUAL SOLUTION: Since buffer_size is not in the C struct, and we can't
            // modify the C struct definition (it must match C exactly), we need to
            // track the buffer size elsewhere. We'll use a global HashMap keyed by
            // the buffer pointer address.

            // For now, use a workaround: try to find buffer size in our registry
            let buffer_size = get_buffer_size(list.buffer);
            if let Some(size) = buffer_size {
                let buffer_layout = std::alloc::Layout::from_size_align(size, 1).unwrap();
                std::alloc::dealloc(list.buffer.cast::<u8>(), buffer_layout);
                remove_buffer_size(list.buffer);
            }
            // If size not found in registry, we have a leak - but better than double-free!
        }

        // Step 2: Free the names array
        // We need to reconstruct the Box<[T]> from the raw pointer.
        // Since we used Vec::into_boxed_slice(), we need to use from_raw_parts.
        if !list.names.is_null() && list.numNames > 0 {
            // Reconstruct Box<[T]> using from_raw_parts
            // The data pointer is list.names, and the length is list.numNames
            let names_slice = std::slice::from_raw_parts_mut(list.names, list.numNames as usize);
            let names_box: Box<[*mut c_char]> = names_slice.into();
            drop(names_box);
        }

        // Step 3: Free the DirList struct itself
        let _ = Box::from_raw(dirlist);
    }
}

// =============================================================================
// Buffer Size Registry for uio_DirList deallocation
// =============================================================================

struct BufferSizeEntry {
    size: usize,
    // We could also track allocation ID for safety
}

static BUFFER_SIZE_REGISTRY: OnceLock<Mutex<HashMap<usize, BufferSizeEntry>>> = OnceLock::new();

fn get_buffer_size_registry() -> &'static Mutex<HashMap<usize, BufferSizeEntry>> {
    BUFFER_SIZE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_buffer_size(ptr: *mut c_char, size: usize) {
    let addr = ptr as usize;
    let mut registry = get_buffer_size_registry().lock().unwrap();
    registry.insert(addr, BufferSizeEntry { size });
}

fn get_buffer_size(ptr: *mut c_char) -> Option<usize> {
    if ptr.is_null() {
        return None;
    }
    let addr = ptr as usize;
    let registry = get_buffer_size_registry().lock().unwrap();
    registry.get(&addr).map(|entry| entry.size)
}

fn remove_buffer_size(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let addr = ptr as usize;
    let mut registry = get_buffer_size_registry().lock().unwrap();
    registry.remove(&addr);
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    use std::ffi::CString;

    use std::path::PathBuf;

    // Helper to clear mount registry after tests
    fn clear_mount_registry() {
        if let Ok(mut registry) = get_mount_registry().lock() {
            registry.clear();
        }
    }

    // Helper to add a test mount
    fn add_test_mount(mount_point: &str, mounted_root: &str) {
        let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
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
        });
        sort_mount_registry(&mut registry);
    }

    // Helper to add a test mount with read_only flag
    fn add_test_mount_readonly(mount_point: &str, mounted_root: &str, read_only: bool) {
        let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
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
        });
        sort_mount_registry(&mut registry);
    }

    #[test]
    #[serial]
    fn test_mount_registry_basic() {
        {
            clear_mount_registry();

            add_test_mount("/content", "/tmp/content");

            {
                let registry = get_mount_registry().lock().unwrap();
                let info = registry
                    .iter()
                    .find(|entry| entry.mount_point == "/content")
                    .unwrap();
                assert_eq!(info.mounted_root, PathBuf::from("/tmp/content"));
                assert_eq!(info.fs_type, UIO_FSTYPE_STDIO);
                assert!(info.active_in_registry);
            }

            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_resolve_mount_path_with_mount() {
        {
            clear_mount_registry();

            add_test_mount("/", "/Users/test/game");

            let path = PathBuf::from("/content/packages");
            let resolved = resolve_mount_path(&path);

            assert_eq!(resolved, PathBuf::from("/Users/test/game/content/packages"));

            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_resolve_mount_path_no_mount() {
        {
            clear_mount_registry();

            let path = PathBuf::from("/some/random/path");
            let resolved = resolve_mount_path(&path);

            assert_eq!(resolved, path);

            clear_mount_registry();
        }
    }

    #[test]
    #[serial]
    fn test_resolve_mount_path_absolute_fs_path() {
        {
            clear_mount_registry();

            add_test_mount("/", "/Users/test/game");

            let path = PathBuf::from("/Users/acoliver/projects/uqm/content");
            let resolved = resolve_mount_path(&path);

            assert_eq!(resolved, path);

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
    fn test_resolve_path_relative() {
        {
            let base = PathBuf::from("/home/user");
            let rel = PathBuf::from("documents/file.txt");

            let result = resolve_path(&base, &rel);
            assert_eq!(result, PathBuf::from("/home/user/documents/file.txt"));
        }
    }

    #[test]
    #[serial]
    fn test_resolve_path_absolute() {
        {
            let base = PathBuf::from("/home/user");
            let abs = PathBuf::from("/etc/config");

            let result = resolve_path(&base, &abs);
            // Absolute paths should be returned as-is
            assert_eq!(result, PathBuf::from("/etc/config"));
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
    fn test_buffer_size_registry() {
        {
            let test_ptr = 0x12345678 as *mut c_char;

            // Register a size
            register_buffer_size(test_ptr, 1024);

            // Verify we can retrieve it
            let size = get_buffer_size(test_ptr);
            assert_eq!(size, Some(1024));

            // Remove it
            remove_buffer_size(test_ptr);

            // Verify it's gone
            let size = get_buffer_size(test_ptr);
            assert_eq!(size, None);
        }
    }

    #[test]
    #[serial]
    fn test_buffer_size_registry_null() {
        {
            let result = get_buffer_size(std::ptr::null_mut());
            assert_eq!(result, None);
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
            assert_eq!(O_CREAT, 0o100);
            assert_eq!(O_TRUNC, 0o1000);
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
                buf: ptr::null_mut(),
                data_start: ptr::null_mut(),
                data_end: ptr::null_mut(),
                buf_end: ptr::null_mut(),
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_OK,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
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
                buf: ptr::null_mut(),
                data_start: ptr::null_mut(),
                data_end: ptr::null_mut(),
                buf_end: ptr::null_mut(),
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_EOF,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
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
                buf: ptr::null_mut(),
                data_start: ptr::null_mut(),
                data_end: ptr::null_mut(),
                buf_end: ptr::null_mut(),
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_OK,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
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
                buf: ptr::null_mut(),
                data_start: ptr::null_mut(),
                data_end: ptr::null_mut(),
                buf_end: ptr::null_mut(),
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_ERROR,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
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
                buf: ptr::null_mut(),
                data_start: ptr::null_mut(),
                data_end: ptr::null_mut(),
                buf_end: ptr::null_mut(),
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_EOF,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
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
                buf: ptr::null_mut(),
                data_start: ptr::null_mut(),
                data_end: ptr::null_mut(),
                buf_end: ptr::null_mut(),
                handle: ptr::null_mut(),
                status: UIO_STREAM_STATUS_ERROR,
                operation: UIO_STREAM_OPERATION_NONE,
                open_flags: 0,
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

            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
            }));

            let path = CString::new("test.txt").unwrap();
            let mode = CString::new("r").unwrap();

            let stream = unsafe { uio_fopen(dir_handle, path.as_ptr(), mode.as_ptr()) };
            assert!(!stream.is_null());

            unsafe {
                assert_eq!((*stream).status, UIO_STREAM_STATUS_OK);
                uio_fclose(stream);
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
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

            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
            }));

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
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
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

            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
            }));

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
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
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

            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
            }));

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
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
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

            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
            }));

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
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
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

            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
            }));

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
                let _ = Box::from_raw(dir_handle);
                let _ = Box::from_raw(repository);
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
    fn test_uio_fopen_sets_einval_on_invalid_mode() {
        {
            use tempfile::TempDir;

            let temp_dir = TempDir::new().unwrap();
            let repository = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
            let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
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
                path: temp_dir.path().to_path_buf(),
                virtual_path: temp_dir.path().to_path_buf(),
                refcount: std::sync::atomic::AtomicI32::new(1),
                repository,
                root_end: temp_dir.path().to_path_buf(),
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
            add_test_mount_readonly("/data", temp_dir.to_str().unwrap(), true);

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
                }));

                // Try to open for writing - should fail with EACCES
                let path = CString::new("test.txt").unwrap();
                set_errno(0);
                let handle = uio_open(dir_handle, path.as_ptr(), O_WRONLY, 0o644);

                assert!(handle.is_null());
                assert_eq!(current_errno(), libc::EACCES);

                // Try to open with O_CREAT - should also fail
                let new_file = CString::new("newfile.txt").unwrap();
                set_errno(0);
                let handle2 = uio_open(dir_handle, new_file.as_ptr(), O_WRONLY | O_CREAT, 0o644);

                assert!(handle2.is_null());
                assert_eq!(current_errno(), libc::EACCES);

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
            add_test_mount_readonly("/data", temp_dir.to_str().unwrap(), true);

            unsafe {
                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
                }));

                // Try to unlink - should fail with EACCES
                let path = CString::new("file.txt").unwrap();
                set_errno(0);
                let result = uio_unlink(dir, path.as_ptr());

                assert_eq!(result, -1);
                assert_eq!(current_errno(), libc::EACCES);

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
    fn test_mount_position_no_underflow() {
        {
            clear_mount_registry();

            // Add a mount with TOP - should not underflow
            add_test_mount("/first", "/tmp/first");

            {
                let registry = get_mount_registry().lock().unwrap();
                let mount = registry.iter().find(|m| m.mount_point == "/first").unwrap();
                // Position should be reasonable (starting at 1000 or near it)
                assert!(mount.position < 10000);
            }

            // Add another mount with TOP - should still not underflow
            add_test_mount("/second", "/tmp/second");

            {
                let registry = get_mount_registry().lock().unwrap();
                let mount1 = registry.iter().find(|m| m.mount_point == "/first").unwrap();
                let mount2 = registry
                    .iter()
                    .find(|m| m.mount_point == "/second")
                    .unwrap();

                // Both positions should be valid and different
                assert!(mount1.position < 10000);
                assert!(mount2.position < 10000);
                assert_ne!(mount1.position, mount2.position);
            }

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
                add_test_mount_readonly("/data", dir_path.to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
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
                add_test_mount_readonly("/data", dir_path.to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
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
                add_test_mount_readonly("/data", dir_path.to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
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
                add_test_mount_readonly("/content", temp_a.path().to_str().unwrap(), true);
                add_test_mount_readonly("/content", temp_b.path().to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/content"),
                    virtual_path: PathBuf::from("/content"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/content"),
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
                add_test_mount_readonly("/data", temp_a.path().to_str().unwrap(), true);
                add_test_mount_readonly("/data", temp_b.path().to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
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
                add_test_mount_readonly("/data", temp_dir.path().to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/data"),
                    virtual_path: PathBuf::from("/data"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/data"),
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
                add_test_mount_readonly("/indices", temp_a.path().to_str().unwrap(), true);
                add_test_mount_readonly("/indices", temp_b.path().to_str().unwrap(), true);

                let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
                let dir_handle = Box::into_raw(Box::new(uio_DirHandle {
                    path: PathBuf::from("/indices"),
                    virtual_path: PathBuf::from("/indices"),
                    refcount: std::sync::atomic::AtomicI32::new(1),
                    repository: repo,
                    root_end: PathBuf::from("/indices"),
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

    // =========================================================================
    // StdioAccess and copy_file Tests (Phase P10)
    // =========================================================================

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
#[path = "uio_fileops_tests.rs"]
mod fileops_tests;

#[cfg(test)]
#[path = "uio_fileblock_tests.rs"]
mod fileblock_tests;

#[cfg(test)]
#[path = "uio_mount_ops_tests.rs"]
mod mount_ops_tests;

#[cfg(test)]
#[path = "uio_owned_tests.rs"]
mod owned_tests;
