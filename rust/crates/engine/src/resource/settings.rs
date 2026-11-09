//! Typed resource settings and persistence through a retained directory.
use super::ffi_bridge::{serialize_index, with_dispatch};
use crate::io::uio_owned::Directory;
use std::{
    ffi::CStr,
    io::{self, Write},
};

#[derive(Clone, Copy, Default)]
pub struct Settings;
impl Settings {
    pub fn boolean(&self, key: &str) -> Option<bool> {
        with_dispatch(|dispatch| {
            (dispatch.get_resource_type(key) == Some("BOOLEAN"))
                .then(|| dispatch.get_boolean_resource(key))
                .flatten()
        })
    }
    pub fn integer(&self, key: &str) -> Option<i32> {
        with_dispatch(|dispatch| {
            (dispatch.get_resource_type(key) == Some("INT32"))
                .then(|| dispatch.get_int_resource(key).map(|value| value as i32))
                .flatten()
        })
    }
    pub fn string(&self, key: &str) -> Option<String> {
        with_dispatch(|dispatch| {
            (dispatch.get_resource_type(key) == Some("STRING"))
                .then(|| dispatch.get_string_resource(key).map(str::to_owned))
                .flatten()
        })
    }
    pub fn put_boolean(&self, key: &str, value: bool) {
        self.put(key, "BOOLEAN", if value { "true" } else { "false" });
    }
    pub fn put_integer(&self, key: &str, value: i32) {
        self.put(key, "INT32", &value.to_string());
    }
    pub fn put_string(&self, key: &str, value: &str) {
        self.put(key, "STRING", value);
    }
    fn put(&self, key: &str, kind: &str, value: &str) {
        assert!(
            !key.contains('\0') && !value.contains('\0'),
            "setting contains NUL"
        );
        with_dispatch(|dispatch| dispatch.process_resource_desc(key, &format!("{kind}:{value}")));
    }
    pub fn save(&self, directory: &Directory, file: &CStr, prefix: &str) -> io::Result<()> {
        let lines = serialize_index(Some(prefix), true);
        let mut stream = directory.create(file)?;
        for line in lines {
            stream.write_all(line.as_bytes())?;
        }
        stream.finish()
    }
}
