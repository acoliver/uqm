//! `makeArgs`: splitting a command line into arguments.
//!
//! There is no quoting or escaping: a word is a run of bytes that are not C
//! `isspace` (space, tab, newline, vertical tab, form feed, carriage return).
//! Bytes of 0x80 and up are word bytes, so a non-UTF-8 path survives intact.

use std::ffi::CString;

/// A line with a NUL byte: no C string can hold it, so no command can name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedNul;

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// The arguments of `line`, in order. An empty or all-whitespace line has none.
pub fn make_args(line: &[u8]) -> Result<Vec<CString>, EmbeddedNul> {
    if line.contains(&0) {
        return Err(EmbeddedNul);
    }
    Ok(line
        .split(|&byte| is_space(byte))
        .filter(|word| !word.is_empty())
        .map(|word| CString::new(word).expect("a word of a NUL-free line has no NUL"))
        .collect())
}
