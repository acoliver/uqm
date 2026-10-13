//! The resource API as the C game calls it: index loading and the typed
//! getters, by key.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

use crate::resource::ffi_bridge::{
    res_GetBoolean, res_GetColor, res_GetInteger, res_GetResourceType, res_GetString, res_HasKey,
    res_IsBoolean, res_IsColor, res_IsInteger, res_IsString, InstallResTypeVectors,
    LoadResourceIndex, SaveResourceIndex, UninitResourceSystem,
};
use crate::resource::ffi_types::ResourceData;

use super::uio_api::{cstr, Dir};

/// The heap types the C subsystems register when the game starts
/// (`sresins.c`, `resgfx.c`, `resinst.c`, `vresins.c`, `dummy.c`). The index
/// loader keeps a type only if it is registered, so a row that expects
/// `GFXRES` to stay `GFXRES` needs them. Nothing is ever loaded from them.
const SUBSYSTEM_TYPES: [&CStr; 9] = [
    c"3DOVID",
    c"STRTAB",
    c"BINTAB",
    c"CONVERSATION",
    c"GFXRES",
    c"FONTRES",
    c"SNDRES",
    c"MUSICRES",
    c"SHIP",
];

unsafe extern "C" fn load_nothing(_descriptor: *const c_char, _data: *mut ResourceData) {}

unsafe extern "C" fn free_nothing(_data: *mut c_void) -> c_int {
    1
}

fn register_subsystem_types() {
    for name in SUBSYSTEM_TYPES {
        // A second load in the same session finds the types registered, and
        // registering a key twice is refused.
        let key = [b"sys.".as_slice(), name.to_bytes()].concat();
        let key = CString::new(key).expect("type names have no NUL");
        if unsafe { res_HasKey(key.as_ptr()) } != 0 {
            continue;
        }
        let installed = unsafe {
            InstallResTypeVectors(name.as_ptr(), Some(load_nothing), Some(free_nothing), None)
        };
        assert_eq!(installed, 1, "{name:?} is registered");
    }
}

pub(super) fn load(dir: &Dir, file: &str, prefix: Option<&str>) {
    register_subsystem_types();
    let file = cstr(file);
    let prefix = prefix.map(cstr);
    unsafe {
        LoadResourceIndex(
            dir.as_resource_dir(),
            file.as_ptr(),
            prefix.as_ref().map_or(ptr::null(), |p| p.as_ptr()),
        );
    }
}

pub(super) fn save(dir: &Dir, file: &str, root: Option<&str>) {
    let file = cstr(file);
    let root = root.map(cstr);
    unsafe {
        SaveResourceIndex(
            dir.as_resource_dir(),
            file.as_ptr(),
            root.as_ref().map_or(ptr::null(), |p| p.as_ptr()),
            0,
        );
    }
}

pub(super) fn uninit() {
    unsafe { UninitResourceSystem() };
}

pub(super) fn has(key: &str) -> bool {
    let key = cstr(key);
    unsafe { res_HasKey(key.as_ptr()) != 0 }
}

pub(super) fn type_of(key: &str) -> Option<String> {
    let key = cstr(key);
    unsafe {
        let ty: *const c_char = res_GetResourceType(key.as_ptr());
        (!ty.is_null()).then(|| CStr::from_ptr(ty).to_string_lossy().into_owned())
    }
}

/// The bytes `res_GetString` returns (a C string, so no NUL inside).
pub(super) fn string(key: &str) -> Vec<u8> {
    let key = cstr(key);
    unsafe {
        CStr::from_ptr(res_GetString(key.as_ptr()))
            .to_bytes()
            .to_vec()
    }
}

pub(super) fn text(key: &str) -> String {
    String::from_utf8(string(key)).expect("fixture value is UTF-8")
}

pub(super) fn integer(key: &str) -> i32 {
    let key = cstr(key);
    unsafe { res_GetInteger(key.as_ptr()) }
}

pub(super) fn boolean(key: &str) -> bool {
    let key = cstr(key);
    unsafe { res_GetBoolean(key.as_ptr()) != 0 }
}

pub(super) fn color(key: &str) -> u32 {
    let key = cstr(key);
    unsafe { res_GetColor(key.as_ptr()) }
}

/// `res_IsString`, `res_IsInteger`, `res_IsBoolean`, `res_IsColor` in order.
pub(super) fn kinds(key: &str) -> [bool; 4] {
    let key = cstr(key);
    unsafe {
        [
            res_IsString(key.as_ptr()) != 0,
            res_IsInteger(key.as_ptr()) != 0,
            res_IsBoolean(key.as_ptr()) != 0,
            res_IsColor(key.as_ptr()) != 0,
        ]
    }
}

/// `(key, value)` pairs of a `.rmp`/`.cfg` text, split the plain way: one
/// entry per non-blank, non-comment line, cut at the first `=`, both sides
/// trimmed. Independent of the production byte scanner on purpose.
pub(super) fn oracle_entries(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (key, value) = line.split_once('=').expect("every fixture line has '='");
            (key.trim().to_string(), value.trim().to_string())
        })
        .collect()
}

pub(super) fn oracle_type(value: &str) -> &str {
    value.split_once(':').map_or("", |(ty, _)| ty)
}

/// Collects row mismatches so one run reports every wrong row.
#[derive(Default)]
pub(super) struct Rows(Vec<String>);

impl Rows {
    pub(super) fn check<T: PartialEq + std::fmt::Debug>(&mut self, row: &str, got: T, want: T) {
        if got != want {
            self.0.push(format!("{row}: got {got:?}, want {want:?}"));
        }
    }

    pub(super) fn finish(self) {
        assert!(self.0.is_empty(), "wrong rows:\n{}", self.0.join("\n"));
    }
}
