// Type handler registry for the resource system
//
// Provides `TypeRegistry` which maps type names to `ResourceHandlers`.
// Types are stored under "sys.<type_name>" keys to match the C convention
// where InstallResTypeVectors stores handlers alongside resource entries.
//
// Also provides built-in value type loaders and toString functions that
// mirror the C implementations in resinit.c.
//
// @plan PLAN-20260224-RES-SWAP.P12
// @plan PLAN-20260224-RES-SWAP.P14
// @requirement REQ-RES-014-017, REQ-RES-004

use std::collections::HashMap;

use super::ffi_types::{ResourceFreeFun, ResourceHandlers, ResourceLoadFun, ResourceStringFun};

/// Registry of resource type handlers.
///
/// Maps type names to their associated load/free/toString function pointers.
/// Keys are stored with a "sys." prefix to match the C convention where
/// type handlers and resource entries share the same HashMap.
pub struct TypeRegistry {
    handlers: HashMap<String, ResourceHandlers>,
}

impl TypeRegistry {
    /// Create a new empty type registry.
    pub fn new() -> Self {
        TypeRegistry {
            handlers: HashMap::new(),
        }
    }

    /// Install resource type vectors for a given type name.
    ///
    /// Stores the handlers under the key `"sys.<type_name>"`.
    /// If a type with the same name already exists, it is overwritten.
    ///
    /// Returns `true` on success.
    pub fn install(
        &mut self,
        type_name: &str,
        load: Option<ResourceLoadFun>,
        free: Option<ResourceFreeFun>,
        to_string: Option<ResourceStringFun>,
    ) -> bool {
        let key = format!("sys.{}", type_name);
        let handlers = ResourceHandlers::new(type_name, load, free, to_string);
        self.handlers.insert(key, handlers);
        true
    }

    /// Look up a type handler by type name.
    ///
    /// Searches for `"sys.<type_name>"` in the registry.
    pub fn lookup(&self, type_name: &str) -> Option<&ResourceHandlers> {
        let key = format!("sys.{}", type_name);
        self.handlers.get(&key)
    }

    /// Return the number of registered types.
    pub fn count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Built-in value type loaders
// =============================================================================

use std::ffi::{c_char, c_uint, CStr};

use super::ffi_types::ResourceData;
use super::resource_type::{parse_c_color, serialize_color};

/// Built-in STRING loader: stores the descriptor pointer directly as `str_ptr`.
///
/// Matches C `UseDescriptorAsRes`: `resdata->str = pathname;`
///
/// # Safety
///
/// `descriptor` must be a valid null-terminated C string.
/// `resdata` must point to a valid `ResourceData`.
pub unsafe extern "C" fn use_descriptor_as_res(descriptor: *const c_char, resdata: *mut ResourceData) {
    if descriptor.is_null() || resdata.is_null() {
        return;
    }
    (*resdata).str_ptr = descriptor;
}

/// Built-in INT32 loader: parses a decimal integer from the descriptor.
///
/// Matches C `DescriptorToInt`: uses `atoi()` semantics (returns 0 for non-numeric).
///
/// # Safety
///
/// `descriptor` must be a valid null-terminated C string.
/// `resdata` must point to a valid `ResourceData`.
pub unsafe extern "C" fn descriptor_to_int(descriptor: *const c_char, resdata: *mut ResourceData) {
    if descriptor.is_null() || resdata.is_null() {
        return;
    }
    let c_str = CStr::from_ptr(descriptor);
    let s = c_str.to_str().unwrap_or("");
    // atoi semantics: parse leading integer, 0 on failure
    let value: i32 = s.trim().parse().unwrap_or(0);
    (*resdata).num = value as u32;
}

/// Built-in BOOLEAN loader: case-insensitive "true" → 1, everything else → 0.
///
/// Matches C `DescriptorToBoolean`.
///
/// # Safety
///
/// `descriptor` must be a valid null-terminated C string.
/// `resdata` must point to a valid `ResourceData`.
pub unsafe extern "C" fn descriptor_to_boolean(descriptor: *const c_char, resdata: *mut ResourceData) {
    if descriptor.is_null() || resdata.is_null() {
        return;
    }
    let c_str = CStr::from_ptr(descriptor);
    let s = c_str.to_str().unwrap_or("");
    let value = if s.trim().eq_ignore_ascii_case("true") {
        1u32
    } else {
        0u32
    };
    (*resdata).num = value;
}

/// Built-in COLOR loader: parses `rgb()`/`rgba()`/`rgb15()` descriptor into packed RGBA.
///
/// Matches C `DescriptorToColor`. Packs as `(R << 24) | (G << 16) | (B << 8) | A`.
///
/// # Safety
///
/// `descriptor` must be a valid null-terminated C string.
/// `resdata` must point to a valid `ResourceData`.
pub unsafe extern "C" fn descriptor_to_color(descriptor: *const c_char, resdata: *mut ResourceData) {
    if descriptor.is_null() || resdata.is_null() {
        return;
    }
    let c_str = CStr::from_ptr(descriptor);
    let s = c_str.to_str().unwrap_or("");
    match parse_c_color(s) {
        Ok((r, g, b, a)) => {
            (*resdata).num =
                ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32);
        }
        Err(_) => {
            (*resdata).num = 0;
        }
    }
}

// =============================================================================
// Built-in toString functions
// =============================================================================

/// Built-in STRING toString: copies the `str_ptr` content to the output buffer.
///
/// Matches C `RawDescriptor`.
///
/// # Safety
///
/// `resdata` must point to a valid `ResourceData` with a valid `str_ptr`.
/// `buf` must point to a buffer of at least `size` bytes.
pub unsafe extern "C" fn raw_descriptor(resdata: *mut ResourceData, buf: *mut c_char, size: c_uint) {
    if resdata.is_null() || buf.is_null() || size == 0 {
        return;
    }
    let src = (*resdata).str_ptr;
    if src.is_null() {
        *buf = 0;
        return;
    }
    let c_str = CStr::from_ptr(src);
    let bytes = c_str.to_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
    *buf.add(copy_len) = 0;
}

/// Built-in INT32 toString: formats `num` as a decimal string.
///
/// Matches C `IntToString`.
///
/// # Safety
///
/// `resdata` must point to a valid `ResourceData`.
/// `buf` must point to a buffer of at least `size` bytes.
pub unsafe extern "C" fn int_to_string(resdata: *mut ResourceData, buf: *mut c_char, size: c_uint) {
    if resdata.is_null() || buf.is_null() || size == 0 {
        return;
    }
    let value = (*resdata).num as i32;
    let formatted = format!("{}", value);
    let bytes = formatted.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
    *buf.add(copy_len) = 0;
}

/// Built-in BOOLEAN toString: "true" if non-zero, "false" otherwise.
///
/// Matches C `BooleanToString`.
///
/// # Safety
///
/// `resdata` must point to a valid `ResourceData`.
/// `buf` must point to a buffer of at least `size` bytes.
pub unsafe extern "C" fn boolean_to_string(resdata: *mut ResourceData, buf: *mut c_char, size: c_uint) {
    if resdata.is_null() || buf.is_null() || size == 0 {
        return;
    }
    let s = if (*resdata).num != 0 { "true" } else { "false" };
    let bytes = s.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
    *buf.add(copy_len) = 0;
}

/// Built-in COLOR toString: serializes packed RGBA to `rgb()`/`rgba()` format.
///
/// Matches C `ColorToString`. Unpacks from `(R << 24) | (G << 16) | (B << 8) | A`.
///
/// # Safety
///
/// `resdata` must point to a valid `ResourceData`.
/// `buf` must point to a buffer of at least `size` bytes.
pub unsafe extern "C" fn color_to_string(resdata: *mut ResourceData, buf: *mut c_char, size: c_uint) {
    if resdata.is_null() || buf.is_null() || size == 0 {
        return;
    }
    let packed = (*resdata).num;
    let r = ((packed >> 24) & 0xFF) as u8;
    let g = ((packed >> 16) & 0xFF) as u8;
    let b = ((packed >> 8) & 0xFF) as u8;
    let a = (packed & 0xFF) as u8;
    let formatted = serialize_color(r, g, b, a);
    let bytes = formatted.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
    *buf.add(copy_len) = 0;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Dummy C-compatible functions for testing
    unsafe extern "C" fn dummy_load(_path: *const c_char, _data: *mut ResourceData) {}
    unsafe extern "C" fn dummy_free(_handle: *mut std::ffi::c_void) -> std::ffi::c_int {
        1
    }
    unsafe extern "C" fn dummy_tostring(
        _data: *mut ResourceData,
        _buf: *mut c_char,
        _size: c_uint,
    ) {
    }

    // =========================================================================
    // TypeRegistry tests
    // =========================================================================

    #[test]
    fn test_type_registry_new_empty() {
        let registry = TypeRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_install_type_vectors() {
        let mut registry = TypeRegistry::new();
        let result = registry.install(
            "TESTTYPE",
            Some(dummy_load),
            Some(dummy_free),
            Some(dummy_tostring),
        );
        assert!(result);
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_install_duplicate_overwrites() {
        let mut registry = TypeRegistry::new();
        registry.install(
            "TESTTYPE",
            Some(dummy_load),
            Some(dummy_free),
            Some(dummy_tostring),
        );
        registry.install("TESTTYPE", Some(dummy_load), None, Some(dummy_tostring));

        assert_eq!(registry.count(), 1);
        let handlers = registry.lookup("TESTTYPE").unwrap();
        assert!(handlers.free_fun.is_none());
    }

    #[test]
    fn test_lookup_existing() {
        let mut registry = TypeRegistry::new();
        registry.install(
            "GFXRES",
            Some(dummy_load),
            Some(dummy_free),
            Some(dummy_tostring),
        );

        let handlers = registry.lookup("GFXRES");
        assert!(handlers.is_some());
        let h = handlers.unwrap();
        assert_eq!(h.type_name(), "GFXRES");
        assert!(h.load_fun.is_some());
        assert!(h.free_fun.is_some());
        assert!(h.to_string.is_some());
    }

    #[test]
    fn test_lookup_nonexistent() {
        let registry = TypeRegistry::new();
        assert!(registry.lookup("NOSUCHTYPE").is_none());
    }

    #[test]
    fn test_count() {
        let mut registry = TypeRegistry::new();
        registry.install("TYPE1", Some(dummy_load), None, None);
        registry.install("TYPE2", Some(dummy_load), None, None);
        registry.install("TYPE3", Some(dummy_load), None, None);
        assert_eq!(registry.count(), 3);
    }

    #[test]
    fn test_install_with_null_fns() {
        let mut registry = TypeRegistry::new();
        registry.install("VALTYPE", Some(dummy_load), None, None);

        let handlers = registry.lookup("VALTYPE").unwrap();
        assert!(handlers.load_fun.is_some());
        assert!(handlers.free_fun.is_none());
        assert!(handlers.to_string.is_none());
    }

    #[test]
    fn test_resource_data_default_zeroed() {
        let data = ResourceData::default();
        unsafe {
            assert_eq!(data.num, 0);
        }
    }

    #[test]
    fn test_resource_data_num() {
        let data = ResourceData { num: 42 };
        unsafe {
            assert_eq!(data.num, 42);
        }
    }

    #[test]
    fn test_resource_data_ptr() {
        let mut value: u32 = 99;
        let data = ResourceData {
            ptr: &mut value as *mut u32 as *mut std::ffi::c_void,
        };
        unsafe {
            assert!(!data.ptr.is_null());
        }
    }

    // =========================================================================
    // Built-in value type loader tests
    // =========================================================================

    #[test]
    fn test_builtin_string_load() {
        let descriptor = CString::new("hello world").unwrap();
        let mut resdata = ResourceData::default();

        unsafe {
            use_descriptor_as_res(descriptor.as_ptr(), &mut resdata);
            let result = CStr::from_ptr(resdata.str_ptr);
            assert_eq!(result.to_str().unwrap(), "hello world");
        }
    }

    #[test]
    fn test_builtin_int32_load() {
        let descriptor = CString::new("42").unwrap();
        let mut resdata = ResourceData::default();

        unsafe {
            descriptor_to_int(descriptor.as_ptr(), &mut resdata);
            assert_eq!(resdata.num, 42);
        }
    }

    #[test]
    fn test_builtin_int32_load_negative() {
        let descriptor = CString::new("-5").unwrap();
        let mut resdata = ResourceData::default();

        unsafe {
            descriptor_to_int(descriptor.as_ptr(), &mut resdata);
            // -5 as u32 is 0xFFFFFFFB
            assert_eq!(resdata.num, (-5i32) as u32);
        }
    }

    #[test]
    fn test_builtin_int32_load_non_numeric() {
        let descriptor = CString::new("abc").unwrap();
        let mut resdata = ResourceData::default();

        unsafe {
            descriptor_to_int(descriptor.as_ptr(), &mut resdata);
            assert_eq!(resdata.num, 0);
        }
    }

    #[test]
    fn test_builtin_boolean_load_true() {
        for s in &["true", "True", "TRUE"] {
            let descriptor = CString::new(*s).unwrap();
            let mut resdata = ResourceData::default();

            unsafe {
                descriptor_to_boolean(descriptor.as_ptr(), &mut resdata);
                assert_eq!(resdata.num, 1, "Expected 1 for '{}'", s);
            }
        }
    }

    #[test]
    fn test_builtin_boolean_load_false() {
        for s in &["false", "anything", "0", ""] {
            let descriptor = CString::new(*s).unwrap();
            let mut resdata = ResourceData::default();

            unsafe {
                descriptor_to_boolean(descriptor.as_ptr(), &mut resdata);
                assert_eq!(resdata.num, 0, "Expected 0 for '{}'", s);
            }
        }
    }

    #[test]
    fn test_builtin_color_load() {
        let descriptor = CString::new("rgb(0x1a, 0x00, 0x1a)").unwrap();
        let mut resdata = ResourceData::default();

        unsafe {
            descriptor_to_color(descriptor.as_ptr(), &mut resdata);
            // Packed: (0x1a << 24) | (0x00 << 16) | (0x1a << 8) | 0xff
            assert_eq!(resdata.num, 0x1a001aff);
        }
    }

    // =========================================================================
    // Built-in toString tests
    // =========================================================================

    #[test]
    fn test_builtin_string_tostring() {
        let source = CString::new("hello").unwrap();
        let mut resdata = ResourceData {
            str_ptr: source.as_ptr(),
        };
        let mut buf = [0u8; 256];

        unsafe {
            raw_descriptor(&mut resdata, buf.as_mut_ptr() as *mut c_char, 256);
            let result = CStr::from_ptr(buf.as_ptr() as *const c_char);
            assert_eq!(result.to_str().unwrap(), "hello");
        }
    }

    #[test]
    fn test_builtin_int_tostring() {
        let mut resdata = ResourceData { num: 42 };
        let mut buf = [0u8; 256];

        unsafe {
            int_to_string(&mut resdata, buf.as_mut_ptr() as *mut c_char, 256);
            let result = CStr::from_ptr(buf.as_ptr() as *const c_char);
            assert_eq!(result.to_str().unwrap(), "42");
        }
    }

    #[test]
    fn test_builtin_boolean_tostring_true() {
        let mut resdata = ResourceData { num: 1 };
        let mut buf = [0u8; 256];

        unsafe {
            boolean_to_string(&mut resdata, buf.as_mut_ptr() as *mut c_char, 256);
            let result = CStr::from_ptr(buf.as_ptr() as *const c_char);
            assert_eq!(result.to_str().unwrap(), "true");
        }
    }

    #[test]
    fn test_builtin_boolean_tostring_false() {
        let mut resdata = ResourceData { num: 0 };
        let mut buf = [0u8; 256];

        unsafe {
            boolean_to_string(&mut resdata, buf.as_mut_ptr() as *mut c_char, 256);
            let result = CStr::from_ptr(buf.as_ptr() as *const c_char);
            assert_eq!(result.to_str().unwrap(), "false");
        }
    }

    #[test]
    fn test_builtin_color_tostring_opaque() {
        // Packed: (0x1a << 24) | (0x00 << 16) | (0x1a << 8) | 0xff = 0x1a001aff
        let mut resdata = ResourceData { num: 0x1a001aff };
        let mut buf = [0u8; 256];

        unsafe {
            color_to_string(&mut resdata, buf.as_mut_ptr() as *mut c_char, 256);
            let result = CStr::from_ptr(buf.as_ptr() as *const c_char);
            assert_eq!(result.to_str().unwrap(), "rgb(0x1a, 0x00, 0x1a)");
        }
    }

    #[test]
    fn test_builtin_color_tostring_transparent() {
        // Packed: (0xff << 24) | (0x00 << 16) | (0x00 << 8) | 0x80 = 0xff000080
        let mut resdata = ResourceData { num: 0xff000080 };
        let mut buf = [0u8; 256];

        unsafe {
            color_to_string(&mut resdata, buf.as_mut_ptr() as *mut c_char, 256);
            let result = CStr::from_ptr(buf.as_ptr() as *const c_char);
            assert_eq!(result.to_str().unwrap(), "rgba(0xff, 0x00, 0x00, 0x80)");
        }
    }

    // =========================================================================
    // Handler type-differentiation tests
    // =========================================================================

    #[test]
    fn test_install_value_type() {
        let mut registry = TypeRegistry::new();
        registry.install("STRING", Some(dummy_load), None, Some(dummy_tostring));

        let handlers = registry.lookup("STRING").unwrap();
        assert!(handlers.free_fun.is_none(), "Value types have no free_fun");
    }

    #[test]
    fn test_install_heap_type() {
        let mut registry = TypeRegistry::new();
        registry.install("GFXRES", Some(dummy_load), Some(dummy_free), None);

        let handlers = registry.lookup("GFXRES").unwrap();
        assert!(handlers.free_fun.is_some(), "Heap types have free_fun");
    }

    #[test]
    fn test_install_type_stores_under_sys_prefix() {
        let mut registry = TypeRegistry::new();
        registry.install(
            "TESTTYPE",
            Some(dummy_load),
            Some(dummy_free),
            Some(dummy_tostring),
        );

        // lookup uses "sys." prefix internally
        let handlers = registry.lookup("TESTTYPE");
        assert!(handlers.is_some());
        assert_eq!(handlers.unwrap().type_name(), "TESTTYPE");
    }

    #[test]
    fn test_install_type_returns_true_on_success() {
        let mut registry = TypeRegistry::new();
        let result = registry.install("NEWTYPE", Some(dummy_load), None, None);
        assert!(result);
    }

    #[test]
    fn test_type_handler_stores_res_type_string() {
        let mut registry = TypeRegistry::new();
        registry.install("MYTYPE", Some(dummy_load), None, None);

        let handlers = registry.lookup("MYTYPE").unwrap();
        assert_eq!(handlers.type_name(), "MYTYPE");
    }

    #[test]
    fn test_type_handler_stores_function_pointers() {
        let mut registry = TypeRegistry::new();
        registry.install(
            "FNTEST",
            Some(dummy_load),
            Some(dummy_free),
            Some(dummy_tostring),
        );

        let handlers = registry.lookup("FNTEST").unwrap();
        assert!(handlers.load_fun.is_some());
        assert!(handlers.free_fun.is_some());
        assert!(handlers.to_string.is_some());
    }
}
