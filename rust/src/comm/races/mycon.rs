//! Mycon dialogue state machine — ported from C.
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
const TELL_US_ABOUT_WORLD: u32 = 1;
const BYE_AND_DIE_HOMEWORLD: u32 = 2;
const RAMBLE_1: u32 = 3;
const RAMBLE_2: u32 = 4;
const RAMBLE_3: u32 = 5;
const RAMBLE_4: u32 = 6;
const RAMBLE_5: u32 = 7;
const RAMBLE_6: u32 = 8;
const RAMBLE_7: u32 = 9;
const RAMBLE_8: u32 = 10;
const RAMBLE_9: u32 = 11;
const RAMBLE_10: u32 = 12;
const RAMBLE_11: u32 = 13;
const RAMBLE_12: u32 = 14;
const RAMBLE_13: u32 = 15;
const RAMBLE_14: u32 = 16;
const RAMBLE_15: u32 = 17;
const RAMBLE_16: u32 = 18;
const RAMBLE_17: u32 = 19;
const RAMBLE_18: u32 = 20;
const RAMBLE_19: u32 = 21;
const RAMBLE_20: u32 = 22;
const RAMBLE_21: u32 = 23;
const RAMBLE_22: u32 = 24;
const RAMBLE_23: u32 = 25;
const RAMBLE_24: u32 = 26;
const RAMBLE_25: u32 = 27;
const RAMBLE_26: u32 = 28;
const RAMBLE_27: u32 = 29;
const RAMBLE_28: u32 = 30;
const RAMBLE_29: u32 = 31;
const RAMBLE_30: u32 = 32;
const RAMBLE_31: u32 = 33;
const RAMBLE_32: u32 = 34;
const QUESTION_1: u32 = 35;
const QUESTION_2: u32 = 36;
const QUESTION_3: u32 = 37;
const QUESTION_4: u32 = 38;
const QUESTION_5: u32 = 39;
const QUESTION_6: u32 = 40;
const QUESTION_7: u32 = 41;
const QUESTION_8: u32 = 42;
const QUESTION_9: u32 = 43;
const QUESTION_10: u32 = 44;
const QUESTION_11: u32 = 45;
const QUESTION_12: u32 = 46;
const QUESTION_13: u32 = 47;
const QUESTION_14: u32 = 48;
const QUESTION_15: u32 = 49;
const QUESTION_16: u32 = 50;
const BYE_SPACE: u32 = 51;
const BYE_AND_DIE_SPACE: u32 = 52;
const GONNA_DIE: u32 = 53;
const INSULT_1: u32 = 54;
const INSULT_2: u32 = 55;
const INSULT_3: u32 = 56;
const INSULT_4: u32 = 57;
const INSULT_5: u32 = 58;
const INSULT_6: u32 = 59;
const INSULT_7: u32 = 60;
const INSULT_8: u32 = 61;
const COME_IN_PEACE: u32 = 62;
const HELLO_HOMEWORLD_1: u32 = 63;
const HELLO_HOMEWORLD_2: u32 = 64;
const HELLO_HOMEWORLD_3: u32 = 65;
const HELLO_HOMEWORLD_4: u32 = 66;
const HELLO_HOMEWORLD_5: u32 = 67;
const HELLO_HOMEWORLD_6: u32 = 68;
const HELLO_HOMEWORLD_7: u32 = 69;
const HELLO_HOMEWORLD_8: u32 = 70;
const HELLO_SPACE_1: u32 = 71;
const HELLO_SPACE_2: u32 = 72;
const HELLO_SPACE_3: u32 = 73;
const HELLO_SPACE_4: u32 = 74;
const HELLO_SPACE_5: u32 = 75;
const HELLO_SPACE_6: u32 = 76;
const HELLO_SPACE_7: u32 = 77;
const HELLO_SPACE_8: u32 = 78;
const LETS_BE_FRIENDS: u32 = 79;
const CAME_TO_HOMEWORLD: u32 = 80;
const SUBMIT_TO_US: u32 = 81;
const BYE_SUN_DEVICE: u32 = 82;
const GOODBYE_SUN_DEVICE: u32 = 83;
const RESPONSE_1: u32 = 84;
const RESPONSE_2: u32 = 85;
const RESPONSE_3: u32 = 86;
const CLUE_1: u32 = 87;
const CLUE_2: u32 = 88;
const CLUE_3: u32 = 89;
const WHAT_ABOUT_SHATTERED: u32 = 90;
const ABOUT_SHATTERED: u32 = 91;
const HELLO_SUN_DEVICE_WORLD_1: u32 = 92;
const HELLO_SUN_DEVICE_WORLD_2: u32 = 93;
const WHATS_UP_SUN_DEVICE: u32 = 94;
const GENERAL_INFO_SUN_DEVICE: u32 = 95;
const LIKE_TO_LAND: u32 = 96;
const NEVER_LET_LAND: u32 = 97;
const BYE_HOMEWORLD: u32 = 98;
const I_HAVE_A_CUNNING_PLAN: u32 = 99;
const DIE_LIAR: u32 = 100;
const HOW_GOES_IMPLANTING: u32 = 101;
const UNFORSEEN_DELAYS: u32 = 102;
const DIE_THIEF: u32 = 103;
const DIE_THIEF_AGAIN: u32 = 104;
const GOODBYE_AND_DIE: u32 = 105;
const AMBUSH_TAIL: u32 = 106;
const RAMBLE_TAIL: u32 = 107;
const WE_GO_TO_IMPLANT: u32 = 108;
const WONT_FALL_FOR_TRICK: u32 = 109;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.mycon.graphics\0";
const RACE_FONT: &[u8] = b"comm.mycon.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.mycon.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.mycon.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.mycon.dialogue\0";

/// Mycon race dialogue implementation.
pub struct MyconDialogue;

impl super::RaceDialogue for MyconDialogue {
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
            bit_range("KNOW_ABOUT_SHATTERED").is_some(),
            "missing game state key: KNOW_ABOUT_SHATTERED"
        );
        assert!(
            bit_range("MYCON_AMBUSH").is_some(),
            "missing game state key: MYCON_AMBUSH"
        );
        assert!(
            bit_range("MYCON_FELL_FOR_AMBUSH").is_some(),
            "missing game state key: MYCON_FELL_FOR_AMBUSH"
        );
        assert!(
            bit_range("MYCON_HOME_VISITS").is_some(),
            "missing game state key: MYCON_HOME_VISITS"
        );
    }
}
