//! The name of a resource type as C holds it.
//!
//! A type identifier is a C string: any bytes but NUL, in any encoding.
//! `InstallResTypeVectors` keeps the caller's pointer and `SaveResourceIndex`
//! writes it back unchanged, so the name is kept as bytes. The `&str`
//! comparisons are for the built-in names, which are ASCII; they compare bytes
//! and never decode.

use std::ffi::{CStr, CString, NulError};
use std::fmt;
use std::sync::Arc;

/// One registered type identifier, whole (not cut to the 27 bytes the
/// registry key keeps). Clones share the one allocation, so the pointer
/// [`RawResourceType::as_c_str`] gives out stays valid as long as any clone,
/// including the registry's, is alive.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RawResourceType(Arc<CStr>);

impl RawResourceType {
    /// # Errors
    ///
    /// [`NulError`] when `bytes` holds a NUL, which no C string can.
    pub fn new(bytes: impl AsRef<[u8]>) -> Result<Self, NulError> {
        Ok(RawResourceType(CString::new(bytes.as_ref())?.into()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.to_bytes()
    }

    pub fn as_c_str(&self) -> &CStr {
        &self.0
    }
}

impl AsRef<[u8]> for RawResourceType {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq<str> for RawResourceType {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&str> for RawResourceType {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

/// The bytes with non-ASCII ones escaped (`G\xe9`), so two different byte
/// strings never print alike.
impl fmt::Display for RawResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_bytes().escape_ascii())
    }
}

impl fmt::Debug for RawResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RawResourceType(b\"{}\")",
            self.as_bytes().escape_ascii()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_bytes_that_are_not_utf8() {
        let name = RawResourceType::new(b"G\xE9").unwrap();
        assert_eq!(name.as_bytes(), b"G\xE9");
        assert_eq!(name.as_c_str().to_bytes(), b"G\xE9");
    }

    #[test]
    fn refuses_a_nul_byte() {
        assert!(RawResourceType::new(b"A\0B").is_err());
    }

    #[test]
    fn compares_with_ascii_text_byte_for_byte() {
        let name = RawResourceType::new("STRING").unwrap();
        assert_eq!(name, "STRING");
        assert_eq!(name, *"STRING");
        assert_ne!(name, "string");
        let latin = RawResourceType::new(b"\xE9").unwrap();
        assert_ne!(latin, "\u{FFFD}");
        assert_ne!(latin, "\u{E9}");
    }

    #[test]
    fn prints_distinct_bytes_distinctly() {
        let latin = RawResourceType::new(b"a\xE9").unwrap();
        let replacement = RawResourceType::new("a\u{FFFD}").unwrap();
        assert_ne!(latin.to_string(), replacement.to_string());
        assert_eq!(latin.to_string(), "a\\xe9");
    }

    #[test]
    fn clones_share_one_allocation() {
        let name = RawResourceType::new("TYPE").unwrap();
        let copy = name.clone();
        assert!(std::ptr::eq(
            name.as_c_str().as_ptr(),
            copy.as_c_str().as_ptr()
        ));
    }
}
