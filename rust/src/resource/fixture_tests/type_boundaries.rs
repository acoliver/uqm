//! Rows T2-T5, T10 and T11: the type identifiers C keeps in its resource map
//! and the `sys.` keys they live under, through the public C-ABI functions.
//!
//! `InstallResTypeVectors` stores a type under `"sys." + identifier` cut to 27
//! bytes (`char key[TYPESIZ]`, `snprintf`), and a descriptor's type text is cut
//! the same way before the lookup in `newResourceDesc`. The type an entry
//! reports is `handlers->resType`, the caller's whole identifier. A second
//! registration under a key already in the map is refused (`CharHashTable_add`
//! returns false). Identifiers are C strings: any bytes but NUL. The loaders
//! and `toString` functions installed here are the built-in INT32 and STRING
//! ones, so only the identifier is new.

use std::ffi::{c_char, CStr};
use std::ptr;

use serial_test::serial;

use crate::resource::ffi_bridge::{
    res_GetIntResource, res_GetInteger, res_GetResource, res_GetResourceType, res_HasKey,
    res_IsInteger, res_Remove, CountResourceTypes, InstallResTypeVectors,
};
use crate::resource::type_registry::{
    descriptor_to_int, int_to_string, raw_descriptor, use_descriptor_as_res,
};

use super::descriptors::{has, integer, key, loaded, saved, string, type_of};
use super::res::Rows;
use super::uio_api::Session;

/// A registration of the built-in INT32 loader under `identifier`.
fn install_number_type(identifier: &[u8]) -> i32 {
    let identifier = key(identifier);
    unsafe {
        InstallResTypeVectors(
            identifier.as_ptr(),
            Some(descriptor_to_int),
            None,
            Some(int_to_string),
        )
    }
}

/// A registration of the built-in STRING loader under `identifier`.
fn install_text_type(identifier: &[u8]) -> i32 {
    let identifier = key(identifier);
    unsafe {
        InstallResTypeVectors(
            identifier.as_ptr(),
            Some(use_descriptor_as_res),
            None,
            Some(raw_descriptor),
        )
    }
}

/// What `res_GetResource` hands back for an entry of a string-valued type
/// installed here: the descriptor, as a C string.
fn payload(name: &[u8]) -> Option<Vec<u8>> {
    let name = key(name);
    let raw = unsafe { res_GetResource(name.as_ptr()) };
    (!raw.is_null()).then(|| unsafe { CStr::from_ptr(raw.cast()) }.to_bytes().to_vec())
}

fn number(name: &[u8]) -> u32 {
    let name = key(name);
    unsafe { res_GetIntResource(name.as_ptr()) }
}

fn sys_key(identifier_bytes: &[u8]) -> Vec<u8> {
    [b"sys.".as_slice(), identifier_bytes].concat()
}

fn type_count() -> u32 {
    unsafe { CountResourceTypes() }
}

#[test]
#[serial]
fn t2_a_type_identifier_that_is_not_utf8_registers_loads_and_saves() {
    let _session = Session::begin();
    let mut rows = Rows::default();
    rows.check("install", install_number_type(b"G\xE9"), 1);
    let cfg = loaded(b"k = G\xE9:42\n", None);
    rows.check("type", type_of(b"k"), Some(b"G\xE9".to_vec()));
    rows.check("value", number(b"k"), 42);
    rows.check(
        "saved",
        saved(&cfg, None, false),
        b"k = G\xE9:42\n".to_vec(),
    );
    rows.check("type key", has(&sys_key(b"G\xE9")), true);
    rows.finish();
}

#[test]
#[serial]
fn t3_a_long_identifier_is_found_by_its_first_27_bytes_and_reports_all_of_them() {
    let _session = Session::begin();
    let identifier = [b'A'; 30];
    let mut rows = Rows::default();
    rows.check("install", install_text_type(&identifier), 1);
    let text = |n: usize, payload: &str| {
        let mut line = format!("k{n} = ").into_bytes();
        line.extend_from_slice(&vec![b'A'; n]);
        line.extend_from_slice(format!(":{payload}\n").as_bytes());
        line
    };
    let file = [text(27, "a"), text(28, "b"), text(40, "c"), text(26, "d")].concat();
    let _cfg = loaded(&file, None);
    for (name, expected) in [(&b"k27"[..], &b"a"[..]), (b"k28", b"b"), (b"k40", b"c")] {
        let row = String::from_utf8_lossy(name).into_owned();
        rows.check(
            &format!("{row} type"),
            type_of(name),
            Some(identifier.to_vec()),
        );
        rows.check(
            &format!("{row} payload"),
            payload(name),
            Some(expected.to_vec()),
        );
    }
    rows.check("k26 type", type_of(b"k26"), Some(b"UNKNOWNRES".to_vec()));
    let mut whole = vec![b'A'; 26];
    whole.extend_from_slice(b":d");
    rows.check("k26 descriptor", payload(b"k26"), Some(whole));
    rows.finish();
}

#[test]
#[serial]
fn t3_the_registered_key_is_the_first_27_bytes_whatever_the_identifier_length() {
    let _session = Session::begin();
    let mut rows = Rows::default();
    rows.check("install", install_text_type(&[b'B'; 30]), 1);
    rows.check("27 bytes", has(&sys_key(&[b'B'; 27])), true);
    rows.check("26 bytes", has(&sys_key(&[b'B'; 26])), false);
    rows.check("28 bytes", has(&sys_key(&[b'B'; 28])), false);
    rows.check("30 bytes", has(&sys_key(&[b'B'; 30])), false);
    rows.finish();
}

#[test]
#[serial]
fn t4_a_cut_inside_a_multibyte_character_neither_panics_nor_loses_the_type() {
    let _session = Session::begin();
    let identifier = "é".repeat(20).into_bytes();
    assert_eq!(identifier.len(), 40);
    let mut rows = Rows::default();
    rows.check("install", install_text_type(&identifier), 1);
    let mut file = b"whole = ".to_vec();
    file.extend_from_slice(&identifier);
    file.extend_from_slice(b":p\ncut = ");
    file.extend_from_slice("é".repeat(14).as_bytes());
    file.extend_from_slice(b":q\n");
    let _cfg = loaded(&file, None);
    rows.check("whole type", type_of(b"whole"), Some(identifier.clone()));
    rows.check("cut type", type_of(b"cut"), Some(identifier.clone()));
    rows.check("whole payload", payload(b"whole"), Some(b"p".to_vec()));
    rows.check("cut payload", payload(b"cut"), Some(b"q".to_vec()));
    // The 27th byte is the first byte of a character.
    let mut key27 = "é".repeat(13).into_bytes();
    key27.push(0xC3);
    rows.check("type key", has(&sys_key(&key27)), true);
    rows.finish();
}

#[test]
#[serial]
fn t5_a_second_registration_under_the_same_key_is_refused_and_the_first_stays() {
    let _session = Session::begin();
    let mut rows = Rows::default();
    rows.check("first", install_number_type(b"DUP"), 1);
    let count = type_count();
    rows.check("second", install_text_type(b"DUP"), 0);
    rows.check("count", type_count(), count);
    let _cfg = loaded(b"k = DUP:7\n", None);
    rows.check("first handlers still load", number(b"k"), 7);
    rows.finish();
}

#[test]
#[serial]
fn t5_two_identifiers_that_share_27_bytes_are_one_key_and_the_first_wins() {
    let _session = Session::begin();
    let first = [vec![b'C'; 27], b"xyz".to_vec()].concat();
    let second = [vec![b'C'; 27], b"123".to_vec()].concat();
    let mut rows = Rows::default();
    rows.check("first", install_number_type(&first), 1);
    let count = type_count();
    rows.check("second", install_text_type(&second), 0);
    rows.check("count", type_count(), count);
    let mut file = b"k = ".to_vec();
    file.extend_from_slice(&second);
    file.extend_from_slice(b":5\n");
    let _cfg = loaded(&file, None);
    rows.check("type", type_of(b"k"), Some(first));
    rows.check("value", number(b"k"), 5);
    rows.finish();
}

#[test]
#[serial]
fn t10_sys_keys_are_keys_but_not_resources() {
    let _session = Session::begin();
    let mut rows = Rows::default();
    rows.check("HasKey sys.INT32", has(b"sys.INT32"), true);
    rows.check("HasKey sys.UNKNOWNRES", has(b"sys.UNKNOWNRES"), true);
    rows.check("HasKey sys.NOSUCH", has(b"sys.NOSUCH"), false);
    rows.check("HasKey INT32", has(b"INT32"), false);
    let name = key(b"sys.INT32");
    unsafe {
        rows.check("type", type_of(b"sys.INT32"), None);
        rows.check("IsInteger", res_IsInteger(name.as_ptr()), 0);
        rows.check("GetInteger", res_GetInteger(name.as_ptr()), 0);
        rows.check(
            "GetResource",
            res_GetResource(name.as_ptr()).is_null(),
            true,
        );
        rows.check("Remove", res_Remove(name.as_ptr()), 0);
        rows.check("HasKey after Remove", res_HasKey(name.as_ptr()), 1);
    }
    rows.finish();
}

#[test]
#[serial]
fn t10_an_index_line_cannot_replace_a_registered_type_through_its_key() {
    let _session = Session::begin();
    let cfg = loaded(
        b"sys.INT32 = STRING:x\nsys.STRING = INT32:1\nn = INT32:5\n",
        None,
    );
    let types = type_count();
    let mut rows = Rows::default();
    rows.check("HasKey sys.INT32", has(b"sys.INT32"), true);
    rows.check("HasKey sys.STRING", has(b"sys.STRING"), true);
    rows.check("type of n", type_of(b"n"), Some(b"INT32".to_vec()));
    rows.check("value of n", integer(b"n"), 5);
    rows.check("type count", type_count(), types);
    rows.check("saved", saved(&cfg, None, false), b"n = INT32:5\n".to_vec());
    // The type still loads the next INT32 line, so it was not replaced.
    let _more = loaded(b"m = INT32:6\n", None);
    rows.check("value of m", integer(b"m"), 6);
    rows.finish();
}

#[test]
#[serial]
fn t10_an_unregistered_sys_key_is_an_ordinary_entry_that_loads_gets_saves_and_goes() {
    let _session = Session::begin();
    let cfg = loaded(
        b"sys.FOO = INT32:7\nsys.bar = STRING:text\nn = INT32:5\n",
        None,
    );
    let types = type_count();
    let mut rows = Rows::default();
    rows.check("HasKey sys.FOO", has(b"sys.FOO"), true);
    rows.check("type", type_of(b"sys.FOO"), Some(b"INT32".to_vec()));
    rows.check("value", integer(b"sys.FOO"), 7);
    rows.check("sys.bar", string(b"sys.bar"), b"text".to_vec());
    rows.check("type count", type_count(), types);
    rows.check(
        "saved",
        saved(&cfg, None, false),
        b"n = INT32:5\nsys.FOO = INT32:7\nsys.bar = STRING:text\n".to_vec(),
    );
    rows.check(
        "saved under the sys. root",
        saved(&cfg, Some(b"sys."), true),
        b"FOO = INT32:7\nbar = STRING:text\n".to_vec(),
    );
    let name = key(b"sys.FOO");
    rows.check("Remove", unsafe { res_Remove(name.as_ptr()) }, 1);
    rows.check("HasKey after Remove", has(b"sys.FOO"), false);
    rows.check("Remove again", unsafe { res_Remove(name.as_ptr()) }, 0);
    rows.check("sys.bar kept", has(b"sys.bar"), true);
    rows.check("registered sys.INT32 kept", has(b"sys.INT32"), true);
    rows.finish();
}

#[test]
#[serial]
fn t10_a_type_cannot_take_the_key_of_an_entry_and_can_after_the_entry_is_removed() {
    let _session = Session::begin();
    let _cfg = loaded(b"sys.LATE = INT32:3\n", None);
    let mut rows = Rows::default();
    let types = type_count();
    rows.check("install over entry", install_number_type(b"LATE"), 0);
    rows.check("type count", type_count(), types);
    rows.check("entry value", integer(b"sys.LATE"), 3);
    let name = key(b"sys.LATE");
    rows.check("Remove", unsafe { res_Remove(name.as_ptr()) }, 1);
    rows.check("install after Remove", install_number_type(b"LATE"), 1);
    rows.check("type count after", type_count(), types + 1);
    rows.check("HasKey as a type", has(b"sys.LATE"), true);
    rows.check("Remove a type", unsafe { res_Remove(name.as_ptr()) }, 0);
    rows.check("still a type", has(b"sys.LATE"), true);
    rows.finish();
}

#[test]
#[serial]
fn t11_the_type_pointer_stays_valid_and_the_same_while_more_types_are_added() {
    let _session = Session::begin();
    assert_eq!(install_number_type(b"ptr\xE9"), 1);
    let _cfg = loaded(b"k = ptr\xE9:1\n", None);
    let name = key(b"k");
    let first: *const c_char = unsafe { res_GetResourceType(name.as_ptr()) };
    assert!(!first.is_null());
    for n in 0..300 {
        assert_eq!(install_number_type(format!("T{n}").as_bytes()), 1);
    }
    let again: *const c_char = unsafe { res_GetResourceType(name.as_ptr()) };
    assert!(ptr::eq(first, again));
    assert_eq!(unsafe { CStr::from_ptr(first) }.to_bytes(), b"ptr\xE9");
}
