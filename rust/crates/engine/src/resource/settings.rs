//! Application-owned preference and binding strings and their historical file encoding.
use super::index::{self, Entry};
use crate::io::Dir;
use std::{
    collections::HashMap,
    ffi::CStr,
    io::{self, Write},
};

#[derive(Debug, Default)]
pub struct Settings {
    entries: HashMap<String, Entry>,
}

impl Settings {
    pub fn has_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.value(key, "BOOLEAN").map(|value| value != 0)
    }
    pub fn integer(&self, key: &str) -> Option<i32> {
        self.value(key, "INT32").map(|value| value as i32)
    }
    fn value(&self, key: &str, kind: &str) -> Option<u32> {
        self.entries
            .get(key)
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.value)
    }
    pub fn string(&self, key: &str) -> Option<String> {
        self.entries
            .get(key)
            .filter(|entry| entry.kind == "STRING")
            .map(|entry| entry.descriptor.clone())
    }
    pub fn put_boolean(&mut self, key: &str, value: bool) {
        self.put(key, "BOOLEAN", if value { "true" } else { "false" });
    }
    pub fn put_integer(&mut self, key: &str, value: i32) {
        self.put(key, "INT32", &value.to_string());
    }
    pub fn put_string(&mut self, key: &str, value: &str) {
        self.put(key, "STRING", value);
    }
    fn put(&mut self, key: &str, kind: &str, descriptor: &str) {
        assert!(
            !key.contains('\0') && !descriptor.contains('\0'),
            "setting contains NUL"
        );
        self.define(key, &format!("{kind}:{descriptor}"));
    }
    pub(super) fn define(&mut self, key: &str, value: &str) {
        index::define(&mut self.entries, key, value);
    }
    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }
    pub fn load_index(&mut self, directory: &Dir, file: &CStr, prefix: Option<&CStr>) {
        index::read_index(directory, file, prefix, |key, value| {
            self.define(key, value)
        });
    }
    pub fn save(&self, directory: &Dir, file: &CStr, prefix: &str) -> io::Result<()> {
        let mut stream = directory.create(file)?;
        for line in index::serialize_entries(&self.entries, prefix, true) {
            stream.write_all(line.as_bytes())?;
        }
        stream.finish()
    }
}

/// Authored gestures and template names, distinct from applied input state.
#[derive(Debug, Default)]
pub struct Bindings {
    entries: Settings,
}
impl Bindings {
    pub fn has_key(&self, key: &str) -> bool {
        self.entries.has_key(key)
    }
    pub fn string(&self, key: &str) -> Option<String> {
        self.entries.string(key)
    }
    pub fn put_string(&mut self, key: &str, value: &str) {
        self.entries.put_string(key, value);
    }
    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key)
    }
    pub fn load_index(&mut self, directory: &Dir, file: &CStr, prefix: Option<&CStr>) {
        self.entries.load_index(directory, file, prefix);
    }
    pub(super) fn define(&mut self, key: &str, value: &str) {
        self.entries.define(key, value);
    }
    pub fn save(&self, directory: &Dir, file: &CStr, prefix: &str) -> io::Result<()> {
        self.entries.save(directory, file, prefix)
    }
}
