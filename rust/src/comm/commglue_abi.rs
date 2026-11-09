//! Script glue, ported from commglue.c (GPL-2.0-or-later).

use crate::comm::Comm;

use crate::c_bindings::global_state::NUMBER_SPEECH_DESC;

/// Collect one-based phrase indices in the same recursive order as NPCNumberPhrase.
unsafe fn number_phrases(
    speech: &NUMBER_SPEECH_DESC,
    mut number: i32,
    top: bool,
    out: &mut Vec<u16>,
) {
    let start = out.len();
    let mut last = None;
    for digit in &speech.Digits[..usize::from(speech.NumDigits)] {
        last = Some(digit);
        let quotient = number / digit.Divider;
        if quotient == 0 {
            continue;
        }
        let quotient = quotient - digit.Subtrahend;
        if quotient < 0 {
            continue;
        }
        if !digit.StrDigits.is_null() {
            assert!(quotient < 10);
            let index = *digit.StrDigits.add(quotient as usize);
            if index == 0 {
                continue;
            }
            out.push(index);
        } else {
            number_phrases(speech, quotient, false, out);
        }
        if !digit.Names.is_null() {
            let mut name = digit.Names;
            while (*name).Divider != 0 {
                if number % (*name).Divider <= (*name).MaxRemainder {
                    out.push((*name).StrIndex);
                    break;
                }
                name = name.add(1);
            }
        } else if digit.CommonNameIndex != 0 {
            out.push(digit.CommonNameIndex);
        }
        number %= digit.Divider;
    }
    if top && out.len() == start {
        assert_eq!(number, 0);
        out.push(*last.expect("number speech has no digits").StrDigits);
    }
}

pub(crate) mod runtime {
    use super::*;
    use crate::engine::StoryKey;
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
        if comm.data.alien_number_speech.is_null() {
            let mut buffer = [0 as c_char; 32];
            libc::snprintf(buffer.as_mut_ptr(), buffer.len(), format, number);
            SpliceTrack(null_mut(), buffer.as_mut_ptr(), null_mut(), None);
        } else {
            let speech = &*comm.data.alien_number_speech.cast::<NUMBER_SPEECH_DESC>();
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
            u32::from(
                world.data.flagship.crew_enlisted == u16::MAX && world.activity() & 0x2000 != 0,
            ) * 3
        } else if cx.kernel.instant_victory != 0 {
            2
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_bindings::global_state::{SPEECH_DIGIT, SPEECH_DIGITNAME};

    #[test]
    fn recursive_number_speech_and_zero_preserve_phrase_order() {
        let mut units = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut tens = [0, 0, 20, 30, 40, 50, 60, 70, 80, 90];
        let mut teens = [11, 12, 13, 14, 15, 16, 17, 18, 19, 100];
        let mut speech: NUMBER_SPEECH_DESC = unsafe { std::mem::zeroed() };
        speech.NumDigits = 5;
        let digit = |divider, sub, strings, common| SPEECH_DIGIT {
            Divider: divider,
            Subtrahend: sub,
            StrDigits: strings,
            Names: std::ptr::null_mut(),
            CommonNameIndex: common,
        };
        speech.Digits[0] = digit(1000, 0, std::ptr::null_mut(), 200);
        speech.Digits[1] = digit(100, 0, units.as_mut_ptr(), 300);
        speech.Digits[2] = digit(10, 0, tens.as_mut_ptr(), 0);
        speech.Digits[3] = digit(1, 10, teens.as_mut_ptr(), 0);
        speech.Digits[4] = digit(1, 0, units.as_mut_ptr(), 0);
        for (number, expected) in [
            (0, vec![1]),
            (17, vec![18]),
            (1204, vec![2, 200, 3, 300, 5]),
            (2034, vec![3, 200, 30, 5]),
        ] {
            let mut out = Vec::new();
            unsafe { number_phrases(&speech, number, true, &mut out) };
            assert_eq!(out, expected);
        }
        let mut names = [
            SPEECH_DIGITNAME {
                Divider: 100,
                MaxRemainder: 1,
                StrIndex: 400,
            },
            SPEECH_DIGITNAME {
                Divider: 100,
                MaxRemainder: 99,
                StrIndex: 500,
            },
            SPEECH_DIGITNAME {
                Divider: 0,
                MaxRemainder: 0,
                StrIndex: 0,
            },
        ];
        speech.Digits[1].Names = names.as_mut_ptr();
        for (number, expected) in [(101, vec![2, 400, 2]), (102, vec![2, 500, 3])] {
            let mut out = Vec::new();
            unsafe { number_phrases(&speech, number, true, &mut out) };
            assert_eq!(out, expected);
        }
    }
}
