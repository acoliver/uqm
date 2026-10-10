//! Rows D1-D12: how a descriptor becomes an entry and how a key is kept, as
//! `resinit.c` `newResourceDesc`, `process_resource_desc`, `res_Remove` and
//! `SaveResourceIndex` do it. Keys, types and descriptors are C strings, so
//! every row goes through the public C-ABI functions with raw bytes.
//!
//! The expected values were traced from the C source (not compiled):
//! a descriptor without a colon, or with a type that is not registered, is
//! stored as `UNKNOWNRES` holding the whole descriptor; the key is any byte
//! string; only STRING, INT32, BOOLEAN and COLOR are written back.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;

use serial_test::serial;
use sha2::{Digest, Sha256};

use crate::resource::ffi_bridge::{
    res_FreeResource, res_GetInteger, res_GetResource, res_GetResourceType, res_GetString,
    res_HasKey, res_IsBoolean, res_IsColor, res_IsInteger, res_IsString, res_PutString, res_Remove,
    SaveResourceIndex,
};

use super::config::{config_dir, Config};
use super::res::{self, Rows};
use super::uio_api::{cstr, Session};

fn key(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("fixture bytes have no NUL")
}

fn has(name: &[u8]) -> bool {
    let name = key(name);
    unsafe { res_HasKey(name.as_ptr()) != 0 }
}

fn type_of(name: &[u8]) -> Option<Vec<u8>> {
    let name = key(name);
    unsafe {
        let ty: *const c_char = res_GetResourceType(name.as_ptr());
        (!ty.is_null()).then(|| CStr::from_ptr(ty).to_bytes().to_vec())
    }
}

fn integer(name: &[u8]) -> i32 {
    let name = key(name);
    unsafe { res_GetInteger(name.as_ptr()) }
}

fn string(name: &[u8]) -> Vec<u8> {
    let name = key(name);
    unsafe {
        CStr::from_ptr(res_GetString(name.as_ptr()))
            .to_bytes()
            .to_vec()
    }
}

/// `res_IsString`, `res_IsInteger`, `res_IsBoolean`, `res_IsColor` in order.
fn kinds(name: &[u8]) -> [bool; 4] {
    let name = key(name);
    unsafe {
        [
            res_IsString(name.as_ptr()) != 0,
            res_IsInteger(name.as_ptr()) != 0,
            res_IsBoolean(name.as_ptr()) != 0,
            res_IsColor(name.as_ptr()) != 0,
        ]
    }
}

/// The C string `res_GetResource` hands back for a string-valued entry
/// (STRING or UNKNOWNRES), and the pointer itself. For INT32, BOOLEAN and
/// COLOR it returns the number in the pointer, which is not read as a string.
fn resource_text(name: &[u8]) -> Option<(Vec<u8>, *const c_void)> {
    let holds_string = matches!(type_of(name).as_deref(), Some(b"STRING" | b"UNKNOWNRES"));
    if !holds_string {
        return None;
    }
    let name = key(name);
    unsafe {
        let raw = res_GetResource(name.as_ptr());
        (!raw.is_null()).then(|| {
            (
                CStr::from_ptr(raw.cast()).to_bytes().to_vec(),
                raw.cast_const(),
            )
        })
    }
}

fn remove(name: &[u8]) -> i32 {
    let name = key(name);
    unsafe { res_Remove(name.as_ptr()) }
}

fn put_string(name: &[u8], value: &[u8]) {
    let (name, value) = (key(name), key(value));
    unsafe { res_PutString(name.as_ptr(), value.as_ptr()) };
}

/// `SaveResourceIndex (dir, "out.cfg", root, strip_root)`; returns the file.
fn saved(cfg: &Config, root: Option<&[u8]>, strip_root: bool) -> Vec<u8> {
    let root = root.map(key);
    let file = cstr("out.cfg");
    unsafe {
        SaveResourceIndex(
            cfg.root.as_resource_dir(),
            file.as_ptr(),
            root.as_ref().map_or(ptr::null(), |r| r.as_ptr()),
            i32::from(strip_root),
        );
    }
    std::fs::read(cfg.dir.path().join("out.cfg")).expect("SaveResourceIndex wrote the file")
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn loaded(file: &[u8], prefix: Option<&str>) -> Config {
    let cfg = config_dir(&[("t.rmp", file)]);
    res::load(&cfg.root, "t.rmp", prefix);
    cfg
}

fn lines_of(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut lines: Vec<Vec<u8>> = bytes
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect();
    lines.sort_unstable();
    lines
}

/// What every unknown or colon-less descriptor looks like to the C getters.
fn check_unknown(rows: &mut Rows, name: &[u8], descriptor: &[u8]) {
    let row = String::from_utf8_lossy(name).into_owned();
    rows.check(&format!("{row} has"), has(name), true);
    rows.check(
        &format!("{row} type"),
        type_of(name),
        Some(b"UNKNOWNRES".to_vec()),
    );
    rows.check(
        &format!("{row} kinds"),
        kinds(name),
        [false, false, false, false],
    );
    rows.check(&format!("{row} GetString"), string(name), Vec::new());
    rows.check(
        &format!("{row} GetResource"),
        resource_text(name).map(|(text, _)| text),
        Some(descriptor.to_vec()),
    );
}

#[test]
#[serial]
fn d1_a_descriptor_without_a_colon_is_unknownres_holding_all_of_it() {
    let _session = Session::begin();
    let cfg = loaded(b"plain = plain/path\n", None);
    let mut rows = Rows::default();
    check_unknown(&mut rows, b"plain", b"plain/path");
    rows.check("saved", saved(&cfg, None, false), Vec::new());
    rows.finish();
}

#[test]
#[serial]
fn d2_an_unregistered_type_keeps_the_type_text_in_the_descriptor() {
    let _session = Session::begin();
    let cfg = loaded(b"future = FUTURE:foo\n", None);
    let mut rows = Rows::default();
    check_unknown(&mut rows, b"future", b"FUTURE:foo");
    rows.check("saved", saved(&cfg, None, false), Vec::new());
    rows.finish();
}

#[test]
#[serial]
fn d3_registered_types_keep_their_type_and_unknown_ones_are_not_written() {
    let _session = Session::begin();
    let cfg = loaded(
        b"s = STRING:a:b\nn = INT32:7\nu = UNKNOWNRES:xyz\nf = FUTURE:foo\nplain = p\n",
        None,
    );
    let mut rows = Rows::default();
    rows.check("s type", type_of(b"s"), Some(b"STRING".to_vec()));
    rows.check("s text", string(b"s"), b"a:b".to_vec());
    rows.check("n type", type_of(b"n"), Some(b"INT32".to_vec()));
    rows.check("n value", integer(b"n"), 7);
    // `UNKNOWNRES` is a registered type, so only what follows its colon is kept.
    rows.check("u type", type_of(b"u"), Some(b"UNKNOWNRES".to_vec()));
    rows.check(
        "u GetResource",
        resource_text(b"u").map(|(text, _)| text),
        Some(b"xyz".to_vec()),
    );
    let want = b"n = INT32:7\ns = STRING:a:b\n";
    let got = saved(&cfg, None, false);
    rows.check(
        "saved sha256",
        sha256(&lines_of(&got).concat()),
        sha256(&lines_of(want).concat()),
    );
    rows.check("saved lines", lines_of(&got), lines_of(want));
    rows.finish();
}

#[test]
#[serial]
fn d4_the_type_text_is_not_trimmed_or_case_folded() {
    let _session = Session::begin();
    let _cfg = loaded(b"e = :foo\nsp = STRING :abc\nlow = string:abc\n", None);
    let mut rows = Rows::default();
    check_unknown(&mut rows, b"e", b":foo");
    check_unknown(&mut rows, b"sp", b"STRING :abc");
    check_unknown(&mut rows, b"low", b"string:abc");
    rows.finish();
}

#[test]
#[serial]
fn d5_a_key_that_is_not_utf8_is_a_key() {
    let _session = Session::begin();
    let cfg = loaded(b"caf\xE9 = INT32:5\n", None);
    let mut rows = Rows::default();
    rows.check("has", has(b"caf\xE9"), true);
    rows.check("value", integer(b"caf\xE9"), 5);
    rows.check("type", type_of(b"caf\xE9"), Some(b"INT32".to_vec()));
    rows.check("kinds", kinds(b"caf\xE9"), [false, true, false, false]);
    let got = saved(&cfg, None, false);
    rows.check("saved sha256", sha256(&got), sha256(b"caf\xE9 = INT32:5\n"));
    rows.check("saved", got, b"caf\xE9 = INT32:5\n".to_vec());
    rows.finish();
}

#[test]
#[serial]
fn d5_a_prefix_is_put_in_front_of_a_key_that_is_not_utf8() {
    let _session = Session::begin();
    let _cfg = loaded(b"caf\xE9 = INT32:5\n", Some("cfg."));
    assert!(has(b"cfg.caf\xE9"));
    assert_eq!(integer(b"cfg.caf\xE9"), 5);
    assert!(!has(b"caf\xE9"));
}

#[test]
#[serial]
fn d5b_keys_that_decode_to_the_same_text_stay_two_keys() {
    let _session = Session::begin();
    let cfg = loaded(b"a\xE9 = INT32:1\na\xE8 = INT32:2\n", None);
    let mut rows = Rows::default();
    rows.check("a\\xE9", integer(b"a\xE9"), 1);
    rows.check("a\\xE8", integer(b"a\xE8"), 2);
    rows.check(
        "saved",
        lines_of(&saved(&cfg, None, false)),
        vec![b"a\xE8 = INT32:2".to_vec(), b"a\xE9 = INT32:1".to_vec()],
    );
    rows.finish();
}

#[test]
#[serial]
fn d5c_remove_finds_a_key_that_is_not_utf8_once() {
    let _session = Session::begin();
    let _cfg = loaded(b"a\xE9 = INT32:1\n", None);
    assert_eq!(remove(b"a\xE9"), 1);
    assert!(!has(b"a\xE9"));
    assert_eq!(remove(b"a\xE9"), 0);
}

#[test]
#[serial]
fn d6_strings_under_keys_that_are_not_utf8_come_back_byte_for_byte() {
    let _session = Session::begin();
    let cfg = loaded(b"k\xE9 = STRING:v\xE7\n", None);
    put_string(b"p\xE9", b"w\xE7");
    let mut rows = Rows::default();
    rows.check("loaded", string(b"k\xE9"), b"v\xE7".to_vec());
    rows.check("put", string(b"p\xE9"), b"w\xE7".to_vec());
    let want = b"k\xE9 = STRING:v\xE7\np\xE9 = STRING:w\xE7\n";
    let got = saved(&cfg, None, false);
    rows.check("saved sha256", sha256(&got), sha256(want));
    rows.check("saved", got, want.to_vec());
    rows.finish();
}

#[test]
#[serial]
fn d6b_a_type_name_that_is_not_utf8_is_unknown_and_kept_as_bytes() {
    let _session = Session::begin();
    let _cfg = loaded(b"t = \xE9T:foo\n", None);
    let mut rows = Rows::default();
    check_unknown(&mut rows, b"t", b"\xE9T:foo");
    rows.finish();
}

#[test]
#[serial]
fn d7_comments_and_white_space_around_keys_and_values() {
    let _session = Session::begin();
    let _cfg = loaded(
        b"  # comment\n  a  =  INT32:1  # trailing\n b=STRING:x y \n",
        None,
    );
    let mut rows = Rows::default();
    rows.check("a", integer(b"a"), 1);
    rows.check("b", string(b"b"), b"x y".to_vec());
    rows.finish();
}

#[test]
#[serial]
fn d8_a_prefixed_key_is_cut_at_254_bytes_wherever_that_falls() {
    let _session = Session::begin();
    let mut file = vec![b'k'; 246];
    file.extend_from_slice(b"\xE9\xE9 = INT32:1\n");
    let mut utf8 = vec![b'u'; 246];
    utf8.extend_from_slice("é = INT32:2\n".as_bytes());
    file.extend_from_slice(&utf8);
    let _cfg = loaded(&file, Some("config."));

    let mut latin = b"config.".to_vec();
    latin.extend_from_slice(&[b'k'; 246]);
    latin.push(0xE9);
    assert_eq!(latin.len(), 254);
    let mut cut_utf8 = b"config.".to_vec();
    cut_utf8.extend_from_slice(&[b'u'; 246]);
    cut_utf8.push(0xC3);
    assert_eq!(cut_utf8.len(), 254);

    let mut rows = Rows::default();
    rows.check("latin-1 cut", has(&latin), true);
    rows.check("latin-1 value", integer(&latin), 1);
    latin.push(0xE9);
    rows.check("latin-1 uncut", has(&latin), false);
    rows.check("utf-8 cut", has(&cut_utf8), true);
    rows.check("utf-8 value", integer(&cut_utf8), 2);
    rows.finish();
}

#[test]
#[serial]
fn d9_the_file_ends_at_its_first_nul() {
    let _session = Session::begin();
    let _cfg = loaded(b"a = INT32:1\n\0b = INT32:2\n", None);
    assert!(has(b"a"));
    assert!(!has(b"b"));
}

#[test]
#[serial]
fn d10_the_last_writer_wins_and_may_change_the_kind_of_entry() {
    let _session = Session::begin();
    let cfg = loaded(b"dup = STRING:one\ndup = FUTURE:two\ndup = INT32:3\n", None);
    let mut rows = Rows::default();
    rows.check("type", type_of(b"dup"), Some(b"INT32".to_vec()));
    rows.check("value", integer(b"dup"), 3);
    rows.check("GetString", string(b"dup"), Vec::new());

    std::fs::write(cfg.dir.path().join("second.rmp"), b"dup = other\n").unwrap();
    res::load(&cfg.root, "second.rmp", None);
    check_unknown(&mut rows, b"dup", b"other");
    rows.check("GetInteger after unknown", integer(b"dup"), 0);

    rows.check("remove", remove(b"dup"), 1);
    rows.check("gone", has(b"dup"), false);
    rows.check("remove again", remove(b"dup"), 0);

    res::load(&cfg.root, "t.rmp", None);
    rows.check("reloaded", integer(b"dup"), 3);
    res::uninit();
    rows.check("after uninit", has(b"dup"), false);
    rows.finish();
}

#[test]
#[serial]
fn d11_the_root_filter_and_strip_work_on_bytes() {
    let _session = Session::begin();
    let cfg = loaded(b"caf\xE9.a = INT32:1\nother = INT32:2\n", None);
    let mut rows = Rows::default();
    rows.check(
        "root",
        saved(&cfg, Some(b"caf\xE9"), false),
        b"caf\xE9.a = INT32:1\n".to_vec(),
    );
    rows.check(
        "root, stripped",
        saved(&cfg, Some(b"caf\xE9"), true),
        b".a = INT32:1\n".to_vec(),
    );
    rows.finish();
}

#[test]
#[serial]
fn d12_freeing_an_unknown_entry_leaves_it_in_place() {
    let _session = Session::begin();
    let _cfg = loaded(b"future = FUTURE:foo\n", None);
    let first = resource_text(b"future").expect("unknown entries answer GetResource");
    let second = resource_text(b"future").expect("unknown entries answer GetResource");
    assert_eq!(first.1, second.1);
    let name = key(b"future");
    unsafe {
        res_FreeResource(name.as_ptr());
        res_FreeResource(name.as_ptr());
    }
    let mut rows = Rows::default();
    check_unknown(&mut rows, b"future", b"FUTURE:foo");
    rows.finish();
}
