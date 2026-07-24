//! Melnorm dialogue state machine — ported from C.
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
const HELLO_NOW_DOWN_TO_BUSINESS_1: u32 = 1;
const HELLO_NOW_DOWN_TO_BUSINESS_2: u32 = 2;
const HELLO_NOW_DOWN_TO_BUSINESS_3: u32 = 3;
const KNOW_OF_YOU: u32 = 4;
const HOW_KNOW: u32 = 5;
const KNOW_BECAUSE: u32 = 6;
const WHAT_ABOUT_YOURSELVES: u32 = 7;
const NO_TALK_ABOUT_OURSELVES: u32 = 8;
const WHAT_FACTORS: u32 = 9;
const FACTORS_ARE: u32 = 10;
const WHAT_ABOUT_UNIVERSE: u32 = 11;
const NO_FREE_LUNCH: u32 = 12;
const GIVING_IS_GOOD_1: u32 = 13;
const GIVING_IS_BAD_1: u32 = 14;
const GIVING_IS_GOOD_2: u32 = 15;
const GIVING_IS_BAD_2: u32 = 16;
const GET_ON_WITH_BUSINESS: u32 = 17;
const TRADE_IS_FOR_THE_WEAK: u32 = 18;
const WERE_NOT_AFRAID: u32 = 19;
const NO_TRADE_NOW: u32 = 20;
const OK_NO_TRADE_NOW_BYE: u32 = 21;
const HELLO_AND_DOWN_TO_BUSINESS_1: u32 = 22;
const HELLO_AND_DOWN_TO_BUSINESS_2: u32 = 23;
const HELLO_AND_DOWN_TO_BUSINESS_3: u32 = 24;
const HELLO_AND_DOWN_TO_BUSINESS_4: u32 = 25;
const HELLO_AND_DOWN_TO_BUSINESS_5: u32 = 26;
const HELLO_AND_DOWN_TO_BUSINESS_6: u32 = 27;
const HELLO_AND_DOWN_TO_BUSINESS_7: u32 = 28;
const HELLO_AND_DOWN_TO_BUSINESS_8: u32 = 29;
const HELLO_AND_DOWN_TO_BUSINESS_9: u32 = 30;
const HELLO_AND_DOWN_TO_BUSINESS_10: u32 = 31;
const WHATS_MY_CREDIT: u32 = 32;
const HELLO_SLIGHTLY_ANGRY_1: u32 = 33;
const HELLO_SLIGHTLY_ANGRY_2: u32 = 34;
const HELLO_SLIGHTLY_ANGRY_3: u32 = 35;
const EXPLORE_RELATIONSHIP: u32 = 36;
const EXAMPLE_OF_RELATIONSHIP: u32 = 37;
const EXCUSE_1: u32 = 38;
const NO_EXCUSE_1: u32 = 39;
const EXCUSE_2: u32 = 40;
const NO_EXCUSE_2: u32 = 41;
const EXCUSE_3: u32 = 42;
const NO_EXCUSE_3: u32 = 43;
const WE_APOLOGIZE: u32 = 44;
const APOLOGY_ACCEPTED: u32 = 45;
const SO_WE_CAN_ATTACK: u32 = 46;
const DECEITFUL_HUMAN: u32 = 47;
const BYE_MELNORME_SLIGHTLY_ANGRY: u32 = 48;
const MELNORME_SLIGHTLY_ANGRY_GOODBYE: u32 = 49;
const HELLO_HATE_YOU_1: u32 = 50;
const HELLO_HATE_YOU_2: u32 = 51;
const HELLO_HATE_YOU_3: u32 = 52;
const WELL_IF_THATS_THE_WAY_YOU_FEEL: u32 = 53;
const YOU_HATE_US_SO_WE_GO_AWAY: u32 = 54;
const HATE_YOU_GOODBYE: u32 = 55;
const WE_FIGHT_AGAIN: u32 = 56;
const RESCUE_EXPLANATION: u32 = 57;
const RESCUE_AGAIN_1: u32 = 58;
const RESCUE_AGAIN_2: u32 = 59;
const RESCUE_AGAIN_3: u32 = 60;
const RESCUE_AGAIN_4: u32 = 61;
const RESCUE_AGAIN_5: u32 = 62;
const CHANGED_MIND: u32 = 63;
const NO_CHANGED_MIND: u32 = 64;
const YES_CHANGED_MIND: u32 = 65;
const SHOULD_WE_HELP_YOU: u32 = 66;
const YES_HELP: u32 = 67;
const NO_HELP: u32 = 68;
const RESCUE_OFFER: u32 = 69;
const RESCUE_TANKS: u32 = 70;
const RESCUE_HOME: u32 = 71;
const TAKE_IT: u32 = 72;
const LEAVE_IT: u32 = 73;
const HAPPY_TO_HAVE_RESCUED: u32 = 74;
const MAYBE_SEE_YOU_LATER: u32 = 75;
const GOODBYE_AND_GOODLUCK: u32 = 76;
const GOODBYE_AND_GOODLUCK_AGAIN: u32 = 77;
const HELLO_PISSED_OFF_1: u32 = 78;
const HELLO_PISSED_OFF_2: u32 = 79;
const HELLO_PISSED_OFF_3: u32 = 80;
const BEG_FORGIVENESS: u32 = 81;
const LOTS_TO_MAKE_UP_FOR: u32 = 82;
const YOU_ARE_SO_RIGHT: u32 = 83;
const ONE_LAST_CHANCE: u32 = 84;
const OK_STRIP_ME: u32 = 85;
const NO_STRIP_NOW: u32 = 86;
const NOT_WORTH_STRIPPING: u32 = 87;
const FAIR_JUSTICE: u32 = 88;
const BYE_MELNORME_PISSED_OFF: u32 = 89;
const MELNORME_PISSED_OFF_GOODBYE: u32 = 90;
const FIGHT_SOME_MORE: u32 = 91;
const OK_FIGHT_SOME_MORE: u32 = 92;
const WHY_BLUE_LIGHT: u32 = 93;
const BLUE_IS_MAD: u32 = 94;
const WE_STRONG_1: u32 = 95;
const YOU_NOT_STRONG_1: u32 = 96;
const WE_STRONG_2: u32 = 97;
const YOU_NOT_STRONG_2: u32 = 98;
const WE_STRONG_3: u32 = 99;
const YOU_NOT_STRONG_3: u32 = 100;
const JUST_TESTING: u32 = 101;
const REALLY_TESTING: u32 = 102;
const YES_REALLY_TESTING: u32 = 103;
const TEST_RESULTS: u32 = 104;
const YOURE_ON: u32 = 105;
const YOU_GIVE_US_NO_CHOICE: u32 = 106;
const TRADING_INFO: u32 = 107;
const BUY_OR_SELL: u32 = 108;
const GOODBYE: u32 = 109;
const WHY_TURNED_PURPLE: u32 = 110;
const BUY: u32 = 111;
const SELL: u32 = 112;
const TURNED_PURPLE_BECAUSE: u32 = 113;
const NOTHING_TO_SELL: u32 = 114;
const WHAT_TO_SELL: u32 = 115;
const OK_DONE_SELLING: u32 = 116;
const SELL_LIFE_DATA: u32 = 117;
const SOLD_LIFE_DATA1: u32 = 118;
const SOLD_LIFE_DATA2: u32 = 119;
const SOLD_LIFE_DATA3: u32 = 120;
const SELL_RAINBOW_LOCATIONS: u32 = 121;
const SOLD_RAINBOW_LOCATIONS1: u32 = 122;
const SOLD_RAINBOW_LOCATIONS2: u32 = 123;
const SOLD_RAINBOW_LOCATIONS3: u32 = 124;
const SELL_PRECURSOR_FIND: u32 = 125;
const SOLD_PRECURSOR_FIND: u32 = 126;
const CHANGED_MIND_NO_SELL: u32 = 127;
const DONE_SELLING: u32 = 128;
const NEED_CREDIT: u32 = 129;
const WHAT_TO_BUY: u32 = 130;
const WHAT_MORE_TO_BUY: u32 = 131;
const OK_DONE_BUYING: u32 = 132;
const BUY_FUEL: u32 = 133;
const DONE_BUYING: u32 = 134;
const BE_LEAVING_NOW: u32 = 135;
const HOW_MUCH_FUEL: u32 = 136;
const BUY_1_FUEL: u32 = 137;
const GOT_FUEL: u32 = 138;
const BUY_5_FUEL: u32 = 139;
const BUY_10_FUEL: u32 = 140;
const BUY_25_FUEL: u32 = 141;
const DONE_BUYING_FUEL: u32 = 142;
const FRIENDLY_GOODBYE: u32 = 143;
const CREDIT_IS0: u32 = 144;
const CREDIT_IS1: u32 = 145;
const NEED_MORE_CREDIT0: u32 = 146;
const NEED_MORE_CREDIT1: u32 = 147;
const BUY_FUEL_INTRO: u32 = 148;
const NO_ROOM_FOR_FUEL: u32 = 149;
const BUY_INFO: u32 = 150;
const BUY_TECHNOLOGY: u32 = 151;
const BUY_CURRENT_EVENTS: u32 = 152;
const BUY_ALIEN_RACES: u32 = 153;
const BUY_HISTORY: u32 = 154;
const DONE_BUYING_INFO: u32 = 155;
const NO_BUY_INFO: u32 = 156;
const BUY_INFO_INTRO: u32 = 157;
const OK_BUY_INFO: u32 = 158;
const OK_NO_BUY_INFO: u32 = 159;
const OK_DONE_BUYING_INFO: u32 = 160;
const OK_BUY_EVENT_1: u32 = 161;
const OK_BUY_EVENT_2: u32 = 162;
const OK_BUY_EVENT_3: u32 = 163;
const OK_BUY_EVENT_4: u32 = 164;
const OK_BUY_EVENT_5: u32 = 165;
const OK_BUY_EVENT_6: u32 = 166;
const OK_BUY_EVENT_7: u32 = 167;
const OK_BUY_EVENT_8: u32 = 168;
const OK_BUY_ALIEN_RACE_1: u32 = 169;
const OK_BUY_ALIEN_RACE_2: u32 = 170;
const OK_BUY_ALIEN_RACE_3: u32 = 171;
const OK_BUY_ALIEN_RACE_4: u32 = 172;
const OK_BUY_ALIEN_RACE_5: u32 = 173;
const OK_BUY_ALIEN_RACE_6: u32 = 174;
const OK_BUY_ALIEN_RACE_7: u32 = 175;
const OK_BUY_ALIEN_RACE_8: u32 = 176;
const OK_BUY_ALIEN_RACE_9: u32 = 177;
const OK_BUY_ALIEN_RACE_10: u32 = 178;
const OK_BUY_ALIEN_RACE_11: u32 = 179;
const OK_BUY_ALIEN_RACE_12: u32 = 180;
const OK_BUY_ALIEN_RACE_13: u32 = 181;
const OK_BUY_ALIEN_RACE_14: u32 = 182;
const OK_BUY_ALIEN_RACE_15: u32 = 183;
const OK_BUY_ALIEN_RACE_16: u32 = 184;
const OK_BUY_HISTORY_1: u32 = 185;
const OK_BUY_HISTORY_2: u32 = 186;
const OK_BUY_HISTORY_3: u32 = 187;
const OK_BUY_HISTORY_4: u32 = 188;
const OK_BUY_HISTORY_5: u32 = 189;
const OK_BUY_HISTORY_6: u32 = 190;
const OK_BUY_HISTORY_7: u32 = 191;
const OK_BUY_HISTORY_8: u32 = 192;
const OK_BUY_HISTORY_9: u32 = 193;
const INFO_ALL_GONE: u32 = 194;
const BUY_NEW_TECH: u32 = 195;
const NO_BUY_NEW_TECH: u32 = 196;
const DONE_BUYING_NEW_TECH: u32 = 197;
const FILL_ME_UP: u32 = 198;
const OK_FILL_YOU_UP: u32 = 199;
const BUY_NEW_TECH_INTRO: u32 = 200;
const OK_BUY_NEW_TECH: u32 = 201;
const OK_NO_BUY_NEW_TECH: u32 = 202;
const OK_DONE_BUYING_NEW_TECH: u32 = 203;
const OK_DONE_BUYING_FUEL: u32 = 204;
const NEW_TECH_1: u32 = 205;
const NEW_TECH_2: u32 = 206;
const NEW_TECH_3: u32 = 207;
const NEW_TECH_4: u32 = 208;
const NEW_TECH_5: u32 = 209;
const NEW_TECH_6: u32 = 210;
const NEW_TECH_7: u32 = 211;
const NEW_TECH_8: u32 = 212;
const NEW_TECH_9: u32 = 213;
const NEW_TECH_10: u32 = 214;
const NEW_TECH_11: u32 = 215;
const NEW_TECH_12: u32 = 216;
const NEW_TECH_13: u32 = 217;
const OK_BUY_NEW_TECH_1: u32 = 218;
const OK_BUY_NEW_TECH_2: u32 = 219;
const OK_BUY_NEW_TECH_3: u32 = 220;
const OK_BUY_NEW_TECH_4: u32 = 221;
const OK_BUY_NEW_TECH_5: u32 = 222;
const OK_BUY_NEW_TECH_6: u32 = 223;
const OK_BUY_NEW_TECH_7: u32 = 224;
const OK_BUY_NEW_TECH_8: u32 = 225;
const OK_BUY_NEW_TECH_9: u32 = 226;
const OK_BUY_NEW_TECH_10: u32 = 227;
const OK_BUY_NEW_TECH_11: u32 = 228;
const OK_BUY_NEW_TECH_12: u32 = 229;
const OK_BUY_NEW_TECH_13: u32 = 230;
const CHARITY: u32 = 231;
const NEW_TECH_ALL_GONE: u32 = 232;
const WE_ARE_FROM_ALLIANCE0: u32 = 233;
const STRIP_HEAD: u32 = 234;
const LANDERS: u32 = 235;
const THRUSTERS: u32 = 236;
const JETS: u32 = 237;
const PODS: u32 = 238;
const BAYS: u32 = 239;
const DYNAMOS: u32 = 240;
const FURNACES: u32 = 241;
const GUNS: u32 = 242;
const BLASTERS: u32 = 243;
const CANNONS: u32 = 244;
const TRACKERS: u32 = 245;
const DEFENSES: u32 = 246;
const NAME_1: u32 = 247;
const NAME_2: u32 = 248;
const NAME_3: u32 = 249;
const NAME_40: u32 = 250;
const NAME_41: u32 = 251;
const ENUMERATE_ONE: u32 = 252;
const ENUMERATE_TWO: u32 = 253;
const ENUMERATE_THREE: u32 = 254;
const ENUMERATE_FOUR: u32 = 255;
const ENUMERATE_FIVE: u32 = 256;
const ENUMERATE_SIX: u32 = 257;
const ENUMERATE_SEVEN: u32 = 258;
const ENUMERATE_EIGHT: u32 = 259;
const ENUMERATE_NINE: u32 = 260;
const ENUMERATE_TEN: u32 = 261;
const ENUMERATE_ELEVEN: u32 = 262;
const ENUMERATE_TWELVE: u32 = 263;
const ENUMERATE_THIRTEEN: u32 = 264;
const ENUMERATE_FOURTEEN: u32 = 265;
const ENUMERATE_FIFTEEN: u32 = 266;
const ENUMERATE_SIXTEEN: u32 = 267;
const END_LIST_WITH_AND: u32 = 268;
const ENUMERATE_ZERO: u32 = 269;
const ENUMERATE_SEVENTEEN: u32 = 270;
const ENUMERATE_EIGHTEEN: u32 = 271;
const ENUMERATE_NINETEEN: u32 = 272;
const ENUMERATE_TWENTY: u32 = 273;
const ENUMERATE_THIRTY: u32 = 274;
const ENUMERATE_FOURTY: u32 = 275;
const ENUMERATE_FIFTY: u32 = 276;
const ENUMERATE_SIXTY: u32 = 277;
const ENUMERATE_SEVENTY: u32 = 278;
const ENUMERATE_EIGHTY: u32 = 279;
const ENUMERATE_NINETY: u32 = 280;
const ENUMERATE_HUNDRED: u32 = 281;
const ENUMERATE_THOUSAND: u32 = 282;

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

const RACE_PMAP_ANIM: &[u8] = b"melnorm\0";
const RACE_FONT: &[u8] = b"melnormfont\0";
const RACE_COLOR_MAP: &[u8] = b"melnormcolr\0";
const RACE_MUSIC: &[u8] = b"melnormmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.melnorm.dialogue\0";

/// Melnorm race dialogue implementation.
pub struct MelnormDialogue;

impl super::RaceDialogue for MelnormDialogue {
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
            bit_range("FOUND_PLUTO_SPATHI").is_some(),
            "missing game state key: FOUND_PLUTO_SPATHI"
        );
        assert!(
            bit_range("IMPROVED_LANDER_CARGO").is_some(),
            "missing game state key: IMPROVED_LANDER_CARGO"
        );
        assert!(
            bit_range("IMPROVED_LANDER_SHOT").is_some(),
            "missing game state key: IMPROVED_LANDER_SHOT"
        );
        assert!(
            bit_range("IMPROVED_LANDER_SPEED").is_some(),
            "missing game state key: IMPROVED_LANDER_SPEED"
        );
        assert!(
            bit_range("KNOW_ABOUT_SHATTERED").is_some(),
            "missing game state key: KNOW_ABOUT_SHATTERED"
        );
    }
}
