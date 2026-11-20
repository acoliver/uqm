//! Control template names.

pub use super::sdl_input::{input_templates, rename_template};
use std::ffi::{CStr, CString};

pub(super) fn bounded_template_name(name: &CStr) -> CString {
    let text = name.to_string_lossy();
    let mut length = 0;
    for character in text.chars() {
        if length + character.len_utf8() > 29 {
            break;
        }
        length += character.len_utf8();
    }
    CString::new(&text.as_bytes()[..length])
        .unwrap_or_else(|_| unreachable!("terminated template name"))
}
