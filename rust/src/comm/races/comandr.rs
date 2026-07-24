//! Comandr dialogue state machine — ported from C.
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
const GLAD_WHEN_YOU_COME_BACK: u32 = 1;
const GIVE_FUEL: u32 = 2;
const GIVE_FUEL_AGAIN: u32 = 3;
const ARE_YOU_SUPPLY_SHIP: u32 = 4;
const DO_YOU_HAVE_RADIO_THIS_TIME: u32 = 5;
const HERE_IS_ANOTHER_LANDER: u32 = 6;
const THE_WHAT_FROM_WHERE: u32 = 7;
const ABOUT_TIME: u32 = 8;
const MESSAGE_GARBLED_1: u32 = 9;
const MESSAGE_GARBLED_2: u32 = 10;
const HERE_IS_A_NEW_LANDER: u32 = 11;
const THIS_MAY_SEEM_SILLY: u32 = 12;
const OK_THE_NAFS: u32 = 13;
const OK_THE_CAN: u32 = 14;
const OK_THE_UFW: u32 = 15;
const OK_THE_NAME_IS_EMPIRE0: u32 = 16;
const OK_THE_NAME_IS_EMPIRE1: u32 = 17;
const FUEL_UP0: u32 = 18;
const FUEL_UP1: u32 = 19;
const WHAT_KIND_OF_IDIOT: u32 = 20;
const DONT_KNOW_WHO_YOU_ARE: u32 = 21;
const THATS_IMPOSSIBLE: u32 = 22;
const ASK_AWAY: u32 = 23;
const RADIOS_ON_MERCURY: u32 = 24;
const THANKS_FOR_HELPING: u32 = 25;
const STARBASE_IS: u32 = 26;
const HAPPENED_TO_EARTH: u32 = 27;
const URQUAN_LEFT: u32 = 28;
const BASE_ON_MOON: u32 = 29;
const ACKNOWLEDGE_SECRET: u32 = 30;
const ABOUT_BASE: u32 = 31;
const GOOD_LUCK_WITH_BASE: u32 = 32;
const DEALT_WITH_BASE_YET: u32 = 33;
const HERE_COMES_ILWRATH: u32 = 34;
const VERY_IMPRESSIVE: u32 = 35;
const IT_WAS_ABANDONED: u32 = 36;
const YOU_REALLY_FOUGHT_BASE: u32 = 37;
const IM_GLAD_YOU_WON: u32 = 38;
const IM_SURE_IT_WAS_DIFFICULT: u32 = 39;
const THAT_WAS_PROBE: u32 = 40;
const DEEP_TROUBLE: u32 = 41;
const GOOD_NEWS: u32 = 42;
const SURE_HOPE: u32 = 43;
const ABOUT_BASE_AGAIN: u32 = 44;
const COOK_BUTTS: u32 = 45;
const OVERTHROW_ALIENS: u32 = 46;
const KILL_MONSTERS: u32 = 47;
const GOOD_LUCK_AGAIN: u32 = 48;
const STARBASE_WILL_BE_READY: u32 = 49;
const OVERTHROW_EVIL_ALIENS: u32 = 50;
const ANNIHILATE_THOSE_MONSTERS: u32 = 51;
const COOK_THEIR_BUTTS: u32 = 52;
const WHERE_GET_RADIOS: u32 = 53;
const WELL_GO_GET_THEM_NOW: u32 = 54;
const WE_WILL_TRANSFER_NOW: u32 = 55;
const WHAT_WILL_YOU_GIVE_US: u32 = 56;
const BEFORE_RADIOS_WE_NEED_INFO: u32 = 57;
const NO_BUT_WELL_HELP0: u32 = 58;
const NO_BUT_WELL_HELP1: u32 = 59;
const YES_THIS_IS_SUPPLY_SHIP: u32 = 60;
const WHAT_SLAVE_PLANET: u32 = 61;
const I_LIED: u32 = 62;
const PLUMB_OUT: u32 = 63;
const WE_ARE_VINDICATOR0: u32 = 64;
const WE_ARE_VINDICATOR1: u32 = 65;
const WE_ARE_VINDICATOR2: u32 = 66;
const FIRST_GIVE_INFO: u32 = 67;
const WE_MUST_GO_NOW: u32 = 68;
const WHERE_CAN_I_GET_RADIOS: u32 = 69;
const OK_I_WILL_GET_RADIOS: u32 = 70;
const WHATS_THIS_STARBASE: u32 = 71;
const WHAT_ABOUT_EARTH: u32 = 72;
const WHERE_ARE_URQUAN: u32 = 73;
const OUR_MISSION_WAS_SECRET: u32 = 74;
const WE_ARE_HERE_TO_HELP: u32 = 75;
const TELL_ME_ABOUT_BASE: u32 = 76;
const WE_WILL_TAKE_CARE_OF_BASE: u32 = 77;
const TELL_ME_AGAIN: u32 = 78;
const BASE_WAS_ABANDONED: u32 = 79;
const WE_FOUGHT_THEM: u32 = 80;
const OH_YES_BIG_FIGHT: u32 = 81;
const I_LIED_IT_WAS_ABANDONED: u32 = 82;
const I_CANT_TALK_ABOUT_IT: u32 = 83;
const NAME_1: u32 = 84;
const NAME_2: u32 = 85;
const NAME_3: u32 = 86;
const NAME_40: u32 = 87;
const NAME_41: u32 = 88;
const I_LOST_MY_LANDER: u32 = 89;
const I_LOST_ANOTHER_LANDER: u32 = 90;
const NEED_FUEL_MERCURY: u32 = 91;
const NEED_FUEL_LUNA: u32 = 92;
const NEED_FUEL_AGAIN: u32 = 93;
const WHAT_WAS_RED_THING: u32 = 94;
const IT_WENT_AWAY: u32 = 95;
const WE_DESTROYED_IT: u32 = 96;
const WHAT_PROBE: u32 = 97;
const TAKE_CARE_OF_BASE_AGAIN: u32 = 98;
const GOODBYE_COMMANDER: u32 = 99;

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

const RACE_PMAP_ANIM: &[u8] = b"comandr\0";
const RACE_FONT: &[u8] = b"comandrfont\0";
const RACE_COLOR_MAP: &[u8] = b"comandrcolr\0";
const RACE_MUSIC: &[u8] = b"comandrmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.comandr.dialogue\0";

/// Comandr race dialogue implementation.
pub struct ComandrDialogue;

impl super::RaceDialogue for ComandrDialogue {
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
            bit_range("CHMMR_BOMB_STATE").is_some(),
            "missing game state key: CHMMR_BOMB_STATE"
        );
        assert!(
            bit_range("GIVEN_FUEL_BEFORE").is_some(),
            "missing game state key: GIVEN_FUEL_BEFORE"
        );
        assert!(
            bit_range("LANDERS_LOST").is_some(),
            "missing game state key: LANDERS_LOST"
        );
        assert!(
            bit_range("MOONBASE_DESTROYED").is_some(),
            "missing game state key: MOONBASE_DESTROYED"
        );
        assert!(
            bit_range("NEW_ALLIANCE_NAME").is_some(),
            "missing game state key: NEW_ALLIANCE_NAME"
        );
    }
}
