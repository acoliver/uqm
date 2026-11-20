//! The resource index, ported from resinit.c and getres.c (GPL-2.0-or-later).
//!
//! Every key the loaded `.rmp` files and the configuration define, with its
//! type and descriptor (`TYPE:descriptor`). Value types hold their parsed
//! value here. Every other type names a file: its owner reads the path from
//! the index and loads an owned value it releases by dropping, so the index
//! never holds loaded data.
use super::color::{parse_c_color, serialize_color};
use super::propfile::parse_propfile;
use crate::host::Files;
use crate::io::Dir;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;

/// Types whose descriptor is a file their owner loads.
const FILE_TYPES: [&str; 8] = [
    "GFXRES",
    "FONTRES",
    "STRTAB",
    "BINTAB",
    "CONVERSATION",
    "SNDRES",
    "MUSICRES",
    "3DOVID",
];

/// One index entry: its type name, its descriptor, and for the integer-like
/// value types (`INT32`, `BOOLEAN`, `COLOR`) the parsed value.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    kind: String,
    descriptor: String,
    value: u32,
}

impl Entry {
    fn parse(kind: &str, descriptor: &str) -> Self {
        let value = match kind {
            // atoi semantics: 0 when the descriptor is not a number
            "INT32" => descriptor.trim().parse::<i32>().unwrap_or(0) as u32,
            "BOOLEAN" => u32::from(descriptor.trim().eq_ignore_ascii_case("true")),
            "COLOR" => {
                parse_c_color(descriptor).map_or(0, |(r, g, b, a)| u32::from_be_bytes([r, g, b, a]))
            }
            _ => 0,
        };
        Self {
            kind: kind.to_owned(),
            descriptor: descriptor.to_owned(),
            value,
        }
    }

    /// The index-file form of a value entry; file entries are not saved.
    fn serialize(&self) -> Option<String> {
        match self.kind.as_str() {
            "STRING" => Some(self.descriptor.clone()),
            "INT32" => Some((self.value as i32).to_string()),
            "BOOLEAN" => Some(if self.value != 0 { "true" } else { "false" }.to_owned()),
            "COLOR" => {
                let [r, g, b, a] = self.value.to_be_bytes();
                Some(serialize_color(r, g, b, a))
            }
            _ => None,
        }
    }
}

static INDEX: Mutex<Option<HashMap<String, Entry>>> = Mutex::new(None);

fn with_index<R>(access: impl FnOnce(&mut HashMap<String, Entry>) -> R) -> R {
    let mut index = INDEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    access(index.get_or_insert_with(HashMap::new))
}

/// Define `key` from a `TYPE:descriptor` index value, replacing any entry.
fn define(index: &mut HashMap<String, Entry>, key: &str, value: &str) {
    let Some((kind, descriptor)) = value.split_once(':') else {
        log::warn!("Invalid resource descriptor '{value}' for key '{key}'");
        return;
    };
    if !matches!(kind, "STRING" | "INT32" | "BOOLEAN" | "COLOR") && !FILE_TYPES.contains(&kind) {
        log::warn!("Unknown resource type '{kind}' for key '{key}', storing as UNKNOWNRES");
    }
    index.insert(key.to_owned(), Entry::parse(kind, descriptor));
}

/// Start an empty index (C: `InitResourceSystem`); an index already started
/// is kept.
pub fn init() {
    with_index(|_| ());
}

/// Drop every entry (C: `UninitResourceSystem`).
pub fn uninit() {
    *INDEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

/// Read the index file `file` in `dir`, prefixing every key with `prefix`
/// (C: `LoadResourceIndex`). A missing, empty or non-UTF-8 file adds nothing.
pub fn load_index(dir: &Dir, file: &CStr, prefix: Option<&CStr>) {
    let Ok(name) = file.to_str() else {
        return;
    };
    let mut text = String::new();
    let read = dir
        .open_read(Path::new(name))
        .and_then(|mut stream| stream.read_to_string(&mut text));
    if let Err(error) = read {
        log::warn!("load_resource_index: cannot read {name}: {error}");
        return;
    }
    let prefix = prefix.and_then(|prefix| prefix.to_str().ok());
    with_index(|index| {
        parse_propfile(&text, &mut |key, value| define(index, key, value), prefix);
    });
}

/// The index lines of the value entries under `prefix`, sorted by key and
/// with the prefix removed when `strip` (C: `SaveResourceIndex`).
pub(super) fn serialize(prefix: &str, strip: bool) -> Vec<String> {
    with_index(|index| {
        let mut keys: Vec<_> = index.keys().filter(|key| key.starts_with(prefix)).collect();
        keys.sort();
        keys.into_iter()
            .filter_map(|key| {
                let entry = &index[key];
                let value = entry.serialize()?;
                let key = if strip {
                    key.strip_prefix(prefix).unwrap_or(key)
                } else {
                    key
                };
                Some(format!("{key} = {}:{value}\n", entry.kind))
            })
            .collect()
    })
}

/// The type name of `key`.
pub fn kind(key: &str) -> Option<String> {
    with_index(|index| index.get(key).map(|entry| entry.kind.clone()))
}

/// The descriptor of `key`, which must be of type `expected`.
pub fn descriptor(key: &CStr, expected: &str) -> Option<String> {
    let key = key.to_str().ok()?;
    with_index(|index| {
        let entry = index.get(key)?;
        assert_eq!(entry.kind, expected, "unexpected resource type for {key}");
        Some(entry.descriptor.clone())
    })
}

/// The file `key` names, which must be of type `expected`.
pub fn path_of(key: &CStr, expected: &str) -> Option<CString> {
    descriptor(key, expected).and_then(|path| CString::new(path).ok())
}

/// The value of a `STRING` entry.
pub fn string(key: &str) -> Option<String> {
    with_index(|index| {
        let entry = index.get(key).filter(|entry| entry.kind == "STRING")?;
        Some(entry.descriptor.clone())
    })
}

/// The value of an entry of type `kind` (`INT32`, `BOOLEAN` or `COLOR`).
pub(super) fn value(key: &str, kind: &str) -> Option<u32> {
    with_index(|index| {
        let entry = index.get(key).filter(|entry| entry.kind == kind)?;
        Some(entry.value)
    })
}

pub fn has_key(key: &str) -> bool {
    with_index(|index| index.contains_key(key))
}

/// Define `key` as `kind:descriptor` (C: `res_Put*`).
pub(super) fn put(key: &str, kind: &str, descriptor: &str) {
    assert!(
        !key.contains('\0') && !descriptor.contains('\0'),
        "setting contains NUL"
    );
    with_index(|index| define(index, key, &format!("{kind}:{descriptor}")));
}

/// Remove `key`; `false` when it was not defined (C: `res_Remove`).
pub fn remove(key: &str) -> bool {
    with_index(|index| index.remove(key).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn fresh() {
        uninit();
        init();
    }

    #[test]
    #[serial]
    fn index_files_define_typed_values_and_file_entries() {
        fresh();
        let mount = crate::io::fixture::TempMount::new();
        mount.write(
            "a.rmp",
            b"name = STRING:hello\ncount = INT32:-7\nflag = BOOLEAN:TRUE\n\
              tint = COLOR:rgb15(31, 0, 16)\ntext = STRTAB:base/text.txt\n\
              odd = WEIRD:thing\nbroken\n",
        );
        load_index(&mount.dir(), c"a.rmp", Some(c"p."));
        assert_eq!(string("p.name").as_deref(), Some("hello"));
        assert_eq!(value("p.count", "INT32"), Some(-7i32 as u32));
        assert_eq!(value("p.flag", "BOOLEAN"), Some(1));
        assert_eq!(value("p.tint", "COLOR"), Some(0xff00_84ff));
        assert_eq!(value("p.name", "INT32"), None);
        assert_eq!(string("p.count"), None);
        assert_eq!(
            path_of(c"p.text", "STRTAB").as_deref(),
            Some(c"base/text.txt")
        );
        assert_eq!(kind("p.odd").as_deref(), Some("WEIRD"));
        assert!(!has_key("broken"));
        assert!(std::panic::catch_unwind(|| path_of(c"p.text", "BINTAB")).is_err());
        load_index(&mount.dir(), c"missing.rmp", None);
        uninit();
        assert!(!has_key("p.name"));
    }

    #[test]
    #[serial]
    fn puts_replace_entries_and_serialize_only_values() {
        fresh();
        put("cfg.b", "BOOLEAN", "false");
        put("cfg.a", "INT32", "12");
        put("cfg.c", "COLOR", "rgba(1, 2, 3, 4)");
        put("cfg.d", "STRING", "text");
        put("cfg.e", "STRTAB", "file.txt");
        put("other", "INT32", "1");
        put("cfg.a", "STRING", "replaced");
        assert_eq!(
            serialize("cfg.", true),
            [
                "a = STRING:replaced\n",
                "b = BOOLEAN:false\n",
                "c = COLOR:rgba(0x01, 0x02, 0x03, 0x04)\n",
                "d = STRING:text\n",
            ]
        );
        assert_eq!(serialize("oth", false), ["other = INT32:1\n"]);
        assert!(remove("cfg.a"));
        assert!(!remove("cfg.a"));
        assert!(std::panic::catch_unwind(|| put("bad\0", "STRING", "x")).is_err());
        uninit();
    }
}
