// Type handler registry for the resource system
//
// Provides `TypeRegistry` which maps type identifiers to `ResourceHandlers`.
// C keeps a type in the same map as the resources, under the key
// "sys.<identifier>" (`InstallResTypeVectors`, resinit.c). The registry is the
// one owner of those keys here: `ResourceDispatch` asks it whether a "sys."
// key exists instead of keeping a second copy of them among the entries.
//
// Also provides built-in value type loaders and toString functions that
// mirror the C implementations in resinit.c.
//
// @plan PLAN-20260224-RES-SWAP.P12
// @plan PLAN-20260224-RES-SWAP.P14
// @requirement REQ-RES-014-017, REQ-RES-004

use std::collections::{hash_map::Entry, HashMap};
use std::fmt;

use super::ffi_types::{ResourceFreeFun, ResourceHandlers, ResourceLoadFun, ResourceStringFun};
use super::raw_resource_type::RawResourceType;

/// Size of the C buffer (`char key[TYPESIZ]`) that holds `"sys."` and a type
/// identifier, NUL included.
const TYPESIZ: usize = 32;

/// The start of every type key in C's resource map.
pub const TYPE_KEY_PREFIX: &[u8] = b"sys.";

/// Longest identifier prefix that is part of a type key: what is left of the C
/// buffer after `"sys."` and the NUL. Longer identifiers are cut to this many
/// bytes, at a byte whether or not that is a character boundary, both when a
/// type is registered and when a descriptor's type text is looked up.
pub const TYPE_KEY_MAX: usize = TYPESIZ - TYPE_KEY_PREFIX.len() - 1;

fn key_of(identifier: &[u8]) -> &[u8] {
    &identifier[..identifier.len().min(TYPE_KEY_MAX)]
}

/// Why a type was not registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    /// The identifier holds a NUL byte, which no C string can.
    InteriorNul,
    /// A type is already registered under the same key, so the same first
    /// [`TYPE_KEY_MAX`] bytes. C keeps the first one and reports failure
    /// (`CharHashTable_add` refuses a key it has), and so does this.
    AlreadyInstalled { installed: RawResourceType },
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallError::InteriorNul => write!(f, "type identifier contains a NUL byte"),
            InstallError::AlreadyInstalled { installed } => {
                write!(f, "type key is already held by {installed}")
            }
        }
    }
}

impl std::error::Error for InstallError {}

/// Registry of resource type handlers.
///
/// Maps type identifiers to their associated load/free/toString function
/// pointers. Identifiers are bytes, and the registry key is the first
/// [`TYPE_KEY_MAX`] of them, as in C. A type stays registered, and the
/// identifier it reports stays at the same address, until the registry is
/// dropped.
pub struct TypeRegistry {
    handlers: HashMap<Box<[u8]>, ResourceHandlers>,
}

impl TypeRegistry {
    /// Create a new empty type registry.
    pub fn new() -> Self {
        TypeRegistry {
            handlers: HashMap::new(),
        }
    }

    /// Install resource type vectors for a type identifier.
    ///
    /// # Errors
    ///
    /// [`InstallError`] when the identifier has a NUL or its key is taken; the
    /// registry is unchanged.
    pub fn install(
        &mut self,
        identifier: impl AsRef<[u8]>,
        load: Option<ResourceLoadFun>,
        free: Option<ResourceFreeFun>,
        to_string: Option<ResourceStringFun>,
    ) -> Result<(), InstallError> {
        let identifier = identifier.as_ref();
        let res_type = RawResourceType::new(identifier).map_err(|_| InstallError::InteriorNul)?;
        match self.handlers.entry(key_of(identifier).into()) {
            Entry::Occupied(taken) => Err(InstallError::AlreadyInstalled {
                installed: taken.get().res_type().clone(),
            }),
            Entry::Vacant(slot) => {
                slot.insert(ResourceHandlers::new(res_type, load, free, to_string));
                Ok(())
            }
        }
    }

    /// Look up a type by the type text of a descriptor or an identifier, cut
    /// to [`TYPE_KEY_MAX`] bytes as C does.
    pub fn lookup(&self, type_text: impl AsRef<[u8]>) -> Option<&ResourceHandlers> {
        self.handlers.get(key_of(type_text.as_ref()))
    }

    /// Whether `key` is the resource-map key of a registered type: `"sys."`
    /// and the stored (cut) identifier, no more and no less.
    pub fn holds_key(&self, key: &[u8]) -> bool {
        key.strip_prefix(TYPE_KEY_PREFIX)
            .is_some_and(|stored| self.handlers.contains_key(stored))
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
pub unsafe extern "C" fn use_descriptor_as_res(
    descriptor: *const c_char,
    resdata: *mut ResourceData,
) {
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
    let bytes = CStr::from_ptr(descriptor).to_bytes();
    let wide = c_strtol(bytes);
    // C leaves the result of atoi() undefined when the number does not fit;
    // the platform libc truncates the long, and so does this.
    if i32::try_from(wide).is_err() {
        log::warn!(
            "INT32 descriptor {:?} does not fit in 32 bits; keeping the low 32 bits",
            String::from_utf8_lossy(bytes)
        );
    }
    (*resdata).num = wide as i32 as u32;
}

/// C `strtol (s, NULL, 10)`: skip leading white space, take an optional sign
/// and the leading decimal digits, ignore the rest, give 0 when there are no
/// digits, and saturate at the 64-bit range.
fn c_strtol(bytes: &[u8]) -> i64 {
    let mut rest = bytes;
    while let [first, tail @ ..] = rest {
        if !matches!(first, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            break;
        }
        rest = tail;
    }
    let negative = match rest {
        [b'-', tail @ ..] => {
            rest = tail;
            true
        }
        [b'+', tail @ ..] => {
            rest = tail;
            false
        }
        _ => false,
    };
    let magnitude = rest
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0i64, |acc, digit| {
            acc.saturating_mul(10)
                .saturating_add(i64::from(digit - b'0'))
        });
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// Built-in BOOLEAN loader: case-insensitive "true" → 1, everything else → 0.
///
/// Matches C `DescriptorToBoolean`.
///
/// # Safety
///
/// `descriptor` must be a valid null-terminated C string.
/// `resdata` must point to a valid `ResourceData`.
pub unsafe extern "C" fn descriptor_to_boolean(
    descriptor: *const c_char,
    resdata: *mut ResourceData,
) {
    if descriptor.is_null() || resdata.is_null() {
        return;
    }
    // strcasecmp (descriptor, "true"): nothing is trimmed.
    let is_true = CStr::from_ptr(descriptor)
        .to_bytes()
        .eq_ignore_ascii_case(b"true");
    (*resdata).num = u32::from(is_true);
}

/// Built-in COLOR loader: parses `rgb()`/`rgba()`/`rgb15()` descriptor into packed RGBA.
///
/// Matches C `DescriptorToColor`. Packs as `(R << 24) | (G << 16) | (B << 8) | A`.
///
/// # Safety
///
/// `descriptor` must be a valid null-terminated C string.
/// `resdata` must point to a valid `ResourceData`.
pub unsafe extern "C" fn descriptor_to_color(
    descriptor: *const c_char,
    resdata: *mut ResourceData,
) {
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
pub unsafe extern "C" fn raw_descriptor(
    resdata: *mut ResourceData,
    buf: *mut c_char,
    size: c_uint,
) {
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
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), copy_len);
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
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), copy_len);
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
pub unsafe extern "C" fn boolean_to_string(
    resdata: *mut ResourceData,
    buf: *mut c_char,
    size: c_uint,
) {
    if resdata.is_null() || buf.is_null() || size == 0 {
        return;
    }
    let s = if (*resdata).num != 0 { "true" } else { "false" };
    let bytes = s.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), copy_len);
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
pub unsafe extern "C" fn color_to_string(
    resdata: *mut ResourceData,
    buf: *mut c_char,
    size: c_uint,
) {
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
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), copy_len);
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
        registry
            .install(
                "TESTTYPE",
                Some(dummy_load),
                Some(dummy_free),
                Some(dummy_tostring),
            )
            .unwrap();
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_install_duplicate_is_refused_and_the_first_stays() {
        let mut registry = TypeRegistry::new();
        registry
            .install(
                "TESTTYPE",
                Some(dummy_load),
                Some(dummy_free),
                Some(dummy_tostring),
            )
            .unwrap();
        let refused = registry
            .install("TESTTYPE", Some(dummy_load), None, Some(dummy_tostring))
            .unwrap_err();

        assert_eq!(
            refused,
            InstallError::AlreadyInstalled {
                installed: RawResourceType::new("TESTTYPE").unwrap()
            }
        );
        assert_eq!(registry.count(), 1);
        let handlers = registry.lookup("TESTTYPE").unwrap();
        assert!(handlers.free_fun.is_some());
    }

    #[test]
    fn test_lookup_existing() {
        let mut registry = TypeRegistry::new();
        registry
            .install(
                "GFXRES",
                Some(dummy_load),
                Some(dummy_free),
                Some(dummy_tostring),
            )
            .unwrap();

        let handlers = registry.lookup("GFXRES");
        assert!(handlers.is_some());
        let h = handlers.unwrap();
        assert_eq!(h.res_type(), "GFXRES");
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
        registry
            .install("TYPE1", Some(dummy_load), None, None)
            .unwrap();
        registry
            .install("TYPE2", Some(dummy_load), None, None)
            .unwrap();
        registry
            .install("TYPE3", Some(dummy_load), None, None)
            .unwrap();
        assert_eq!(registry.count(), 3);
    }

    #[test]
    fn test_install_with_null_fns() {
        let mut registry = TypeRegistry::new();
        registry
            .install("VALTYPE", Some(dummy_load), None, None)
            .unwrap();

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
        registry
            .install("STRING", Some(dummy_load), None, Some(dummy_tostring))
            .unwrap();

        let handlers = registry.lookup("STRING").unwrap();
        assert!(handlers.free_fun.is_none(), "Value types have no free_fun");
    }

    #[test]
    fn test_install_heap_type() {
        let mut registry = TypeRegistry::new();
        registry
            .install("GFXRES", Some(dummy_load), Some(dummy_free), None)
            .unwrap();

        let handlers = registry.lookup("GFXRES").unwrap();
        assert!(handlers.free_fun.is_some(), "Heap types have free_fun");
    }

    #[test]
    fn test_install_type_stores_under_sys_prefix() {
        let mut registry = TypeRegistry::new();
        registry
            .install(
                "TESTTYPE",
                Some(dummy_load),
                Some(dummy_free),
                Some(dummy_tostring),
            )
            .unwrap();

        let handlers = registry.lookup("TESTTYPE");
        assert!(handlers.is_some());
        assert_eq!(handlers.unwrap().res_type(), "TESTTYPE");
        assert!(registry.holds_key(b"sys.TESTTYPE"));
        assert!(!registry.holds_key(b"TESTTYPE"));
    }

    #[test]
    fn test_install_type_returns_ok_on_success() {
        let mut registry = TypeRegistry::new();
        let result = registry.install("NEWTYPE", Some(dummy_load), None, None);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_type_handler_stores_res_type_string() {
        let mut registry = TypeRegistry::new();
        registry
            .install("MYTYPE", Some(dummy_load), None, None)
            .unwrap();

        let handlers = registry.lookup("MYTYPE").unwrap();
        assert_eq!(handlers.res_type(), "MYTYPE");
    }

    #[test]
    fn test_type_handler_stores_function_pointers() {
        let mut registry = TypeRegistry::new();
        registry
            .install(
                "FNTEST",
                Some(dummy_load),
                Some(dummy_free),
                Some(dummy_tostring),
            )
            .unwrap();

        let handlers = registry.lookup("FNTEST").unwrap();
        assert!(handlers.load_fun.is_some());
        assert!(handlers.free_fun.is_some());
        assert!(handlers.to_string.is_some());
    }

    // =========================================================================
    // Identifier bytes and the 27-byte key (resinit.c InstallResTypeVectors,
    // newResourceDesc)
    // =========================================================================

    fn install_plain(registry: &mut TypeRegistry, identifier: impl AsRef<[u8]>) {
        registry
            .install(identifier, Some(dummy_load), None, None)
            .unwrap();
    }

    #[test]
    fn test_key_limit_is_what_resinit_c_leaves_after_prefix_and_nul() {
        // `#define TYPESIZ 32` in resinit.c sizes the buffer for "sys." + type
        // + NUL. Delete this test together with resinit.c.
        let source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sc2/src/libs/resource/resinit.c"
        ))
        .expect("resinit.c is read for TYPESIZ; remove this test when the file is retired");
        let typesiz: usize = source
            .lines()
            .find_map(|line| line.strip_prefix("#define TYPESIZ "))
            .expect("resinit.c defines TYPESIZ")
            .trim()
            .parse()
            .expect("TYPESIZ is a number");
        assert_eq!(TYPE_KEY_MAX, typesiz - "sys.".len() - 1);
        assert_eq!(TYPE_KEY_MAX, 27);
    }

    #[test]
    fn test_ascii_identifiers_look_up_exactly() {
        let mut registry = TypeRegistry::new();
        for name in ["STRING", "INT32", "BOOLEAN", "COLOR", "UNKNOWNRES"] {
            install_plain(&mut registry, name);
        }
        assert_eq!(registry.lookup("INT32").unwrap().res_type(), "INT32");
        assert!(registry.lookup("int32").is_none());
        assert!(registry.lookup("INT32 ").is_none());
        assert!(registry.lookup("").is_none());
    }

    #[test]
    fn test_identifier_that_is_not_utf8_installs_and_is_found_by_its_bytes() {
        let mut registry = TypeRegistry::new();
        install_plain(&mut registry, b"G\xE9");
        install_plain(&mut registry, "G\u{FFFD}");
        assert_eq!(registry.count(), 2);
        assert_eq!(
            registry.lookup(b"G\xE9").unwrap().res_type().as_bytes(),
            b"G\xE9"
        );
        assert_eq!(
            registry.lookup("G\u{FFFD}").unwrap().res_type().as_bytes(),
            "G\u{FFFD}".as_bytes()
        );
    }

    #[test]
    fn test_long_identifier_is_keyed_by_27_bytes_and_keeps_all_of_them() {
        let mut registry = TypeRegistry::new();
        let identifier = "A".repeat(30);
        install_plain(&mut registry, &identifier);
        for length in [27, 28, 30, 40] {
            let found = registry.lookup("A".repeat(length)).unwrap();
            assert_eq!(
                found.res_type().as_bytes(),
                identifier.as_bytes(),
                "{length}"
            );
        }
        assert!(registry.lookup("A".repeat(26)).is_none());
    }

    #[test]
    fn test_cut_inside_a_multibyte_character_is_a_byte_cut() {
        let mut registry = TypeRegistry::new();
        let identifier = "\u{E9}".repeat(20);
        install_plain(&mut registry, &identifier);
        let cut: Vec<u8> = identifier.as_bytes()[..TYPE_KEY_MAX].to_vec();
        assert_eq!(cut.last(), Some(&0xC3));
        assert!(std::str::from_utf8(&cut).is_err());
        assert!(registry.holds_key(&[b"sys.".as_slice(), &cut].concat()));
        let found = registry.lookup("\u{E9}".repeat(14)).unwrap();
        assert_eq!(found.res_type().as_bytes(), identifier.as_bytes());
    }

    #[test]
    fn test_identifiers_sharing_27_bytes_are_one_key_and_the_first_stays() {
        let mut registry = TypeRegistry::new();
        let first = format!("{}xyz", "C".repeat(27));
        let second = format!("{}123", "C".repeat(27));
        install_plain(&mut registry, &first);
        let refused = registry
            .install(&second, Some(dummy_load), Some(dummy_free), None)
            .unwrap_err();
        assert_eq!(
            refused,
            InstallError::AlreadyInstalled {
                installed: RawResourceType::new(&first).unwrap()
            }
        );
        assert_eq!(registry.count(), 1);
        let kept = registry.lookup(&second).unwrap();
        assert_eq!(kept.res_type().as_bytes(), first.as_bytes());
        assert!(kept.free_fun.is_none());
    }

    #[test]
    fn test_identifier_with_a_nul_is_refused() {
        let mut registry = TypeRegistry::new();
        let refused = registry.install(b"A\0B", Some(dummy_load), None, None);
        assert_eq!(refused, Err(InstallError::InteriorNul));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_holds_key_is_the_prefix_and_the_cut_identifier_only() {
        let mut registry = TypeRegistry::new();
        install_plain(&mut registry, "B".repeat(30));
        assert!(registry.holds_key(format!("sys.{}", "B".repeat(27)).as_bytes()));
        assert!(!registry.holds_key(format!("sys.{}", "B".repeat(26)).as_bytes()));
        assert!(!registry.holds_key(format!("sys.{}", "B".repeat(28)).as_bytes()));
        assert!(!registry.holds_key(format!("sys.{}", "B".repeat(30)).as_bytes()));
        assert!(!registry.holds_key(b"sys."));
    }
}
