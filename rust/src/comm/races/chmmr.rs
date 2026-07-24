//! Chmmr dialogue state machine — ported from C.
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
const WHY_YOU_HERE_1: u32 = 1;
const WHY_YOU_HERE_2: u32 = 2;
const WHY_YOU_HERE_3: u32 = 3;
const WHY_YOU_HERE_4: u32 = 4;
const FIND_OUT_WHATS_UP: u32 = 5;
const HYBRID_PROCESS: u32 = 6;
const NEED_HELP: u32 = 7;
const CANT_HELP: u32 = 8;
const WHY_NO_HELP: u32 = 9;
const LONG_TIME: u32 = 10;
const WHAT_IF_MORE_ENERGY: u32 = 11;
const DANGER_TO_US: u32 = 12;
const NEED_ADVICE: u32 = 13;
const WHAT_ADVICE: u32 = 14;
const HOW_DEFEAT_URQUAN: u32 = 15;
const DEFEAT_LIKE_SO: u32 = 16;
const WHAT_ABOUT_TPET: u32 = 17;
const SCARY_BUT_USEFUL: u32 = 18;
const WHAT_ABOUT_BOMB: u32 = 19;
const ABOUT_BOMB: u32 = 20;
const WHAT_ABOUT_SUN_DEVICE: u32 = 21;
const ABOUT_SUN_DEVICE: u32 = 22;
const WHAT_ABOUT_SAMATRA: u32 = 23;
const ABOUT_SAMATRA: u32 = 24;
const ENOUGH_ADVICE: u32 = 25;
const OK_ENOUGH_ADVICE: u32 = 26;
const BYE_SHIELDED: u32 = 27;
const GOODBYE_SHIELDED: u32 = 28;
const WE_ARE_FREE: u32 = 29;
const WHO_ARE_YOU: u32 = 30;
const I_AM_CAPTAIN0: u32 = 31;
const I_AM_CAPTAIN1: u32 = 32;
const I_AM_CAPTAIN2: u32 = 33;
const I_AM_SAVIOR: u32 = 34;
const I_AM_SILLY: u32 = 35;
const WHY_HAVE_YOU_FREED_US: u32 = 36;
const SERIOUS_1: u32 = 37;
const SERIOUS_2: u32 = 38;
const SILLY: u32 = 39;
const WILL_HELP_ANALYZE_LOGS: u32 = 40;
const YOU_KNOW_SAMATRA: u32 = 41;
const DONT_KNOW_ABOUT_SAMATRA: u32 = 42;
const NEED_DISTRACTION: u32 = 43;
const HAVE_TALKING_PET: u32 = 44;
const NEED_WEAPON: u32 = 45;
const HAVE_BOMB: u32 = 46;
const RETURN_WHEN_READY: u32 = 47;
const YOU_ARE_READY: u32 = 48;
const FURTHER_ASSISTANCE: u32 = 49;
const NO_FURTHER_ASSISTANCE: u32 = 50;
const TECH_HELP: u32 = 51;
const USE_OUR_SHIPS_BEFORE: u32 = 52;
const WHERE_WEAPON: u32 = 53;
const PRECURSOR_WEAPON: u32 = 54;
const WHERE_DISTRACTION: u32 = 55;
const PSYCHIC_WEAPONRY: u32 = 56;
const WHAT_NOW: u32 = 57;
const WE_WILL_IMPROVE_BOMB: u32 = 58;
const MODIFY_VESSEL: u32 = 59;
const WONT_HURT_MY_SHIP: u32 = 60;
const WILL_DESTROY_IT: u32 = 61;
const BUMMER_ABOUT_MY_SHIP: u32 = 62;
const DEAD_SILENCE: u32 = 63;
const OTHER_ASSISTANCE: u32 = 64;
const USE_OUR_SHIPS_AFTER: u32 = 65;
const PROCEED: u32 = 66;
const TAKE_2_WEEKS: u32 = 67;
const HELLO_AFTER_BOMB_1: u32 = 68;
const HELLO_AFTER_BOMB_2: u32 = 69;
const WHATS_UP_AFTER_BOMB: u32 = 70;
const GENERAL_INFO_AFTER_BOMB_1: u32 = 71;
const GENERAL_INFO_AFTER_BOMB_2: u32 = 72;
const WHAT_DO_AFTER_BOMB: u32 = 73;
const DO_AFTER_BOMB: u32 = 74;
const BYE_AFTER_BOMB: u32 = 75;
const GOODBYE_AFTER_BOMB: u32 = 76;
const BYE: u32 = 77;
const GOODBYE: u32 = 78;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.chmmr.graphics\0";
const RACE_FONT: &[u8] = b"comm.chmmr.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.chmmr.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.chmmr.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.chmmr.dialogue\0";

/// Chmmr race dialogue implementation.
pub struct ChmmrDialogue;

impl super::RaceDialogue for ChmmrDialogue {
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
            bit_range("AWARE_OF_SAMATRA").is_some(),
            "missing game state key: AWARE_OF_SAMATRA"
        );
        assert!(
            bit_range("CHMMR_BOMB_STATE").is_some(),
            "missing game state key: CHMMR_BOMB_STATE"
        );
        assert!(
            bit_range("CHMMR_EMERGING").is_some(),
            "missing game state key: CHMMR_EMERGING"
        );
        assert!(
            bit_range("CHMMR_HOME_VISITS").is_some(),
            "missing game state key: CHMMR_HOME_VISITS"
        );
        assert!(
            bit_range("CHMMR_STACK").is_some(),
            "missing game state key: CHMMR_STACK"
        );
    }
}
