//! Druuge dialogue state machine — ported from C.
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
const AMBUSH_IS_FIRST_HELLO: u32 = 1;
const INIT_BOMB_WORLD_HELLO: u32 = 2;
const SUBSEQ_BOMB_WORLD_HELLO: u32 = 3;
const WHATS_UP_AT_BOMB_PLANET: u32 = 4;
const GEN_INFO_AT_BOMB_PLANET: u32 = 5;
const WE_GET_BOMB: u32 = 6;
const NOT_GET_BOMB: u32 = 7;
const THEN_WE_TAKE_BOMB: u32 = 8;
const FIGHT_FOR_BOMB: u32 = 9;
const GOODBYE_FROM_BOMB_PLANET: u32 = 10;
const NOT_ENOUGH_ROOM: u32 = 11;
const TRADE_FOR_SPHERE: u32 = 12;
const NO_WAY: u32 = 13;
const OK_REGULAR_DEAL: u32 = 14;
const WAY: u32 = 15;
const OK_HERES_SPHERE: u32 = 16;
const WHATS_THE_SPHERE_AGAIN: u32 = 17;
const SPHERE_IS: u32 = 18;
const WE_SELL_FOR_CREW: u32 = 19;
const I_WILL_NEVER_TRADE_CREW: u32 = 20;
const YOUR_LOSS: u32 = 21;
const ISNT_THIS_SLAVE_TRADING: u32 = 22;
const NO_SLAVE_TRADE: u32 = 23;
const WHAT_DO_WITH_CREW: u32 = 24;
const HAVE_FUN: u32 = 25;
const IM_READY_TO_BUY: u32 = 26;
const THIS_FOR_SALE: u32 = 27;
const HAVE_SPHERE: u32 = 28;
const HAVE_ART_2: u32 = 29;
const HAVE_ART_1: u32 = 30;
const SHIPS_AND_FUEL: u32 = 31;
const BOUGHT_SHIP: u32 = 32;
const BOUGHT_FUEL: u32 = 33;
const BOUGHT_ART_2: u32 = 34;
const BOUGHT_ART_1: u32 = 35;
const BOUGHT_SPHERE: u32 = 36;
const REPEAT_WHAT_TO_SELL: u32 = 37;
const INIT_SPACE_HELLO: u32 = 38;
const SUBSEQUENT_SPACE_HELLO: u32 = 39;
const WHATS_UP_IN_SPACE: u32 = 40;
const GENERAL_INFO_IN_SPACE_1: u32 = 41;
const GENERAL_INFO_IN_SPACE_2: u32 = 42;
const GENERAL_INFO_IN_SPACE_3: u32 = 43;
const GENERAL_INFO_IN_SPACE_4: u32 = 44;
const GOODBYE_FROM_SPACE: u32 = 45;
const HSTL_TRADE_WORLD_HELLO_1: u32 = 46;
const HSTL_TRADE_WORLD_HELLO_2: u32 = 47;
const HOSTILE_SPACE_HELLO_1: u32 = 48;
const HOSTILE_SPACE_HELLO_2: u32 = 49;
const INITIAL_TRADE_WORLD_HELLO: u32 = 50;
const SSQ_TRADE_WORLD_HELLO_1: u32 = 51;
const SSQ_TRADE_WORLD_HELLO_2: u32 = 52;
const SSQ_TRADE_WORLD_HELLO_3: u32 = 53;
const SSQ_TRADE_WORLD_HELLO_4: u32 = 54;
const WHATS_UP_AT_TRADE_WORLD: u32 = 55;
const GEN_INFO_AT_TRADE_WORLD_1: u32 = 56;
const GEN_INFO_AT_TRADE_WORLD_2: u32 = 57;
const GEN_INFO_AT_TRADE_WORLD_3: u32 = 58;
const GEN_INFO_AT_TRADE_WORLD_4: u32 = 59;
const SCAN_MAIDENS: u32 = 60;
const SCAN_FRAGMENTS: u32 = 61;
const SCAN_DRUUGE_CASTER: u32 = 62;
const SCAN_ARILOU_SPAWNER: u32 = 63;
const ENOUGH_FRAGMENTS: u32 = 64;
const READY_TO_BUY: u32 = 65;
const READY_TO_SELL: u32 = 66;
const BYE_FROM_TRADE_WORLD_1: u32 = 67;
const BYE_FROM_TRADE_WORLD_2: u32 = 68;
const NOT_ENOUGH_CREW: u32 = 69;
const EXCHANGE_MADE: u32 = 70;
const OK_DONE_BUYING: u32 = 71;
const OK_DONE_SELLING: u32 = 72;
const BYE: u32 = 73;
const WANT_TO_SELL: u32 = 74;
const WANT_TO_BUY: u32 = 75;
const BUY_DRUUGE_SHIP: u32 = 76;
const BUY_FUEL: u32 = 77;
const BUY_ART_1: u32 = 78;
const BUY_ART_2: u32 = 79;
const BUY_ROSY_SPHERE: u32 = 80;
const DONE_BUYING: u32 = 81;
const DONE_SELLING: u32 = 82;
const SELL_MAIDENS: u32 = 83;
const SELL_CASTER: u32 = 84;
const SELL_FRAGMENTS: u32 = 85;
const SELL_SPAWNER: u32 = 86;
const BOUGHT_MAIDENS: u32 = 87;
const BOUGHT_FRAGMENTS: u32 = 88;
const BOUGHT_CASTER: u32 = 89;
const YOU_GET: u32 = 90;
const YOU_ALSO_GET: u32 = 91;
const BOUGHT_SPAWNER: u32 = 92;
const SALVAGE_YOUR_SHIP_1: u32 = 93;
const SALVAGE_YOUR_SHIP_2: u32 = 94;
const DEAL_FOR_STATED_SHIPS: u32 = 95;
const DEAL_FOR_LESS_SHIPS: u32 = 96;
const DEAL_FOR_NO_SHIPS: u32 = 97;
const FUEL0: u32 = 98;
const FUEL1: u32 = 99;
const HIDEOUS_DEAL: u32 = 100;
const BAD_DEAL: u32 = 101;
const FAIR_DEAL: u32 = 102;
const GOOD_DEAL: u32 = 103;
const FINE_DEAL: u32 = 104;
const OUT_TAKES: u32 = 105;

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

const RACE_PMAP_ANIM: &[u8] = b"comm.druuge.graphics\0";
const RACE_FONT: &[u8] = b"comm.druuge.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.druuge.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.druuge.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.druuge.dialogue\0";

/// Druuge race dialogue implementation.
pub struct DruugeDialogue;

impl super::RaceDialogue for DruugeDialogue {
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
            bit_range("ARTIFACT_2_ON_SHIP").is_some(),
            "missing game state key: ARTIFACT_2_ON_SHIP"
        );
        assert!(
            bit_range("ARTIFACT_3_ON_SHIP").is_some(),
            "missing game state key: ARTIFACT_3_ON_SHIP"
        );
        assert!(
            bit_range("ATTACKED_DRUUGE").is_some(),
            "missing game state key: ATTACKED_DRUUGE"
        );
        assert!(
            bit_range("BOMB_VISITS").is_some(),
            "missing game state key: BOMB_VISITS"
        );
        assert!(
            bit_range("BURV_BROADCASTERS_ON_SHIP").is_some(),
            "missing game state key: BURV_BROADCASTERS_ON_SHIP"
        );
    }
}
