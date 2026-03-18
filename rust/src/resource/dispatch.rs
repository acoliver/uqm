//! Resource dispatch layer — lazy loading, refcounting, and lifecycle management
//!
//! Provides `ResourceDispatch` which uses `TypeRegistry` to implement the full
//! resource get/free/detach/remove lifecycle matching the C `getres.c` dispatch functions.
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
/// Merges resource descriptor fields (fname, res_type) with `ResourceData`
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

        // @plan PLAN-20260314-RESOURCE.P04
        // @requirement REQ-RES-UNK-001
        let (handler_key, is_value_type) = if let Some(h) = handlers {
            let is_value = h.free_fun.is_none();
            (type_name.to_string(), is_value)
        } else {
            log::warn!(
                "Unknown resource type '{}' for key '{}', storing as UNKNOWNRES",
                type_name,
                key
            );
            ("UNKNOWNRES".to_string(), true)
        };

        let fname_cstring = CString::new(path).ok();
        let mut data = ResourceData::default();

        // For value types, call loadFun immediately
        // @plan PLAN-20260314-RESOURCE.P04
        // @requirement REQ-RES-UNK-001
        if is_value_type {
            if let Some(h) = self.type_registry.lookup(&handler_key) {
                if let Some(load_fn) = h.load_fun {
                    if let Some(ref cs) = fname_cstring {
                        unsafe {
                            load_fn(cs.as_ptr(), &mut data);
                        }
                    }
                }
            }
        }

        // @plan PLAN-20260314-RESOURCE.P06
        // @requirement REQ-RES-OWN-009
        // Check if replacing an existing entry with loaded heap data
        if let Some(old) = self.entries.get(key) {
            unsafe {
                if !old.data.ptr.is_null() {
                    if let Some(handler) = self.type_registry.lookup(&old.type_handler_key) {
                        if let Some(free_fun) = handler.free_fun {
                            if old.refcount > 0 {
                                log::warn!(
                                    "Replacing resource '{}' with outstanding refcount={}",
                                    key,
                                    old.refcount
                                );
                            }
                            free_fun(old.data.ptr);
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

        // @plan PLAN-20260314-RESOURCE.P04
        // @requirement REQ-RES-LOAD-011
        // Check if this is a value type (no free_fun)
        let is_value_type = self
            .type_registry
            .lookup(&desc.type_handler_key)
            .map(|h| h.free_fun.is_none())
            .unwrap_or(false);

        if is_value_type {
            // Value type: return str_ptr if non-null, else num as pointer
            desc.refcount += 1;
            let str_ptr = unsafe { desc.data.str_ptr };
            if !str_ptr.is_null() {
                return str_ptr as *mut c_void;
            } else {
                return unsafe { desc.data.num } as *mut c_void;
            }
        }

        // Heap type: lazy load if not yet loaded (ptr is null)
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

    /// Clean up all loaded heap resources during shutdown.
    ///
    /// Iterates all entries and calls freeFun on loaded heap resources
    /// (ptr non-null and handler has free_fun). Value types (no free_fun)
    /// and unloaded entries (ptr null) are skipped.
    ///
    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LIFE-004, REQ-RES-OWN-010
    pub fn cleanup_all_entries(&mut self) {
        for (key, entry) in self.entries.iter_mut() {
            unsafe {
                if !entry.data.ptr.is_null() {
                    if let Some(handler) = self.type_registry.lookup(&entry.type_handler_key) {
                        if let Some(free_fun) = handler.free_fun {
                            free_fun(entry.data.ptr);
                            entry.data.ptr = ptr::null_mut();
                        }
                    }
                }
            }
        }
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

    // @plan PLAN-20260314-RESOURCE.P04
    // @requirement REQ-RES-UNK-001
    unsafe extern "C" fn unknownres_load_wrapper(
        descriptor: *const c_char,
        data: *mut ResourceData,
    ) {
        if descriptor.is_null() || data.is_null() {
            return;
        }
        (*data).str_ptr = descriptor;
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

    // =========================================================================
    // P03 Value-Type Dispatch TDD tests (RED PHASE)
    // @plan PLAN-20260314-RESOURCE.P03
    // =========================================================================

    #[test]
    fn test_unknownres_registered_as_value_type() {
        // @plan PLAN-20260314-RESOURCE.P03
        // @requirement REQ-RES-UNK-001
        //
        // UNKNOWNRES should be registered as a value type (free_fun = None)
        // and have a load function to store the descriptor string.

        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        // Register UNKNOWNRES (currently done in ffi_bridge init, but verify here)
        dispatch
            .type_registry
            .install("UNKNOWNRES", Some(unknownres_load_wrapper), None, None);

        let handlers = dispatch.type_registry.lookup("UNKNOWNRES").unwrap();

        // UNKNOWNRES should be a value type (no free_fun)
        assert!(
            handlers.free_fun.is_none(),
            "UNKNOWNRES should be a value type with free_fun = None"
        );

        // UNKNOWNRES should have a load_fun to store descriptor
        // GAP: Currently fails because load_fun is None
        assert!(
            handlers.load_fun.is_some(),
            "UNKNOWNRES should have load_fun to store descriptor"
        );
    }

    #[test]
    fn test_process_resource_desc_unknown_type_stores_as_value() {
        // @plan PLAN-20260314-RESOURCE.P03
        // @requirement REQ-RES-UNK-001
        //
        // When an unknown type is encountered, it should be stored as UNKNOWNRES
        // with the descriptor string preserved in str_ptr (eager load for value types).

        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .type_registry
            .install("UNKNOWNRES", Some(unknownres_load_wrapper), None, None);

        // Process an entry with unknown type
        dispatch.process_resource_desc("mykey", "FAKETYPE:some/path.dat");

        let desc = dispatch.entries.get("mykey").unwrap();
        assert_eq!(desc.type_handler_key, "UNKNOWNRES");
        assert_eq!(desc.res_type, "FAKETYPE"); // Preserved original type

        // GAP: Currently fails because UNKNOWNRES has no loadFun and is_value_type is false
        // The str_ptr should be set via eager load
        assert!(
            !unsafe { desc.data.str_ptr.is_null() },
            "UNKNOWNRES entry should have str_ptr set via eager load"
        );
    }

    #[test]
    fn test_get_resource_value_type_string_returns_str_ptr() {
        // @plan PLAN-20260314-RESOURCE.P03
        // @requirement REQ-RES-LOAD-011
        //
        // get_resource() on a STRING value type should return str_ptr
        // and increment refcount.

        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("mykey", "STRING:hello");

        let desc_before = dispatch.entries.get("mykey").unwrap();
        let expected_ptr = unsafe { desc_before.data.str_ptr };

        // GAP: Currently fails because get_resource treats all types as heap types
        let result = dispatch.get_resource("mykey");
        assert!(!result.is_null(), "get_resource should return str_ptr");
        assert_eq!(
            result as *const c_char, expected_ptr,
            "get_resource should return the str_ptr value"
        );

        let desc_after = dispatch.entries.get("mykey").unwrap();
        assert_eq!(desc_after.refcount, 1, "refcount should be incremented");
    }

    #[test]
    fn test_get_resource_value_type_int_returns_num_as_ptr() {
        // @plan PLAN-20260314-RESOURCE.P03
        // @requirement REQ-RES-LOAD-011
        //
        // get_resource() on an INT32 value type should return num as *mut c_void
        // and increment refcount.

        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("mykey", "INT32:42");

        // GAP: Currently fails because get_resource checks data.ptr which is null for value types
        let result = dispatch.get_resource("mykey");
        assert!(
            !result.is_null(),
            "get_resource should return num as pointer"
        );
        assert_eq!(
            result as usize, 42,
            "get_resource should return num cast to pointer"
        );

        let desc = dispatch.entries.get("mykey").unwrap();
        assert_eq!(desc.refcount, 1, "refcount should be incremented");
    }

    #[test]
    fn test_get_resource_unknownres_returns_str_ptr() {
        // @plan PLAN-20260314-RESOURCE.P03
        // @requirement REQ-RES-UNK-003
        //
        // get_resource() on an UNKNOWNRES entry should return the str_ptr
        // (descriptor string) and increment refcount.

        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .type_registry
            .install("UNKNOWNRES", Some(unknownres_load_wrapper), None, None);

        dispatch.process_resource_desc("mykey", "BOGUS:myfile.dat");

        let desc_before = dispatch.entries.get("mykey").unwrap();
        assert_eq!(desc_before.type_handler_key, "UNKNOWNRES");

        // GAP: Currently fails because UNKNOWNRES entry doesn't eager-load str_ptr
        // and get_resource doesn't handle value types correctly
        let result = dispatch.get_resource("mykey");
        assert!(
            !result.is_null(),
            "get_resource on UNKNOWNRES should return str_ptr"
        );

        let desc_after = dispatch.entries.get("mykey").unwrap();
        assert_eq!(desc_after.refcount, 1, "refcount should be incremented");

        // Verify the pointer points to the descriptor string
        let expected_ptr = unsafe { desc_after.data.str_ptr };
        assert_eq!(
            result as *const c_char, expected_ptr,
            "returned pointer should match str_ptr"
        );
    }

    #[test]
    fn test_get_resource_heap_type_still_lazy_loads() {
        // @plan PLAN-20260314-RESOURCE.P03
        // @requirement REQ-RES-LOAD-001
        //
        // Heap types should still use lazy loading behavior.
        // This test verifies existing behavior is preserved.

        let mut dispatch = ResourceDispatch::new();

        // Mock heap-type loader that sets a sentinel value
        unsafe extern "C" fn mock_heap_load(_path: *const c_char, data: *mut ResourceData) {
            // Simulate loading by setting ptr to a non-null value
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }

        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(mock_heap_load as ResourceLoadFun),
            Some(dummy_free as ResourceFreeFun),
            None,
        );

        // Manually insert unloaded heap entry
        let desc = FullResourceDesc {
            fname: "test.dat".to_string(),
            res_type: "MOCKHEAP".to_string(),
            data: ResourceData {
                ptr: ptr::null_mut(),
            },
            refcount: 0,
            type_handler_key: "MOCKHEAP".to_string(),
            fname_cstring: CString::new("test.dat").ok(),
        };
        dispatch.entries.insert("heap.key".to_string(), desc);

        // First get_resource should trigger lazy load
        let result = dispatch.get_resource("heap.key");
        assert_eq!(
            result as usize, 0xDEADBEEF,
            "get_resource should trigger lazy load"
        );

        let desc = dispatch.entries.get("heap.key").unwrap();
        assert_eq!(desc.refcount, 1, "refcount should be 1 after first get");
        assert_eq!(
            unsafe { desc.data.ptr } as usize,
            0xDEADBEEF,
            "loadFun should have been called"
        );
    }

    // =========================================================================
    // P06: Lifecycle & Replacement Cleanup tests
    // @plan PLAN-20260314-RESOURCE.P06
    // =========================================================================

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-OWN-009
    #[test]
    fn test_process_resource_desc_replacement_calls_free_fun() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FREE_CALLED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn mock_free_tracking(data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }

        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }

        let mut dispatch = ResourceDispatch::new();
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(mock_load as ResourceLoadFun),
            Some(mock_free_tracking as ResourceFreeFun),
            None,
        );

        // Insert initial entry and simulate it being loaded
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file1.dat");
        let entry = dispatch.entries.get_mut("test.key").unwrap();
        unsafe {
            entry.data.ptr = 0xDEADBEEF as *mut c_void;
        }

        FREE_CALLED.store(false, Ordering::SeqCst);

        // Replace the entry
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file2.dat");

        assert!(
            FREE_CALLED.load(Ordering::SeqCst),
            "freeFun should have been called on old entry"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-OWN-009
    #[test]
    fn test_process_resource_desc_replacement_warns_on_refcount() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FREE_CALLED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn mock_free_tracking(data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }

        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }

        let mut dispatch = ResourceDispatch::new();
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(mock_load as ResourceLoadFun),
            Some(mock_free_tracking as ResourceFreeFun),
            None,
        );

        // Insert initial entry and simulate it being loaded with refcount > 0
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file1.dat");
        let entry = dispatch.entries.get_mut("test.key").unwrap();
        unsafe {
            entry.data.ptr = 0xDEADBEEF as *mut c_void;
        }
        entry.refcount = 2;

        FREE_CALLED.store(false, Ordering::SeqCst);

        // Replace the entry (should warn but still call freeFun)
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file2.dat");

        assert!(
            FREE_CALLED.load(Ordering::SeqCst),
            "freeFun should have been called even with refcount > 0"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-OWN-005
    #[test]
    fn test_process_resource_desc_replacement_value_type_no_free() {
        register_builtin_value_types(&mut ResourceDispatch::new());
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        // Insert a STRING entry
        dispatch.process_resource_desc("test.key", "STRING:value1");

        // Replace with new STRING entry (should not crash)
        dispatch.process_resource_desc("test.key", "STRING:value2");

        let entry = dispatch.entries.get("test.key").unwrap();
        assert_eq!(entry.fname, "value2");
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LIFE-004
    #[test]
    fn test_uninit_frees_loaded_heap_resources() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FREE_CALLED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn mock_free_tracking(data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }

        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }

        let mut dispatch = ResourceDispatch::new();
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(mock_load as ResourceLoadFun),
            Some(mock_free_tracking as ResourceFreeFun),
            None,
        );

        // Insert and simulate loaded entry
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file.dat");
        let entry = dispatch.entries.get_mut("test.key").unwrap();
        unsafe {
            entry.data.ptr = 0xDEADBEEF as *mut c_void;
        }

        FREE_CALLED.store(false, Ordering::SeqCst);

        // Call cleanup
        dispatch.cleanup_all_entries();

        assert!(
            FREE_CALLED.load(Ordering::SeqCst),
            "cleanup_all_entries should call freeFun on loaded heap resources"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-OWN-005
    #[test]
    fn test_uninit_skips_value_types() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.str", "STRING:value");
        dispatch.process_resource_desc("test.int", "INT32:42");

        // Should not crash (value types have no freeFun)
        dispatch.cleanup_all_entries();
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-OWN-005
    #[test]
    fn test_uninit_skips_unloaded_heap_entries() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FREE_CALLED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn mock_free_tracking(data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }

        unsafe extern "C" fn mock_load(_path: *const c_char, _data: *mut ResourceData) {
            // Don't set ptr - leave it null
        }

        let mut dispatch = ResourceDispatch::new();
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(mock_load as ResourceLoadFun),
            Some(mock_free_tracking as ResourceFreeFun),
            None,
        );

        // Insert entry but don't load it (ptr remains null)
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file.dat");

        FREE_CALLED.store(false, Ordering::SeqCst);

        // Call cleanup
        dispatch.cleanup_all_entries();

        assert!(
            !FREE_CALLED.load(Ordering::SeqCst),
            "cleanup_all_entries should NOT call freeFun on unloaded entries"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LOAD-007
    #[test]
    fn test_free_resource_on_value_type_never_calls_free_fun() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.str", "STRING:value");
        dispatch.process_resource_desc("test.int", "INT32:42");

        // Should log warning but not crash (value types have no freeFun)
        dispatch.free_resource("test.str");
        dispatch.free_resource("test.int");
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LOAD-007
    #[test]
    fn test_detach_resource_on_value_type_returns_null_without_destructor() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.str", "STRING:value");
        dispatch.process_resource_desc("test.bool", "BOOLEAN:true");

        // Should return null and log warning (value types can't be detached)
        let result_str = dispatch.detach_resource("test.str");
        let result_bool = dispatch.detach_resource("test.bool");

        assert!(result_str.is_null(), "detach on STRING should return null");
        assert!(
            result_bool.is_null(),
            "detach on BOOLEAN should return null"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LOAD-008
    #[test]
    fn test_remove_materialized_heap_entry_frees_and_erases_key() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FREE_CALLED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn mock_free_tracking(data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }

        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }

        let mut dispatch = ResourceDispatch::new();
        dispatch.type_registry.install(
            "MOCKHEAP",
            Some(mock_load as ResourceLoadFun),
            Some(mock_free_tracking as ResourceFreeFun),
            None,
        );

        // Insert and simulate loaded entry
        dispatch.process_resource_desc("test.key", "MOCKHEAP:file.dat");
        let entry = dispatch.entries.get_mut("test.key").unwrap();
        unsafe {
            entry.data.ptr = 0xDEADBEEF as *mut c_void;
        }

        FREE_CALLED.store(false, Ordering::SeqCst);

        // Remove the entry
        let result = dispatch.remove_resource("test.key");

        assert!(result, "remove_resource should return true");
        assert!(
            FREE_CALLED.load(Ordering::SeqCst),
            "remove_resource should call freeFun"
        );
        assert!(
            !dispatch.entries.contains_key("test.key"),
            "key should be removed from map"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LOAD-008
    #[test]
    fn test_remove_value_type_erases_key_without_heap_destructor() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);

        dispatch.process_resource_desc("test.str", "STRING:value");
        dispatch.process_resource_desc("test.int", "INT32:42");

        // Should succeed and not crash (no destructor for value types)
        let result_str = dispatch.remove_resource("test.str");
        let result_int = dispatch.remove_resource("test.int");

        assert!(result_str, "remove_resource on STRING should succeed");
        assert!(result_int, "remove_resource on INT32 should succeed");
        assert!(!dispatch.entries.contains_key("test.str"));
        assert!(!dispatch.entries.contains_key("test.int"));
    }
}
