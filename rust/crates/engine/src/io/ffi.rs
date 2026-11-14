// FFI bindings for I/O module
// Provides C-compatible interface for file and directory operations

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

use crate::io::{copy_file as copy_path, file_exists as path_exists};

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

// Opaque representation of uio_DirHandle from C
// We only use it as a pointer, never dereference it
pub use crate::io::uio_bridge::uio_DirHandle;

// Opaque representation of uio_Stream from C (used by uio_fopen/fclose)
pub use crate::io::uio_bridge::uio_Stream;

// Opaque representation of uio_Handle from C (used by uio_open/close/read/write/fstat)
pub use crate::io::uio_bridge::uio_Handle;

// stat struct for uio_fstat
pub use libc::stat;

// FFI bindings to C uio_* functions

pub use crate::io::uio_bridge::uio_fopen;

pub use crate::io::uio_bridge::uio_fclose;

pub use crate::io::uio_bridge::uio_open;

pub use crate::io::uio_bridge::uio_close;

pub use crate::io::uio_bridge::uio_read;

pub use crate::io::uio_bridge::uio_write;

pub use crate::io::uio_bridge::uio_fstat;

pub use crate::io::uio_bridge::uio_unlink;

/// Check if a file exists (Phase 1 bridge)
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
pub unsafe fn file_exists(name: *const c_char) -> c_int {
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
        Some(p) if path_exists(&p) => 1,
        Some(_) | None => 0,
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
    let stream = uio_fopen(dir, file_name, c"rb".as_ptr() as *const c_char);
    if stream.is_null() {
        return 0;
    }

    // File exists, close the stream
    uio_fclose(stream);
    1
}

// Keep the existing rust_* prefixed functions for compatibility

/// Check if a file exists
///
/// # Safety
///
/// - `path` must point to a valid null-terminated C string or be null
pub unsafe fn rust_file_exists(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) if path_exists(&p) => 1,
        Some(_) | None => 0,
    }
}

/// Copy a file
///
/// # Safety
///
/// `src_path` and `dst_path` must be valid null-terminated C strings or null.
pub unsafe fn rust_copy_file(src_path: *const c_char, dst_path: *const c_char) -> c_int {
    match (cstr_to_path(src_path), cstr_to_path(dst_path)) {
        (Some(src), Some(dst)) => match copy_path(&src, &dst) {
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
