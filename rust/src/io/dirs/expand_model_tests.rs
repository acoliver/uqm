//! Randomised checks of `expand_path` (fixture spec section E): an
//! independent in-place transcription of the C `EP_DOTS` loop, structural
//! properties of the output, and the `$NAME` / `${NAME}` expansion against
//! direct concatenation.

use std::collections::HashMap;

use proptest::collection::vec;
use proptest::prelude::*;

use super::expand::*;
use super::test_support::{bytes_or_errno, FakeEnv};

/// The `EP_DOTS` block of `expandPath`, transcribed with one buffer, byte
/// indices and a trailing NUL exactly like the C pointers (`memmove` becomes
/// `copy_within`, `strrchr2` is spelled out). `Err` is the `EINVAL` exit.
fn c_dots_model(path: &[u8]) -> Result<Vec<u8>, ()> {
    let mut buf = path.to_vec();
    buf.push(0);
    let path_start = usize::from(buf[0] == b'/');
    let mut start_part = path_start;
    let mut destptr = path_start;
    loop {
        let mut end_part = start_part;
        while buf[end_part] != b'/' && buf[end_part] != 0 {
            end_part += 1;
        }
        let width = end_part - start_part;
        if width == 1 && buf[start_part] == b'.' {
            // "." is ignored
        } else if width == 2 && buf[start_part] == b'.' && buf[start_part + 1] == b'.' {
            let mut end = destptr as isize - 1;
            let last_slash = loop {
                end -= 1;
                if end < path_start as isize {
                    break None;
                }
                if buf[end as usize] == b'/' {
                    break Some(end as usize);
                }
            };
            match last_slash {
                None if destptr == path_start => return Err(()),
                None => destptr = path_start,
                Some(slash) => {
                    destptr = slash;
                    if buf[end_part] == b'/' {
                        destptr += 1;
                    }
                }
            }
        } else {
            buf.copy_within(start_part..end_part, destptr);
            destptr += width;
            if buf[end_part] == b'/' {
                buf[destptr] = b'/';
                destptr += 1;
            }
        }
        if buf[end_part] == 0 {
            break;
        }
        start_part = end_part + 1;
    }
    buf.truncate(destptr);
    Ok(buf)
}

fn component() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("a"),
        Just("bb"),
        Just("."),
        Just(".."),
        Just(""),
        Just("...")
    ]
}

/// Paths made of the components above, optionally rooted.
fn dotted_path() -> impl Strategy<Value = Vec<u8>> {
    (any::<bool>(), vec(component(), 0..8)).prop_map(|(rooted, parts)| {
        let joined = parts.join("/");
        if rooted {
            format!("/{joined}").into_bytes()
        } else {
            joined.into_bytes()
        }
    })
}

fn path_byte() -> impl Strategy<Value = u8> {
    prop_oneof![
        Just(b'/'),
        Just(b'.'),
        Just(b'$'),
        Just(b'~'),
        Just(b'{'),
        Just(b'}'),
        Just(b'\\'),
        Just(b'V'),
        1u8..=255,
    ]
}

fn expand(src: &[u8], len: usize, what: i32) -> Result<Vec<u8>, i32> {
    bytes_or_errno(expand_path(&FakeEnv::standard(), src, len, what))
}

proptest! {
    #[test]
    fn dots_match_the_transcribed_c_loop(path in dotted_path()) {
        let expected = c_dots_model(&path).map_err(|()| libc::EINVAL);
        prop_assert_eq!(expand(&path, 4096, EP_DOTS), expected);
    }

    #[test]
    fn dots_match_the_transcribed_c_loop_on_arbitrary_bytes(
        path in vec(prop_oneof![Just(b'/'), Just(b'.'), 1u8..=255], 0..40),
    ) {
        let expected = c_dots_model(&path).map_err(|()| libc::EINVAL);
        prop_assert_eq!(expand(&path, 4096, EP_DOTS), expected);
    }

    #[test]
    fn expansion_never_panics_and_results_fit_the_buffer(
        src in vec(path_byte(), 0..40),
        what in 0..256i32,
        len in 0usize..80,
    ) {
        if let Ok(result) = expand(&src, len, what) {
            prop_assert!(result.len() < len);
        }
    }

    #[test]
    fn dots_and_single_separators_leave_no_trace_in_the_output(path in dotted_path()) {
        if let Ok(result) = expand(&path, 4096, EP_DOTS | EP_SINGLESEP) {
            prop_assert!(result.split(|byte| *byte == b'/').all(|part| part != b"." && part != b".."));
            prop_assert!(!result.windows(2).any(|pair| pair == b"//"));
        }
    }

    #[test]
    fn variables_expand_to_their_values_and_nothing_is_rescanned(
        tokens in vec(token(), 0..8),
        values in vec("[a-z/]{0,5}", 4),
        set in vec(any::<bool>(), 4),
    ) {
        let mut env = FakeEnv::standard();
        env.vars = HashMap::new();
        let mut expected = Vec::new();
        let mut src = Vec::new();
        for token in &tokens {
            match token {
                Token::Literal(text) => {
                    src.extend_from_slice(text.as_bytes());
                    expected.extend_from_slice(text.as_bytes());
                }
                Token::Plain(index) | Token::Braced(index) => {
                    let name = format!("N{index}");
                    src.extend_from_slice(match token {
                        Token::Plain(_) => format!("${name}").into_bytes(),
                        _ => format!("${{{name}}}").into_bytes(),
                    }.as_slice());
                    if set[*index] {
                        expected.extend_from_slice(values[*index].as_bytes());
                    }
                }
            }
        }
        for index in 0..4 {
            if set[index] {
                env.vars.insert(format!("N{index}").into_bytes(), values[index].clone().into_bytes());
            }
        }
        let got = bytes_or_errno(expand_path(&env, &src, 4096, EP_ENVVARS));
        prop_assert_eq!(got, Ok(expected));
    }
}

#[derive(Debug, Clone)]
enum Token {
    Literal(String),
    Plain(usize),
    Braced(usize),
}

/// Literals hold only separators and dots, so a literal can never extend the
/// name of an unbraced variable in front of it.
fn token() -> impl Strategy<Value = Token> {
    prop_oneof![
        "[/.-]{0,4}".prop_map(Token::Literal),
        (0..4usize).prop_map(Token::Plain),
        (0..4usize).prop_map(Token::Braced),
    ]
}
