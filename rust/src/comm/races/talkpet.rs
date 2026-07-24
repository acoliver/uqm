//! Talkpet dialogue state machine — ported from C.
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
const HELLO_AT_UMGAH: u32 = 1;
const WHAT_ARE_YOU: u32 = 2;
const JUST_TALKING_PET: u32 = 3;
const TALKING_PETS_DUMB: u32 = 4;
const OH_NO_YOU_DONT: u32 = 5;
const WHAT_DO_TO_UMGAH: u32 = 6;
const DID_NOTHING: u32 = 7;
const UMGAH_ZOMBIES: u32 = 8;
const WORKS_LIKE_THIS: u32 = 9;
const WE_ARE_VINDICATOR0: u32 = 10;
const WE_ARE_VINDICATOR1: u32 = 11;
const GOOD_FOR_YOU: u32 = 12;
const MUST_EXPLAIN_PRESENCE: u32 = 13;
const EXPLAIN_NOTHING_MONKEY_BOY: u32 = 14;
const BYE_AT_UMGAH: u32 = 15;
const GOODBYE_AT_UMGAH: u32 = 16;
const HYPNOTIZE_AGAIN_1: u32 = 17;
const HYPNOTIZE_AGAIN_2: u32 = 18;
const HYPNOTIZE_AGAIN_3: u32 = 19;
const HYPNOTIZE_AGAIN_4: u32 = 20;
const HYPNO_TAIL: u32 = 21;
const CANT_COMPEL: u32 = 22;
const LETS_MAKE_A_DEAL: u32 = 23;
const WHAT_KIND_OF_DEAL: u32 = 24;
const HELP_DEFEAT_URQUAN: u32 = 25;
const OK_LETS_DO_IT: u32 = 26;
const COMING_ABOARD: u32 = 27;
const HOW_TRUST: u32 = 28;
const TRUST: u32 = 29;
const BONELESS_DWEEB: u32 = 30;
const YOUR_BONELESS_DWEEB: u32 = 31;
const WHAT_ARE_YOU_REALLY: u32 = 32;
const POOR_DNYARRI: u32 = 33;
const HARD_TO_BELIEVE: u32 = 34;
const ITS_TRUE: u32 = 35;
const BULLSHIT: u32 = 36;
const WORTH_A_TRY: u32 = 37;
const KILL_YOU: u32 = 38;
const PLEASE_DONT: u32 = 39;
const MUST_KILL: u32 = 40;
const DONT_KILL: u32 = 41;
const WANT_KILL_1: u32 = 42;
const WANT_KILL_2: u32 = 43;
const WANT_KILL_3: u32 = 44;
const GLAD_YOU_WONT_KILL: u32 = 45;
const WHATS_UP_ONBOARD: u32 = 46;
const GENERAL_INFO_ONBOARD_1: u32 = 47;
const GENERAL_INFO_ONBOARD_2: u32 = 48;
const GENERAL_INFO_ONBOARD_3: u32 = 49;
const GENERAL_INFO_ONBOARD_4: u32 = 50;
const GENERAL_INFO_ONBOARD_5: u32 = 51;
const GENERAL_INFO_ONBOARD_6: u32 = 52;
const GENERAL_INFO_ONBOARD_7: u32 = 53;
const GENERAL_INFO_ONBOARD_8: u32 = 54;
const HELLO_AS_DEVICE_1: u32 = 55;
const HELLO_AS_DEVICE_2: u32 = 56;
const HELLO_AS_DEVICE_3: u32 = 57;
const HELLO_AS_DEVICE_4: u32 = 58;
const HELLO_AS_DEVICE_5: u32 = 59;
const HELLO_AS_DEVICE_6: u32 = 60;
const HELLO_AS_DEVICE_7: u32 = 61;
const HELLO_AS_DEVICE_8: u32 = 62;
const CYBORG_PEP_TALK: u32 = 63;
const HUMAN_PEP_TALK: u32 = 64;
const I_SENSE_MY_SLAVES: u32 = 65;
const HAVENT_GOT_EVERYTHING: u32 = 66;
const NEED_BOMB: u32 = 67;
const SOUP_UP_BOMB: u32 = 68;
const SOUP_UP_FLEET: u32 = 69;
const SOUP_UP_FLAGSHIP: u32 = 70;
const COMEBACK_WHEN_READY: u32 = 71;
const WHAT_NOW: u32 = 72;
const DO_THIS: u32 = 73;
const COMPEL_URQUAN: u32 = 74;
const HERE_WE_GO: u32 = 75;
const IM_SCARED: u32 = 76;
const STUPID_FOP: u32 = 77;
const COMPEL_THAT_SHIP: u32 = 78;
const SAVING_MY_POWER: u32 = 79;
const ANY_SUGGESTIONS: u32 = 80;
const SUGGESTION_1: u32 = 81;
const SUGGESTION_2: u32 = 82;
const SUGGESTION_3: u32 = 83;
const SUGGESTION_4: u32 = 84;
const SUGGESTION_5: u32 = 85;
const SUGGESTION_6: u32 = 86;
const SUGGESTION_7: u32 = 87;
const SUGGESTION_8: u32 = 88;
const ABOUT_YOUR_RACE: u32 = 89;
const WHAT_ABOUT_RACE: u32 = 90;
const YOU_LIED: u32 = 91;
const SO_WHAT: u32 = 92;
const BYE_ONBOARD: u32 = 93;
const GOODBYE_ONBOARD: u32 = 94;
const WHAT_ABOUT_PHYSIOLOGY: u32 = 95;
const NO_TALK_ABOUT_SELF: u32 = 96;
const WHAT_ABOUT_POWERS: u32 = 97;
const NOT_POWERS_BUT_FLOWERS: u32 = 98;
const YES_FLOWERS: u32 = 99;
const GOOD_HUMAN: u32 = 100;
const WISH_TO_GO_NOW: u32 = 101;
const EXCELLENT_IDEA: u32 = 102;
const WHAT_ABOUT_YOUR_HISTORY: u32 = 103;
const ABOUT_HISTORY: u32 = 104;
const SENTIENT_MILIEU: u32 = 105;
const ABOUT_SENTIENT_MILIEU: u32 = 106;
const WHAT_ABOUT_WAR: u32 = 107;
const ABOUT_WAR: u32 = 108;
const ENOUGH_INFO: u32 = 109;
const OK_ENOUGH_INFO: u32 = 110;
const UMGAH_ALL_GONE: u32 = 111;
const HELLO_AFTER_COMPEL_URQUAN: u32 = 112;
const OUT_TAKES: u32 = 113;

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

const RACE_PMAP_ANIM: &[u8] = b"talkpet\0";
const RACE_FONT: &[u8] = b"talkpetfont\0";
const RACE_COLOR_MAP: &[u8] = b"talkpetcolr\0";
const RACE_MUSIC: &[u8] = b"talkpetmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.talkpet.dialogue\0";

/// Talkpet race dialogue implementation.
pub struct TalkpetDialogue;

impl super::RaceDialogue for TalkpetDialogue {
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
            bit_range("ARILOU_STACK_2").is_some(),
            "missing game state key: ARILOU_STACK_2"
        );
        assert!(
            bit_range("AWARE_OF_SAMATRA").is_some(),
            "missing game state key: AWARE_OF_SAMATRA"
        );
        assert!(
            bit_range("CHMMR_BOMB_STATE").is_some(),
            "missing game state key: CHMMR_BOMB_STATE"
        );
        assert!(
            bit_range("DNYARRI_LIED").is_some(),
            "missing game state key: DNYARRI_LIED"
        );
        assert!(
            bit_range("KNOW_UMGAH_ZOMBIES").is_some(),
            "missing game state key: KNOW_UMGAH_ZOMBIES"
        );
    }
}
