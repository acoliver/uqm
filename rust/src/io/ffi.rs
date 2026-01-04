// FFI bindings for I/O module
// Provides C-compatible interface for file and directory operations

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;

use crate::io::{
    copy_file, create_directory, create_directory_all, delete_file, directory_exists, file_exists,
    get_file_size, remove_directory, remove_directory_all, DirHandle,
};

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

/// Delete a file
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_delete_file(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => match delete_file(&p) {
            Ok(()) => 1,
            Err(_) => 0,
        },
        None => 0,
    }
}

/// Get file size
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_get_file_size(path: *const c_char) -> u64 {
    match cstr_to_path(path) {
        Some(p) => get_file_size(&p).unwrap_or(0),
        None => 0,
    }
}

/// Check if a directory exists
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_directory_exists(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => {
            if directory_exists(&p) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Create a directory
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_create_directory(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => match create_directory(&p) {
            Ok(()) => 1,
            Err(_) => 0,
        },
        None => 0,
    }
}

/// Create directory and all parent directories
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_create_directory_all(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => match create_directory_all(&p) {
            Ok(()) => 1,
            Err(_) => 0,
        },
        None => 0,
    }
}

/// Remove a directory
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_remove_directory(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => match remove_directory(&p) {
            Ok(()) => 1,
            Err(_) => 0,
        },
        None => 0,
    }
}

/// Remove a directory and all contents
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_remove_directory_all(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(p) => match remove_directory_all(&p) {
            Ok(()) => 1,
            Err(_) => 0,
        },
        None => 0,
    }
}

/// Directory handle for FFI
pub struct FFIDirHandle {
    entries: Vec<String>,
    current_index: usize,
}

impl FFIDirHandle {
    fn new(entries: Vec<String>) -> Self {
        FFIDirHandle {
            entries,
            current_index: 0,
        }
    }
}

/// Open a directory for enumeration
///
/// # Safety
///
/// `path` must be a valid null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn rust_open_directory(path: *const c_char) -> *mut FFIDirHandle {
    match cstr_to_path(path) {
        Some(p) => match DirHandle::open(&p) {
            Ok(mut handle) => {
                let mut entries = Vec::new();
                while let Some(Ok(entry)) = handle.next_entry() {
                    entries.push(entry.file_name());
                }
                Box::into_raw(Box::new(FFIDirHandle::new(entries)))
            }
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

/// Get the next entry from a directory handle
///
/// # Safety
///
/// `handle` must be a valid pointer from `rust_open_directory`. `buffer` must be a valid mutable pointer.
#[no_mangle]
pub unsafe extern "C" fn rust_read_directory_entry(
    handle: *mut FFIDirHandle,
    buffer: *mut c_char,
    buffer_size: usize,
) -> c_int {
    if handle.is_null() || buffer.is_null() || buffer_size == 0 {
        return 0;
    }

    let handle = &mut *handle;

    if handle.current_index >= handle.entries.len() {
        return 0; // No more entries
    }

    let entry = &handle.entries[handle.current_index];

    let c_string = match CString::new(entry.as_str()) {
        Ok(s) => s,
        Err(_) => {
            handle.current_index += 1;
            return 0;
        }
    };

    let bytes = c_string.as_bytes_with_nul();
    if bytes.len() > buffer_size {
        handle.current_index += 1;
        return 0;
    }

    ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, bytes.len());
    handle.current_index += 1;
    1
}

/// Close a directory handle
///
/// # Safety
///
/// `handle` must be a valid pointer from `rust_open_directory`.
#[no_mangle]
pub unsafe extern "C" fn rust_close_directory(handle: *mut FFIDirHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// Reset directory enumeration to the beginning
///
/// # Safety
///
/// `handle` must be a valid pointer from `rust_open_directory`.
#[no_mangle]
pub unsafe extern "C" fn rust_rewind_directory(handle: *mut FFIDirHandle) {
    if !handle.is_null() {
        (*handle).current_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
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

    #[test]
    fn test_rust_delete_file() {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir).unwrap();
        let test_file = test_dir.join("ffi_delete.txt");

        // Create file
        fs::write(&test_file, "test").unwrap();
        assert!(test_file.exists());
        let c_path = CString::new(test_file.to_str().unwrap()).unwrap();

        unsafe {
            // Delete file
            assert_eq!(rust_delete_file(c_path.as_ptr()), 1);
        }
        assert!(!test_file.exists());

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_get_file_size() {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir).unwrap();
        let test_file = test_dir.join("ffi_size.txt");
        let content = b"test content";

        fs::write(&test_file, content).unwrap();
        let c_path = CString::new(test_file.to_str().unwrap()).unwrap();
        let size: u64;
        unsafe {
            size = rust_get_file_size(c_path.as_ptr());
        }

        assert_eq!(size, content.len() as u64);

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_directory_exists() {
        let test_dir = get_test_dir();
        let c_path = CString::new(test_dir.to_str().unwrap()).unwrap();

        // Should not exist initially
        unsafe {
            assert_eq!(rust_directory_exists(c_path.as_ptr()), 0);

            // Create directory
            fs::create_dir(&test_dir).unwrap();
        }
        unsafe {
            assert_eq!(rust_directory_exists(c_path.as_ptr()), 1);
        }

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_create_directory() {
        let test_dir = get_test_dir();
        let c_path = CString::new(test_dir.to_str().unwrap()).unwrap();

        assert!(!test_dir.exists());

        unsafe {
            assert_eq!(rust_create_directory(c_path.as_ptr()), 1);
        }
        assert!(test_dir.exists());

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_create_directory_all() {
        let test_dir = get_test_dir();
        let nested_dir = test_dir.join("ffi_create_all").join("nested").join("path");

        let c_path = CString::new(nested_dir.to_str().unwrap()).unwrap();

        unsafe {
            assert_eq!(rust_create_directory_all(c_path.as_ptr()), 1);
        }
        assert!(nested_dir.exists());

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_remove_directory() {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir).unwrap();
        let remove_dir = test_dir.join("ffi_remove_dir");

        fs::create_dir(&remove_dir).unwrap();
        assert!(remove_dir.exists());
        let c_path = CString::new(remove_dir.to_str().unwrap()).unwrap();

        unsafe {
            assert_eq!(rust_remove_directory(c_path.as_ptr()), 1);
        }
        assert!(!remove_dir.exists());

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rust_directory_enumeration() {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir).unwrap();
        let enum_dir = test_dir.join("ffi_enum");
        fs::create_dir(&enum_dir).unwrap();

        // Create some files
        fs::write(enum_dir.join("file1.txt"), "").unwrap();
        fs::write(enum_dir.join("file2.txt"), "").unwrap();
        fs::write(enum_dir.join("file3.txt"), "").unwrap();
        let c_path = CString::new(enum_dir.to_str().unwrap()).unwrap();

        let handle = unsafe { rust_open_directory(c_path.as_ptr()) };
        assert!(!handle.is_null());

        let mut entries = Vec::new();
        let mut buffer = [0i8; 256];

        while unsafe { rust_read_directory_entry(handle, buffer.as_mut_ptr(), 256) } != 0 {
            let entry = unsafe {
                CStr::from_ptr(buffer.as_ptr())
                    .to_string_lossy()
                    .to_string()
            };
            entries.push(entry);
        }

        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&String::from("file1.txt")));
        assert!(entries.contains(&String::from("file2.txt")));
        assert!(entries.contains(&String::from("file3.txt")));

        // Test rewind
        unsafe { rust_rewind_directory(handle) };
        entries.clear();

        while unsafe { rust_read_directory_entry(handle, buffer.as_mut_ptr(), 256) } != 0 {
            let entry = unsafe {
                CStr::from_ptr(buffer.as_ptr())
                    .to_string_lossy()
                    .to_string()
            };
            entries.push(entry);
        }

        assert_eq!(entries.len(), 3);

        unsafe { rust_close_directory(handle) };

        // Cleanup
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_null_pointers() {
        unsafe {
            assert_eq!(rust_file_exists(ptr::null()), 0);
            assert_eq!(rust_copy_file(ptr::null(), ptr::null()), 0);
            assert_eq!(rust_delete_file(ptr::null()), 0);
            assert_eq!(rust_get_file_size(ptr::null()), 0);
            assert_eq!(rust_directory_exists(ptr::null()), 0);
            assert_eq!(rust_create_directory(ptr::null()), 0);
            assert_eq!(rust_create_directory_all(ptr::null()), 0);
            assert_eq!(rust_remove_directory(ptr::null()), 0);
            assert_eq!(rust_open_directory(ptr::null()).is_null(), true);
            rust_close_directory(ptr::null_mut());
            rust_rewind_directory(ptr::null_mut());
        }
    }
}
