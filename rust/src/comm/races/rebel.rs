//! Rebel dialogue state machine — ported from C.
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
const REBEL_HELLO_1: u32 = 1;
const REBEL_HELLO_2: u32 = 2;
const REBEL_HELLO_3: u32 = 3;
const REBEL_HELLO_4: u32 = 4;
const HOW_GOES_REVOLUTION: u32 = 5;
const REBEL_REVOLUTION_1: u32 = 6;
const REBEL_REVOLUTION_2: u32 = 7;
const REBEL_REVOLUTION_3: u32 = 8;
const REBEL_REVOLUTION_4: u32 = 9;
const ANY_SHIPS: u32 = 10;
const NO_ROOM: u32 = 11;
const HAVE_ALL_SHIPS: u32 = 12;
const HAVE_FEW_SHIPS: u32 = 13;
const NO_SHIPS_YET: u32 = 14;
const GIVE_INFO_REBELS: u32 = 15;
const WHAT_INFO: u32 = 16;
const WHAT_ABOUT_ROYALTY: u32 = 17;
const ABOUT_ROYALTY: u32 = 18;
const WHAT_ABOUT_WAR: u32 = 19;
const ABOUT_WAR: u32 = 20;
const WHAT_ABOUT_URQUAN: u32 = 21;
const ABOUT_URQUAN: u32 = 22;
const WHAT_ABOUT_VUX: u32 = 23;
const ABOUT_VUX: u32 = 24;
const WHAT_ABOUT_CLUE: u32 = 25;
const ABOUT_CLUE: u32 = 26;
const ENOUGH_INFO: u32 = 27;
const OK_ENOUGH_INFO: u32 = 28;
const BYE_REBEL: u32 = 29;
const GOODBYE_REBEL: u32 = 30;
const YEHAT_CAVALRY: u32 = 31;
const WHAT_ABOUT_PKUNK_REBEL: u32 = 32;
const PKUNK_ABSORBED_REBEL: u32 = 33;
const HATE_PKUNK_REBEL: u32 = 34;

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

const RACE_PMAP_ANIM: &[u8] = b"rebel\0";
const RACE_FONT: &[u8] = b"rebelfont\0";
const RACE_COLOR_MAP: &[u8] = b"rebelcolr\0";
const RACE_MUSIC: &[u8] = b"rebelmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.rebel.dialogue\0";

/// Rebel race dialogue implementation.
pub struct RebelDialogue;

impl super::RaceDialogue for RebelDialogue {
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
            bit_range("PKUNK_HOME_VISITS").is_some(),
            "missing game state key: PKUNK_HOME_VISITS"
        );
        assert!(
            bit_range("PKUNK_VISITS").is_some(),
            "missing game state key: PKUNK_VISITS"
        );
        assert!(
            bit_range("YEHAT_ABSORBED_PKUNK").is_some(),
            "missing game state key: YEHAT_ABSORBED_PKUNK"
        );
        assert!(
            bit_range("YEHAT_REBEL_INFO").is_some(),
            "missing game state key: YEHAT_REBEL_INFO"
        );
        assert!(
            bit_range("YEHAT_REBEL_TOLD_PKUNK").is_some(),
            "missing game state key: YEHAT_REBEL_TOLD_PKUNK"
        );
    }
}
