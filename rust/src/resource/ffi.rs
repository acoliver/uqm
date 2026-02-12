// FFI bindings for Resource module
// Provides C-compatible interface for resource loading

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use super::cache::ResourceCache;
use super::index::ResourceIndex;
use super::loader::ResourceLoader;
use super::resource_system::ResourceSystem;

/// Global resource system
static GLOBAL_RESOURCE_SYSTEM: Mutex<Option<ResourceSystem>> = Mutex::new(None);

/// Global resource loader (initialized once, thread-safe)
static GLOBAL_RESOURCE_LOADER: OnceLock<Mutex<Option<ResourceLoader>>> = OnceLock::new();

/// Global resource cache (initialized once, thread-safe)
static GLOBAL_RESOURCE_CACHE: OnceLock<ResourceCache> = OnceLock::new();

/// Get the global resource loader mutex
fn get_loader_mutex() -> &'static Mutex<Option<ResourceLoader>> {
    GLOBAL_RESOURCE_LOADER.get_or_init(|| Mutex::new(None))
}

/// Initialize the global resource system
///
/// # Safety
///
/// - `base_path` must point to a valid null-terminated C string or be null
/// - The memory referenced by `base_path` must not be modified for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_init_resource_system(base_path: *const c_char) -> c_int {
    match cstr_to_path(base_path) {
        Some(path) => {
            let mut system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
            *system = Some(ResourceSystem::new(path));
            1
        }
        None => 0,
    }
}

/// Load an index file
///
/// # Safety
///
/// - `path` must point to a valid null-terminated C string or be null
/// - The memory referenced by `path` must not be modified for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_load_index(path: *const c_char) -> c_int {
    match cstr_to_path(path) {
        Some(path_buf) => {
            let mut system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
            if let Some(sys) = system.as_mut() {
                match sys.load_index(&path_buf) {
                    Ok(()) => 1,
                    Err(_) => 0,
                }
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Get a string resource
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - The memory referenced by `name` must not be modified for the duration of this call
/// - Returns ownership of a newly allocated string that must be freed by the caller
#[no_mangle]
pub unsafe extern "C" fn rust_get_string_resource(name: *const c_char) -> *mut c_char {
    match cstr_to_string(name) {
        Some(name_str) => {
            let mut system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
            if let Some(sys) = system.as_mut() {
                match sys.get_string(&name_str) {
                    Ok(val) => CString::new(val.as_str())
                        .map(|s| s.into_raw())
                        .unwrap_or(ptr::null_mut()),
                    Err(_) => ptr::null_mut(),
                }
            } else {
                ptr::null_mut()
            }
        }
        None => ptr::null_mut(),
    }
}

/// Get an integer resource
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - The memory referenced by `name` must not be modified for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_int_resource(name: *const c_char) -> i32 {
    match cstr_to_string(name) {
        Some(name_str) => {
            let mut system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
            if let Some(sys) = system.as_mut() {
                sys.get_int(&name_str).unwrap_or(0)
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Get a boolean resource
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - The memory referenced by `name` must not be modified for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_bool_resource(name: *const c_char) -> c_int {
    match cstr_to_string(name) {
        Some(name_str) => {
            let mut system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
            if let Some(sys) = system.as_mut() {
                if sys.get_bool(&name_str).unwrap_or(false) {
                    1
                } else {
                    0
                }
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Enable or disable the resource system
#[no_mangle]
pub extern "C" fn rust_set_resources_enabled(enabled: c_int) {
    let mut system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
    if let Some(sys) = system.as_mut() {
        sys.set_enabled(enabled != 0);
    }
}

/// Check if resources are enabled
#[no_mangle]
pub extern "C" fn rust_resources_enabled() -> c_int {
    let system = GLOBAL_RESOURCE_SYSTEM.lock().unwrap();
    if let Some(sys) = system.as_ref() {
        if sys.is_enabled() {
            1
        } else {
            0
        }
    } else {
        0
    }
}

// Helper functions

unsafe fn cstr_to_path(c_str: *const c_char) -> Option<std::path::PathBuf> {
    if c_str.is_null() {
        return None;
    }

    CStr::from_ptr(c_str)
        .to_str()
        .ok()
        .map(std::path::PathBuf::from)
}

unsafe fn cstr_to_string(c_str: *const c_char) -> Option<String> {
    if c_str.is_null() {
        return None;
    }

    CStr::from_ptr(c_str).to_str().ok().map(String::from)
}

/// Free a string allocated by Rust
///
/// # Safety
///
/// - `s` must be either null or a pointer previously returned from Rust
/// - If non-null, `s` must point to memory allocated by Rust and not already freed
#[no_mangle]
pub unsafe extern "C" fn rust_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

// =============================================================================
// Resource Loading FFI
// =============================================================================

/// Initialize the global resource loader with the given base path and index file
///
/// # Safety
///
/// - `base_path` must point to a valid null-terminated C string or be null
/// - `index_path` must point to a valid null-terminated C string or be null
/// - Both strings must not be modified for the duration of this call
///
/// # Returns
/// 1 on success, 0 on failure
#[no_mangle]
pub unsafe extern "C" fn rust_resource_loader_init(
    base_path: *const c_char,
    index_path: *const c_char,
) -> c_int {
    let base = match cstr_to_path(base_path) {
        Some(p) => p,
        None => return 0,
    };

    let idx_path = match cstr_to_path(index_path) {
        Some(p) => p,
        None => return 0,
    };

    // Load the resource index from file
    let index = match ResourceIndex::from_file(&idx_path) {
        Ok(idx) => idx,
        Err(_) => return 0,
    };

    // Create the loader
    let loader = ResourceLoader::new(base, index);

    // Store in global
    let mut guard = get_loader_mutex().lock().unwrap();
    *guard = Some(loader);
    1
}

/// Load a resource as raw bytes
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - `out_size` must point to a valid usize or be null
/// - The returned pointer must be freed with `rust_resource_free`
///
/// # Returns
/// Pointer to the loaded bytes, or null on failure.
/// The size is written to `out_size` if non-null.
#[no_mangle]
pub unsafe extern "C" fn rust_resource_load(
    name: *const c_char,
    out_size: *mut usize,
) -> *mut u8 {
    let resource_name = match cstr_to_string(name) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let guard = get_loader_mutex().lock().unwrap();
    let loader = match guard.as_ref() {
        Some(l) => l,
        None => return ptr::null_mut(),
    };

    match loader.load(&resource_name) {
        Ok(data) => {
            let size = data.len();
            if !out_size.is_null() {
                *out_size = size;
            }

            // Allocate and copy data to a new buffer that C can manage
            let ptr = libc::malloc(size) as *mut u8;
            if ptr.is_null() {
                return ptr::null_mut();
            }
            ptr::copy_nonoverlapping(data.as_ptr(), ptr, size);
            ptr
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Load a resource as a null-terminated C string
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - The returned pointer must be freed with `rust_free_string`
///
/// # Returns
/// Pointer to the loaded string, or null on failure (including non-UTF8 content)
#[no_mangle]
pub unsafe extern "C" fn rust_resource_load_string(name: *const c_char) -> *mut c_char {
    let resource_name = match cstr_to_string(name) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let guard = get_loader_mutex().lock().unwrap();
    let loader = match guard.as_ref() {
        Some(l) => l,
        None => return ptr::null_mut(),
    };

    match loader.load_string(&resource_name) {
        Ok(content) => CString::new(content)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        Err(_) => ptr::null_mut(),
    }
}

/// Check if a resource exists in the index
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
///
/// # Returns
/// 1 if the resource exists, 0 otherwise
#[no_mangle]
pub unsafe extern "C" fn rust_resource_exists(name: *const c_char) -> c_int {
    let resource_name = match cstr_to_string(name) {
        Some(s) => s,
        None => return 0,
    };

    let guard = get_loader_mutex().lock().unwrap();
    if let Some(loader) = guard.as_ref() {
        if loader.exists(&resource_name) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// Free memory allocated by rust_resource_load
///
/// # Safety
///
/// - `ptr` must be either null or a pointer returned from `rust_resource_load`
/// - `size` must be the size that was returned in `out_size` from `rust_resource_load`
/// - The pointer must not have been freed already
#[no_mangle]
pub unsafe extern "C" fn rust_resource_free(ptr: *mut u8, _size: usize) {
    if !ptr.is_null() {
        libc::free(ptr as *mut libc::c_void);
    }
}

// =============================================================================
// Cache FFI
// =============================================================================

/// Initialize the global resource cache with the given maximum size
///
/// Note: The cache can only be initialized once. Subsequent calls will fail.
///
/// # Arguments
/// * `max_size_bytes` - Maximum cache size in bytes
///
/// # Returns
/// 1 on success, 0 if already initialized
#[no_mangle]
pub extern "C" fn rust_cache_init(max_size_bytes: usize) -> c_int {
    // Try to set the cache - returns Err if already set
    if GLOBAL_RESOURCE_CACHE
        .set(ResourceCache::new(max_size_bytes))
        .is_ok()
    {
        1
    } else {
        0
    }
}

/// Get a resource from the cache
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - `out_size` must point to a valid usize or be null
///
/// # Returns
/// Pointer to a copy of the cached data, or null if not found.
/// The size is written to `out_size` if non-null.
/// The returned pointer must be freed with `rust_resource_free`.
#[no_mangle]
pub unsafe extern "C" fn rust_cache_get(
    name: *const c_char,
    out_size: *mut usize,
) -> *mut u8 {
    let key = match cstr_to_string(name) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let cache = match GLOBAL_RESOURCE_CACHE.get() {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    match cache.get(&key) {
        Some(resource) => {
            let size = resource.size;
            if !out_size.is_null() {
                *out_size = size;
            }

            // Handle empty data case
            if size == 0 {
                // Return a non-null but zero-size allocation
                // This matches the semantics: "found in cache" vs "not found"
                let ptr = libc::malloc(1) as *mut u8;
                return ptr;
            }

            // Allocate and copy data
            let ptr = libc::malloc(size) as *mut u8;
            if ptr.is_null() {
                return ptr::null_mut();
            }
            ptr::copy_nonoverlapping(resource.data.as_ptr(), ptr, size);
            ptr
        }
        None => ptr::null_mut(),
    }
}

/// Insert a resource into the cache
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
/// - `data` must point to `size` bytes of valid memory or be null (with size=0)
/// - The data is copied into the cache
#[no_mangle]
pub unsafe extern "C" fn rust_cache_insert(
    name: *const c_char,
    data: *const u8,
    size: usize,
) {
    let key = match cstr_to_string(name) {
        Some(s) => s,
        None => return,
    };

    let cache = match GLOBAL_RESOURCE_CACHE.get() {
        Some(c) => c,
        None => return,
    };

    // Copy the data into a Vec
    let data_vec = if data.is_null() || size == 0 {
        Vec::new()
    } else {
        let mut vec = vec![0u8; size];
        ptr::copy_nonoverlapping(data, vec.as_mut_ptr(), size);
        vec
    };

    cache.insert(&key, data_vec);
}

/// Clear all entries from the cache
#[no_mangle]
pub extern "C" fn rust_cache_clear() {
    if let Some(cache) = GLOBAL_RESOURCE_CACHE.get() {
        cache.clear();
    }
}

/// Get the current cache size in bytes
///
/// # Returns
/// The current size of all cached data, or 0 if cache not initialized
#[no_mangle]
pub extern "C" fn rust_cache_size() -> usize {
    match GLOBAL_RESOURCE_CACHE.get() {
        Some(cache) => cache.size_bytes(),
        None => 0,
    }
}

/// Get the number of entries in the cache
///
/// # Returns
/// The number of cached entries, or 0 if cache not initialized
#[no_mangle]
pub extern "C" fn rust_cache_len() -> usize {
    match GLOBAL_RESOURCE_CACHE.get() {
        Some(cache) => cache.len(),
        None => 0,
    }
}

/// Check if a key exists in the cache
///
/// # Safety
///
/// - `name` must point to a valid null-terminated C string or be null
///
/// # Returns
/// 1 if the key exists in cache, 0 otherwise
#[no_mangle]
pub unsafe extern "C" fn rust_cache_contains(name: *const c_char) -> c_int {
    let key = match cstr_to_string(name) {
        Some(s) => s,
        None => return 0,
    };

    match GLOBAL_RESOURCE_CACHE.get() {
        Some(cache) => {
            if cache.contains(&key) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // =========================================================================
    // Original ResourceSystem tests
    // =========================================================================

    #[test]
    fn test_rust_init_resource_system() {
        let temp_dir = env::temp_dir();
        let c_path = CString::new(temp_dir.to_str().unwrap()).unwrap();

        unsafe {
            let result = rust_init_resource_system(c_path.as_ptr());
            assert_eq!(result, 1);
        }
    }

    #[test]
    fn test_null_pointers() {
        unsafe {
            assert_eq!(rust_init_resource_system(ptr::null()), 0);
            assert_eq!(rust_get_string_resource(ptr::null()).is_null(), true);
            assert_eq!(rust_get_int_resource(ptr::null()), 0);
            assert_eq!(rust_get_bool_resource(ptr::null()), 0);
        }
    }

    #[test]
    fn test_resources_enabled() {
        let temp_dir = env::temp_dir();
        let c_path = CString::new(temp_dir.to_str().unwrap()).unwrap();

        unsafe {
            rust_init_resource_system(c_path.as_ptr());

            rust_set_resources_enabled(0);
            assert_eq!(rust_resources_enabled(), 0);

            rust_set_resources_enabled(1);
            assert_eq!(rust_resources_enabled(), 1);
        }
    }

    // =========================================================================
    // Resource Loading FFI tests
    // =========================================================================

    /// Helper to create a test environment with files and index
    fn setup_test_loader() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        // Create test files
        fs::write(base_path.join("test.txt"), "Hello, World!").unwrap();
        fs::write(base_path.join("data.bin"), vec![0x01, 0x02, 0x03, 0x04]).unwrap();

        // Create index file (simple format: name = path)
        let index_content = r#"test.resource = test.txt
binary.data = data.bin
"#;
        let mut index_file = fs::File::create(base_path.join("test.rmp")).unwrap();
        index_file.write_all(index_content.as_bytes()).unwrap();

        temp_dir
    }

    #[test]
    fn test_ffi_load_resource() {
        let temp_dir = setup_test_loader();
        let base_path = temp_dir.path();

        let c_base = CString::new(base_path.to_str().unwrap()).unwrap();
        let c_index = CString::new(base_path.join("test.rmp").to_str().unwrap()).unwrap();

        unsafe {
            // Initialize loader
            let result = rust_resource_loader_init(c_base.as_ptr(), c_index.as_ptr());
            assert_eq!(result, 1, "Loader init should succeed");

            // Load resource
            let c_name = CString::new("test.resource").unwrap();
            let mut size: usize = 0;
            let ptr = rust_resource_load(c_name.as_ptr(), &mut size);

            assert!(!ptr.is_null(), "Should load resource");
            assert_eq!(size, 13, "Should be 13 bytes");

            // Check content
            let slice = std::slice::from_raw_parts(ptr, size);
            assert_eq!(slice, b"Hello, World!");

            // Free the memory
            rust_resource_free(ptr, size);
        }
    }

    #[test]
    fn test_ffi_load_string() {
        let temp_dir = setup_test_loader();
        let base_path = temp_dir.path();

        let c_base = CString::new(base_path.to_str().unwrap()).unwrap();
        let c_index = CString::new(base_path.join("test.rmp").to_str().unwrap()).unwrap();

        unsafe {
            // Initialize loader
            rust_resource_loader_init(c_base.as_ptr(), c_index.as_ptr());

            // Load as string
            let c_name = CString::new("test.resource").unwrap();
            let ptr = rust_resource_load_string(c_name.as_ptr());

            assert!(!ptr.is_null(), "Should load string");

            // Check content
            let cstr = CStr::from_ptr(ptr);
            assert_eq!(cstr.to_str().unwrap(), "Hello, World!");

            // Free the string
            rust_free_string(ptr);
        }
    }

    #[test]
    fn test_ffi_exists() {
        let temp_dir = setup_test_loader();
        let base_path = temp_dir.path();

        let c_base = CString::new(base_path.to_str().unwrap()).unwrap();
        let c_index = CString::new(base_path.join("test.rmp").to_str().unwrap()).unwrap();

        unsafe {
            // Initialize loader
            rust_resource_loader_init(c_base.as_ptr(), c_index.as_ptr());

            // Check exists
            let c_exists = CString::new("test.resource").unwrap();
            let c_not_exists = CString::new("nonexistent.resource").unwrap();

            assert_eq!(rust_resource_exists(c_exists.as_ptr()), 1);
            assert_eq!(rust_resource_exists(c_not_exists.as_ptr()), 0);
        }
    }

    // =========================================================================
    // Cache FFI tests
    // =========================================================================

    // Note: These tests use a separate OnceLock, so we can't test init multiple times
    // in the same process. We test the cache behavior assuming init succeeded.

    #[test]
    fn test_ffi_cache_init() {
        // Cache is OnceLock, so init only works once per process
        // First init should succeed or fail (if already initialized by another test)
        let result = rust_cache_init(1024 * 1024);
        // Either 1 (first init) or 0 (already initialized)
        assert!(result == 0 || result == 1);

        // Second init should fail (already initialized)
        let result2 = rust_cache_init(2 * 1024 * 1024);
        assert_eq!(result2, 0);
    }

    #[test]
    fn test_ffi_cache_get_insert() {
        // Ensure cache is initialized (may already be)
        let _ = rust_cache_init(1024 * 1024);

        unsafe {
            // Insert data
            let key = CString::new("test.cache.key").unwrap();
            let data: Vec<u8> = vec![1, 2, 3, 4, 5];
            rust_cache_insert(key.as_ptr(), data.as_ptr(), data.len());

            // Get data back
            let mut size: usize = 0;
            let ptr = rust_cache_get(key.as_ptr(), &mut size);

            assert!(!ptr.is_null(), "Should find cached data");
            assert_eq!(size, 5, "Should be 5 bytes");

            // Verify content
            let slice = std::slice::from_raw_parts(ptr, size);
            assert_eq!(slice, &[1, 2, 3, 4, 5]);

            // Free the returned copy
            rust_resource_free(ptr, size);
        }
    }

    #[test]
    fn test_ffi_null_handling() {
        // Ensure cache is initialized
        let _ = rust_cache_init(1024 * 1024);

        unsafe {
            // Null name for resource load
            let mut size: usize = 0;
            assert!(rust_resource_load(ptr::null(), &mut size).is_null());
            assert!(rust_resource_load_string(ptr::null()).is_null());
            assert_eq!(rust_resource_exists(ptr::null()), 0);

            // Null name for cache operations
            assert!(rust_cache_get(ptr::null(), &mut size).is_null());
            rust_cache_insert(ptr::null(), ptr::null(), 0); // Should not crash

            // Null out_size pointer (should still work)
            let key = CString::new("null.test").unwrap();
            let data: Vec<u8> = vec![10, 20, 30];
            rust_cache_insert(key.as_ptr(), data.as_ptr(), data.len());
            let ptr = rust_cache_get(key.as_ptr(), ptr::null_mut());
            assert!(!ptr.is_null());

            // Free null pointer should not crash
            rust_resource_free(ptr::null_mut(), 0);
        }
    }

    #[test]
    fn test_ffi_cache_clear_and_size() {
        // Ensure cache is initialized
        let _ = rust_cache_init(1024 * 1024);

        unsafe {
            // Insert some data
            let key1 = CString::new("clear.test.1").unwrap();
            let key2 = CString::new("clear.test.2").unwrap();
            let data1: Vec<u8> = vec![1, 2, 3, 4, 5];
            let data2: Vec<u8> = vec![10, 20, 30, 40, 50, 60];

            rust_cache_insert(key1.as_ptr(), data1.as_ptr(), data1.len());
            rust_cache_insert(key2.as_ptr(), data2.as_ptr(), data2.len());

            // Check size increased
            let size_before = rust_cache_size();
            assert!(size_before >= 11, "Cache should have at least 11 bytes");

            // Check contains
            assert_eq!(rust_cache_contains(key1.as_ptr()), 1);
            assert_eq!(rust_cache_contains(key2.as_ptr()), 1);

            // Clear cache
            rust_cache_clear();

            // Check cleared
            let size_after = rust_cache_size();
            assert_eq!(size_after, 0, "Cache should be empty after clear");
            assert_eq!(rust_cache_len(), 0, "Cache should have 0 entries");

            // Keys should no longer exist
            assert_eq!(rust_cache_contains(key1.as_ptr()), 0);
            assert_eq!(rust_cache_contains(key2.as_ptr()), 0);
        }
    }

    #[test]
    fn test_ffi_cache_empty_data() {
        // Ensure cache is initialized
        let _ = rust_cache_init(1024 * 1024);

        unsafe {
            // Insert empty data
            let key = CString::new("empty.data").unwrap();
            rust_cache_insert(key.as_ptr(), ptr::null(), 0);

            // Should be able to get it back
            let mut size: usize = 999;
            let ptr = rust_cache_get(key.as_ptr(), &mut size);

            // Empty data is valid - we return a non-null ptr to distinguish
            // "found with zero size" from "not found"
            assert!(!ptr.is_null(), "Empty data should return non-null ptr");
            assert_eq!(size, 0, "Size should be 0");

            // Free the ptr (even for zero-size, we allocated 1 byte)
            rust_resource_free(ptr, 0);
        }
    }

    #[test]
    fn test_ffi_loader_not_initialized() {
        // Access loader before initialization - we can't truly test this
        // since other tests may have initialized it, but we can test null handling
        unsafe {
            let c_name = CString::new("any.resource").unwrap();
            let mut size: usize = 0;

            // These should return null/0 gracefully without crashing
            // (loader may or may not be initialized by other tests)
            let _ = rust_resource_load(c_name.as_ptr(), &mut size);
            let _ = rust_resource_load_string(c_name.as_ptr());
            let _ = rust_resource_exists(c_name.as_ptr());
        }
    }

    #[test]
    fn test_ffi_loader_nonexistent_resource() {
        let temp_dir = setup_test_loader();
        let base_path = temp_dir.path();

        let c_base = CString::new(base_path.to_str().unwrap()).unwrap();
        let c_index = CString::new(base_path.join("test.rmp").to_str().unwrap()).unwrap();

        unsafe {
            rust_resource_loader_init(c_base.as_ptr(), c_index.as_ptr());

            // Try to load nonexistent resource
            let c_name = CString::new("does.not.exist").unwrap();
            let mut size: usize = 0;
            let ptr = rust_resource_load(c_name.as_ptr(), &mut size);

            assert!(ptr.is_null(), "Should return null for nonexistent resource");
        }
    }
}
