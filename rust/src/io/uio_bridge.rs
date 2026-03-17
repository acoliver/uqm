// Minimal stdio-backed UIO implementation
// Reference: sc2/src/libs/uio/io.h, uiostream.h

use crate::bridge_log::rust_bridge_log_msg;
use libc::{mode_t, off_t, size_t};
use std::collections::HashMap;

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int, c_long};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

// =============================================================================
// Mount Point Registry
const UIO_MOUNT_RDONLY: c_int = 1 << 1;
const UIO_MOUNT_LOCATION_MASK: c_int = 3 << 2;
const UIO_MOUNT_BELOW: c_int = 2 << 2;
const UIO_MOUNT_ABOVE: c_int = 3 << 2;

const UIO_FSTYPE_STDIO: c_int = 1;
const UIO_FSTYPE_ZIP: c_int = 2;

const UIO_STREAM_STATUS_OK: c_int = 0;
const UIO_STREAM_STATUS_EOF: c_int = 1;
const UIO_STREAM_STATUS_ERROR: c_int = 2;

const UIO_STREAM_OPERATION_NONE: c_int = 0;
const UIO_STREAM_OPERATION_READ: c_int = 1;
const UIO_STREAM_OPERATION_WRITE: c_int = 2;

// =============================================================================

struct MountInfo {
    id: usize,
    repository: usize,
    handle_ptr: usize,
    mount_point: String,
    mounted_root: PathBuf,
    fs_type: c_int,
    active_in_registry: bool,
}

static MOUNT_REGISTRY: OnceLock<Mutex<Vec<MountInfo>>> = OnceLock::new();
static NEXT_MOUNT_ID: AtomicUsize = AtomicUsize::new(1);

fn get_mount_registry() -> &'static Mutex<Vec<MountInfo>> {
    MOUNT_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

// Types matching C structures from io.h and uiostream.h

#[repr(C)]
pub struct uio_DirHandle {
    path: PathBuf,
    refcount: std::sync::atomic::AtomicI32,
    repository: *mut uio_Repository,
    root_end: PathBuf,
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

#[no_mangle]
pub unsafe extern "C" fn uio_rename(
    old_dir: *mut uio_DirHandle,
    old_path: *const c_char,
    new_dir: *mut uio_DirHandle,
    new_path: *const c_char,
) -> c_int {
    log_marker("uio_rename called");

    if old_dir.is_null() || new_dir.is_null() {
        return -1;
    }

    let old_dir_path = &(*old_dir).path;
    let new_dir_path = &(*new_dir).path;

    let old_full = match cstr_to_pathbuf(old_path) {
        Some(p) => resolve_path(old_dir_path, &p),
        None => return -1,
    };

    let new_full = match cstr_to_pathbuf(new_path) {
        Some(p) => resolve_path(new_dir_path, &p),
        None => return -1,
    };

    match fs::rename(&old_full, &new_full) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_access(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    _mode: c_int,
) -> c_int {
    log_marker("uio_access called");

    if dir.is_null() {
        return -1;
    }

    let dir_path = &(*dir).path;
    let full_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => return -1,
    };

    // Simple existence check
    match full_path.exists() {
        true => 0,
        false => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_stat(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    stat_buf: *mut stat,
) -> c_int {
    log_marker("uio_stat called");

    if dir.is_null() || stat_buf.is_null() {
        return -1;
    }

    let dir_path = &(*dir).path;
    let full_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => return -1,
    };

    match fs::metadata(&full_path) {
        Ok(meta) => {
            (*stat_buf).st_size = meta.len() as i64;
            (*stat_buf).st_mode = if meta.is_file() { 0o100000 } else { 0o040000 };
            (*stat_buf).st_mode |= if meta.permissions().readonly() {
                0o444
            } else {
                0o666
            };
            0 // Success
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_mkdir(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    _mode: mode_t,
) -> c_int {
    log_marker("uio_mkdir called");

    if dir.is_null() {
        return -1;
    }

    let dir_path = &(*dir).path;
    let full_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => return -1,
    };

    match fs::create_dir(&full_path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_rmdir(dir: *mut uio_DirHandle, path: *const c_char) -> c_int {
    log_marker("uio_rmdir called");

    if dir.is_null() {
        return -1;
    }

    let dir_path = &(*dir).path;
    let full_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => return -1,
    };

    match fs::remove_dir(&full_path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_lseek(handle: *mut uio_Handle, offset: off_t, whence: c_int) -> c_int {
    log_marker("uio_lseek called");

    if handle.is_null() {
        return -1;
    }

    // handle is a Mutex<File>
    let file = &(*handle);
    let mut guard = match file.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let seek_from = match whence {
        SEEK_SET => SeekFrom::Start(offset as u64),
        SEEK_CUR => SeekFrom::Current(offset as i64),
        SEEK_END => SeekFrom::End(offset as i64),
        _ => return -1,
    };

    match guard.seek(seek_from) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// Internal structure to track allocation metadata for uio_DirList
// This mirrors the C uio_DirList struct but with additional tracking
struct uio_DirListInternal {
    names_ptr: *mut *mut c_char,
    num_names: c_int,
    buffer_ptr: *mut c_char,
    names_capacity: usize,  // Capacity of names array for proper deallocation
    buffer_capacity: usize, // Size of buffer for proper deallocation
}

// C-compatible uio_DirList struct (must match C definition exactly)
#[repr(C)]
pub struct uio_DirList {
    names: *mut *mut c_char,
    numNames: c_int,
    buffer: *mut c_char,
}

// uio_Handle is type-aliased to Mutex<File> for our implementation
pub type uio_Handle = Mutex<std::fs::File>;

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
    let raw = path.to_string_lossy();
    if raw.is_empty() || raw == "/" {
        return "/".to_string();
    }

    let trimmed = raw.trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", trimmed)
    }
}

fn normalize_virtual_path(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return PathBuf::from("/");
    }

    if path.is_absolute() {
        let mount = normalize_mount_point(path);
        return PathBuf::from(mount);
    }

    PathBuf::from(format!("/{}", path.to_string_lossy().trim_matches('/')))
}

fn register_mount(
    repository: *mut uio_Repository,
    mount_point: &Path,
    mounted_root: PathBuf,
    fs_type: c_int,
    active_in_registry: bool,
) -> *mut uio_MountHandle {
    let id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
    let handle = Box::new(uio_MountHandle {
        repository,
        id,
        fs_type,
    });
    let handle_ptr = Box::into_raw(handle);

    let info = MountInfo {
        id,
        repository: repository_key(repository),
        handle_ptr: mount_handle_key(handle_ptr),
        mount_point: normalize_mount_point(mount_point),
        mounted_root,
        fs_type,
        active_in_registry,
    };

    let mut registry = get_mount_registry().lock().unwrap();
    registry.push(info);
    sort_mount_registry(&mut registry);
    handle_ptr
}

fn sort_mount_registry(registry: &mut Vec<MountInfo>) {
    registry.sort_by(|a, b| {
        b.active_in_registry
            .cmp(&a.active_in_registry)
            .then_with(|| b.mount_point.len().cmp(&a.mount_point.len()))
            .then_with(|| b.id.cmp(&a.id))
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
    let lossy = path.to_string_lossy();
    let bytes = lossy.as_bytes();
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

fn resolve_file_location(
    dir: *mut uio_DirHandle,
    in_path: *const c_char,
) -> Option<(usize, PathBuf)> {
    if dir.is_null() {
        return None;
    }

    let dir_path = unsafe { &(*dir).path };
    let input = unsafe { cstr_to_pathbuf(in_path) }?;
    let candidate = resolve_path(dir_path, &input);
    let registry = get_mount_registry().lock().unwrap();

    if is_real_filesystem_path(&candidate) {
        return registry
            .iter()
            .filter(|entry| entry.active_in_registry)
            .filter(|entry| {
                candidate == entry.mounted_root || candidate.starts_with(&entry.mounted_root)
            })
            .max_by_key(|entry| entry.mounted_root.components().count())
            .map(|entry| (entry.handle_ptr, candidate.clone()));
    }

    resolve_virtual_mount_path(&registry, &candidate)
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

#[no_mangle]
pub unsafe extern "C" fn uio_getFileLocation(
    dir: *mut uio_DirHandle,
    inPath: *const c_char,
    _flags: c_int,
    mountHandle: *mut *mut uio_MountHandle,
    outPath: *mut *mut c_char,
) -> c_int {
    log_marker("uio_getFileLocation called");

    let Some((handle_ptr, resolved_path)) = resolve_file_location(dir, inPath) else {
        if !mountHandle.is_null() {
            *mountHandle = ptr::null_mut();
        }
        if !outPath.is_null() {
            *outPath = ptr::null_mut();
        }
        return -1;
    };

    let duplicated = duplicate_c_string(&resolved_path);
    if duplicated.is_null() {
        return -1;
    }

    if !mountHandle.is_null() {
        *mountHandle = handle_ptr as *mut uio_MountHandle;
    }
    if !outPath.is_null() {
        *outPath = duplicated;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn uio_unmountDir(mountHandle: *mut uio_MountHandle) -> c_int {
    log_marker("uio_unmountDir called");

    if mountHandle.is_null() {
        return -1;
    }

    if remove_mount_entry(mountHandle).is_none() {
        return -1;
    }

    let _ = Box::from_raw(mountHandle);
    0
}

#[no_mangle]
pub unsafe extern "C" fn uio_unmountAllDirs(repository: *mut uio_Repository) -> c_int {
    log_marker("uio_unmountAllDirs called");

    for handle_ptr in remove_repository_mounts(repository) {
        let _ = Box::from_raw(handle_ptr as *mut uio_MountHandle);
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn uio_getMountFileSystemType(mountHandle: *mut uio_MountHandle) -> c_int {
    log_marker("uio_getMountFileSystemType called");
    if mountHandle.is_null() {
        return 0;
    }

    (*mountHandle).fs_type
}

#[no_mangle]
pub unsafe extern "C" fn uio_transplantDir(
    mountPoint: *const c_char,
    sourceDir: *mut uio_DirHandle,
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    log_marker("uio_transplantDir called");

    if sourceDir.is_null() {
        return ptr::null_mut();
    }

    if (flags & UIO_MOUNT_RDONLY) != UIO_MOUNT_RDONLY {
        return ptr::null_mut();
    }

    let mount_point = match cstr_to_pathbuf(mountPoint) {
        Some(path) => path,
        None => return ptr::null_mut(),
    };

    let location = flags & UIO_MOUNT_LOCATION_MASK;
    let relative_required = location == UIO_MOUNT_ABOVE || location == UIO_MOUNT_BELOW;
    if relative_required != !relative.is_null() {
        return ptr::null_mut();
    }

    register_mount(
        (*sourceDir).repository,
        &mount_point,
        (*sourceDir).path.clone(),
        UIO_FSTYPE_STDIO,
        true,
    )
}

// =============================================================================
// uio_fgets / uio_fgetc / uio_ungetc / uio_fprintf / uio_fputc / uio_fputs
// uio_fflush / uio_feof / uio_ferror / uio_clearerr / uio_streamHandle
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn uio_fgets(
    buf: *mut c_char,
    size: c_int,
    stream: *mut uio_Stream,
) -> *mut c_char {
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
    rust_bridge_log_msg("RUST_UIO: uio_fgets entry");
    if stream.is_null() || buf.is_null() || size <= 0 {
        rust_bridge_log_msg("RUST_UIO: uio_fgets invalid args");
        return ptr::null_mut();
    }

    let max_len = size as usize;
    if max_len == 0 {
        return ptr::null_mut();
    }

    let s = &mut *stream;
    if s.handle.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fgets null handle");
        return ptr::null_mut();
    }

    let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
    if handle_ptr.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fgets null handle");
        return ptr::null_mut();
    }
    let mut guard = match (*handle_ptr).lock() {
        Ok(g) => g,
        Err(_) => {
            rust_bridge_log_msg("RUST_UIO: uio_fgets lock failed");
            return ptr::null_mut();
        }
    };

    // Safe bounds check before creating slice
    if max_len == 0 {
        return ptr::null_mut();
    }

    let buffer = slice::from_raw_parts_mut(buf as *mut u8, max_len);
    let mut count = 0usize;

    // Ensure we leave room for null terminator
    let max_read = if max_len > 0 { max_len - 1 } else { 0 };

    while count < max_read {
        let mut byte = [0u8; 1];
        let read = match guard.read(&mut byte) {
            Ok(n) => n,
            Err(_) => {
                rust_bridge_log_msg("RUST_UIO: uio_fgets read failed");
                // On EOF with some data, still return what we have
                break;
            }
        };
        if read == 0 {
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

    // Safely null-terminate (count is guaranteed < max_len here)
    if count < max_len {
        buffer[count] = 0;
    }
    buf
}

#[no_mangle]
pub unsafe extern "C" fn uio_fgetc(stream: *mut uio_Stream) -> c_int {
    rust_bridge_log_msg("RUST_UIO: uio_fgetc entry");
    if stream.is_null() {
        return -1;
    }
    let s = &mut *stream;
    if s.handle.is_null() {
        return -1;
    }
    let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
    if handle_ptr.is_null() {
        return -1;
    }
    let mut guard = match (*handle_ptr).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    let mut byte = [0u8; 1];
    match guard.read(&mut byte) {
        Ok(1) => byte[0] as c_int,
        _ => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_ungetc(c: c_int, stream: *mut uio_Stream) -> c_int {
    rust_bridge_log_msg("RUST_UIO: uio_ungetc entry");
    if stream.is_null() {
        return -1;
    }
    let s = &mut *stream;
    if s.handle.is_null() {
        return -1;
    }
    let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
    if handle_ptr.is_null() {
        return -1;
    }
    let mut guard = match (*handle_ptr).lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    if guard.seek(SeekFrom::Current(-1)).is_err() {
        return -1;
    }
    c
}

// uio_vfprintf uses va_list - we can't implement variadic functions in stable Rust
// This stub just returns error
#[no_mangle]
pub unsafe extern "C" fn uio_vfprintf(
    _stream: *mut uio_Stream,
    _format: *const c_char,
    _args: *mut libc::c_void,
) -> c_int {
    log_marker("uio_vfprintf called - stub");
    -1 // Error
}

#[no_mangle]
pub unsafe extern "C" fn uio_fputc(c: c_int, stream: *mut uio_Stream) -> c_int {
    if stream.is_null() {
        return -1;
    }

    let s = &*stream;
    if s.handle.is_null() {
        return -1;
    }

    let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
    let file_mutex = &*handle_ptr;

    let byte = c as u8;

    if let Ok(mut file) = file_mutex.lock() {
        use std::io::Write;
        match file.write_all(&[byte]) {
            Ok(()) => c, // Return the character written
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_fputs(s: *const c_char, stream: *mut uio_Stream) -> c_int {
    if stream.is_null() || s.is_null() {
        return -1;
    }

    let s_stream = &*stream;
    if s_stream.handle.is_null() {
        return -1;
    }

    let handle_ptr = s_stream.handle as *mut Mutex<std::fs::File>;
    let file_mutex = &*handle_ptr;

    let cstr = std::ffi::CStr::from_ptr(s);
    let bytes = cstr.to_bytes();

    if let Ok(mut file) = file_mutex.lock() {
        use std::io::Write;
        match file.write_all(bytes) {
            Ok(()) => 0, // Success (non-negative means success for fputs)
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_fflush(stream: *mut uio_Stream) -> c_int {
    if stream.is_null() {
        return 0; // Flushing NULL stream is a no-op success
    }

    let s = &*stream;
    if s.handle.is_null() {
        return 0;
    }

    let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
    let file_mutex = &*handle_ptr;

    if let Ok(mut file) = file_mutex.lock() {
        use std::io::Write;
        match file.flush() {
            Ok(()) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_feof(_stream: *mut uio_Stream) -> c_int {
    log_marker("uio_feof called - stub");
    1 // Always true (EOF)
}

#[no_mangle]
pub unsafe extern "C" fn uio_ferror(_stream: *mut uio_Stream) -> c_int {
    log_marker("uio_ferror called - stub");
    0 // No error
}

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

    let s = &*stream;
    if s.handle.is_null() {
        return 0;
    }

    let total_bytes = size * nmemb;
    if total_bytes == 0 {
        return nmemb; // Writing 0 items is always successful
    }

    let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
    let file_mutex = &*handle_ptr;

    let data = std::slice::from_raw_parts(ptr as *const u8, total_bytes);

    if let Ok(mut file) = file_mutex.lock() {
        use std::io::Write;
        match file.write_all(data) {
            Ok(()) => nmemb, // Return number of items written
            Err(_) => 0,
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_clearerr(_stream: *mut uio_Stream) {
    log_marker("uio_clearerr called - stub");
}

#[no_mangle]
// =============================================================================
// uio_openFileBlock / uio_closeFileBlock / uio_accessFileBlock /
// uio_copyFileBlock / uio_setFileBlockUsageHint / uio_openFileBlock2
// =============================================================================

// =============================================================================
// uio_openFileBlock / uio_closeFileBlock / uio_accessFileBlock /
// uio_copyFileBlock / uio_setFileBlockUsageHint / uio_openFileBlock2
// =============================================================================
#[repr(C)]
pub struct uio_FileBlock {
    _private: [u8; 0],
}

#[no_mangle]
pub unsafe extern "C" fn uio_openFileBlock(_handle: *mut uio_Handle) -> *mut uio_FileBlock {
    log_marker("uio_openFileBlock called - stub");
    // Return a dummy non-null pointer
    let block = Box::new(uio_FileBlock { _private: [] });
    Box::leak(block) as *mut uio_FileBlock
}

#[no_mangle]
pub unsafe extern "C" fn uio_openFileBlock2(
    _handle: *mut uio_Handle,
    _flags: c_int,
) -> *mut uio_FileBlock {
    log_marker("uio_openFileBlock2 called - stub");
    let block = Box::new(uio_FileBlock { _private: [] });
    Box::leak(block) as *mut uio_FileBlock
}

#[no_mangle]
pub unsafe extern "C" fn uio_closeFileBlock(_block: *mut uio_FileBlock) -> c_int {
    log_marker("uio_closeFileBlock called - stub");
    if !_block.is_null() {
        let _ = Box::from_raw(_block);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn uio_accessFileBlock(
    _block: *mut uio_FileBlock,
    _offset: off_t,
    _length: size_t,
    _flags: c_int,
) -> isize {
    log_marker("uio_accessFileBlock called - stub");
    -1 // Error
}

#[no_mangle]
pub unsafe extern "C" fn uio_copyFileBlock(
    _block: *mut uio_FileBlock,
    _offset: off_t,
    _buffer: *mut c_char,
    _length: size_t,
) -> c_int {
    log_marker("uio_copyFileBlock called - stub");
    -1 // Error
}

#[no_mangle]
pub unsafe extern "C" fn uio_setFileBlockUsageHint(
    _block: *mut uio_FileBlock,
    _usage: c_int,
    _flags: c_int,
) {
    log_marker("uio_setFileBlockUsageHint called - stub");
}

// =============================================================================
// uio_getFileSystemHandler / uio_gPFileFlagsFromPRootFlags / uio_walkGPPath
// =============================================================================

#[repr(C)]
pub struct uio_FileSystemHandler {
    _private: [u8; 0],
}

#[no_mangle]
pub unsafe extern "C" fn uio_getFileSystemHandler(_id: c_int) -> *mut uio_FileSystemHandler {
    log_marker("uio_getFileSystemHandler called - stub");
    // Return a dummy handler
    let handler = Box::new(uio_FileSystemHandler { _private: [] });
    Box::leak(handler) as *mut uio_FileSystemHandler
}

#[no_mangle]
pub unsafe extern "C" fn uio_gPFileFlagsFromPRootFlags(_flags: c_int) -> c_int {
    log_marker("uio_gPFileFlagsFromPRootFlags called - stub");
    _flags // Pass through unchanged
}

#[no_mangle]
pub unsafe extern "C" fn uio_walkGPPath(
    _startGPDir: *mut uio_GPDir,
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

#[no_mangle]
pub unsafe extern "C" fn uio_DirHandle_print(
    _dirHandle: *const uio_DirHandle,
    _outStream: *mut libc::FILE,
) {
    log_marker("uio_DirHandle_print called - stub");
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDirHandle_delete(_handle: *mut uio_PDirHandle) {
    log_marker("uio_GPDirHandle_delete called - stub");
    if !_handle.is_null() {
        let _ = Box::from_raw(_handle);
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_addFile(
    _gPDir: *mut uio_GPDir,
    _fileName: *const c_char,
    _file: *mut uio_GPFile,
) {
    log_marker("uio_GPDir_addFile called - stub");
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_closeEntries(_gPDir: *mut uio_GPDir) {
    log_marker("uio_GPDir_closeEntries called - stub");
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_commitSubDir(
    _gPDir: *mut uio_GPDir,
    _dirName: *const c_char,
    _subDir: *mut uio_GPDir,
    _flags: c_int,
) {
    log_marker("uio_GPDir_commitSubDir called - stub");
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_getGPDirEntry(
    _gPDir: *mut uio_GPDir,
    _name: *const c_char,
) -> *mut uio_GPDirEntry {
    log_marker("uio_GPDir_getGPDirEntry called - stub");
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_getPDirEntryHandle(
    _entry: *mut uio_GPDirEntry,
) -> *mut uio_PDirHandle {
    log_marker("uio_GPDir_getPDirEntryHandle called - stub");
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_openEntries(_gPDir: *mut uio_GPDir) -> c_int {
    log_marker("uio_GPDir_openEntries called - stub");
    -1
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_prepareSubDir(
    _gPDir: *mut uio_GPDir,
    _dirName: *const c_char,
) -> *mut uio_GPDir {
    log_marker("uio_GPDir_prepareSubDir called - stub");
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPDir_readEntries(_gPDir: *mut uio_GPDir) -> c_int {
    log_marker("uio_GPDir_readEntries called - stub");
    -1
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPFileHandle_delete(_handle: *mut uio_Handle) {
    log_marker("uio_GPFileHandle_delete called - stub");
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPFile_delete(_gPFile: *mut uio_GPFile) {
    log_marker("uio_GPFile_delete called - stub");
    if !_gPFile.is_null() {
        let _ = Box::from_raw(_gPFile);
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPFile_new(
    _pRoot: *mut uio_PRoot,
    _extra: *mut libc::c_void,
    _flags: c_int,
) -> *mut uio_GPFile {
    log_marker("uio_GPFile_new called - stub");
    let file = Box::new(uio_GPFile { _private: [] });
    Box::leak(file) as *mut uio_GPFile
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPRoot_delete(_gPRoot: *mut uio_GPRoot) {
    log_marker("uio_GPRoot_delete called - stub");
    if !_gPRoot.is_null() {
        let _ = Box::from_raw(_gPRoot);
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPRoot_makePRoot(_gPRoot: *mut uio_GPRoot) -> *mut uio_PRoot {
    log_marker("uio_GPRoot_makePRoot called - stub");
    // For simplicity, just cast (assuming compatible layout)
    _gPRoot as *mut uio_PRoot
}

#[no_mangle]
pub unsafe extern "C" fn uio_GPRoot_umount(_pRoot: *mut uio_PRoot) -> c_int {
    log_marker("uio_GPRoot_umount called - stub");
    0
}

#[no_mangle]
pub unsafe extern "C" fn uio_Handle_new(
    _root: *mut uio_PRoot,
    _native: *mut libc::c_void,
    _openFlags: c_int,
) -> *mut uio_Handle {
    log_marker("uio_Handle_new called - stub");
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn uio_PRoot_getRootDirHandle(_pRoot: *mut uio_PRoot) -> *mut uio_PDirHandle {
    log_marker("uio_PRoot_getRootDirHandle called - stub");
    let handle = Box::new(uio_PDirHandle { _private: [] });
    Box::leak(handle) as *mut uio_PDirHandle
}

#[no_mangle]
pub unsafe extern "C" fn uio_StdioAccessHandle_getPath(
    _handle: *mut uio_StdioAccessHandle,
) -> *const c_char {
    log_marker("uio_StdioAccessHandle_getPath called - stub");
    ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn uio_getStdioAccess(
    _dir: *mut uio_DirHandle,
    _path: *const c_char,
    _flags: c_int,
) -> *mut uio_StdioAccessHandle {
    log_marker("uio_getStdioAccess called - stub");
    let handle = Box::new(uio_StdioAccessHandle { _private: [] });
    Box::leak(handle) as *mut uio_StdioAccessHandle
}

#[no_mangle]
pub unsafe extern "C" fn uio_releaseStdioAccess(_handle: *mut uio_StdioAccessHandle) {
    log_marker("uio_releaseStdioAccess called - stub");
    if !_handle.is_null() {
        let _ = Box::from_raw(_handle);
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_printMounts(
    _outStream: *mut libc::FILE,
    _repository: *const uio_Repository,
) {
    log_marker("uio_printMounts called - stub");
}

#[no_mangle]
pub unsafe extern "C" fn uio_streamHandle(stream: *mut uio_Stream) -> *mut uio_Handle {
    log_marker("uio_streamHandle called");
    if stream.is_null() {
        return ptr::null_mut();
    }
    (*stream).handle
}

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
#[no_mangle]
pub unsafe extern "C" fn rust_bridge_log_msg_c(message: *const c_char) -> c_int {
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

// Helper: Convert path to absolute path if relative
fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

const MATCH_LITERAL: c_int = 0;
const MATCH_PREFIX: c_int = 1;
const MATCH_SUFFIX: c_int = 2;
const MATCH_SUBSTRING: c_int = 3;
const MATCH_REGEX: c_int = 4;
const MATCH_REGEX_ALT: c_int = 5;

fn matches_pattern(name: &str, pattern: &str, match_type: c_int) -> bool {
    if pattern.is_empty() {
        return true;
    }

    let lower = name.to_ascii_lowercase();

    let matches_regex = |pat: &str| {
        if pat == r"\.[rR][mM][pP]$" {
            lower.ends_with(".rmp")
        } else if pat == r"\.([zZ][iI][pP]|[uU][qQ][mM])$" {
            lower.ends_with(".zip") || lower.ends_with(".uqm")
        } else if pat.contains("[rR][mM][pP]") {
            lower.ends_with(".rmp")
        } else if pat.contains("[zZ][iI][pP]") || pat.contains("[uU][qQ][mM]") {
            lower.ends_with(".zip") || lower.ends_with(".uqm")
        } else {
            lower.contains(&pat.to_ascii_lowercase())
        }
    };

    match match_type {
        MATCH_LITERAL => name == pattern,
        MATCH_PREFIX => name.starts_with(pattern),
        MATCH_SUFFIX => name.ends_with(pattern),
        MATCH_SUBSTRING => name.contains(pattern),
        MATCH_REGEX | MATCH_REGEX_ALT => matches_regex(pattern),
        _ => matches_regex(pattern),
    }
}

// =============================================================================
// uio_init / uio_unInit / uio_openRepository / uio_closeRepository
// =============================================================================

// uio_init / uio_unInit / uio_openRepository / uio_closeRepository
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn uio_init() {
    log_marker("uio_init called");
}

#[no_mangle]
pub unsafe extern "C" fn uio_unInit() {
    log_marker("uio_unInit called");
}

#[no_mangle]
pub unsafe extern "C" fn uio_openRepository(flags: c_int) -> *mut uio_Repository {
    log_marker("uio_openRepository called");
    let repo = Box::new(uio_Repository { flags });
    Box::leak(repo) as *mut uio_Repository
}

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
    eprintln!("RUST_UIO: uio_openDir called with path: {:?}", c_path);

    // Resolve through mount registry
    let resolved = resolve_mount_path(&c_path);

    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_openDir resolved to: {:?}",
        resolved
    ));
    eprintln!("RUST_UIO: uio_openDir resolved to: {:?}", resolved);

    // Create directory handle (don't fail if it doesn't exist - may be created later)
    let handle = Box::new(uio_DirHandle {
        path: resolved.clone(),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: _repository,
        root_end: resolved.clone(), // For now, root_end = full path
    });
    Box::leak(handle) as *mut uio_DirHandle
}

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

#[no_mangle]
pub unsafe extern "C" fn uio_mountDir(
    _destRep: *mut uio_Repository,
    mountPoint: *const c_char,
    _fsType: c_int,
    sourceDir: *mut uio_DirHandle,
    sourcePath: *const c_char,

    inPath: *const c_char,
    _autoMount: *mut *mut (),
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    let mount_point = match cstr_to_pathbuf(mountPoint) {
        Some(p) => p,
        None => {
            rust_bridge_log_msg("RUST_UIO: uio_mountDir: null mountPoint");
            return ptr::null_mut();
        }
    };

    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_mountDir called: mountPoint={:?}, inPath ptr={:?}, sourceDir={:?}, flags={}, relative={:?}",
        mount_point, inPath, sourceDir, flags, relative
    ));

    if !sourceDir.is_null() {
        let base_path = (*sourceDir).path.clone();
        let rel_path = cstr_to_pathbuf(sourcePath).unwrap_or_default();
        let mounted_root = if rel_path.as_os_str().is_empty() || rel_path == Path::new("/") {
            base_path.clone()
        } else {
            resolve_path(&base_path, &rel_path)
        };
        rust_bridge_log_msg(&format!(
            "RUST_UIO: uio_mountDir: sourceDir set, sourcePath {:?} -> {:?}",
            rel_path, mounted_root
        ));

        let active_in_registry = _fsType != UIO_FSTYPE_ZIP;
        return register_mount(
            _destRep,
            &mount_point,
            mounted_root,
            _fsType,
            active_in_registry,
        );
    }

    let mounted_root = if !inPath.is_null() {
        match cstr_to_pathbuf(inPath) {
            Some(path) => {
                rust_bridge_log_msg(&format!(
                    "RUST_UIO: uio_mountDir: using inPath {:?} as source",
                    path
                ));
                path
            }
            None => {
                rust_bridge_log_msg(
                    "RUST_UIO: uio_mountDir: inPath conversion failed, using empty path",
                );
                PathBuf::new()
            }
        }
    } else {
        rust_bridge_log_msg("RUST_UIO: uio_mountDir: inPath is NULL, using empty path");
        PathBuf::new()
    };

    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_mountDir: mounting root={:?} at {:?}",
        mounted_root, mount_point
    ));

    register_mount(_destRep, &mount_point, mounted_root, _fsType, true)
}

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

    let handle = Box::new(uio_DirHandle {
        path: resolved.clone(),
        refcount: std::sync::atomic::AtomicI32::new(1),
        repository: (*base).repository,
        root_end: (*base).root_end.clone(),
    });
    Box::leak(handle) as *mut uio_DirHandle
}

// =============================================================================
// uio_open / uio_close / uio_read / uio_write / uio_fstat
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn uio_open(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    flags: c_int,
    _mode: c_int,
) -> *mut uio_Handle {
    rust_bridge_log_msg(&format!("RUST_UIO: uio_open called with flags {}", flags));

    if dir.is_null() {
        return ptr::null_mut();
    }

    let dir_path = &(*dir).path;
    let file_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => return ptr::null_mut(),
    };

    let mut opts = OpenOptions::new();

    match flags & 3 {
        O_RDONLY => {
            opts.read(true);
        }
        O_WRONLY => {
            opts.write(true);
        }
        O_RDWR => {
            opts.read(true).write(true);
        }
        _ => {
            opts.read(true);
        }
    }

    if (flags & O_CREAT) != 0 {
        opts.create(true);
    }
    if (flags & O_TRUNC) != 0 {
        opts.truncate(true);
    }

    let file = match opts.open(&file_path) {
        Ok(f) => f,
        Err(err) => {
            log_marker(&format!(
                "uio_open failed: path={:?} err={}",
                file_path, err
            ));
            return ptr::null_mut();
        }
    };

    // Return Mutex<File> directly as uio_Handle (type-aliased)
    Box::leak(Box::new(Mutex::new(file))) as *mut uio_Handle
}

#[no_mangle]
pub unsafe extern "C" fn uio_close(handle: *mut uio_Handle) -> c_int {
    log_marker("uio_close called");
    if !handle.is_null() {
        // handle is a Mutex<File>
        let _ = Box::from_raw(handle);
    }
    0 // Success
}

#[no_mangle]
pub unsafe extern "C" fn uio_read(handle: *mut uio_Handle, buf: *mut u8, count: size_t) -> isize {
    if handle.is_null() || buf.is_null() || count == 0 {
        return -1;
    }

    // handle is a Mutex<File>
    let file = &(*handle);
    let mut guard = match file.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let buffer = slice::from_raw_parts_mut(buf, count);
    match guard.read(buffer) {
        Ok(n) => n as isize,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_write(
    handle: *mut uio_Handle,
    buf: *const u8,
    count: size_t,
) -> isize {
    if handle.is_null() || buf.is_null() || count == 0 {
        return -1;
    }

    // handle is a Mutex<File>
    let file = &(*handle);
    let mut guard = match file.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let buffer = slice::from_raw_parts(buf, count);
    match guard.write_all(buffer) {
        Ok(_) => count as isize,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_fstat(handle: *mut uio_Handle, stat_buf: *mut stat) -> c_int {
    if handle.is_null() || stat_buf.is_null() {
        return -1;
    }

    // handle is a Mutex<File>
    let file = &(*handle);
    let guard = match file.lock() {
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

#[no_mangle]
pub unsafe extern "C" fn uio_unlink(dir: *mut uio_DirHandle, path: *const c_char) -> c_int {
    log_marker("uio_unlink called");

    if dir.is_null() {
        return -1;
    }

    let dir_path = &(*dir).path;
    let file_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => return -1,
    };

    match fs::remove_file(&file_path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// =============================================================================
// uio_fopen / uio_fclose / uio_fread / uio_fseek / uio_ftell
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn uio_fopen(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    mode: *const c_char,
) -> *mut uio_Stream {
    rust_bridge_log_msg("RUST_UIO: uio_fopen entry");
    log_marker("uio_fopen called");

    if dir.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fopen null dir");
        return ptr::null_mut();
    }
    if mode.is_null() {
        rust_bridge_log_msg("RUST_UIO: uio_fopen null mode");
        return ptr::null_mut();
    }

    let dir_path = &(*dir).path;
    let file_path = match cstr_to_pathbuf(path) {
        Some(p) => resolve_path(dir_path, &p),
        None => {
            rust_bridge_log_msg("RUST_UIO: uio_fopen null path");
            return ptr::null_mut();
        }
    };

    let mode_str = std::ffi::CStr::from_ptr(mode).to_string_lossy();
    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_fopen path={:?} mode={}",
        file_path, mode_str
    ));

    let mut opts = OpenOptions::new();
    let mut open_flags = 0i32;

    if mode_str.contains("r") {
        opts.read(true);
        open_flags = O_RDONLY;
    }
    if mode_str.contains("w") {
        opts.write(true).create(true).truncate(true);
        open_flags = O_WRONLY | O_CREAT | O_TRUNC;
    }
    if mode_str.contains("a") {
        opts.append(true).create(true);
    }
    if mode_str.contains("+") {
        opts.read(true).write(true);
    }

    let file = match opts.open(&file_path) {
        Ok(f) => f,
        Err(err) => {
            log_marker(&format!(
                "uio_open failed: path={:?} err={}",
                file_path, err
            ));
            return ptr::null_mut();
        }
    };

    let stream = Box::new(uio_Stream {
        buf: ptr::null_mut(),
        data_start: ptr::null_mut(),
        data_end: ptr::null_mut(),
        buf_end: ptr::null_mut(),
        handle: Box::leak(Box::new(Mutex::new(file))) as *mut Mutex<std::fs::File>
            as *mut uio_Handle,
        status: 0,    // uio_Stream_STATUS_OK
        operation: 0, // uio_StreamOperation_none
        open_flags: open_flags,
    });
    let stream_ptr = Box::leak(stream) as *mut uio_Stream;
    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_fopen returning stream={:?}",
        stream_ptr
    ));
    stream_ptr
}

#[no_mangle]
pub unsafe extern "C" fn uio_fclose(stream: *mut uio_Stream) -> c_int {
    log_marker("uio_fclose called");
    if !stream.is_null() {
        let s = &*stream;
        // Free buffer if allocated
        if !s.buf.is_null() {
            // TODO: Need buffer size to deallocate properly
            // For now, we leak it
        }
        if !s.handle.is_null() {
            // Reconstruct Box<Mutex<File>> from raw pointer
            let handle_ptr = s.handle as *mut Mutex<std::fs::File>;
            let _ = Box::from_raw(handle_ptr);
        }
        let _ = Box::from_raw(stream);
    }
    0 // Success
}
#[no_mangle]
pub unsafe extern "C" fn rust_uio_fread(
    buf: *mut libc::c_void,
    size: size_t,
    nmemb: size_t,
    stream: *mut uio_Stream,
) -> size_t {
    rust_bridge_log_msg(&format!(
        "RUST_UIO: uio_fread entry stream={:?} buf={:?} size={} nmemb={} (raw size={:#x} nmemb={:#x})",
        stream, buf, size, nmemb, size as usize, nmemb as usize
    ));
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
            s.operation = 1; // uio_StreamOperation_read
            if n < total_bytes {
                s.status = 1; // uio_Stream_STATUS_EOF
            }
            rust_bridge_log_msg(&format!(
                "RUST_UIO: uio_fread requested={} read={}",
                total_bytes, n
            ));
            n / size
        }
        Err(err) => {
            rust_bridge_log_msg(&format!("RUST_UIO: uio_fread error: {}", err));
            s.status = 2; // uio_Stream_STATUS_ERROR
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_fseek(
    stream: *mut uio_Stream,
    offset: c_long,
    whence: c_int,
) -> c_int {
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
        SEEK_CUR => SeekFrom::Current(offset as i64),
        SEEK_END => SeekFrom::End(offset as i64),
        _ => return -1,
    };

    match guard.seek(seek_from) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn uio_ftell(stream: *mut uio_Stream) -> c_long {
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

#[no_mangle]
pub unsafe extern "C" fn uio_getDirList(
    dir: *mut uio_DirHandle,
    path: *const c_char,
    _pattern: *const c_char,
    _matchType: c_int,
) -> *mut uio_DirList {
    log_marker(&format!(
        "uio_getDirList called: dir=0x{:x} path=0x{:x} pattern=0x{:x}",
        dir as usize, path as usize, _pattern as usize
    ));

    if dir.is_null() {
        log_marker("uio_getDirList: null dir handle");
        return ptr::null_mut();
    }

    let dir_path = &(*dir).path;
    let rel_path = match cstr_to_pathbuf(path) {
        Some(p) => p,
        None => {
            log_marker("uio_getDirList: null path string");
            return ptr::null_mut();
        }
    };
    log_marker(&format!(
        "uio_getDirList: dir_path={:?} rel_path={:?}",
        dir_path, rel_path
    ));
    let list_path = resolve_path(dir_path, &rel_path);

    let pattern_str = if _pattern.is_null() {
        ""
    } else {
        match std::ffi::CStr::from_ptr(_pattern).to_str() {
            Ok(s) => s,
            Err(_) => "",
        }
    };
    log_marker(&format!("uio_getDirList: pattern_str='{}'", pattern_str));

    log_marker(&format!(
        "uio_getDirList: list_path={:?} pattern='{}' matchType={}",
        list_path, pattern_str, _matchType
    ));

    let entries = match fs::read_dir(&list_path) {
        Ok(e) => e,
        Err(err) => {
            log_marker(&format!(
                "uio_getDirList: read_dir failed for {:?}: {}",
                list_path, err
            ));
            return ptr::null_mut();
        }
    };

    // Collect all names into a vector first
    let mut name_strings: Vec<String> = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(name_osstr) = entry.file_name().to_str() {
                if matches_pattern(name_osstr, pattern_str, _matchType) {
                    name_strings.push(name_osstr.to_string());
                }
            }
        }
    }

    if name_strings.is_empty() {
        log_marker(&format!(
            "uio_getDirList: no matches for pattern '{}' in {:?}",
            pattern_str, list_path
        ));
        // Return empty DirList - allocate a zeroed struct
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
        list_path
    ));

    // Allocate a single contiguous buffer for all strings
    let total_size: usize = name_strings.iter().map(|s| s.len() + 1).sum();
    let buffer_layout = std::alloc::Layout::from_size_align(total_size, 1).unwrap();
    let buffer_ptr = std::alloc::alloc(buffer_layout) as *mut c_char;
    if buffer_ptr.is_null() {
        return ptr::null_mut();
    }

    // Register the buffer size for later deallocation
    register_buffer_size(buffer_ptr, total_size);

    // Allocate array of pointers using Vec for capacity tracking
    let num_names = name_strings.len();
    let mut names_vec: Vec<*mut c_char> = Vec::with_capacity(num_names);

    // Copy strings into buffer and collect pointers
    let mut offset = 0;
    for (i, name) in name_strings.iter().enumerate() {
        let name_bytes = name.as_bytes();
        let dst = buffer_ptr.add(offset);

        // Copy string bytes including null terminator
        std::ptr::copy_nonoverlapping(name_bytes.as_ptr() as *const c_char, dst, name_bytes.len());
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
}

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
                std::alloc::dealloc(list.buffer as *mut u8, buffer_layout);
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
        let handle = Box::into_raw(Box::new(uio_MountHandle {
            repository,
            id: NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst),
            fs_type: UIO_FSTYPE_STDIO,
        }));

        let mut registry = get_mount_registry().lock().unwrap();
        registry.push(MountInfo {
            id: unsafe { (*handle).id },
            repository: repository_key(repository),
            handle_ptr: handle as usize,
            mount_point: normalize_mount_point(Path::new(mount_point)),
            mounted_root: PathBuf::from(mounted_root),
            fs_type: UIO_FSTYPE_STDIO,
            active_in_registry: true,
        });
        sort_mount_registry(&mut registry);
    }

    #[test]
    #[serial]
    fn test_mount_registry_basic() {
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

    #[test]
    #[serial]
    fn test_resolve_mount_path_with_mount() {
        clear_mount_registry();

        add_test_mount("/", "/Users/test/game");

        let path = PathBuf::from("/content/packages");
        let resolved = resolve_mount_path(&path);

        assert_eq!(resolved, PathBuf::from("/Users/test/game/content/packages"));

        clear_mount_registry();
    }

    #[test]
    #[serial]
    fn test_resolve_mount_path_no_mount() {
        clear_mount_registry();

        let path = PathBuf::from("/some/random/path");
        let resolved = resolve_mount_path(&path);

        assert_eq!(resolved, path);

        clear_mount_registry();
    }

    #[test]
    #[serial]
    fn test_resolve_mount_path_absolute_fs_path() {
        clear_mount_registry();

        add_test_mount("/", "/Users/test/game");

        let path = PathBuf::from("/Users/acoliver/projects/uqm/content");
        let resolved = resolve_mount_path(&path);

        assert_eq!(resolved, path);

        clear_mount_registry();
    }

    #[test]
    #[serial]
    fn test_cstr_to_pathbuf_valid() {
        let test_path = CString::new("/test/path").unwrap();
        let result = unsafe { cstr_to_pathbuf(test_path.as_ptr()) };

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PathBuf::from("/test/path"));
    }

    #[test]
    #[serial]
    fn test_cstr_to_pathbuf_null() {
        let result = unsafe { cstr_to_pathbuf(std::ptr::null()) };
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_resolve_path_relative() {
        let base = PathBuf::from("/home/user");
        let rel = PathBuf::from("documents/file.txt");

        let result = resolve_path(&base, &rel);
        assert_eq!(result, PathBuf::from("/home/user/documents/file.txt"));
    }

    #[test]
    #[serial]
    fn test_resolve_path_absolute() {
        let base = PathBuf::from("/home/user");
        let abs = PathBuf::from("/etc/config");

        let result = resolve_path(&base, &abs);
        // Absolute paths should be returned as-is
        assert_eq!(result, PathBuf::from("/etc/config"));
    }

    #[test]
    #[serial]
    fn test_matches_pattern_literal() {
        assert!(matches_pattern("test.txt", "test.txt", MATCH_LITERAL));
        assert!(!matches_pattern("test.txt", "other.txt", MATCH_LITERAL));
        assert!(!matches_pattern("test.txt", "TEST.TXT", MATCH_LITERAL)); // Case-sensitive
    }

    #[test]
    #[serial]
    fn test_matches_pattern_prefix() {
        assert!(matches_pattern("test.txt", "test", MATCH_PREFIX));
        assert!(!matches_pattern("test.txt", "txt", MATCH_PREFIX));
    }

    #[test]
    #[serial]
    fn test_matches_pattern_suffix() {
        assert!(matches_pattern("test.txt", ".txt", MATCH_SUFFIX));
        assert!(!matches_pattern("test.txt", ".doc", MATCH_SUFFIX));
    }

    #[test]
    #[serial]
    fn test_matches_pattern_substring() {
        assert!(matches_pattern("mytest.txt", "test", MATCH_SUBSTRING));
        assert!(!matches_pattern("mytest.txt", "foo", MATCH_SUBSTRING));
    }

    #[test]
    #[serial]
    fn test_matches_pattern_regex_rmp() {
        // Test the .rmp regex pattern
        assert!(matches_pattern("file.rmp", r"\.[rR][mM][pP]$", MATCH_REGEX));
        assert!(matches_pattern("file.RMP", r"\.[rR][mM][pP]$", MATCH_REGEX));
        assert!(!matches_pattern(
            "file.txt",
            r"\.[rR][mM][pP]$",
            MATCH_REGEX
        ));
    }

    #[test]
    #[serial]
    fn test_matches_pattern_regex_zip_uqm() {
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

    #[test]
    #[serial]
    fn test_matches_pattern_empty_pattern() {
        // Empty pattern should match everything
        assert!(matches_pattern("anything.txt", "", MATCH_LITERAL));
        assert!(matches_pattern("anything.txt", "", MATCH_REGEX));
    }

    #[test]
    #[serial]
    fn test_buffer_size_registry() {
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

    #[test]
    #[serial]
    fn test_buffer_size_registry_null() {
        let result = get_buffer_size(std::ptr::null_mut());
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_seek_constants() {
        // Verify our seek constants match expected values
        assert_eq!(SEEK_SET, 0);
        assert_eq!(SEEK_CUR, 1);
        assert_eq!(SEEK_END, 2);
    }

    #[test]
    #[serial]
    fn test_open_flags_constants() {
        // Verify file open flags
        assert_eq!(O_RDONLY, 0);
        assert_eq!(O_WRONLY, 1);
        assert_eq!(O_RDWR, 2);
        assert_eq!(O_CREAT, 0o100);
        assert_eq!(O_TRUNC, 0o1000);
    }
}
