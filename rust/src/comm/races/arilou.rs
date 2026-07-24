#![allow(dead_code)]

//! Arilou dialogue state machine — ported from `comm/arilou/arilouc.c`.
//!
//! This is the reference implementation for per-race dialogue porting.
//! All primitives (NPCPhrase, Response, GET/SET_GAME_STATE, setSegue) are
//! now Rust-owned, so the dialogue runs entirely in Rust.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P12

use std::ffi::c_char;
use std::os::raw::c_int;

use crate::comm::segue::Segue;
use crate::comm::types::{AnimationDescData, CommData, TextAlign, TextValign};
use crate::state::game_state_keys::bit_range;

// ---------------------------------------------------------------------------
// String indices (from comm/arilou/strings.h)
// ---------------------------------------------------------------------------

const NULL_PHRASE: u32 = 0; // used for reference completeness
const INIT_HELLO: u32 = 1;
const CONFUSED_BY_HELLO: u32 = 2;
const CONFUSED_RESPONSE: u32 = 3;
const HAPPY_BY_HELLO: u32 = 4;
const HAPPY_RESPONSE: u32 = 5;
const MIFFED_BY_HELLO: u32 = 6;
const MIFFED_RESPONSE: u32 = 7;
const FRIENDLY_SPACE_HELLO_1: u32 = 8;
const FRIENDLY_SPACE_HELLO_2: u32 = 9;
const FRIENDLY_SPACE_HELLO_3: u32 = 10;
const FRIENDLY_SPACE_HELLO_4: u32 = 11;
const FRDLY_HOMEWORLD_HELLO_1: u32 = 12;
const FRDLY_HOMEWORLD_HELLO_2: u32 = 13;
const FRDLY_HOMEWORLD_HELLO_3: u32 = 14;
const FRDLY_HOMEWORLD_HELLO_4: u32 = 15;
const WHATS_UP_1: u32 = 16;
const WHATS_UP_2: u32 = 17;
const GENERAL_INFO_1: u32 = 18;
const GENERAL_INFO_2: u32 = 19;
const GENERAL_INFO_3: u32 = 20;
const GENERAL_INFO_4: u32 = 21;
const WHY_YOU_HERE: u32 = 22;
const LEARN_THINGS: u32 = 23;
const WHAT_THINGS: u32 = 24;
const THESE_THINGS: u32 = 25;
const WHY_DO_IT: u32 = 26;
const DO_IT_BECAUSE: u32 = 27;
const GIVE_ME_INFO_1: u32 = 28;
const ARILOU_HINTS_1: u32 = 29;
const GIVE_ME_INFO_2: u32 = 30;
const ARILOU_HINTS_2: u32 = 31;
const ARILOU_HINTS_3: u32 = 32;
const ARILOU_HINTS_4: u32 = 33;
const BYE_FRIENDLY_SPACE: u32 = 34;
const GOODBYE_FRIENDLY_SPACE: u32 = 35;
const GOT_PART_YET_1: u32 = 36;
const GOT_PART_YET_2: u32 = 37; // reserved for future dialogue branch
const INIT_ANGRY_HWLD_HELLO: u32 = 38;
const INVADERS_FROM_MARS: u32 = 39;
const HAD_OUR_REASONS: u32 = 40;
const BUG_EYED_FRUITCAKES: u32 = 41;
const WE_NEVER_FRIENDS: u32 = 42;
const OK_LETS_BE_FRIENDS: u32 = 43;
const NO_ALLY_BUT_MUCH_GIVE: u32 = 44;
const WHY_SHOULD_I_TRUST: u32 = 45;
const TRUST_BECAUSE: u32 = 46;
const WHAT_ABOUT_INTERFERENCE: u32 = 47;
const INTERFERENCE_NECESSARY: u32 = 48;
const I_JUST_LIKE_TO_LEAVE: u32 = 49;
const SORRY_NO_LEAVE: u32 = 50;
const WHAT_ABOUT_WAR: u32 = 51;
const ABOUT_WAR: u32 = 52;
const WHAT_ABOUT_URQUAN: u32 = 53;
const ABOUT_URQUAN: u32 = 54;
const BEST_IF_I_KILLED_YOU: u32 = 55;
const WICKED_HUMAN: u32 = 56;
const WHAT_DID_ON_EARTH: u32 = 57;
const DID_THIS: u32 = 58;
const WHY_DID_THIS: u32 = 59;
const IDF_PARASITES: u32 = 60;
const TELL_MORE: u32 = 61;
const NOT_NOW: u32 = 62;
const UMGAH_ACTING_WEIRD: u32 = 63;
const LEARNED_ABOUT_UMGAH: u32 = 64;
const WELL_GO_CHECK: u32 = 65;
const NO_NEWS_YET: u32 = 66;
const UMGAH_UNDER_COMPULSION: u32 = 67;
const WHAT_DO_NOW: u32 = 68;
const GO_FIND_OUT: u32 = 69;
const TELL_ARILOU_ABOUT_TPET: u32 = 70;
const BAD_NEWS_ABOUT_TPET: u32 = 71;
const WHAT_DO_ABOUT_TPET: u32 = 72;
const DANGEROUS_BUT_USEFUL: u32 = 73;
const WHAT_GIVE_ME: u32 = 74;
const ABOUT_PORTAL: u32 = 75;
const WHAT_ABOUT_TPET: u32 = 76;
const ABOUT_TPET: u32 = 77;
const ABOUT_PORTAL_AGAIN: u32 = 78;
const PORTAL_AGAIN: u32 = 79;
const GOT_IT: u32 = 80;
const CLEVER_HUMAN: u32 = 81;
const GIVE_PORTAL: u32 = 82;
const BYE_FRIENDLY_HOMEWORLD: u32 = 83;
const GOODBYE_FRDLY_HOMEWORLD: u32 = 84;
const HOSTILE_GOODBYE_1: u32 = 85;
const HOSTILE_GOODBYE_2: u32 = 86;
const HOSTILE_GOODBYE_3: u32 = 87;
const HOSTILE_GOODBYE_4: u32 = 88;
const ANGRY_SPACE_HELLO_1: u32 = 89;
const ANGRY_SPACE_HELLO_2: u32 = 90;
const LETS_FIGHT: u32 = 91;
const NO_FIGHT: u32 = 92;
const IM_SORRY: u32 = 93;
const APOLOGIZE_AT_HOMEWORLD: u32 = 94;
const BYE_ANGRY_SPACE: u32 = 95;
const GOODBYE_ANGRY_SPACE: u32 = 96;
const OUT_TAKES: u32 = 97;

// ---------------------------------------------------------------------------
// FFI declarations for comm primitives
// ---------------------------------------------------------------------------

extern "C" {
    fn rust_NPCPhrase_cb(index: c_int, cb: Option<extern "C" fn()>);
    fn rust_PhraseEnabled(index: c_int) -> c_int;
    fn rust_DisablePhrase(index: c_int);
    fn DoResponsePhrase(
        response_ref: u32,
        response_func: Option<extern "C" fn(u32)>,
        construct_str: *const c_char,
    );
}

extern "C" {
    fn rust_get_game_state_bits(start: c_int, end: c_int) -> u8;
    fn rust_set_game_state_bits(start: c_int, end: c_int, val: u8);
}

extern "C" {
    fn rust_add_event_relative(days_offset: u32, func_index: u8) -> u32;
}

// ---------------------------------------------------------------------------
// Game state helpers (read/write through Rust singleton)
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

fn get_current_activity() -> u16 {
    unsafe { crate::mainloop::c_extern::get_current_activity() }
}

// ---------------------------------------------------------------------------
// Comm state helpers
// ---------------------------------------------------------------------------

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
    // DoResponsePhrase resolves the text from the string table and calls
    // rust_DoResponsePhrase internally.
    unsafe {
        DoResponsePhrase(phrase, Some(callback), std::ptr::null());
    }
}

fn set_segue(segue: Segue) {
    crate::comm::state::COMM_STATE.write().set_segue(segue);
}

fn get_segue() -> Segue {
    crate::comm::state::COMM_STATE.read().get_segue()
}

// ---------------------------------------------------------------------------
// Activity constants (from globdata.h)
// ---------------------------------------------------------------------------

const WON_LAST_BATTLE: u8 = 5;

fn lobyte(val: u16) -> u8 {
    (val & 0xFF) as u8
}

// ---------------------------------------------------------------------------
// Dialogue callback functions
// ---------------------------------------------------------------------------

/// ExitConversation callback
extern "C" fn exit_conversation(r: u32) {
    set_segue(Segue::Peace);

    if r == BYE_ANGRY_SPACE {
        npc_phrase(GOODBYE_ANGRY_SPACE);
    } else if r == BYE_FRIENDLY_SPACE {
        npc_phrase(GOODBYE_FRIENDLY_SPACE);
    } else if r == BYE_FRIENDLY_HOMEWORLD {
        npc_phrase(GOODBYE_FRDLY_HOMEWORLD);
    } else if r == LETS_FIGHT {
        npc_phrase(NO_FIGHT);
    } else if r == BUG_EYED_FRUITCAKES {
        npc_phrase(WE_NEVER_FRIENDS);
        set_gs("ARILOU_MANNER", 2);
    } else if r == BEST_IF_I_KILLED_YOU {
        npc_phrase(WICKED_HUMAN);
        set_gs("ARILOU_MANNER", 2);
    }
}

/// ArilouHome callback
extern "C" fn arilou_home(r: u32) {
    let mut last_stack: usize = 0;
    let mut p_str: [u32; 4] = [0; 4];

    if r == CONFUSED_BY_HELLO {
        npc_phrase(CONFUSED_RESPONSE);
    } else if r == HAPPY_BY_HELLO {
        npc_phrase(HAPPY_RESPONSE);
    } else if r == MIFFED_BY_HELLO {
        npc_phrase(MIFFED_RESPONSE);
    } else if r == OK_LETS_BE_FRIENDS {
        npc_phrase(NO_ALLY_BUT_MUCH_GIVE);
    } else if r == WHAT_ABOUT_WAR {
        npc_phrase(ABOUT_WAR);
        set_gs("ARILOU_STACK_1", 1);
    } else if r == WHAT_ABOUT_URQUAN {
        npc_phrase(ABOUT_URQUAN);
        set_gs("ARILOU_STACK_1", 2);
    } else if r == TELL_ARILOU_ABOUT_TPET {
        npc_phrase(BAD_NEWS_ABOUT_TPET);
        last_stack = 1;
        set_gs("ARILOU_STACK_2", 1);
    } else if r == WHAT_DO_ABOUT_TPET {
        npc_phrase(DANGEROUS_BUT_USEFUL);
        last_stack = 1;
        set_gs("ARILOU_STACK_2", 2);
    } else if r == LEARNED_ABOUT_UMGAH {
        if get_gs("ARILOU_CHECKED_UMGAH") != 2 {
            npc_phrase(NO_NEWS_YET);
        } else {
            npc_phrase(UMGAH_UNDER_COMPULSION);
            last_stack = 1;
        }
        disable_phrase(LEARNED_ABOUT_UMGAH);
    } else if r == UMGAH_ACTING_WEIRD {
        npc_phrase(WELL_GO_CHECK);
        set_gs("ARILOU_CHECKED_UMGAH", 1);
        unsafe { rust_add_event_relative(10, 0) }; // ARILOU_UMGAH_CHECK event
        disable_phrase(UMGAH_ACTING_WEIRD);
    } else if r == WHAT_DO_NOW {
        npc_phrase(GO_FIND_OUT);
        set_gs("ARILOU_CHECKED_UMGAH", 3);
    } else if r == WHAT_DID_ON_EARTH {
        npc_phrase(DID_THIS);
        last_stack = 2;
        set_gs("ARILOU_STACK_3", 1);
    } else if r == WHY_DID_THIS {
        npc_phrase(IDF_PARASITES);
        last_stack = 2;
        set_gs("ARILOU_STACK_3", 2);
    } else if r == TELL_MORE {
        npc_phrase(NOT_NOW);
        last_stack = 2;
        set_gs("ARILOU_STACK_3", 3);
    } else if r == WHAT_GIVE_ME {
        npc_phrase(ABOUT_PORTAL);
        last_stack = 3;
        set_gs("KNOW_ARILOU_WANT_WRECK", 1);
        disable_phrase(WHAT_GIVE_ME);
    } else if r == WHAT_ABOUT_TPET {
        npc_phrase(ABOUT_TPET);
        set_gs("ARILOU_STACK_4", 1);
    } else if r == ABOUT_PORTAL_AGAIN {
        npc_phrase(PORTAL_AGAIN);
        disable_phrase(ABOUT_PORTAL_AGAIN);
    } else if r == GOT_IT {
        if get_gs("ARILOU_HOME_VISITS") == 1 {
            npc_phrase(CLEVER_HUMAN);
        }
        npc_phrase(GIVE_PORTAL);
        set_gs("PORTAL_KEY_ON_SHIP", 0);
        set_gs("PORTAL_SPAWNER", 1);
        set_gs("PORTAL_SPAWNER_ON_SHIP", 1);
    }

    // Build response list based on game state
    match get_gs("ARILOU_STACK_1") {
        0 => p_str[0] = WHAT_ABOUT_WAR,
        1 => p_str[0] = WHAT_ABOUT_URQUAN,
        _ => {}
    }

    if get_gs("TALKING_PET") != 0 {
        match get_gs("ARILOU_STACK_2") {
            0 => p_str[1] = TELL_ARILOU_ABOUT_TPET,
            1 => p_str[1] = WHAT_DO_ABOUT_TPET,
            _ => {}
        }
    } else if get_gs("KNOW_UMGAH_ZOMBIES") != 0 {
        if get_gs("ARILOU_CHECKED_UMGAH") == 0 {
            p_str[1] = UMGAH_ACTING_WEIRD;
        } else if phrase_enabled(LEARNED_ABOUT_UMGAH) && phrase_enabled(UMGAH_ACTING_WEIRD) {
            p_str[1] = LEARNED_ABOUT_UMGAH;
        } else if get_gs("ARILOU_CHECKED_UMGAH") == 2 {
            p_str[1] = WHAT_DO_NOW;
        }
    }

    match get_gs("ARILOU_STACK_3") {
        0 => p_str[2] = WHAT_DID_ON_EARTH,
        1 => p_str[2] = WHY_DID_THIS,
        2 => p_str[2] = TELL_MORE,
        _ => {}
    }

    if get_gs("KNOW_ARILOU_WANT_WRECK") == 0 {
        p_str[3] = WHAT_GIVE_ME;
    } else if get_gs("ARILOU_STACK_4") == 0 {
        p_str[3] = WHAT_ABOUT_TPET;
    }

    // Add responses — last stack first, then the rest
    if p_str[last_stack] != 0 {
        response(p_str[last_stack], arilou_home);
    }
    for (i, &p) in p_str.iter().enumerate() {
        if i != last_stack && p != 0 {
            response(p, arilou_home);
        }
    }

    // Portal-related responses
    if get_gs("KNOW_ARILOU_WANT_WRECK") != 0 {
        if get_gs("PORTAL_KEY_ON_SHIP") != 0 {
            response(GOT_IT, arilou_home);
        } else if phrase_enabled(ABOUT_PORTAL_AGAIN) && get_gs("PORTAL_SPAWNER") == 0 {
            response(ABOUT_PORTAL_AGAIN, arilou_home);
        }
    }

    if get_gs("ARILOU_MANNER") != 3 {
        response(BEST_IF_I_KILLED_YOU, exit_conversation);
    }
    response(BYE_FRIENDLY_HOMEWORLD, exit_conversation);
}

/// AngryHomeArilou callback
extern "C" fn angry_home_arilou(r: u32) {
    if r == INVADERS_FROM_MARS {
        npc_phrase(HAD_OUR_REASONS);
        disable_phrase(INVADERS_FROM_MARS);
    } else if r == WHY_SHOULD_I_TRUST {
        npc_phrase(TRUST_BECAUSE);
        disable_phrase(WHY_SHOULD_I_TRUST);
    } else if r == WHAT_ABOUT_INTERFERENCE {
        npc_phrase(INTERFERENCE_NECESSARY);
        disable_phrase(WHAT_ABOUT_INTERFERENCE);
    } else if r == I_JUST_LIKE_TO_LEAVE {
        npc_phrase(SORRY_NO_LEAVE);
        disable_phrase(I_JUST_LIKE_TO_LEAVE);
    }

    if phrase_enabled(INVADERS_FROM_MARS) {
        response(INVADERS_FROM_MARS, angry_home_arilou);
    } else {
        response(BUG_EYED_FRUITCAKES, exit_conversation);
    }
    if phrase_enabled(WHY_SHOULD_I_TRUST) {
        response(WHY_SHOULD_I_TRUST, angry_home_arilou);
    } else if phrase_enabled(WHAT_ABOUT_INTERFERENCE) {
        response(WHAT_ABOUT_INTERFERENCE, angry_home_arilou);
    }
    response(OK_LETS_BE_FRIENDS, arilou_home);
    response(I_JUST_LIKE_TO_LEAVE, angry_home_arilou);
}

/// AngrySpaceArilou callback
extern "C" fn angry_space_arilou(r: u32) {
    if r == IM_SORRY {
        npc_phrase(APOLOGIZE_AT_HOMEWORLD);
        disable_phrase(IM_SORRY);
    }

    response(LETS_FIGHT, exit_conversation);
    if phrase_enabled(IM_SORRY) {
        response(IM_SORRY, angry_space_arilou);
    }
    response(BYE_ANGRY_SPACE, exit_conversation);
}

/// FriendlySpaceArilou callback
extern "C" fn friendly_space_arilou(r: u32) {
    if r == CONFUSED_BY_HELLO {
        npc_phrase(CONFUSED_RESPONSE);
    } else if r == HAPPY_BY_HELLO {
        npc_phrase(HAPPY_RESPONSE);
    } else if r == MIFFED_BY_HELLO {
        npc_phrase(MIFFED_RESPONSE);
    } else if r == WHATS_UP_1 || r == WHATS_UP_2 {
        let mut num_visits = get_gs("ARILOU_INFO");
        match num_visits {
            0 => npc_phrase(GENERAL_INFO_1),
            1 => npc_phrase(GENERAL_INFO_2),
            2 => npc_phrase(GENERAL_INFO_3),
            3 => {
                npc_phrase(GENERAL_INFO_4);
                num_visits = 2; // --NumVisits (don't advance past 3)
            }
            _ => {}
        }
        num_visits = num_visits.wrapping_add(1);
        if num_visits > 3 {
            num_visits = 3;
        }
        set_gs("ARILOU_INFO", num_visits);
        disable_phrase(WHATS_UP_2);
    } else if r == WHY_YOU_HERE {
        npc_phrase(LEARN_THINGS);
        set_gs("ARILOU_STACK_5", 1);
    } else if r == WHAT_THINGS {
        npc_phrase(THESE_THINGS);
        set_gs("ARILOU_STACK_5", 2);
    } else if r == WHY_DO_IT {
        npc_phrase(DO_IT_BECAUSE);
        set_gs("ARILOU_STACK_5", 3);
    } else if r == GIVE_ME_INFO_1 || r == GIVE_ME_INFO_2 {
        let mut num_visits = get_gs("ARILOU_HINTS");
        match num_visits {
            0 => npc_phrase(ARILOU_HINTS_1),
            1 => {
                npc_phrase(ARILOU_HINTS_2);
                if get_gs("KNOW_ABOUT_SHATTERED") < 2 {
                    set_gs("KNOW_ABOUT_SHATTERED", 2);
                }
            }
            2 => {
                npc_phrase(ARILOU_HINTS_3);
                set_gs("KNOW_URQUAN_STORY", 1);
                set_gs("KNOW_KOHR_AH_STORY", 1);
            }
            3 => {
                npc_phrase(ARILOU_HINTS_4);
                num_visits = 2; // --NumVisits
            }
            _ => {}
        }
        num_visits = num_visits.wrapping_add(1);
        if num_visits > 3 {
            num_visits = 3;
        }
        set_gs("ARILOU_HINTS", num_visits);
        disable_phrase(GIVE_ME_INFO_2);
    }

    // Build response list
    match get_gs("ARILOU_STACK_5") {
        0 => response(WHY_YOU_HERE, friendly_space_arilou),
        1 => response(WHAT_THINGS, friendly_space_arilou),
        2 => response(WHY_DO_IT, friendly_space_arilou),
        _ => {}
    }

    if phrase_enabled(WHATS_UP_2) {
        if get_gs("ARILOU_INFO") == 0 {
            response(WHATS_UP_1, friendly_space_arilou);
        } else {
            response(WHATS_UP_2, friendly_space_arilou);
        }
    }

    if phrase_enabled(GIVE_ME_INFO_2) {
        if get_gs("ARILOU_HINTS") == 0 {
            response(GIVE_ME_INFO_1, friendly_space_arilou);
        } else {
            response(GIVE_ME_INFO_2, friendly_space_arilou);
        }
    }

    response(BYE_FRIENDLY_SPACE, exit_conversation);
}

// ---------------------------------------------------------------------------
// Intro, post-encounter, and uninit
// ---------------------------------------------------------------------------

/// Intro — the init_encounter_func callback.
fn intro() {
    if lobyte(get_current_activity()) == WON_LAST_BATTLE {
        npc_phrase(OUT_TAKES);
        set_segue(Segue::Peace);
        return;
    }

    if get_gs("MET_ARILOU") == 0 {
        if get_gs("ARILOU_SPACE_SIDE") <= 1 {
            npc_phrase(INIT_HELLO);
        } else {
            npc_phrase(FRDLY_HOMEWORLD_HELLO_1);
            set_gs("ARILOU_HOME_VISITS", 1);
        }
        response(CONFUSED_BY_HELLO, friendly_space_arilou);
        response(HAPPY_BY_HELLO, friendly_space_arilou);
        response(MIFFED_BY_HELLO, friendly_space_arilou);
        set_gs("MET_ARILOU", 1);
        return;
    }

    let manner = get_gs("ARILOU_MANNER");
    if manner == 2 {
        let mut num_visits = get_gs("ARILOU_VISITS");
        match num_visits {
            0 => npc_phrase(HOSTILE_GOODBYE_1),
            1 => npc_phrase(HOSTILE_GOODBYE_2),
            2 => npc_phrase(HOSTILE_GOODBYE_3),
            3 => {
                npc_phrase(HOSTILE_GOODBYE_4);
                num_visits = 2; // --NumVisits
            }
            _ => {}
        }
        num_visits = num_visits.wrapping_add(1);
        if num_visits > 3 {
            num_visits = 3;
        }
        set_gs("ARILOU_VISITS", num_visits);
        set_segue(Segue::Peace);
    } else if manner == 1 {
        if get_gs("ARILOU_SPACE_SIDE") > 1 {
            npc_phrase(INIT_ANGRY_HWLD_HELLO);
            set_gs("ARILOU_HOME_VISITS", 1);
            angry_home_arilou(0);
        } else {
            let mut num_visits = get_gs("ARILOU_VISITS");
            match num_visits {
                0 => npc_phrase(ANGRY_SPACE_HELLO_1),
                1 => {
                    npc_phrase(ANGRY_SPACE_HELLO_2);
                    num_visits = 0; // --NumVisits
                }
                _ => {}
            }
            num_visits = num_visits.wrapping_add(1);
            if num_visits > 1 {
                num_visits = 1;
            }
            set_gs("ARILOU_VISITS", num_visits);
            angry_space_arilou(0);
        }
    } else {
        if get_gs("ARILOU_SPACE_SIDE") <= 1 {
            let mut num_visits = get_gs("ARILOU_VISITS");
            match num_visits {
                0 => npc_phrase(FRIENDLY_SPACE_HELLO_1),
                1 => npc_phrase(FRIENDLY_SPACE_HELLO_2),
                2 => npc_phrase(FRIENDLY_SPACE_HELLO_3),
                3 => {
                    npc_phrase(FRIENDLY_SPACE_HELLO_4);
                    num_visits = 2; // --NumVisits
                }
                _ => {}
            }
            num_visits = num_visits.wrapping_add(1);
            if num_visits > 3 {
                num_visits = 3;
            }
            set_gs("ARILOU_VISITS", num_visits);
            friendly_space_arilou(0);
        } else {
            if get_gs("PORTAL_SPAWNER") == 0 && get_gs("KNOW_ARILOU_WANT_WRECK") != 0 {
                let mut num_visits = get_gs("NO_PORTAL_VISITS");
                match num_visits {
                    0 => npc_phrase(GOT_PART_YET_1),
                    1 => {
                        npc_phrase(GOT_PART_YET_1);
                        num_visits = 0; // --NumVisits
                    }
                    _ => {}
                }
                num_visits = num_visits.wrapping_add(1);
                if num_visits > 1 {
                    num_visits = 1;
                }
                set_gs("NO_PORTAL_VISITS", num_visits);
            } else {
                let mut num_visits = get_gs("ARILOU_HOME_VISITS");
                match num_visits {
                    0 => npc_phrase(FRDLY_HOMEWORLD_HELLO_1),
                    1 => npc_phrase(FRDLY_HOMEWORLD_HELLO_2),
                    2 => npc_phrase(FRDLY_HOMEWORLD_HELLO_3),
                    3 => {
                        npc_phrase(FRDLY_HOMEWORLD_HELLO_4);
                        num_visits = 2; // --NumVisits
                    }
                    _ => {}
                }
                num_visits = num_visits.wrapping_add(1);
                if num_visits > 3 {
                    num_visits = 3;
                }
                set_gs("ARILOU_HOME_VISITS", num_visits);
            }
            arilou_home(0);
        }
    }
}

/// post_arilou_enc — post-encounter processing.
fn post_arilou_enc() {
    let manner = get_gs("ARILOU_MANNER");

    if get_segue() == Segue::Hostile && manner != 2 {
        set_gs("ARILOU_MANNER", 1);
        if manner != 1 {
            set_gs("ARILOU_VISITS", 0);
            set_gs("ARILOU_HOME_VISITS", 0);
        }
    }

    if get_gs("ARILOU_SPACE_SIDE") > 1 && get_gs("ARILOU_HOME_VISITS") <= 1 {
        set_gs("UMGAH_ZOMBIE_BLOBBIES", 1);
        set_gs("UMGAH_VISITS", 0);
        set_gs("UMGAH_HOME_VISITS", 0);

        if get_gs("ARILOU_MANNER") < 2 {
            set_gs("ARILOU_MANNER", 3);
        }
    }
}

/// uninit_arilou — cleanup (returns 0).
fn uninit_arilou() -> u32 {
    0
}

// ---------------------------------------------------------------------------
// Resource keys (from comm/arilou/resinst.h)
// ---------------------------------------------------------------------------

// These are C string literals (RESOURCE type = const char*).
// In Rust, we use CStr-compatible statics.
const ARILOU_PMAP_ANIM: &[u8] = b"arilou\0";
const ARILOU_FONT: &[u8] = b"arifont\0";
const ARILOU_COLOR_MAP: &[u8] = b"aricolr\0";
const ARILOU_MUSIC: &[u8] = b"arimusic\0";
const ARILOU_CONVERSATION_PHRASES: &[u8] = b"ariphrases\0";

// ---------------------------------------------------------------------------
// ArilouDialogue: RaceDialogue implementation
// ---------------------------------------------------------------------------

/// Arilou race dialogue implementation.
pub struct ArilouDialogue;

impl super::RaceDialogue for ArilouDialogue {
    fn init(&self) -> CommData {
        // Resource keys
        let mut data = CommData {
            alien_frame_res: ARILOU_PMAP_ANIM.as_ptr() as *const _,
            alien_font_res: ARILOU_FONT.as_ptr() as *const _,
            alien_colormap_res: ARILOU_COLOR_MAP.as_ptr() as *const _,
            alien_song_res: ARILOU_MUSIC.as_ptr() as *const _,
            alien_alt_song_res: std::ptr::null(),
            conversation_phrases_res: ARILOU_CONVERSATION_PHRASES.as_ptr() as *const _,
            alien_text_align: TextAlign::Center,
            alien_text_valign: TextValign::Top,
            alien_text_fcolor: 0x00FFFFFF,
            alien_text_bcolor: 0x00000000,
            alien_text_baseline_x: 0,
            alien_text_baseline_y: 0,
            alien_text_width: 0,
            alien_song_flags: 0,
            num_animations: 20,
            ..CommData::default()
        };

        // The ambient animations are populated from the C LOCDATA struct
        // during the sync process. When fully ported, these will be filled
        // in from the C struct.
        data.alien_transition_desc = AnimationDescData::default();
        data.alien_talk_desc = AnimationDescData::default();

        // Set segue based on game state
        if get_gs("ARILOU_SPACE_SIDE") > 1
            || get_gs("ARILOU_MANNER") == 3
            || lobyte(get_current_activity()) == WON_LAST_BATTLE
        {
            set_segue(Segue::Peace);
        } else {
            set_segue(Segue::Hostile);
        }

        data
    }

    fn intro(&self) {
        intro();
    }

    fn post_encounter(&self) {
        post_arilou_enc();
    }

    fn uninit(&self) -> u32 {
        uninit_arilou()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_indices_start_at_zero() {
        assert_eq!(NULL_PHRASE, 0);
        assert_eq!(INIT_HELLO, 1);
    }

    #[test]
    fn test_lobyte() {
        assert_eq!(lobyte(0x0103), 3);
        assert_eq!(lobyte(0x0205), 5);
    }

    #[test]
    fn test_game_state_key_lookup() {
        // Verify that Arilou-related keys exist in the state table
        assert!(bit_range("ARILOU_SPACE_SIDE").is_some());
        assert!(bit_range("ARILOU_MANNER").is_some());
        assert!(bit_range("MET_ARILOU").is_some());
        assert!(bit_range("BATTLE_SEGUE").is_some());
    }

    #[test]
    fn test_game_state_key_ranges() {
        let (start, end) = bit_range("ARILOU_SPACE_SIDE").unwrap();
        assert_eq!(start, 42);
        assert_eq!(end, 43);

        let (start, end) = bit_range("BATTLE_SEGUE").unwrap();
        assert_eq!(start, 15);
        assert_eq!(end, 15);

        let (start, end) = bit_range("MET_ARILOU").unwrap();
        assert_eq!(start, 128);
        assert_eq!(end, 128);
    }

    #[test]
    fn test_resource_keys_are_null_terminated() {
        assert_eq!(ARILOU_PMAP_ANIM.last(), Some(&0));
        assert_eq!(ARILOU_FONT.last(), Some(&0));
        assert_eq!(ARILOU_COLOR_MAP.last(), Some(&0));
        assert_eq!(ARILOU_MUSIC.last(), Some(&0));
        assert_eq!(ARILOU_CONVERSATION_PHRASES.last(), Some(&0));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        assert!(bit_range("NONEXISTENT_KEY").is_none());
    }
}
