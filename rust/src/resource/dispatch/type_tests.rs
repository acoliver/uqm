//! Types whose identifier is raw bytes, and the `sys.` keys of the registry.
//!
//! The heap type here is a test injection: its callbacks only count calls and
//! hand out a fixed buffer, so the rows can see which pointer the dispatch
//! passes where. Nothing in it stands for a game type.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::Mutex;

use serial_test::serial;

use super::*;
use crate::resource::type_registry::{descriptor_to_int, int_to_string, use_descriptor_as_res};

static LOADS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static LAST_FREED: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LAST_DESCRIPTOR: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static BUFFER: [u8; 8] = [0; 8];

fn buffer() -> *mut c_void {
    BUFFER.as_ptr().cast_mut().cast()
}

unsafe extern "C" fn counting_load(descriptor: *const c_char, data: *mut ResourceData) {
    LOADS.fetch_add(1, Ordering::SeqCst);
    *LAST_DESCRIPTOR.lock().unwrap() = CStr::from_ptr(descriptor).to_bytes().to_vec();
    (*data).ptr = buffer();
}

unsafe extern "C" fn counting_free(handle: *mut c_void) -> c_int {
    FREES.fetch_add(1, Ordering::SeqCst);
    LAST_FREED.store(handle, Ordering::SeqCst);
    1
}

const RAW_NAME: &[u8] = b"H\xE9";

/// A descriptor value of the raw-named type: `H\xE9` and `tail`.
fn value(tail: &[u8]) -> Vec<u8> {
    [RAW_NAME, tail].concat()
}

fn dispatch_with_raw_heap_type() -> ResourceDispatch {
    LOADS.store(0, Ordering::SeqCst);
    FREES.store(0, Ordering::SeqCst);
    LAST_FREED.store(ptr::null_mut(), Ordering::SeqCst);
    let mut dispatch = ResourceDispatch::new();
    dispatch
        .type_registry
        .install(RAW_NAME, Some(counting_load), Some(counting_free), None)
        .unwrap();
    dispatch
}

fn dispatch_with_builtin_numbers() -> ResourceDispatch {
    let mut dispatch = ResourceDispatch::new();
    dispatch
        .type_registry
        .install("INT32", Some(descriptor_to_int), None, Some(int_to_string))
        .unwrap();
    dispatch
        .type_registry
        .install("STRING", Some(use_descriptor_as_res), None, None)
        .unwrap();
    dispatch
}

#[test]
#[serial]
fn a_raw_named_heap_type_loads_once_and_is_freed_with_the_pointer_it_loaded() {
    let mut dispatch = dispatch_with_raw_heap_type();
    dispatch
        .process_resource_desc("k", value(b":path/\xE7"))
        .unwrap();
    assert_eq!(dispatch.entry("k").unwrap().res_type.as_bytes(), RAW_NAME);
    assert_eq!(LOADS.load(Ordering::SeqCst), 0, "heap types load lazily");

    assert_eq!(dispatch.get_resource("k"), buffer());
    assert_eq!(dispatch.get_resource("k"), buffer());
    assert_eq!(LOADS.load(Ordering::SeqCst), 1);
    assert_eq!(*LAST_DESCRIPTOR.lock().unwrap(), b"path/\xE7");
    assert_eq!(dispatch.entry("k").unwrap().refcount, 2);

    dispatch.free_resource("k");
    assert_eq!(FREES.load(Ordering::SeqCst), 0, "still referenced");
    dispatch.free_resource("k");
    assert_eq!(FREES.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_FREED.load(Ordering::SeqCst), buffer());

    assert_eq!(dispatch.get_resource("k"), buffer());
    assert_eq!(LOADS.load(Ordering::SeqCst), 2, "a freed resource reloads");
}

#[test]
#[serial]
fn replacing_a_live_raw_typed_entry_frees_the_old_data_once() {
    let mut dispatch = dispatch_with_raw_heap_type();
    let line = value(b":one");
    dispatch.process_resource_desc("k", &line).unwrap();
    dispatch.get_resource("k");

    dispatch.process_resource_desc("k", &line).unwrap();
    assert_eq!(FREES.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_FREED.load(Ordering::SeqCst), buffer());
    assert!(dispatch.entry("k").unwrap().data_is_unloaded());

    assert!(dispatch.remove_resource("k"));
    assert_eq!(FREES.load(Ordering::SeqCst), 1, "nothing loaded to free");
}

#[test]
#[serial]
fn detaching_hands_the_pointer_over_without_calling_free() {
    let mut dispatch = dispatch_with_raw_heap_type();
    dispatch.process_resource_desc("k", value(b":d")).unwrap();
    dispatch.get_resource("k");
    assert_eq!(dispatch.detach_resource("k"), buffer());
    assert_eq!(FREES.load(Ordering::SeqCst), 0);
    assert!(dispatch.entry("k").unwrap().data_is_unloaded());
    assert_eq!(dispatch.get_resource("k"), buffer());
    assert_eq!(LOADS.load(Ordering::SeqCst), 2);
}

#[test]
#[serial]
fn cleanup_frees_what_is_still_loaded_and_only_that() {
    let mut dispatch = dispatch_with_raw_heap_type();
    for key in ["loaded", "unloaded"] {
        dispatch.process_resource_desc(key, value(b":c")).unwrap();
    }
    dispatch.get_resource("loaded");
    dispatch.cleanup_all_entries();
    assert_eq!(FREES.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_FREED.load(Ordering::SeqCst), buffer());
}

#[test]
fn type_keys_are_found_without_being_entries() {
    let mut dispatch = dispatch_with_builtin_numbers();
    let long = "L".repeat(30);
    dispatch
        .type_registry
        .install(&long, Some(use_descriptor_as_res), None, None)
        .unwrap();

    assert!(dispatch.has_key("sys.INT32"));
    assert!(dispatch.has_key("sys.UNKNOWNRES"));
    assert!(!dispatch.has_key("sys.NOSUCH"));
    assert!(!dispatch.has_key("INT32"));
    assert!(dispatch.has_key(format!("sys.{}", "L".repeat(27))));
    assert!(!dispatch.has_key(format!("sys.{long}")));
    assert!(dispatch.is_type_key("sys.STRING"));
    assert!(dispatch.entries.is_empty(), "no shadow entries for types");
    assert!(dispatch.get_resource_type("sys.INT32").is_none());
    assert!(dispatch.entry("sys.INT32").is_none());
}

#[test]
fn the_key_of_a_registered_type_cannot_become_an_entry_and_the_type_survives() {
    let mut dispatch = dispatch_with_builtin_numbers();
    for key in ["sys.INT32", "sys.UNKNOWNRES", "sys.STRING"] {
        assert_eq!(
            dispatch.process_resource_desc(key, "STRING:x"),
            Err(DescriptorError::RegisteredTypeKey),
            "{key}"
        );
    }
    assert!(dispatch.entries.is_empty(), "no shadow entry for a type");
    assert_eq!(dispatch.type_registry.count(), 3);
    assert!(dispatch.has_key("sys.INT32"));
    dispatch.process_resource_desc("n", "INT32:5").unwrap();
    assert_eq!(dispatch.get_int_resource("n"), Some(5));
    assert!(!dispatch.remove_resource("sys.INT32"));
    assert!(dispatch.has_key("sys.INT32"));
}

#[test]
fn an_unregistered_sys_key_is_an_ordinary_resource_through_its_whole_life() {
    let mut dispatch = dispatch_with_builtin_numbers();
    for key in ["sys.FOO", "sys.", "sys.int32", "sys.INT32x"] {
        assert!(!dispatch.has_key(key), "{key} before");
        dispatch.process_resource_desc(key, "INT32:7").unwrap();
        assert!(dispatch.has_key(key), "{key} stored");
        assert!(!dispatch.is_type_key(key), "{key} is no type");
        assert_eq!(dispatch.get_int_resource(key), Some(7), "{key}");
        assert_eq!(dispatch.get_resource_type(key).unwrap(), "INT32", "{key}");
    }
    assert_eq!(
        dispatch.entries.len(),
        4,
        "one ordinary store, one entry per key"
    );
    assert_eq!(dispatch.type_registry.count(), 3, "no type was added");

    dispatch
        .process_resource_desc("sys.FOO", "STRING:again")
        .unwrap();
    assert_eq!(dispatch.get_string_resource("sys.FOO"), Some(&b"again"[..]));

    assert!(dispatch.remove_resource("sys.FOO"));
    assert!(!dispatch.has_key("sys.FOO"));
    assert!(
        dispatch.has_key("sys.INT32"),
        "the registered type is untouched"
    );
    assert!(!dispatch.remove_resource("sys.FOO"));
}

#[test]
fn a_type_whose_key_an_entry_already_holds_is_refused_and_the_entry_stays() {
    let mut dispatch = dispatch_with_builtin_numbers();
    dispatch
        .process_resource_desc("sys.NEWTYPE", "INT32:9")
        .unwrap();
    let before = dispatch.type_registry.count();

    let refused = dispatch.install_type("NEWTYPE", Some(use_descriptor_as_res), None, None);
    assert_eq!(refused, Err(InstallError::KeyHeldByEntry));
    assert_eq!(dispatch.type_registry.count(), before);
    assert!(dispatch.type_registry.lookup("NEWTYPE").is_none());
    assert_eq!(dispatch.get_int_resource("sys.NEWTYPE"), Some(9));

    // Only the first 27 bytes are the key: a longer identifier collides too.
    let long = format!("{}tail", "N".repeat(27));
    dispatch
        .process_resource_desc(format!("sys.{}", "N".repeat(27)), "INT32:1")
        .unwrap();
    assert_eq!(
        dispatch.install_type(&long, Some(use_descriptor_as_res), None, None),
        Err(InstallError::KeyHeldByEntry)
    );

    // Once the entry is gone the key is free for the type, which then owns it.
    assert!(dispatch.remove_resource("sys.NEWTYPE"));
    dispatch
        .install_type("NEWTYPE", Some(use_descriptor_as_res), None, None)
        .unwrap();
    assert!(dispatch.is_type_key("sys.NEWTYPE"));
    assert_eq!(
        dispatch.process_resource_desc("sys.NEWTYPE", "INT32:2"),
        Err(DescriptorError::RegisteredTypeKey)
    );
}

#[test]
fn a_key_that_only_contains_sys_is_an_ordinary_key() {
    let mut dispatch = dispatch_with_builtin_numbers();
    dispatch
        .process_resource_desc("my.sys.INT32", "INT32:1")
        .unwrap();
    dispatch.process_resource_desc("system", "INT32:2").unwrap();
    assert_eq!(dispatch.get_int_resource("my.sys.INT32"), Some(1));
    assert_eq!(dispatch.get_int_resource("system"), Some(2));
}
