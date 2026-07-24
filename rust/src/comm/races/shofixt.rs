//! Shofixt dialogue state machine — ported from C.
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
const NAME_1: u32 = 1;
const NAME_2: u32 = 2;
const NAME_3: u32 = 3;
const NAME_40: u32 = 4;
const NAME_41: u32 = 5;
const TANAKA: u32 = 6;
const KATANA: u32 = 7;
const HOSTILE_KATANA_1: u32 = 8;
const HOSTILE_KATANA_2: u32 = 9;
const HOSTILE_KATANA_3: u32 = 10;
const HOSTILE_KATANA_4: u32 = 11;
const HOSTILE_TANAKA_1: u32 = 12;
const HOSTILE_TANAKA_2: u32 = 13;
const HOSTILE_TANAKA_3: u32 = 14;
const HOSTILE_TANAKA_4: u32 = 15;
const HOSTILE_TANAKA_5: u32 = 16;
const HOSTILE_TANAKA_6: u32 = 17;
const HOSTILE_TANAKA_7: u32 = 18;
const HOSTILE_TANAKA_8: u32 = 19;
const DONT_ATTACK: u32 = 20;
const TYPICAL_PLOY: u32 = 21;
const HEY_STOP: u32 = 22;
const ONLY_STOP: u32 = 23;
const LOOK_YOU_ARE: u32 = 24;
const TOO_BAD: u32 = 25;
const DONT_KNOW: u32 = 26;
const NEVER: u32 = 27;
const LOOK0: u32 = 28;
const LOOK1: u32 = 29;
const FOR_YOU: u32 = 30;
const NO_BLOODSHED: u32 = 31;
const YES_BLOODSHED: u32 = 32;
const DONT_WANT_TO_FIGHT: u32 = 33;
const MUST_FIGHT_YOU_URQUAN_1: u32 = 34;
const MUST_FIGHT_YOU_URQUAN_2: u32 = 35;
const MUST_FIGHT_YOU_URQUAN_3: u32 = 36;
const MUST_FIGHT_YOU_URQUAN_4: u32 = 37;
const NO_ONE_INSULTS: u32 = 38;
const YOU_LIMP: u32 = 39;
const MIGHTY_WORDS: u32 = 40;
const HANG_YOUR: u32 = 41;
const DONKEY_BREATH: u32 = 42;
const DGRUNTI: u32 = 43;
const I_AM_CAPTAIN0: u32 = 44;
const I_AM_CAPTAIN1: u32 = 45;
const I_AM_CAPTAIN2: u32 = 46;
const I_AM_CAPTAIN3: u32 = 47;
const I_AM_NICE: u32 = 48;
const I_AM_GUY: u32 = 49;
const SO_SORRY: u32 = 50;
const MUST_UNDERSTAND: u32 = 51;
const NICE_BUT_WHAT_IS_DONKEY: u32 = 52;
const IS_DEFEAT_TRUE: u32 = 53;
const YES_AND_NO: u32 = 54;
const BUTT_BLASTED: u32 = 55;
const CLOBBERED: u32 = 56;
const VERY_SAD_KILL_SELF: u32 = 57;
const IMPORTANT_DUTY: u32 = 58;
const WHAT_DUTY: u32 = 59;
const NEED_YOU_FOR_DUTY: u32 = 60;
const OK_WILL_BE_SENTRY: u32 = 61;
const DONT_DO_IT: u32 = 62;
const YES_I_DO_IT: u32 = 63;
const GO_AHEAD: u32 = 64;
const ON_SECOND_THOUGHT: u32 = 65;
const PROCREATING_WILDLY: u32 = 66;
const REPLENISHING_YOUR_SPECIES: u32 = 67;
const HOPE_YOU_HAVE: u32 = 68;
const SOUNDS_GREAT_BUT_HOW: u32 = 69;
const FEMALES: u32 = 70;
const NUBILES: u32 = 71;
const RAT_BABES: u32 = 72;
const LEAPING_HAPPINESS: u32 = 73;
const BYE0: u32 = 74;
const BYE1: u32 = 75;
const GOODBYE0: u32 = 76;
const GOODBYE1: u32 = 77;
const WHY_HERE0: u32 = 78;
const WHY_HERE1: u32 = 79;
const I_GUARD: u32 = 80;
const WHERE_WORLD: u32 = 81;
const BLEW_IT_UP: u32 = 82;
const HOW_SURVIVE: u32 = 83;
const NOT_HERE: u32 = 84;
const WHAT_HAPPENED: u32 = 85;
const MET_VUX: u32 = 86;
const GLORY_DEVICE: u32 = 87;
const SWITCH_BROKE: u32 = 88;
const BYE: u32 = 89;
const GOODBYE: u32 = 90;
const FRIENDLY_HELLO: u32 = 91;
const REPORT0: u32 = 92;
const REPORT1: u32 = 93;
const NOTHING_NEW: u32 = 94;
const OUT_TAKES: u32 = 95;

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

const RACE_PMAP_ANIM: &[u8] = b"shofixt\0";
const RACE_FONT: &[u8] = b"shofixtfont\0";
const RACE_COLOR_MAP: &[u8] = b"shofixtcolr\0";
const RACE_MUSIC: &[u8] = b"shofixtmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.shofixt.dialogue\0";

/// Shofixt race dialogue implementation.
pub struct ShofixtDialogue;

impl super::RaceDialogue for ShofixtDialogue {
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
            bit_range("MAIDENS_ON_SHIP").is_some(),
            "missing game state key: MAIDENS_ON_SHIP"
        );
        assert!(
            bit_range("SHOFIXTI_KIA").is_some(),
            "missing game state key: SHOFIXTI_KIA"
        );
        assert!(
            bit_range("SHOFIXTI_RECRUITED").is_some(),
            "missing game state key: SHOFIXTI_RECRUITED"
        );
        assert!(
            bit_range("SHOFIXTI_STACK1").is_some(),
            "missing game state key: SHOFIXTI_STACK1"
        );
        assert!(
            bit_range("SHOFIXTI_STACK2").is_some(),
            "missing game state key: SHOFIXTI_STACK2"
        );
    }
}
