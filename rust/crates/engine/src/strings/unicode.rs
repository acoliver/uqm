//! UTF-8 helpers used by the Rust engine.
//!
//! Decoding is strict: overlong forms and truncated sequences yield `0`
//! after resynchronising on the next non-continuation byte.

use crate::log_abi::log_message;
use crate::logging::LogLevel;
use std::ffi::c_int;

pub type UniChar = u32;

/// Decode one character from `cursor` and advance past it; the end of the
/// slice reads as NUL. Bad encodings yield `0` after resynchronising on the
/// next non-continuation byte.
pub fn next_char(cursor: &mut &[u8]) -> UniChar {
    fn byte(cursor: &[u8]) -> u8 {
        cursor.first().copied().unwrap_or(0)
    }
    fn advance(cursor: &mut &[u8]) {
        *cursor = cursor.get(1..).unwrap_or(&[]);
    }
    fn bad(cursor: &mut &[u8]) -> UniChar {
        log_message(LogLevel::Warning, "Warning: Invalid UTF8 sequence.");
        while byte(cursor) & 0xc0 == 0x80 {
            advance(cursor);
        }
        0
    }
    let lead = byte(cursor);
    if lead < 0x80 {
        advance(cursor);
        return UniChar::from(lead);
    }
    let (continuations, lead_bits, min) = if (lead & 0xe0) == 0xc0 {
        (1, lead & 0x1f, 0x80)
    } else if (lead & 0xf0) == 0xe0 {
        (2, lead & 0x0f, 0x800)
    } else if (lead & 0xf8) == 0xf0 {
        (3, lead & 0x07, 0x10000)
    } else {
        return bad(cursor);
    };
    let mut result = UniChar::from(lead_bits);
    advance(cursor);
    for _ in 0..continuations {
        if byte(cursor) & 0xc0 != 0x80 {
            return bad(cursor);
        }
        result = (result << 6) | UniChar::from(byte(cursor) & 0x3f);
        advance(cursor);
    }
    if result < min {
        return bad(cursor);
    }
    result
}

/// Counts the valid prefix using the engine decoder's accepted code points.
pub fn count_utf8_bytes(bytes: &[u8]) -> usize {
    utf8_prefix(bytes, usize::MAX).0
}

/// Splits at a bounded character prefix accepted by the engine decoder.
pub fn split_utf8_prefix(bytes: &[u8], characters: usize) -> (&[u8], &[u8]) {
    bytes.split_at(utf8_prefix(bytes, characters).1)
}

/// Compare a bounded prefix using the engine's decoder and native case rules.
pub fn case_prefix_matches(mut source: &[u8], mut destination: &[u8], count: usize) -> bool {
    for _ in 0..count {
        let Some((first, first_width)) = utf8_character(source) else {
            return false;
        };
        let Some((second, second_width)) = utf8_character(destination) else {
            return false;
        };
        if uni_char_to_upper(first) != uni_char_to_upper(second) {
            return false;
        }
        source = &source[first_width..];
        destination = &destination[second_width..];
    }
    true
}

fn utf8_prefix(bytes: &[u8], limit: usize) -> (usize, usize) {
    let mut cursor = 0;
    let mut count = 0;
    while count < limit {
        let Some((_, width)) = utf8_character(&bytes[cursor..]) else {
            break;
        };
        cursor += width;
        count += 1;
    }
    (count, cursor)
}

/// Decode the engine's accepted UTF-8 prefix, including non-scalar code points.
pub fn utf8_character(bytes: &[u8]) -> Option<(UniChar, usize)> {
    let (width, mut value, minimum) = match *bytes.first()? {
        1..=0x7f => (1, u32::from(bytes[0]), 0),
        0xc0..=0xdf => (2, u32::from(bytes[0] & 0x1f), 0x80),
        0xe0..=0xef => (3, u32::from(bytes[0] & 0x0f), 0x800),
        0xf0..=0xf7 => (4, u32::from(bytes[0] & 0x07), 0x10000),
        _ => return None,
    };
    let encoded = bytes.get(..width)?;
    let mut valid = true;
    for &byte in &encoded[1..] {
        if byte & 0xc0 != 0x80 {
            valid = false;
            break;
        }
        value = (value << 6) | u32::from(byte & 0x3f);
    }
    if !valid || value < minimum {
        log_message(LogLevel::Warning, "Warning: Invalid UTF8 sequence.");
        return None;
    }
    Some((value, width))
}

/// Every non-control character except the private use area (0xE000-0xF8FF).
pub fn uni_char_is_graph(ch: UniChar) -> c_int {
    c_int::from((ch > 0xa0 && !(0xe000..=0xf8ff).contains(&ch)) || (ch > 0x20 && ch < 0x7f))
}

pub fn uni_char_is_print(ch: UniChar) -> c_int {
    c_int::from(ch == 0x20 || uni_char_is_graph(ch) != 0)
}

/// Upper case as the C library's default locale defines it: ASCII letters only.
pub fn uni_char_to_upper(ch: UniChar) -> UniChar {
    match char::from_u32(ch) {
        Some(c) if c.is_ascii_lowercase() => UniChar::from(c.to_ascii_uppercase()),
        _ => ch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(mut bytes: &[u8]) -> Vec<UniChar> {
        let mut out = Vec::new();
        while !bytes.is_empty() {
            out.push(next_char(&mut bytes));
        }
        out
    }

    #[test]
    fn decodes_each_sequence_length() {
        assert_eq!(
            decode_all(b"a\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80"),
            [0x61, 0xe9, 0x20ac, 0x1f600]
        );
    }

    #[test]
    fn overlong_and_truncated_sequences_decode_to_zero_and_resync() {
        // Overlong 'A', then a lead byte followed by ASCII, then stray
        // continuation bytes.
        let mut cursor = &b"\xc1\x81Z\xe2\x82Y\x80\x80X"[..];
        assert_eq!(next_char(&mut cursor), 0);
        assert_eq!(cursor[0], b'Z');
        cursor = &cursor[1..];
        assert_eq!(next_char(&mut cursor), 0);
        assert_eq!(cursor[0], b'Y');
        cursor = &cursor[1..];
        assert_eq!(next_char(&mut cursor), 0);
        assert_eq!(cursor, b"X");
    }

    #[test]
    fn the_end_of_the_slice_reads_as_nul() {
        let mut cursor = &b"\xe2\x82"[..];
        assert_eq!(next_char(&mut cursor), 0);
        assert!(cursor.is_empty());
        assert_eq!(next_char(&mut cursor), 0);
    }

    #[test]
    fn character_classes() {
        assert_eq!(uni_char_is_graph(0x20), 0);
        assert_eq!(uni_char_is_graph(0x41), 1);
        assert_eq!(uni_char_is_graph(0x7f), 0);
        assert_eq!(uni_char_is_graph(0xa0), 0);
        assert_eq!(uni_char_is_graph(0xa1), 1);
        assert_eq!(uni_char_is_graph(0xe000), 0);
        assert_eq!(uni_char_is_graph(0xf8ff), 0);
        assert_eq!(uni_char_is_graph(0xf900), 1);
        assert_eq!(uni_char_is_print(0x20), 1);
        assert_eq!(uni_char_is_print(0x0a), 0);
        assert_eq!(uni_char_to_upper(u32::from(b'a')), u32::from(b'A'));
        assert_eq!(uni_char_to_upper(u32::from(b'Q')), u32::from(b'Q'));
        assert_eq!(uni_char_to_upper(0xe9), 0xe9);
        assert_eq!(uni_char_to_upper(0x20ac), 0x20ac);
    }
}

#[cfg(test)]
mod bounded_count_tests {
    #[test]
    fn splits_bounded_prefixes_with_the_same_legacy_code_point_rules() {
        let text = b"A\xc3\xa9\xed\xa0\x80\xf7\xbf\xbf\xbfZ\xe9";
        assert_eq!(super::split_utf8_prefix(text, 0), (&b""[..], &text[..]));
        assert_eq!(
            super::split_utf8_prefix(text, 4),
            (&text[..10], &text[10..])
        );
        assert_eq!(
            super::split_utf8_prefix(text, 99),
            (&text[..11], &text[11..])
        );
        assert_eq!(
            super::split_utf8_prefix(b"a\0b", 99),
            (&b"a"[..], &b"\0b"[..])
        );
        assert_eq!(
            super::split_utf8_prefix(b"\xe2\x82", 1),
            (&b""[..], &b"\xe2\x82"[..])
        );
    }

    #[test]
    fn counts_preserve_legacy_prefix_and_code_point_rules() {
        for (bytes, expected) in [
            (&b"ab\0cd\0"[..], 2),
            (c"éclair".to_bytes_with_nul(), 6),
            (&b"a\xe2\x28\xa1\0"[..], 1),
            (&b"a\xc0\x80\0"[..], 1),
            (&b"\xed\xa0\x80\0"[..], 1),
            (&b"\xf7\xbf\xbf\xbf\0"[..], 1),
            (&b"\xe2\x82"[..], 0),
            (&b""[..], 0),
        ] {
            assert_eq!(super::count_utf8_bytes(bytes), expected, "{bytes:?}");
        }
    }
}

#[cfg(test)]
mod bounded_prefix_tests {
    use super::*;

    #[test]
    fn bounded_prefix_uses_native_case_and_rejects_missing_or_malformed_characters() {
        assert!(case_prefix_matches(b"aLp", b"ALPHA", 3));
        assert!(!case_prefix_matches(b"Alphaa", b"Alpha", 6));
        assert!(!case_prefix_matches(b"", b"", 1));
        assert!(!case_prefix_matches(&[0xc0, 0x80], &[0xc0, 0x80], 1));
        assert!(!case_prefix_matches("β".as_bytes(), "Β".as_bytes(), 1));
        assert!(!case_prefix_matches("é".as_bytes(), "É".as_bytes(), 1));
        assert!(case_prefix_matches(
            "α Centauri".as_bytes(),
            "α centauri".as_bytes(),
            3
        ));
    }
}
