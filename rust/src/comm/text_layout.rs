//! Word wrapping used by dialogue subtitles and response lists.

use super::sis_graphics::CText;
use std::ffi::c_char;

pub(crate) struct FittedLine {
    pub end_of_line: bool,
    pub characters: u16,
    pub next: usize,
}

pub(crate) fn fit_text_line(
    text: &std::ffi::CStr,
    initial_count: u16,
    max_width: i16,
    max_chars: u16,
    mut measure: impl FnMut(u16) -> (i16, u16),
) -> FittedLine {
    let text = text
        .to_str()
        .unwrap_or_else(|error| panic!("text layout UTF-8: {error}"));
    let mut count = 0u16;
    let mut position = 0;
    let mut accepted = initial_count;
    loop {
        let word_start = position;
        let (eol, done) = loop {
            let Some(ch) = text[position..].chars().next() else {
                break (true, true);
            };
            position += ch.len_utf8();
            let eol = ch == '\n' || ch == '\r';
            let done = eol || count >= max_chars;
            if done || ch == ' ' {
                break (eol, done);
            }
            count = count.wrapping_add(1);
        };
        let (width, measured_count) = measure(count);
        if width >= max_width {
            return FittedLine {
                end_of_line: false,
                characters: accepted,
                next: word_start,
            };
        }
        accepted = measured_count;
        if done {
            return FittedLine {
                end_of_line: eol,
                characters: accepted,
                next: position,
            };
        }
        count = count.wrapping_add(1);
    }
}

/// # Safety
/// Text must reference a NUL-terminated UTF-8 string; measure may inspect it.
pub(super) unsafe fn fit_line(
    text: &mut CText,
    next: &mut *const c_char,
    max_width: i16,
    max_chars: u16,
    mut measure: impl FnMut(&mut CText) -> i16,
) -> bool {
    let input = std::ffi::CStr::from_ptr(text.p_str);
    let fitted = fit_text_line(input, text.char_count, max_width, max_chars, |count| {
        text.char_count = count;
        (measure(text), text.char_count)
    });
    text.char_count = fitted.characters;
    *next = input.as_ptr().add(fitted.next);
    fitted.end_of_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    fn wrap(input: &CStr, width: i16, limit: u16) -> (bool, u16, usize) {
        let mut text = CText::new(input.as_ptr(), u16::MAX);
        let mut next = std::ptr::null();
        let eol = unsafe { fit_line(&mut text, &mut next, width, limit, |t| t.char_count as i16) };
        let result = (eol, text.char_count, unsafe {
            next.offset_from(input.as_ptr()) as usize
        });
        let owned = fit_text_line(input, u16::MAX, width, limit, |count| (count as i16, count));
        assert_eq!(result, (owned.end_of_line, owned.characters, owned.next));
        result
    }
    #[test]
    fn wrapping_keeps_the_measurements_adjusted_character_count() {
        let line = fit_text_line(c"one two", u16::MAX, 7, u16::MAX, |count| {
            (count as i16, count.saturating_sub(1))
        });
        assert!(!line.end_of_line);
        assert_eq!((line.characters, line.next), (2, 4));
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
