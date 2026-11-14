//! Vux dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

pub(crate) use crate::comm::races::descriptor::Point;

pub(crate) mod dialogue;
#[cfg(test)]
mod parity_tests;
mod phrases;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Combat,
    Menagerie,
    Normal,
}

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

pub(crate) const NULL_PHRASE: u32 = 0;
pub(crate) const ZEX_HELLO_1: u32 = 1;
pub(crate) const ZEX_HELLO_2: u32 = 2;
pub(crate) const ZEX_HELLO_3: u32 = 3;
pub(crate) const ZEX_HELLO_4: u32 = 4;
pub(crate) const FIGHT_OR_TRADE_1: u32 = 5;
pub(crate) const FIGHT_OR_TRADE_2: u32 = 6;
pub(crate) const WHAT_YOU_DO_HERE: u32 = 7;
pub(crate) const MY_MENAGERIE: u32 = 8;
pub(crate) const WHAT_ABOUT_MENAGERIE: u32 = 9;
pub(crate) const NEED_NEW_CREATURE: u32 = 10;
pub(crate) const WHAT_ABOUT_CREATURE: u32 = 11;
pub(crate) const ABOUT_CREATURE: u32 = 12;
pub(crate) const ABOUT_CREATURE_AGAIN: u32 = 13;
pub(crate) const CREATURE_AGAIN: u32 = 14;
pub(crate) const I_HAVE_BEAST: u32 = 15;
pub(crate) const GIVE_BEAST: u32 = 16;
pub(crate) const OK_TAKE_BEAST: u32 = 17;
pub(crate) const FOOL_AIEE0: u32 = 18;
pub(crate) const FOOL_AIEE1: u32 = 19;
pub(crate) const WHY_TRUST_1: u32 = 20;
pub(crate) const TRUST_1: u32 = 21;
pub(crate) const WHY_TRUST_2: u32 = 22;
pub(crate) const TRUST_2: u32 = 23;
pub(crate) const WHY_TRUST_3: u32 = 24;
pub(crate) const TRUST_3: u32 = 25;
pub(crate) const WHY_DONT_YOU_ATTACK: u32 = 26;
pub(crate) const LIKE_YOU: u32 = 27;
pub(crate) const WHY_LIKE_ME: u32 = 28;
pub(crate) const LIKE_BECAUSE: u32 = 29;
pub(crate) const ARE_YOU_A_PERVERT: u32 = 30;
pub(crate) const CALL_ME_WHAT_YOU_WISH: u32 = 31;
pub(crate) const TAKE_BY_FORCE: u32 = 32;
pub(crate) const PRECURSOR_DEVICE: u32 = 33;
pub(crate) const REGARDLESS: u32 = 34;
pub(crate) const THEN_FIGHT: u32 = 35;
pub(crate) const YOU_LIED: u32 = 36;
pub(crate) const YUP_LIED: u32 = 37;
pub(crate) const KILL_YOU: u32 = 38;
pub(crate) const FIGHT_AGAIN: u32 = 39;
pub(crate) const BYE_ZEX: u32 = 40;
pub(crate) const GOODBYE_ZEX: u32 = 41;
pub(crate) const HOMEWORLD_HELLO_1: u32 = 42;
pub(crate) const HOMEWORLD_HELLO_2: u32 = 43;
pub(crate) const HOMEWORLD_HELLO_3: u32 = 44;
pub(crate) const HOMEWORLD_HELLO_4: u32 = 45;
pub(crate) const SPACE_HELLO_1: u32 = 46;
pub(crate) const SPACE_HELLO_2: u32 = 47;
pub(crate) const SPACE_HELLO_3: u32 = 48;
pub(crate) const SPACE_HELLO_4: u32 = 49;
pub(crate) const KILL_YOU_SQUIDS_1: u32 = 50;
pub(crate) const KILL_YOU_SQUIDS_2: u32 = 51;
pub(crate) const KILL_YOU_SQUIDS_3: u32 = 52;
pub(crate) const KILL_YOU_SQUIDS_4: u32 = 53;
pub(crate) const WE_FIGHT: u32 = 54;
pub(crate) const WHY_SO_MEAN: u32 = 55;
pub(crate) const URQUAN_SLAVES: u32 = 56;
pub(crate) const DEEPER_REASON: u32 = 57;
pub(crate) const OLD_INSULT: u32 = 58;
pub(crate) const IF_WE_APOLOGIZE: u32 = 59;
pub(crate) const PROBABLY_NOT: u32 = 60;
pub(crate) const TRY_ANY_WAY: u32 = 61;
pub(crate) const NOPE: u32 = 62;
pub(crate) const APOLOGY_1: u32 = 64;
pub(crate) const NOT_ACCEPTED_1: u32 = 65;
pub(crate) const APOLOGY_2: u32 = 66;
pub(crate) const NOT_ACCEPTED_2: u32 = 67;
pub(crate) const APOLOGY_3: u32 = 68;
pub(crate) const NOT_ACCEPTED_3: u32 = 69;
pub(crate) const APOLOGY_4: u32 = 70;
pub(crate) const NOT_ACCEPTED_4: u32 = 71;
pub(crate) const APOLOGY_5: u32 = 72;
pub(crate) const NOT_ACCEPTED_5: u32 = 73;
pub(crate) const APOLOGY_6: u32 = 74;
pub(crate) const NOT_ACCEPTED_6: u32 = 75;
pub(crate) const APOLOGY_7: u32 = 76;
pub(crate) const NOT_ACCEPTED_7: u32 = 77;
pub(crate) const APOLOGY_8: u32 = 78;
pub(crate) const NOT_ACCEPTED_8: u32 = 79;
pub(crate) const APOLOGY_9: u32 = 80;
pub(crate) const NOT_ACCEPTED_9: u32 = 81;
pub(crate) const APOLOGY_10: u32 = 82;
pub(crate) const TRUTH: u32 = 83;
pub(crate) const WHATS_UP_HOSTILE: u32 = 84;
pub(crate) const GENERAL_INFO_HOSTILE_1: u32 = 85;
pub(crate) const GENERAL_INFO_HOSTILE_2: u32 = 86;
pub(crate) const GENERAL_INFO_HOSTILE_3: u32 = 87;
pub(crate) const GENERAL_INFO_HOSTILE_4: u32 = 88;
pub(crate) const CANT_WE_BE_FRIENDS_1: u32 = 89;
pub(crate) const NEVER_UGLY_HUMANS_1: u32 = 90;
pub(crate) const CANT_WE_BE_FRIENDS_2: u32 = 91;
pub(crate) const NEVER_UGLY_HUMANS_2: u32 = 92;
pub(crate) const CANT_WE_BE_FRIENDS_3: u32 = 93;
pub(crate) const NEVER_UGLY_HUMANS_3: u32 = 94;
pub(crate) const CANT_WE_BE_FRIENDS_4: u32 = 95;
pub(crate) const NEVER_UGLY_HUMANS_4: u32 = 96;
pub(crate) const BYE_HOSTILE_SPACE: u32 = 97;
pub(crate) const GOODBYE_AND_DIE_HOSTILE_SPACE_1: u32 = 98;
pub(crate) const GOODBYE_AND_DIE_HOSTILE_SPACE_2: u32 = 99;
pub(crate) const GOODBYE_AND_DIE_HOSTILE_SPACE_3: u32 = 100;
pub(crate) const GOODBYE_AND_DIE_HOSTILE_SPACE_4: u32 = 101;
pub(crate) const OUT_TAKES: u32 = 102;

pub(crate) fn text_layout(screen_width: i32) -> (Point, u16) {
    let text_width = screen_width - 80;
    (
        Point {
            x: (1 + (text_width >> 1) + (text_width >> 2)) as i16,
            y: 0,
        },
        ((text_width - 16) >> 1) as u16,
    )
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
