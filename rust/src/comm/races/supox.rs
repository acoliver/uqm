//! Supox dialogue state machine — ported from C.
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
const NEUTRAL_HOMEWORLD_HELLO_1: u32 = 3;
const NEUTRAL_HOMEWORLD_HELLO_2: u32 = 4;
const HOSTILE_SPACE_HELLO_1: u32 = 5;
const HOSTILE_SPACE_HELLO_2: u32 = 6;
const ALLIED_HOMEWORLD_HELLO_1: u32 = 7;
const ALLIED_HOMEWORLD_HELLO_2: u32 = 8;
const ALLIED_HOMEWORLD_HELLO_3: u32 = 9;
const ALLIED_HOMEWORLD_HELLO_4: u32 = 10;
const I_AM0: u32 = 11;
const I_AM1: u32 = 12;
const WE_ARE_SUPOX: u32 = 13;
const MY_SHIP0: u32 = 14;
const MY_SHIP1: u32 = 15;
const OUR_SHIP: u32 = 16;
const FROM_ALLIANCE0: u32 = 17;
const FROM_ALLIANCE1: u32 = 18;
const FROM_SUPOX: u32 = 19;
const ARE_YOU_COPYING: u32 = 20;
const YEAH_SORRY: u32 = 21;
const WHY_COPY: u32 = 22;
const SYMBIOTS: u32 = 23;
const TELL_US_OF_YOUR_SPECIES: u32 = 24;
const OUR_SPECIES: u32 = 25;
const PLANTS_ARENT_INTELLIGENT: u32 = 26;
const PROVES_WERE_SPECIAL: u32 = 27;
const ANYONE_AROUND_HERE: u32 = 28;
const UTWIG_NEARBY: u32 = 29;
const WHAT_RELATION_TO_UTWIG: u32 = 30;
const UTWIG_ALLIES: u32 = 31;
const WHATS_WRONG_WITH_UTWIG: u32 = 32;
const BROKE_ULTRON: u32 = 33;
const WHATS_ULTRON: u32 = 34;
const TAKE_ULTRON: u32 = 35;
const WHAT_DO_I_DO_NOW: u32 = 36;
const FIX_IT: u32 = 37;
const THANKS_NOW_WE_EAT_YOU: u32 = 38;
const HIDEOUS_MONSTERS: u32 = 39;
const GOT_FIXED_ULTRON: u32 = 40;
const GOOD_GIVE_TO_UTWIG: u32 = 41;
const LOOK_I_REPAIRED_LOTS: u32 = 42;
const ALMOST_THERE: u32 = 43;
const LOOK_I_SLIGHTLY_REPAIRED: u32 = 44;
const GREAT_DO_MORE: u32 = 45;
const WHERE_GET_REPAIRS: u32 = 46;
const ANCIENT_RHYME: u32 = 47;
const BYE_NEUTRAL: u32 = 48;
const GOODBYE_NEUTRAL: u32 = 49;
const ABOUT_BATTLE: u32 = 50;
const HELLO_BEFORE_KOHRAH_SPACE_1: u32 = 51;
const HELLO_BEFORE_KOHRAH_SPACE_2: u32 = 52;
const HELLO_DURING_KOHRAH_SPACE_1: u32 = 53;
const HELLO_DURING_KOHRAH_SPACE_2: u32 = 54;
const HELLO_AFTER_KOHRAH_SPACE_1: u32 = 55;
const HELLO_AFTER_KOHRAH_SPACE_2: u32 = 56;
const WHATS_UP_AFTER_SPACE: u32 = 57;
const GENERAL_INFO_AFTER_SPACE_1: u32 = 58;
const GENERAL_INFO_AFTER_SPACE_2: u32 = 59;
const WHAT_NOW_AFTER_SPACE: u32 = 60;
const DO_THIS_AFTER_SPACE: u32 = 61;
const BYE_AFTER_SPACE: u32 = 62;
const GOODBYE_AFTER_SPACE: u32 = 63;
const WHATS_UP_BEFORE_SPACE: u32 = 64;
const GENERAL_INFO_BEFORE_SPACE_1: u32 = 65;
const GENERAL_INFO_BEFORE_SPACE_2: u32 = 66;
const WHAT_NOW_BEFORE_SPACE: u32 = 67;
const DO_THIS_BEFORE_SPACE: u32 = 68;
const BYE_BEFORE_SPACE: u32 = 69;
const GOODBYE_BEFORE_SPACE: u32 = 70;
const HOW_WENT_WAR: u32 = 71;
const HOW_GOES_WAR: u32 = 72;
const BATTLE_HAPPENS_1: u32 = 73;
const BATTLE_HAPPENS_2: u32 = 74;
const FLEET_ON_WAY: u32 = 75;
const LEARN_NEW_INFO: u32 = 76;
const NO_NEW_INFO: u32 = 77;
const SAMATRA: u32 = 78;
const WHAT_NOW_HOMEWORLD: u32 = 79;
const HOPE_KILL_EACH_OTHER: u32 = 80;
const UP_TO_YOU: u32 = 81;
const CAN_YOU_HELP: u32 = 82;
const HOW_HELP: u32 = 83;
const DONT_NEED: u32 = 84;
const HAVE_4_SHIPS: u32 = 85;
const GIVE_INFO: u32 = 86;
const GOOD_HINTS: u32 = 87;
const HOW_IS_ULTRON: u32 = 88;
const ULTRON_IS_GREAT: u32 = 89;
const BYE_ALLIED_HOMEWORLD: u32 = 90;
const GOODBYE_ALLIED_HOMEWORLD: u32 = 91;
const NAME_1: u32 = 92;
const NAME_2: u32 = 93;
const NAME_3: u32 = 94;
const NAME_40: u32 = 95;
const NAME_41: u32 = 96;
const OUT_TAKES: u32 = 97;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.supox.graphics\0";
const RACE_FONT: &[u8] = b"comm.supox.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.supox.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.supox.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.supox.dialogue\0";

/// Supox race dialogue implementation.
pub struct SupoxDialogue;

impl super::RaceDialogue for SupoxDialogue {
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
            bit_range("BOMB_VISITS").is_some(),
            "missing game state key: BOMB_VISITS"
        );
        assert!(
            bit_range("GLOBAL_FLAGS_AND_DATA").is_some(),
            "missing game state key: GLOBAL_FLAGS_AND_DATA"
        );
        assert!(
            bit_range("SUPOX_HOME_VISITS").is_some(),
            "missing game state key: SUPOX_HOME_VISITS"
        );
        assert!(
            bit_range("SUPOX_HOSTILE").is_some(),
            "missing game state key: SUPOX_HOSTILE"
        );
        assert!(
            bit_range("SUPOX_INFO").is_some(),
            "missing game state key: SUPOX_INFO"
        );
    }
}
