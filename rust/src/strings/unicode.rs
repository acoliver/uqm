//! UTF-8 helpers used by the Rust engine.
//!
//! Engine strings are NUL-terminated byte buffers; pointer operations retain
//! their storage and lifetime contracts. Decoding is strict: overlong
//! forms and truncated sequences yield `0` after resynchronising on the next
//! non-continuation byte.

use std::ffi::{c_char, c_int};

use crate::log_abi::{log_message, LOG_WARNING};

pub type UniChar = u32;

/// Skips continuation bytes (`10xxxxxx`).
unsafe fn resync(ptr: &mut *const u8) {
    while (**ptr & 0xc0) == 0x80 {
        *ptr = ptr.add(1);
    }
}

/// Decodes one character and advances `*ptr` past it. Returns `0` when the
/// encoding is bad; callers tell that apart from a real NUL by checking
/// `**ptr == 0` beforehand.
///
/// # Safety
/// `ptr` and `*ptr` must be valid, and the string NUL-terminated.
pub unsafe fn get_char_from_string(ptr: *mut *const u8) -> UniChar {
    let p = &mut *ptr;
    let lead = **p;
    if lead < 0x80 {
        *p = p.add(1);
        return UniChar::from(lead);
    }

    let (continuations, lead_bits, min) = if (lead & 0xe0) == 0xc0 {
        (1, lead & 0x1f, 0x80)
    } else if (lead & 0xf0) == 0xe0 {
        (2, lead & 0x0f, 0x800)
    } else if (lead & 0xf8) == 0xf0 {
        (3, lead & 0x07, 0x10000)
    } else {
        return bad_sequence(p);
    };

    let mut result = UniChar::from(lead_bits);
    *p = p.add(1);
    for _ in 0..continuations {
        if (**p & 0xc0) != 0x80 {
            return bad_sequence(p);
        }
        result = (result << 6) | UniChar::from(**p & 0x3f);
        *p = p.add(1);
    }
    if result < min {
        return bad_sequence(p);
    }
    result
}

unsafe fn bad_sequence(p: &mut *const u8) -> UniChar {
    log_message(LOG_WARNING, "Warning: Invalid UTF8 sequence.");
    resync(p);
    0
}

/// Like [`get_char_from_string`] but never reads at or past `end`; on any
/// failure `*ptr` is moved to `end`.
///
/// # Safety
/// `ptr` and `*ptr` must be valid and `end` must lie in the same allocation.
pub unsafe fn get_char_from_string_n(ptr: *mut *const u8, end: *const u8) -> UniChar {
    let p = &mut *ptr;
    if *p != end {
        let lead = **p;
        let num_bytes: usize = if lead < 0x80 {
            1
        } else if (lead & 0xe0) == 0xc0 {
            2
        } else if (lead & 0xf0) == 0xe0 {
            3
        } else if (lead & 0xf8) == 0xf0 {
            4
        } else {
            0
        };
        if num_bytes != 0 && (end as usize).wrapping_sub(*p as usize) >= num_bytes {
            return get_char_from_string(ptr);
        }
    }
    *p = end;
    0
}

/// Finds the next line (terminated by CRLF, LFCR, LF or CR). Returns `start`
/// and sets `*end` to the line end and `*start_next` to the next line start,
/// or returns NULL if the string is not valid UTF-8.
///
/// # Safety
/// All pointers must be valid and `start` NUL-terminated.
pub unsafe fn get_line_from_string(
    start: *const u8,
    end: *mut *const u8,
    start_next: *mut *const u8,
) -> *mut u8 {
    let mut ptr = start;
    loop {
        if *ptr == 0 {
            *end = ptr;
            *start_next = ptr;
            return start as *mut u8;
        }
        let last = ptr;
        let ch = get_char_from_string(&mut ptr);
        if ch == 0 {
            *end = last;
            *start_next = ptr;
            return std::ptr::null_mut();
        }
        if ch == UniChar::from(b'\n') {
            *end = last;
            if *ptr == 0 {
                *start_next = ptr;
                return start as *mut u8;
            }
            let ch = get_char_from_string(&mut ptr);
            if ch == 0 {
                return std::ptr::null_mut();
            }
            *start_next = if ch == UniChar::from(b'\r') {
                ptr
            } else {
                *end
            };
            return start as *mut u8;
        } else if ch == UniChar::from(b'\r') {
            *end = last;
            *start_next = ptr;
            return start as *mut u8;
        }
    }
}

/// Number of characters in a NUL-terminated UTF-8 string, stopping at the
/// first bad sequence.
///
/// # Safety
/// `start` must be NUL-terminated.
pub unsafe fn utf8_string_count(start: *const u8) -> usize {
    let mut p = start;
    let mut count = 0;
    while get_char_from_string(&mut p) != 0 {
        count += 1;
    }
    count
}

/// # Safety
/// `start..end` must be readable.
pub unsafe fn utf8_string_count_n(start: *const u8, end: *const u8) -> usize {
    let mut p = start;
    let mut count = 0;
    while get_char_from_string_n(&mut p, end) != 0 {
        count += 1;
    }
    count
}

/// Character position of `ch` in the string, or -1.
///
/// # Safety
/// `s` must be NUL-terminated.
pub unsafe fn utf8_string_pos(s: *const u8, ch: UniChar) -> c_int {
    let mut p = s;
    let mut pos: c_int = 0;
    while *p != 0 {
        if get_char_from_string(&mut p) == ch {
            return pos;
        }
        pos += 1;
    }
    if ch == 0 {
        pos
    } else {
        -1
    }
}

/// `strncpy` that guarantees NUL termination. Returns NULL when `size` is 0.
/// May cut a multi-byte character in half.
///
/// # Safety
/// `dst` must hold `size` bytes and `src` be NUL-terminated.
pub unsafe fn utf8_string_copy(dst: *mut u8, size: usize, src: *const u8) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    libc::strncpy(dst.cast::<c_char>(), src.cast::<c_char>(), size);
    *dst.add(size - 1) = 0;
    dst
}

/// Byte-wise comparison; collating order is not implemented.
///
/// # Safety
/// Both strings must be NUL-terminated.
pub unsafe fn utf8_string_compare(a: *const u8, b: *const u8) -> c_int {
    libc::strcmp(a.cast::<c_char>(), b.cast::<c_char>())
}

/// Skips up to `num` characters, stopping before a bad sequence.
///
/// # Safety
/// `ptr` must be NUL-terminated.
pub unsafe fn skip_utf8_chars(ptr: *const u8, num: usize) -> *mut u8 {
    let mut p = ptr;
    for _ in 0..num {
        let old = p;
        if get_char_from_string(&mut p) == 0 {
            return old as *mut u8;
        }
    }
    p as *mut u8
}

/// As [`getUniCharFromStringN`] for a NUL-terminated source.
///
/// # Safety
/// `wstr` must hold `maxcount` entries and `start` be NUL-terminated.
pub unsafe fn get_uni_char_from_string(
    wstr: *mut UniChar,
    maxcount: usize,
    start: *const u8,
) -> usize {
    decode_into(wstr, maxcount, start, |p| get_char_from_string(p))
}

unsafe fn decode_into(
    wstr: *mut UniChar,
    maxcount: usize,
    mut src: *const u8,
    mut next_char: impl FnMut(&mut *const u8) -> UniChar,
) -> usize {
    if maxcount == 0 {
        return 0;
    }
    let mut stored = 0;
    while stored < maxcount - 1 {
        let ch = next_char(&mut src);
        if ch == 0 {
            break;
        }
        *wstr.add(stored) = ch;
        stored += 1;
    }
    *wstr.add(stored) = 0;
    stored
}

struct Range {
    limit: UniChar,
    marker: u8,
    mask: u8,
}

const RANGES: [Range; 6] = [
    Range {
        limit: 0x0000_007f,
        marker: 0x00,
        mask: 0x7f,
    },
    Range {
        limit: 0x0000_07ff,
        marker: 0xc0,
        mask: 0x1f,
    },
    Range {
        limit: 0x0000_ffff,
        marker: 0xe0,
        mask: 0x0f,
    },
    Range {
        limit: 0x001f_ffff,
        marker: 0xf0,
        mask: 0x07,
    },
    Range {
        limit: 0x03ff_ffff,
        marker: 0xf8,
        mask: 0x03,
    },
    Range {
        limit: 0x7fff_ffff,
        marker: 0xfc,
        mask: 0x01,
    },
];

/// Encodes one character. Returns the bytes written, 0 for an unsupported
/// character, or minus the bytes needed when `size` is too small. No NUL is
/// written.
///
/// # Safety
/// `ptr` must hold `size` bytes.
pub unsafe fn get_string_from_char(ptr: *mut u8, size: usize, ch: UniChar) -> c_int {
    let Some(index) = RANGES.iter().position(|r| ch <= r.limit) else {
        log_message(
            LOG_WARNING,
            &format!("Warning: Invalid or unsupported unicode char ({ch})"),
        );
        return 0;
    };
    if index + 1 > size {
        return -(index as c_int + 1);
    }
    let mut rest = ch;
    for slot in (1..=index).rev() {
        *ptr.add(slot) = (rest & 0x3f) as u8 | 0x80;
        rest >>= 6;
    }
    let range = &RANGES[index];
    *ptr = (rest as u8 & range.mask) | range.marker;
    index as c_int + 1
}

/// Encodes `count` wide characters, replacing unsupported ones with `?` and
/// truncating what does not fit. Always writes a NUL unless `size` is 0.
/// Returns the bytes used including the NUL.
///
/// # Safety
/// `ptr` must hold `size` bytes and `wstr` `count` entries.
pub unsafe fn get_string_from_wide_n(
    ptr: *mut u8,
    size: usize,
    wstr: *const UniChar,
    count: usize,
) -> usize {
    if size == 0 {
        return 0;
    }
    let mut room = size - 1;
    let mut next = ptr;
    for i in 0..count {
        if room == 0 {
            break;
        }
        let used = match get_string_from_char(next, room, *wstr.add(i)) {
            n if n < 0 => break,
            0 => {
                *next = b'?';
                1
            }
            n => n as usize,
        };
        room -= used;
        next = next.add(used);
    }
    *next = 0;
    next.offset_from(ptr) as usize + 1
}

/// As [`get_string_from_wide_n`] for a 0-terminated source.
///
/// # Safety
/// `ptr` must hold `size` bytes and `wstr` be 0-terminated.
pub unsafe fn get_string_from_wide(ptr: *mut u8, size: usize, wstr: *const UniChar) -> usize {
    let mut len = 0;
    while *wstr.add(len) != 0 {
        len += 1;
    }
    get_string_from_wide_n(ptr, size, wstr, len)
}

/// Every non-control character except the private use area (0xE000-0xF8FF).
pub fn uni_char_is_graph(ch: UniChar) -> c_int {
    c_int::from((ch > 0xa0 && !(0xe000..=0xf8ff).contains(&ch)) || (ch > 0x20 && ch < 0x7f))
}

pub fn uni_char_is_print(ch: UniChar) -> c_int {
    c_int::from(ch == 0x20 || uni_char_is_graph(ch) != 0)
}

/// Latin-1 only, through the C library's current locale.
pub fn uni_char_to_upper(ch: UniChar) -> UniChar {
    if ch < 0x100 {
        // SAFETY: toupper is defined for values 0..=255.
        unsafe { libc::toupper(ch as c_int) as UniChar }
    } else {
        ch
    }
}

pub fn uni_char_to_lower(ch: UniChar) -> UniChar {
    if ch < 0x100 {
        // SAFETY: tolower is defined for values 0..=255.
        unsafe { libc::tolower(ch as c_int) as UniChar }
    } else {
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_helpers_have_rust_signatures() {
        let _: fn(UniChar) -> c_int = uni_char_is_graph;
        let _: fn(UniChar) -> c_int = uni_char_is_print;
        let _: fn(UniChar) -> UniChar = uni_char_to_upper;
        let _: fn(UniChar) -> UniChar = uni_char_to_lower;
        let _: unsafe fn(*mut *const u8) -> UniChar = get_char_from_string;
    }

    fn decode_all(bytes: &[u8]) -> Vec<UniChar> {
        let mut p = bytes.as_ptr();
        let mut out = Vec::new();
        unsafe {
            while *p != 0 {
                out.push(get_char_from_string(&mut p));
            }
        }
        out
    }

    #[test]
    fn decodes_each_sequence_length() {
        assert_eq!(
            decode_all(b"a\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80\0"),
            [0x61, 0xe9, 0x20ac, 0x1f600]
        );
    }

    #[test]
    fn overlong_and_truncated_sequences_decode_to_zero_and_resync() {
        // Overlong 'A', then a lead byte followed by ASCII, then stray
        // continuation bytes.
        let bytes = b"\xc1\x81Z\xe2\x82Y\x80\x80X\0";
        let mut p = bytes.as_ptr();
        unsafe {
            assert_eq!(get_char_from_string(&mut p), 0);
            assert_eq!(*p, b'Z');
            p = p.add(1);
            assert_eq!(get_char_from_string(&mut p), 0);
            assert_eq!(*p, b'Y');
            p = p.add(1);
            assert_eq!(get_char_from_string(&mut p), 0);
            assert_eq!(*p, b'X');
        }
    }

    #[test]
    fn bounded_decode_moves_to_end_on_truncation() {
        let bytes = b"\xe2\x82\xac";
        unsafe {
            let mut p = bytes.as_ptr();
            let end = p.add(2);
            assert_eq!(get_char_from_string_n(&mut p, end), 0);
            assert_eq!(p, end);
            let mut p = bytes.as_ptr();
            assert_eq!(get_char_from_string_n(&mut p, p.add(3)), 0x20ac);
            assert_eq!(p, bytes.as_ptr().add(3));
            let s = b"ab\xc3\xa9";
            assert_eq!(utf8_string_count_n(s.as_ptr(), s.as_ptr().add(4)), 3);
            assert_eq!(utf8_string_count_n(s.as_ptr(), s.as_ptr().add(3)), 2);
        }
    }

    #[test]
    fn counts_and_finds_characters() {
        let s = b"h\xc3\xa9llo\0";
        unsafe {
            assert_eq!(utf8_string_count(s.as_ptr()), 5);
            assert_eq!(utf8_string_pos(s.as_ptr(), 0xe9), 1);
            assert_eq!(utf8_string_pos(s.as_ptr(), u32::from(b'o')), 4);
            assert_eq!(utf8_string_pos(s.as_ptr(), u32::from(b'z')), -1);
            assert_eq!(utf8_string_pos(s.as_ptr(), 0), 5);
            assert_eq!(skip_utf8_chars(s.as_ptr(), 2).offset_from(s.as_ptr()), 3);
            assert_eq!(skip_utf8_chars(s.as_ptr(), 99).offset_from(s.as_ptr()), 6);
        }
    }

    unsafe fn line(src: &[u8]) -> Option<(usize, usize)> {
        let start = src.as_ptr();
        let (mut end, mut next) = (start, start);
        let ret = get_line_from_string(start, &mut end, &mut next);
        (!ret.is_null()).then(|| {
            (
                end.offset_from(start) as usize,
                next.offset_from(start) as usize,
            )
        })
    }

    #[test]
    fn splits_lines_on_cr_lf_crlf_lfcr() {
        unsafe {
            assert_eq!(line(b"ab\0"), Some((2, 2)));
            assert_eq!(line(b"ab\ncd\0"), Some((2, 2)));
            assert_eq!(line(b"ab\n\rcd\0"), Some((2, 4)));
            assert_eq!(line(b"ab\rcd\0"), Some((2, 3)));
            assert_eq!(line(b"ab\n\0"), Some((2, 3)));
            assert_eq!(line(b"ab\xffcd\0"), None);
        }
    }

    #[test]
    fn copy_terminates_and_pads_like_strncpy() {
        let mut dst = [0x55u8; 8];
        unsafe {
            assert!(utf8_string_copy(dst.as_mut_ptr(), 0, c"x".as_ptr().cast()).is_null());
            utf8_string_copy(dst.as_mut_ptr(), 3, c"abcdef".as_ptr().cast());
            assert_eq!(&dst[..4], b"ab\0\x55");
            utf8_string_copy(dst.as_mut_ptr(), 6, c"a".as_ptr().cast());
            assert_eq!(&dst[..6], b"a\0\0\0\0\0");
        }
    }

    #[test]
    fn encodes_characters_and_reports_short_buffers() {
        let mut buf = [0u8; 8];
        unsafe {
            assert_eq!(get_string_from_char(buf.as_mut_ptr(), 8, 0x20ac), 3);
            assert_eq!(&buf[..3], b"\xe2\x82\xac");
            assert_eq!(get_string_from_char(buf.as_mut_ptr(), 2, 0x20ac), -3);
            assert_eq!(get_string_from_char(buf.as_mut_ptr(), 8, 0x8000_0000), 0);
            assert_eq!(get_string_from_char(buf.as_mut_ptr(), 8, 0x7fff_ffff), 6);
            assert_eq!(&buf[..6], b"\xfd\xbf\xbf\xbf\xbf\xbf");
        }
    }

    #[test]
    fn wide_round_trip_and_truncation() {
        let wide = [0x68, 0xe9, 0x20ac, 0x8000_0000, 0];
        let mut buf = [0xaau8; 16];
        unsafe {
            assert_eq!(get_string_from_wide(buf.as_mut_ptr(), 16, wide.as_ptr()), 8);
            assert_eq!(&buf[..8], b"h\xc3\xa9\xe2\x82\xac?\0");
            // Room for 4 bytes + NUL: the euro sign no longer fits.
            assert_eq!(get_string_from_wide(buf.as_mut_ptr(), 5, wide.as_ptr()), 4);
            assert_eq!(&buf[..4], b"h\xc3\xa9\0");
            assert_eq!(get_string_from_wide(buf.as_mut_ptr(), 0, wide.as_ptr()), 0);

            let mut back = [0xffff_ffffu32; 8];
            let src = b"h\xc3\xa9\xe2\x82\xac\0";
            assert_eq!(
                get_uni_char_from_string(back.as_mut_ptr(), 8, src.as_ptr()),
                3
            );
            assert_eq!(&back[..4], &[0x68, 0xe9, 0x20ac, 0]);
            assert_eq!(
                get_uni_char_from_string(back.as_mut_ptr(), 3, c"abcd".as_ptr().cast()),
                2
            );
            assert_eq!(&back[..3], &[0x61, 0x62, 0]);
            assert_eq!(
                get_uni_char_from_string(back.as_mut_ptr(), 0, c"abcd".as_ptr().cast()),
                0
            );
        }
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
        assert_eq!(uni_char_to_lower(u32::from(b'Q')), u32::from(b'q'));
        assert_eq!(uni_char_to_upper(0x20ac), 0x20ac);
    }
}
