//! Thradd dialogue state machine — ported from C.
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
const HOSTILE_SPACE_HELLO_1: u32 = 1;
const HOSTILE_SPACE_HELLO_2: u32 = 2;
const HOSTILE_SPACE_HELLO_3: u32 = 3;
const HOSTILE_SPACE_HELLO_4: u32 = 4;
const HOSTILE_HOMEWORLD_HELLO_1: u32 = 5;
const HOSTILE_HOMEWORLD_HELLO_2: u32 = 6;
const HOSTILE_HOMEWORLD_HELLO_3: u32 = 7;
const HOSTILE_HOMEWORLD_HELLO_4: u32 = 8;
const WHATS_UP_HOSTILE_1: u32 = 9;
const WHATS_UP_HOSTILE_2: u32 = 10;
const GENERAL_INFO_HOSTILE_1: u32 = 11;
const GENERAL_INFO_HOSTILE_2: u32 = 12;
const GENERAL_INFO_HOSTILE_3: u32 = 13;
const GENERAL_INFO_HOSTILE_4: u32 = 14;
const WHAT_ABOUT_YOU_1: u32 = 15;
const ABOUT_US_1: u32 = 16;
const WHAT_ABOUT_YOU_2: u32 = 17;
const ABOUT_US_2: u32 = 18;
const WHAT_ABOUT_URQUAN_1: u32 = 19;
const ABOUT_URQUAN_1: u32 = 20;
const WHAT_ABOUT_URQUAN_2: u32 = 21;
const ABOUT_URQUAN_2: u32 = 22;
const GOT_IDEA: u32 = 23;
const GOOD_IDEA: u32 = 24;
const WE_GO_TO_IMPRESS_URQUAN_1: u32 = 25;
const WE_GO_TO_IMPRESS_URQUAN_2: u32 = 26;
const WE_IMPRESSING_URQUAN_1: u32 = 27;
const WE_IMPRESSING_URQUAN_2: u32 = 28;
const WE_IMPRESSED_URQUAN_1: u32 = 29;
const WE_IMPRESSED_URQUAN_2: u32 = 30;
const HOSTILE_HELIX_HELLO_1: u32 = 31;
const HOSTILE_HELIX_HELLO_2: u32 = 32;
const SUBMIT_1: u32 = 33;
const NO_SUBMIT_1: u32 = 34;
const SUBMIT_2: u32 = 35;
const NO_SUBMIT_2: u32 = 36;
const BE_FRIENDS_1: u32 = 37;
const NO_FRIENDS_1: u32 = 38;
const BE_FRIENDS_2: u32 = 39;
const NO_FRIENDS_2: u32 = 40;
const HOW_IMPRESSED_URQUAN_1: u32 = 41;
const IMPRESSED_LIKE_SO_1: u32 = 42;
const HOW_IMPRESSED_URQUAN_2: u32 = 43;
const IMPRESSED_LIKE_SO_2: u32 = 44;
const BYE_HOSTILE_1: u32 = 45;
const GOODBYE_HOSTILE_1: u32 = 46;
const BYE_HOSTILE_2: u32 = 47;
const GOODBYE_HOSTILE_2: u32 = 48;
const WHY_YOU_HERE_HOSTILE: u32 = 49;
const NONE_OF_YOUR_CONCERN: u32 = 50;
const DEMAND_TO_LAND: u32 = 51;
const NO_DEMAND: u32 = 52;
const WHAT_ABOUT_THIS_WORLD: u32 = 53;
const BLUE_HELIX: u32 = 54;
const WHATS_HELIX_HOSTILE: u32 = 55;
const HELIX_IS_HOSTILE: u32 = 56;
const I_NEED_TO_LAND_LIE: u32 = 57;
const CAUGHT_LIE: u32 = 58;
const BYE_HOSTILE_HELIX: u32 = 59;
const GOODBYE_HOSTILE_HELIX: u32 = 60;
const DIE_THIEF_1: u32 = 61;
const DIE_THIEF_2: u32 = 62;
const AMAZING_PERFORMANCE: u32 = 63;
const IMPRESSIVE_PERFORMANCE: u32 = 64;
const ADEQUATE_PERFORMANCE: u32 = 65;
const HELLO_POLITE_1: u32 = 66;
const HELLO_POLITE_2: u32 = 67;
const HELLO_POLITE_3: u32 = 68;
const HELLO_POLITE_4: u32 = 69;
const HELLO_RHYME_1: u32 = 70;
const HELLO_RHYME_2: u32 = 71;
const HELLO_RHYME_3: u32 = 72;
const HELLO_RHYME_4: u32 = 73;
const HELLO_PIG_LATIN_1: u32 = 74;
const HELLO_PIG_LATIN_2: u32 = 75;
const HELLO_PIG_LATIN_3: u32 = 76;
const HELLO_PIG_LATIN_4: u32 = 77;
const HELLO_LIKE_YOU_1: u32 = 78;
const HELLO_LIKE_YOU_2: u32 = 79;
const HELLO_LIKE_YOU_3: u32 = 80;
const HELLO_LIKE_YOU_4: u32 = 81;
const WELCOME_SPACE0: u32 = 82;
const WELCOME_SPACE1: u32 = 83;
const WELCOME_HOMEWORLD0: u32 = 84;
const WELCOME_HOMEWORLD1: u32 = 85;
const WELCOME_HELIX0: u32 = 86;
const WELCOME_HELIX1: u32 = 87;
const WHY_YOU_HERE_ALLY: u32 = 88;
const GUARDING_HELIX_ALLY: u32 = 89;
const WHATS_HELIX_ALLY: u32 = 90;
const HELIX_IS_ALLY: u32 = 91;
const MAY_I_LAND: u32 = 92;
const SURE_LAND: u32 = 93;
const WHATS_UP_ALLY: u32 = 94;
const GENERAL_INFO_ALLY_1: u32 = 95;
const GENERAL_INFO_ALLY_2: u32 = 96;
const GENERAL_INFO_ALLY_3: u32 = 97;
const GENERAL_INFO_ALLY_4: u32 = 98;
const HOW_SHOULD_WE_ACT: u32 = 99;
const FRIENDLY: u32 = 100;
const OK_FRIENDLY: u32 = 101;
const WACKY: u32 = 102;
const OK_WACKY: u32 = 103;
const JUST_LIKE_US: u32 = 104;
const OK_JUST_LIKE_YOU: u32 = 105;
const WORK_TO_DO: u32 = 106;
const CONTEMPLATIVE: u32 = 107;
const OK_CONTEMPLATIVE: u32 = 108;
const HOW_GOES_CULTURE: u32 = 109;
const CONTEMP_GOES_1: u32 = 110;
const CONTEMP_GOES_2: u32 = 111;
const FRIENDLY_GOES_1: u32 = 112;
const FRIENDLY_GOES_2: u32 = 113;
const WACKY_GOES_1: u32 = 114;
const WACKY_GOES_2: u32 = 115;
const LIKE_YOU_GOES_1: u32 = 116;
const LIKE_YOU_GOES_2: u32 = 117;
const BYE_ALLY: u32 = 118;
const GOODBYE_ALLY_1: u32 = 119;
const GOODBYE_ALLY_2: u32 = 120;
const GOODBYE_ALLY_3: u32 = 121;
const GOODBYE_ALLY_4: u32 = 122;
const BE_POLITE: u32 = 123;
const OK_POLITE: u32 = 124;
const SPEAK_PIG_LATIN: u32 = 125;
const OK_PIG_LATIN: u32 = 126;
const USE_RHYMES: u32 = 127;
const OK_RHYMES: u32 = 128;
const JUST_THE_WAY_WE_DO: u32 = 129;
const OK_WAY_YOU_DO: u32 = 130;
const WHAT_NAME_FOR_CULTURE: u32 = 131;
const ALLIANCE_NAME: u32 = 132;
const OK_ALLIANCE_NAME: u32 = 133;
const NAME_TAIL: u32 = 134;
const YOU_DECIDE: u32 = 135;
const OK_CULTURE_20: u32 = 136;
const FAT: u32 = 137;
const OK_FAT: u32 = 138;
const THE_SLAVE_EMPIRE0: u32 = 139;
const THE_SLAVE_EMPIRE1: u32 = 140;
const OK_SLAVE: u32 = 141;
const FAT_JERKS: u32 = 142;
const CULTURE: u32 = 143;
const SLAVE_EMPIRE: u32 = 144;
const NAME_1: u32 = 145;
const NAME_2: u32 = 146;
const NAME_3: u32 = 147;
const NAME_40: u32 = 148;
const NAME_41: u32 = 149;
const HAVING_FUN_WITH_ILWRATH_1: u32 = 150;
const HAVING_FUN_WITH_ILWRATH_2: u32 = 151;
const GO_AWAY_FIGHTING_ILWRATH_1: u32 = 152;
const GO_AWAY_FIGHTING_ILWRATH_2: u32 = 153;
const OUT_TAKES: u32 = 154;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.thraddash.graphics\0";
const RACE_FONT: &[u8] = b"comm.thraddash.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.thraddash.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.thraddash.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.thraddash.dialogue\0";

/// Thradd race dialogue implementation.
pub struct ThraddDialogue;

impl super::RaceDialogue for ThraddDialogue {
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
            bit_range("AQUA_HELIX").is_some(),
            "missing game state key: AQUA_HELIX"
        );
        assert!(
            bit_range("GLOBAL_FLAGS_AND_DATA").is_some(),
            "missing game state key: GLOBAL_FLAGS_AND_DATA"
        );
        assert!(
            bit_range("HELIX_UNPROTECTED").is_some(),
            "missing game state key: HELIX_UNPROTECTED"
        );
        assert!(
            bit_range("HELIX_VISITS").is_some(),
            "missing game state key: HELIX_VISITS"
        );
        assert!(
            bit_range("ILWRATH_FIGHT_THRADDASH").is_some(),
            "missing game state key: ILWRATH_FIGHT_THRADDASH"
        );
    }
}
