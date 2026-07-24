//! Slyland dialogue state machine — ported from C.
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
const WE_COME_IN_PEACE_1: u32 = 1;
const WE_COME_IN_PEACE_2: u32 = 2;
const WE_COME_IN_PEACE_3: u32 = 3;
const WE_COME_IN_PEACE_4: u32 = 4;
const WE_COME_IN_PEACE_5: u32 = 5;
const WE_COME_IN_PEACE_6: u32 = 6;
const WE_COME_IN_PEACE_7: u32 = 7;
const WE_COME_IN_PEACE_8: u32 = 8;
const THREAT_1: u32 = 9;
const THREAT_2: u32 = 10;
const THREAT_3: u32 = 11;
const THREAT_4: u32 = 12;
const PROGRAMMED_TO_DEFEND_1: u32 = 13;
const PROGRAMMED_TO_DEFEND_2: u32 = 14;
const PROGRAMMED_TO_DEFEND_3: u32 = 15;
const PROGRAMMED_TO_DEFEND_4: u32 = 16;
const SOMETHING_WRONG_1: u32 = 17;
const SOMETHING_WRONG_2: u32 = 18;
const SOMETHING_WRONG_3: u32 = 19;
const SOMETHING_WRONG_4: u32 = 20;
const NOMINAL_FUNCTION_1: u32 = 21;
const NOMINAL_FUNCTION_2: u32 = 22;
const NOMINAL_FUNCTION_3: u32 = 23;
const NOMINAL_FUNCTION_4: u32 = 24;
const WE_ARE_US_1: u32 = 25;
const WE_ARE_US_2: u32 = 26;
const WE_ARE_US_3: u32 = 27;
const WE_ARE_US_4: u32 = 28;
const THIS_IS_PROBE_1: u32 = 29;
const THIS_IS_PROBE_2: u32 = 30;
const THIS_IS_PROBE_3: u32 = 31;
const THIS_IS_PROBE_40: u32 = 32;
const THIS_IS_PROBE_41: u32 = 33;
const THIS_IS_PROBE_42: u32 = 34;
const WHY_ATTACK_1: u32 = 35;
const WHY_ATTACK_2: u32 = 36;
const WHY_ATTACK_3: u32 = 37;
const WHY_ATTACK_4: u32 = 38;
const PEACEFUL_MISSION_1: u32 = 39;
const PEACEFUL_MISSION_2: u32 = 40;
const PEACEFUL_MISSION_3: u32 = 41;
const PEACEFUL_MISSION_4: u32 = 42;
const BYE_1: u32 = 43;
const BYE_2: u32 = 44;
const BYE_3: u32 = 45;
const BYE_4: u32 = 46;
const GOODBYE_1: u32 = 47;
const GOODBYE_2: u32 = 48;
const GOODBYE_3: u32 = 49;
const GOODBYE_4: u32 = 50;
const HOSTILE: u32 = 51;
const DESTRUCT_SEQUENCE: u32 = 52;
const DESTRUCT_CODE: u32 = 53;
const COORD_PLUS: u32 = 54;
const COORD_MINUS: u32 = 55;
const COORD_POINT: u32 = 56;
const ENUMERATE_HUNDRED: u32 = 57;
const ENUMERATE_THOUSAND: u32 = 58;
const ENUMERATE_ZERO: u32 = 59;
const ENUMERATE_ONE: u32 = 60;
const ENUMERATE_TWO: u32 = 61;
const ENUMERATE_THREE: u32 = 62;
const ENUMERATE_FOUR: u32 = 63;
const ENUMERATE_FIVE: u32 = 64;
const ENUMERATE_SIX: u32 = 65;
const ENUMERATE_SEVEN: u32 = 66;
const ENUMERATE_EIGHT: u32 = 67;
const ENUMERATE_NINE: u32 = 68;
const ENUMERATE_TEN: u32 = 69;
const ENUMERATE_ELEVEN: u32 = 70;
const ENUMERATE_TWELVE: u32 = 71;
const ENUMERATE_THIRTEEN: u32 = 72;
const ENUMERATE_FOURTEEN: u32 = 73;
const ENUMERATE_FIFTEEN: u32 = 74;
const ENUMERATE_SIXTEEN: u32 = 75;
const ENUMERATE_SEVENTEEN: u32 = 76;
const ENUMERATE_EIGHTEEN: u32 = 77;
const ENUMERATE_NINETEEN: u32 = 78;
const ENUMERATE_TWENTY: u32 = 79;
const ENUMERATE_THIRTY: u32 = 80;
const ENUMERATE_FOURTY: u32 = 81;
const ENUMERATE_FIFTY: u32 = 82;
const ENUMERATE_SIXTY: u32 = 83;
const ENUMERATE_SEVENTY: u32 = 84;
const ENUMERATE_EIGHTY: u32 = 85;
const ENUMERATE_NINETY: u32 = 86;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.probe.graphics\0";
const RACE_FONT: &[u8] = b"comm.probe.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.probe.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.probe.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.probe.dialogue\0";

/// Slyland race dialogue implementation.
pub struct SlylandDialogue;

impl super::RaceDialogue for SlylandDialogue {
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
            bit_range("DESTRUCT_CODE_ON_SHIP").is_some(),
            "missing game state key: DESTRUCT_CODE_ON_SHIP"
        );
        assert!(
            bit_range("PROBE_EXHIBITED_BUG").is_some(),
            "missing game state key: PROBE_EXHIBITED_BUG"
        );
        assert!(
            bit_range("SLYLANDRO_PROBE_EXIT").is_some(),
            "missing game state key: SLYLANDRO_PROBE_EXIT"
        );
        assert!(
            bit_range("SLYLANDRO_PROBE_ID").is_some(),
            "missing game state key: SLYLANDRO_PROBE_ID"
        );
        assert!(
            bit_range("SLYLANDRO_PROBE_INFO").is_some(),
            "missing game state key: SLYLANDRO_PROBE_INFO"
        );
    }
}
