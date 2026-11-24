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
use std::sync::Arc;

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
pub(super) struct Entry {
    pub(super) kind: String,
    pub(super) descriptor: String,
    pub(super) value: u32,
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

pub(super) fn read_index(
    dir: &Dir,
    file: &CStr,
    prefix: Option<&CStr>,
    mut accept: impl FnMut(&str, &str),
) {
    let Ok(name) = file.to_str() else {
        return;
    };
    let mut text = String::new();
    if let Err(error) = dir
        .open_read(Path::new(name))
        .and_then(|mut stream| stream.read_to_string(&mut text))
    {
        log::warn!("load_resource_index: cannot read {name}: {error}");
        return;
    }
    parse_propfile(
        &text,
        &mut accept,
        prefix.and_then(|prefix| prefix.to_str().ok()),
    );
}

pub(super) fn define(index: &mut HashMap<String, Entry>, key: &str, value: &str) {
    if let Some(entry) = parse_entry(key, value) {
        index.insert(key.to_owned(), entry);
    }
}
fn parse_entry(key: &str, value: &str) -> Option<Entry> {
    let Some((kind, descriptor)) = value.split_once(':') else {
        log::warn!("Invalid resource descriptor '{value}' for key '{key}'");
        return None;
    };
    if !matches!(kind, "STRING" | "INT32" | "BOOLEAN" | "COLOR") && !FILE_TYPES.contains(&kind) {
        log::warn!("Unknown resource type '{kind}' for key '{key}', storing as UNKNOWNRES");
    }
    Some(Entry::parse(kind, descriptor))
}
pub(super) fn serialize_entries(
    index: &HashMap<String, Entry>,
    prefix: &str,
    strip: bool,
) -> Vec<String> {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Graphic,
    Font,
    Text,
    Binary,
    Conversation,
    Sound,
    Music,
    Video,
}
impl AssetKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Graphic => "GFXRES",
            Self::Font => "FONTRES",
            Self::Text => "STRTAB",
            Self::Binary => "BINTAB",
            Self::Conversation => "CONVERSATION",
            Self::Sound => "SNDRES",
            Self::Music => "MUSICRES",
            Self::Video => "3DOVID",
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexSource {
    pub directory: Dir,
    pub index: CString,
}
#[derive(Clone)]
struct Definition {
    entry: Entry,
    source: IndexSource,
}

/// Identity includes the run, so two first revisions cannot alias.
#[derive(Clone, Debug)]
pub struct Revision {
    session: Arc<()>,
    generation: u64,
    identity: Arc<()>,
}
impl PartialEq for Revision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.session, &other.session) && Arc::ptr_eq(&self.identity, &other.identity)
    }
}
impl Eq for Revision {}

pub struct CatalogBuilder {
    entries: HashMap<String, Definition>,
    files: Dir,
    revision: Revision,
}
impl CatalogBuilder {
    pub fn new(files: Dir) -> Self {
        Self {
            entries: HashMap::new(),
            files: files.read_only(),
            revision: Revision {
                session: Arc::new(()),
                generation: 0,
                identity: Arc::new(()),
            },
        }
    }
    pub fn import_index(
        &mut self,
        dir: &Dir,
        file: &CStr,
        prefix: Option<&CStr>,
        settings: &mut super::settings::Settings,
        bindings: &mut super::settings::Bindings,
    ) {
        let source = IndexSource {
            directory: dir.read_only(),
            index: file.to_owned(),
        };
        read_index(dir, file, prefix, |key, value| {
            if key.starts_with("config.") {
                settings.define(key, value);
            } else if key.starts_with("keys.") || key.starts_with("menu.") {
                bindings.define(key, value);
            } else if let Some(entry) = parse_entry(key, value) {
                self.entries.insert(
                    key.to_owned(),
                    Definition {
                        entry,
                        source: source.clone(),
                    },
                );
                self.revision.generation = self
                    .revision
                    .generation
                    .checked_add(1)
                    .expect("catalog revision exhausted");
            }
        });
    }
    pub fn finish(self) -> Content {
        Content(Arc::new(ContentData {
            entries: self.entries,
            files: self.files,
            revision: self.revision,
        }))
    }
    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            self.revision.generation = self
                .revision
                .generation
                .checked_add(1)
                .expect("catalog revision exhausted");
        }
        removed
    }
}
struct ContentData {
    entries: HashMap<String, Definition>,
    files: Dir,
    revision: Revision,
}
#[derive(Clone)]
pub struct Content(Arc<ContentData>);

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("resource {key} is not defined")]
    Missing { key: String },
    #[error("unexpected resource type for {key}: expected {expected}, found {actual}")]
    WrongKind {
        key: String,
        expected: &'static str,
        actual: String,
    },
}
pub struct Resolved<'a> {
    pub kind: AssetKind,
    pub descriptor: &'a str,
    pub source: &'a IndexSource,
    pub revision: &'a Revision,
    pub files: &'a Dir,
}
impl Content {
    pub fn files(&self) -> &Dir {
        &self.0.files
    }
    pub fn revision(&self) -> &Revision {
        &self.0.revision
    }
    pub fn revise(&self) -> CatalogBuilder {
        CatalogBuilder {
            entries: self.0.entries.clone(),
            files: self.0.files.clone(),
            revision: Revision {
                session: self.0.revision.session.clone(),
                generation: self.0.revision.generation,
                identity: Arc::new(()),
            },
        }
    }
    pub fn kind(&self, key: &str) -> Option<&str> {
        self.0
            .entries
            .get(key)
            .map(|entry| entry.entry.kind.as_str())
    }
    pub fn resolve(&self, key: &CStr, expected: AssetKind) -> Result<Resolved<'_>, ResolveError> {
        let key = key.to_string_lossy();
        let definition = self
            .0
            .entries
            .get(key.as_ref())
            .ok_or_else(|| ResolveError::Missing {
                key: key.to_string(),
            })?;
        if definition.entry.kind != expected.name() {
            return Err(ResolveError::WrongKind {
                key: key.to_string(),
                expected: expected.name(),
                actual: definition.entry.kind.clone(),
            });
        }
        Ok(Resolved {
            kind: expected,
            descriptor: &definition.entry.descriptor,
            source: &definition.source,
            revision: &self.0.revision,
            files: &self.0.files,
        })
    }
    pub fn descriptor(&self, key: &CStr, expected: AssetKind) -> Option<&str> {
        match self.resolve(key, expected) {
            Ok(resolved) => Some(resolved.descriptor),
            Err(ResolveError::Missing { .. }) => None,
            Err(error @ ResolveError::WrongKind { .. }) => panic!("{error}"),
        }
    }
    pub fn path_of(&self, key: &CStr, expected: AssetKind) -> Option<CString> {
        CString::new(self.descriptor(key, expected)?).ok()
    }
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
