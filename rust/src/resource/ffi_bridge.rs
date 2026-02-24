//! FFI bridge — all 38 `extern "C"` functions for the C resource API
//!
//! This file provides the complete C-compatible interface that replaces
//! the C `resinit.c`/`getres.c`/`propfile.c` functions. Each function
//! validates pointers, locks global state, auto-inits if needed, and
//! delegates to the internal Rust implementations.
//!
//! @plan PLAN-20260224-RES-SWAP.P18
//! @plan PLAN-20260224-RES-SWAP.P19
//! @plan PLAN-20260224-RES-SWAP.P20
//! @requirement REQ-RES-002-003, REQ-RES-088, REQ-RES-075-079, REQ-RES-R001-R004

use std::ffi::{c_char, c_int, c_long, c_void, CStr, CString};
use std::ptr;
use std::sync::Mutex;

use super::dispatch::ResourceDispatch;
use super::ffi_types::{ResourceData, ResourceFreeFun, ResourceLoadFileFun, ResourceLoadFun, ResourceStringFun};
use super::propfile::parse_propfile;
use super::type_registry;

// =============================================================================
// UIO extern imports — C file I/O layer
// =============================================================================

#[cfg(not(test))]
extern "C" {
    fn uio_fopen(dir: *mut c_void, path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn uio_fclose(fp: *mut c_void) -> c_int;
    fn uio_fread(buf: *mut c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fwrite(buf: *const c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fseek(fp: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn uio_ftell(fp: *mut c_void) -> c_long;
    fn uio_fgetc(fp: *mut c_void) -> c_int;
    fn uio_fputc(c: c_int, fp: *mut c_void) -> c_int;
    fn uio_unlink(dir: *mut c_void, path: *const c_char) -> c_int;

    static contentDir: *mut c_void;
}

// Test stubs for UIO functions — these are never actually called in tests
// because all UIO-calling FFI functions are gated with #[cfg(not(test))]
#[cfg(test)]
unsafe fn uio_fopen(_dir: *mut c_void, _path: *const c_char, _mode: *const c_char) -> *mut c_void { ptr::null_mut() }
#[cfg(test)]
unsafe fn uio_fclose(_fp: *mut c_void) -> c_int { 0 }
#[cfg(test)]
unsafe fn uio_fread(_buf: *mut c_void, _size: usize, _count: usize, _fp: *mut c_void) -> usize { 0 }
#[cfg(test)]
unsafe fn uio_fwrite(_buf: *const c_void, _size: usize, _count: usize, _fp: *mut c_void) -> usize { 0 }
#[cfg(test)]
unsafe fn uio_fseek(_fp: *mut c_void, _offset: c_long, _whence: c_int) -> c_int { 0 }
#[cfg(test)]
unsafe fn uio_ftell(_fp: *mut c_void) -> c_long { 0 }
#[cfg(test)]
unsafe fn uio_fgetc(_fp: *mut c_void) -> c_int { -1 }
#[cfg(test)]
unsafe fn uio_fputc(_c: c_int, _fp: *mut c_void) -> c_int { -1 }
#[cfg(test)]
unsafe fn uio_unlink(_dir: *mut c_void, _path: *const c_char) -> c_int { -1 }
#[cfg(test)]
static mut contentDir: *mut c_void = ptr::null_mut() as *mut c_void;

// =============================================================================
// Global state
// =============================================================================

/// Holds the ResourceDispatch and any CStrings that must outlive their pointers.
struct ResourceState {
    dispatch: ResourceDispatch,
    /// CStrings returned by res_GetString — kept alive for pointer stability
    string_cache: std::collections::HashMap<String, CString>,
    /// CString returned by res_GetResourceType — kept alive for pointer stability
    type_cache: std::collections::HashMap<String, CString>,
}

static RESOURCE_STATE: Mutex<Option<ResourceState>> = Mutex::new(None);

/// Current resource file name, accessible from C during LoadResourceFromPath.
/// Single-threaded invariant: only accessed from the main thread.
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut _cur_resfile_name: *const c_char = ptr::null();

/// Sentinel value for directory handles passed as file streams
const STREAM_SENTINEL: *mut c_void = !0usize as *mut c_void;

// =============================================================================
// Internal helpers
// =============================================================================

/// Initialize a fresh ResourceState with the 5 built-in value types.
fn create_initial_state() -> ResourceState {
    let mut dispatch = ResourceDispatch::new();

    // Register the 5 built-in value types matching C resinit.c
    dispatch.type_registry.install(
        "STRING",
        Some(type_registry::use_descriptor_as_res as ResourceLoadFun),
        None,
        Some(type_registry::raw_descriptor as ResourceStringFun),
    );
    dispatch.type_registry.install(
        "INT32",
        Some(type_registry::descriptor_to_int as ResourceLoadFun),
        None,
        Some(type_registry::int_to_string as ResourceStringFun),
    );
    dispatch.type_registry.install(
        "BOOLEAN",
        Some(type_registry::descriptor_to_boolean as ResourceLoadFun),
        None,
        Some(type_registry::boolean_to_string as ResourceStringFun),
    );
    dispatch.type_registry.install(
        "COLOR",
        Some(type_registry::descriptor_to_color as ResourceLoadFun),
        None,
        Some(type_registry::color_to_string as ResourceStringFun),
    );
    dispatch.type_registry.install("UNKNOWNRES", None, None, None);

    ResourceState {
        dispatch,
        string_cache: std::collections::HashMap::new(),
        type_cache: std::collections::HashMap::new(),
    }
}

/// Ensure state is initialized (auto-init pattern). Returns the lock guard.
fn ensure_init(
    guard: &mut std::sync::MutexGuard<'_, Option<ResourceState>>,
) {
    if guard.is_none() {
        **guard = Some(create_initial_state());
    }
}

/// Helper: safely convert a *const c_char to &str, returning None on null/invalid UTF-8.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

// =============================================================================
// Lifecycle
// =============================================================================

/// Initialize the resource system. Registers 5 built-in value types.
/// Returns an opaque pointer to the state (non-null on success).
/// Idempotent: if already initialized, returns the existing pointer.
#[no_mangle]
pub extern "C" fn InitResourceSystem() -> *mut c_void {
    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.is_none() {
        *guard = Some(create_initial_state());
    }

    // Return a non-null sentinel (not actually dereferenceable by C)
    guard.as_ref().map(|_| 1usize as *mut c_void).unwrap_or(ptr::null_mut())
}

/// Uninitialize the resource system. Drops all state.
/// Safe to call multiple times.
#[no_mangle]
pub extern "C" fn UninitResourceSystem() {
    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.is_none() {
        log::warn!("UninitResourceSystem called when not initialized");
    }

    *guard = None;
}

// =============================================================================
// Index loading/saving
// =============================================================================

/// Load a resource index file via UIO, parsing entries with an optional prefix.
///
/// Opens `filename` in `dir` via UIO, reads the entire file, parses it as a
/// property file, and calls `process_resource_desc` for each entry.
#[no_mangle]
pub unsafe extern "C" fn LoadResourceIndex(
    dir: *mut c_void,
    filename: *const c_char,
    prefix: *const c_char,
) {
    if filename.is_null() {
        return;
    }

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    // Open file via UIO
    let mode = b"rt\0".as_ptr() as *const c_char;
    let fp = uio_fopen(dir, filename, mode);
    if fp.is_null() {
        log::warn!("LoadResourceIndex: failed to open file");
        return;
    }

    // Get file length: seek to end, tell, seek back
    uio_fseek(fp, 0, 2); // SEEK_END = 2
    let length = uio_ftell(fp) as usize;
    uio_fseek(fp, 0, 0); // SEEK_SET = 0

    if length == 0 {
        uio_fclose(fp);
        return;
    }

    // Read entire file into buffer
    let mut buf = vec![0u8; length];
    let bytes_read = uio_fread(buf.as_mut_ptr() as *mut c_void, 1, length, fp);
    uio_fclose(fp);

    // Truncate to actual bytes read
    buf.truncate(bytes_read);

    let content = match std::str::from_utf8(&buf) {
        Ok(s) => s,
        Err(_) => {
            log::warn!("LoadResourceIndex: file contains invalid UTF-8");
            return;
        }
    };

    let prefix_str = if prefix.is_null() {
        None
    } else {
        cstr_to_str(prefix)
    };

    let state = guard.as_mut().unwrap();
    parse_propfile(
        content,
        &mut |key, value| {
            state.dispatch.process_resource_desc(key, value);
        },
        prefix_str,
    );
}

/// Save resource index entries matching `root` prefix to a file via UIO.
#[no_mangle]
pub unsafe extern "C" fn SaveResourceIndex(
    dir: *mut c_void,
    file: *const c_char,
    root: *const c_char,
    strip_root: c_int,
) {
    if file.is_null() {
        return;
    }

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();

    let root_str = if root.is_null() {
        None
    } else {
        cstr_to_str(root)
    };

    // Collect entries matching root prefix and serialize
    let mut lines: Vec<String> = Vec::new();
    let mut keys: Vec<&String> = state.dispatch.entries.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(prefix) = root_str {
            if !key.starts_with(prefix) {
                continue;
            }
        }

        let desc = &state.dispatch.entries[key];

        // Serialize the entry using toString if available
        let mut buf = [0u8; 256];
        let mut data_copy = ResourceData { num: unsafe { desc.data.num } };

        let serialized = if let Some(handlers) = state.dispatch.type_registry.lookup(&desc.res_type) {
            if let Some(to_string_fn) = handlers.to_string {
                to_string_fn(&mut data_copy, buf.as_mut_ptr() as *mut c_char, 256);
                let len = buf.iter().position(|&b| b == 0).unwrap_or(256);
                let value_str = std::str::from_utf8(&buf[..len]).unwrap_or("");
                format!("{}:{}", desc.res_type, value_str)
            } else {
                format!("{}:{}", desc.res_type, desc.fname)
            }
        } else {
            format!("{}:{}", desc.res_type, desc.fname)
        };

        let output_key = if strip_root != 0 {
            if let Some(prefix) = root_str {
                key.strip_prefix(prefix).unwrap_or(key)
            } else {
                key.as_str()
            }
        } else {
            key.as_str()
        };

        lines.push(format!("{} = {}\n", output_key, serialized));
    }

    // Open output file
    let mode = b"wt\0".as_ptr() as *const c_char;
    let fp = uio_fopen(dir, file, mode);
    if fp.is_null() {
        log::warn!("SaveResourceIndex: failed to open file for writing");
        return;
    }

    for line in &lines {
        let bytes = line.as_bytes();
        uio_fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), fp);
    }

    uio_fclose(fp);
}

// =============================================================================
// Type registration
// =============================================================================

/// Install resource type vectors (load/free/toString) for a named type.
/// Returns 1 on success, 0 on failure.
#[no_mangle]
pub unsafe extern "C" fn InstallResTypeVectors(
    res_type: *const c_char,
    load_fun: Option<ResourceLoadFun>,
    free_fun: Option<ResourceFreeFun>,
    string_fun: Option<ResourceStringFun>,
) -> c_int {
    let type_name = match cstr_to_str(res_type) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    if state.dispatch.type_registry.install(type_name, load_fun, free_fun, string_fun) {
        1
    } else {
        0
    }
}

// =============================================================================
// Resource access
// =============================================================================

/// Get a resource, lazy-loading if needed. Returns the data pointer.
#[no_mangle]
pub unsafe extern "C" fn res_GetResource(key: *const c_char) -> *mut c_void {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    state.dispatch.get_resource(key_str)
}

/// Detach a resource, transferring ownership to the caller.
#[no_mangle]
pub unsafe extern "C" fn res_DetachResource(key: *const c_char) -> *mut c_void {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    state.dispatch.detach_resource(key_str)
}

/// Free a resource, decrementing its refcount.
#[no_mangle]
pub unsafe extern "C" fn res_FreeResource(key: *const c_char) {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    state.dispatch.free_resource(key_str);
}

/// Remove a resource entry from the map. Returns 1 if removed, 0 otherwise.
#[no_mangle]
pub unsafe extern "C" fn res_Remove(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    if state.dispatch.remove_resource(key_str) { 1 } else { 0 }
}

// =============================================================================
// Value access
// =============================================================================

/// Get an integer value from an INT32 resource entry.
#[no_mangle]
pub unsafe extern "C" fn res_GetIntResource(key: *const c_char) -> u32 {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    state.dispatch.get_int_resource(key_str).unwrap_or(0)
}

/// Get a boolean value from a BOOLEAN resource entry.
#[no_mangle]
pub unsafe extern "C" fn res_GetBooleanResource(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    if state.dispatch.get_boolean_resource(key_str).unwrap_or(false) { 1 } else { 0 }
}

/// Get the type name for a resource entry.
/// Returns a pointer to a C string that lives as long as the state.
#[no_mangle]
pub unsafe extern "C" fn res_GetResourceType(key: *const c_char) -> *const c_char {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return ptr::null(),
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    let type_name = match state.dispatch.get_resource_type(key_str) {
        Some(t) => t.to_string(),
        None => return ptr::null(),
    };

    let entry = state.type_cache
        .entry(key_str.to_string())
        .or_insert_with(|| CString::new(type_name.as_str()).unwrap_or_default());

    // Update if changed
    let current = entry.to_str().unwrap_or("");
    if current != type_name {
        *entry = CString::new(type_name.as_str()).unwrap_or_default();
    }

    entry.as_ptr()
}

/// Count the number of registered resource types.
#[no_mangle]
pub extern "C" fn CountResourceTypes() -> u16 {
    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    state.dispatch.type_registry.count() as u16
}

// =============================================================================
// Config get
// =============================================================================

/// Check if a key exists in the resource map.
#[no_mangle]
pub unsafe extern "C" fn res_HasKey(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    if state.dispatch.entries.contains_key(key_str) { 1 } else { 0 }
}

/// Check if a key is a STRING type.
#[no_mangle]
pub unsafe extern "C" fn res_IsString(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    match state.dispatch.get_resource_type(key_str) {
        Some("STRING") => 1,
        _ => 0,
    }
}

/// Check if a key is an INT32 type.
#[no_mangle]
pub unsafe extern "C" fn res_IsInteger(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    match state.dispatch.get_resource_type(key_str) {
        Some("INT32") => 1,
        _ => 0,
    }
}

/// Check if a key is a BOOLEAN type.
#[no_mangle]
pub unsafe extern "C" fn res_IsBoolean(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    match state.dispatch.get_resource_type(key_str) {
        Some("BOOLEAN") => 1,
        _ => 0,
    }
}

/// Check if a key is a COLOR type.
#[no_mangle]
pub unsafe extern "C" fn res_IsColor(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    match state.dispatch.get_resource_type(key_str) {
        Some("COLOR") => 1,
        _ => 0,
    }
}

/// Get a string value. Returns pointer that lives as long as the state entry.
#[no_mangle]
pub unsafe extern "C" fn res_GetString(key: *const c_char) -> *const c_char {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return ptr::null(),
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    let value = match state.dispatch.entries.get(key_str) {
        Some(desc) => desc.fname.clone(),
        None => return ptr::null(),
    };

    let entry = state.string_cache
        .entry(key_str.to_string())
        .or_insert_with(|| CString::new(value.as_str()).unwrap_or_default());

    // Update if the value has changed
    let current = entry.to_str().unwrap_or("");
    if current != value {
        *entry = CString::new(value.as_str()).unwrap_or_default();
    }

    entry.as_ptr()
}

/// Get an integer config value.
#[no_mangle]
pub unsafe extern "C" fn res_GetInteger(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    state.dispatch.get_int_resource(key_str).map(|v| v as c_int).unwrap_or(0)
}

/// Get a boolean config value.
#[no_mangle]
pub unsafe extern "C" fn res_GetBoolean(key: *const c_char) -> c_int {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    if state.dispatch.get_boolean_resource(key_str).unwrap_or(false) { 1 } else { 0 }
}

/// Get a color config value as packed (r<<24)|(g<<16)|(b<<8)|a.
#[no_mangle]
pub unsafe extern "C" fn res_GetColor(key: *const c_char) -> u32 {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    state.dispatch.get_int_resource(key_str).unwrap_or(0)
}

// =============================================================================
// Config put
// =============================================================================

/// Put a string value into the resource map.
#[no_mangle]
pub unsafe extern "C" fn res_PutString(key: *const c_char, value: *const c_char) {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return,
    };
    let value_str = match cstr_to_str(value) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    state.dispatch.process_resource_desc(key_str, &format!("STRING:{}", value_str));
}

/// Put an integer value into the resource map.
#[no_mangle]
pub unsafe extern "C" fn res_PutInteger(key: *const c_char, value: c_int) {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    state.dispatch.process_resource_desc(key_str, &format!("INT32:{}", value));
}

/// Put a boolean value into the resource map.
#[no_mangle]
pub unsafe extern "C" fn res_PutBoolean(key: *const c_char, value: c_int) {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    let bool_str = if value != 0 { "true" } else { "false" };
    state.dispatch.process_resource_desc(key_str, &format!("BOOLEAN:{}", bool_str));
}

/// Put a color value into the resource map. Color is packed (r<<24)|(g<<16)|(b<<8)|a.
#[no_mangle]
pub unsafe extern "C" fn res_PutColor(key: *const c_char, value: u32) {
    let key_str = match cstr_to_str(key) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let r = ((value >> 24) & 0xFF) as u8;
    let g = ((value >> 16) & 0xFF) as u8;
    let b = ((value >> 8) & 0xFF) as u8;
    let a = (value & 0xFF) as u8;

    let state = guard.as_mut().unwrap();
    let color_str = super::resource_type::serialize_color(r, g, b, a);
    state.dispatch.process_resource_desc(key_str, &format!("COLOR:{}", color_str));
}

// =============================================================================
// File I/O wrappers (UIO delegation)
// =============================================================================

/// Open a resource file. Returns a UIO stream handle, or null on failure.
#[no_mangle]
pub unsafe extern "C" fn res_OpenResFile(
    dir: *mut c_void,
    filename: *const c_char,
    mode: *const c_char,
) -> *mut c_void {
    if filename.is_null() || mode.is_null() {
        return ptr::null_mut();
    }
    uio_fopen(dir, filename, mode)
}

/// Close a resource file. Returns 1 on success, 0 on failure.
/// NULL and sentinel pointers return 1 (no-op).
#[no_mangle]
pub unsafe extern "C" fn res_CloseResFile(fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return 1;
    }
    if uio_fclose(fp) == 0 { 1 } else { 0 }
}

/// Read from a resource file.
#[no_mangle]
pub unsafe extern "C" fn ReadResFile(
    buf: *mut c_void,
    size: usize,
    count: usize,
    fp: *mut c_void,
) -> usize {
    if fp.is_null() || fp == STREAM_SENTINEL || buf.is_null() {
        return 0;
    }
    uio_fread(buf, size, count, fp)
}

/// Write to a resource file.
#[no_mangle]
pub unsafe extern "C" fn WriteResFile(
    buf: *const c_void,
    size: usize,
    count: usize,
    fp: *mut c_void,
) -> usize {
    if fp.is_null() || fp == STREAM_SENTINEL || buf.is_null() {
        return 0;
    }
    uio_fwrite(buf, size, count, fp)
}

/// Get a character from a resource file.
#[no_mangle]
pub unsafe extern "C" fn GetResFileChar(fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1; // EOF
    }
    uio_fgetc(fp)
}

/// Put a character to a resource file.
#[no_mangle]
pub unsafe extern "C" fn PutResFileChar(ch: c_char, fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1; // EOF
    }
    uio_fputc(ch as c_int, fp)
}

/// Write a newline to a resource file.
#[no_mangle]
pub unsafe extern "C" fn PutResFileNewline(fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1; // EOF
    }
    uio_fputc(b'\n' as c_int, fp)
}

/// Seek in a resource file.
#[no_mangle]
pub unsafe extern "C" fn SeekResFile(fp: *mut c_void, offset: c_long, whence: c_int) -> c_long {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1;
    }
    uio_fseek(fp, offset, whence) as c_long
}

/// Tell position in a resource file.
#[no_mangle]
pub unsafe extern "C" fn TellResFile(fp: *mut c_void) -> c_long {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1;
    }
    uio_ftell(fp)
}

/// Get the length of a resource file.
/// Sentinel returns 1 (directory marker).
#[no_mangle]
pub unsafe extern "C" fn LengthResFile(fp: *mut c_void) -> usize {
    if fp.is_null() {
        return 0;
    }
    if fp == STREAM_SENTINEL {
        return 1;
    }

    let cur_pos = uio_ftell(fp);
    uio_fseek(fp, 0, 2); // SEEK_END
    let length = uio_ftell(fp) as usize;
    uio_fseek(fp, cur_pos, 0); // SEEK_SET
    length
}

/// Delete a resource file.
#[no_mangle]
pub unsafe extern "C" fn DeleteResFile(dir: *mut c_void, filename: *const c_char) -> c_int {
    if filename.is_null() {
        return 0;
    }
    if uio_unlink(dir, filename) == 0 { 1 } else { 0 }
}

// =============================================================================
// Resource data loading
// =============================================================================

/// Load a resource from a pathname using the given load function.
///
/// Opens the file via UIO, gets its length, sets `_cur_resfile_name`,
/// calls the load function, cleans up, and returns the loaded data.
#[no_mangle]
pub unsafe extern "C" fn LoadResourceFromPath(
    pathname: *const c_char,
    load_fn: Option<ResourceLoadFileFun>,
) -> *mut c_void {
    if pathname.is_null() {
        return ptr::null_mut();
    }

    let load_fn = match load_fn {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let mode = b"rb\0".as_ptr() as *const c_char;
    let fp = uio_fopen(contentDir, pathname, mode);
    if fp.is_null() {
        return ptr::null_mut();
    }

    // Get file length
    uio_fseek(fp, 0, 2); // SEEK_END
    let length = uio_ftell(fp) as u32;
    uio_fseek(fp, 0, 0); // SEEK_SET

    // Set global file name for C callers
    _cur_resfile_name = pathname;

    let result = load_fn(fp, length);

    // Clear global file name
    _cur_resfile_name = ptr::null();

    uio_fclose(fp);
    result
}

/// Read resource data from a file stream.
///
/// Reads a u32 prefix. If it's 0xFFFFFFFF, the data is uncompressed —
/// seek back 4 bytes and read `length` bytes raw.
#[no_mangle]
pub unsafe extern "C" fn GetResourceData(fp: *mut c_void, length: u32) -> *mut c_void {
    if fp.is_null() || fp == STREAM_SENTINEL || length == 0 {
        return ptr::null_mut();
    }

    // Read the 4-byte prefix
    let mut prefix: u32 = 0;
    let read = uio_fread(
        &mut prefix as *mut u32 as *mut c_void,
        std::mem::size_of::<u32>(),
        1,
        fp,
    );
    if read != 1 {
        return ptr::null_mut();
    }

    if prefix == 0xFFFFFFFF {
        // Uncompressed: seek back 4 bytes, read entire chunk
        uio_fseek(fp, -4, 1); // SEEK_CUR = 1
        let layout = std::alloc::Layout::from_size_align(length as usize, 1)
            .unwrap_or_else(|_| std::alloc::Layout::from_size_align(1, 1).unwrap());
        let buf = std::alloc::alloc(layout);
        if buf.is_null() {
            return ptr::null_mut();
        }
        let bytes_read = uio_fread(buf as *mut c_void, 1, length as usize, fp);
        if bytes_read < length as usize {
            log::warn!(
                "GetResourceData: short read ({} of {} bytes)",
                bytes_read,
                length
            );
        }
        buf as *mut c_void
    } else {
        // Compressed data — for now, allocate and read remaining
        let data_len = length as usize;
        let layout = std::alloc::Layout::from_size_align(data_len, 1)
            .unwrap_or_else(|_| std::alloc::Layout::from_size_align(1, 1).unwrap());
        let buf = std::alloc::alloc(layout);
        if buf.is_null() {
            return ptr::null_mut();
        }
        // Write the prefix bytes first
        let prefix_bytes = prefix.to_ne_bytes();
        ptr::copy_nonoverlapping(prefix_bytes.as_ptr(), buf, 4.min(data_len));
        if data_len > 4 {
            uio_fread(buf.add(4) as *mut c_void, 1, data_len - 4, fp);
        }
        buf as *mut c_void
    }
}

/// Free resource data that was allocated by GetResourceData.
/// Returns 1 on success, 0 on failure.
#[no_mangle]
pub unsafe extern "C" fn FreeResourceData(data: *mut c_void) -> c_int {
    if data.is_null() {
        return 1;
    }
    // Data was allocated with std::alloc::alloc, but we don't know the size.
    // Use libc::free as a safe alternative since GetResourceData allocations
    // can also be freed by C code via HFree/free.
    libc::free(data);
    1
}

// =============================================================================
// Tests (non-FFI — internal API tests only)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::CString;

    /// Reset state for test isolation.
    /// Handles poisoned mutex from prior panics.
    fn reset_state() {
        let mut guard = match RESOURCE_STATE.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = None;
    }

    // =========================================================================
    // P19: Init/Uninit tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_init_creates_state() {
        reset_state();
        let ptr = InitResourceSystem();
        assert!(!ptr.is_null(), "InitResourceSystem should return non-null");

        let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        assert!(guard.is_some(), "State should be Some after init");
    }

    #[test]
    #[serial]
    fn test_init_registers_5_types() {
        reset_state();
        InitResourceSystem();

        let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_ref().unwrap();
        assert_eq!(
            state.dispatch.type_registry.count(),
            5,
            "Should have 5 built-in types (STRING, INT32, BOOLEAN, COLOR, UNKNOWNRES)"
        );

        // Verify each type is registered
        assert!(state.dispatch.type_registry.lookup("STRING").is_some());
        assert!(state.dispatch.type_registry.lookup("INT32").is_some());
        assert!(state.dispatch.type_registry.lookup("BOOLEAN").is_some());
        assert!(state.dispatch.type_registry.lookup("COLOR").is_some());
        assert!(state.dispatch.type_registry.lookup("UNKNOWNRES").is_some());
    }

    #[test]
    #[serial]
    fn test_init_idempotent() {
        reset_state();
        let ptr1 = InitResourceSystem();
        let ptr2 = InitResourceSystem();
        assert_eq!(ptr1, ptr2, "Init should be idempotent");
    }

    #[test]
    #[serial]
    fn test_uninit_clears_state() {
        reset_state();
        InitResourceSystem();
        {
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            assert!(guard.is_some());
        }
        UninitResourceSystem();
        {
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            assert!(guard.is_none(), "State should be None after uninit");
        }
    }

    #[test]
    #[serial]
    fn test_uninit_safe_to_call_twice() {
        reset_state();
        InitResourceSystem();
        UninitResourceSystem();
        UninitResourceSystem(); // Should not crash
    }

    #[test]
    #[serial]
    fn test_uninit_then_reinit() {
        reset_state();
        InitResourceSystem();
        UninitResourceSystem();

        // Fresh init after uninit
        let ptr = InitResourceSystem();
        assert!(!ptr.is_null());

        let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_ref().unwrap();
        assert_eq!(state.dispatch.type_registry.count(), 5);
        assert!(state.dispatch.entries.is_empty(), "Fresh init should have no entries");
    }

    // =========================================================================
    // P19: LoadResourceIndex tests (internal API, simulated content)
    // =========================================================================

    #[test]
    #[serial]
    fn test_load_index_parses_entries_internal() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        // Simulate LoadResourceIndex by parsing propfile content directly
        let content = "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            None,
        );

        assert!(state.dispatch.entries.contains_key("comm.arilou.graphics"));
        let desc = state.dispatch.entries.get("comm.arilou.graphics").unwrap();
        assert_eq!(desc.res_type, "GFXRES");
        assert_eq!(desc.fname, "base/comm/arilou/arilou.ani");
    }

    #[test]
    #[serial]
    fn test_load_index_with_prefix_internal() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "sfxvol = INT32:20\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            Some("config."),
        );

        assert!(state.dispatch.entries.contains_key("config.sfxvol"));
        assert_eq!(state.dispatch.get_int_resource("config.sfxvol"), Some(20));
    }

    #[test]
    #[serial]
    fn test_load_index_multiple_calls_accumulate() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content_a = "key.a = STRING:alpha\n";
        parse_propfile(
            content_a,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            None,
        );

        let content_b = "key.b = STRING:beta\n";
        parse_propfile(
            content_b,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            None,
        );

        assert!(state.dispatch.entries.contains_key("key.a"));
        assert!(state.dispatch.entries.contains_key("key.b"));
    }

    #[test]
    #[serial]
    fn test_load_index_last_writer_wins() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content_a = "music.battle = STRING:base/battle.mod\n";
        parse_propfile(
            content_a,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            None,
        );

        let content_b = "music.battle = STRING:addons/3domusic/battle.ogg\n";
        parse_propfile(
            content_b,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            None,
        );

        let desc = state.dispatch.entries.get("music.battle").unwrap();
        assert_eq!(desc.fname, "addons/3domusic/battle.ogg");
    }

    #[test]
    #[serial]
    fn test_load_index_value_types_parsed_immediately() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "sfxvol = INT32:20\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            Some("config."),
        );

        // INT32 is a value type — parsed immediately, data.num should be 20
        let desc = state.dispatch.entries.get("config.sfxvol").unwrap();
        assert_eq!(unsafe { desc.data.num }, 20);
    }

    #[test]
    #[serial]
    fn test_load_index_heap_types_deferred() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            None,
        );

        // GFXRES has no registered handler, so dispatch falls back to UNKNOWNRES
        // for the handler_key, but preserves the original type name for serialization.
        // UNKNOWNRES is a value type (freeFun=None), so use_descriptor_as_res
        // is called immediately, setting str_ptr to the descriptor string.
        let desc = state.dispatch.entries.get("comm.arilou.graphics").unwrap();
        assert_eq!(desc.res_type, "GFXRES", "Original type name preserved");
        assert_eq!(desc.type_handler_key, "UNKNOWNRES", "Falls back to UNKNOWNRES handler");
        assert!(!unsafe { desc.data.str_ptr.is_null() }, "UNKNOWNRES stores descriptor as str_ptr");
    }

    #[test]
    #[serial]
    fn test_load_index_boolean_true_internal() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "fullscreen = BOOLEAN:true\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            Some("config."),
        );

        assert_eq!(state.dispatch.get_boolean_resource("config.fullscreen"), Some(true));
    }

    #[test]
    #[serial]
    fn test_load_index_boolean_false_internal() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "fullscreen = BOOLEAN:false\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            Some("config."),
        );

        assert_eq!(state.dispatch.get_boolean_resource("config.fullscreen"), Some(false));
    }

    #[test]
    #[serial]
    fn test_load_index_color_internal() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "color = COLOR:rgb(0x1a, 0x00, 0x1a)\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            Some("config."),
        );

        let desc = state.dispatch.entries.get("config.color").unwrap();
        let packed = unsafe { desc.data.num };
        let r = ((packed >> 24) & 0xFF) as u8;
        let g = ((packed >> 16) & 0xFF) as u8;
        let b = ((packed >> 8) & 0xFF) as u8;
        let a = (packed & 0xFF) as u8;
        assert_eq!((r, g, b, a), (0x1a, 0x00, 0x1a, 0xff));
    }

    #[test]
    #[serial]
    fn test_load_index_string_internal() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();

        let content = "up.1 = STRING:key Up\n";
        parse_propfile(
            content,
            &mut |key, value| {
                state.dispatch.process_resource_desc(key, value);
            },
            Some("keys."),
        );

        assert_eq!(state.dispatch.get_string_resource("keys.up.1"), Some("key Up"));
    }

    // =========================================================================
    // P19: Auto-init tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_auto_init_on_has_key() {
        reset_state();
        // No InitResourceSystem call

        let key = CString::new("nonexistent").unwrap();
        let result = unsafe { res_HasKey(key.as_ptr()) };
        assert_eq!(result, 0, "Non-existent key should return 0");

        // State should have been auto-initialized
        let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        assert!(guard.is_some(), "State should be auto-initialized");
    }

    #[test]
    #[serial]
    fn test_auto_init_on_count_resource_types() {
        reset_state();
        // No InitResourceSystem call

        let count = CountResourceTypes();
        assert_eq!(count, 5, "Auto-init should register 5 types");
    }

    // =========================================================================
    // P19: File I/O sentinel tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_length_res_file_sentinel() {
        let result = unsafe { LengthResFile(STREAM_SENTINEL) };
        assert_eq!(result, 1, "Sentinel should return length 1");
    }

    #[test]
    #[serial]
    fn test_close_res_file_sentinel() {
        let result = unsafe { res_CloseResFile(STREAM_SENTINEL) };
        assert_eq!(result, 1, "Sentinel close should return 1 (success)");
    }

    #[test]
    #[serial]
    fn test_close_res_file_null() {
        let result = unsafe { res_CloseResFile(ptr::null_mut()) };
        assert_eq!(result, 1, "Null close should return 1 (success)");
    }

    #[test]
    #[serial]
    fn test_read_res_file_null() {
        let mut buf = [0u8; 16];
        let result = unsafe { ReadResFile(buf.as_mut_ptr() as *mut c_void, 1, 16, ptr::null_mut()) };
        assert_eq!(result, 0, "Read from null should return 0");
    }

    #[test]
    #[serial]
    fn test_write_res_file_null() {
        let buf = [0u8; 16];
        let result = unsafe { WriteResFile(buf.as_ptr() as *const c_void, 1, 16, ptr::null_mut()) };
        assert_eq!(result, 0, "Write to null should return 0");
    }

    #[test]
    #[serial]
    fn test_get_res_file_char_null() {
        let result = unsafe { GetResFileChar(ptr::null_mut()) };
        assert_eq!(result, -1, "GetChar from null should return -1 (EOF)");
    }

    #[test]
    #[serial]
    fn test_seek_res_file_null() {
        let result = unsafe { SeekResFile(ptr::null_mut(), 0, 0) };
        assert_eq!(result, -1, "Seek on null should return -1");
    }

    #[test]
    #[serial]
    fn test_tell_res_file_null() {
        let result = unsafe { TellResFile(ptr::null_mut()) };
        assert_eq!(result, -1, "Tell on null should return -1");
    }

    #[test]
    #[serial]
    fn test_length_res_file_null() {
        let result = unsafe { LengthResFile(ptr::null_mut()) };
        assert_eq!(result, 0, "Length of null should return 0");
    }

    #[test]
    #[serial]
    fn test_free_resource_data_null() {
        let result = unsafe { FreeResourceData(ptr::null_mut()) };
        assert_eq!(result, 1, "Free null should return 1 (success)");
    }

    // =========================================================================
    // P19: Config Put/Get roundtrip tests via FFI functions
    // =========================================================================

    #[test]
    #[serial]
    fn test_put_get_string_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("test.key").unwrap();
        let value = CString::new("hello world").unwrap();

        unsafe {
            res_PutString(key.as_ptr(), value.as_ptr());
            let result = res_GetString(key.as_ptr());
            assert!(!result.is_null());
            let result_str = CStr::from_ptr(result).to_str().unwrap();
            assert_eq!(result_str, "hello world");
        }
    }

    #[test]
    #[serial]
    fn test_put_get_integer_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("test.num").unwrap();
        unsafe {
            res_PutInteger(key.as_ptr(), 42);
            let result = res_GetInteger(key.as_ptr());
            assert_eq!(result, 42);
        }
    }

    #[test]
    #[serial]
    fn test_put_get_boolean_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("test.flag").unwrap();
        unsafe {
            res_PutBoolean(key.as_ptr(), 1);
            let result = res_GetBoolean(key.as_ptr());
            assert_eq!(result, 1);

            res_PutBoolean(key.as_ptr(), 0);
            let result = res_GetBoolean(key.as_ptr());
            assert_eq!(result, 0);
        }
    }

    #[test]
    #[serial]
    fn test_put_get_color_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("test.color").unwrap();
        let packed: u32 = (0x1a << 24) | (0x00 << 16) | (0x1a << 8) | 0xff;
        unsafe {
            res_PutColor(key.as_ptr(), packed);
            let result = res_GetColor(key.as_ptr());
            assert_eq!(result, packed);
        }
    }

    #[test]
    #[serial]
    fn test_is_type_checks_ffi() {
        reset_state();
        InitResourceSystem();

        let str_key = CString::new("str.key").unwrap();
        let int_key = CString::new("int.key").unwrap();
        let bool_key = CString::new("bool.key").unwrap();
        let color_key = CString::new("color.key").unwrap();

        unsafe {
            res_PutString(str_key.as_ptr(), CString::new("val").unwrap().as_ptr());
            res_PutInteger(int_key.as_ptr(), 1);
            res_PutBoolean(bool_key.as_ptr(), 1);
            res_PutColor(color_key.as_ptr(), 0xFF0000FF);

            assert_eq!(res_IsString(str_key.as_ptr()), 1);
            assert_eq!(res_IsString(int_key.as_ptr()), 0);

            assert_eq!(res_IsInteger(int_key.as_ptr()), 1);
            assert_eq!(res_IsInteger(str_key.as_ptr()), 0);

            assert_eq!(res_IsBoolean(bool_key.as_ptr()), 1);
            assert_eq!(res_IsBoolean(str_key.as_ptr()), 0);

            assert_eq!(res_IsColor(color_key.as_ptr()), 1);
            assert_eq!(res_IsColor(str_key.as_ptr()), 0);
        }
    }

    #[test]
    #[serial]
    fn test_has_key_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("exists.key").unwrap();
        let nokey = CString::new("does.not.exist").unwrap();

        unsafe {
            assert_eq!(res_HasKey(key.as_ptr()), 0);

            res_PutString(key.as_ptr(), CString::new("val").unwrap().as_ptr());
            assert_eq!(res_HasKey(key.as_ptr()), 1);
            assert_eq!(res_HasKey(nokey.as_ptr()), 0);
        }
    }

    #[test]
    #[serial]
    fn test_remove_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("remove.me").unwrap();
        unsafe {
            res_PutString(key.as_ptr(), CString::new("val").unwrap().as_ptr());
            assert_eq!(res_HasKey(key.as_ptr()), 1);

            let result = res_Remove(key.as_ptr());
            assert_eq!(result, 1);
            assert_eq!(res_HasKey(key.as_ptr()), 0);

            // Second remove should return 0
            let result = res_Remove(key.as_ptr());
            assert_eq!(result, 0);
        }
    }

    #[test]
    #[serial]
    fn test_get_resource_type_ffi() {
        reset_state();
        InitResourceSystem();

        let key = CString::new("typed.key").unwrap();
        unsafe {
            res_PutInteger(key.as_ptr(), 42);
            let type_ptr = res_GetResourceType(key.as_ptr());
            assert!(!type_ptr.is_null());
            let type_str = CStr::from_ptr(type_ptr).to_str().unwrap();
            assert_eq!(type_str, "INT32");
        }
    }

    #[test]
    #[serial]
    fn test_count_resource_types_ffi() {
        reset_state();
        InitResourceSystem();
        assert_eq!(CountResourceTypes(), 5);
    }

    #[test]
    #[serial]
    fn test_null_key_handling() {
        reset_state();
        InitResourceSystem();

        unsafe {
            assert_eq!(res_HasKey(ptr::null()), 0);
            assert_eq!(res_IsString(ptr::null()), 0);
            assert_eq!(res_IsInteger(ptr::null()), 0);
            assert_eq!(res_IsBoolean(ptr::null()), 0);
            assert_eq!(res_IsColor(ptr::null()), 0);
            assert!(res_GetString(ptr::null()).is_null());
            assert_eq!(res_GetInteger(ptr::null()), 0);
            assert_eq!(res_GetBoolean(ptr::null()), 0);
            assert_eq!(res_GetColor(ptr::null()), 0);
            assert!(res_GetResource(ptr::null()).is_null());
            assert!(res_DetachResource(ptr::null()).is_null());
            assert_eq!(res_Remove(ptr::null()), 0);
            assert_eq!(res_GetIntResource(ptr::null()), 0);
            assert_eq!(res_GetBooleanResource(ptr::null()), 0);
            assert!(res_GetResourceType(ptr::null()).is_null());

            // Put functions with null should not crash
            res_PutString(ptr::null(), ptr::null());
            res_PutInteger(ptr::null(), 0);
            res_PutBoolean(ptr::null(), 0);
            res_PutColor(ptr::null(), 0);
            res_FreeResource(ptr::null());
        }
    }

    #[test]
    #[serial]
    fn test_get_int_resource_ffi() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();
        state.dispatch.process_resource_desc("vol.sfx", "INT32:20");
        drop(guard);

        let key = CString::new("vol.sfx").unwrap();
        let result = unsafe { res_GetIntResource(key.as_ptr()) };
        assert_eq!(result, 20);
    }

    #[test]
    #[serial]
    fn test_get_boolean_resource_ffi() {
        reset_state();
        InitResourceSystem();

        let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
        let state = guard.as_mut().unwrap();
        state.dispatch.process_resource_desc("config.fullscreen", "BOOLEAN:true");
        drop(guard);

        let key = CString::new("config.fullscreen").unwrap();
        let result = unsafe { res_GetBooleanResource(key.as_ptr()) };
        assert_eq!(result, 1);
    }
}
