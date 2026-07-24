//! Utwig dialogue state machine — ported from C.
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
const NEUTRAL_SPACE_HELLO_1: u32 = 1;
const NEUTRAL_SPACE_HELLO_2: u32 = 2;
const HOSTILE_SPACE_HELLO_1: u32 = 3;
const HOSTILE_SPACE_HELLO_2: u32 = 4;
const BOMB_WORLD_HELLO_1: u32 = 5;
const BOMB_WORLD_HELLO_2: u32 = 6;
const HOSTILE_BOMB_HELLO_1: u32 = 7;
const HOSTILE_BOMB_HELLO_2: u32 = 8;
const NEUTRAL_HOMEWORLD_HELLO_1: u32 = 9;
const NEUTRAL_HOMEWORLD_HELLO_2: u32 = 10;
const NEUTRAL_HOMEWORLD_HELLO_3: u32 = 11;
const NEUTRAL_HOMEWORLD_HELLO_4: u32 = 12;
const HOSTILE_HOMEWORLD_HELLO_1: u32 = 13;
const HOSTILE_HOMEWORLD_HELLO_2: u32 = 14;
const WHY_YOU_HERE: u32 = 15;
const WE_GUARD_BOMB: u32 = 16;
const WHAT_ABOUT_BOMB: u32 = 17;
const ABOUT_BOMB: u32 = 18;
const GIVE_US_BOMB_OR_DIE: u32 = 19;
const GUARDS_WARN: u32 = 20;
const DEMAND_BOMB: u32 = 21;
const GUARDS_FIGHT: u32 = 22;
const MAY_WE_HAVE_BOMB: u32 = 23;
const NO_BOMB: u32 = 24;
const PLEASE: u32 = 25;
const SORRY_NO_BOMB: u32 = 26;
const WHATS_UP_BOMB: u32 = 27;
const GENERAL_INFO_BOMB_1: u32 = 28;
const GENERAL_INFO_BOMB_2: u32 = 29;
const BYE_BOMB: u32 = 30;
const GOODBYE_BOMB: u32 = 31;
const HEY_WAIT_GOT_ULTRON: u32 = 32;
const TAUNT_US_BUT_WE_LOOK: u32 = 33;
const TRICKED_US_1: u32 = 34;
const TRICKED_US_2: u32 = 35;
const WE_ARE_VINDICATOR0: u32 = 36;
const WE_ARE_VINDICATOR1: u32 = 37;
const WE_ARE_VINDICATOR2: u32 = 38;
const WOULD_BE_HAPPY_BUT: u32 = 39;
const WHY_SAD: u32 = 40;
const ULTRON_BROKE: u32 = 41;
const WHAT_ULTRON: u32 = 42;
const GLORIOUS_ULTRON: u32 = 43;
const DONT_BE_BABIES: u32 = 44;
const MOCK_OUR_PAIN: u32 = 45;
const REAL_SORRY_ABOUT_ULTRON: u32 = 46;
const APPRECIATE_SYMPATHY: u32 = 47;
const WHAT_ABOUT_YOU_1: u32 = 48;
const ABOUT_US_1: u32 = 49;
const WHAT_ABOUT_YOU_2: u32 = 50;
const ABOUT_US_2: u32 = 51;
const WHAT_ABOUT_YOU_3: u32 = 52;
const ABOUT_US_3: u32 = 53;
const WHAT_ABOUT_URQUAN_1: u32 = 54;
const ABOUT_URQUAN_1: u32 = 55;
const WHAT_ABOUT_URQUAN_2: u32 = 56;
const ABOUT_URQUAN_2: u32 = 57;
const GOT_ULTRON: u32 = 58;
const DONT_WANT_TO_LOOK: u32 = 59;
const SICK_TRICK_1: u32 = 60;
const SICK_TRICK_2: u32 = 61;
const BYE_NEUTRAL: u32 = 62;
const GOODBYE_NEUTRAL: u32 = 63;
const TOO_LATE: u32 = 64;
const NAME_1: u32 = 65;
const NAME_2: u32 = 66;
const NAME_3: u32 = 67;
const NAME_40: u32 = 68;
const NAME_41: u32 = 69;
const HAPPY_DAYS: u32 = 70;
const OK_ATTACK_KOHRAH: u32 = 71;
const WHATS_UP_AFTER_SPACE: u32 = 72;
const GENERAL_INFO_AFTER_SPACE_1: u32 = 73;
const GENERAL_INFO_AFTER_SPACE_2: u32 = 74;
const WHAT_NOW_AFTER_SPACE: u32 = 75;
const DO_THIS_AFTER_SPACE: u32 = 76;
const BYE_AFTER_SPACE: u32 = 77;
const GOODBYE_AFTER_SPACE: u32 = 78;
const WHATS_UP_BEFORE_SPACE: u32 = 79;
const GENERAL_INFO_BEFORE_SPACE_1: u32 = 80;
const GENERAL_INFO_BEFORE_SPACE_2: u32 = 81;
const WHAT_NOW_BEFORE_SPACE: u32 = 82;
const DO_THIS_BEFORE_SPACE: u32 = 83;
const BYE_BEFORE_SPACE: u32 = 84;
const GOODBYE_BEFORE_SPACE: u32 = 85;
const HOW_WENT_WAR: u32 = 86;
const ABOUT_BATTLE: u32 = 87;
const HOW_GOES_WAR: u32 = 88;
const BATTLE_HAPPENS_1: u32 = 89;
const BATTLE_HAPPENS_2: u32 = 90;
const FLEET_ON_WAY: u32 = 91;
const LEARN_NEW_INFO: u32 = 92;
const NO_NEW_INFO: u32 = 93;
const SAMATRA: u32 = 94;
const WHAT_NOW_HOMEWORLD: u32 = 95;
const HOPE_KILL_EACH_OTHER: u32 = 96;
const HOW_IS_ULTRON: u32 = 97;
const ULTRON_IS_GREAT: u32 = 98;
const BYE_ALLIED_HOMEWORLD: u32 = 99;
const GOODBYE_ALLIED_HOMEWORLD: u32 = 100;
const ALLIED_HOMEWORLD_HELLO_1: u32 = 101;
const ALLIED_HOMEWORLD_HELLO_2: u32 = 102;
const ALLIED_HOMEWORLD_HELLO_3: u32 = 103;
const ALLIED_HOMEWORLD_HELLO_4: u32 = 104;
const HELLO_BEFORE_KOHRAH_SPACE_1: u32 = 105;
const HELLO_BEFORE_KOHRAH_SPACE_2: u32 = 106;
const HELLO_DURING_KOHRAH_SPACE_1: u32 = 107;
const HELLO_DURING_KOHRAH_SPACE_2: u32 = 108;
const HELLO_AFTER_KOHRAH_SPACE_1: u32 = 109;
const HELLO_AFTER_KOHRAH_SPACE_2: u32 = 110;
const UP_TO_YOU: u32 = 111;
const CAN_YOU_HELP: u32 = 112;
const HOW_HELP: u32 = 113;
const DONT_NEED: u32 = 114;
const HAVE_4_SHIPS: u32 = 115;
const NO_ULTRON_AT_BOMB: u32 = 116;
const OUT_TAKES: u32 = 117;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.utwig.graphics\0";
const RACE_FONT: &[u8] = b"comm.utwig.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.utwig.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.utwig.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.utwig.dialogue\0";

/// Utwig race dialogue implementation.
pub struct UtwigDialogue;

impl super::RaceDialogue for UtwigDialogue {
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
            bit_range("BOMB_INFO").is_some(),
            "missing game state key: BOMB_INFO"
        );
        assert!(
            bit_range("BOMB_STACK1").is_some(),
            "missing game state key: BOMB_STACK1"
        );
        assert!(
            bit_range("BOMB_STACK2").is_some(),
            "missing game state key: BOMB_STACK2"
        );
        assert!(
            bit_range("BOMB_VISITS").is_some(),
            "missing game state key: BOMB_VISITS"
        );
        assert!(
            bit_range("GLOBAL_FLAGS_AND_DATA").is_some(),
            "missing game state key: GLOBAL_FLAGS_AND_DATA"
        );
    }
}
