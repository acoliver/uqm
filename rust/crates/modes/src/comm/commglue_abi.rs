//! Script glue, ported from commglue.c (GPL-2.0-or-later).

use crate::comm::Comm;
use uqm_engine::StoryKey;

use crate::comm::races::descriptor::NumberSpeech;

/// Collect one-based phrase indices in the same recursive order as NPCNumberPhrase.
fn number_phrases(
    speech: &NumberSpeech,
    mut number: i32,
    top: bool,
    out: &mut Vec<u16>,
) -> Result<(), &'static str> {
    let start = out.len();
    for digit in speech.digits {
        let quotient = number / digit.divider;
        if quotient == 0 {
            continue;
        }
        let quotient = quotient - digit.subtrahend;
        if quotient < 0 {
            continue;
        }
        if let Some(strings) = digit.strings {
            let index = strings[quotient as usize];
            if index == 0 {
                continue;
            }
            out.push(index);
        } else {
            number_phrases(speech, quotient, false, out)?;
        }
        if !digit.names.is_empty() {
            if let Some(name) = digit
                .names
                .iter()
                .find(|name| number % name.divider <= name.max_remainder)
            {
                out.push(name.phrase);
            }
        } else if digit.common != 0 {
            out.push(digit.common);
        }
        number %= digit.divider;
    }
    if top && out.len() == start {
        assert_eq!(number, 0);
        out.push(
            speech
                .digits
                .last()
                .and_then(|digit| digit.strings)
                .ok_or("number speech has no zero digit")?[0],
        );
    }
    Ok(())
}

pub fn set_segue(comm: &mut Comm, cx: &mut uqm_engine::battle::Cx<'_>, segue: u32) {
    comm.set_segue(crate::comm::segue::Segue::from(segue));
    let world = &mut cx.world;
    match segue {
        0 => world.set_story_state(StoryKey::BATTLE_SEGUE, 0),
        1 => world.set_story_state(StoryKey::BATTLE_SEGUE, 1),
        2 => {
            cx.kernel.instant_victory = 1;
            world.set_story_state(StoryKey::BATTLE_SEGUE, 1);
        }
        3 => {
            world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
            world.data.flagship.crew_enlisted = u16::MAX;
            world.set_activity(world.activity() | 0x2000);
        }
        _ => unreachable!("invalid Segue"),
    }
}
pub fn get_segue(cx: &uqm_engine::battle::Cx<'_>) -> u32 {
    let world = &cx.world;
    if world.story_state(StoryKey::BATTLE_SEGUE) == 0 {
        u32::from(world.data.flagship.crew_enlisted == u16::MAX && world.activity() & 0x2000 != 0)
            * 3
    } else if cx.kernel.instant_victory != 0 {
        2
    } else {
        1
    }
}

pub(crate) mod runtime {
    use super::*;
    use std::ffi::CString;

    pub(super) fn number_text(number: i32, padded: bool) -> Result<CString, std::ffi::NulError> {
        let text = if padded {
            format!("{number:03}")
        } else {
            number.to_string()
        };
        CString::new(text)
    }

    pub fn npc_number(comm: &mut Comm, number: i32, padded: bool) {
        let speech = comm.data.definition.alien_number_speech;
        let (text, indices) = (|| {
            let text = number_text(number, padded).map_err(|_| "decimal integer contains NUL")?;
            let mut indices = Vec::new();
            if let Some(speech) = speech {
                number_phrases(speech, number, true, &mut indices)?;
            }
            Ok::<_, &str>((text, indices))
        })()
        .unwrap_or_else(|error| panic!("invalid number speech: {error}"));
        if speech.is_some() {
            assert!(indices.len() < uqm_engine::sound::trackplayer::MAX_MULTI_TRACKS);
            let tracks: Vec<_> = indices
                .into_iter()
                .map_while(|index| {
                    comm.display
                        .phrases
                        .as_ref()?
                        .phrase(index.wrapping_sub(1))?
                        .clip
                })
                .collect();
            uqm_engine::sound::heart_ffi::splice_multi_track(
                comm.phrase_files.as_ref(),
                &tracks,
                Some(&text),
            );
        } else {
            uqm_engine::sound::heart_ffi::splice_track(
                comm.phrase_files.as_ref(),
                None,
                Some(&text),
                None,
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::races::descriptor::{DigitName, SpeechDigit};

    #[test]
    fn number_text_preserves_decimal_and_three_column_printf_bytes() {
        for (number, plain, padded) in [
            (0, c"0", c"000"),
            (7, c"7", c"007"),
            (-7, c"-7", c"-07"),
            (1234, c"1234", c"1234"),
            (i32::MIN, c"-2147483648", c"-2147483648"),
            (i32::MAX, c"2147483647", c"2147483647"),
        ] {
            assert_eq!(
                runtime::number_text(number, false).unwrap().as_c_str(),
                plain
            );
            assert_eq!(
                runtime::number_text(number, true).unwrap().as_c_str(),
                padded
            );
        }
        let _: fn(&mut Comm, i32, bool) = runtime::npc_number;
    }

    #[test]
    fn immutable_race_number_tables_preserve_zero_teens_and_recursive_thousands() {
        use crate::comm::races::{melnorm, slyland};
        fn assert_shared(_: &'static NumberSpeech) {}
        for (speech, [zero, one, ten, nineteen, twenty, ninety, hundred, thousand, nine]) in [
            (
                melnorm::numbers::descriptor(),
                [269, 252, 261, 272, 273, 280, 281, 282, 260],
            ),
            (
                slyland::numbers::descriptor(),
                [59, 60, 69, 78, 79, 86, 57, 58, 68],
            ),
        ] {
            assert_shared(speech);
            for (number, expected) in [
                (0, vec![zero]),
                (1, vec![one]),
                (10, vec![ten]),
                (19, vec![nineteen]),
                (20, vec![twenty]),
                (99, vec![ninety, nine]),
                (100, vec![one, hundred]),
                (999, vec![nine, hundred, ninety, nine]),
                (1000, vec![one, thousand]),
                (1_001_010, vec![one, thousand, one, thousand, ten]),
            ] {
                let mut out = Vec::new();
                number_phrases(speech, number, true, &mut out).unwrap();
                assert_eq!(out, expected, "{number}");
            }
        }
    }

    #[test]
    fn recursive_number_speech_and_zero_preserve_phrase_order() {
        static UNITS: [u16; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        static TENS: [u16; 10] = [0, 0, 20, 30, 40, 50, 60, 70, 80, 90];
        static TEENS: [u16; 10] = [11, 12, 13, 14, 15, 16, 17, 18, 19, 100];
        const DIGITS: [SpeechDigit; 5] = [
            SpeechDigit::new(1000, 0, None, 200),
            SpeechDigit::new(100, 0, Some(&UNITS), 300),
            SpeechDigit::new(10, 0, Some(&TENS), 0),
            SpeechDigit::new(1, 10, Some(&TEENS), 0),
            SpeechDigit::new(1, 0, Some(&UNITS), 0),
        ];
        let speech = NumberSpeech { digits: &DIGITS };
        for (number, expected) in [
            (0, vec![1]),
            (17, vec![18]),
            (1204, vec![2, 200, 3, 300, 5]),
            (2034, vec![3, 200, 30, 5]),
        ] {
            let mut out = Vec::new();
            number_phrases(&speech, number, true, &mut out).unwrap();
            assert_eq!(out, expected);
        }
        const NAMED: [SpeechDigit; 5] = {
            let mut digits = DIGITS;
            digits[1].names = &[
                DigitName {
                    divider: 100,
                    max_remainder: 1,
                    phrase: 400,
                },
                DigitName {
                    divider: 100,
                    max_remainder: 99,
                    phrase: 500,
                },
            ];
            digits
        };
        let speech = NumberSpeech { digits: &NAMED };
        for (number, expected) in [(101, vec![2, 400, 2]), (102, vec![2, 500, 3])] {
            let mut out = Vec::new();
            number_phrases(&speech, number, true, &mut out).unwrap();
            assert_eq!(out, expected);
        }
    }
}
