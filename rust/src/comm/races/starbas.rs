//! Starbas dialogue state machine — ported from C.
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
const BEFORE_WE_GO_ON_1: u32 = 1;
const BEFORE_WE_GO_ON_2: u32 = 2;
const BEFORE_WE_GO_ON_3: u32 = 3;
const BEFORE_WE_GO_ON_4: u32 = 4;
const BEFORE_WE_GO_ON_5: u32 = 5;
const BEFORE_WE_GO_ON_6: u32 = 6;
const BEFORE_WE_GO_ON_7: u32 = 7;
const NORMAL_HELLO_A0: u32 = 8;
const NORMAL_HELLO_A1: u32 = 9;
const NORMAL_HELLO_B0: u32 = 10;
const NORMAL_HELLO_B1: u32 = 11;
const NORMAL_HELLO_C0: u32 = 12;
const NORMAL_HELLO_C1: u32 = 13;
const NORMAL_HELLO_D0: u32 = 14;
const NORMAL_HELLO_D1: u32 = 15;
const NORMAL_HELLO_E0: u32 = 16;
const NORMAL_HELLO_E1: u32 = 17;
const NORMAL_HELLO_F0: u32 = 18;
const NORMAL_HELLO_F1: u32 = 19;
const NORMAL_HELLO_G0: u32 = 20;
const NORMAL_HELLO_G1: u32 = 21;
const NORMAL_HELLO_H0: u32 = 22;
const NORMAL_HELLO_H1: u32 = 23;
const RETURN_HELLO: u32 = 24;
const NORMAL_HELLO_TAIL: u32 = 25;
const NORMAL_GOODBYE_A0: u32 = 26;
const NORMAL_GOODBYE_A1: u32 = 27;
const NORMAL_GOODBYE_B0: u32 = 28;
const NORMAL_GOODBYE_B1: u32 = 29;
const NORMAL_GOODBYE_C0: u32 = 30;
const NORMAL_GOODBYE_C1: u32 = 31;
const NORMAL_GOODBYE_D0: u32 = 32;
const NORMAL_GOODBYE_D1: u32 = 33;
const NORMAL_GOODBYE_E0: u32 = 34;
const NORMAL_GOODBYE_E1: u32 = 35;
const NORMAL_GOODBYE_F0: u32 = 36;
const NORMAL_GOODBYE_F1: u32 = 37;
const NORMAL_GOODBYE_G0: u32 = 38;
const NORMAL_GOODBYE_G1: u32 = 39;
const NORMAL_GOODBYE_H0: u32 = 40;
const NORMAL_GOODBYE_H1: u32 = 41;
const LIGHT_LOAD_A0: u32 = 42;
const LIGHT_LOAD_A1: u32 = 43;
const LIGHT_LOAD_B0: u32 = 44;
const LIGHT_LOAD_B1: u32 = 45;
const LIGHT_LOAD_C0: u32 = 46;
const LIGHT_LOAD_C1: u32 = 47;
const LIGHT_LOAD_D0: u32 = 48;
const LIGHT_LOAD_D1: u32 = 49;
const LIGHT_LOAD_E0: u32 = 50;
const LIGHT_LOAD_E1: u32 = 51;
const LIGHT_LOAD_F0: u32 = 52;
const LIGHT_LOAD_F1: u32 = 53;
const LIGHT_LOAD_G0: u32 = 54;
const LIGHT_LOAD_G1: u32 = 55;
const MEDIUM_LOAD_A0: u32 = 56;
const MEDIUM_LOAD_A1: u32 = 57;
const MEDIUM_LOAD_B0: u32 = 58;
const MEDIUM_LOAD_B1: u32 = 59;
const MEDIUM_LOAD_C0: u32 = 60;
const MEDIUM_LOAD_C1: u32 = 61;
const MEDIUM_LOAD_D0: u32 = 62;
const MEDIUM_LOAD_D1: u32 = 63;
const MEDIUM_LOAD_E0: u32 = 64;
const MEDIUM_LOAD_E1: u32 = 65;
const MEDIUM_LOAD_F0: u32 = 66;
const MEDIUM_LOAD_F1: u32 = 67;
const MEDIUM_LOAD_G0: u32 = 68;
const MEDIUM_LOAD_G1: u32 = 69;
const HEAVY_LOAD_A0: u32 = 70;
const HEAVY_LOAD_A1: u32 = 71;
const HEAVY_LOAD_B0: u32 = 72;
const HEAVY_LOAD_B1: u32 = 73;
const HEAVY_LOAD_C0: u32 = 74;
const HEAVY_LOAD_C1: u32 = 75;
const HEAVY_LOAD_D0: u32 = 76;
const HEAVY_LOAD_D1: u32 = 77;
const HEAVY_LOAD_E0: u32 = 78;
const HEAVY_LOAD_E1: u32 = 79;
const HEAVY_LOAD_F0: u32 = 80;
const HEAVY_LOAD_F1: u32 = 81;
const HEAVY_LOAD_G0: u32 = 82;
const HEAVY_LOAD_G1: u32 = 83;
const STARBASE_IS_READY_A: u32 = 84;
const STARBASE_IS_READY_B: u32 = 85;
const STARBASE_IS_READY_C: u32 = 86;
const WHAT_KIND_OF_INFO: u32 = 87;
const WHICH_FUNCTION: u32 = 88;
const WHICH_HISTORY: u32 = 89;
const WHICH_MISSION: u32 = 90;
const OK_NO_NEED_INFO: u32 = 91;
const ABOUT_FUEL: u32 = 92;
const ABOUT_MODULES: u32 = 93;
const ABOUT_CREW0: u32 = 94;
const ABOUT_CREW1: u32 = 95;
const ABOUT_SHIPS: u32 = 96;
const ABOUT_RU: u32 = 97;
const ABOUT_MINERALS: u32 = 98;
const ABOUT_LIFE: u32 = 99;
const OK_ENOUGH_STARBASE: u32 = 100;
const OK_ENOUGH_MISSION: u32 = 101;
const GET_MINERALS: u32 = 102;
const ABOUT_ALIENS: u32 = 103;
const MUST_DEFEAT: u32 = 104;
const DEFEAT_LIKE_SO: u32 = 105;
const FIND_URQUAN: u32 = 106;
const FIGHT_URQUAN: u32 = 107;
const ALLY_LIKE_SO: u32 = 108;
const STRONG_LIKE_SO: u32 = 109;
const OK_ENOUGH_DEFEAT: u32 = 110;
const WHICH_ALIEN: u32 = 111;
const WHICH_WAR: u32 = 112;
const WHICH_ANCIENT: u32 = 113;
const OK_ENOUGH_HISTORY: u32 = 114;
const WHICH_ALLIANCE: u32 = 115;
const WHICH_HIERARCHY: u32 = 116;
const ABOUT_OTHER: u32 = 117;
const OK_ENOUGH_ALIENS: u32 = 118;
const ABOUT_SHOFIXTI: u32 = 119;
const ABOUT_YEHAT: u32 = 120;
const ABOUT_ARILOU: u32 = 121;
const ABOUT_CHENJESU: u32 = 122;
const ABOUT_MMRNMHRM: u32 = 123;
const ABOUT_SYREEN: u32 = 124;
const OK_ENOUGH_ALLIANCE: u32 = 125;
const ABOUT_URQUAN: u32 = 126;
const ABOUT_MYCON: u32 = 127;
const ABOUT_SPATHI: u32 = 128;
const ABOUT_UMGAH: u32 = 129;
const ABOUT_ANDROSYNTH: u32 = 130;
const ABOUT_VUX: u32 = 131;
const ABOUT_ILWRATH: u32 = 132;
const OK_ENOUGH_HIERARCHY: u32 = 133;
const ABOUT_PRECURSORS: u32 = 134;
const ABOUT_OLD_RACES: u32 = 135;
const ABOUT_ALIENS_ON_EARTH: u32 = 136;
const OK_ENOUGH_ANCIENT: u32 = 137;
const URQUAN_STARTED_WAR: u32 = 138;
const WAR_WAS_LIKE_SO: u32 = 139;
const LOST_WAR_BECAUSE: u32 = 140;
const AFTER_WAR: u32 = 141;
const OK_ENOUGH_WAR: u32 = 142;
const STARBASE_BULLETIN_TAIL: u32 = 143;
const BETWEEN_BULLETINS: u32 = 144;
const STARBASE_BULLETIN_1: u32 = 145;
const STARBASE_BULLETIN_2: u32 = 146;
const STARBASE_BULLETIN_3: u32 = 147;
const STARBASE_BULLETIN_4: u32 = 148;
const STARBASE_BULLETIN_5: u32 = 149;
const STARBASE_BULLETIN_6: u32 = 150;
const STARBASE_BULLETIN_7: u32 = 151;
const STARBASE_BULLETIN_8: u32 = 152;
const STARBASE_BULLETIN_9: u32 = 153;
const STARBASE_BULLETIN_10: u32 = 154;
const STARBASE_BULLETIN_11: u32 = 155;
const STARBASE_BULLETIN_12: u32 = 156;
const STARBASE_BULLETIN_13: u32 = 157;
const STARBASE_BULLETIN_14: u32 = 158;
const STARBASE_BULLETIN_15: u32 = 159;
const STARBASE_BULLETIN_16: u32 = 160;
const STARBASE_BULLETIN_18: u32 = 161;
const STARBASE_BULLETIN_19: u32 = 162;
const STARBASE_BULLETIN_22: u32 = 163;
const STARBASE_BULLETIN_27: u32 = 164;
const STARBASE_BULLETIN_28: u32 = 165;
const STARBASE_BULLETIN_29: u32 = 166;
const STARBASE_BULLETIN_30: u32 = 167;
const DEVICE_HEAD: u32 = 168;
const BETWEEN_DEVICES: u32 = 169;
const DEVICE_TAIL: u32 = 170;
const ABOUT_PORTAL: u32 = 171;
const ABOUT_TALKPET: u32 = 172;
const ABOUT_BOMB: u32 = 173;
const ABOUT_SUN: u32 = 174;
const ABOUT_MAIDENS: u32 = 175;
const ABOUT_SPHERE: u32 = 176;
const ABOUT_HELIX: u32 = 177;
const ABOUT_SPINDLE: u32 = 178;
const ABOUT_ULTRON_0: u32 = 179;
const ABOUT_ULTRON_1: u32 = 180;
const ABOUT_ULTRON_2: u32 = 181;
const ABOUT_ULTRON_3: u32 = 182;
const ABOUT_UCASTER: u32 = 183;
const ABOUT_BCASTER: u32 = 184;
const ABOUT_SHIELD: u32 = 185;
const ABOUT_EGGCASE_0: u32 = 186;
const ABOUT_SHUTTLE: u32 = 187;
const ABOUT_VUXBEAST0: u32 = 188;
const ABOUT_VUXBEAST1: u32 = 189;
const ABOUT_DESTRUCT: u32 = 190;
const ABOUT_WARPPOD: u32 = 191;
const ABOUT_ARTIFACT_2: u32 = 192;
const ABOUT_ARTIFACT_3: u32 = 193;
const LETS_SEE: u32 = 194;
const GO_GET_MINERALS: u32 = 195;
const IMPROVE_FLAGSHIP_WITH_RU: u32 = 196;
const GOT_OK_FLAGSHIP: u32 = 197;
const GO_ALLY_WITH_ALIENS: u32 = 198;
const MADE_SOME_ALLIES: u32 = 199;
const GET_SHIPS_BY_MINING_OR_ALLIANCE: u32 = 200;
const GOT_OK_FLEET: u32 = 201;
const BUY_COMBAT_SHIPS: u32 = 202;
const GO_LEARN_ABOUT_URQUAN: u32 = 203;
const MAKE_FLAGSHIP_AWESOME: u32 = 204;
const KNOW_ABOUT_SAMATRA: u32 = 205;
const GOT_AWESOME_FLAGSHIP: u32 = 206;
const GOT_BOMB: u32 = 207;
const FIND_WAY_TO_DESTROY_SAMATRA: u32 = 208;
const MUST_INCREASE_BOMB_STRENGTH: u32 = 209;
const MUST_ACQUIRE_AWESOME_FLEET: u32 = 210;
const MUST_ELIMINATE_URQUAN_GUARDS: u32 = 211;
const CHMMR_IMPROVED_BOMB: u32 = 212;
const GOT_AWESOME_FLEET: u32 = 213;
const GO_DESTROY_SAMATRA: u32 = 214;
const GOOD_LUCK_AGAIN: u32 = 215;
const IMPROVE_1: u32 = 216;
const IMPROVE_2: u32 = 217;
const NEED_THRUSTERS_1: u32 = 218;
const NEED_THRUSTERS_2: u32 = 219;
const NEED_TURN_1: u32 = 220;
const NEED_TURN_2: u32 = 221;
const NEED_GUNS_1: u32 = 222;
const NEED_GUNS_2: u32 = 223;
const NEED_CREW_1: u32 = 224;
const NEED_CREW_2: u32 = 225;
const NEED_FUEL_1: u32 = 226;
const NEED_FUEL_2: u32 = 227;
const NEED_STORAGE_1: u32 = 228;
const NEED_LANDERS_2: u32 = 229;
const NEED_LANDERS_1: u32 = 230;
const NEED_DYNAMOS_1: u32 = 231;
const NEED_DYNAMOS_2: u32 = 232;
const NEED_POINT: u32 = 233;
const HAVE_MINERALS: u32 = 234;
const GOODBYE_COMMANDER: u32 = 235;
const REPEAT_BULLETINS: u32 = 236;
const NEED_INFO: u32 = 237;
const STARBASE_FUNCTIONS: u32 = 238;
const HISTORY: u32 = 239;
const OUR_MISSION: u32 = 240;
const NO_NEED_INFO: u32 = 241;
const ENOUGH_STARBASE: u32 = 242;
const ENOUGH_MISSION: u32 = 243;
const TELL_ME_ABOUT_FUEL0: u32 = 244;
const TELL_ME_ABOUT_FUEL1: u32 = 245;
const TELL_ME_ABOUT_MODULES0: u32 = 246;
const TELL_ME_ABOUT_MODULES1: u32 = 247;
const TELL_ME_ABOUT_CREW: u32 = 248;
const TELL_ME_ABOUT_SHIPS: u32 = 249;
const TELL_ME_ABOUT_RU: u32 = 250;
const TELL_ME_ABOUT_MINERALS: u32 = 251;
const TELL_ME_ABOUT_LIFE: u32 = 252;
const WHERE_GET_MINERALS: u32 = 253;
const WHAT_ABOUT_ALIENS: u32 = 254;
const WHAT_ABOUT_URQUAN: u32 = 255;
const HOW_DEFEAT: u32 = 256;
const HOW_FIND_URQUAN: u32 = 257;
const HOW_FIGHT_URQUAN: u32 = 258;
const HOW_ALLY: u32 = 259;
const ENOUGH_DEFEAT: u32 = 260;
const ALIEN_RACES: u32 = 261;
const THE_WAR: u32 = 262;
const ANCIENT_HISTORY: u32 = 263;
const ENOUGH_HISTORY: u32 = 264;
const WHAT_ABOUT_ALLIANCE: u32 = 265;
const WHAT_ABOUT_HIERARCHY: u32 = 266;
const WHAT_ABOUT_OTHER: u32 = 267;
const ENOUGH_ALIENS: u32 = 268;
const SHOFIXTI: u32 = 269;
const YEHAT: u32 = 270;
const ARILOU: u32 = 271;
const CHENJESU: u32 = 272;
const MMRNMHRM: u32 = 273;
const SYREEN: u32 = 274;
const ENOUGH_ALLIANCE: u32 = 275;
const URQUAN: u32 = 276;
const MYCON: u32 = 277;
const SPATHI: u32 = 278;
const UMGAH: u32 = 279;
const ANDROSYNTH: u32 = 280;
const VUX: u32 = 281;
const ILWRATH: u32 = 282;
const ENOUGH_HIERARCHY: u32 = 283;
const PRECURSORS: u32 = 284;
const OLD_RACES: u32 = 285;
const ALIENS_ON_EARTH: u32 = 286;
const ENOUGH_ANCIENT: u32 = 287;
const WHAT_STARTED_WAR: u32 = 288;
const WHAT_WAS_WAR_LIKE: u32 = 289;
const WHY_LOSE_WAR: u32 = 290;
const WHAT_AFTER_WAR: u32 = 291;
const ENOUGH_WAR: u32 = 292;
const NEW_DEVICES: u32 = 293;
const HOW_GET_STRONG: u32 = 294;
const WHAT_DO_NOW: u32 = 295;
const YOUR_FLAGSHIP_PC: u32 = 296;
const YOUR_FLAGSHIP_3DO0: u32 = 297;
const YOUR_FLAGSHIP_3DO1: u32 = 298;
const YOUR_FLAGSHIP_3DO2: u32 = 299;
const SPACE: u32 = 300;

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

const RACE_PMAP_ANIM: &[u8] = b"starbas\0";
const RACE_FONT: &[u8] = b"starbasfont\0";
const RACE_COLOR_MAP: &[u8] = b"starbascolr\0";
const RACE_MUSIC: &[u8] = b"starbasmusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.starbas.dialogue\0";

/// Starbas race dialogue implementation.
pub struct StarbasDialogue;

impl super::RaceDialogue for StarbasDialogue {
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
            bit_range("AQUA_HELIX_ON_SHIP").is_some(),
            "missing game state key: AQUA_HELIX_ON_SHIP"
        );
        assert!(
            bit_range("ARILOU_MANNER").is_some(),
            "missing game state key: ARILOU_MANNER"
        );
        assert!(
            bit_range("ARTIFACT_2_ON_SHIP").is_some(),
            "missing game state key: ARTIFACT_2_ON_SHIP"
        );
        assert!(
            bit_range("ARTIFACT_3_ON_SHIP").is_some(),
            "missing game state key: ARTIFACT_3_ON_SHIP"
        );
        assert!(
            bit_range("AWARE_OF_SAMATRA").is_some(),
            "missing game state key: AWARE_OF_SAMATRA"
        );
    }
}
