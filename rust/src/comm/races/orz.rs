//! Orz dialogue state machine — ported from C.
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
const INIT_HELLO: u32 = 1;
const WHO_YOU: u32 = 2;
const WE_ARE_ORZ: u32 = 3;
const WHY_HERE: u32 = 4;
const HERE_BECAUSE: u32 = 5;
const ALLIED_HOMEWORLD_HELLO_1: u32 = 6;
const ALLIED_HOMEWORLD_HELLO_2: u32 = 7;
const ALLIED_HOMEWORLD_HELLO_3: u32 = 8;
const ALLIED_HOMEWORLD_HELLO_4: u32 = 9;
const ALLIED_SPACE_HELLO_1: u32 = 10;
const ALLIED_SPACE_HELLO_2: u32 = 11;
const ALLIED_SPACE_HELLO_3: u32 = 12;
const ALLIED_SPACE_HELLO_4: u32 = 13;
const WHATS_UP_ALLY: u32 = 14;
const GENERAL_INFO_ALLY_1: u32 = 15;
const GENERAL_INFO_ALLY_2: u32 = 16;
const GENERAL_INFO_ALLY_3: u32 = 17;
const GENERAL_INFO_ALLY_4: u32 = 18;
const MORE_ABOUT_YOU: u32 = 19;
const ABOUT_US_1: u32 = 20;
const ABOUT_US_2: u32 = 21;
const ABOUT_US_3: u32 = 22;
const ABOUT_US_4: u32 = 23;
const WHERE_ANDROSYN: u32 = 24;
const DISEMBLE_ABOUT_ANDROSYN: u32 = 25;
const MUST_KNOW_ABOUT_ANDROSYN: u32 = 26;
const KNOW_TOO_MUCH: u32 = 27;
const DONT_REALLY_CARE: u32 = 28;
const YOU_ARE_OUR_FRIENDS: u32 = 29;
const ABOUT_ANDRO_1: u32 = 30;
const FORGET_ANDRO_1: u32 = 31;
const ABOUT_ANDRO_2: u32 = 32;
const FORGET_ANDRO_2: u32 = 33;
const ABOUT_ANDRO_3: u32 = 34;
const BLEW_IT: u32 = 35;
const NEUTRAL_HOMEWORLD_HELLO_1: u32 = 36;
const NEUTRAL_HOMEWORLD_HELLO_2: u32 = 37;
const NEUTRAL_HOMEWORLD_HELLO_3: u32 = 38;
const NEUTRAL_HOMEWORLD_HELLO_4: u32 = 39;
const NEUTRAL_SPACE_HELLO_1: u32 = 40;
const NEUTRAL_SPACE_HELLO_2: u32 = 41;
const NEUTRAL_SPACE_HELLO_3: u32 = 42;
const NEUTRAL_SPACE_HELLO_4: u32 = 43;
const HOSTILE_1: u32 = 44;
const HOSTILITY_IS_BAD_1: u32 = 45;
const HOSTILE_2: u32 = 46;
const HOSTILITY_IS_BAD_2: u32 = 47;
const WE_ARE_VINDICATOR0: u32 = 48;
const WE_ARE_VINDICATOR1: u32 = 49;
const WE_ARE_VINDICATOR2: u32 = 50;
const NICE_TO_MEET_YOU: u32 = 51;
const SEEM_LIKE_NICE_GUYS: u32 = 52;
const ARE_NICE_WANT_ALLY: u32 = 53;
const TALK_ABOUT_ALLIANCE: u32 = 54;
const OK_TALK_ALLIANCE: u32 = 55;
const YES_ALLIANCE: u32 = 56;
const GREAT: u32 = 57;
const NO_ALLIANCE: u32 = 58;
const MAYBE_LATER: u32 = 59;
const DECIDE_LATER: u32 = 60;
const OK_LATER: u32 = 61;
const WHY_SO_TRUSTING: u32 = 62;
const TRUSTING_BECAUSE: u32 = 63;
const BYE_NEUTRAL: u32 = 64;
const GOODBYE_NEUTRAL: u32 = 65;
const ANGRY_SPACE_HELLO_1: u32 = 66;
const ANGRY_SPACE_HELLO_2: u32 = 67;
const ANGRY_HOMEWORLD_HELLO_1: u32 = 68;
const ANGRY_HOMEWORLD_HELLO_2: u32 = 69;
const WHATS_UP_ANGRY: u32 = 70;
const GENERAL_INFO_ANGRY_1: u32 = 71;
const GENERAL_INFO_ANGRY_2: u32 = 72;
const WERE_SORRY: u32 = 73;
const APOLOGY_ACCEPTED: u32 = 74;
const INSULT_1: u32 = 75;
const INSULT_2: u32 = 76;
const INSULT_3: u32 = 77;
const INSULT_4: u32 = 78;
const INSULT_5: u32 = 79;
const INSULT_6: u32 = 80;
const INSULT_7: u32 = 81;
const INSULT_8: u32 = 82;
const INSULTED_1: u32 = 83;
const INSULTED_2: u32 = 84;
const INSULTED_3: u32 = 85;
const INSULTED_4: u32 = 86;
const BYE_ANGRY: u32 = 87;
const GOODBYE_ANGRY: u32 = 88;
const ANGRY_TAALO_HELLO_1: u32 = 89;
const ANGRY_TAALO_HELLO_2: u32 = 90;
const FRIENDLY_ALLIED_TAALO_HELLO_1: u32 = 91;
const FRIENDLY_ALLIED_TAALO_HELLO_2: u32 = 92;
const DEMAND_TO_LAND: u32 = 93;
const NO_DEMAND: u32 = 94;
const ASK_NICELY: u32 = 95;
const WHY_YOU_HERE: u32 = 96;
const ANGRY_EXPLANATION: u32 = 97;
const FRIENDLY_EXPLANATION: u32 = 98;
const WHAT_IS_THIS_PLACE: u32 = 99;
const FRIENDLY_PLACE: u32 = 100;
const ANGRY_PLACE: u32 = 101;
const MAY_WE_LAND: u32 = 102;
const SURE_LAND: u32 = 103;
const ALLIES_CAN_VISIT: u32 = 104;
const MAKE_ALLIANCE: u32 = 105;
const CANT_ALLY_HERE: u32 = 106;
const WHY_BUSY: u32 = 107;
const BUSY_BECAUSE: u32 = 108;
const BYE_TAALO: u32 = 109;
const BYE_ALLY: u32 = 110;
const GOODBYE_ALLY: u32 = 111;
const FRIENDLY_TAALO_GOODBYE: u32 = 112;
const ANGRY_TAALO_GOODBYE: u32 = 113;
const HOSTILE_HELLO_1: u32 = 114;
const HOSTILE_HELLO_2: u32 = 115;
const OUT_TAKES: u32 = 116;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.orz.graphics\0";
const RACE_FONT: &[u8] = b"comm.orz.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.orz.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.orz.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.orz.dialogue\0";

/// Orz race dialogue implementation.
pub struct OrzDialogue;

impl super::RaceDialogue for OrzDialogue {
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
            bit_range("MET_ORZ_BEFORE").is_some(),
            "missing game state key: MET_ORZ_BEFORE"
        );
        assert!(
            bit_range("ORZ_ANDRO_STATE").is_some(),
            "missing game state key: ORZ_ANDRO_STATE"
        );
        assert!(
            bit_range("ORZ_GENERAL_INFO").is_some(),
            "missing game state key: ORZ_GENERAL_INFO"
        );
        assert!(
            bit_range("ORZ_HOME_VISITS").is_some(),
            "missing game state key: ORZ_HOME_VISITS"
        );
    }
}
