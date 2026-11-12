//! Script glue, ported from commglue.c (GPL-2.0-or-later).

use crate::comm::Comm;
use crate::engine::StoryKey;

use crate::comm::races::descriptor::NumberSpeech;

/// Collect one-based phrase indices in the same recursive order as NPCNumberPhrase.
fn number_phrases(speech: &NumberSpeech, mut number: i32, top: bool, out: &mut Vec<u16>) {
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
            number_phrases(speech, quotient, false, out);
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
                .expect("number speech has no zero digit")[0],
        );
    }
}

pub fn set_segue(comm: &mut Comm, cx: &mut crate::engine::battle::Cx<'_>, segue: u32) {
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
pub fn get_segue(cx: &crate::engine::battle::Cx<'_>) -> u32 {
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

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::*;
    use std::ffi::c_char;
    use std::ptr::null_mut;

    use crate::sound::heart_ffi::splice_multi_track as SpliceMultiTrack;
    use crate::sound::heart_ffi::splice_track as SpliceTrack;

    use crate::strings::string_table::GetStringSoundClip;

    use crate::strings::string_table::SetAbsStringTableIndex;

    /// # Safety
    /// Called from the game thread with a valid printf integer format or null.
    pub unsafe fn npc_number(comm: &mut Comm, number: i32, format: *const c_char) {
        let format = if format.is_null() {
            c"%d".as_ptr()
        } else {
            format
        };
        if let Some(speech) = comm.data.alien_number_speech {
            let mut indices = Vec::new();
            number_phrases(speech, number, true, &mut indices);
            let mut tracks = [std::ptr::null::<c_char>(); 20];
            assert!(indices.len() < tracks.len());
            for (track, index) in tracks.iter_mut().zip(indices) {
                *track = GetStringSoundClip(SetAbsStringTableIndex(
                    (comm.data.conversation_phrases).cast(),
                    index.wrapping_sub(1),
                ))
                .cast();
            }
            let mut buffer = [0 as c_char; 60];
            libc::snprintf(buffer.as_mut_ptr(), buffer.len(), format, number);
            SpliceMultiTrack(tracks.as_ptr(), buffer.as_ptr());
        } else {
            let mut buffer = [0 as c_char; 32];
            libc::snprintf(buffer.as_mut_ptr(), buffer.len(), format, number);
            SpliceTrack(null_mut(), buffer.as_mut_ptr(), null_mut(), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::races::descriptor::{DigitName, SpeechDigit};

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
                number_phrases(speech, number, true, &mut out);
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
            number_phrases(&speech, number, true, &mut out);
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
            number_phrases(&speech, number, true, &mut out);
            assert_eq!(out, expected);
        }
    }
}
