//! Blackur dialogue state machine — ported from C.
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
const SENSE_EVIL: u32 = 1;
const HELLO_AND_DIE_1: u32 = 2;
const HELLO_AND_DIE_2: u32 = 3;
const HELLO_AND_DIE_3: u32 = 4;
const HELLO_AND_DIE_4: u32 = 5;
const HELLO_AND_DIE_5: u32 = 6;
const HELLO_AND_DIE_6: u32 = 7;
const HELLO_AND_DIE_7: u32 = 8;
const HELLO_AND_DIE_8: u32 = 9;
const HELLO_SAMATRA: u32 = 10;
const WE_KILL_ALL_1: u32 = 11;
const WE_KILL_ALL_2: u32 = 12;
const WE_KILL_ALL_3: u32 = 13;
const WE_KILL_ALL_4: u32 = 14;
const WHY_KILL_ALL_1: u32 = 15;
const WHY_KILL_ALL_2: u32 = 16;
const WHY_KILL_ALL_3: u32 = 17;
const WHY_KILL_ALL_4: u32 = 18;
const KILL_BECAUSE_1: u32 = 19;
const KILL_BECAUSE_2: u32 = 20;
const KILL_BECAUSE_3: u32 = 21;
const KILL_BECAUSE_4: u32 = 22;
const PLEASE_DONT_KILL_1: u32 = 23;
const WILL_KILL_1: u32 = 24;
const PLEASE_DONT_KILL_2: u32 = 25;
const WILL_KILL_2: u32 = 26;
const PLEASE_DONT_KILL_3: u32 = 27;
const WILL_KILL_3: u32 = 28;
const PLEASE_DONT_KILL_4: u32 = 29;
const WILL_KILL_4: u32 = 30;
const BYE_FRENZY_1: u32 = 31;
const BYE_FRENZY_2: u32 = 32;
const BYE_FRENZY_3: u32 = 33;
const BYE_FRENZY_4: u32 = 34;
const GOODBYE_AND_DIE_FRENZY_1: u32 = 35;
const GOODBYE_AND_DIE_FRENZY_2: u32 = 36;
const GOODBYE_AND_DIE_FRENZY_3: u32 = 37;
const GOODBYE_AND_DIE_FRENZY_4: u32 = 38;
const THREAT_1: u32 = 39;
const RESISTANCE_IS_USELESS_1: u32 = 40;
const THREAT_2: u32 = 41;
const RESISTANCE_IS_USELESS_2: u32 = 42;
const THREAT_3: u32 = 43;
const RESISTANCE_IS_USELESS_3: u32 = 44;
const THREAT_4: u32 = 45;
const RESISTANCE_IS_USELESS_4: u32 = 46;
const KEY_PHRASE: u32 = 47;
const RESPONSE_TO_KEY_PHRASE: u32 = 48;
const WHY_DO_YOU_DESTROY: u32 = 49;
const WE_WERE_SLAVES: u32 = 50;
const RELATIONSHIP_WITH_URQUAN: u32 = 51;
const WE_ARE_URQUAN_TOO: u32 = 52;
const WHAT_ABOUT_CULTURE: u32 = 53;
const BONE_GARDENS: u32 = 54;
const HOW_LEAVE_ME_ALONE: u32 = 55;
const YOU_DIE: u32 = 56;
const GUESS_THATS_ALL: u32 = 57;
const THEN_DIE: u32 = 58;
const WHAT_ARE_YOU_HOVERING_OVER: u32 = 59;
const BONE_PILE: u32 = 60;
const YOU_SURE_ARE_CREEPY: u32 = 61;
const YES_CREEPY: u32 = 62;
const STOP_THAT_GROSS_BLINKING: u32 = 63;
const DIE_HUMAN: u32 = 64;
const PLEAD_1: u32 = 65;
const PLEADING_IS_USELESS_1: u32 = 66;
const PLEAD_2: u32 = 67;
const PLEADING_IS_USELESS_2: u32 = 68;
const PLEAD_3: u32 = 69;
const PLEADING_IS_USELESS_3: u32 = 70;
const PLEAD_4: u32 = 71;
const PLEADING_IS_USELESS_4: u32 = 72;
const BYE: u32 = 73;
const GOODBYE_AND_DIE: u32 = 74;
const GAME_OVER_DUDE: u32 = 75;
const OUT_TAKES: u32 = 76;

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

const RACE_PMAP_ANIM: &[u8] = b"blackur\0";
const RACE_FONT: &[u8] = b"blackurfont\0";
const RACE_COLOR_MAP: &[u8] = b"blackurcolr\0";
const RACE_MUSIC: &[u8] = b"blackurmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.blackur.dialogue\0";

/// Blackur race dialogue implementation.
pub struct BlackurDialogue;

impl super::RaceDialogue for BlackurDialogue {
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
            bit_range("KNOW_KOHR_AH_STORY").is_some(),
            "missing game state key: KNOW_KOHR_AH_STORY"
        );
        assert!(
            bit_range("KOHR_AH_BYES").is_some(),
            "missing game state key: KOHR_AH_BYES"
        );
        assert!(
            bit_range("KOHR_AH_FRENZY").is_some(),
            "missing game state key: KOHR_AH_FRENZY"
        );
        assert!(
            bit_range("KOHR_AH_INFO").is_some(),
            "missing game state key: KOHR_AH_INFO"
        );
    }
}
