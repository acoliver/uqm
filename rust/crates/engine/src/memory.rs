use crate::logging::{log_add, LogLevel};
use std::ffi::c_void;

/// Allocate memory using the system malloc
///
/// # Safety
/// This function calls into libc's malloc and is unsafe
///
/// @plan PLAN-20260314-MEMORY.P05
/// @requirement REQ-MEM-ALLOC-001, REQ-MEM-ALLOC-002, REQ-MEM-ALLOC-003, REQ-MEM-ZERO-001,
/// REQ-MEM-OOM-001, REQ-MEM-OOM-002, REQ-MEM-OOM-003, REQ-MEM-ALLOC-008
pub unsafe fn rust_hmalloc(size: usize) -> *mut c_void {
    if size == 0 {
        // Return a non-null pointer for zero-size allocation
        let ptr = libc::malloc(1);
        if ptr.is_null() {
            log_add(LogLevel::Fatal, "HMalloc() FATAL: out of memory.");
            std::process::abort();
        }
        return ptr;
    }

    let ptr = libc::malloc(size);
    if ptr.is_null() {
        log_add(LogLevel::Fatal, "HMalloc() FATAL: out of memory.");
        std::process::abort();
    }
    ptr
}

/// Free memory that was allocated with rust_hmalloc
///
/// # Safety
/// This function calls into libc's free and is unsafe
///
/// @plan PLAN-20260314-MEMORY.P05
/// @requirement REQ-MEM-ALLOC-006, REQ-MEM-ALLOC-007, REQ-MEM-OWN-003
pub unsafe fn rust_hfree(ptr: *mut c_void) {
    if !ptr.is_null() {
        libc::free(ptr);
    }
}

/// Allocate and zero-fill memory using the system malloc
///
/// # Safety
/// This function calls into libc's malloc and memset and is unsafe
///
/// @plan PLAN-20260314-MEMORY.P05
/// @requirement REQ-MEM-ALLOC-004, REQ-MEM-ALLOC-009, REQ-MEM-ZERO-001, REQ-MEM-ZERO-002,
/// REQ-MEM-OOM-001, REQ-MEM-OOM-002, REQ-MEM-OOM-003, REQ-MEM-ALLOC-008
pub unsafe fn rust_hcalloc(size: usize) -> *mut c_void {
    if size == 0 {
        // Return a non-null pointer for zero-size allocation
        let ptr = libc::malloc(1);
        if ptr.is_null() {
            log_add(LogLevel::Fatal, "HCalloc() FATAL: out of memory.");
            std::process::abort();
        }
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
///
/// @plan PLAN-20260314-MEMORY.P05
/// @requirement REQ-MEM-ALLOC-005, REQ-MEM-ALLOC-007, REQ-MEM-ALLOC-010, REQ-MEM-ZERO-003,
/// REQ-MEM-OOM-001, REQ-MEM-OOM-002, REQ-MEM-OOM-003, REQ-MEM-OWN-004, REQ-MEM-OWN-005,
/// REQ-MEM-ALLOC-008
pub unsafe fn rust_hrealloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if size == 0 {
        // If new size is 0, free the pointer and return a minimal allocation
        if !ptr.is_null() {
            libc::free(ptr);
        }
        let new_ptr = libc::malloc(1);
        if new_ptr.is_null() {
            log_add(LogLevel::Fatal, "HRealloc() FATAL: out of memory.");
            std::process::abort();
        }
        return new_ptr;
    }

    let new_ptr = libc::realloc(ptr, size);
    if new_ptr.is_null() {
        log_add(LogLevel::Fatal, "HRealloc() FATAL: out of memory.");
        std::process::abort();
    }
    new_ptr
}

/// Initialize the memory management system
///
/// # Safety
/// Called during the game lifecycle.
///
/// @plan PLAN-20260314-MEMORY.P05
/// @requirement REQ-MEM-LIFE-001, REQ-MEM-LIFE-003, REQ-MEM-LIFE-005, REQ-MEM-LIFE-006
pub unsafe fn rust_mem_init() -> bool {
    // In later phases, this might initialize custom allocators
    log_add(LogLevel::Info, "Rust memory management initialized.");
    true
}

/// Deinitialize the memory management system
///
/// # Safety
/// Called during the game lifecycle.
///
/// @plan PLAN-20260314-MEMORY.P05
/// @requirement REQ-MEM-LIFE-002, REQ-MEM-LIFE-004, REQ-MEM-LIFE-005
pub unsafe fn rust_mem_uninit() -> bool {
    // In later phases, this might deinitialize custom allocators
    log_add(LogLevel::Info, "Rust memory management deinitialized.");
    true
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
    fn test_null_free_is_safe() {
        unsafe {
            // HFree(NULL) must be a safe no-op (specification §14.1)
            rust_hfree(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_realloc_null_ptr_acts_as_malloc() {
        unsafe {
            log_init(10);

            // HRealloc(NULL, size) must behave as HMalloc(size) (specification §14.1)
            let ptr = rust_hrealloc(std::ptr::null_mut(), 64);
            assert!(!ptr.is_null());

            // Verify writable storage
            let byte_ptr = ptr as *mut u8;
            for i in 0..64 {
                *byte_ptr.add(i) = i as u8;
            }
            for i in 0..64 {
                assert_eq!(*byte_ptr.add(i), i as u8);
            }

            rust_hfree(ptr);
        }
    }
}
