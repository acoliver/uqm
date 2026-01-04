// FFI bindings for Resource module
// Provides C-compatible interface for resource loading

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::Mutex;

use super::resource_system::ResourceSystem;

/// Global resource system
static GLOBAL_RESOURCE_SYSTEM: Mutex<Option<ResourceSystem>> = Mutex::new(None);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

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
}
