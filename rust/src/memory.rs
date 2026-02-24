use crate::logging::{log_add, LogLevel};
use std::ffi::c_void;

/// Allocate memory using the system malloc
///
/// # Safety
/// This function calls into libc's malloc and is unsafe
#[no_mangle]
pub unsafe extern "C" fn rust_hmalloc(size: usize) -> *mut c_void {
    if size == 0 {
        // Return a non-null pointer for zero-size allocation
        return libc::malloc(1);
    }

    let ptr = libc::malloc(size);
    if ptr.is_null() {
        // Log fatal error if allocation failed
        // @plan PLAN-20260224-MEM-SWAP.P05 @requirement REQ-MEM-005
        log_add(LogLevel::Fatal, "HMalloc() FATAL: out of memory.");
        std::process::abort();
    }
    ptr
}

/// Free memory that was allocated with rust_hmalloc
///
/// # Safety
/// This function calls into libc's free and is unsafe
#[no_mangle]
pub unsafe extern "C" fn rust_hfree(ptr: *mut c_void) {
    if !ptr.is_null() {
        libc::free(ptr);
    }
}

/// Allocate and zero-fill memory using the system malloc
///
/// # Safety
/// This function calls into libc's malloc and memset and is unsafe
#[no_mangle]
pub unsafe extern "C" fn rust_hcalloc(size: usize) -> *mut c_void {
    if size == 0 {
        // Return a non-null pointer for zero-size allocation
        let ptr = libc::malloc(1);
        libc::memset(ptr, 0, 1);
        return ptr;
    }

    let ptr = libc::malloc(size);
    if ptr.is_null() {
        log_add(LogLevel::Fatal, "HCalloc() FATAL: out of memory.");
        std::process::abort();
    }
    libc::memset(ptr, 0, size);
    ptr
}

/// Reallocate memory to a new size
///
/// # Safety
/// This function calls into libc's realloc and is unsafe
#[no_mangle]
pub unsafe extern "C" fn rust_hrealloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if size == 0 {
        // If new size is 0, free the pointer and return a minimal allocation
        if !ptr.is_null() {
            libc::free(ptr);
        }
        return libc::malloc(1);
    }

    let new_ptr = libc::realloc(ptr, size);
    if new_ptr.is_null() && size > 0 {
        log_add(LogLevel::Fatal, "HRealloc() FATAL: out of memory.");
        std::process::abort();
    }
    new_ptr
}

/// Initialize the memory management system
///
/// # Safety
/// This function is meant to be called from C code
#[no_mangle]
pub unsafe extern "C" fn rust_mem_init() -> bool {
    // In later phases, this might initialize custom allocators
    log_add(LogLevel::Info, "Rust memory management initialized.");
    true
}

/// Deinitialize the memory management system
///
/// # Safety
/// This function is meant to be called from C code
#[no_mangle]
pub unsafe extern "C" fn rust_mem_uninit() -> bool {
    // In later phases, this might deinitialize custom allocators
    log_add(LogLevel::Info, "Rust memory management deinitialized.");
    true
}

/// Helper function to allocate memory for a C-compatible array of strings
///
/// This is useful for creating arguments arrays to pass to C code
///
/// # Safety
/// This function allocates memory that must be freed with HFree
#[allow(dead_code)]
pub unsafe fn copy_argv_to_c(argv: &[String]) -> (*mut *mut i8, Vec<*mut i8>) {
    use std::ffi::CString;
    use std::ptr;

    // Convert each Rust string to a C string
    let mut c_strings: Vec<*mut i8> = Vec::with_capacity(argv.len());
    for arg in argv {
        let c_string = CString::new(arg.as_str()).expect("Failed to convert argument to C string");
        c_strings.push(c_string.into_raw());
    }

    // Allocate an array of pointers
    let array_ptr = rust_hmalloc(std::mem::size_of::<*mut i8>() * (argv.len() + 1)) as *mut *mut i8;
    if array_ptr.is_null() {
        // Clean up the allocated strings
        for ptr in &c_strings {
            if !ptr.is_null() {
                libc::free(*ptr as *mut c_void);
            }
        }
        panic!("Failed to allocate argv array");
    }

    // Copy the pointers to the array
    for (i, ptr) in c_strings.iter().enumerate() {
        ptr::write(array_ptr.add(i), *ptr);
    }
    // Null-terminate the array
    ptr::write(array_ptr.add(argv.len()), ptr::null_mut());

    (array_ptr, c_strings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::log_init;

    #[test]
    fn test_hmalloc_hfree() {
        unsafe {
            log_init(10);

            let ptr = rust_hmalloc(100);
            assert!(!ptr.is_null());

            // Write some data
            let byte_ptr = ptr as *mut u8;
            for i in 0..100 {
                *byte_ptr.add(i) = i as u8;
            }

            // Verify the data
            for i in 0..100 {
                assert_eq!(*byte_ptr.add(i), i as u8);
            }

            rust_hfree(ptr);
        }
    }

    #[test]
    fn test_hcalloc() {
        unsafe {
            log_init(10);

            let ptr = rust_hcalloc(100);
            assert!(!ptr.is_null());

            // Verify memory is zeroed
            let byte_ptr = ptr as *mut u8;
            for i in 0..100 {
                assert_eq!(*byte_ptr.add(i), 0);
            }

            rust_hfree(ptr);
        }
    }

    #[test]
    fn test_hrealloc() {
        unsafe {
            log_init(10);

            let ptr = rust_hmalloc(10);
            assert!(!ptr.is_null());

            // Write some data
            let byte_ptr = ptr as *mut u8;
            for i in 0..10 {
                *byte_ptr.add(i) = i as u8;
            }

            // Reallocate to larger size
            let new_ptr = rust_hrealloc(ptr, 100);
            assert!(!new_ptr.is_null());

            // Verify old data is still there
            let new_byte_ptr = new_ptr as *mut u8;
            for i in 0..10 {
                assert_eq!(*new_byte_ptr.add(i), i as u8);
            }

            rust_hfree(new_ptr);
        }
    }

    #[test]
    fn test_zero_size_allocations() {
        unsafe {
            log_init(10);

            let ptr = rust_hmalloc(0);
            assert!(!ptr.is_null());

            let calloc_ptr = rust_hcalloc(0);
            assert!(!calloc_ptr.is_null());

            // Reallocation to zero size should work
            let realloc_ptr = rust_hrealloc(ptr, 0);
            assert!(!realloc_ptr.is_null());

            rust_hfree(calloc_ptr);
            rust_hfree(realloc_ptr);
        }
    }

    #[test]
    fn test_copy_argv_to_c() {
        use std::ffi::CStr;

        unsafe {
            let argv = vec![
                "program".to_string(),
                "arg1".to_string(),
                "arg2".to_string(),
            ];

            let (array_ptr, _) = copy_argv_to_c(&argv);

            // Verify the array
            let first: *mut i8 = *array_ptr.add(0);
            let second: *mut i8 = *array_ptr.add(1);
            let third: *mut i8 = *array_ptr.add(2);
            let fourth: *mut i8 = *array_ptr.add(3);

            assert!(!first.is_null());
            assert!(!second.is_null());
            assert!(!third.is_null());
            assert!(fourth.is_null());

            // Verify the strings
            let first_str = CStr::from_ptr(first).to_str().unwrap();
            let second_str = CStr::from_ptr(second).to_str().unwrap();
            let third_str = CStr::from_ptr(third).to_str().unwrap();

            assert_eq!(first_str, "program");
            assert_eq!(second_str, "arg1");
            assert_eq!(third_str, "arg2");

            // Clean up
            let mut i = 0;
            loop {
                let ptr = *array_ptr.add(i);
                if ptr.is_null() {
                    break;
                }
                libc::free(ptr as *mut c_void);
                i += 1;
            }
            rust_hfree(array_ptr as *mut c_void);
        }
    }
}
