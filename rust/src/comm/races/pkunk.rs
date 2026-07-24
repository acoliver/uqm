//! Pkunk dialogue state machine — ported from C.
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
const GIVE_SPINDLE: u32 = 1;
const NAME_1: u32 = 2;
const NAME_2: u32 = 3;
const NAME_3: u32 = 4;
const NAME_40: u32 = 5;
const NAME_41: u32 = 6;
const NEUTRAL_SPACE_HELLO_1: u32 = 7;
const NEUTRAL_SPACE_HELLO_3: u32 = 8;
const NEUTRAL_SPACE_HELLO_2: u32 = 9;
const NEUTRAL_SPACE_HELLO_4: u32 = 10;
const FRIENDLY_SPACE_HELLO_1: u32 = 11;
const FRIENDLY_SPACE_HELLO_2: u32 = 12;
const FRIENDLY_SPACE_HELLO_3: u32 = 13;
const FRIENDLY_SPACE_HELLO_4: u32 = 14;
const FRIENDLY_SPACE_HELLO_5: u32 = 15;
const FRIENDLY_SPACE_HELLO_6: u32 = 16;
const FRIENDLY_SPACE_HELLO_7: u32 = 17;
const FRIENDLY_SPACE_HELLO_8: u32 = 18;
const NEUTRAL_HOMEWORLD_HELLO_1: u32 = 19;
const NEUTRAL_HOMEWORLD_HELLO_2: u32 = 20;
const NEUTRAL_HOMEWORLD_HELLO_3: u32 = 21;
const NEUTRAL_HOMEWORLD_HELLO_4: u32 = 22;
const FRIENDLY_HOMEWORLD_HELLO_1: u32 = 23;
const FRIENDLY_HOMEWORLD_HELLO_2: u32 = 24;
const FRIENDLY_HOMEWORLD_HELLO_3: u32 = 25;
const FRIENDLY_HOMEWORLD_HELLO_4: u32 = 26;
const FRIENDLY_HOMEWORLD_HELLO_5: u32 = 27;
const FRIENDLY_HOMEWORLD_HELLO_6: u32 = 28;
const FRIENDLY_HOMEWORLD_HELLO_7: u32 = 29;
const FRIENDLY_HOMEWORLD_HELLO_8: u32 = 30;
const WHATS_UP_NEUTRAL: u32 = 31;
const GENERAL_INFO_NEUTRAL_1: u32 = 32;
const GENERAL_INFO_NEUTRAL_2: u32 = 33;
const GENERAL_INFO_NEUTRAL_3: u32 = 34;
const GENERAL_INFO_NEUTRAL_4: u32 = 35;
const GOOD_REASON_1: u32 = 36;
const WE_GO_HOME_1: u32 = 37;
const GOOD_REASON_2: u32 = 38;
const WE_GO_HOME_2: u32 = 39;
const BAD_REASON_1: u32 = 40;
const NO_GO_HOME_1: u32 = 41;
const BAD_REASON_2: u32 = 42;
const NO_GO_HOME_2: u32 = 43;
const SENSE_KOHRAH_VICTORY: u32 = 44;
const SPIRITUAL_PROBLEMS_1: u32 = 45;
const SPIRITUAL_PROBLEMS_2: u32 = 46;
const SPIRITUAL_PROBLEMS_3: u32 = 47;
const SPIRITUAL_PROBLEMS_4: u32 = 48;
const HATE_YOU_FOREVER_1: u32 = 49;
const HATE_YOU_FOREVER_2: u32 = 50;
const HATE_YOU_FOREVER_3: u32 = 51;
const HATE_YOU_FOREVER_4: u32 = 52;
const MIGRATING_SPACE_1: u32 = 53;
const MIGRATING_SPACE_2: u32 = 54;
const MIGRATING_SPACE_3: u32 = 55;
const MIGRATING_SPACE_4: u32 = 56;
const MIGRATING_SPACE_5: u32 = 57;
const MIGRATING_SPACE_6: u32 = 58;
const MIGRATING_SPACE_7: u32 = 59;
const MIGRATING_SPACE_8: u32 = 60;
const DIE_IDIOT_FOOLS: u32 = 61;
const VERY_WELL: u32 = 62;
const WHY_INSULTS: u32 = 63;
const RELEASE_TENSION: u32 = 64;
const WHAT_ABOUT_YOU_ANGRY: u32 = 65;
const ABOUT_US_ANGRY: u32 = 66;
const WHAT_ABOUT_YOU: u32 = 67;
const SHOULD_BE_FRIENDS: u32 = 68;
const YES_FRIENDS: u32 = 69;
const TRY_TO_BE_NICER: u32 = 70;
const CANT_ASK_FOR_MORE: u32 = 71;
const VISIT_OUR_HOMEWORLD: u32 = 72;
const CAN_BE_FRIENDS: u32 = 73;
const BYE_ANGRY: u32 = 74;
const GOODBYE_ANGRY: u32 = 75;
const WE_CONQUER: u32 = 76;
const WHY_CONQUER: u32 = 77;
const CONQUER_BECAUSE_1: u32 = 78;
const NOT_CONQUER_10: u32 = 79;
const NOT_CONQUER_11: u32 = 80;
const NOT_CONQUER_12: u32 = 81;
const NOT_CONQUER_1: u32 = 82;
const CONQUER_BECAUSE_2: u32 = 83;
const NOT_CONQUER_2: u32 = 84;
const MUST_CONQUER: u32 = 85;
const BAD_IDEA: u32 = 86;
const NO_CONQUEST: u32 = 87;
const GOOD_IDEA: u32 = 88;
const WE_ARE_VINDICATOR0: u32 = 89;
const WE_ARE_VINDICATOR1: u32 = 90;
const WE_ARE_VINDICATOR2: u32 = 91;
const WHY_YOU_HERE: u32 = 92;
const WE_HERE_TO_HELP: u32 = 93;
const NEED_HELP: u32 = 94;
const WE_NEED_HELP: u32 = 95;
const GIVE_HELP: u32 = 96;
const EXPLORING_UNIVERSE: u32 = 97;
const SENSE_DEEPER_CONFLICT: u32 = 98;
const FUN_CRUISE: u32 = 99;
const REPRESS: u32 = 100;
const WHY_ILWRATH_FIGHT: u32 = 101;
const ILWRATH_FIGHT_BECAUSE: u32 = 102;
const WHEN_FIGHT_START: u32 = 103;
const FIGHT_START_WHEN: u32 = 104;
const HOW_GOES_FIGHT: u32 = 105;
const FIGHT_GOES: u32 = 106;
const HOW_GOES_WAR: u32 = 107;
const WAR_GOES_1: u32 = 108;
const WAR_GOES_2: u32 = 109;
const WAR_GOES_3: u32 = 110;
const WAR_GOES_4: u32 = 111;
const HOW_STOP_FIGHT: u32 = 112;
const STOP_FIGHT_LIKE_SO: u32 = 113;
const ENOUGH_ILWRATH: u32 = 114;
const OK_ENOUGH_ILWRATH: u32 = 115;
const WHAT_ABOUT_HISTORY: u32 = 116;
const ABOUT_HISTORY: u32 = 117;
const WHAT_ABOUT_YEHAT: u32 = 118;
const ABOUT_YEHAT: u32 = 119;
const WHAT_ABOUT_CULTURE: u32 = 120;
const ABOUT_CULTURE: u32 = 121;
const ELABORATE_CULTURE: u32 = 122;
const OK_ELABORATE_CULTURE: u32 = 123;
const WHAT_ABOUT_FUTURE: u32 = 124;
const ABOUT_FUTURE: u32 = 125;
const ENOUGH_ABOUT_YOU: u32 = 126;
const OK_ENOUGH_ABOUT_US: u32 = 127;
const ABOUT_US: u32 = 128;
const WHERE_FLEET_1: u32 = 129;
const WHERE_FLEET_2: u32 = 130;
const WHERE_FLEET_3: u32 = 131;
const MIGRATING_HOMEWORLD_1: u32 = 132;
const MIGRATING_HOMEWORLD_2: u32 = 133;
const MIGRATING_HOMEWORLD_3: u32 = 134;
const RETURNING_FROM_YEHAT_1: u32 = 135;
const RETURNING_FROM_YEHAT_2: u32 = 136;
const AM_WORRIED_1: u32 = 137;
const AM_WORRIED_2: u32 = 138;
const AM_WORRIED_3: u32 = 139;
const DONT_WORRY_1: u32 = 140;
const DONT_WORRY_2: u32 = 141;
const DONT_WORRY_3: u32 = 142;
const FORM_ALLIANCE: u32 = 143;
const GO_TO_HOMEWORLD: u32 = 144;
const CAN_YOU_HELP: u32 = 145;
const GO_TO_HOMEWORLD_AGAIN: u32 = 146;
const HOSTILE_GREETING: u32 = 147;
const DONT_BE_HOSTILE: u32 = 148;
const OBEY: u32 = 149;
const NO_OBEY: u32 = 150;
const NEUTRAL_BYE_SPACE: u32 = 151;
const NEUTRAL_GOODBYE_SPACE: u32 = 152;
const SHIP_GIFT: u32 = 153;
const NO_ROOM: u32 = 154;
const FRIENDLY_BYE_SPACE: u32 = 155;
const FRIENDLY_GOODBYE_SPACE: u32 = 156;
const BYE_FRIENDLY: u32 = 157;
const GOODBYE_FRIENDLY: u32 = 158;
const ALMOST_ALLIANCE: u32 = 159;
const INIT_NO_ROOM: u32 = 160;
const INIT_SHIP_GIFT: u32 = 161;
const SUIT_YOURSELF: u32 = 162;
const GOODBYE_MIGRATION: u32 = 163;
const WHAT_ABOUT_ILWRATH: u32 = 164;
const ABOUT_ILWRATH: u32 = 165;
const WHATS_UP_SPACE: u32 = 166;
const SHIPS_AT_HOME: u32 = 167;
const GENERAL_INFO_SPACE_1: u32 = 168;
const GENERAL_INFO_SPACE_2: u32 = 169;
const GENERAL_INFO_SPACE_3: u32 = 170;
const GENERAL_INFO_SPACE_4: u32 = 171;
const GENERAL_INFO_SPACE_5: u32 = 172;
const GENERAL_INFO_SPACE_6: u32 = 173;
const GENERAL_INFO_SPACE_7: u32 = 174;
const GENERAL_INFO_SPACE_8: u32 = 175;
const TELL_MY_FORTUNE: u32 = 176;
const FORTUNE_IS_1: u32 = 177;
const FORTUNE_IS_2: u32 = 178;
const FORTUNE_IS_3: u32 = 179;
const FORTUNE_IS_4: u32 = 180;
const FORTUNE_IS_5: u32 = 181;
const FORTUNE_IS_6: u32 = 182;
const FORTUNE_IS_7: u32 = 183;
const FORTUNE_IS_8: u32 = 184;
const OUT_TAKES: u32 = 185;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.pkunk.graphics\0";
const RACE_FONT: &[u8] = b"comm.pkunk.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.pkunk.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.pkunk.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.pkunk.dialogue\0";

/// Pkunk race dialogue implementation.
pub struct PkunkDialogue;

impl super::RaceDialogue for PkunkDialogue {
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
            bit_range("CLEAR_SPINDLE").is_some(),
            "missing game state key: CLEAR_SPINDLE"
        );
        assert!(
            bit_range("CLEAR_SPINDLE_ON_SHIP").is_some(),
            "missing game state key: CLEAR_SPINDLE_ON_SHIP"
        );
        assert!(
            bit_range("GLOBAL_FLAGS_AND_DATA").is_some(),
            "missing game state key: GLOBAL_FLAGS_AND_DATA"
        );
        assert!(
            bit_range("ILWRATH_DECEIVED").is_some(),
            "missing game state key: ILWRATH_DECEIVED"
        );
        assert!(
            bit_range("KNOW_KOHR_AH_STORY").is_some(),
            "missing game state key: KNOW_KOHR_AH_STORY"
        );
    }
}
