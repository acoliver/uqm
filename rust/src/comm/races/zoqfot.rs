//! Zoqfot dialogue state machine — ported from C.
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
const WE_ARE0: u32 = 1;
const WE_ARE1: u32 = 2;
const WE_ARE2: u32 = 3;
const WE_ARE3: u32 = 4;
const WE_ARE4: u32 = 5;
const WE_ARE5: u32 = 6;
const WE_ARE6: u32 = 7;
const WE_ARE7: u32 = 8;
const SCOUT_HELLO0: u32 = 9;
const SCOUT_HELLO1: u32 = 10;
const SCOUT_HELLO2: u32 = 11;
const SCOUT_HELLO3: u32 = 12;
const INIT_HOME_HELLO0: u32 = 13;
const INIT_HOME_HELLO1: u32 = 14;
const INIT_HOME_HELLO2: u32 = 15;
const INIT_HOME_HELLO3: u32 = 16;
const WHICH_FOT: u32 = 17;
const HE_IS0: u32 = 18;
const HE_IS1: u32 = 19;
const HE_IS2: u32 = 20;
const HE_IS3: u32 = 21;
const HE_IS4: u32 = 22;
const HE_IS5: u32 = 23;
const HE_IS6: u32 = 24;
const HE_IS7: u32 = 25;
const WE_ARE_VINDICATOR0: u32 = 26;
const WE_ARE_VINDICATOR1: u32 = 27;
const WE_ARE_VINDICATOR2: u32 = 28;
const WE_GLAD0: u32 = 29;
const WE_GLAD1: u32 = 30;
const WE_GLAD2: u32 = 31;
const WE_GLAD3: u32 = 32;
const WE_GLAD4: u32 = 33;
const WE_GLAD5: u32 = 34;
const QUIET_TOADIES: u32 = 35;
const TOLD_YOU0: u32 = 36;
const TOLD_YOU1: u32 = 37;
const TOLD_YOU2: u32 = 38;
const TOLD_YOU3: u32 = 39;
const TOLD_YOU4: u32 = 40;
const TOLD_YOU5: u32 = 41;
const TOLD_YOU6: u32 = 42;
const TOLD_YOU7: u32 = 43;
const YOUR_RACE: u32 = 44;
const YEARS_AGO0: u32 = 45;
const YEARS_AGO1: u32 = 46;
const YEARS_AGO2: u32 = 47;
const YEARS_AGO3: u32 = 48;
const YEARS_AGO4: u32 = 49;
const YEARS_AGO5: u32 = 50;
const YEARS_AGO6: u32 = 51;
const YEARS_AGO7: u32 = 52;
const YEARS_AGO8: u32 = 53;
const YEARS_AGO9: u32 = 54;
const YEARS_AGO10: u32 = 55;
const YEARS_AGO11: u32 = 56;
const YEARS_AGO12: u32 = 57;
const YEARS_AGO13: u32 = 58;
const WHERE_FROM: u32 = 59;
const TRAVELED_FAR0: u32 = 60;
const TRAVELED_FAR1: u32 = 61;
const TRAVELED_FAR2: u32 = 62;
const TRAVELED_FAR3: u32 = 63;
const TRAVELED_FAR4: u32 = 64;
const TRAVELED_FAR5: u32 = 65;
const WHAT_EMERGENCY: u32 = 66;
const UNDER_ATTACK0: u32 = 67;
const UNDER_ATTACK1: u32 = 68;
const UNDER_ATTACK2: u32 = 69;
const UNDER_ATTACK3: u32 = 70;
const UNDER_ATTACK4: u32 = 71;
const UNDER_ATTACK5: u32 = 72;
const UNDER_ATTACK6: u32 = 73;
const UNDER_ATTACK7: u32 = 74;
const UNDER_ATTACK8: u32 = 75;
const UNDER_ATTACK9: u32 = 76;
const UNDER_ATTACK10: u32 = 77;
const UNDER_ATTACK11: u32 = 78;
const TOUGH_LUCK: u32 = 79;
const NOT_HELPFUL0: u32 = 80;
const NOT_HELPFUL1: u32 = 81;
const NOT_HELPFUL2: u32 = 82;
const NOT_HELPFUL3: u32 = 83;
const NOT_HELPFUL4: u32 = 84;
const NOT_HELPFUL5: u32 = 85;
const WHAT_LOOK_LIKE: u32 = 86;
const LOOK_LIKE0: u32 = 87;
const LOOK_LIKE1: u32 = 88;
const LOOK_LIKE2: u32 = 89;
const LOOK_LIKE3: u32 = 90;
const VALUABLE_INFO: u32 = 91;
const GOODBYE0: u32 = 92;
const GOODBYE1: u32 = 93;
const GOODBYE2: u32 = 94;
const GOODBYE3: u32 = 95;
const ALL_VERY_INTERESTING: u32 = 96;
const SEE_TOLD_YOU0: u32 = 97;
const SEE_TOLD_YOU1: u32 = 98;
const SEE_TOLD_YOU2: u32 = 99;
const SEE_TOLD_YOU3: u32 = 100;
const HOW_CAN_I_HELP: u32 = 101;
const ALLY_WITH_US0: u32 = 102;
const ALLY_WITH_US1: u32 = 103;
const ALLY_WITH_US2: u32 = 104;
const ALLY_WITH_US3: u32 = 105;
const ALLY_WITH_US4: u32 = 106;
const ALLY_WITH_US5: u32 = 107;
const DECIDE_LATER: u32 = 108;
const PLEASE_HURRY0: u32 = 109;
const PLEASE_HURRY1: u32 = 110;
const EMMISSARIES0: u32 = 111;
const EMMISSARIES1: u32 = 112;
const EMMISSARIES2: u32 = 113;
const EMMISSARIES3: u32 = 114;
const EMMISSARIES4: u32 = 115;
const EMMISSARIES5: u32 = 116;
const EMMISSARIES6: u32 = 117;
const EMMISSARIES7: u32 = 118;
const SURE: u32 = 119;
const WE_ALLY0: u32 = 120;
const WE_ALLY1: u32 = 121;
const WE_ALLY2: u32 = 122;
const WE_ALLY3: u32 = 123;
const WE_ALLY4: u32 = 124;
const WE_ALLY5: u32 = 125;
const NEVER: u32 = 126;
const WE_ENEMIES0: u32 = 127;
const WE_ENEMIES1: u32 = 128;
const HOSTILE_HELLO_10: u32 = 129;
const HOSTILE_HELLO_11: u32 = 130;
const HOSTILE_HELLO_20: u32 = 131;
const HOSTILE_HELLO_21: u32 = 132;
const HOSTILE_HELLO_22: u32 = 133;
const HOSTILE_HELLO_23: u32 = 134;
const HOSTILE_HELLO_24: u32 = 135;
const HOSTILE_HELLO_25: u32 = 136;
const HOSTILE_HELLO_30: u32 = 137;
const HOSTILE_HELLO_31: u32 = 138;
const HOSTILE_HELLO_40: u32 = 139;
const HOSTILE_HELLO_41: u32 = 140;
const NEUTRAL_HOME_HELLO_10: u32 = 141;
const NEUTRAL_HOME_HELLO_11: u32 = 142;
const NEUTRAL_HOME_HELLO_12: u32 = 143;
const NEUTRAL_HOME_HELLO_13: u32 = 144;
const NEUTRAL_HOME_HELLO_20: u32 = 145;
const NEUTRAL_HOME_HELLO_21: u32 = 146;
const NEUTRAL_HOME_HELLO_22: u32 = 147;
const NEUTRAL_HOME_HELLO_23: u32 = 148;
const ALLIED_HOME_HELLO_10: u32 = 149;
const ALLIED_HOME_HELLO_11: u32 = 150;
const ALLIED_HOME_HELLO_12: u32 = 151;
const ALLIED_HOME_HELLO_13: u32 = 152;
const ALLIED_HOME_HELLO_20: u32 = 153;
const ALLIED_HOME_HELLO_21: u32 = 154;
const ALLIED_HOME_HELLO_22: u32 = 155;
const ALLIED_HOME_HELLO_23: u32 = 156;
const ALLIED_HOME_HELLO_24: u32 = 157;
const ALLIED_HOME_HELLO_25: u32 = 158;
const ALLIED_HOME_HELLO_26: u32 = 159;
const ALLIED_HOME_HELLO_27: u32 = 160;
const ALLIED_HOME_HELLO_30: u32 = 161;
const ALLIED_HOME_HELLO_31: u32 = 162;
const ALLIED_HOME_HELLO_40: u32 = 163;
const ALLIED_HOME_HELLO_41: u32 = 164;
const THANKS_FOR_RESCUE0: u32 = 165;
const THANKS_FOR_RESCUE1: u32 = 166;
const THANKS_FOR_RESCUE2: u32 = 167;
const THANKS_FOR_RESCUE3: u32 = 168;
const THANKS_FOR_RESCUE4: u32 = 169;
const THANKS_FOR_RESCUE5: u32 = 170;
const THANKS_FOR_RESCUE6: u32 = 171;
const THANKS_FOR_RESCUE7: u32 = 172;
const THANKS_FOR_RESCUE8: u32 = 173;
const THANKS_FOR_RESCUE9: u32 = 174;
const THANKS_FOR_RESCUE10: u32 = 175;
const THANKS_FOR_RESCUE11: u32 = 176;
const BYE_HOMEWORLD: u32 = 177;
const GOODBYE_HOME0: u32 = 178;
const GOODBYE_HOME1: u32 = 179;
const WHATS_UP_HOMEWORLD: u32 = 180;
const GENERAL_INFO_10: u32 = 181;
const GENERAL_INFO_11: u32 = 182;
const GENERAL_INFO_12: u32 = 183;
const GENERAL_INFO_13: u32 = 184;
const GENERAL_INFO_20: u32 = 185;
const GENERAL_INFO_21: u32 = 186;
const GENERAL_INFO_22: u32 = 187;
const GENERAL_INFO_23: u32 = 188;
const GENERAL_INFO_24: u32 = 189;
const GENERAL_INFO_25: u32 = 190;
const GENERAL_INFO_26: u32 = 191;
const GENERAL_INFO_27: u32 = 192;
const GENERAL_INFO_30: u32 = 193;
const GENERAL_INFO_31: u32 = 194;
const GENERAL_INFO_32: u32 = 195;
const GENERAL_INFO_33: u32 = 196;
const GENERAL_INFO_34: u32 = 197;
const GENERAL_INFO_35: u32 = 198;
const GENERAL_INFO_40: u32 = 199;
const GENERAL_INFO_41: u32 = 200;
const GENERAL_INFO_42: u32 = 201;
const GENERAL_INFO_43: u32 = 202;
const GENERAL_INFO_44: u32 = 203;
const GENERAL_INFO_45: u32 = 204;
const GENERAL_INFO_46: u32 = 205;
const GENERAL_INFO_47: u32 = 206;
const GENERAL_INFO_48: u32 = 207;
const GENERAL_INFO_49: u32 = 208;
const GENERAL_INFO_410: u32 = 209;
const GENERAL_INFO_411: u32 = 210;
const ANY_WAR_NEWS: u32 = 211;
const UTWIG_DELAY0: u32 = 212;
const UTWIG_DELAY1: u32 = 213;
const UTWIG_DELAY2: u32 = 214;
const UTWIG_DELAY3: u32 = 215;
const UTWIG_DELAY4: u32 = 216;
const UTWIG_DELAY5: u32 = 217;
const UTWIG_DELAY6: u32 = 218;
const UTWIG_DELAY7: u32 = 219;
const UTWIG_DELAY8: u32 = 220;
const UTWIG_DELAY9: u32 = 221;
const UTWIG_DELAY10: u32 = 222;
const UTWIG_DELAY11: u32 = 223;
const UTWIG_DELAY12: u32 = 224;
const UTWIG_DELAY13: u32 = 225;
const KOHRAH_WINNING0: u32 = 226;
const KOHRAH_WINNING1: u32 = 227;
const KOHRAH_WINNING2: u32 = 228;
const KOHRAH_WINNING3: u32 = 229;
const KOHRAH_WINNING4: u32 = 230;
const KOHRAH_WINNING5: u32 = 231;
const KOHRAH_WINNING6: u32 = 232;
const KOHRAH_WINNING7: u32 = 233;
const KOHRAH_WINNING8: u32 = 234;
const KOHRAH_WINNING9: u32 = 235;
const URQUAN_NEARLY_GONE0: u32 = 236;
const URQUAN_NEARLY_GONE1: u32 = 237;
const URQUAN_NEARLY_GONE2: u32 = 238;
const URQUAN_NEARLY_GONE3: u32 = 239;
const URQUAN_NEARLY_GONE4: u32 = 240;
const URQUAN_NEARLY_GONE5: u32 = 241;
const KOHRAH_FRENZY0: u32 = 242;
const KOHRAH_FRENZY1: u32 = 243;
const KOHRAH_FRENZY2: u32 = 244;
const KOHRAH_FRENZY3: u32 = 245;
const KOHRAH_FRENZY4: u32 = 246;
const KOHRAH_FRENZY5: u32 = 247;
const KOHRAH_FRENZY6: u32 = 248;
const KOHRAH_FRENZY7: u32 = 249;
const KOHRAH_FRENZY8: u32 = 250;
const KOHRAH_FRENZY9: u32 = 251;
const KOHRAH_FRENZY10: u32 = 252;
const KOHRAH_FRENZY11: u32 = 253;
const NO_WAR_NEWS0: u32 = 254;
const NO_WAR_NEWS1: u32 = 255;
const I_WANT_ALLIANCE: u32 = 256;
const GOOD0: u32 = 257;
const GOOD1: u32 = 258;
const GOOD2: u32 = 259;
const GOOD3: u32 = 260;
const GOOD4: u32 = 261;
const GOOD5: u32 = 262;
const GOOD6: u32 = 263;
const GOOD7: u32 = 264;
const GOOD8: u32 = 265;
const GOOD9: u32 = 266;
const WANT_SPECIFIC_INFO: u32 = 267;
const WHAT_SPECIFIC_INFO0: u32 = 268;
const WHAT_SPECIFIC_INFO1: u32 = 269;
const ENOUGH_INFO: u32 = 270;
const OK_ENOUGH_INFO: u32 = 271;
const WHAT_ABOUT_OTHERS: u32 = 272;
const ABOUT_OTHERS0: u32 = 273;
const ABOUT_OTHERS1: u32 = 274;
const ABOUT_OTHERS2: u32 = 275;
const ABOUT_OTHERS3: u32 = 276;
const ABOUT_OTHERS4: u32 = 277;
const ABOUT_OTHERS5: u32 = 278;
const ABOUT_OTHERS6: u32 = 279;
const ABOUT_OTHERS7: u32 = 280;
const ABOUT_OTHERS8: u32 = 281;
const ABOUT_OTHERS9: u32 = 282;
const ABOUT_OTHERS10: u32 = 283;
const ABOUT_OTHERS11: u32 = 284;
const ABOUT_OTHERS12: u32 = 285;
const ABOUT_OTHERS13: u32 = 286;
const WHAT_ABOUT_ZEBRANKY: u32 = 287;
const ABOUT_ZEBRANKY0: u32 = 288;
const ABOUT_ZEBRANKY1: u32 = 289;
const ABOUT_ZEBRANKY2: u32 = 290;
const ABOUT_ZEBRANKY3: u32 = 291;
const ABOUT_ZEBRANKY4: u32 = 292;
const ABOUT_ZEBRANKY5: u32 = 293;
const ABOUT_ZEBRANKY6: u32 = 294;
const ABOUT_ZEBRANKY7: u32 = 295;
const WHAT_ABOUT_PAST: u32 = 296;
const ABOUT_PAST0: u32 = 297;
const ABOUT_PAST1: u32 = 298;
const ABOUT_PAST2: u32 = 299;
const ABOUT_PAST3: u32 = 300;
const ABOUT_PAST4: u32 = 301;
const ABOUT_PAST5: u32 = 302;
const ABOUT_PAST6: u32 = 303;
const ABOUT_PAST7: u32 = 304;
const ABOUT_PAST8: u32 = 305;
const ABOUT_PAST9: u32 = 306;
const ABOUT_PAST10: u32 = 307;
const ABOUT_PAST11: u32 = 308;
const WHAT_ABOUT_STINGER: u32 = 309;
const ABOUT_STINGER0: u32 = 310;
const ABOUT_STINGER1: u32 = 311;
const ABOUT_STINGER2: u32 = 312;
const ABOUT_STINGER3: u32 = 313;
const ABOUT_STINGER4: u32 = 314;
const ABOUT_STINGER5: u32 = 315;
const WHAT_ABOUT_GUY_IN_BACK: u32 = 316;
const ABOUT_GUY0: u32 = 317;
const ABOUT_GUY1: u32 = 318;
const NAME_1: u32 = 319;
const NAME_2: u32 = 320;
const NAME_3: u32 = 321;
const NAME_40: u32 = 322;
const NAME_41: u32 = 323;
const OUT_TAKES0: u32 = 324;
const OUT_TAKES1: u32 = 325;
const OUT_TAKES2: u32 = 326;
const OUT_TAKES3: u32 = 327;
const OUT_TAKES4: u32 = 328;
const OUT_TAKES5: u32 = 329;
const OUT_TAKES6: u32 = 330;
const OUT_TAKES7: u32 = 331;
const OUT_TAKES8: u32 = 332;
const OUT_TAKES9: u32 = 333;
const OUT_TAKES10: u32 = 334;
const OUT_TAKES11: u32 = 335;
const OUT_TAKES12: u32 = 336;
const OUT_TAKES13: u32 = 337;

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

const RACE_PMAP_ANIM: &[u8] = b"zoqfot\0";
const RACE_FONT: &[u8] = b"zoqfotfont\0";
const RACE_COLOR_MAP: &[u8] = b"zoqfotcolr\0";
const RACE_MUSIC: &[u8] = b"zoqfotmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.zoqfot.dialogue\0";

/// Zoqfot race dialogue implementation.
pub struct ZoqfotDialogue;

impl super::RaceDialogue for ZoqfotDialogue {
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
            bit_range("KOHR_AH_FRENZY").is_some(),
            "missing game state key: KOHR_AH_FRENZY"
        );
        assert!(
            bit_range("MET_ZOQFOT").is_some(),
            "missing game state key: MET_ZOQFOT"
        );
        assert!(
            bit_range("UTWIG_SUPOX_MISSION").is_some(),
            "missing game state key: UTWIG_SUPOX_MISSION"
        );
        assert!(
            bit_range("ZOQFOT_DISTRESS").is_some(),
            "missing game state key: ZOQFOT_DISTRESS"
        );
    }
}
