//! Resource dispatch layer — lazy loading, refcounting, and lifecycle management
//!
//! Provides `ResourceDispatch` which uses `TypeRegistry` to implement the full
//! resource get/free/detach/remove lifecycle matching the C `getres.c` dispatch functions.
//!
//! Resource keys are C strings and are kept as bytes: the C game can name a
//! resource with any byte sequence, and two such sequences are two resources
//! whether or not they decode to the same text.
//!
//! @plan PLAN-20260224-RES-SWAP.P15
//! @plan PLAN-20260224-RES-SWAP.P17
//! @requirement REQ-RES-026-046, REQ-RES-083-085, REQ-RES-101-103

use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::fmt;
use std::ptr;

use super::ffi_types::{ResourceData, ResourceHandlers};
use super::raw_resource_type::RawResourceType;
use super::type_registry::{use_descriptor_as_res, TypeRegistry, TYPE_KEY_PREFIX};

/// The type of every entry whose descriptor names no registered type. It is
/// registered by [`ResourceDispatch::new`], as `InitResourceSystem` does in C.
const UNKNOWN_TYPE: &str = "UNKNOWNRES";

/// Combined resource descriptor with FFI data and refcount.
///
/// Merges the descriptor string with `ResourceData` (the loaded data union)
/// and refcount tracking for the dispatch lifecycle.
pub struct FullResourceDesc {
    /// Identifier of the type handler that owns this entry: "STRING",
    /// "INT32", "BOOLEAN", "COLOR", "GFXRES", ... or `UNKNOWNRES` when the
    /// descriptor had no registered type. This is what C reports as the
    /// entry's type: the whole identifier the type was registered under, which
    /// may be longer than the descriptor's type text and need not be UTF-8.
    pub res_type: RawResourceType,
    /// FFI-compatible data union (num/ptr/str_ptr)
    pub data: ResourceData,
    /// Reference count for loaded heap resources
    pub refcount: u32,
    /// The string the loader receives: the bytes after `TYPE:` for a
    /// registered type, the whole value for `UNKNOWNRES`. It is a C string
    /// because `data.str_ptr` of the string types points into it; its heap
    /// buffer does not move when the entry does.
    descriptor: CString,
}

impl FullResourceDesc {
    /// The descriptor bytes exactly as the index file had them, whatever
    /// their encoding.
    pub fn descriptor_bytes(&self) -> &[u8] {
        self.descriptor.to_bytes()
    }

    #[cfg(test)]
    fn data_is_unloaded(&self) -> bool {
        unsafe { self.data.ptr.is_null() }
    }
}

/// Why a descriptor did not become an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptorError {
    /// The key or the descriptor holds a NUL byte, which no C string can.
    /// An index file cannot produce one: its text ends at the first NUL.
    InteriorNul,
    /// The type is registered without a load function. C refuses to create
    /// the entry and leaves any earlier entry for the key alone.
    NoLoadFunction { type_name: RawResourceType },
    /// The key starts with `sys.`, the part of the resource map that holds the
    /// registered types. C lets such a key replace a type with an ordinary
    /// entry and then reads that entry as a type; the key is refused instead,
    /// and no ordinary resource has a `sys.` name.
    TypeNamespaceKey,
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescriptorError::InteriorNul => {
                write!(f, "key or descriptor contains a NUL byte")
            }
            DescriptorError::NoLoadFunction { type_name } => {
                write!(f, "type {type_name} has no load function")
            }
            DescriptorError::TypeNamespaceKey => {
                write!(f, "key is in the sys. namespace of the registered types")
            }
        }
    }
}

impl std::error::Error for DescriptorError {}

/// Resource dispatch system combining entries and type registry.
///
/// This is the main dispatch layer that implements lazy loading, refcounting,
/// and lifecycle management. It operates on `FullResourceDesc` entries and
/// uses the `TypeRegistry` to look up load/free function pointers.
pub struct ResourceDispatch {
    /// Resource entries keyed by the bytes of the resource name
    pub entries: HashMap<Box<[u8]>, FullResourceDesc>,
    /// Type handler registry
    pub type_registry: TypeRegistry,
    /// Current resource file name (set during load operations)
    pub cur_resfile_name: Option<CString>,
}

impl ResourceDispatch {
    /// Create a new resource dispatch system. Only `UNKNOWNRES` is
    /// registered; it is a value type whose value is its descriptor.
    pub fn new() -> Self {
        let mut type_registry = TypeRegistry::new();
        type_registry
            .install(UNKNOWN_TYPE, Some(use_descriptor_as_res), None, None)
            .unwrap();
        ResourceDispatch {
            entries: HashMap::new(),
            type_registry,
            cur_resfile_name: None,
        }
    }

    /// The entry stored under `key`.
    pub fn entry(&self, key: impl AsRef<[u8]>) -> Option<&FullResourceDesc> {
        self.entries.get(key.as_ref())
    }

    /// Parse a `TYPE:descriptor` value and create an entry in the dispatch
    /// map, as `newResourceDesc` and `process_resource_desc` do in C.
    ///
    /// A value with no colon, or whose type is not registered, becomes an
    /// `UNKNOWNRES` entry that keeps the whole value as its descriptor. The
    /// key is any bytes; the type is looked up as written, so it is neither
    /// trimmed nor case-folded.
    ///
    /// For value types (where `free_fun` is `None`), the `load_fun` is called
    /// immediately to populate `data`. For heap types, loading is deferred
    /// until `get_resource()` is called (lazy loading). An existing entry for
    /// the key is removed first, so the last writer wins.
    ///
    /// # Errors
    ///
    /// [`DescriptorError`] when nothing was stored and the old entry, if any,
    /// is unchanged.
    pub fn process_resource_desc(
        &mut self,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> Result<(), DescriptorError> {
        let (key, value) = (key.as_ref(), value.as_ref());
        if key.contains(&0) {
            return Err(DescriptorError::InteriorNul);
        }
        if key.starts_with(TYPE_KEY_PREFIX) {
            return Err(DescriptorError::TypeNamespaceKey);
        }
        let (handlers, descriptor) = self.resolve_type(key, value);
        let (res_type, load_fun, is_value_type) = (
            handlers.res_type().clone(),
            handlers.load_fun,
            handlers.free_fun.is_none(),
        );
        let descriptor = CString::new(descriptor).map_err(|_| DescriptorError::InteriorNul)?;

        let Some(load_fun) = load_fun else {
            log::warn!(
                "Unable to load '{}'; no handler for type {} defined",
                String::from_utf8_lossy(key),
                res_type
            );
            return Err(DescriptorError::NoLoadFunction {
                type_name: res_type,
            });
        };

        let mut data = ResourceData::default();
        if is_value_type {
            // SAFETY: `descriptor` is a NUL-terminated string that is stored
            // with `data` below and never reallocated.
            unsafe { load_fun(descriptor.as_ptr(), &mut data) };
        }

        // @plan PLAN-20260314-RESOURCE.P06
        // @requirement REQ-RES-OWN-009
        // The old entry's loaded data is released before it is replaced.
        self.remove_resource(key);
        self.entries.insert(
            key.into(),
            FullResourceDesc {
                res_type,
                data,
                refcount: 0,
                descriptor,
            },
        );
        Ok(())
    }

    /// The handlers and the descriptor for `value`: the registered type
    /// named before the first colon with the text after it, or `UNKNOWNRES`
    /// with all of `value`. The type text is looked up as C does, cut to the
    /// registry's key length.
    ///
    /// @requirement REQ-RES-UNK-001
    fn resolve_type<'s, 'v>(
        &'s self,
        key: &[u8],
        value: &'v [u8],
    ) -> (&'s ResourceHandlers, &'v [u8]) {
        let registered = value.iter().position(|&b| b == b':').and_then(|colon| {
            let handlers = self.type_registry.lookup(&value[..colon])?;
            Some((handlers, &value[colon + 1..]))
        });
        registered.unwrap_or_else(|| {
            log::warn!(
                "No registered type in descriptor '{}' for resource '{}'; treating as {}",
                String::from_utf8_lossy(value),
                String::from_utf8_lossy(key),
                UNKNOWN_TYPE
            );
            let unknown = self
                .type_registry
                .lookup(UNKNOWN_TYPE)
                .expect("ResourceDispatch::new registers UNKNOWNRES and nothing removes a type");
            (unknown, value)
        })
    }

    /// Lazy-load and return a resource pointer, incrementing its refcount.
    ///
    /// For heap types, the first call triggers `loadFun` via the type handler.
    /// Subsequent calls return the cached pointer. Each call increments the
    /// refcount. Returns null if the key is not found, or if loading fails.
    ///
    /// Pseudocode: component-003.md lines 28-50
    /// @requirement REQ-RES-026-033
    pub fn get_resource(&mut self, key: impl AsRef<[u8]>) -> *mut c_void {
        let key = key.as_ref();
        let Some(desc) = self.entries.get_mut(key) else {
            log::warn!(
                "Trying to get undefined resource '{}'",
                String::from_utf8_lossy(key)
            );
            return ptr::null_mut();
        };

        let handlers = self.type_registry.lookup(&desc.res_type);

        // @plan PLAN-20260314-RESOURCE.P04
        // @requirement REQ-RES-LOAD-011
        // Check if this is a value type (no free_fun)
        let is_value_type = handlers.is_some_and(|h| h.free_fun.is_none());

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
        if unsafe { desc.data.ptr.is_null() } {
            if let Some(load_fn) = handlers.and_then(|h| h.load_fun) {
                self.cur_resfile_name = Some(desc.descriptor.clone());
                unsafe {
                    load_fn(desc.descriptor.as_ptr(), &mut desc.data);
                }
                self.cur_resfile_name = None;
            }
        }

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
    pub fn free_resource(&mut self, key: impl AsRef<[u8]>) {
        let key = key.as_ref();
        let Some(desc) = self.entries.get_mut(key) else {
            log::warn!(
                "Trying to free undefined resource '{}'",
                String::from_utf8_lossy(key)
            );
            return;
        };

        // Check for non-heap type (no freeFun)
        let free_fn = self
            .type_registry
            .lookup(&desc.res_type)
            .and_then(|h| h.free_fun);

        if free_fn.is_none() {
            log::warn!(
                "Trying to free a non-heap resource '{}'",
                String::from_utf8_lossy(key)
            );
            return;
        }

        let ptr = unsafe { desc.data.ptr };
        if ptr.is_null() {
            log::warn!(
                "Trying to free not loaded resource '{}'",
                String::from_utf8_lossy(key)
            );
            return;
        }

        if desc.refcount == 0 {
            log::warn!(
                "Freeing an unreferenced resource '{}'",
                String::from_utf8_lossy(key)
            );
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
    pub fn detach_resource(&mut self, key: impl AsRef<[u8]>) -> *mut c_void {
        let key = key.as_ref();
        let Some(desc) = self.entries.get_mut(key) else {
            log::warn!(
                "Trying to detach undefined resource '{}'",
                String::from_utf8_lossy(key)
            );
            return ptr::null_mut();
        };

        // Check for non-heap type
        let has_free_fn = self
            .type_registry
            .lookup(&desc.res_type)
            .and_then(|h| h.free_fun)
            .is_some();

        if !has_free_fn {
            log::warn!("Trying to detach a non-heap resource");
            return ptr::null_mut();
        }

        let ptr = unsafe { desc.data.ptr };
        if ptr.is_null() {
            log::warn!(
                "Trying to detach not loaded resource '{}'",
                String::from_utf8_lossy(key)
            );
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
    pub fn remove_resource(&mut self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        let Some(desc) = self.entries.get(key) else {
            return false;
        };

        let ptr = unsafe { desc.data.ptr };
        if !ptr.is_null() {
            if desc.refcount > 0 {
                log::warn!(
                    "Replacing '{}' while it is live",
                    String::from_utf8_lossy(key)
                );
            }

            let free_fn = self
                .type_registry
                .lookup(&desc.res_type)
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
    pub fn get_int_resource(&self, key: impl AsRef<[u8]>) -> Option<u32> {
        let desc = self.entries.get(key.as_ref())?;
        Some(unsafe { desc.data.num })
    }

    /// Get a boolean value from a BOOLEAN resource entry.
    ///
    /// Returns `true` if `data.num != 0`, `false` otherwise.
    ///
    /// @requirement REQ-RES-102
    pub fn get_boolean_resource(&self, key: impl AsRef<[u8]>) -> Option<bool> {
        let desc = self.entries.get(key.as_ref())?;
        Some(unsafe { desc.data.num } != 0)
    }

    /// Get the descriptor bytes of a resource entry (the value of a STRING
    /// resource).
    ///
    /// @requirement REQ-RES-103
    pub fn get_string_resource(&self, key: impl AsRef<[u8]>) -> Option<&[u8]> {
        self.entries
            .get(key.as_ref())
            .map(FullResourceDesc::descriptor_bytes)
    }

    /// Get the type identifier of a resource entry.
    pub fn get_resource_type(&self, key: impl AsRef<[u8]>) -> Option<&RawResourceType> {
        let desc = self.entries.get(key.as_ref())?;
        Some(&desc.res_type)
    }

    /// Whether `key` names an entry or a registered type, as `res_HasKey` asks
    /// of C's one map. Types are answered from the registry; they are not
    /// among [`Self::entries`].
    pub fn has_key(&self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        self.entries.contains_key(key) || self.type_registry.holds_key(key)
    }

    /// Whether `key` names a registered type (`sys.` and a stored identifier).
    /// Such a key has no entry, so no resource getter has anything to say
    /// about it.
    pub fn is_type_key(&self, key: impl AsRef<[u8]>) -> bool {
        self.type_registry.holds_key(key.as_ref())
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
        for entry in self.entries.values_mut() {
            unsafe {
                if !entry.data.ptr.is_null() {
                    if let Some(handler) = self.type_registry.lookup(&entry.res_type) {
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

    /// Helper: register the 4 built-in value types in a dispatch system.
    fn register_builtin_value_types(dispatch: &mut ResourceDispatch) {
        dispatch
            .type_registry
            .install("STRING", Some(string_load_wrapper), None, None)
            .unwrap();
        dispatch
            .type_registry
            .install("INT32", Some(int_load_wrapper), None, None)
            .unwrap();
        dispatch
            .type_registry
            .install("BOOLEAN", Some(bool_load_wrapper), None, None)
            .unwrap();
        dispatch
            .type_registry
            .install("COLOR", Some(color_load_wrapper), None, None)
            .unwrap();
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
        // Only the fallback type is registered, as InitResourceSystem does first in C.
        assert_eq!(dispatch.type_registry.count(), 1);
        assert!(dispatch.type_registry.lookup("UNKNOWNRES").is_some());
        assert!(dispatch.cur_resfile_name.is_none());
    }

    #[test]
    fn test_process_resource_desc_string() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.greeting", "STRING:hello")
            .unwrap();
        let desc = dispatch.entry("test.greeting").unwrap();
        assert_eq!(desc.res_type, "STRING");
        assert_eq!(desc.descriptor_bytes(), b"hello");
        // STRING loader stores str_ptr, but in tests we verify the entry exists
        assert_eq!(desc.res_type, "STRING");
    }

    #[test]
    fn test_process_resource_desc_int32() {
        unsafe {
            let mut dispatch = ResourceDispatch::new();
            register_builtin_value_types(&mut dispatch);
            dispatch
                .process_resource_desc("test.num", "INT32:42")
                .unwrap();
            let desc = dispatch.entry("test.num").unwrap();
            assert_eq!(desc.res_type, "INT32");
            assert_eq!(desc.data.num, 42);
        }
    }

    #[test]
    fn test_process_resource_desc_boolean() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.flag", "BOOLEAN:true")
            .unwrap();
        let desc = dispatch.entry("test.flag").unwrap();
        assert_eq!(desc.res_type, "BOOLEAN");
        assert_eq!(unsafe { desc.data.num }, 1);
    }

    #[test]
    fn test_process_resource_desc_color() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.color", "COLOR:rgb(255,0,0)")
            .unwrap();
        let desc = dispatch.entry("test.color").unwrap();
        assert_eq!(desc.res_type, "COLOR");
        // Packed: (255 << 24) | (0 << 16) | (0 << 8) | 255 = 0xff0000ff
        assert_eq!(unsafe { desc.data.num }, 0xff0000ff);
    }

    #[test]
    fn test_process_resource_desc_unknown_type() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.mystery", "XYZTYPE:somedata")
            .unwrap();
        let desc = dispatch.entry("test.mystery").unwrap();
        assert_eq!(desc.res_type, "UNKNOWNRES");
        assert_eq!(desc.descriptor_bytes(), b"XYZTYPE:somedata");
    }

    #[test]
    fn test_get_int_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("vol.sfx", "INT32:20")
            .unwrap();
        assert_eq!(dispatch.get_int_resource("vol.sfx"), Some(20));
    }

    #[test]
    fn test_get_boolean_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("config.fullscreen", "BOOLEAN:true")
            .unwrap();
        assert_eq!(
            dispatch.get_boolean_resource("config.fullscreen"),
            Some(true)
        );
        dispatch
            .process_resource_desc("config.subtitles", "BOOLEAN:false")
            .unwrap();
        assert_eq!(
            dispatch.get_boolean_resource("config.subtitles"),
            Some(false)
        );
    }

    #[test]
    fn test_get_string_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("config.scaler", "STRING:no")
            .unwrap();
        assert_eq!(
            dispatch.get_string_resource("config.scaler"),
            Some(b"no".as_slice())
        );
    }

    #[test]
    fn test_get_resource_type() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.num", "INT32:42")
            .unwrap();
        assert_eq!(
            dispatch
                .get_resource_type("test.num")
                .map(|name| name.to_string())
                .as_deref(),
            Some("INT32")
        );
        dispatch
            .process_resource_desc("test.flag", "BOOLEAN:true")
            .unwrap();
        assert_eq!(
            dispatch
                .get_resource_type("test.flag")
                .map(|name| name.to_string())
                .as_deref(),
            Some("BOOLEAN")
        );
        assert_eq!(dispatch.get_resource_type("no.such.key"), None);
    }

    #[test]
    fn test_remove_resource() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.val", "INT32:99")
            .unwrap();
        assert!(dispatch.entry("test.val").is_some());
        assert!(dispatch.remove_resource("test.val"));
        assert!(!dispatch.entry("test.val").is_some());
        // Second remove returns false
        assert!(!dispatch.remove_resource("test.val"));
    }

    #[test]
    fn test_free_resource_not_loaded() {
        unsafe {
            let mut dispatch = ResourceDispatch::new();
            // Register a heap type
            dispatch
                .type_registry
                .install(
                    "MOCKHEAP",
                    Some(dummy_load_noop as ResourceLoadFun),
                    Some(dummy_free as ResourceFreeFun),
                    Some(dummy_tostring as ResourceStringFun),
                )
                .unwrap();
            // Manually insert an entry with null ptr (not loaded)
            let desc = FullResourceDesc {
                res_type: RawResourceType::new("MOCKHEAP").unwrap(),
                data: ResourceData {
                    ptr: ptr::null_mut(),
                },
                refcount: 0,
                descriptor: CString::new("somefile.dat").unwrap(),
            };
            dispatch.entries.insert("test.heap".as_bytes().into(), desc);
            // Should warn but not crash
            dispatch.free_resource("test.heap");
            // Entry should still exist, unchanged
            let d = dispatch.entry("test.heap").unwrap();
            assert_eq!(d.refcount, 0);
            assert!(d.data.ptr.is_null());
        }
    }

    #[test]
    fn test_detach_resource_refcount_too_high() {
        let mut dispatch = ResourceDispatch::new();
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(dummy_load_noop as ResourceLoadFun),
                Some(dummy_free as ResourceFreeFun),
                Some(dummy_tostring as ResourceStringFun),
            )
            .unwrap();
        // Manually insert a loaded entry with refcount > 1
        let mut value: u32 = 0xDEAD;
        let desc = FullResourceDesc {
            res_type: RawResourceType::new("MOCKHEAP").unwrap(),
            data: ResourceData {
                ptr: &mut value as *mut u32 as *mut c_void,
            },
            refcount: 3,
            descriptor: CString::new("somefile.dat").unwrap(),
        };
        dispatch.entries.insert("test.heap".as_bytes().into(), desc);
        // Should return null because refcount > 1
        let result = dispatch.detach_resource("test.heap");
        assert!(result.is_null());
        // Entry should be unchanged
        let d = dispatch.entry("test.heap").unwrap();
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
        let dispatch = ResourceDispatch::new();
        let handlers = dispatch.type_registry.lookup("UNKNOWNRES").unwrap();
        // UNKNOWNRES should be a value type (no free_fun)
        assert!(
            handlers.free_fun.is_none(),
            "UNKNOWNRES should be a value type with free_fun = None"
        );
        // UNKNOWNRES should have a load_fun to store descriptor
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
        // Process an entry with unknown type
        dispatch
            .process_resource_desc("mykey", "FAKETYPE:some/path.dat")
            .unwrap();
        let desc = dispatch.entry("mykey").unwrap();
        assert_eq!(desc.res_type, "UNKNOWNRES");
        assert_eq!(desc.descriptor_bytes(), b"FAKETYPE:some/path.dat");
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
        dispatch
            .process_resource_desc("mykey", "STRING:hello")
            .unwrap();
        let desc_before = dispatch.entry("mykey").unwrap();
        let expected_ptr = unsafe { desc_before.data.str_ptr };
        // GAP: Currently fails because get_resource treats all types as heap types
        let result = dispatch.get_resource("mykey");
        assert!(!result.is_null(), "get_resource should return str_ptr");
        assert_eq!(
            result as *const c_char, expected_ptr,
            "get_resource should return the str_ptr value"
        );
        let desc_after = dispatch.entry("mykey").unwrap();
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
        dispatch.process_resource_desc("mykey", "INT32:42").unwrap();
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
        let desc = dispatch.entry("mykey").unwrap();
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
            .process_resource_desc("mykey", "BOGUS:myfile.dat")
            .unwrap();
        let desc_before = dispatch.entry("mykey").unwrap();
        assert_eq!(desc_before.res_type, "UNKNOWNRES");
        let result = dispatch.get_resource("mykey");
        assert!(
            !result.is_null(),
            "get_resource on UNKNOWNRES should return str_ptr"
        );
        let desc_after = dispatch.entry("mykey").unwrap();
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
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(mock_heap_load as ResourceLoadFun),
                Some(dummy_free as ResourceFreeFun),
                None,
            )
            .unwrap();
        // Manually insert unloaded heap entry
        let desc = FullResourceDesc {
            res_type: RawResourceType::new("MOCKHEAP").unwrap(),
            data: ResourceData {
                ptr: ptr::null_mut(),
            },
            refcount: 0,
            descriptor: CString::new("test.dat").unwrap(),
        };
        dispatch.entries.insert("heap.key".as_bytes().into(), desc);
        // First get_resource should trigger lazy load
        let result = dispatch.get_resource("heap.key");
        assert_eq!(
            result as usize, 0xDEADBEEF,
            "get_resource should trigger lazy load"
        );
        let desc = dispatch.entry("heap.key").unwrap();
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
        unsafe extern "C" fn mock_free_tracking(_data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }
        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }
        let mut dispatch = ResourceDispatch::new();
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(mock_load as ResourceLoadFun),
                Some(mock_free_tracking as ResourceFreeFun),
                None,
            )
            .unwrap();
        // Insert initial entry and simulate it being loaded
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file1.dat")
            .unwrap();
        let entry = dispatch.entries.get_mut("test.key".as_bytes()).unwrap();
        entry.data.ptr = 0xDEADBEEF as *mut c_void;
        FREE_CALLED.store(false, Ordering::SeqCst);
        // Replace the entry
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file2.dat")
            .unwrap();
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
        unsafe extern "C" fn mock_free_tracking(_data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }
        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }
        let mut dispatch = ResourceDispatch::new();
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(mock_load as ResourceLoadFun),
                Some(mock_free_tracking as ResourceFreeFun),
                None,
            )
            .unwrap();
        // Insert initial entry and simulate it being loaded with refcount > 0
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file1.dat")
            .unwrap();
        let entry = dispatch.entries.get_mut("test.key".as_bytes()).unwrap();
        entry.data.ptr = 0xDEADBEEF as *mut c_void;
        entry.refcount = 2;
        FREE_CALLED.store(false, Ordering::SeqCst);
        // Replace the entry (should warn but still call freeFun)
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file2.dat")
            .unwrap();
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
        dispatch
            .process_resource_desc("test.key", "STRING:value1")
            .unwrap();
        // Replace with new STRING entry (should not crash)
        dispatch
            .process_resource_desc("test.key", "STRING:value2")
            .unwrap();
        let entry = dispatch.entry("test.key").unwrap();
        assert_eq!(entry.descriptor_bytes(), b"value2");
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LIFE-004
    #[test]
    fn test_uninit_frees_loaded_heap_resources() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static FREE_CALLED: AtomicBool = AtomicBool::new(false);
        unsafe extern "C" fn mock_free_tracking(_data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }
        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }
        let mut dispatch = ResourceDispatch::new();
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(mock_load as ResourceLoadFun),
                Some(mock_free_tracking as ResourceFreeFun),
                None,
            )
            .unwrap();
        // Insert and simulate loaded entry
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file.dat")
            .unwrap();
        let entry = dispatch.entries.get_mut("test.key".as_bytes()).unwrap();
        entry.data.ptr = 0xDEADBEEF as *mut c_void;
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
        dispatch
            .process_resource_desc("test.str", "STRING:value")
            .unwrap();
        dispatch
            .process_resource_desc("test.int", "INT32:42")
            .unwrap();
        // Should not crash (value types have no freeFun)
        dispatch.cleanup_all_entries();
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-OWN-005
    #[test]
    fn test_uninit_skips_unloaded_heap_entries() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static FREE_CALLED: AtomicBool = AtomicBool::new(false);
        unsafe extern "C" fn mock_free_tracking(_data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }
        unsafe extern "C" fn mock_load(_path: *const c_char, _data: *mut ResourceData) {
            // Don't set ptr - leave it null
        }
        let mut dispatch = ResourceDispatch::new();
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(mock_load as ResourceLoadFun),
                Some(mock_free_tracking as ResourceFreeFun),
                None,
            )
            .unwrap();
        // Insert entry but don't load it (ptr remains null)
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file.dat")
            .unwrap();
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
        dispatch
            .process_resource_desc("test.str", "STRING:value")
            .unwrap();
        dispatch
            .process_resource_desc("test.int", "INT32:42")
            .unwrap();
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
        dispatch
            .process_resource_desc("test.str", "STRING:value")
            .unwrap();
        dispatch
            .process_resource_desc("test.bool", "BOOLEAN:true")
            .unwrap();
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
        unsafe extern "C" fn mock_free_tracking(_data: *mut c_void) -> c_int {
            FREE_CALLED.store(true, Ordering::SeqCst);
            1
        }
        unsafe extern "C" fn mock_load(_path: *const c_char, data: *mut ResourceData) {
            (*data).ptr = 0xDEADBEEF as *mut c_void;
        }
        let mut dispatch = ResourceDispatch::new();
        dispatch
            .type_registry
            .install(
                "MOCKHEAP",
                Some(mock_load as ResourceLoadFun),
                Some(mock_free_tracking as ResourceFreeFun),
                None,
            )
            .unwrap();
        // Insert and simulate loaded entry
        dispatch
            .process_resource_desc("test.key", "MOCKHEAP:file.dat")
            .unwrap();
        let entry = dispatch.entries.get_mut("test.key".as_bytes()).unwrap();
        entry.data.ptr = 0xDEADBEEF as *mut c_void;
        FREE_CALLED.store(false, Ordering::SeqCst);
        // Remove the entry
        let result = dispatch.remove_resource("test.key");
        assert!(result, "remove_resource should return true");
        assert!(
            FREE_CALLED.load(Ordering::SeqCst),
            "remove_resource should call freeFun"
        );
        assert!(
            !dispatch.entry("test.key").is_some(),
            "key should be removed from map"
        );
    }

    /// @plan PLAN-20260314-RESOURCE.P06
    /// @requirement REQ-RES-LOAD-008
    #[test]
    fn test_remove_value_type_erases_key_without_heap_destructor() {
        let mut dispatch = ResourceDispatch::new();
        register_builtin_value_types(&mut dispatch);
        dispatch
            .process_resource_desc("test.str", "STRING:value")
            .unwrap();
        dispatch
            .process_resource_desc("test.int", "INT32:42")
            .unwrap();
        // Should succeed and not crash (no destructor for value types)
        let result_str = dispatch.remove_resource("test.str");
        let result_int = dispatch.remove_resource("test.int");
        assert!(result_str, "remove_resource on STRING should succeed");
        assert!(result_int, "remove_resource on INT32 should succeed");
        assert!(!dispatch.entry("test.str").is_some());
        assert!(!dispatch.entry("test.int").is_some());
    }
}

#[cfg(test)]
mod descriptor_tests;
#[cfg(test)]
mod type_tests;
