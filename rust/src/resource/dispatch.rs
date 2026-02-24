//! Resource dispatch layer — lazy loading, refcounting, and lifecycle management
//!
//! Provides `ResourceDispatch` which ties together `ResourceMap` (from config_api)
//! and `TypeRegistry` to implement the full resource get/free/detach/remove lifecycle
//! matching the C `getres.c` dispatch functions.
//!
//! @plan PLAN-20260224-RES-SWAP.P15
//! @plan PLAN-20260224-RES-SWAP.P17
//! @requirement REQ-RES-026-046, REQ-RES-083-085, REQ-RES-101-103

use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::ptr;

use super::ffi_types::ResourceData;
use super::type_registry::TypeRegistry;

/// Combined resource descriptor with FFI data and refcount.
///
/// Merges config_api's `ResourceDesc` (fname, res_type) with ffi_types' `ResourceData`
/// (the loaded data union) and refcount tracking for the dispatch lifecycle.
pub struct FullResourceDesc {
    /// File path or value string (from TYPE:path in the index)
    pub fname: String,
    /// Type name: "STRING", "INT32", "BOOLEAN", "COLOR", "GFXRES", etc.
    pub res_type: String,
    /// FFI-compatible data union (num/ptr/str_ptr)
    pub data: ResourceData,
    /// Reference count for loaded heap resources
    pub refcount: u32,
    /// Key into TypeRegistry for handler lookup (e.g., "GFXRES")
    pub type_handler_key: String,
    /// Owned CString for passing fname to C loaders — kept alive for pointer stability
    fname_cstring: Option<CString>,
}

/// Resource dispatch system combining entries and type registry.
///
/// This is the main dispatch layer that implements lazy loading, refcounting,
/// and lifecycle management. It operates on `FullResourceDesc` entries and
/// uses the `TypeRegistry` to look up load/free function pointers.
pub struct ResourceDispatch {
    /// Resource entries keyed by resource name
    pub entries: HashMap<String, FullResourceDesc>,
    /// Type handler registry
    pub type_registry: TypeRegistry,
    /// Current resource file name (set during load operations)
    pub cur_resfile_name: Option<String>,
}

impl ResourceDispatch {
    /// Create a new empty resource dispatch system.
    pub fn new() -> Self {
        ResourceDispatch {
            entries: HashMap::new(),
            type_registry: TypeRegistry::new(),
            cur_resfile_name: None,
        }
    }

    /// Parse a `TYPE:path` value and create an entry in the dispatch map.
    ///
    /// For value types (where `free_fun` is `None`), the `load_fun` is called
    /// immediately to populate `data`. For heap types, loading is deferred
    /// until `get_resource()` is called (lazy loading).
    ///
    /// If the type is not found in the registry, the entry is stored as
    /// `UNKNOWNRES` with a warning.
    pub fn process_resource_desc(&mut self, key: &str, value: &str) {
        let (type_name, path) = match value.split_once(':') {
            Some((t, p)) => (t, p),
            None => {
                log::warn!("Invalid resource descriptor '{}' for key '{}'", value, key);
                return;
            }
        };

        let handlers = self.type_registry.lookup(type_name);

        let (handler_key, is_value_type) = if let Some(h) = handlers {
            let is_value = h.free_fun.is_none();
            (type_name.to_string(), is_value)
        } else {
            log::warn!(
                "Unknown resource type '{}' for key '{}', storing as UNKNOWNRES",
                type_name,
                key
            );
            ("UNKNOWNRES".to_string(), false)
        };

        let fname_cstring = CString::new(path).ok();
        let mut data = ResourceData::default();

        // For value types, call loadFun immediately
        if is_value_type {
            if let Some(h) = self.type_registry.lookup(type_name) {
                if let Some(load_fn) = h.load_fun {
                    if let Some(ref cs) = fname_cstring {
                        unsafe {
                            load_fn(cs.as_ptr(), &mut data);
                        }
                    }
                }
            }
        }

        let desc = FullResourceDesc {
            fname: path.to_string(),
            res_type: type_name.to_string(),
            data,
            refcount: 0,
            type_handler_key: handler_key,
            fname_cstring,
        };

        self.entries.insert(key.to_string(), desc);
    }

    /// Lazy-load and return a resource pointer, incrementing its refcount.
    ///
    /// For heap types, the first call triggers `loadFun` via the type handler.
    /// Subsequent calls return the cached pointer. Each call increments the
    /// refcount. Returns null if the key is not found, or if loading fails.
    ///
    /// Pseudocode: component-003.md lines 28-50
    /// @requirement REQ-RES-026-033
    pub fn get_resource(&mut self, key: &str) -> *mut c_void {
        let desc = match self.entries.get_mut(key) {
            Some(d) => d,
            None => {
                log::warn!("Trying to get undefined resource '{}'", key);
                return ptr::null_mut();
            }
        };

        // Lazy load if not yet loaded (ptr is null)
        let ptr_is_null = unsafe { desc.data.ptr.is_null() };
        if ptr_is_null {
            // Look up the type handler to get loadFun
            let load_fn = self
                .type_registry
                .lookup(&desc.type_handler_key)
                .and_then(|h| h.load_fun);

            if let Some(load_fn) = load_fn {
                let desc = self.entries.get_mut(key).unwrap();
                if let Some(ref cs) = desc.fname_cstring {
                    self.cur_resfile_name = Some(desc.fname.clone());
                    unsafe {
                        load_fn(cs.as_ptr(), &mut desc.data);
                    }
                    self.cur_resfile_name = None;
                }
            }
        }

        let desc = self.entries.get_mut(key).unwrap();

        // Check if load succeeded
        let ptr = unsafe { desc.data.ptr };
        if ptr.is_null() {
            return ptr::null_mut();
        }

        desc.refcount += 1;
        ptr
    }

    /// Decrement refcount and free the resource when it reaches zero.
    ///
    /// Calls `freeFun` via the type handler when refcount drops to zero, then
    /// sets the data pointer to null. Logs warnings for undefined keys,
    /// non-heap types, unloaded resources, and zero-refcount free attempts.
    ///
    /// Pseudocode: component-003.md lines 57-83
    /// @requirement REQ-RES-034-040
    pub fn free_resource(&mut self, key: &str) {
        let desc = match self.entries.get_mut(key) {
            Some(d) => d,
            None => {
                log::warn!("Trying to free undefined resource '{}'", key);
                return;
            }
        };

        // Check for non-heap type (no freeFun)
        let free_fn = self
            .type_registry
            .lookup(&desc.type_handler_key)
            .and_then(|h| h.free_fun);

        if free_fn.is_none() {
            log::warn!("Trying to free a non-heap resource '{}'", key);
            return;
        }

        let ptr = unsafe { desc.data.ptr };
        if ptr.is_null() {
            log::warn!("Trying to free not loaded resource '{}'", key);
            return;
        }

        if desc.refcount == 0 {
            log::warn!("Freeing an unreferenced resource '{}'", key);
        }

        if desc.refcount > 0 {
            desc.refcount -= 1;
        }

        if desc.refcount == 0 {
            if let Some(free_fn) = free_fn {
                unsafe {
                    free_fn(desc.data.ptr);
                }
            }
            desc.data = ResourceData {
                ptr: ptr::null_mut(),
            };
        }
    }

    /// Transfer ownership of a loaded resource, detaching it from the dispatch.
    ///
    /// Returns the data pointer and clears the entry (ptr=null, refcount=0).
    /// Fails (returns null) if the key is undefined, the type is non-heap,
    /// the resource isn't loaded, or the refcount is > 1.
    ///
    /// Pseudocode: component-003.md lines 84-108
    /// @requirement REQ-RES-041-046
    pub fn detach_resource(&mut self, key: &str) -> *mut c_void {
        let desc = match self.entries.get_mut(key) {
            Some(d) => d,
            None => {
                log::warn!("Trying to detach undefined resource '{}'", key);
                return ptr::null_mut();
            }
        };

        // Check for non-heap type
        let has_free_fn = self
            .type_registry
            .lookup(&desc.type_handler_key)
            .and_then(|h| h.free_fun)
            .is_some();

        if !has_free_fn {
            log::warn!("Trying to detach a non-heap resource");
            return ptr::null_mut();
        }

        let ptr = unsafe { desc.data.ptr };
        if ptr.is_null() {
            log::warn!("Trying to detach not loaded resource '{}'", key);
            return ptr::null_mut();
        }

        if desc.refcount > 1 {
            log::warn!(
                "Trying to detach a resource referenced {} times",
                desc.refcount
            );
            return ptr::null_mut();
        }

        let result = ptr;
        desc.data = ResourceData {
            ptr: ptr::null_mut(),
        };
        desc.refcount = 0;
        result
    }

    /// Free a loaded resource's data and remove the entry from the map.
    ///
    /// If the resource is loaded and has a `freeFun`, calls it before removing.
    /// Logs a warning if the resource is still referenced (refcount > 0).
    /// Returns `true` if the entry existed and was removed, `false` otherwise.
    ///
    /// Pseudocode: component-003.md lines 109-124
    /// @requirement REQ-RES-083-085
    pub fn remove_resource(&mut self, key: &str) -> bool {
        let desc = match self.entries.get(key) {
            Some(d) => d,
            None => return false,
        };

        let ptr = unsafe { desc.data.ptr };
        if !ptr.is_null() {
            if desc.refcount > 0 {
                log::warn!("Replacing '{}' while it is live", key);
            }

            let free_fn = self
                .type_registry
                .lookup(&desc.type_handler_key)
                .and_then(|h| h.free_fun);

            if let Some(free_fn) = free_fn {
                unsafe {
                    free_fn(ptr);
                }
            }
        }

        self.entries.remove(key);
        true
    }

    /// Get the numeric data value for an INT32 resource entry.
    ///
    /// Accesses `data.num` directly without type checking — the caller
    /// is responsible for ensuring the entry is an INT32 type.
    ///
    /// @requirement REQ-RES-101
    pub fn get_int_resource(&self, key: &str) -> Option<u32> {
        let desc = self.entries.get(key)?;
        Some(unsafe { desc.data.num })
    }

    /// Get a boolean value from a BOOLEAN resource entry.
    ///
    /// Returns `true` if `data.num != 0`, `false` otherwise.
    ///
    /// @requirement REQ-RES-102
    pub fn get_boolean_resource(&self, key: &str) -> Option<bool> {
        let desc = self.entries.get(key)?;
        Some(unsafe { desc.data.num } != 0)
    }

    /// Get the fname (path/value string) for a STRING resource entry.
    ///
    /// @requirement REQ-RES-103
    pub fn get_string_resource(&self, key: &str) -> Option<&str> {
        let desc = self.entries.get(key)?;
        Some(&desc.fname)
    }

    /// Get the type name for a resource entry.
    pub fn get_resource_type(&self, key: &str) -> Option<&str> {
        let desc = self.entries.get(key)?;
        Some(&desc.res_type)
    }
}

impl Default for ResourceDispatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{c_char, c_int, c_uint};

    use crate::resource::ffi_types::{ResourceFreeFun, ResourceLoadFun, ResourceStringFun};

    // extern "C" wrappers around the built-in Rust loaders for use with
    // the FFI-typed TypeRegistry (which expects extern "C" fn pointers).

    unsafe extern "C" fn string_load_wrapper(d: *const c_char, r: *mut ResourceData) {
        crate::resource::type_registry::use_descriptor_as_res(d, r);
    }

    unsafe extern "C" fn int_load_wrapper(d: *const c_char, r: *mut ResourceData) {
        crate::resource::type_registry::descriptor_to_int(d, r);
    }

    unsafe extern "C" fn bool_load_wrapper(d: *const c_char, r: *mut ResourceData) {
        crate::resource::type_registry::descriptor_to_boolean(d, r);
    }

    unsafe extern "C" fn color_load_wrapper(d: *const c_char, r: *mut ResourceData) {
        crate::resource::type_registry::descriptor_to_color(d, r);
    }

    /// Helper: register the 4 built-in value types in a dispatch system.
    fn register_builtin_value_types(dispatch: &mut ResourceDispatch) {
        dispatch
            .type_registry
            .install("STRING", Some(string_load_wrapper), None, None);
        dispatch
            .type_registry
            .install("INT32", Some(int_load_wrapper), None, None);
        dispatch
            .type_registry
            .install("BOOLEAN", Some(bool_load_wrapper), None, None);
        dispatch
            .type_registry
            .install("COLOR", Some(color_load_wrapper), None, None);
    }

    // Dummy heap-type free function for tests
    unsafe extern "C" fn dummy_free(_handle: *mut c_void) -> c_int {
        1
    }

    // Dummy heap-type load function that does nothing (leaves ptr null)
    unsafe extern "C" fn dummy_load_noop(_path: *const c_char, _data: *mut ResourceData) {}

    // Dummy toString for heap types
    unsafe extern "C" fn dummy_tostring(
        _data: *mut ResourceData,
        _buf: *mut c_char,
        _size: c_uint,
    ) {
    }

    // =========================================================================
    // P15/P16/P17 tests
    // =========================================================================

    #[test]
    fn test_resource_dispatch_new() {
        let dispatch = ResourceDispatch::new();
        assert!(dispatch.entries.is_empty());
        assert_eq!(dispatch.type_registry.count(), 0);
        assert!(dispatch.cur_resfile_name.is_none());
    }

    #[test]
    fn test_process_resource_desc_string() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.greeting", "STRING:hello");

        let desc = dispatch.entries.get("test.greeting").unwrap();
        assert_eq!(desc.res_type, "STRING");
        assert_eq!(desc.fname, "hello");
        // STRING loader stores str_ptr, but in tests we verify the entry exists
        assert_eq!(desc.type_handler_key, "STRING");
    }

    #[test]
    fn test_process_resource_desc_int32() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.num", "INT32:42");

        let desc = dispatch.entries.get("test.num").unwrap();
        assert_eq!(desc.res_type, "INT32");
        assert_eq!(unsafe { desc.data.num }, 42);
    }

    #[test]
    fn test_process_resource_desc_boolean() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.flag", "BOOLEAN:true");

        let desc = dispatch.entries.get("test.flag").unwrap();
        assert_eq!(desc.res_type, "BOOLEAN");
        assert_eq!(unsafe { desc.data.num }, 1);
    }

    #[test]
    fn test_process_resource_desc_color() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.color", "COLOR:rgb(255,0,0)");

        let desc = dispatch.entries.get("test.color").unwrap();
        assert_eq!(desc.res_type, "COLOR");
        // Packed: (255 << 24) | (0 << 16) | (0 << 8) | 255 = 0xff0000ff
        assert_eq!(unsafe { desc.data.num }, 0xff0000ff);
    }

    #[test]
    fn test_process_resource_desc_unknown_type() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.mystery", "XYZTYPE:somedata");

        let desc = dispatch.entries.get("test.mystery").unwrap();
        assert_eq!(desc.res_type, "XYZTYPE");
        assert_eq!(desc.type_handler_key, "UNKNOWNRES");
        assert_eq!(desc.fname, "somedata");
    }

    #[test]
    fn test_get_int_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("vol.sfx", "INT32:20");
        assert_eq!(dispatch.get_int_resource("vol.sfx"), Some(20));
    }

    #[test]
    fn test_get_boolean_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("config.fullscreen", "BOOLEAN:true");
        assert_eq!(
            dispatch.get_boolean_resource("config.fullscreen"),
            Some(true)
        );

        dispatch.process_resource_desc("config.subtitles", "BOOLEAN:false");
        assert_eq!(
            dispatch.get_boolean_resource("config.subtitles"),
            Some(false)
        );
    }

    #[test]
    fn test_get_string_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("config.scaler", "STRING:no");
        assert_eq!(dispatch.get_string_resource("config.scaler"), Some("no"));
    }

    #[test]
    fn test_get_resource_type() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.num", "INT32:42");
        assert_eq!(dispatch.get_resource_type("test.num"), Some("INT32"));

        dispatch.process_resource_desc("test.flag", "BOOLEAN:true");
        assert_eq!(dispatch.get_resource_type("test.flag"), Some("BOOLEAN"));

        assert_eq!(dispatch.get_resource_type("no.such.key"), None);
    }

    #[test]
    fn test_remove_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.val", "INT32:99");
        assert!(dispatch.entries.contains_key("test.val"));

        assert!(dispatch.remove_resource("test.val"));
        assert!(!dispatch.entries.contains_key("test.val"));

        // Second remove returns false
        assert!(!dispatch.remove_resource("test.val"));
    }

    #[test]
    fn test_free_resource_not_loaded() {
        let mut dispatch = ResourceDispatch::new();
        // Register a heap type
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(dummy_load_noop as ResourceLoadFun),
            Some(dummy_free as ResourceFreeFun),
            Some(dummy_tostring as ResourceStringFun),
        );

        // Manually insert an entry with null ptr (not loaded)
        let desc = FullResourceDesc {
            fname: "somefile.dat".to_string(),
            res_type: "MOCKHEAP".to_string(),
            data: ResourceData {
                ptr: ptr::null_mut(),
            },
            refcount: 0,
            type_handler_key: "MOCKHEAP".to_string(),
            fname_cstring: CString::new("somefile.dat").ok(),
        };
        dispatch.entries.insert("test.heap".to_string(), desc);

        // Should warn but not crash
        dispatch.free_resource("test.heap");

        // Entry should still exist, unchanged
        let d = dispatch.entries.get("test.heap").unwrap();
        assert_eq!(d.refcount, 0);
        assert!(unsafe { d.data.ptr.is_null() });
    }

    #[test]
    fn test_detach_resource_refcount_too_high() {
        let mut dispatch = ResourceDispatch::new();
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(dummy_load_noop as ResourceLoadFun),
            Some(dummy_free as ResourceFreeFun),
            Some(dummy_tostring as ResourceStringFun),
        );

        // Manually insert a loaded entry with refcount > 1
        let mut value: u32 = 0xDEAD;
        let desc = FullResourceDesc {
            fname: "somefile.dat".to_string(),
            res_type: "MOCKHEAP".to_string(),
            data: ResourceData {
                ptr: &mut value as *mut u32 as *mut c_void,
            },
            refcount: 3,
            type_handler_key: "MOCKHEAP".to_string(),
            fname_cstring: CString::new("somefile.dat").ok(),
        };
        dispatch.entries.insert("test.heap".to_string(), desc);

        // Should return null because refcount > 1
        let result = dispatch.detach_resource("test.heap");
        assert!(result.is_null());

        // Entry should be unchanged
        let d = dispatch.entries.get("test.heap").unwrap();
        assert_eq!(d.refcount, 3);
        assert!(!unsafe { d.data.ptr.is_null() });
    }
}
