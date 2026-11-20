//! Typed resource settings and persistence through a retained directory.
use super::index;
use crate::io::Dir;
use std::{
    ffi::CStr,
    io::{self, Write},
};

#[derive(Clone, Copy, Default)]
pub struct Settings;
impl Settings {
    pub fn kind(&self, key: &CStr) -> Option<String> {
        index::kind(key.to_str().ok()?)
    }

    pub(crate) fn descriptor(&self, key: &CStr, expected: &str) -> Option<String> {
        index::descriptor(key, expected)
    }

    pub fn has_key(&self, key: &str) -> bool {
        index::has_key(key)
    }
    pub fn boolean(&self, key: &str) -> Option<bool> {
        index::value(key, "BOOLEAN").map(|value| value != 0)
    }
    pub fn integer(&self, key: &str) -> Option<i32> {
        index::value(key, "INT32").map(|value| value as i32)
    }
    pub fn string(&self, key: &str) -> Option<String> {
        index::string(key)
    }
    pub fn put_boolean(&self, key: &str, value: bool) {
        index::put(key, "BOOLEAN", if value { "true" } else { "false" });
    }
    pub fn put_integer(&self, key: &str, value: i32) {
        index::put(key, "INT32", &value.to_string());
    }
    pub fn put_string(&self, key: &str, value: &str) {
        index::put(key, "STRING", value);
    }
    pub fn remove(&self, key: &str) -> bool {
        index::remove(key)
    }
    pub fn save(&self, directory: &Dir, file: &CStr, prefix: &str) -> io::Result<()> {
        let lines = index::serialize(prefix, true);
        let mut stream = directory.create(file)?;
        for line in lines {
            stream.write_all(line.as_bytes())?;
        }
        stream.finish()
    }
}
