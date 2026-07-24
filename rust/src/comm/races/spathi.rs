//! Spathi dialogue state machine — ported from C.
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
const SORRY_ABOUT_THAT: u32 = 1;
const IDENTIFY: u32 = 2;
const I_FWIFFO: u32 = 3;
const HI_THERE: u32 = 4;
const ARE_YOU_SURE: u32 = 5;
const DONT_KILL: u32 = 6;
const WE_FIGHT_1: u32 = 7;
const WE_FIGHT_2: u32 = 8;
const OK_WONT: u32 = 9;
const DO_CULTURAL: u32 = 10;
const WEZZY_WEZZAH: u32 = 11;
const DIE_SLUGBOY: u32 = 12;
const BEGIN_RITUAL: u32 = 13;
const MUST_DO_RITUAL_AT_HOME: u32 = 14;
const YOU_WONT_DIE_YET: u32 = 15;
const ETERNAL_GRATITUDE: u32 = 16;
const WE_FIGHT: u32 = 17;
const PAY_FOR_CRIMES: u32 = 18;
const CLUTCH_MAVEN: u32 = 19;
const YOU_MAY_LIVE: u32 = 20;
const HONEST_AND_FRIENDLY: u32 = 21;
const WHAT_ARE_COORDINATES: u32 = 22;
const COORDINATES_ARE: u32 = 23;
const TELL_ME_COORDINATES: u32 = 24;
const FAKE_COORDINATES: u32 = 25;
const TOO_SCARY: u32 = 26;
const YOUVE_GOT_ME_ALL_WRONG: u32 = 27;
const SORRY_NO_COORDS: u32 = 28;
const WHAT_DOING_ON_PLUTO_1: u32 = 29;
const ABOUT_20_YEARS_AGO: u32 = 30;
const WHAT_DOING_ON_PLUTO_2: u32 = 31;
const WHEN_URQUAN_ARRIVED: u32 = 32;
const WHERE_ARE_URQUAN: u32 = 33;
const URQUAN_LEFT: u32 = 34;
const WHAT_ABOUT_OTHER_RACES: u32 = 35;
const ABOUT_OTHER_RACES: u32 = 36;
const WHAT_DOING_ON_PLUTO_3: u32 = 37;
const WHAT_ABOUT_YOURSELF: u32 = 38;
const ABOUT_MYSELF: u32 = 39;
const STATIONED_ON_EARTH_MOON: u32 = 40;
const WHAT_BLAZE_OF_GLORY: u32 = 41;
const BLAZE_IS: u32 = 42;
const WHAT_ABOUT_MOONBASE: u32 = 43;
const SET_UP_BASE: u32 = 44;
const WHAT_ABOUT_ILWRATH: u32 = 45;
const ABOUT_ILWRATH: u32 = 46;
const WHAT_ABOUT_OTHER_SPATHI: u32 = 47;
const REALLY_THOUSANDS: u32 = 48;
const SPATHI_ARE: u32 = 49;
const WHAT_ENEMY: u32 = 50;
const ENEMY_IS: u32 = 51;
const WHEN_ILWRATH: u32 = 52;
const THEN_ILWRATH: u32 = 53;
const WHY_YOU_HERE: u32 = 54;
const DREW_SHORT_STRAW: u32 = 55;
const HOW_MANY_CREW: u32 = 56;
const JUST_ME: u32 = 57;
const THOUSANDS: u32 = 58;
const FULL_OF_MONSTERS: u32 = 59;
const HOW_TRUE: u32 = 60;
const JOIN_US: u32 = 61;
const WILL_JOIN: u32 = 62;
const WONT_JOIN_1: u32 = 63;
const GIVE_SHIP_OR_DIE: u32 = 64;
const WONT_JOIN_2: u32 = 65;
const WONT_JOIN_3: u32 = 66;
const GEE_THANKS: u32 = 67;
const CHANGED_MIND: u32 = 68;
const YOURE_FORGIVEN: u32 = 69;
const THANKS_FOR_FORGIVENESS: u32 = 70;
const HATE_YOU_FOREVER_SPACE: u32 = 71;
const INIT_ANGRY_HELLO_SPACE: u32 = 72;
const SUBSEQUENT_ANGRY_HELLO_SPACE: u32 = 73;
const INIT_NEUTRAL_HELLO_SPACE: u32 = 74;
const SUBSEQUENT_NEUTRAL_HELLO_SPACE: u32 = 75;
const INIT_FRIENDLY_HELLO_SPACE: u32 = 76;
const SUBSEQUENT_FRIENDLY_HELLO_SPACE: u32 = 77;
const INIT_ALLIED_HELLO_SPACE: u32 = 78;
const SUBSEQUENT_ALLIED_HELLO_SPACE: u32 = 79;
const GIVE_INFO_SPACE: u32 = 80;
const HERES_SOME_INFO: u32 = 81;
const WE_SORRY_SPACE: u32 = 82;
const APOLOGIZE_AT_HOMEWORLD: u32 = 83;
const WE_FIGHT_AGAIN_SPACE: u32 = 84;
const OK_FIGHT_AGAIN_SPACE: u32 = 85;
const BYE_ANGRY_SPACE: u32 = 86;
const GOODBYE_ANGRY_SPACE: u32 = 87;
const LOOK_WEIRD: u32 = 88;
const YOU_LOOK_WEIRD: u32 = 89;
const NO_LOOK_REALLY_WEIRD: u32 = 90;
const NO_YOU_LOOK_REALLY_WEIRD: u32 = 91;
const COME_IN_PEACE: u32 = 92;
const AGAINST_NATURE: u32 = 93;
const PREPARE_TO_DIE: u32 = 94;
const ALWAYS_PREPARED: u32 = 95;
const SINCE_FRIENDLY_GIVE_STUFF: u32 = 96;
const GIVE_ADVICE: u32 = 97;
const WHATS_UP_SPACE_1: u32 = 98;
const GENERAL_INFO_SPACE_1: u32 = 99;
const BYE_FRIENDLY_SPACE: u32 = 100;
const GOODBYE_FRIENDLY_SPACE: u32 = 101;
const LOOKING_FOR_A_FEW_GOOD_SQUIDS: u32 = 102;
const URQUAN_SLAVES: u32 = 103;
const WHY_SLAVES: u32 = 104;
const UMGAH_TRICK: u32 = 105;
const TELL_US_ABOUT_YOU: u32 = 106;
const ABOUT_US: u32 = 107;
const WHAT_YOU_REALLY_WANT: u32 = 108;
const WANT_THIS: u32 = 109;
const HOW_ABOUT_ALLIANCE: u32 = 110;
const SURE: u32 = 111;
const PART_IN_PEACE: u32 = 112;
const KEEP_IT_SECRET: u32 = 113;
const HEARD_YOURE_COWARDS: u32 = 114;
const DARN_TOOTIN: u32 = 115;
const WANNA_FIGHT: u32 = 116;
const YES_WE_DO: u32 = 117;
const SO_LETS_FIGHT: u32 = 118;
const OK_LETS_FIGHT: u32 = 119;
const SO_LETS_FIGHT_ALREADY: u32 = 120;
const DONT_REALLY_WANT_TO_FIGHT: u32 = 121;
const ATTACK_YOU_NOW: u32 = 122;
const YIPES: u32 = 123;
const WHATS_UP_SPACE_2: u32 = 124;
const GENERAL_INFO_SPACE_2: u32 = 125;
const GIVE_US_INFO_FROM_SPACE: u32 = 126;
const GET_INFO_FROM_SPATHIWA: u32 = 127;
const GIVE_US_RESOURCES_SPACE: u32 = 128;
const GET_RESOURCES_FROM_SPATHIWA: u32 = 129;
const WHAT_DO_FOR_FUN: u32 = 130;
const DO_THIS_FOR_FUN: u32 = 131;
const BYE_ALLY_SPACE: u32 = 132;
const GOODBYE_ALLY_SPACE: u32 = 133;
const OK_WE_FIGHT_AT_PLUTO: u32 = 134;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.spathi.graphics\0";
const RACE_FONT: &[u8] = b"comm.spathi.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.spathi.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.spathi.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.spathi.dialogue\0";

/// Spathi race dialogue implementation.
pub struct SpathiDialogue;

impl super::RaceDialogue for SpathiDialogue {
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
            bit_range("FOUND_PLUTO_SPATHI").is_some(),
            "missing game state key: FOUND_PLUTO_SPATHI"
        );
        assert!(
            bit_range("KNOW_SPATHI_PASSWORD").is_some(),
            "missing game state key: KNOW_SPATHI_PASSWORD"
        );
        assert!(
            bit_range("SPATHI_HOME_VISITS").is_some(),
            "missing game state key: SPATHI_HOME_VISITS"
        );
        assert!(
            bit_range("SPATHI_MANNER").is_some(),
            "missing game state key: SPATHI_MANNER"
        );
        assert!(
            bit_range("SPATHI_QUEST").is_some(),
            "missing game state key: SPATHI_QUEST"
        );
    }
}
