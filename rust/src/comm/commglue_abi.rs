//! Script glue, ported from commglue.c (GPL-2.0-or-later).

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

#[cfg(not(test))]
mod runtime {
    use super::*;
    use crate::comm::locdata::{CGlobData, COMM_DATA};
    use std::ffi::{c_char, c_void};
    use std::ptr::null_mut;

    #[allow(clashing_extern_declarations)]
    extern "C" {
        static mut GlobData: CGlobData;
        static mut instantVictory: i32;
        static mut shared_phrase_buf: [c_char; 2048];
        fn SetAbsStringTableIndex(table: *mut c_void, index: u16) -> *mut c_void;
        fn SetRelStringTableIndex(table: *mut c_void, index: i16) -> *mut c_void;
        fn GetStringAddress(entry: *mut c_void) -> *const c_char;
        fn GetStringSoundClip(entry: *mut c_void) -> *mut c_void;
        fn SpliceTrack(
            clip: *mut c_char,
            text: *mut c_char,
            timestamp: *mut c_char,
            callback: Option<unsafe extern "C" fn()>,
        );
        fn SpliceMultiTrack(tracks: *mut *mut c_char, text: *mut c_char);
    }

    fn get(key: &str) -> u8 {
        let (first, last) =
            crate::state::game_state_keys::bit_range(key).expect("known comm state");
        unsafe { crate::state::ffi::rust_get_game_state_bits(first as i32, last as i32) }
    }
    fn set(key: &str, value: u8) {
        let (first, last) =
            crate::state::game_state_keys::bit_range(key).expect("known comm state");
        unsafe { crate::state::ffi::rust_set_game_state_bits(first as i32, last as i32, value) }
    }

    /// # Safety
    /// Called from the game thread with a valid printf integer format or null.
    #[no_mangle]
    pub unsafe extern "C" fn NPCNumber(number: i32, format: *const c_char) {
        let format = if format.is_null() {
            c"%d".as_ptr()
        } else {
            format
        };
        if COMM_DATA.alien_number_speech.is_null() {
            let mut buffer = [0 as c_char; 32];
            libc::snprintf(buffer.as_mut_ptr(), buffer.len(), format, number);
            SpliceTrack(null_mut(), buffer.as_mut_ptr(), null_mut(), None);
        } else {
            let speech = &*COMM_DATA.alien_number_speech.cast::<NUMBER_SPEECH_DESC>();
            let mut indices = Vec::new();
            number_phrases(speech, number, true, &mut indices);
            let mut tracks = [null_mut(); 20];
            assert!(indices.len() < tracks.len());
            for (track, index) in tracks.iter_mut().zip(indices) {
                *track = GetStringSoundClip(SetAbsStringTableIndex(
                    COMM_DATA.conversation_phrases,
                    index.wrapping_sub(1),
                ))
                .cast();
            }
            let mut buffer = [0 as c_char; 60];
            libc::snprintf(buffer.as_mut_ptr(), buffer.len(), format, number);
            SpliceMultiTrack(tracks.as_mut_ptr(), buffer.as_mut_ptr());
        }
    }

    /// # Safety
    /// Buffer must hold the complete alliance name, including commander and suffix.
    #[no_mangle]
    pub unsafe extern "C" fn GetAllianceName(buffer: *mut c_char, first: u16) {
        let variant = get("NEW_ALLIANCE_NAME");
        let entry = SetAbsStringTableIndex(
            COMM_DATA.conversation_phrases,
            first.wrapping_sub(1).wrapping_add(variant.into()),
        );
        libc::strcpy(buffer, GetStringAddress(entry));
        if variant == 3 {
            libc::strcat(
                buffer,
                (&raw const GlobData.sis_state.commander_name).cast(),
            );
            libc::strcat(buffer, GetStringAddress(SetRelStringTableIndex(entry, 1)));
        }
    }

    /// # Safety
    /// Buffers must be live, nonoverlapping, and large enough for both fragments.
    #[no_mangle]
    pub unsafe extern "C" fn rust_append_response_fragment(
        mut buffer: *mut c_char,
        phrase: i32,
        name: *const c_char,
    ) -> *mut c_char {
        let text = GetStringAddress(SetAbsStringTableIndex(
            COMM_DATA.conversation_phrases,
            (phrase - 1) as u16,
        ));
        libc::strcpy(buffer, text);
        buffer = buffer.add(libc::strlen(buffer));
        if !name.is_null() {
            libc::strcpy(buffer, name);
            buffer = buffer.add(libc::strlen(name));
        }
        buffer
    }

    /// # Safety
    /// End must belong to the same live response buffer as start.
    #[no_mangle]
    pub unsafe extern "C" fn rust_finish_response(start: *mut c_char, end: *mut c_char) {
        *end = 0;
        if start == (&raw mut shared_phrase_buf).cast() && end > start.add(2048) {
            panic!("shared_phrase_buf size exceeded");
        }
    }

    /// # Safety
    /// Called on the game thread with a valid Segue enum value.
    #[no_mangle]
    pub unsafe extern "C" fn setSegue(segue: i32) {
        match segue {
            0 => set("BATTLE_SEGUE", 0),
            1 => set("BATTLE_SEGUE", 1),
            2 => {
                instantVictory = 1;
                set("BATTLE_SEGUE", 1);
            }
            3 => {
                set("BATTLE_SEGUE", 0);
                GlobData.sis_state.crew_enlisted = u16::MAX;
                GlobData.game_state.current_activity |= 0x2000;
            }
            _ => unreachable!("invalid Segue"),
        }
    }

    /// # Safety
    /// Called on the game thread with initialized game state.
    #[no_mangle]
    pub unsafe extern "C" fn getSegue() -> i32 {
        if get("BATTLE_SEGUE") == 0 {
            if GlobData.sis_state.crew_enlisted == u16::MAX
                && GlobData.game_state.current_activity & 0x2000 != 0
            {
                3
            } else {
                0
            }
        } else if instantVictory != 0 {
            2
        } else {
            1
        }
    }

    /// # Safety
    /// Called on the game thread with initialized resources.
    #[no_mangle]
    pub unsafe extern "C" fn init_race(conversation: u32) -> *mut c_void {
        crate::comm::dispatch::rust_init_race_dispatch(conversation)
    }

    /// # Safety
    /// Called on the game thread with a valid phrase index and callback.
    #[no_mangle]
    pub unsafe extern "C" fn NPCPhrase_cb(index: i32, callback: Option<unsafe extern "C" fn()>) {
        crate::comm::ffi::rust_NPCPhrase_cb(index, callback);
    }

    /// # Safety
    /// Called on the game thread with a nonnegative phrase index.
    #[no_mangle]
    pub unsafe extern "C" fn NPCPhrase_splice(index: i32) {
        crate::comm::ffi::rust_NPCPhrase_splice(index);
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
