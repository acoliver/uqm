//! Word wrapping used by dialogue subtitles and response lists.

use super::sis_graphics::CText;
use std::ffi::c_char;

/// # Safety
/// Text must reference a NUL-terminated UTF-8 string; measure may inspect it.
pub(super) unsafe fn fit_line(
    text: &mut CText,
    next: &mut *const c_char,
    max_width: i16,
    max_chars: u16,
    mut measure: impl FnMut(&mut CText) -> i16,
) -> bool {
    let mut count = 0u16;
    let mut ptr = text.p_str.cast::<u8>();
    loop {
        let word_start = ptr;
        let (eol, done) = loop {
            if *ptr == 0 {
                break (true, true);
            }
            let ch = crate::strings::unicode::get_char_from_string(&mut ptr);
            let eol = ch == 0 || ch == u32::from(b'\n') || ch == u32::from(b'\r');
            let done = eol || count >= max_chars;
            if done || ch == u32::from(b' ') {
                break (eol, done);
            }
            count = count.wrapping_add(1);
        };
        let old_count = text.char_count;
        text.char_count = count;
        if measure(text) >= max_width {
            text.char_count = old_count;
            *next = word_start.cast();
            return false;
        }
        if done {
            *next = ptr.cast();
            return eol;
        }
        count = count.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    fn wrap(input: &CStr, width: i16, limit: u16) -> (bool, u16, usize) {
        let mut text = CText::new(input.as_ptr(), u16::MAX);
        let mut next = std::ptr::null();
        let eol = unsafe { fit_line(&mut text, &mut next, width, limit, |t| t.char_count as i16) };
        (eol, text.char_count, unsafe {
            next.offset_from(input.as_ptr()) as usize
        })
    }

    #[test]
    fn wraps_before_word_when_width_is_equal_not_only_greater() {
        assert_eq!(wrap(c"one two", 7, u16::MAX), (false, 3, 4));
        assert_eq!(wrap(c"one two", 8, u16::MAX), (true, 7, 7));
    }

    #[test]
    fn line_endings_consume_only_the_current_delimiter() {
        assert_eq!(wrap(c"one\r\ntwo", 20, u16::MAX), (true, 3, 4));
        assert_eq!(wrap(c"one\ntwo", 20, u16::MAX), (true, 3, 4));
        assert_eq!(wrap(c"", 20, u16::MAX), (true, 0, 0));
    }

    #[test]
    fn limit_counts_characters_and_next_advances_past_the_first_excess_character() {
        assert_eq!(wrap(c"abcd", 20, 2), (false, 2, 3));
        assert_eq!(wrap(c"ééé z", 20, 2), (false, 2, 6));
        assert_eq!(wrap(c"ééé z", 4, u16::MAX), (false, 3, 7));
    }

    #[test]
    fn repeated_spaces_are_included_in_the_character_count() {
        assert_eq!(wrap(c"a  b", 5, u16::MAX), (true, 4, 4));
        assert_eq!(wrap(c"a  b", 4, u16::MAX), (false, 2, 3));
    }
}
