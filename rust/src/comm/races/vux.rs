//! Vux dialogue state machine — ported from C.
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
const ZEX_HELLO_1: u32 = 1;
const ZEX_HELLO_2: u32 = 2;
const ZEX_HELLO_3: u32 = 3;
const ZEX_HELLO_4: u32 = 4;
const FIGHT_OR_TRADE_1: u32 = 5;
const FIGHT_OR_TRADE_2: u32 = 6;
const WHAT_YOU_DO_HERE: u32 = 7;
const MY_MENAGERIE: u32 = 8;
const WHAT_ABOUT_MENAGERIE: u32 = 9;
const NEED_NEW_CREATURE: u32 = 10;
const WHAT_ABOUT_CREATURE: u32 = 11;
const ABOUT_CREATURE: u32 = 12;
const ABOUT_CREATURE_AGAIN: u32 = 13;
const CREATURE_AGAIN: u32 = 14;
const I_HAVE_BEAST: u32 = 15;
const GIVE_BEAST: u32 = 16;
const OK_TAKE_BEAST: u32 = 17;
const FOOL_AIEE0: u32 = 18;
const FOOL_AIEE1: u32 = 19;
const WHY_TRUST_1: u32 = 20;
const TRUST_1: u32 = 21;
const WHY_TRUST_2: u32 = 22;
const TRUST_2: u32 = 23;
const WHY_TRUST_3: u32 = 24;
const TRUST_3: u32 = 25;
const WHY_DONT_YOU_ATTACK: u32 = 26;
const LIKE_YOU: u32 = 27;
const WHY_LIKE_ME: u32 = 28;
const LIKE_BECAUSE: u32 = 29;
const ARE_YOU_A_PERVERT: u32 = 30;
const CALL_ME_WHAT_YOU_WISH: u32 = 31;
const TAKE_BY_FORCE: u32 = 32;
const PRECURSOR_DEVICE: u32 = 33;
const REGARDLESS: u32 = 34;
const THEN_FIGHT: u32 = 35;
const YOU_LIED: u32 = 36;
const YUP_LIED: u32 = 37;
const KILL_YOU: u32 = 38;
const FIGHT_AGAIN: u32 = 39;
const BYE_ZEX: u32 = 40;
const GOODBYE_ZEX: u32 = 41;
const HOMEWORLD_HELLO_1: u32 = 42;
const HOMEWORLD_HELLO_2: u32 = 43;
const HOMEWORLD_HELLO_3: u32 = 44;
const HOMEWORLD_HELLO_4: u32 = 45;
const SPACE_HELLO_1: u32 = 46;
const SPACE_HELLO_2: u32 = 47;
const SPACE_HELLO_3: u32 = 48;
const SPACE_HELLO_4: u32 = 49;
const KILL_YOU_SQUIDS_1: u32 = 50;
const KILL_YOU_SQUIDS_2: u32 = 51;
const KILL_YOU_SQUIDS_3: u32 = 52;
const KILL_YOU_SQUIDS_4: u32 = 53;
const WE_FIGHT: u32 = 54;
const WHY_SO_MEAN: u32 = 55;
const URQUAN_SLAVES: u32 = 56;
const DEEPER_REASON: u32 = 57;
const OLD_INSULT: u32 = 58;
const IF_WE_APOLOGIZE: u32 = 59;
const PROBABLY_NOT: u32 = 60;
const TRY_ANY_WAY: u32 = 61;
const NOPE: u32 = 62;
const APOLOGIZE_IN_SPACE: u32 = 63;
const APOLOGY_1: u32 = 64;
const NOT_ACCEPTED_1: u32 = 65;
const APOLOGY_2: u32 = 66;
const NOT_ACCEPTED_2: u32 = 67;
const APOLOGY_3: u32 = 68;
const NOT_ACCEPTED_3: u32 = 69;
const APOLOGY_4: u32 = 70;
const NOT_ACCEPTED_4: u32 = 71;
const APOLOGY_5: u32 = 72;
const NOT_ACCEPTED_5: u32 = 73;
const APOLOGY_6: u32 = 74;
const NOT_ACCEPTED_6: u32 = 75;
const APOLOGY_7: u32 = 76;
const NOT_ACCEPTED_7: u32 = 77;
const APOLOGY_8: u32 = 78;
const NOT_ACCEPTED_8: u32 = 79;
const APOLOGY_9: u32 = 80;
const NOT_ACCEPTED_9: u32 = 81;
const APOLOGY_10: u32 = 82;
const TRUTH: u32 = 83;
const WHATS_UP_HOSTILE: u32 = 84;
const GENERAL_INFO_HOSTILE_1: u32 = 85;
const GENERAL_INFO_HOSTILE_2: u32 = 86;
const GENERAL_INFO_HOSTILE_3: u32 = 87;
const GENERAL_INFO_HOSTILE_4: u32 = 88;
const CANT_WE_BE_FRIENDS_1: u32 = 89;
const NEVER_UGLY_HUMANS_1: u32 = 90;
const CANT_WE_BE_FRIENDS_2: u32 = 91;
const NEVER_UGLY_HUMANS_2: u32 = 92;
const CANT_WE_BE_FRIENDS_3: u32 = 93;
const NEVER_UGLY_HUMANS_3: u32 = 94;
const CANT_WE_BE_FRIENDS_4: u32 = 95;
const NEVER_UGLY_HUMANS_4: u32 = 96;
const BYE_HOSTILE_SPACE: u32 = 97;
const GOODBYE_AND_DIE_HOSTILE_SPACE_1: u32 = 98;
const GOODBYE_AND_DIE_HOSTILE_SPACE_2: u32 = 99;
const GOODBYE_AND_DIE_HOSTILE_SPACE_3: u32 = 100;
const GOODBYE_AND_DIE_HOSTILE_SPACE_4: u32 = 101;
const OUT_TAKES: u32 = 102;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.vux.graphics\0";
const RACE_FONT: &[u8] = b"comm.vux.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.vux.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.vux.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.vux.dialogue\0";

/// Vux race dialogue implementation.
pub struct VuxDialogue;

impl super::RaceDialogue for VuxDialogue {
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
            bit_range("KNOW_ZEX_WANTS_MONSTER").is_some(),
            "missing game state key: KNOW_ZEX_WANTS_MONSTER"
        );
        assert!(
            bit_range("VUX_BEAST_ON_SHIP").is_some(),
            "missing game state key: VUX_BEAST_ON_SHIP"
        );
        assert!(
            bit_range("VUX_HOME_VISITS").is_some(),
            "missing game state key: VUX_HOME_VISITS"
        );
        assert!(
            bit_range("VUX_INFO").is_some(),
            "missing game state key: VUX_INFO"
        );
    }
}
