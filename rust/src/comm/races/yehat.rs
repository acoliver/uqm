//! Yehat dialogue state machine — ported from C.
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
const HOMEWORLD_HELLO_1: u32 = 1;
const HOMEWORLD_HELLO_2: u32 = 2;
const WHATS_UP_HOMEWORLD: u32 = 3;
const GENERAL_INFO_HOMEWORLD_1: u32 = 4;
const GENERAL_INFO_HOMEWORLD_2: u32 = 5;
const I_DEMAND_YOU_ALLY_HOMEWORLD0: u32 = 6;
const I_DEMAND_YOU_ALLY_HOMEWORLD1: u32 = 7;
const I_DEMAND_YOU_ALLY_HOMEWORLD2: u32 = 8;
const I_DEMAND_YOU_ALLY_HOMEWORLD3: u32 = 9;
const ENEMY_MUST_DIE: u32 = 10;
const AT_LEAST_HELP_US_HOMEWORLD: u32 = 11;
const NO_HELP_ENEMY: u32 = 12;
const GIVE_INFO: u32 = 13;
const NO_INFO_FOR_ENEMY: u32 = 14;
const WHAT_ABOUT_PKUNK_ROYALIST: u32 = 15;
const PKUNK_ABSORBED_ROYALIST: u32 = 16;
const HATE_PKUNK_ROYALIST: u32 = 17;
const BYE_HOMEWORLD: u32 = 18;
const GOODBYE_AND_DIE_HOMEWORLD: u32 = 19;
const SPACE_HELLO_1: u32 = 20;
const SPACE_HELLO_2: u32 = 21;
const SPACE_HELLO_3: u32 = 22;
const SPACE_HELLO_4: u32 = 23;
const WHATS_UP_SPACE_1: u32 = 24;
const GENERAL_INFO_SPACE_1: u32 = 25;
const WHATS_UP_SPACE_2: u32 = 26;
const GENERAL_INFO_SPACE_2: u32 = 27;
const WHATS_UP_SPACE_3: u32 = 28;
const GENERAL_INFO_SPACE_3: u32 = 29;
const WHATS_UP_SPACE_4: u32 = 30;
const GENERAL_INFO_SPACE_4: u32 = 31;
const I_DEMAND_YOU_ALLY_SPACE0: u32 = 32;
const I_DEMAND_YOU_ALLY_SPACE1: u32 = 33;
const I_DEMAND_YOU_ALLY_SPACE2: u32 = 34;
const I_DEMAND_YOU_ALLY_SPACE3: u32 = 35;
const WE_CANNOT_1: u32 = 36;
const OBLIGATION: u32 = 37;
const WE_CANNOT_2: u32 = 38;
const AT_LEAST_HELP_US_SPACE: u32 = 39;
const SORRY_CANNOT: u32 = 40;
const DISHONOR: u32 = 41;
const HERES_A_HINT: u32 = 42;
const WHAT_ABOUT_PKUNK_SPACE: u32 = 43;
const PKUNK_ABSORBED_SPACE: u32 = 44;
const HATE_PKUNK_SPACE: u32 = 45;
const BYE_SPACE: u32 = 46;
const GO_IN_PEACE: u32 = 47;
const GOODBYE_AND_DIE_SPACE: u32 = 48;
const SHOFIXTI_ALIVE_1: u32 = 49;
const SHOFIXTI_ALIVE_2: u32 = 50;
const SEND_HIM_OVER_1: u32 = 51;
const SEND_HIM_OVER_2: u32 = 52;
const NOT_HERE: u32 = 53;
const NOT_SEND: u32 = 54;
const JUST_A_TRICK_1: u32 = 55;
const JUST_A_TRICK_2: u32 = 56;
const OK_SEND: u32 = 57;
const WE_REVOLT: u32 = 58;
const ROYALIST_SPACE_HELLO_1: u32 = 59;
const ROYALIST_SPACE_HELLO_2: u32 = 60;
const ROYALIST_HOMEWORLD_HELLO_1: u32 = 61;
const ROYALIST_HOMEWORLD_HELLO_2: u32 = 62;
const HOW_IS_REBELLION: u32 = 63;
const ROYALIST_REBELLION_1: u32 = 64;
const ROYALIST_REBELLION_2: u32 = 65;
const SORRY_ABOUT_REVOLUTION: u32 = 66;
const ALL_YOUR_FAULT: u32 = 67;
const BYE_ROYALIST: u32 = 68;
const GOODBYE_AND_DIE_ROYALIST: u32 = 69;
const NAME_1: u32 = 70;
const NAME_2: u32 = 71;
const NAME_3: u32 = 72;
const NAME_40: u32 = 73;
const NAME_41: u32 = 74;
const OUT_TAKES: u32 = 75;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.yehat.graphics\0";
const RACE_FONT: &[u8] = b"comm.yehat.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.yehat.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.yehat.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.yehat.dialogue\0";

/// Yehat race dialogue implementation.
pub struct YehatDialogue;

impl super::RaceDialogue for YehatDialogue {
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
            bit_range("NO_YEHAT_ALLY_HOME").is_some(),
            "missing game state key: NO_YEHAT_ALLY_HOME"
        );
        assert!(
            bit_range("NO_YEHAT_ALLY_SPACE").is_some(),
            "missing game state key: NO_YEHAT_ALLY_SPACE"
        );
        assert!(
            bit_range("NO_YEHAT_HELP_HOME").is_some(),
            "missing game state key: NO_YEHAT_HELP_HOME"
        );
        assert!(
            bit_range("NO_YEHAT_HELP_SPACE").is_some(),
            "missing game state key: NO_YEHAT_HELP_SPACE"
        );
    }
}
