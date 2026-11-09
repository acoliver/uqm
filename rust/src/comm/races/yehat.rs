//! Yehat dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

pub(crate) use crate::comm::locdata::CPoint;

pub(crate) mod api;
pub(crate) mod dialogue;
#[cfg(test)]
mod parity_tests;
mod phrases;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Exit,
    Royalists,
    StartRevolt,
    Home,
    Space,
}

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

pub(crate) const NULL_PHRASE: u32 = 0;
pub(crate) const HOMEWORLD_HELLO_1: u32 = 1;
pub(crate) const HOMEWORLD_HELLO_2: u32 = 2;
pub(crate) const WHATS_UP_HOMEWORLD: u32 = 3;
pub(crate) const GENERAL_INFO_HOMEWORLD_1: u32 = 4;
pub(crate) const GENERAL_INFO_HOMEWORLD_2: u32 = 5;
pub(crate) const I_DEMAND_YOU_ALLY_HOMEWORLD0: u32 = 6;
pub(crate) const I_DEMAND_YOU_ALLY_HOMEWORLD1: u32 = 7;
pub(crate) const I_DEMAND_YOU_ALLY_HOMEWORLD2: u32 = 8;
pub(crate) const I_DEMAND_YOU_ALLY_HOMEWORLD3: u32 = 9;
pub(crate) const ENEMY_MUST_DIE: u32 = 10;
pub(crate) const AT_LEAST_HELP_US_HOMEWORLD: u32 = 11;
pub(crate) const NO_HELP_ENEMY: u32 = 12;
pub(crate) const GIVE_INFO: u32 = 13;
pub(crate) const NO_INFO_FOR_ENEMY: u32 = 14;
pub(crate) const WHAT_ABOUT_PKUNK_ROYALIST: u32 = 15;
pub(crate) const PKUNK_ABSORBED_ROYALIST: u32 = 16;
pub(crate) const HATE_PKUNK_ROYALIST: u32 = 17;
pub(crate) const BYE_HOMEWORLD: u32 = 18;
pub(crate) const GOODBYE_AND_DIE_HOMEWORLD: u32 = 19;
pub(crate) const SPACE_HELLO_1: u32 = 20;
pub(crate) const SPACE_HELLO_2: u32 = 21;
pub(crate) const SPACE_HELLO_3: u32 = 22;
pub(crate) const SPACE_HELLO_4: u32 = 23;
pub(crate) const WHATS_UP_SPACE_1: u32 = 24;
pub(crate) const GENERAL_INFO_SPACE_1: u32 = 25;
pub(crate) const WHATS_UP_SPACE_2: u32 = 26;
pub(crate) const GENERAL_INFO_SPACE_2: u32 = 27;
pub(crate) const WHATS_UP_SPACE_3: u32 = 28;
pub(crate) const GENERAL_INFO_SPACE_3: u32 = 29;
pub(crate) const WHATS_UP_SPACE_4: u32 = 30;
pub(crate) const GENERAL_INFO_SPACE_4: u32 = 31;
pub(crate) const I_DEMAND_YOU_ALLY_SPACE0: u32 = 32;
pub(crate) const I_DEMAND_YOU_ALLY_SPACE1: u32 = 33;
pub(crate) const I_DEMAND_YOU_ALLY_SPACE2: u32 = 34;
pub(crate) const I_DEMAND_YOU_ALLY_SPACE3: u32 = 35;
pub(crate) const WE_CANNOT_1: u32 = 36;
pub(crate) const OBLIGATION: u32 = 37;
pub(crate) const WE_CANNOT_2: u32 = 38;
pub(crate) const AT_LEAST_HELP_US_SPACE: u32 = 39;
pub(crate) const SORRY_CANNOT: u32 = 40;
pub(crate) const DISHONOR: u32 = 41;
pub(crate) const HERES_A_HINT: u32 = 42;
pub(crate) const BYE_SPACE: u32 = 46;
pub(crate) const GO_IN_PEACE: u32 = 47;
pub(crate) const GOODBYE_AND_DIE_SPACE: u32 = 48;
pub(crate) const SHOFIXTI_ALIVE_1: u32 = 49;
pub(crate) const SHOFIXTI_ALIVE_2: u32 = 50;
pub(crate) const SEND_HIM_OVER_1: u32 = 51;
pub(crate) const SEND_HIM_OVER_2: u32 = 52;
pub(crate) const NOT_HERE: u32 = 53;
pub(crate) const NOT_SEND: u32 = 54;
pub(crate) const JUST_A_TRICK_1: u32 = 55;
pub(crate) const JUST_A_TRICK_2: u32 = 56;
pub(crate) const OK_SEND: u32 = 57;
pub(crate) const WE_REVOLT: u32 = 58;
pub(crate) const ROYALIST_SPACE_HELLO_1: u32 = 59;
pub(crate) const ROYALIST_SPACE_HELLO_2: u32 = 60;
pub(crate) const ROYALIST_HOMEWORLD_HELLO_1: u32 = 61;
pub(crate) const ROYALIST_HOMEWORLD_HELLO_2: u32 = 62;
pub(crate) const HOW_IS_REBELLION: u32 = 63;
pub(crate) const ROYALIST_REBELLION_1: u32 = 64;
pub(crate) const ROYALIST_REBELLION_2: u32 = 65;
pub(crate) const SORRY_ABOUT_REVOLUTION: u32 = 66;
pub(crate) const ALL_YOUR_FAULT: u32 = 67;
pub(crate) const BYE_ROYALIST: u32 = 68;
pub(crate) const GOODBYE_AND_DIE_ROYALIST: u32 = 69;
pub(crate) const NAME_1: u32 = 70;
pub(crate) const OUT_TAKES: u32 = 75;

pub(crate) fn text_layout(screen_width: i32) -> (CPoint, u16) {
    (
        CPoint {
            x: ((screen_width - 78) * 2 / 3) as i16,
            y: 60,
        },
        ((screen_width - 96) * 2 / 3) as u16,
    )
}

#[cfg(test)]
use crate::comm::descriptors::yehat::descriptor;

#[cfg(test)]
mod tests;
