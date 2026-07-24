//! Ilwrath dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

#![allow(dead_code)]

use std::ffi::c_char;
use std::os::raw::c_int;

use crate::comm::segue::Segue;
use crate::comm::types::{CommData, TextAlign, TextValign};
use crate::state::game_state_keys::bit_range;

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

const NULL_PHRASE: u32 = 0;
const NEVER_ENOUGH: u32 = 1;
const OK_WARSHIP: u32 = 2;
const OK_DWE: u32 = 3;
const OK_YOUBOO: u32 = 4;
const OK_DILRAT: u32 = 5;
const BIG_FUN: u32 = 6;
const FAST_AS_CAN: u32 = 7;
const GLORIOUS_WORSHIP: u32 = 8;
const ON_WAY: u32 = 9;
const GODS_RETURN_1: u32 = 10;
const GODS_RETURN_2: u32 = 11;
const GODS_RETURN_3: u32 = 12;
const JUST_GRUNTS: u32 = 13;
const GRUNTS_AGAIN: u32 = 14;
const WHAT_ORDERS: u32 = 15;
const WE_WORSHIP_1: u32 = 16;
const WE_WORSHIP_2: u32 = 17;
const WE_WORSHIP_3: u32 = 18;
const SUBSEQUENT_CHMMR_HELLO: u32 = 19;
const INIT_CHMMR_HELLO: u32 = 20;
const OK_ENOUGH_ILWRATH: u32 = 21;
const OK_ENOUGH_GODS: u32 = 22;
const SEND_MESSAGE: u32 = 23;
const CAME_FROM: u32 = 24;
const WHO_BLASTS_WHO: u32 = 25;
const NO_SURRENDER: u32 = 26;
const NOT_REASONABLE: u32 = 27;
const SUBSEQUENT_HOME_HELLO: u32 = 28;
const GENERAL_INFO: u32 = 29;
const GOODBYE_AND_DIE: u32 = 30;
const DECEIVERS: u32 = 31;
const NO_PEACE: u32 = 32;
const NO_ALLIANCE: u32 = 33;
const ILWRATH_BELIEVE: u32 = 34;
const OK_KILL_THRADDASH: u32 = 35;
const GOODBYE_GODS: u32 = 36;
const INIT_HELLO_SPACE: u32 = 37;
const SUBSEQUENT_HELLO_SPACE_1: u32 = 38;
const SUBSEQUENT_HELLO_SPACE_2: u32 = 39;
const SUBSEQUENT_HELLO_SPACE_3: u32 = 40;
const SUBSEQUENT_HELLO_SPACE_4: u32 = 41;
const GENERAL_INFO_SPACE_1: u32 = 42;
const GENERAL_INFO_SPACE_2: u32 = 43;
const GENERAL_INFO_SPACE_3: u32 = 44;
const GENERAL_INFO_SPACE_4: u32 = 45;
const GENERAL_INFO_SPACE_5: u32 = 46;
const STRENGTH_NOT_ALL: u32 = 47;
const NO_SLAY_BY_THOUSANDS: u32 = 48;
const NO_EASE_UP: u32 = 49;
const GOODBYE_AND_DIE_SPACE: u32 = 50;
const INIT_HOME_HELLO: u32 = 51;
const GOODBYE_AND_DIE_HOMEWORLD: u32 = 52;
const SO_MUCH_TO_KNOW: u32 = 53;
const LONG_AGO: u32 = 54;
const KILLED_GOOD_GODS: u32 = 55;
const CHANNEL_44: u32 = 56;
const BECAUSE_44: u32 = 57;
const WHAT_ABOUT_ILWRATH: u32 = 58;
const ABOUT_PHYSIO: u32 = 59;
const ABOUT_HISTORY: u32 = 60;
const ABOUT_CULTURE: u32 = 61;
const ABOUT_URQUAN: u32 = 62;
const URQUAN_TOO_NICE: u32 = 63;
const OF_COURSE_WERE_EVIL: u32 = 64;
const DONT_CONFUSE_US: u32 = 65;
const ON_WAY_TO_THRADDASH: u32 = 66;
const HAPPY_FIGHTING_THRADDASH: u32 = 67;
const SAY_WARSHIP: u32 = 68;
const SAY_DWE: u32 = 69;
const SAY_YOUBOO: u32 = 70;
const SAY_DILLRAT: u32 = 71;
const ENOUGH_ORDERS: u32 = 72;
const OTHER_DIVINE_ORDERS: u32 = 73;
const WORSHIP_US: u32 = 74;
const BYE_GODS: u32 = 75;
const ENOUGH_ILWRATH: u32 = 76;
const ENOUGH_GODS: u32 = 77;
const WHERE_YOU_COME_FROM: u32 = 78;
const IT_WILL_BE_A_PLEASURE: u32 = 79;
const BE_REASONABLE: u32 = 80;
const SURRENDER: u32 = 81;
const WHATS_UP: u32 = 82;
const BYE: u32 = 83;
const WANT_PEACE: u32 = 84;
const WANT_ALLIANCE: u32 = 85;
const GO_KILL_THRADDASH: u32 = 86;
const WHATS_UP_SPACE_1: u32 = 87;
const WHATS_UP_SPACE_2: u32 = 88;
const WHATS_UP_SPACE_3: u32 = 89;
const WHATS_UP_SPACE_4: u32 = 90;
const WHATS_UP_SPACE_5: u32 = 91;
const YOU_ARE_WEAK: u32 = 92;
const SLAY_BY_THOUSANDS: u32 = 93;
const EASE_UP: u32 = 94;
const BYE_SPACE: u32 = 95;
const BYE_HOMEWORLD: u32 = 96;
const WANT_INFO_ON_GODS: u32 = 97;
const WHEN_START_WORSHIP: u32 = 98;
const ANY_GOOD_GODS: u32 = 99;
const HOW_TALK_WITH_GODS: u32 = 100;
const WHY_44: u32 = 101;
const WANT_INFO_ON_ILWRATH: u32 = 102;
const WHAT_ABOUT_PHYSIO: u32 = 103;
const WHAT_ABOUT_HISTORY: u32 = 104;
const WHAT_ABOUT_CULTURE: u32 = 105;
const WHAT_ABOUT_URQUAN: u32 = 106;
const ARE_YOU_EVIL: u32 = 107;
const BUT_EVIL_IS_DEFINED: u32 = 108;

// ---------------------------------------------------------------------------
// Game state helpers
// ---------------------------------------------------------------------------

fn get_gs(key: &str) -> u8 {
    let (start, end) = match bit_range(key) {
        Some(r) => r,
        None => return 0,
    };
    unsafe { rust_get_game_state_bits(start as c_int, end as c_int) }
}

fn set_gs(key: &str, val: u8) {
    if let Some((start, end)) = bit_range(key) {
        unsafe { rust_set_game_state_bits(start as c_int, end as c_int, val) };
    }
}

extern "C" {
    fn rust_NPCPhrase_cb(index: c_int, cb: Option<extern "C" fn()>);
    fn rust_PhraseEnabled(index: c_int) -> c_int;
    fn rust_DisablePhrase(index: c_int);
    fn DoResponsePhrase(
        response_ref: u32,
        response_func: Option<extern "C" fn(u32)>,
        construct_str: *const c_char,
    );
    fn rust_get_game_state_bits(start: c_int, end: c_int) -> u8;
    fn rust_set_game_state_bits(start: c_int, end: c_int, val: u8);
    fn rust_add_event_relative(days_offset: u32, func_index: u8) -> u32;
}

fn npc_phrase(index: u32) {
    if index == 0 {
        return;
    }
    unsafe { rust_NPCPhrase_cb(index as c_int, None) };
}

fn phrase_enabled(index: u32) -> bool {
    unsafe { rust_PhraseEnabled(index as c_int) != 0 }
}

fn disable_phrase(index: u32) {
    unsafe { rust_DisablePhrase(index as c_int) };
}

fn response(phrase: u32, callback: extern "C" fn(u32)) {
    unsafe { DoResponsePhrase(phrase, Some(callback), std::ptr::null()) };
}

fn set_segue(segue: Segue) {
    crate::comm::state::COMM_STATE.write().set_segue(segue);
}

fn get_segue() -> Segue {
    crate::comm::state::COMM_STATE.read().get_segue()
}

fn get_current_activity() -> u16 {
    unsafe { crate::mainloop::c_extern::get_current_activity() }
}

fn lobyte(val: u16) -> u8 {
    (val & 0xFF) as u8
}

// ---------------------------------------------------------------------------
// Resource keys (from resinst.h)
// ---------------------------------------------------------------------------

const RACE_PMAP_ANIM: &[u8] = b"comm.ilwrath.graphics\0";
const RACE_FONT: &[u8] = b"comm.ilwrath.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.ilwrath.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.ilwrath.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.ilwrath.dialogue\0";

/// Ilwrath race dialogue implementation.
pub struct IlwrathDialogue;

impl super::RaceDialogue for IlwrathDialogue {
    fn init(&self) -> CommData {
        CommData {
            alien_frame_res: RACE_PMAP_ANIM.as_ptr() as *const _,
            alien_font_res: RACE_FONT.as_ptr() as *const _,
            alien_colormap_res: RACE_COLOR_MAP.as_ptr() as *const _,
            alien_song_res: RACE_MUSIC.as_ptr() as *const _,
            alien_alt_song_res: std::ptr::null(),
            conversation_phrases_res: RACE_CONVERSATION_PHRASES.as_ptr() as *const _,
            alien_text_align: TextAlign::Center,
            alien_text_valign: TextValign::Top,
            alien_text_fcolor: 0x00FFFFFF,
            alien_text_bcolor: 0x00000000,
            ..CommData::default()
        }
    }

    fn intro(&self) {
        // TODO: Port intro state machine from C
    }

    fn post_encounter(&self) {
        // TODO: Port post_encounter from C
    }

    fn uninit(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_keys_are_null_terminated() {
        assert_eq!(RACE_PMAP_ANIM.last(), Some(&0));
        assert_eq!(RACE_FONT.last(), Some(&0));
        assert_eq!(RACE_COLOR_MAP.last(), Some(&0));
        assert_eq!(RACE_MUSIC.last(), Some(&0));
    }

    #[test]
    fn test_game_state_keys_exist() {
        assert!(
            bit_range("GLOBAL_FLAGS_AND_DATA").is_some(),
            "missing game state key: GLOBAL_FLAGS_AND_DATA"
        );
        assert!(
            bit_range("ILWRATH_CHMMR_VISITS").is_some(),
            "missing game state key: ILWRATH_CHMMR_VISITS"
        );
        assert!(
            bit_range("ILWRATH_DECEIVED").is_some(),
            "missing game state key: ILWRATH_DECEIVED"
        );
        assert!(
            bit_range("ILWRATH_FIGHT_THRADDASH").is_some(),
            "missing game state key: ILWRATH_FIGHT_THRADDASH"
        );
        assert!(
            bit_range("ILWRATH_GODS_SPOKEN").is_some(),
            "missing game state key: ILWRATH_GODS_SPOKEN"
        );
    }
}
