//! Urquan dialogue state machine — ported from C.
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
const HELLO_SAMATRA: u32 = 1;
const SENSE_EVIL: u32 = 2;
const INIT_URQUAN_WAKE_UP: u32 = 3;
const WHERE_AM_I: u32 = 4;
const YOU_ARE_HERE: u32 = 5;
const WHY_DOES_MY_HEAD_HURT: u32 = 6;
const HURTS_BECAUSE: u32 = 7;
const WHAT_ABOUT_2_WEEKS: u32 = 8;
const ABOUT_2_WEEKS: u32 = 9;
const COMPULSION: u32 = 10;
const WHAT_COMPULSION: u32 = 11;
const WASCALLY_LITTLE_GUY: u32 = 12;
const WHAT_IT_LOOK_LIKE: u32 = 13;
const TERRAN_AMPHIBIAN: u32 = 14;
const TALKING_PET_ON_STEROIDS: u32 = 15;
const BAD_NEWS: u32 = 16;
const TURD_AND_TOAD: u32 = 17;
const WHAT_IS_TURD_AND_TOAD: u32 = 18;
const ALIEN_MIND_CONTROL: u32 = 19;
const WHAT_FELT_LIKE: u32 = 20;
const POSSESSED_BY_DEVIL: u32 = 21;
const STUPID_DEVIL: u32 = 22;
const FALLING_ASLEEP: u32 = 23;
const SOMEONE_ELSE_CONTROLLED: u32 = 24;
const SOUNDS_FAMILIAR: u32 = 25;
const BEFORE_COFFEE: u32 = 26;
const EXPLAIN: u32 = 27;
const WHY_EXPLAIN: u32 = 28;
const MUST_EXPLAIN: u32 = 29;
const BYE_INIT_HYPNO: u32 = 30;
const GOODBYE_AND_DIE_INIT_HYPNO: u32 = 31;
const SUBSEQUENT_URQUAN_WAKE_UP: u32 = 32;
const UH_OH: u32 = 33;
const NO_UH_OH: u32 = 34;
const STOP_MEETING: u32 = 35;
const NO_STOP_MEETING: u32 = 36;
const BYE_SUB_HYPNO: u32 = 37;
const GOODBYE_AND_DIE_SUB_HYPNO: u32 = 38;
const CAUGHT_YA: u32 = 39;
const INIT_FLEE_HUMAN: u32 = 40;
const SUBSEQUENT_FLEE_HUMAN: u32 = 41;
const WHY_FLEE: u32 = 42;
const FLEE_BECAUSE: u32 = 43;
const WHAT_HAPPENS_NOW: u32 = 44;
const HAPPENS_NOW: u32 = 45;
const WHAT_ABOUT_YOU: u32 = 46;
const ABOUT_US: u32 = 47;
const BYE_WARS_OVER: u32 = 48;
const GOODBYE_WARS_OVER: u32 = 49;
const SEND_MESSAGE: u32 = 50;
const INIT_HELLO: u32 = 51;
const SUBSEQUENT_HELLO_1: u32 = 52;
const SUBSEQUENT_HELLO_2: u32 = 53;
const SUBSEQUENT_HELLO_3: u32 = 54;
const SUBSEQUENT_HELLO_4: u32 = 55;
const YOU_MUST_SURRENDER: u32 = 56;
const NOPE: u32 = 57;
const I_SURRENDER: u32 = 58;
const DISOBEDIENCE_PUNISHED: u32 = 59;
const I_WONT_SURRENDER: u32 = 60;
const BAD_CHOICE: u32 = 61;
const I_WILL_SURRENDER: u32 = 62;
const GOOD_CHOICE: u32 = 63;
const KEY_PHRASE: u32 = 64;
const URQUAN_STORY: u32 = 65;
const LIKE_TO_LEAVE: u32 = 66;
const INDEPENDENCE_IS_BAD: u32 = 67;
const WHATS_UP_1: u32 = 68;
const GENERAL_INFO_1: u32 = 69;
const WHATS_UP_2: u32 = 70;
const GENERAL_INFO_2: u32 = 71;
const WHATS_UP_3: u32 = 72;
const GENERAL_INFO_3: u32 = 73;
const WHATS_UP_4: u32 = 74;
const GENERAL_INFO_4: u32 = 75;
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

const RACE_PMAP_ANIM: &[u8] = b"comm.urquan.graphics\0";
const RACE_FONT: &[u8] = b"comm.urquan.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.urquan.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.urquan.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.urquan.dialogue\0";

/// Urquan race dialogue implementation.
pub struct UrquanDialogue;

impl super::RaceDialogue for UrquanDialogue {
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
            bit_range("KNOW_URQUAN_STORY").is_some(),
            "missing game state key: KNOW_URQUAN_STORY"
        );
        assert!(
            bit_range("KOHR_AH_FRENZY").is_some(),
            "missing game state key: KOHR_AH_FRENZY"
        );
        assert!(
            bit_range("MENTIONED_PET_COMPULSION").is_some(),
            "missing game state key: MENTIONED_PET_COMPULSION"
        );
        assert!(
            bit_range("PLAYER_HYPNOTIZED").is_some(),
            "missing game state key: PLAYER_HYPNOTIZED"
        );
    }
}
