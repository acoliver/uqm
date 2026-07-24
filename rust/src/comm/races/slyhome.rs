//! Slyhome dialogue state machine — ported from C.
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
const HELLO_1: u32 = 1;
const HELLO_2: u32 = 2;
const HELLO_3: u32 = 3;
const HELLO_4: u32 = 4;
const RECALL_PROGRAM_1: u32 = 5;
const WE_ARE_US0: u32 = 6;
const WE_ARE_US1: u32 = 7;
const WE_ARE_US2: u32 = 8;
const TERRIBLY_EXCITING: u32 = 9;
const HAPPY_TO_TELL_MORE: u32 = 10;
const TELL_MORE: u32 = 11;
const WOULD_YOU_LIKE_TO_KNOW_MORE: u32 = 12;
const YES_TELL_MORE: u32 = 13;
const WE_COME_FROM_EARTH: u32 = 14;
const OK_EARTH: u32 = 15;
const WE_EXPLORE: u32 = 16;
const OK_EXPLORE: u32 = 17;
const WE_FIGHT_URQUAN: u32 = 18;
const URQUAN_NICE_GUYS: u32 = 19;
const NOT_SAME_URQUAN: u32 = 20;
const PERSONALITY_CHANGE: u32 = 21;
const WE_GATHER: u32 = 22;
const MAYBE_INTERESTED: u32 = 23;
const ENOUGH_ABOUT_ME: u32 = 24;
const OK_ENOUGH_YOU: u32 = 25;
const WHAT_OTHER_VISITORS: u32 = 26;
const VISITORS: u32 = 27;
const ANY_OTHER_VISITORS: u32 = 28;
const LONG_AGO: u32 = 29;
const WHAT_ABOUT_SENTIENT_MILIEU: u32 = 30;
const MET_TAALO_THEY_ARE_FROM: u32 = 31;
const WHO_ELSE: u32 = 32;
const PRECURSORS: u32 = 33;
const PRECURSORS_YOW: u32 = 34;
const ABOUT_PRECURSORS: u32 = 35;
const MUST_KNOW_MORE: u32 = 36;
const ALL_WE_KNOW: u32 = 37;
const WHO_ARE_YOU: u32 = 38;
const WE_ARE_SLY: u32 = 39;
const LIKE_MORE_ABOUT_YOU: u32 = 40;
const SURE_KNOW_WHAT: u32 = 41;
const WHAT_ABOUT_HOME: u32 = 42;
const ABOUT_HOME: u32 = 43;
const WHAT_ABOUT_CULTURE: u32 = 44;
const ABOUT_CULTURE: u32 = 45;
const WHAT_ABOUT_HISTORY: u32 = 46;
const ABOUT_HISTORY: u32 = 47;
const WHAT_ABOUT_BIOLOGY: u32 = 48;
const ABOUT_BIOLOGY: u32 = 49;
const ENOUGH_INFO: u32 = 50;
const OK_ENOUGH_INFO: u32 = 51;
const WHERE_ARE_YOU: u32 = 52;
const DOWN_HERE: u32 = 53;
const THATS_IMPOSSIBLE_1: u32 = 54;
const NO_ITS_NOT_1: u32 = 55;
const THATS_IMPOSSIBLE_2: u32 = 56;
const NO_ITS_NOT_2: u32 = 57;
const BYE: u32 = 58;
const GOODBYE_1: u32 = 59;
const GOODBYE_2: u32 = 60;
const WHAT_ARE_PROBES: u32 = 61;
const PROBES_ARE: u32 = 62;
const KNOW_MORE_PROBE: u32 = 63;
const OK_WHAT_MORE_PROBE: u32 = 64;
const WHERE_PROBES_FROM: u32 = 65;
const PROBES_FROM_MELNORME: u32 = 66;
const WHY_SELL: u32 = 67;
const SELL_FOR_INFO: u32 = 68;
const HOW_LONG_AGO: u32 = 69;
const FIFTY_THOUSAND_ROTATIONS: u32 = 70;
const WHATS_PROBES_MISSION: u32 = 71;
const SEEK_OUT_NEW_LIFE: u32 = 72;
const IF_ONLY_ONE: u32 = 73;
const THEY_REPLICATE: u32 = 74;
const ENOUGH_PROBE: u32 = 75;
const OK_ENOUGH_PROBE: u32 = 76;
const WHY_PROBE_ALWAYS_ATTACK: u32 = 77;
const ONLY_DEFEND: u32 = 78;
const TALK_MORE_PROBE_ATTACK: u32 = 79;
const NO_PROBLEM_BUT_SURE: u32 = 80;
const TELL_ME_ABOUT_BASICS: u32 = 81;
const BASIC_COMMANDS: u32 = 82;
const TELL_BASICS_AGAIN: u32 = 83;
const OK_BASICS_AGAIN: u32 = 84;
const WHAT_EFFECT: u32 = 85;
const AFFECTS_BEHAVIOR: u32 = 86;
const HOW_DOES_PROBE_DEFEND: u32 = 87;
const ONLY_SELF_DEFENSE: u32 = 88;
const COMBAT_BEHAVIOR: u32 = 89;
const MISSILE_BATTERIES: u32 = 90;
const WHAT_MISSILE_BATTERIES: u32 = 91;
const LIGHTNING_ONLY_FOR_HARVESTING: u32 = 92;
const TELL_ME_ABOUT_REP_1: u32 = 93;
const ABOUT_REP: u32 = 94;
const WHAT_SET_PRIORITY: u32 = 95;
const MAXIMUM: u32 = 96;
const ENOUGH_PROBLEM: u32 = 97;
const OK_ENOUGH_PROBLEM: u32 = 98;
const PROBE_HAS_BUG: u32 = 99;
const NO_IT_DOESNT: u32 = 100;
const TELL_ME_ABOUT_ATTACK: u32 = 101;
const ATTACK_NO_PROBLEM: u32 = 102;
const TELL_ME_ABOUT_REP_2: u32 = 103;
const REP_NO_PROBLEM: u32 = 104;
const WHAT_ABOUT_REP_PRIORITIES: u32 = 105;
const MAXIMUM_SO_WHAT: u32 = 106;
const THINK_ABOUT_REP_PRIORITIES: u32 = 107;
const UH_OH: u32 = 108;
const HUNT_THEM_DOWN: u32 = 109;
const GROW_TOO_FAST: u32 = 110;
const SUE_MELNORME: u32 = 111;
const SIGNED_WAIVER: u32 = 112;
const RECALL_SIGNAL: u32 = 113;
const NOT_THIS_MODEL: u32 = 114;
const MEGA_SELF_DESTRUCT: u32 = 115;
const WHY_YES_THERE_IS: u32 = 116;

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

const RACE_PMAP_ANIM: &[u8] = b"slyhome\0";
const RACE_FONT: &[u8] = b"slyhomefont\0";
const RACE_COLOR_MAP: &[u8] = b"slyhomecolr\0";
const RACE_MUSIC: &[u8] = b"slyhomemusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.slyhome.dialogue\0";

/// Slyhome race dialogue implementation.
pub struct SlyhomeDialogue;

impl super::RaceDialogue for SlyhomeDialogue {
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
            bit_range("PLAYER_KNOWS_EFFECTS").is_some(),
            "missing game state key: PLAYER_KNOWS_EFFECTS"
        );
        assert!(
            bit_range("PLAYER_KNOWS_PRIORITY").is_some(),
            "missing game state key: PLAYER_KNOWS_PRIORITY"
        );
        assert!(
            bit_range("PLAYER_KNOWS_PROBE").is_some(),
            "missing game state key: PLAYER_KNOWS_PROBE"
        );
        assert!(
            bit_range("PLAYER_KNOWS_PROGRAM").is_some(),
            "missing game state key: PLAYER_KNOWS_PROGRAM"
        );
    }
}
