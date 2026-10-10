// FFI-compatible type definitions for the resource system
//
// These types mirror the C definitions in reslib.h and provide
// a stable ABI for interop between Rust and C resource handlers.
//
// @plan PLAN-20260224-RES-SWAP.P12
// @requirement REQ-RES-R008, REQ-RES-R009, REQ-RES-R010

use std::ffi::{c_char, c_int, c_uint, c_void};

use super::raw_resource_type::RawResourceType;

/// FFI-compatible resource data union matching the C `RESOURCE_DATA` type.
///
/// ```c
/// typedef union { DWORD num; void *ptr; const char *str; } RESOURCE_DATA;
/// ```
#[repr(C)]
pub union ResourceData {
    pub num: u32,
    pub ptr: *mut c_void,
    pub str_ptr: *const c_char,
}

impl Default for ResourceData {
    fn default() -> Self {
        // Zero all 8 bytes of the union to ensure ptr reads as null
        ResourceData {
            ptr: std::ptr::null_mut(),
        }
    }
}

// SAFETY: ResourceData is a C union used only through unsafe FFI paths.
// The caller is responsible for ensuring correct field access.
unsafe impl Send for ResourceData {}
unsafe impl Sync for ResourceData {}

/// C function pointer type for loading a resource from a pathname.
///
/// ```c
/// typedef void (ResourceLoadFun)(const char *pathname, RESOURCE_DATA *resdata);
/// ```
pub type ResourceLoadFun = unsafe extern "C" fn(*const c_char, *mut ResourceData);

/// C function pointer type for freeing a resource handle.
///
/// ```c
/// typedef BOOLEAN (ResourceFreeFun)(void *handle);
/// ```
pub type ResourceFreeFun = unsafe extern "C" fn(*mut c_void) -> c_int;

/// C function pointer type for converting a resource to a string representation.
///
/// ```c
/// typedef void (ResourceStringFun)(RESOURCE_DATA *handle, char *buf, unsigned int size);
/// ```
pub type ResourceStringFun = unsafe extern "C" fn(*mut ResourceData, *mut c_char, c_uint);

/// C function pointer type for loading a resource from a file stream.
///
/// ```c
/// typedef void *(ResourceLoadFileFun)(uio_Stream *fp, DWORD len);
/// ```
pub type ResourceLoadFileFun = unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void;

/// A registered resource type: its identifier and the functions that load,
/// free and serialize its resources. A `None` function is the C `NULL`.
///
/// This is the Rust side of C's `ResourceHandlers`; C never sees it, so it has
/// no C layout. The identifier is kept whole and as bytes (see
/// [`RawResourceType`]).
pub struct ResourceHandlers {
    res_type: RawResourceType,
    /// Load function — called to deserialize a resource from its descriptor
    pub load_fun: Option<ResourceLoadFun>,
    /// Free function — called to release a heap-allocated resource (None for value types)
    pub free_fun: Option<ResourceFreeFun>,
    /// ToString function — called to serialize a resource back to descriptor form
    pub to_string: Option<ResourceStringFun>,
}

impl ResourceHandlers {
    pub fn new(
        res_type: RawResourceType,
        load_fun: Option<ResourceLoadFun>,
        free_fun: Option<ResourceFreeFun>,
        to_string: Option<ResourceStringFun>,
    ) -> Self {
        ResourceHandlers {
            res_type,
            load_fun,
            free_fun,
            to_string,
        }
    }

    /// The identifier the type was registered under, whole.
    pub fn res_type(&self) -> &RawResourceType {
        &self.res_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_data_default_zeroed() {
        let data = ResourceData::default();
        unsafe {
            assert_eq!(data.num, 0);
        }
    }

    #[test]
    fn test_resource_data_num() {
        let mut data = ResourceData::default();
        data.num = 42;
        unsafe {
            assert_eq!(data.num, 42);
        }
    }

    #[test]
    fn test_resource_data_ptr() {
        let mut value: u32 = 123;
        let data = ResourceData {
            ptr: &mut value as *mut u32 as *mut c_void,
        };
        unsafe {
            assert!(!data.ptr.is_null());
        }
    }

    #[test]
    fn test_resource_handlers_new() {
        let name = RawResourceType::new("TESTTYPE").unwrap();
        let handlers = ResourceHandlers::new(name, None, None, None);
        assert_eq!(handlers.res_type(), "TESTTYPE");
        assert!(handlers.load_fun.is_none());
        assert!(handlers.free_fun.is_none());
        assert!(handlers.to_string.is_none());
    }

    #[test]
    fn test_resource_handlers_keep_a_long_identifier_whole() {
        let long_name = "A".repeat(64);
        let handlers =
            ResourceHandlers::new(RawResourceType::new(&long_name).unwrap(), None, None, None);
        assert_eq!(handlers.res_type().as_bytes(), long_name.as_bytes());
    }

    #[test]
    fn test_resource_handlers_keep_an_identifier_that_is_not_utf8() {
        let handlers =
            ResourceHandlers::new(RawResourceType::new(b"G\xE9").unwrap(), None, None, None);
        assert_eq!(handlers.res_type().as_bytes(), b"G\xE9");
    }
}
