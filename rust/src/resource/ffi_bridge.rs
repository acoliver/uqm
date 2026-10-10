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

use super::dispatch::{FullResourceDesc, ResourceDispatch};
use super::ffi_types::{
    ResourceData, ResourceFreeFun, ResourceLoadFileFun, ResourceLoadFun, ResourceStringFun,
};
#[cfg(test)]
use super::propfile::parse_propfile;
use super::propfile::parse_propfile_bytes;
use super::type_registry;
#[cfg(not(test))]
use crate::io::ffi::{uio_DirHandle, uio_Stream};

// =============================================================================
// UIO extern imports — C file I/O layer
// =============================================================================

#[cfg(not(test))]
extern "C" {
    #[link_name = "uio_fopen"]
    fn uio_fopen_raw(
        dir: *mut uio_DirHandle,
        path: *const c_char,
        mode: *const c_char,
    ) -> *mut uio_Stream;
    #[link_name = "uio_fclose"]
    fn uio_fclose_raw(fp: *mut uio_Stream) -> c_int;
    fn uio_fread(buf: *mut c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fwrite(buf: *const c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fseek(fp: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn uio_ftell(fp: *mut c_void) -> c_long;
    fn uio_fgetc(fp: *mut c_void) -> c_int;
    fn uio_fputc(c: c_int, fp: *mut c_void) -> c_int;
    #[link_name = "uio_unlink"]
    fn uio_unlink_raw(dir: *mut uio_DirHandle, path: *const c_char) -> c_int;
    // @plan PLAN-20260314-RESOURCE.P08
    // @requirement REQ-RES-FILE-003
    fn uio_stat(dir: *mut c_void, path: *const c_char, stat_buf: *mut libc::stat) -> c_int;

    static contentDir: *mut c_void;

    // C subsystem type registration functions — called from InitResourceSystem
    fn InstallGraphicResTypes() -> c_int;
    fn InstallStringTableResType() -> c_int;
    #[cfg(not(feature = "audio_heart"))]
    fn InstallAudioResTypes() -> c_int;
    fn InstallVideoResType() -> c_int;
    fn InstallCodeResType() -> c_int;
}

#[cfg(not(test))]
unsafe fn uio_fopen(dir: *mut c_void, path: *const c_char, mode: *const c_char) -> *mut c_void {
    uio_fopen_raw(dir.cast(), path, mode).cast()
}

#[cfg(not(test))]
unsafe fn uio_fclose(fp: *mut c_void) -> c_int {
    uio_fclose_raw(fp.cast())
}

#[cfg(not(test))]
unsafe fn uio_unlink(dir: *mut c_void, path: *const c_char) -> c_int {
    uio_unlink_raw(dir.cast(), path)
}

// When audio_heart feature is enabled, register Rust audio resource callbacks
// instead of C's InstallAudioResTypes.
#[cfg(all(not(test), feature = "audio_heart"))]
use crate::sound::heart_ffi::{DestroyMusic, DestroySound, LoadMusicFile, LoadSoundFile};

#[cfg(all(not(test), feature = "audio_heart"))]
unsafe extern "C" fn rust_load_sound_bank(pathname: *const c_char, resdata: *mut ResourceData) {
    let name = if pathname.is_null() {
        "<null>"
    } else {
        std::ffi::CStr::from_ptr(pathname)
            .to_str()
            .unwrap_or("<bad utf8>")
    };
    eprintln!("[audio_heart] rust_load_sound_bank called: {}", name);
    (*resdata).ptr = LoadSoundFile(pathname);
    eprintln!(
        "[audio_heart] rust_load_sound_bank result: {:?}",
        (*resdata).ptr
    );
}

#[cfg(all(not(test), feature = "audio_heart"))]
unsafe extern "C" fn rust_free_sound_bank(data: *mut c_void) -> c_int {
    DestroySound(data);
    1
}

#[cfg(all(not(test), feature = "audio_heart"))]
unsafe extern "C" fn rust_load_music(pathname: *const c_char, resdata: *mut ResourceData) {
    let name = if pathname.is_null() {
        "<null>"
    } else {
        std::ffi::CStr::from_ptr(pathname)
            .to_str()
            .unwrap_or("<bad utf8>")
    };
    eprintln!("[audio_heart] rust_load_music called: {}", name);
    (*resdata).ptr = LoadMusicFile(pathname);
    eprintln!("[audio_heart] rust_load_music result: {:?}", (*resdata).ptr);
}

#[cfg(all(not(test), feature = "audio_heart"))]
unsafe extern "C" fn rust_free_music(data: *mut c_void) -> c_int {
    DestroyMusic(data);
    1
}

// In tests the same entry points run against the Rust UIO implementation
// that ships in the binary, so file-backed paths see real files.
#[cfg(test)]
use crate::io::uio_bridge as real_uio;

#[cfg(test)]
unsafe fn uio_fopen(dir: *mut c_void, path: *const c_char, mode: *const c_char) -> *mut c_void {
    real_uio::uio_fopen(dir.cast(), path, mode).cast()
}
#[cfg(test)]
unsafe fn uio_fclose(fp: *mut c_void) -> c_int {
    real_uio::uio_fclose(fp.cast())
}
#[cfg(test)]
unsafe fn uio_fread(buf: *mut c_void, size: usize, count: usize, fp: *mut c_void) -> usize {
    real_uio::uio_fread(buf, size, count, fp.cast())
}
#[cfg(test)]
unsafe fn uio_fwrite(buf: *const c_void, size: usize, count: usize, fp: *mut c_void) -> usize {
    real_uio::uio_fwrite(buf, size, count, fp.cast())
}
#[cfg(test)]
unsafe fn uio_fseek(fp: *mut c_void, offset: c_long, whence: c_int) -> c_int {
    real_uio::uio_fseek(fp.cast(), offset, whence)
}
#[cfg(test)]
unsafe fn uio_ftell(fp: *mut c_void) -> c_long {
    real_uio::uio_ftell(fp.cast())
}
#[cfg(test)]
unsafe fn uio_fgetc(fp: *mut c_void) -> c_int {
    real_uio::uio_fgetc(fp.cast())
}
#[cfg(test)]
unsafe fn uio_fputc(c: c_int, fp: *mut c_void) -> c_int {
    real_uio::uio_fputc(c, fp.cast())
}
#[cfg(test)]
unsafe fn uio_unlink(dir: *mut c_void, path: *const c_char) -> c_int {
    real_uio::uio_unlink(dir.cast(), path)
}
#[cfg(test)]
unsafe fn uio_stat(dir: *mut c_void, path: *const c_char, stat_buf: *mut libc::stat) -> c_int {
    real_uio::uio_stat(dir.cast(), path, stat_buf)
}

#[cfg(test)]
#[allow(
    non_upper_case_globals,
    reason = "test stub mirrors the legacy C global symbol"
)]
static mut contentDir: *mut c_void = ptr::null_mut();

// =============================================================================
// Global state
// =============================================================================

/// Holds the ResourceDispatch and any CStrings that must outlive their pointers.
struct ResourceState {
    dispatch: ResourceDispatch,
    /// Type names handed to C by res_GetResourceType, by type — kept alive
    /// for pointer stability
    type_cache: std::collections::HashMap<String, CString>,
    /// Whether C subsystem type registration has been performed
    c_types_registered: bool,
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

/// Initialize a fresh ResourceState with the 5 built-in value types
/// (`UNKNOWNRES` is installed by `ResourceDispatch::new`).
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

    ResourceState {
        dispatch,
        type_cache: std::collections::HashMap::new(),
        c_types_registered: false,
    }
}

/// Ensure state is initialized (auto-init pattern). Returns the lock guard.
fn ensure_init(guard: &mut std::sync::MutexGuard<'_, Option<ResourceState>>) {
    if guard.is_none() {
        **guard = Some(create_initial_state());
    }
}

/// The bytes of a C string key, which need not be UTF-8; `None` for NULL.
unsafe fn cstr_bytes<'a>(ptr: *const c_char) -> Option<&'a [u8]> {
    (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_bytes())
}

/// Store `descriptor` under `key` as a `put` or an index line would. A
/// descriptor that cannot become an entry is reported and the call goes on,
/// as C does.
fn put_descriptor(state: &mut ResourceState, key: &[u8], descriptor: &[u8]) {
    if let Err(err) = state.dispatch.process_resource_desc(key, descriptor) {
        log::warn!(
            "Resource '{}' not stored: {}",
            String::from_utf8_lossy(key),
            err
        );
    }
}

/// Helper: safely convert a *const c_char to &str, returning None on null/invalid UTF-8.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

/// The numeric payload of `key` when it is stored as `type_name`; `None` for
/// a missing key or any other type, as the C typed getters answer.
///
/// `ResourceData` is a union: reading `num` from a STRING entry would hand
/// back part of a pointer.
fn typed_number(state: &ResourceState, key: &[u8], type_name: &str) -> Option<u32> {
    let entry = state.dispatch.entry(key)?;
    if entry.res_type != type_name {
        return None;
    }
    // SAFETY: value types store their parsed number in `num` when loaded.
    Some(unsafe { entry.data.num })
}

// =============================================================================
// Lifecycle
// =============================================================================

/// Initialize the resource system. Registers 5 built-in value types.
/// Returns an opaque pointer to the state (non-null on success).
/// Idempotent: if already initialized, returns the existing pointer.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn InitResourceSystem() -> *mut c_void {
    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.is_none() {
        *guard = Some(create_initial_state());
    }

    // Check if C subsystem types need registration
    let needs_c_registration = guard.as_ref().is_some_and(|s| !s.c_types_registered);
    if needs_c_registration {
        // Mark as registered BEFORE dropping lock to prevent re-entry
        if let Some(ref mut state) = *guard {
            state.c_types_registered = true;
        }
        // Must drop the lock before calling C subsystem registration,
        // because they will call InstallResTypeVectors which needs the lock
        drop(guard);

        // Register the 9 heap types via C subsystem init functions
        // (same calls as C InitResourceSystem in resinit.c)
        #[cfg(not(test))]
        unsafe {
            InstallGraphicResTypes();
            InstallStringTableResType();
            #[cfg(feature = "audio_heart")]
            {
                eprintln!("[audio_heart] Registering Rust SNDRES/MUSICRES callbacks");
                InstallResTypeVectors(
                    c"SNDRES".as_ptr() as *const c_char,
                    Some(rust_load_sound_bank),
                    Some(rust_free_sound_bank),
                    None,
                );
                InstallResTypeVectors(
                    c"MUSICRES".as_ptr() as *const c_char,
                    Some(rust_load_music),
                    Some(rust_free_music),
                    None,
                );
            }
            #[cfg(not(feature = "audio_heart"))]
            {
                InstallAudioResTypes();
            }
            InstallVideoResType();
            InstallCodeResType();
        }
    }

    // Return a non-null sentinel (not actually dereferenceable by C)
    std::ptr::dangling_mut::<c_void>()
}

/// Uninitialize the resource system. Drops all state.
/// Safe to call multiple times.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn UninitResourceSystem() {
    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.is_none() {
        log::warn!("UninitResourceSystem called when not initialized");
    }

    // @plan PLAN-20260314-RESOURCE.P06
    // @requirement REQ-RES-LIFE-004
    if let Some(ref mut state) = *guard {
        state.dispatch.cleanup_all_entries();
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
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
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
    let mode = c"rt".as_ptr() as *const c_char;
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

    // The index is bytes, not text: a Latin-1 value on one line must not cost
    // the rest of the file.
    let prefix_bytes = (!prefix.is_null()).then(|| CStr::from_ptr(prefix).to_bytes());

    let state = guard.as_mut().unwrap();
    parse_propfile_bytes(
        &buf,
        &mut |key, value| put_descriptor(state, key, value),
        prefix_bytes,
    );
}

/// The `key = TYPE:text` line `SaveResourceIndex` writes for `desc` under
/// the name `written_key`; `None` for an entry whose handler has no
/// `toString` (the heap types and `UNKNOWNRES`).
///
/// @plan PLAN-20260314-RESOURCE.P07
/// @requirement REQ-RES-IDX-005
fn index_line(
    dispatch: &ResourceDispatch,
    written_key: &[u8],
    desc: &FullResourceDesc,
) -> Option<Vec<u8>> {
    let to_string_fn = dispatch.type_registry.lookup(&desc.res_type)?.to_string?;

    // STRING writes the owned descriptor bytes as loaded: the `str_ptr` may
    // dangle after a type change, and the bytes may not be UTF-8.
    let mut line = written_key.to_vec();
    line.extend_from_slice(b" = ");
    line.extend_from_slice(desc.res_type.as_bytes());
    line.push(b':');
    if desc.res_type == "STRING" {
        line.extend_from_slice(desc.descriptor_bytes());
    } else {
        let mut buf = [0u8; 256];
        let mut data_copy: ResourceData = unsafe { std::ptr::read(&desc.data) };
        unsafe {
            to_string_fn(&mut data_copy, buf.as_mut_ptr() as *mut c_char, 256);
        }
        let len = buf.iter().position(|&b| b == 0).unwrap_or(256);
        line.extend_from_slice(&buf[..len]);
    }
    line.push(b'\n');
    Some(line)
}

/// Save resource index entries matching `root` prefix to a file via UIO.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
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
    let root = cstr_bytes(root);

    // Entries under the root, in key order so the file is reproducible.
    let mut entries: Vec<(&[u8], &FullResourceDesc)> = state
        .dispatch
        .entries
        .iter()
        .map(|(key, desc)| (&key[..], desc))
        .filter(|(key, _)| root.is_none_or(|prefix| key.starts_with(prefix)))
        .collect();
    entries.sort_unstable_by_key(|&(key, _)| key);

    let lines: Vec<Vec<u8>> = entries
        .into_iter()
        .filter_map(|(key, desc)| {
            let written_key = match root {
                Some(prefix) if strip_root != 0 => &key[prefix.len()..],
                _ => key,
            };
            index_line(&state.dispatch, written_key, desc)
        })
        .collect();

    // Open output file
    let mode = c"wt".as_ptr() as *const c_char;
    let fp = uio_fopen(dir, file, mode);
    if fp.is_null() {
        log::warn!("SaveResourceIndex: failed to open file for writing");
        return;
    }

    for line in &lines {
        uio_fwrite(line.as_ptr() as *const c_void, 1, line.len(), fp);
    }

    uio_fclose(fp);
}

// =============================================================================
// Type registration
// =============================================================================

/// Install resource type vectors (load/free/toString) for a named type.
/// Returns 1 on success, 0 on failure.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
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
    if state
        .dispatch
        .type_registry
        .install(type_name, load_fun, free_fun, string_fun)
    {
        1
    } else {
        0
    }
}

// =============================================================================
// Resource access
// =============================================================================

/// Get a resource, lazy-loading if needed. Returns the data pointer.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetResource(key: *const c_char) -> *mut c_void {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_DetachResource(key: *const c_char) -> *mut c_void {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_FreeResource(key: *const c_char) {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_Remove(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    if state.dispatch.remove_resource(key_str) {
        1
    } else {
        0
    }
}

// =============================================================================
// Value access
// =============================================================================

/// Get an integer value from an INT32 resource entry.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetIntResource(key: *const c_char) -> u32 {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetBooleanResource(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    if state
        .dispatch
        .get_boolean_resource(key_str)
        .unwrap_or(false)
    {
        1
    } else {
        0
    }
}

/// Get the type name for a resource entry.
/// Returns a pointer to a C string that lives as long as the state.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetResourceType(key: *const c_char) -> *const c_char {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return ptr::null(),
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let ResourceState {
        dispatch,
        type_cache,
        ..
    } = guard.as_mut().unwrap();
    let Some(type_name) = dispatch.get_resource_type(key_str) else {
        return ptr::null();
    };

    type_cache
        .entry(type_name.to_string())
        .or_insert_with(|| CString::new(type_name).unwrap_or_default())
        .as_ptr()
}

/// Count the number of registered resource types.
/// @plan PLAN-20260314-RESOURCE.P09
/// @requirement REQ-RES-TYPE-004
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn CountResourceTypes() -> u32 {
    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    state.dispatch.type_registry.count() as u32
}

// =============================================================================
// Config get
// =============================================================================

/// Check if a key exists in the resource map.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_HasKey(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    if state.dispatch.entry(key_str).is_some() {
        1
    } else {
        0
    }
}

/// Check if a key is a STRING type.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_IsString(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_IsInteger(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_IsBoolean(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_IsColor(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
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
/// @plan PLAN-20260314-RESOURCE.P05
/// @requirement REQ-RES-CONF-003
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetString(key: *const c_char) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if key.is_null() {
        return EMPTY.as_ptr() as *const c_char;
    }

    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return EMPTY.as_ptr() as *const c_char,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();

    // Check if entry exists
    let entry = match state.dispatch.entry(key_str) {
        Some(e) => e,
        None => return EMPTY.as_ptr() as *const c_char,
    };

    // Type check: must be STRING
    if entry.res_type != "STRING" {
        return EMPTY.as_ptr() as *const c_char;
    }

    // Value check: str_ptr must be non-null
    let str_ptr = entry.data.str_ptr;
    if str_ptr.is_null() {
        return EMPTY.as_ptr() as *const c_char;
    }

    str_ptr
}

/// Get an integer config value.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetInteger(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    typed_number(state, key_str, "INT32").map_or(0, |v| v as c_int)
}

/// Get a boolean config value.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetBoolean(key: *const c_char) -> c_int {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    c_int::from(typed_number(state, key_str, "BOOLEAN").is_some_and(|v| v != 0))
}

/// Get a color config value as packed (r<<24)|(g<<16)|(b<<8)|a.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_GetColor(key: *const c_char) -> u32 {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return 0,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_ref().unwrap();
    typed_number(state, key_str, "COLOR").unwrap_or(0)
}

// =============================================================================
// Config put
// =============================================================================

/// Put a string value into the resource map.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_PutString(key: *const c_char, value: *const c_char) {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return,
    };
    let value_str = match cstr_bytes(value) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    put_descriptor(state, key_str, &[b"STRING:", value_str].concat());
}

/// Put an integer value into the resource map.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_PutInteger(key: *const c_char, value: c_int) {
    let key_str = match cstr_bytes(key) {
        Some(s) => s,
        None => return,
    };

    let mut guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    ensure_init(&mut guard);

    let state = guard.as_mut().unwrap();
    put_descriptor(state, key_str, format!("INT32:{value}").as_bytes());
}

/// Put a boolean value into the resource map.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_PutBoolean(key: *const c_char, value: c_int) {
    let key_str = match cstr_bytes(key) {
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
    put_descriptor(state, key_str, format!("BOOLEAN:{bool_str}").as_bytes());
}

/// Put a color value into the resource map. Color is packed (r<<24)|(g<<16)|(b<<8)|a.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_PutColor(key: *const c_char, value: u32) {
    let key_str = match cstr_bytes(key) {
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
    put_descriptor(state, key_str, format!("COLOR:{color_str}").as_bytes());
}

// =============================================================================
// File I/O wrappers (UIO delegation)
// =============================================================================

/// Open a resource file. Returns a UIO stream handle, or null on failure.
/// Returns STREAM_SENTINEL (~0) if the target is a directory.
/// @plan PLAN-20260314-RESOURCE.P08
/// @requirement REQ-RES-FILE-002
/// @requirement REQ-RES-FILE-003
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn res_OpenResFile(
    dir: *mut c_void,
    filename: *const c_char,
    mode: *const c_char,
) -> *mut c_void {
    if filename.is_null() || mode.is_null() {
        return ptr::null_mut();
    }

    // Check if target is a directory using uio_stat
    let mut sb: libc::stat = std::mem::zeroed();
    if uio_stat(dir, filename, &mut sb) == 0 {
        // S_ISDIR: check if st_mode has the directory bit set
        // Use libc::S_IFMT and libc::S_IFDIR macros for portability
        if (sb.st_mode & libc::S_IFMT) == libc::S_IFDIR {
            return STREAM_SENTINEL;
        }
    }

    // Not a directory or stat failed — fall through to regular open
    uio_fopen(dir, filename, mode)
}

/// Close a resource file. Returns 1 on success, 0 on failure.
/// NULL and sentinel pointers return 1 (no-op).
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn res_CloseResFile(fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return 1;
    }
    if uio_fclose(fp) == 0 {
        1
    } else {
        0
    }
}

/// Read from a resource file.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
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
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn GetResFileChar(fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1; // EOF
    }
    uio_fgetc(fp)
}

/// Put a character to a resource file.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn PutResFileChar(ch: c_char, fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1; // EOF
    }
    uio_fputc(ch as c_int, fp)
}

/// Write a newline to a resource file.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn PutResFileNewline(fp: *mut c_void) -> c_int {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1; // EOF
    }
    uio_fputc(b'\n' as c_int, fp)
}

/// Seek in a resource file.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn SeekResFile(fp: *mut c_void, offset: c_long, whence: c_int) -> c_long {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1;
    }
    uio_fseek(fp, offset, whence) as c_long
}

/// Tell position in a resource file.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn TellResFile(fp: *mut c_void) -> c_long {
    if fp.is_null() || fp == STREAM_SENTINEL {
        return -1;
    }
    uio_ftell(fp)
}

/// Get the length of a resource file.
/// Sentinel returns 1 (directory marker).
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
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
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn DeleteResFile(dir: *mut c_void, filename: *const c_char) -> c_int {
    if filename.is_null() {
        return 0;
    }
    if uio_unlink(dir, filename) == 0 {
        1
    } else {
        0
    }
}

// =============================================================================
// Resource data loading
// =============================================================================

/// Load a resource from a pathname using the given load function.
///
/// Opens the file via UIO, gets its length, sets `_cur_resfile_name`,
/// calls the load function, cleans up, and returns the loaded data.
/// @plan PLAN-20260314-RESOURCE.P08
/// @requirement REQ-RES-FILE-005
/// @requirement REQ-RES-FILE-008
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
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

    let mode = c"rb".as_ptr() as *const c_char;
    let fp = res_OpenResFile(contentDir, pathname, mode);

    // Reject null only. STREAM_SENTINEL means "directory" and must be
    // passed through to the load function (e.g. _GetFontData reads glyph
    // PNGs from directory entries). The C original does NOT reject sentinel.
    if fp.is_null() {
        return ptr::null_mut();
    }

    // Get file length
    let length = LengthResFile(fp);
    if length == 0 {
        log::warn!("LoadResourceFromPath: zero-length file, closing and returning null");
        res_CloseResFile(fp);
        return ptr::null_mut();
    }

    // Set global file name for C callers
    _cur_resfile_name = pathname;

    let result = load_fn(fp, length as u32);

    // Clear global file name
    _cur_resfile_name = ptr::null();

    res_CloseResFile(fp);
    result
}

/// Read resource data from a file stream.
///
/// Reads the 4-byte legacy prefix from the provided stream.
/// If the prefix is `0xFFFFFFFF` (uncompressed marker), allocates and reads
/// the remaining `length - 4` payload bytes. Returns null on any failure
/// or if the prefix indicates a compressed resource (not supported).
/// @plan PLAN-20260314-RESOURCE.P09
/// @requirement REQ-RES-FILE-006
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn GetResourceData(fp: *mut c_void, length: u32) -> *mut c_void {
    if fp.is_null() || fp == STREAM_SENTINEL || length == 0 {
        return ptr::null_mut();
    }

    // Read the 4-byte length prefix (legacy format)
    // Resource data used to be prefixed by its length in package files.
    // A valid length prefix indicated LZ-compressed data, and
    // a length prefix ~0 (0xFFFFFFFF) meant uncompressed.
    // Currently, .ct and .xlt files still carry a ~0 prefix.
    let mut comp_len: u32 = 0;
    let read = uio_fread(
        &mut comp_len as *mut u32 as *mut c_void,
        std::mem::size_of::<u32>(),
        1,
        fp,
    );
    if read != 1 {
        return ptr::null_mut();
    }

    if comp_len != 0xFFFFFFFF {
        log::warn!("LZ-compressed binary data not supported");
        return ptr::null_mut();
    }

    // Uncompressed: subtract the 4-byte prefix from the total length
    let data_len = (length as usize).saturating_sub(4);
    if data_len == 0 {
        return ptr::null_mut();
    }

    // Allocate via HMalloc (matching C's AllocResourceData)
    let buf = libc::malloc(data_len) as *mut u8;
    if buf.is_null() {
        return ptr::null_mut();
    }

    let bytes_read = uio_fread(buf as *mut c_void, 1, data_len, fp);
    if bytes_read != data_len {
        libc::free(buf as *mut c_void);
        return ptr::null_mut();
    }

    buf as *mut c_void
}

/// Free resource data that was allocated by GetResourceData.
/// Returns 1 on success, 0 on failure.
///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
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

    /// A heap type that loads nothing: enough to see that a registered
    /// non-value type keeps its type name and path and is not loaded early.
    unsafe extern "C" fn mock_heap_load(_path: *const c_char, _data: *mut ResourceData) {}

    unsafe extern "C" fn mock_heap_free(_data: *mut c_void) -> c_int {
        1
    }

    fn install_mock_gfxres(state: &mut ResourceState) {
        state.dispatch.type_registry.install(
            "GFXRES",
            Some(mock_heap_load),
            Some(mock_heap_free),
            None,
        );
    }

    // =========================================================================
    // P19: Init/Uninit tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_init_creates_state() {
        unsafe {
            reset_state();
            let ptr = InitResourceSystem();
            assert!(!ptr.is_null(), "InitResourceSystem should return non-null");

            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            assert!(guard.is_some(), "State should be Some after init");
        }
    }

    #[test]
    #[serial]
    fn test_init_registers_5_types() {
        unsafe {
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
    }

    #[test]
    #[serial]
    fn test_init_idempotent() {
        unsafe {
            reset_state();
            let ptr1 = InitResourceSystem();
            let ptr2 = InitResourceSystem();
            assert_eq!(ptr1, ptr2, "Init should be idempotent");
        }
    }

    #[test]
    #[serial]
    fn test_uninit_clears_state() {
        unsafe {
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
    }

    #[test]
    #[serial]
    fn test_uninit_safe_to_call_twice() {
        unsafe {
            reset_state();
            InitResourceSystem();
            UninitResourceSystem();
            UninitResourceSystem(); // Should not crash
        }
    }

    #[test]
    #[serial]
    fn test_uninit_then_reinit() {
        unsafe {
            reset_state();
            InitResourceSystem();
            UninitResourceSystem();

            // Fresh init after uninit
            let ptr = InitResourceSystem();
            assert!(!ptr.is_null());

            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_ref().unwrap();
            assert_eq!(state.dispatch.type_registry.count(), 5);
            assert!(
                state.dispatch.entries.is_empty(),
                "Fresh init should have no entries"
            );
        }
    }

    // =========================================================================
    // P19: LoadResourceIndex tests (internal API, simulated content)
    // =========================================================================

    #[test]
    #[serial]
    fn test_load_index_parses_entries_internal() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            // Simulate LoadResourceIndex by parsing propfile content directly
            install_mock_gfxres(state);
            let content = "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            let desc = state.dispatch.entry("comm.arilou.graphics").unwrap();
            assert_eq!(desc.res_type, "GFXRES");
            assert_eq!(desc.descriptor_bytes(), b"base/comm/arilou/arilou.ani");
        }
    }

    #[test]
    #[serial]
    fn test_load_index_with_prefix_internal() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "sfxvol = INT32:20\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                Some("config."),
            );

            assert!(state.dispatch.entry("config.sfxvol").is_some());
            assert_eq!(state.dispatch.get_int_resource("config.sfxvol"), Some(20));
        }
    }

    #[test]
    #[serial]
    fn test_load_index_multiple_calls_accumulate() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content_a = "key.a = STRING:alpha\n";
            parse_propfile(
                content_a,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            let content_b = "key.b = STRING:beta\n";
            parse_propfile(
                content_b,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            assert!(state.dispatch.entry("key.a").is_some());
            assert!(state.dispatch.entry("key.b").is_some());
        }
    }

    #[test]
    #[serial]
    fn test_load_index_last_writer_wins() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content_a = "music.battle = STRING:base/battle.mod\n";
            parse_propfile(
                content_a,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            let content_b = "music.battle = STRING:addons/3domusic/battle.ogg\n";
            parse_propfile(
                content_b,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            let desc = state.dispatch.entry("music.battle").unwrap();
            assert_eq!(desc.descriptor_bytes(), b"addons/3domusic/battle.ogg");
        }
    }

    #[test]
    #[serial]
    fn test_load_index_value_types_parsed_immediately() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "sfxvol = INT32:20\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                Some("config."),
            );

            // INT32 is a value type — parsed immediately, data.num should be 20
            let desc = state.dispatch.entry("config.sfxvol").unwrap();
            assert_eq!({ desc.data.num }, 20);
        }
    }

    #[test]
    #[serial]
    fn test_load_index_heap_types_deferred() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            install_mock_gfxres(state);
            let content = "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            // A registered heap type is not loaded until the resource is asked for.
            let desc = state.dispatch.entry("comm.arilou.graphics").unwrap();
            assert_eq!(desc.res_type, "GFXRES");
            assert!(desc.data.ptr.is_null(), "heap data is loaded lazily");
            assert_eq!(desc.refcount, 0);
        }
    }

    #[test]
    #[serial]
    fn test_load_index_unregistered_type_is_unknownres_with_whole_descriptor() {
        // @plan PLAN-20260314-RESOURCE.P04
        // @requirement REQ-RES-UNK-001
        // C `newResourceDesc`: a type nobody registered is not kept; the entry
        // is UNKNOWNRES and its descriptor is the whole value.
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                None,
            );

            let desc = state.dispatch.entry("comm.arilou.graphics").unwrap();
            assert_eq!(desc.res_type, "UNKNOWNRES");
            assert_eq!(
                desc.descriptor_bytes(),
                b"GFXRES:base/comm/arilou/arilou.ani"
            );
            assert!(
                !{ desc.data.str_ptr.is_null() },
                "UNKNOWNRES value type has str_ptr set via eager load"
            );
        }
    }

    #[test]
    #[serial]
    fn test_load_index_boolean_true_internal() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "fullscreen = BOOLEAN:true\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                Some("config."),
            );

            assert_eq!(
                state.dispatch.get_boolean_resource("config.fullscreen"),
                Some(true)
            );
        }
    }

    #[test]
    #[serial]
    fn test_load_index_boolean_false_internal() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "fullscreen = BOOLEAN:false\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                Some("config."),
            );

            assert_eq!(
                state.dispatch.get_boolean_resource("config.fullscreen"),
                Some(false)
            );
        }
    }

    #[test]
    #[serial]
    fn test_load_index_color_internal() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "color = COLOR:rgb(0x1a, 0x00, 0x1a)\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                Some("config."),
            );

            let desc = state.dispatch.entry("config.color").unwrap();
            let packed = { desc.data.num };
            let r = ((packed >> 24) & 0xFF) as u8;
            let g = ((packed >> 16) & 0xFF) as u8;
            let b = ((packed >> 8) & 0xFF) as u8;
            let a = (packed & 0xFF) as u8;
            assert_eq!((r, g, b, a), (0x1a, 0x00, 0x1a, 0xff));
        }
    }

    #[test]
    #[serial]
    fn test_load_index_string_internal() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();

            let content = "up.1 = STRING:key Up\n";
            parse_propfile(
                content,
                &mut |key, value| {
                    state.dispatch.process_resource_desc(key, value).unwrap();
                },
                Some("keys."),
            );

            assert_eq!(
                state.dispatch.get_string_resource("keys.up.1"),
                Some(b"key Up".as_slice())
            );
        }
    }

    // =========================================================================
    // P19: Auto-init tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_auto_init_on_has_key() {
        {
            reset_state();
            // No InitResourceSystem call

            let key = CString::new("nonexistent").unwrap();
            let result = unsafe { res_HasKey(key.as_ptr()) };
            assert_eq!(result, 0, "Non-existent key should return 0");

            // State should have been auto-initialized
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            assert!(guard.is_some(), "State should be auto-initialized");
        }
    }

    #[test]
    #[serial]
    fn test_auto_init_on_count_resource_types() {
        unsafe {
            reset_state();
            // No InitResourceSystem call

            let count = CountResourceTypes();
            assert_eq!(count, 5, "Auto-init should register 5 types");
        }
    }

    // =========================================================================
    // P19: File I/O sentinel tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_length_res_file_sentinel() {
        {
            let result = unsafe { LengthResFile(STREAM_SENTINEL) };
            assert_eq!(result, 1, "Sentinel should return length 1");
        }
    }

    #[test]
    #[serial]
    fn test_close_res_file_sentinel() {
        {
            let result = unsafe { res_CloseResFile(STREAM_SENTINEL) };
            assert_eq!(result, 1, "Sentinel close should return 1 (success)");
        }
    }

    #[test]
    #[serial]
    fn test_close_res_file_null() {
        {
            let result = unsafe { res_CloseResFile(ptr::null_mut()) };
            assert_eq!(result, 1, "Null close should return 1 (success)");
        }
    }

    #[test]
    #[serial]
    fn test_read_res_file_null() {
        {
            let mut buf = [0u8; 16];
            let result =
                unsafe { ReadResFile(buf.as_mut_ptr() as *mut c_void, 1, 16, ptr::null_mut()) };
            assert_eq!(result, 0, "Read from null should return 0");
        }
    }

    #[test]
    #[serial]
    fn test_write_res_file_null() {
        {
            let buf = [0u8; 16];
            let result =
                unsafe { WriteResFile(buf.as_ptr() as *const c_void, 1, 16, ptr::null_mut()) };
            assert_eq!(result, 0, "Write to null should return 0");
        }
    }

    #[test]
    #[serial]
    fn test_get_res_file_char_null() {
        {
            let result = unsafe { GetResFileChar(ptr::null_mut()) };
            assert_eq!(result, -1, "GetChar from null should return -1 (EOF)");
        }
    }

    #[test]
    #[serial]
    fn test_seek_res_file_null() {
        {
            let result = unsafe { SeekResFile(ptr::null_mut(), 0, 0) };
            assert_eq!(result, -1, "Seek on null should return -1");
        }
    }

    #[test]
    #[serial]
    fn test_tell_res_file_null() {
        {
            let result = unsafe { TellResFile(ptr::null_mut()) };
            assert_eq!(result, -1, "Tell on null should return -1");
        }
    }

    #[test]
    #[serial]
    fn test_length_res_file_null() {
        {
            let result = unsafe { LengthResFile(ptr::null_mut()) };
            assert_eq!(result, 0, "Length of null should return 0");
        }
    }

    #[test]
    #[serial]
    fn test_free_resource_data_null() {
        {
            let result = unsafe { FreeResourceData(ptr::null_mut()) };
            assert_eq!(result, 1, "Free null should return 1 (success)");
        }
    }

    // =========================================================================
    // P08: Directory sentinel and LoadResourceFromPath guard tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_stream_sentinel_constant_is_all_bits_set() {
        {
            // @plan PLAN-20260314-RESOURCE.P08
            // @requirement REQ-RES-FILE-003
            assert_eq!(
                STREAM_SENTINEL, !0usize as *mut c_void,
                "STREAM_SENTINEL should be all bits set (~0)"
            );
        }
    }

    #[test]
    #[serial]
    fn test_length_res_file_returns_1_for_sentinel() {
        {
            // @plan PLAN-20260314-RESOURCE.P08
            // @requirement REQ-RES-FILE-003
            let result = unsafe { LengthResFile(STREAM_SENTINEL) };
            assert_eq!(result, 1, "LengthResFile(STREAM_SENTINEL) should return 1");
        }
    }

    // =========================================================================
    // P19: Config Put/Get roundtrip tests via FFI functions
    // =========================================================================

    #[test]
    #[serial]
    fn test_put_get_string_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("test.key").unwrap();
            let value = CString::new("hello world").unwrap();

            {
                res_PutString(key.as_ptr(), value.as_ptr());
                let result = res_GetString(key.as_ptr());
                assert!(!result.is_null());
                let result_str = CStr::from_ptr(result).to_str().unwrap();
                assert_eq!(result_str, "hello world");
            }
        }
    }

    #[test]
    #[serial]
    fn test_put_get_integer_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("test.num").unwrap();
            {
                res_PutInteger(key.as_ptr(), 42);
                let result = res_GetInteger(key.as_ptr());
                assert_eq!(result, 42);
            }
        }
    }

    #[test]
    #[serial]
    fn test_put_get_boolean_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("test.flag").unwrap();
            {
                res_PutBoolean(key.as_ptr(), 1);
                let result = res_GetBoolean(key.as_ptr());
                assert_eq!(result, 1);

                res_PutBoolean(key.as_ptr(), 0);
                let result = res_GetBoolean(key.as_ptr());
                assert_eq!(result, 0);
            }
        }
    }

    #[test]
    #[serial]
    fn test_put_get_color_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("test.color").unwrap();
            let packed: u32 = (0x1a << 24) | (0x1a << 8) | 0xff;
            {
                res_PutColor(key.as_ptr(), packed);
                let result = res_GetColor(key.as_ptr());
                assert_eq!(result, packed);
            }
        }
    }

    #[test]
    #[serial]
    fn test_is_type_checks_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let str_key = CString::new("str.key").unwrap();
            let int_key = CString::new("int.key").unwrap();
            let bool_key = CString::new("bool.key").unwrap();
            let color_key = CString::new("color.key").unwrap();

            {
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
    }

    #[test]
    #[serial]
    fn test_has_key_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("exists.key").unwrap();
            let nokey = CString::new("does.not.exist").unwrap();

            {
                assert_eq!(res_HasKey(key.as_ptr()), 0);

                res_PutString(key.as_ptr(), CString::new("val").unwrap().as_ptr());
                assert_eq!(res_HasKey(key.as_ptr()), 1);
                assert_eq!(res_HasKey(nokey.as_ptr()), 0);
            }
        }
    }

    #[test]
    #[serial]
    fn test_remove_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("remove.me").unwrap();
            {
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
    }

    #[test]
    #[serial]
    fn test_get_resource_type_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let key = CString::new("typed.key").unwrap();
            {
                res_PutInteger(key.as_ptr(), 42);
                let type_ptr = res_GetResourceType(key.as_ptr());
                assert!(!type_ptr.is_null());
                let type_str = CStr::from_ptr(type_ptr).to_str().unwrap();
                assert_eq!(type_str, "INT32");
            }
        }
    }

    #[test]
    #[serial]
    fn test_count_resource_types_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();
            assert_eq!(CountResourceTypes(), 5);
        }
    }
    /// @plan PLAN-20260314-RESOURCE.P09
    /// @requirement REQ-RES-TYPE-004
    #[test]
    #[serial]
    fn test_count_resource_types_returns_u32() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let count = CountResourceTypes();
            // Should return correct count as u32 (not u16)
            assert_eq!(count, 5);

            // Verify the return type is u32 (this compiles correctly if type is u32)
            let _type_check: u32 = count;
        }
    }

    #[test]
    #[serial]
    fn test_null_key_handling() {
        unsafe {
            reset_state();
            InitResourceSystem();

            {
                assert_eq!(res_HasKey(ptr::null()), 0);
                assert_eq!(res_IsString(ptr::null()), 0);
                assert_eq!(res_IsInteger(ptr::null()), 0);
                assert_eq!(res_IsBoolean(ptr::null()), 0);
                assert_eq!(res_IsColor(ptr::null()), 0);
                // @plan PLAN-20260314-RESOURCE.P05: res_GetString now returns empty string, never null
                let str_ptr = res_GetString(ptr::null());
                assert!(!str_ptr.is_null(), "res_GetString should never return null");
                assert_eq!(
                    CStr::from_ptr(str_ptr).to_str().unwrap(),
                    "",
                    "res_GetString should return empty string for null key"
                );
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
    }

    #[test]
    #[serial]
    fn test_get_int_resource_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();
            state
                .dispatch
                .process_resource_desc("vol.sfx", "INT32:20")
                .unwrap();
            drop(guard);

            let key = CString::new("vol.sfx").unwrap();
            let result = { res_GetIntResource(key.as_ptr()) };
            assert_eq!(result, 20);
        }
    }

    #[test]
    #[serial]
    fn test_get_boolean_resource_ffi() {
        unsafe {
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();
            state
                .dispatch
                .process_resource_desc("config.fullscreen", "BOOLEAN:true")
                .unwrap();
            drop(guard);

            let key = CString::new("config.fullscreen").unwrap();
            let result = { res_GetBooleanResource(key.as_ptr()) };
            assert_eq!(result, 1);
        }
    }

    // =========================================================================
    // P05: res_GetString Parity tests
    // @plan PLAN-20260314-RESOURCE.P05
    // =========================================================================

    #[test]
    #[serial]
    fn test_res_get_string_returns_empty_for_missing_key() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P05
            // @requirement REQ-RES-CONF-003
            reset_state();
            InitResourceSystem();

            let key = CString::new("nonexistent.key").unwrap();
            let result = { res_GetString(key.as_ptr()) };

            assert!(!result.is_null(), "res_GetString should never return null");
            let result_str = { CStr::from_ptr(result) }.to_str().unwrap();
            assert_eq!(
                result_str, "",
                "res_GetString should return empty string for missing key"
            );
        }
    }

    #[test]
    #[serial]
    fn test_res_get_string_returns_empty_for_integer_entry() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P05
            // @requirement REQ-RES-CONF-003
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();
            state
                .dispatch
                .process_resource_desc("config.sfxvol", "INT32:128")
                .unwrap();
            drop(guard);

            let key = CString::new("config.sfxvol").unwrap();
            let result = { res_GetString(key.as_ptr()) };

            assert!(!result.is_null(), "res_GetString should never return null");
            let result_str = { CStr::from_ptr(result) }.to_str().unwrap();
            assert_eq!(
                result_str, "",
                "res_GetString should return empty string for INT32 entry, not the integer value"
            );
        }
    }

    #[test]
    #[serial]
    fn test_res_get_string_returns_empty_for_boolean_entry() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P05
            // @requirement REQ-RES-CONF-003
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();
            state
                .dispatch
                .process_resource_desc("config.fullscreen", "BOOLEAN:true")
                .unwrap();
            drop(guard);

            let key = CString::new("config.fullscreen").unwrap();
            let result = { res_GetString(key.as_ptr()) };

            assert!(!result.is_null(), "res_GetString should never return null");
            let result_str = { CStr::from_ptr(result) }.to_str().unwrap();
            assert_eq!(
                result_str, "",
                "res_GetString should return empty string for BOOLEAN entry"
            );
        }
    }

    #[test]
    #[serial]
    fn test_res_get_string_returns_value_for_string_entry() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P05
            // @requirement REQ-RES-CONF-003
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();
            state
                .dispatch
                .process_resource_desc("config.name", "STRING:Player")
                .unwrap();
            drop(guard);

            let key = CString::new("config.name").unwrap();
            let result = { res_GetString(key.as_ptr()) };

            assert!(!result.is_null(), "res_GetString should never return null");
            let result_str = { CStr::from_ptr(result) }.to_str().unwrap();
            assert_eq!(
                result_str, "Player",
                "res_GetString should return actual string value for STRING entry"
            );
        }
    }

    #[test]
    #[serial]
    fn test_res_get_string_returns_empty_for_null_key() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P05
            // @requirement REQ-RES-ERR-003
            reset_state();
            InitResourceSystem();

            let result = { res_GetString(ptr::null()) };

            assert!(
                !result.is_null(),
                "res_GetString should never return null, even for null key"
            );
            let result_str = { CStr::from_ptr(result) }.to_str().unwrap();
            assert_eq!(
                result_str, "",
                "res_GetString should return empty string for null key"
            );
        }
    }

    #[test]
    #[serial]
    fn test_res_get_string_returns_empty_for_unknownres_entry() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P05
            // @requirement REQ-RES-UNK-003
            reset_state();
            InitResourceSystem();

            let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_mut().unwrap();
            state
                .dispatch
                .process_resource_desc("unknown.key", "FAKETYPE:somedata.dat")
                .unwrap();
            drop(guard);

            let key = CString::new("unknown.key").unwrap();
            let result = { res_GetString(key.as_ptr()) };

            assert!(!result.is_null(), "res_GetString should never return null");
            let result_str = { CStr::from_ptr(result) }.to_str().unwrap();
            assert_eq!(
                result_str, "",
                "res_GetString should return empty string for UNKNOWNRES entry (type mismatch)"
            );
        }
    }

    // =========================================================================
    // P07: SaveResourceIndex Filtering tests
    // @plan PLAN-20260314-RESOURCE.P07
    // =========================================================================

    /// Helper: Get the serialized entries that would be written by SaveResourceIndex
    /// filtering logic. This tests the filtering at the dispatch level since UIO
    /// isn't available in tests.
    fn get_saveable_entries(state: &ResourceState, root: Option<&str>) -> Vec<(String, String)> {
        let mut entries: Vec<_> = state
            .dispatch
            .entries
            .iter()
            .filter(|(key, _)| root.is_none_or(|prefix| key.starts_with(prefix.as_bytes())))
            .collect();
        entries.sort_unstable_by_key(|&(key, _)| key);

        entries
            .into_iter()
            .filter_map(|(key, desc)| {
                let line = index_line(&state.dispatch, key, desc)?;
                let line = String::from_utf8(line).expect("test entries are UTF-8");
                let (name, value) = line
                    .trim_end()
                    .split_once(" = ")
                    .expect("a key = value line");
                Some((name.to_string(), value.to_string()))
            })
            .collect()
    }

    #[test]
    #[serial]
    fn test_save_resource_index_skips_entries_without_to_string() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P07
            // @requirement REQ-RES-IDX-005
            //
            // SaveResourceIndex should skip entries whose handler has no toString function.
            // This tests the filtering logic at the dispatch level.

            reset_state();
            InitResourceSystem();

            // Add a STRING entry (has toString)
            let key1 = CString::new("config.name").unwrap();
            let val1 = CString::new("Player").unwrap();
            res_PutString(key1.as_ptr(), val1.as_ptr());

            // Register a heap type without toString (like GFXRES)
            unsafe extern "C" fn dummy_heap_load(_path: *const c_char, data: *mut ResourceData) {
                (*data).ptr = 0xDEADBEEF as *mut c_void;
            }
            unsafe extern "C" fn dummy_heap_free(_data: *mut c_void) -> c_int {
                1
            }

            {
                let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
                let state = guard.as_mut().unwrap();
                state.dispatch.type_registry.install(
                    "HEAPTYPE",
                    Some(dummy_heap_load),
                    Some(dummy_heap_free),
                    None, // No toString
                );

                // Add a heap entry
                state
                    .dispatch
                    .process_resource_desc("gfx.test", "HEAPTYPE:test.gfx")
                    .unwrap();
            }

            // Get saveable entries
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_ref().unwrap();
            let entries = get_saveable_entries(state, None);

            // Should only contain the STRING entry, not the HEAPTYPE entry
            assert_eq!(entries.len(), 1, "Should only save entries with toString");
            assert_eq!(entries[0].0, "config.name");
            assert_eq!(entries[0].1, "STRING:Player");
        }
    }

    #[test]
    #[serial]
    fn test_save_resource_index_skips_unknownres_entries() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P07
            // @requirement REQ-RES-UNK-002
            //
            // SaveResourceIndex should skip UNKNOWNRES entries because they have no toString.

            reset_state();
            InitResourceSystem();

            // Add a valid STRING entry
            let key1 = CString::new("config.name").unwrap();
            let val1 = CString::new("Player").unwrap();
            res_PutString(key1.as_ptr(), val1.as_ptr());

            // Add an unknown type entry
            {
                let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
                let state = guard.as_mut().unwrap();
                state
                    .dispatch
                    .process_resource_desc("addon.bar", "FAKETYPE:addon/bar.dat")
                    .unwrap();
            }

            // Get saveable entries
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_ref().unwrap();
            let entries = get_saveable_entries(state, None);

            // Should only contain the STRING entry, not the UNKNOWNRES entry
            assert_eq!(entries.len(), 1, "Should skip UNKNOWNRES entries");
            assert_eq!(entries[0].0, "config.name");
            assert_eq!(entries[0].1, "STRING:Player");
        }
    }

    #[test]
    #[serial]
    fn test_save_emits_all_value_types_with_to_string() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P07
            // @requirement REQ-RES-IDX-005
            //
            // SaveResourceIndex should emit all value types with toString functions.

            reset_state();
            InitResourceSystem();

            // Add STRING, INT32, BOOLEAN, COLOR entries (all have toString)
            let key1 = CString::new("config.name").unwrap();
            let val1 = CString::new("Player").unwrap();
            res_PutString(key1.as_ptr(), val1.as_ptr());

            let key2 = CString::new("config.sfxvol").unwrap();
            res_PutInteger(key2.as_ptr(), 128);

            let key3 = CString::new("config.fullscreen").unwrap();
            res_PutBoolean(key3.as_ptr(), 1);

            let key4 = CString::new("config.menucolor").unwrap();
            // Packed: (0x1a << 24) | (0x00 << 16) | (0x1a << 8) | 0xff = 0x1a001aff
            res_PutColor(key4.as_ptr(), 0x1a001aff);

            // Get saveable entries
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_ref().unwrap();
            let entries = get_saveable_entries(state, None);

            // All four should be present
            assert_eq!(
                entries.len(),
                4,
                "Should emit all value types with toString"
            );

            // Verify each entry
            assert_eq!(entries[0].0, "config.fullscreen");
            assert_eq!(entries[0].1, "BOOLEAN:true");

            assert_eq!(entries[1].0, "config.menucolor");
            assert_eq!(entries[1].1, "COLOR:rgb(0x1a, 0x00, 0x1a)");

            assert_eq!(entries[2].0, "config.name");
            assert_eq!(entries[2].1, "STRING:Player");

            assert_eq!(entries[3].0, "config.sfxvol");
            assert_eq!(entries[3].1, "INT32:128");
        }
    }

    #[test]
    #[serial]
    fn test_save_respects_root_filter() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P07
            // @requirement REQ-RES-IDX-005
            //
            // SaveResourceIndex should only emit entries matching the root prefix.

            reset_state();
            InitResourceSystem();

            // Add entries with different prefixes
            let key1 = CString::new("config.name").unwrap();
            let val1 = CString::new("Player").unwrap();
            res_PutString(key1.as_ptr(), val1.as_ptr());

            let key2 = CString::new("addon.bar").unwrap();
            let val2 = CString::new("SomeAddon").unwrap();
            res_PutString(key2.as_ptr(), val2.as_ptr());

            let key3 = CString::new("config.sfxvol").unwrap();
            res_PutInteger(key3.as_ptr(), 128);

            // Get saveable entries with root filter
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_ref().unwrap();
            let entries = get_saveable_entries(state, Some("config."));

            // Should only contain config.* entries
            assert_eq!(
                entries.len(),
                2,
                "Should only emit entries matching root prefix"
            );
            assert_eq!(entries[0].0, "config.name");
            assert_eq!(entries[1].0, "config.sfxvol");
        }
    }

    #[test]
    #[serial]
    fn test_save_filters_heap_and_unknown_before_root_check() {
        unsafe {
            // @plan PLAN-20260314-RESOURCE.P07
            // @requirement REQ-RES-IDX-005
            //
            // Filtering by toString should happen independently of root filtering.

            reset_state();
            InitResourceSystem();

            // Register heap type without toString
            unsafe extern "C" fn dummy_heap_load(_path: *const c_char, data: *mut ResourceData) {
                (*data).ptr = 0xDEADBEEF as *mut c_void;
            }
            unsafe extern "C" fn dummy_heap_free(_data: *mut c_void) -> c_int {
                1
            }

            {
                let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
                let state = guard.as_mut().unwrap();
                state.dispatch.type_registry.install(
                    "HEAPTYPE",
                    Some(dummy_heap_load),
                    Some(dummy_heap_free),
                    None,
                );

                // Add heap entry with "config." prefix
                state
                    .dispatch
                    .process_resource_desc("config.gfx", "HEAPTYPE:test.gfx")
                    .unwrap();

                // Add unknown entry with "config." prefix
                state
                    .dispatch
                    .process_resource_desc("config.unknown", "FAKETYPE:test.dat")
                    .unwrap();
            }

            // Add valid STRING entry
            let key = CString::new("config.name").unwrap();
            let val = CString::new("Player").unwrap();
            res_PutString(key.as_ptr(), val.as_ptr());

            // Get saveable entries with root filter
            let guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
            let state = guard.as_ref().unwrap();
            let entries = get_saveable_entries(state, Some("config."));

            // Should only contain the STRING entry, even though heap/unknown entries match prefix
            assert_eq!(
                entries.len(),
                1,
                "Should filter by toString before root check"
            );
            assert_eq!(entries[0].0, "config.name");
        }
    }
}

#[cfg(test)]
#[path = "ffi_bridge_file_tests.rs"]
mod file_tests;
