//! Rust-side ABI integration tests for the memory subsystem.
//!
//! These tests exercise the exported `extern "C"` functions through the library
//! crate's public API, verifying ABI-surface behavior from outside the module.
//!
//! @plan PLAN-20260314-MEMORY.P05
//! @requirement REQ-MEM-INT-009 (partial — Rust-side ABI surface only)

use uqm_rust::memory::*;

#[test]
fn test_allocate_and_free_via_exported_abi() {
    unsafe {
        let ptr = rust_hmalloc(64);
        assert!(!ptr.is_null(), "rust_hmalloc(64) returned null");

        let byte_ptr = ptr as *mut u8;
        for i in 0..64 {
            *byte_ptr.add(i) = (i * 3) as u8;
        }
        for i in 0..64 {
            assert_eq!(*byte_ptr.add(i), (i * 3) as u8);
        }

        rust_hfree(ptr);
    }
}

#[test]
fn test_calloc_zero_fill_via_exported_abi() {
    unsafe {
        let ptr = rust_hcalloc(128);
        assert!(!ptr.is_null(), "rust_hcalloc(128) returned null");

        let byte_ptr = ptr as *mut u8;
        for i in 0..128 {
            assert_eq!(*byte_ptr.add(i), 0, "byte {} not zero-filled", i);
        }

        rust_hfree(ptr);
    }
}

#[test]
fn test_realloc_preserves_data_via_exported_abi() {
    unsafe {
        let ptr = rust_hmalloc(32);
        assert!(!ptr.is_null());

        let byte_ptr = ptr as *mut u8;
        for i in 0..32 {
            *byte_ptr.add(i) = (i + 10) as u8;
        }

        let ptr2 = rust_hrealloc(ptr, 256);
        assert!(!ptr2.is_null(), "rust_hrealloc returned null");

        let byte_ptr2 = ptr2 as *mut u8;
        for i in 0..32 {
            assert_eq!(
                *byte_ptr2.add(i),
                (i + 10) as u8,
                "data not preserved at byte {}",
                i
            );
        }

        rust_hfree(ptr2);
    }
}

#[test]
fn test_zero_size_normalization_via_exported_abi() {
    unsafe {
        let p1 = rust_hmalloc(0);
        assert!(!p1.is_null(), "rust_hmalloc(0) returned null");

        let p2 = rust_hcalloc(0);
        assert!(!p2.is_null(), "rust_hcalloc(0) returned null");

        let p3 = rust_hrealloc(std::ptr::null_mut(), 0);
        assert!(!p3.is_null(), "rust_hrealloc(null, 0) returned null");

        rust_hfree(p1);
        rust_hfree(p2);
        rust_hfree(p3);
    }
}

#[test]
fn test_lifecycle_smoke_via_exported_abi() {
    unsafe {
        assert!(rust_mem_init(), "rust_mem_init() should return true");
        assert!(rust_mem_uninit(), "rust_mem_uninit() should return true");
    }
}

#[test]
fn test_realloc_zero_from_live_pointer_via_exported_abi() {
    unsafe {
        let ptr = rust_hmalloc(16);
        assert!(!ptr.is_null());

        let ptr2 = rust_hrealloc(ptr, 0);
        assert!(!ptr2.is_null(), "rust_hrealloc(ptr, 0) returned null");

        rust_hfree(ptr2);
    }
}
