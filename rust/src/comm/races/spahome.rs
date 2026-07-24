//! Spahome dialogue state machine — ported from C.
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
const KILLED_FWIFFO: u32 = 1;
const POOR_FWIFFO: u32 = 2;
const FWIFFO_FINE: u32 = 3;
const NOT_LIKELY: u32 = 4;
const WE_ATTACK_AGAIN: u32 = 5;
const WE_FIGHT_AGAIN: u32 = 6;
const BYE_NO_ALLY_OFFER: u32 = 7;
const GOODBYE_NO_ALLY_OFFER: u32 = 8;
const BYE_ANGRY_SPATHI: u32 = 9;
const GOODBYE_ANGRY_SPATHI: u32 = 10;
const WHY_DONT_YOU_DO_IT: u32 = 11;
const WE_WONT_BECAUSE: u32 = 12;
const MEAN_GUYS_RETURN: u32 = 13;
const WE_APOLOGIZE: u32 = 14;
const DONT_BELIEVE: u32 = 15;
const HELLO_AGAIN: u32 = 16;
const HATE_YOU_FOREVER: u32 = 17;
const WHAT_IS_PASSWORD: u32 = 18;
const WHAT_IS_PASSWORD_AGAIN: u32 = 19;
const WE_ARE_VINDICATOR0: u32 = 20;
const WE_ARE_VINDICATOR1: u32 = 21;
const WE_ARE_VINDICATOR2: u32 = 22;
const GORT_MERENGA: u32 = 23;
const GUPH_FLORP: u32 = 24;
const PLEEESE: u32 = 25;
const WAGNGL_FTHAGN: u32 = 26;
const SCREW_PASSWORD: u32 = 27;
const GOOD_PASSWORD: u32 = 28;
const WRONG_PASSWORD: u32 = 29;
const NO_PASSWORD: u32 = 30;
const WHAT_DO_I_GET: u32 = 31;
const YOU_GET_TO_LIVE: u32 = 32;
const YES_GOOD_PASSWORD: u32 = 33;
const SPATHI_ON_PLUTO: u32 = 34;
const WHERE_SPATHI: u32 = 35;
const HOSTAGE: u32 = 36;
const GUN_TO_HEAD: u32 = 37;
const WE_COME_IN_PEACE: u32 = 38;
const OF_COURSE: u32 = 39;
const KILLED_SPATHI: u32 = 40;
const MISUNDERSTANDING: u32 = 41;
const JUST_MISUNDERSTANDING: u32 = 42;
const GIVE_US_RESOURCES: u32 = 43;
const NO_RESOURCES: u32 = 44;
const RESOURCES_PLEASE: u32 = 45;
const SORRY_NO_RESOURCES: u32 = 46;
const BYE_ALLY: u32 = 47;
const GOODBYE_ALLY: u32 = 48;
const WHAT_ABOUT_HIERARCHY: u32 = 49;
const WHAT_ABOUT_HISTORY: u32 = 50;
const WHAT_ABOUT_ALLIANCE: u32 = 51;
const WHAT_ABOUT_OTHER: u32 = 52;
const WHAT_ABOUT_PRECURSORS: u32 = 53;
const ENOUGH_INFO: u32 = 54;
const OK_ENOUGH_INFO: u32 = 55;
const ABOUT_HIERARCHY: u32 = 56;
const ABOUT_HISTORY: u32 = 57;
const ABOUT_ALLIANCE: u32 = 58;
const ABOUT_OTHER: u32 = 59;
const ABOUT_PRECURSORS: u32 = 60;
const LITTLE_MISTAKE: u32 = 61;
const BIG_MISTAKE: u32 = 62;
const BYE_BEFORE_PARTY: u32 = 63;
const QUEST_AGAIN: u32 = 64;
const GOODBYE_BEFORE_PARTY: u32 = 65;
const GOOD_START: u32 = 66;
const SOMETHING_FISHY: u32 = 67;
const NOTHING_FISHY: u32 = 68;
const SURRENDER: u32 = 69;
const NO_SURRENDER: u32 = 70;
const SURRENDER_OR_DIE: u32 = 71;
const DEFEND_OURSELVES: u32 = 72;
const HAND_IN_FRIENDSHIP: u32 = 73;
const TOO_AFRAID: u32 = 74;
const STRONGER: u32 = 75;
const YOURE_NOT: u32 = 76;
const YES_WE_ARE: u32 = 77;
const NO_YOURE_NOT: u32 = 78;
const HOW_PROVE: u32 = 79;
const BETTER_IDEA: u32 = 80;
const SHARE_INFO: u32 = 81;
const NO_INFO: u32 = 82;
const WE_UNDERSTAND: u32 = 83;
const PROVE_STRENGTH: u32 = 84;
const YOUR_BEHAVIOR: u32 = 85;
const WHAT_TEST: u32 = 86;
const BEFORE_ACCEPT: u32 = 87;
const WIPE_EVIL: u32 = 88;
const THINK_MORE: u32 = 89;
const COWARD: u32 = 90;
const TELL_EVIL: u32 = 91;
const I_ACCEPT: u32 = 92;
const AWAIT_RETURN: u32 = 93;
const TALK_TEST: u32 = 94;
const ALREADY_GOT_THEM: u32 = 95;
const EARLY_BIRD_CHECK: u32 = 96;
const NOT_SURPRISED: u32 = 97;
const TEST_AGAIN: u32 = 98;
const TOO_DANGEROUS: u32 = 99;
const WE_AGREE: u32 = 100;
const HOW_GO_EFFORTS: u32 = 101;
const KILLED_THEM_ALL_1: u32 = 102;
const KILLED_THEM_ALL_2: u32 = 103;
const WILL_CHECK_1: u32 = 104;
const WILL_CHECK_2: u32 = 105;
const ZAPPED_A_FEW: u32 = 106;
const RETURN_COMPLETE: u32 = 107;
const MUST_DESTROY_ALL: u32 = 108;
const NO_LANDING: u32 = 109;
const SAW_CREATURES: u32 = 110;
const YOU_FORTUNATE: u32 = 111;
const YOU_LIED_1: u32 = 112;
const YOU_LIED_2: u32 = 113;
const BYE_FROM_PARTY_1: u32 = 114;
const BYE_FROM_PARTY_2: u32 = 115;
const BYE_FROM_PARTY_3: u32 = 116;
const GOODBYE_FROM_PARTY: u32 = 117;
const MUST_PARTY_1: u32 = 118;
const MUST_PARTY_2: u32 = 119;
const MUST_PARTY_3: u32 = 120;
const DEALS_A_DEAL: u32 = 121;
const WAIT_A_WHILE: u32 = 122;
const HOW_LONG: u32 = 123;
const TEN_YEARS: u32 = 124;
const RENEGING: u32 = 125;
const ADULT_VIEW: u32 = 126;
const RETURN_BEASTS: u32 = 127;
const WHAT_RELATIONSHIP: u32 = 128;
const MINDS_AND_MIGHT: u32 = 129;
const HUH: u32 = 130;
const FELLOWSHIP: u32 = 131;
const WHAT: u32 = 132;
const DO_AS_WE_SAY: u32 = 133;
const DEPART_FOR_EARTH: u32 = 134;
const HELLO_ALLIES_1: u32 = 135;
const HELLO_ALLIES_2: u32 = 136;
const HELLO_ALLIES_3: u32 = 137;
const WHATS_UP: u32 = 138;
const GENERAL_INFO_1: u32 = 139;
const GENERAL_INFO_2: u32 = 140;
const GENERAL_INFO_3: u32 = 141;
const GENERAL_INFO_4: u32 = 142;
const GENERAL_INFO_5: u32 = 143;
const LIKE_SOME_INFO: u32 = 144;
const WHAT_ABOUT: u32 = 145;

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

const RACE_PMAP_ANIM: &[u8] = b"spahome\0";
const RACE_FONT: &[u8] = b"spahomefont\0";
const RACE_COLOR_MAP: &[u8] = b"spahomecolr\0";
const RACE_MUSIC: &[u8] = b"spahomemusic\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.spahome.dialogue\0";

/// Spahome race dialogue implementation.
pub struct SpahomeDialogue;

impl super::RaceDialogue for SpahomeDialogue {
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
            bit_range("KNOW_KOHR_AH_STORY").is_some(),
            "missing game state key: KNOW_KOHR_AH_STORY"
        );
        assert!(
            bit_range("KNOW_SPATHI_EVIL").is_some(),
            "missing game state key: KNOW_SPATHI_EVIL"
        );
        assert!(
            bit_range("KNOW_SPATHI_PASSWORD").is_some(),
            "missing game state key: KNOW_SPATHI_PASSWORD"
        );
        assert!(
            bit_range("KNOW_SPATHI_QUEST").is_some(),
            "missing game state key: KNOW_SPATHI_QUEST"
        );
    }
}
