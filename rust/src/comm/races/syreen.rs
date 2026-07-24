//! Syreen dialogue state machine — ported from C.
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
const HELLO_BEFORE_AMBUSH_1: u32 = 1;
const HELLO_BEFORE_AMBUSH_2: u32 = 2;
const HELLO_BEFORE_AMBUSH_3: u32 = 3;
const HELLO_BEFORE_AMBUSH_4: u32 = 4;
const WE_ARE_VICE_SQUAD: u32 = 5;
const OK_VICE: u32 = 6;
const WE_ARE_THE_ONE_FOR_YOU_BABY: u32 = 7;
const MAYBE_CAPTAIN: u32 = 8;
const WE_ARE_VINDICATOR0: u32 = 9;
const WE_ARE_VINDICATOR1: u32 = 10;
const WE_ARE_VINDICATOR2: u32 = 11;
const WELCOME_VINDICATOR0: u32 = 12;
const WELCOME_VINDICATOR1: u32 = 13;
const WELCOME_VINDICATOR2: u32 = 14;
const WE_ARE_IMPRESSED: u32 = 15;
const SO_AM_I_CAPTAIN: u32 = 16;
const HOW_CAN_YOU_BE_HERE: u32 = 17;
const WE_HERE_TO_HELP: u32 = 18;
const NO_NEED_HELP: u32 = 19;
const WE_NEED_HELP: u32 = 20;
const CANT_GIVE_HELP: u32 = 21;
const I_NEED_YOU: u32 = 22;
const OK_NEED: u32 = 23;
const I_NEED_TOUCH_O_VISION: u32 = 24;
const TOUCH_O_VISION: u32 = 25;
const KNOW_ABOUT_DEEP_CHILDREN: u32 = 26;
const WHAT_ABOUT_DEEP_CHILDREN: u32 = 27;
const MYCONS_INVOLVED: u32 = 28;
const WHAT_PROOF: u32 = 29;
const HAVE_NO_PROOF: u32 = 30;
const NEED_PROOF: u32 = 31;
const HAVE_PROOF: u32 = 32;
const SEE_PROOF: u32 = 33;
const LOOK_AT_EGG_SACKS: u32 = 34;
const HORRIBLE_TRUTH: u32 = 35;
const WHAT_DOING_HERE: u32 = 36;
const OUR_NEW_WORLD: u32 = 37;
const WHAT_ABOUT_WAR: u32 = 38;
const ABOUT_WAR: u32 = 39;
const HELP_US: u32 = 40;
const WONT_HELP: u32 = 41;
const WHAT_ABOUT_HISTORY: u32 = 42;
const BEFORE_WAR: u32 = 43;
const WHAT_ABOUT_HOMEWORLD: u32 = 44;
const ABOUT_HOMEWORLD: u32 = 45;
const WHAT_HAPPENED: u32 = 46;
const DONT_KNOW_HOW: u32 = 47;
const WHAT_ABOUT_OUTFIT: u32 = 48;
const HOPE_YOU_LIKE_IT: u32 = 49;
const WHERE_MATES: u32 = 50;
const MATES_KILLED: u32 = 51;
const GET_LONELY: u32 = 52;
const MAKE_OUT_ALL_RIGHT: u32 = 53;
const BYE: u32 = 54;
const GOODBYE: u32 = 55;
const MUST_ACT: u32 = 56;
const WHATS_NEXT_STEP: u32 = 57;
const OPEN_VAULT: u32 = 58;
const WHERE_IS_IT: u32 = 59;
const DONT_KNOW_WHERE: u32 = 60;
const BEEN_THERE: u32 = 61;
const GREAT: u32 = 62;
const GIVE_SHUTTLE: u32 = 63;
const IM_ON_MY_WAY: u32 = 64;
const DOING_THIS_FOR_YOU: u32 = 65;
const IF_I_DIE: u32 = 66;
const GOOD_LUCK: u32 = 67;
const OK_FOUND_VAULT: u32 = 68;
const WHAT_NOW: u32 = 69;
const HERES_THE_PLAN: u32 = 70;
const WHATS_MY_REWARD: u32 = 71;
const HERES_REWARD: u32 = 72;
const BYE_AFTER_VAULT: u32 = 73;
const GOODBYE_AFTER_VAULT: u32 = 74;
const HELLO_AFTER_AMBUSH_1: u32 = 75;
const HELLO_AFTER_AMBUSH_2: u32 = 76;
const HELLO_AFTER_AMBUSH_3: u32 = 77;
const HELLO_AFTER_AMBUSH_4: u32 = 78;
const WHAT_NOW_AFTER_AMBUSH: u32 = 79;
const DO_THIS_AFTER_AMBUSH: u32 = 80;
const WHAT_ABOUT_YOU: u32 = 81;
const ABOUT_ME: u32 = 82;
const WHATS_UP_AFTER_AMBUSH: u32 = 83;
const GENERAL_INFO_AFTER_AMBUSH_1: u32 = 84;
const GENERAL_INFO_AFTER_AMBUSH_2: u32 = 85;
const GENERAL_INFO_AFTER_AMBUSH_3: u32 = 86;
const GENERAL_INFO_AFTER_AMBUSH_4: u32 = 87;
const BYE_AFTER_AMBUSH: u32 = 88;
const GOODBYE_AFTER_AMBUSH: u32 = 89;
const FOUND_VAULT_YET_1: u32 = 90;
const FOUND_VAULT_YET_2: u32 = 91;
const VAULT_HINT: u32 = 92;
const OK_HINT: u32 = 93;
const FOUND_VAULT: u32 = 94;
const BYE_BEFORE_VAULT: u32 = 95;
const GOODBYE_BEFORE_VAULT: u32 = 96;
const WHAT_DO_I_GET_FOR_THIS: u32 = 97;
const GRATITUDE: u32 = 98;
const NOT_SURE: u32 = 99;
const PLEASE: u32 = 100;
const READY_FOR_AMBUSH: u32 = 101;
const REPEAT_PLAN: u32 = 102;
const OK_REPEAT_PLAN: u32 = 103;
const BYE_BEFORE_AMBUSH: u32 = 104;
const GOODBYE_BEFORE_AMBUSH: u32 = 105;
const WHAT_ABOUT_US: u32 = 106;
const ABOUT_US: u32 = 107;
const MORE_COMFORTABLE: u32 = 108;
const IN_THE_SPIRIT: u32 = 109;
const OK_SPIRIT: u32 = 110;
const WHAT_IN_MIND: u32 = 111;
const SOMETHING_LIKE_THIS: u32 = 112;
const HANDS_OFF: u32 = 113;
const OK_WONT_USE_HANDS: u32 = 114;
const WHY_LIGHTS_OFF: u32 = 115;
const LIGHTS_OFF_BECAUSE: u32 = 116;
const EVIL_MONSTER: u32 = 117;
const NOT_EVIL_MONSTER: u32 = 118;
const DISEASE: u32 = 119;
const JUST_RELAX: u32 = 120;
const WHAT_HAPPENS_IF_I_TOUCH_THIS: u32 = 121;
const THIS_HAPPENS: u32 = 122;
const ARE_YOU_SURE_THIS_IS_OK: u32 = 123;
const YES_SURE: u32 = 124;
const BOY_THEY_NEVER_TAUGHT: u32 = 125;
const THEN_LET_ME_TEACH: u32 = 126;
const NOT_MUCH_MORE_TO_SAY: u32 = 127;
const THEN_STOP_TALKING: u32 = 128;
const LATER: u32 = 129;
const SEX_GOODBYE: u32 = 130;
const OUT_TAKES: u32 = 131;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.syreen.graphics\0";
const RACE_FONT: &[u8] = b"comm.syreen.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.syreen.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.syreen.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.syreen.dialogue\0";

/// Syreen race dialogue implementation.
pub struct SyreenDialogue;

impl super::RaceDialogue for SyreenDialogue {
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
            bit_range("EGG_CASE0_ON_SHIP").is_some(),
            "missing game state key: EGG_CASE0_ON_SHIP"
        );
        assert!(
            bit_range("EGG_CASE1_ON_SHIP").is_some(),
            "missing game state key: EGG_CASE1_ON_SHIP"
        );
        assert!(
            bit_range("EGG_CASE2_ON_SHIP").is_some(),
            "missing game state key: EGG_CASE2_ON_SHIP"
        );
        assert!(
            bit_range("KNOW_ABOUT_SHATTERED").is_some(),
            "missing game state key: KNOW_ABOUT_SHATTERED"
        );
        assert!(
            bit_range("KNOW_SYREEN_VAULT").is_some(),
            "missing game state key: KNOW_SYREEN_VAULT"
        );
    }
}
