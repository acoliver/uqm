// FFI bindings for I/O module
// Provides C-compatible interface for file and directory operations

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

use crate::io::{copy_file, file_exists};

/// Convert C string to PathBuf
unsafe fn cstr_to_path(c_str: *const c_char) -> Option<PathBuf> {
    if c_str.is_null() {
        return None;
    }

    CStr::from_ptr(c_str).to_str().ok().map(PathBuf::from)
}

/// Convert C string to String
#[allow(dead_code)]
unsafe fn cstr_to_string(c_str: *const c_char) -> Option<String> {
    if c_str.is_null() {
        return None;
    }

    CStr::from_ptr(c_str).to_str().ok().map(String::from)
}

/// Write a log marker to the rust-bridge.log file
fn log_marker(marker: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("rust-bridge.log")
    {
        let _ = writeln!(file, "{}", marker);
    }
}

// Opaque representation of uio_DirHandle from C
// We only use it as a pointer, never dereference it
#[repr(C)]
pub struct uio_DirHandle {
    _private: [u8; 0],
}

// Opaque representation of uio_Stream from C (used by uio_fopen/fclose)
#[repr(C)]
pub struct uio_Stream {
    _private: [u8; 0],
}

// Opaque representation of uio_Handle from C (used by uio_open/close/read/write/fstat)
#[repr(C)]
pub struct uio_Handle {
    _private: [u8; 0],
}

// stat struct for uio_fstat
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_ino: u64,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_atime: i64,
    pub st_mtime: i64,
    pub st_ctime: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
}

// FFI bindings to C uio_* functions
extern "C" {
    // uio_fopen/uio_fclose - for fileExists2
    pub fn uio_fopen(
        dir: *mut uio_DirHandle,
        path: *const c_char,
        mode: *const c_char,
    ) -> *mut uio_Stream;
    pub fn uio_fclose(stream: *mut uio_Stream) -> c_int;

    // uio_open/uio_close - for copyFile
    pub fn uio_open(
        dir: *mut uio_DirHandle,
        path: *const c_char,
        flags: c_int,
        mode: c_int,
    ) -> *mut uio_Handle;
    pub fn uio_close(handle: *mut uio_Handle) -> c_int;

    // uio_read/uio_write - for copyFile
    pub fn uio_read(handle: *mut uio_Handle, buf: *mut u8, count: usize) -> isize;
    pub fn uio_write(handle: *mut uio_Handle, buf: *const u8, count: usize) -> isize;

    // uio_fstat - for copyFile (to get file permissions)
    pub fn uio_fstat(handle: *mut uio_Handle, stat_buf: *mut stat) -> c_int;

    // uio_unlink - for copyFile (to clean up on error)
    pub fn uio_unlink(dir: *mut uio_DirHandle, path: *const c_char) -> c_int;
}

// errno variable from C
extern "C" {
    #[link_name = "errno"]
    static mut errno: c_int;
}

// Constants for uio_open flags
const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const O_RDWR: c_int = 2;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;
const O_BINARY: c_int = 0;

// Constants for file permissions (mode)
const S_IRWXU: c_int = 0o700;
const S_IRWXG: c_int = 0o070;
const S_IRWXO: c_int = 0o007;

// EINTR for retry on interrupt
const EINTR: c_int = 4;

/// Check if a file exists (Phase 1 bridge)
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
#[no_mangle]
pub unsafe extern "C" fn fileExists(name: *const c_char) -> c_int {
    // Write to log immediately to verify we're being called
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("rust-bridge.log")
    {
        let _ = writeln!(file, "RUST_FILE_EXISTS_CALLED");
    }

    match cstr_to_path(name) {
        Some(p) => {
            if file_exists(&p) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Check if a file exists in a directory (Phase 1 bridge)
///
/// # Safety
///
/// - `dir` must be a valid pointer to uio_DirHandle or be null
/// - `file_name` must point to a valid null-terminated C string or be null
#[no_mangle]
pub unsafe extern "C" fn fileExists2(dir: *mut uio_DirHandle, file_name: *const c_char) -> c_int {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Log for debugging (only if log doesn't exist yet to avoid spam)
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("rust-bridge.log")
    {
        let _ = writeln!(file, "RUST_FILE_EXISTS2_CALLED");
    }

    // Check for null parameters
    if dir.is_null() || file_name.is_null() {
        return 0;
    }

    // Use C's uio_fopen to check if file exists (matches files.c behavior)
    let stream = uio_fopen(dir, file_name, b"rb\0".as_ptr() as *const c_char);
    if stream.is_null() {
        return 0;
    }

    // File exists, close the stream
    uio_fclose(stream);
    1
}

/// Copy a file (Phase 1 bridge)
///
/// # Safety
///
/// - `src_dir` must be a valid pointer to uio_DirHandle or be null
/// - `src_name` must point to a valid null-terminated C string or be null
/// - `dst_dir` must be a valid pointer to uio_DirHandle or be null
/// - `new_name` must point to a valid null-terminated C string or be null
#[no_mangle]
pub unsafe extern "C" fn copyFile(
    src_dir: *mut uio_DirHandle,
    src_name: *const c_char,
    dst_dir: *mut uio_DirHandle,
    new_name: *const c_char,
) -> c_int {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Log for debugging
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("rust-bridge.log")
    {
        let _ = writeln!(file, "RUST_COPY_FILE_CALLED");
    }

    // Check for null parameters
    if src_dir.is_null() || src_name.is_null() || dst_dir.is_null() || new_name.is_null() {
        return -1;
    }

    // Open source file for reading (matches files.c behavior)
    let src = uio_open(src_dir, src_name, O_RDONLY | O_BINARY, 0);
    if src.is_null() {
        return -1;
    }

    // Get file stats to preserve permissions
    let mut stat_buf: stat = std::mem::zeroed();
    if uio_fstat(src, &mut stat_buf) == -1 {
        uio_close(src);
        return -1;
    }

    // Open destination file for writing (O_EXCL ensures we don't overwrite)
    let dst = uio_open(
        dst_dir,
        new_name,
        O_WRONLY | O_CREAT | O_EXCL | O_BINARY,
        (stat_buf.st_mode as c_int) & (S_IRWXU | S_IRWXG | S_IRWXO),
    );
    if dst.is_null() {
        uio_close(src);
        return -1;
    }

    // Buffer for copying data (64KB buffer like files.c)
    const BUFSIZE: usize = 65536;
    let mut buf = vec![0u8; BUFSIZE];

    // Copy loop - handles partial writes and EINTR like files.c
    loop {
        let num_in_buf = uio_read(src, buf.as_mut_ptr(), BUFSIZE);
        if num_in_buf == -1 {
            if errno == EINTR {
                continue;
            }
            // Error reading - clean up and delete partial copy
            uio_close(src);
            uio_close(dst);
            uio_unlink(dst_dir, new_name);
            return -1;
        }
        if num_in_buf == 0 {
            break; // EOF
        }

        // Write all data, handling partial writes
        let mut remaining = num_in_buf as usize;
        let mut buf_ptr = buf.as_ptr();

        while remaining > 0 {
            let num_written = uio_write(dst, buf_ptr, remaining);
            if num_written == -1 {
                if errno == EINTR {
                    continue;
                }
                // Error writing - clean up and delete partial copy
                uio_close(src);
                uio_close(dst);
                uio_unlink(dst_dir, new_name);
                return -1;
            }
            remaining -= num_written as usize;
            buf_ptr = buf_ptr.add(num_written as usize);
        }
    }

    // Success - close handles
    uio_close(src);
    uio_close(dst);
    0
}

// Keep the existing rust_* prefixed functions for compatibility

/// Check if a file exists
///
/// # Safety
///
/// - `path` must point to a valid null-terminated C string or be null
#[no_mangle]
pub unsafe extern "C" fn rust_file_exists(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => {
            if file_exists(&p) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Copy a file
///
/// # Safety
///
/// `src_path` and `dst_path` must be valid null-terminated C strings or null.
#[no_mangle]
pub unsafe extern "C" fn rust_copy_file(src_path: *const c_char, dst_path: *const c_char) -> c_int {
    match (cstr_to_path(src_path), cstr_to_path(dst_path)) {
        (Some(src), Some(dst)) => match copy_file(&src, &dst) {
            Ok(()) => 1,
            Err(_) => 0,
        },
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::ffi::CString;
    use std::fs;
    use std::path::Path;
    use std::ptr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn get_test_dir() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut dir = env::temp_dir();
        dir.push(format!(
            "uqm_ffi_test_{:08}_{}",
            std::process::id(),
            counter
        ));
        dir
    }

    fn cleanup_test_dir(path: &Path) {
        if path.exists() {
            let _ = fs::remove_dir_all(path);
        }
    }

    #[test]
    fn test_rust_file_exists() {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir).unwrap();
        let test_file = test_dir.join("test_ffi_file.txt");

        // Should not exist initially
        let c_path = CString::new(test_file.to_str().unwrap()).unwrap();
        unsafe {
            assert_eq!(rust_file_exists(c_path.as_ptr()), 0);

            // Create file
            fs::write(&test_file, "test").unwrap();

            // Should exist now
            assert_eq!(rust_file_exists(c_path.as_ptr()), 1);
        }

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_copy_file() {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir).unwrap();
        let base_dir = &test_dir;
        let src = base_dir.join("ffi_src.txt");
        let dst = base_dir.join("ffi_dst.txt");

        // Create source file
        fs::write(&src, "test content").unwrap();

        let c_src = CString::new(src.to_str().unwrap()).unwrap();
        let c_dst = CString::new(dst.to_str().unwrap()).unwrap();

        // Copy file
        unsafe {
            assert_eq!(rust_copy_file(c_src.as_ptr(), c_dst.as_ptr()), 1);
        }
        assert!(dst.exists());

        // Cleanup
        cleanup_test_dir(&test_dir);
    }
}
