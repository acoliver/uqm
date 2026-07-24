//! Umgah dialogue state machine — ported from C.
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
const HWLD_PRE_ZOMBIE_HELLO_1: u32 = 1;
const HWLD_PRE_ZOMBIE_HELLO_2: u32 = 2;
const HWLD_PRE_ZOMBIE_HELLO_3: u32 = 3;
const HWLD_PRE_ZOMBIE_HELLO_4: u32 = 4;
const SPACE_PRE_ZOMBIE_HELLO_1: u32 = 5;
const SPACE_PRE_ZOMBIE_HELLO_2: u32 = 6;
const SPACE_PRE_ZOMBIE_HELLO_3: u32 = 7;
const SPACE_PRE_ZOMBIE_HELLO_4: u32 = 8;
const UNKNOWN_ZOMBIE_HELLO_1: u32 = 9;
const UNKNOWN_ZOMBIE_HELLO_2: u32 = 10;
const UNKNOWN_ZOMBIE_HELLO_3: u32 = 11;
const UNKNOWN_ZOMBIE_HELLO_4: u32 = 12;
const DESTROY_INTERFERER_1: u32 = 13;
const DESTROY_INTERFERER_2: u32 = 14;
const DESTROY_INTERFERER_3: u32 = 15;
const DESTROY_INTERFERER_4: u32 = 16;
const REVEALED_ZOMBIE_HELLO_1: u32 = 17;
const REVEALED_ZOMBIE_HELLO_2: u32 = 18;
const REVEALED_ZOMBIE_HELLO_3: u32 = 19;
const REVEALED_ZOMBIE_HELLO_4: u32 = 20;
const HOSTILE_HELLO_1: u32 = 21;
const HOSTILE_HELLO_2: u32 = 22;
const HOSTILE_HELLO_3: u32 = 23;
const HOSTILE_HELLO_4: u32 = 24;
const REWARD_AT_HOMEWORLD_1: u32 = 25;
const REWARD_AT_HOMEWORLD_2: u32 = 26;
const POST_ZOMBIE_HWLD_HELLO: u32 = 27;
const OWE_ME_BIG_TIME: u32 = 28;
const OUR_LARGESSE: u32 = 29;
const GIVE_LIFEDATA: u32 = 30;
const THANKS: u32 = 31;
const WHAT_DO_WITH_TPET: u32 = 32;
const TRICK_URQUAN: u32 = 33;
const ANY_JOKES: u32 = 34;
const SURE: u32 = 35;
const WHAT_BEFORE_TPET: u32 = 36;
const TRKD_SPATHI_AND_ILWRATH: u32 = 37;
const WHERE_CASTER: u32 = 38;
const SPATHI_TOOK_THEM: u32 = 39;
const SO_WHAT_FOR_NOW: u32 = 40;
const DO_THIS_NOW: u32 = 41;
const BYE_POST_ZOMBIE: u32 = 42;
const FUNNY_IDEA: u32 = 43;
const WHATS_UP_PRE_ZOMBIE: u32 = 44;
const GENERAL_INFO_PRE_ZOMBIE: u32 = 45;
const EVIL_BLOBBIES_GIVE_UP: u32 = 46;
const NOT_EVIL_BLOBBIES: u32 = 47;
const EVIL_BLOBBIES_MUST_DIE: u32 = 48;
const OH_NO_WE_WONT: u32 = 49;
const CAN_WE_BE_FRIENDS: u32 = 50;
const SURE_FRIENDS: u32 = 51;
const WANT_TO_DEFEAT_URQUAN: u32 = 52;
const FINE_BY_US: u32 = 53;
const BYE_PRE_ZOMBIE: u32 = 54;
const GOODBYE_PRE_ZOMBIE: u32 = 55;
const THREAT: u32 = 56;
const NO_THREAT: u32 = 57;
const WHATS_UP_ZOMBIES: u32 = 58;
const GENERAL_INFO_ZOMBIE: u32 = 59;
const HOW_GOES_TPET: u32 = 60;
const WHAT_TPET: u32 = 61;
const YOU_TOLD_US: u32 = 62;
const SADLY_IT_DIED: u32 = 63;
const DONT_BELIEVE: u32 = 64;
const THEN_DIE: u32 = 65;
const BYE_UNKNOWN: u32 = 66;
const GOODBYE_UNKNOWN: u32 = 67;
const EVIL_BLOBBIES: u32 = 68;
const YES_VERY_EVIL: u32 = 69;
const GIVE_UP_OR_DIE: u32 = 70;
const NOT_GIVE_UP: u32 = 71;
const WE_VINDICATOR0: u32 = 72;
const WE_VINDICATOR1: u32 = 73;
const WE_VINDICATOR2: u32 = 74;
const GOOD_FOR_YOU_1: u32 = 75;
const COME_IN_PEACE: u32 = 76;
const GOOD_FOR_YOU_2: u32 = 77;
const KNOW_ANY_JOKES: u32 = 78;
const JOKE_1: u32 = 79;
const BETTER_JOKE: u32 = 80;
const JOKE_2: u32 = 81;
const NOT_VERY_FUNNY: u32 = 82;
const YES_WE_ARE: u32 = 83;
const WHAT_ABOUT_TPET: u32 = 84;
const ARILOU_TOLD_US: u32 = 85;
const BYE_ZOMBIE: u32 = 86;
const GOODBYE_ZOMBIE: u32 = 87;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.umgah.graphics\0";
const RACE_FONT: &[u8] = b"comm.umgah.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.umgah.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.umgah.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.umgah.dialogue\0";

/// Umgah race dialogue implementation.
pub struct UmgahDialogue;

impl super::RaceDialogue for UmgahDialogue {
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
            bit_range("KNOW_UMGAH_ZOMBIES").is_some(),
            "missing game state key: KNOW_UMGAH_ZOMBIES"
        );
        assert!(
            bit_range("MET_NORMAL_UMGAH").is_some(),
            "missing game state key: MET_NORMAL_UMGAH"
        );
        assert!(
            bit_range("TALKING_PET").is_some(),
            "missing game state key: TALKING_PET"
        );
        assert!(
            bit_range("TALKING_PET_VISITS").is_some(),
            "missing game state key: TALKING_PET_VISITS"
        );
    }
}
