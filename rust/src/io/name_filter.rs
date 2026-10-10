//! Name filters for `uio_getDirList`.
//!
//! Names are raw bytes: a host file name is whatever bytes the operating
//! system holds, and only an archive entry name is guaranteed to be Unicode.
//! Matching therefore never converts a name to `str`.

use std::os::raw::c_int;

use regex::bytes::Regex;

use crate::bridge_log::rust_bridge_log_msg;

pub(super) const MATCH_LITERAL: c_int = 0;
pub(super) const MATCH_PREFIX: c_int = 1;
pub(super) const MATCH_SUFFIX: c_int = 2;
pub(super) const MATCH_SUBSTRING: c_int = 3;
pub(super) const MATCH_REGEX: c_int = 4;

/// A pattern and match type, prepared once for a whole listing.
pub(super) enum NameFilter {
    /// An empty pattern selects every name.
    Any,
    Literal(Vec<u8>),
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    Substring(Vec<u8>),
    /// A regular expression over the name's bytes. `None` is a pattern the
    /// regex engine rejects, which selects nothing. Regex syntax is Unicode,
    /// so a name's invalid UTF-8 bytes are matched only by the parts of a
    /// pattern that do not need a character (`.` does not consume them).
    Regex(Option<Regex>),
}

impl NameFilter {
    /// Prepares `pattern` for `match_type`. An unknown match type is treated
    /// as a regex.
    pub(super) fn new(pattern: &[u8], match_type: c_int) -> Self {
        if pattern.is_empty() {
            return NameFilter::Any;
        }
        let owned = pattern.to_vec();
        match match_type {
            MATCH_LITERAL => NameFilter::Literal(owned),
            MATCH_PREFIX => NameFilter::Prefix(owned),
            MATCH_SUFFIX => NameFilter::Suffix(owned),
            MATCH_SUBSTRING => NameFilter::Substring(owned),
            MATCH_REGEX => NameFilter::Regex(compile(pattern)),
            // An unknown type is treated as a regex, as it always was here.
            _ => NameFilter::Regex(compile(pattern)),
        }
    }

    pub(super) fn matches(&self, name: &[u8]) -> bool {
        match self {
            NameFilter::Any => true,
            NameFilter::Literal(pattern) => name == pattern.as_slice(),
            NameFilter::Prefix(pattern) => name.starts_with(pattern),
            NameFilter::Suffix(pattern) => name.ends_with(pattern),
            NameFilter::Substring(pattern) => {
                name.windows(pattern.len()).any(|window| window == pattern)
            }
            NameFilter::Regex(regex) => regex.as_ref().is_some_and(|re| re.is_match(name)),
        }
    }
}

/// The caller's pattern is external input: one that is not valid UTF-8 or
/// not valid regex syntax matches nothing, and the listing carries on.
fn compile(pattern: &[u8]) -> Option<Regex> {
    let source = std::str::from_utf8(pattern).ok()?;
    match Regex::new(source) {
        Ok(regex) => Some(regex),
        Err(error) => {
            rust_bridge_log_msg(&format!(
                "RUST_UIO: rejected regex pattern {source:?}: {error}"
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(name: &[u8], pattern: &[u8], match_type: c_int) -> bool {
        NameFilter::new(pattern, match_type).matches(name)
    }

    #[test]
    fn an_empty_pattern_selects_every_name_whatever_the_type() {
        for match_type in [MATCH_LITERAL, MATCH_REGEX, 99] {
            assert!(matches(b"anything", b"", match_type));
            assert!(matches(b"\xff", b"", match_type));
        }
    }

    #[test]
    fn plain_matches_compare_bytes_so_invalid_utf8_names_are_selected() {
        let name = b"pack\xff.zip";
        assert!(matches(name, name, MATCH_LITERAL));
        assert!(matches(name, b"pack\xff", MATCH_PREFIX));
        assert!(matches(name, b"\xff.zip", MATCH_SUFFIX));
        assert!(matches(name, b"\xff", MATCH_SUBSTRING));
        assert!(!matches(name, b"pack\xfe", MATCH_PREFIX));
        assert!(!matches(name, b"\xfe", MATCH_SUBSTRING));
        assert!(!matches(b"short", b"much longer pattern", MATCH_SUBSTRING));
    }

    #[test]
    fn a_regex_over_a_name_with_invalid_bytes_matches_what_it_spells_out() {
        let extension = br"\.([zZ][iI][pP]|[uU][qQ][mM])$";
        assert!(matches(b"pack\xff.ZIP", extension, MATCH_REGEX));
        assert!(
            matches(b"pack\xff.uqm", extension, 5),
            "the alternate regex type"
        );
        assert!(!matches(b"pack\xff.txt", extension, MATCH_REGEX));
        assert!(matches("café.zip".as_bytes(), extension, MATCH_REGEX));
    }

    #[test]
    fn a_pattern_the_regex_engine_cannot_use_selects_nothing() {
        assert!(!matches(b"a(b", b"a(b", MATCH_REGEX), "invalid syntax");
        assert!(!matches(b"\xff", b"\xff", MATCH_REGEX), "not UTF-8");
        assert!(!matches(b"\xff", b"\xff", 99), "unknown type acts as regex");
    }
}
