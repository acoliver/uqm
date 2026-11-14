//! Slyland dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

pub(crate) const WE_COME_IN_PEACE_1: u32 = 1;
pub(crate) const WE_COME_IN_PEACE_2: u32 = 2;
pub(crate) const WE_COME_IN_PEACE_3: u32 = 3;
pub(crate) const WE_COME_IN_PEACE_4: u32 = 4;
pub(crate) const WE_COME_IN_PEACE_5: u32 = 5;
pub(crate) const WE_COME_IN_PEACE_6: u32 = 6;
pub(crate) const WE_COME_IN_PEACE_7: u32 = 7;
pub(crate) const WE_COME_IN_PEACE_8: u32 = 8;
pub(crate) const THREAT_1: u32 = 9;
pub(crate) const THREAT_2: u32 = 10;
pub(crate) const THREAT_3: u32 = 11;
pub(crate) const THREAT_4: u32 = 12;
pub(crate) const PROGRAMMED_TO_DEFEND_1: u32 = 13;
pub(crate) const PROGRAMMED_TO_DEFEND_2: u32 = 14;
pub(crate) const PROGRAMMED_TO_DEFEND_3: u32 = 15;
pub(crate) const PROGRAMMED_TO_DEFEND_4: u32 = 16;
pub(crate) const SOMETHING_WRONG_1: u32 = 17;
pub(crate) const SOMETHING_WRONG_2: u32 = 18;
pub(crate) const SOMETHING_WRONG_3: u32 = 19;
pub(crate) const SOMETHING_WRONG_4: u32 = 20;
pub(crate) const NOMINAL_FUNCTION_1: u32 = 21;
pub(crate) const NOMINAL_FUNCTION_2: u32 = 22;
pub(crate) const NOMINAL_FUNCTION_3: u32 = 23;
pub(crate) const NOMINAL_FUNCTION_4: u32 = 24;
pub(crate) const WE_ARE_US_1: u32 = 25;
pub(crate) const WE_ARE_US_2: u32 = 26;
pub(crate) const WE_ARE_US_3: u32 = 27;
pub(crate) const WE_ARE_US_4: u32 = 28;
pub(crate) const THIS_IS_PROBE_1: u32 = 29;
pub(crate) const THIS_IS_PROBE_2: u32 = 30;
pub(crate) const THIS_IS_PROBE_3: u32 = 31;
pub(crate) const THIS_IS_PROBE_40: u32 = 32;
pub(crate) const THIS_IS_PROBE_41: u32 = 33;
pub(crate) const THIS_IS_PROBE_42: u32 = 34;
pub(crate) const WHY_ATTACK_1: u32 = 35;
pub(crate) const WHY_ATTACK_2: u32 = 36;
pub(crate) const WHY_ATTACK_3: u32 = 37;
pub(crate) const WHY_ATTACK_4: u32 = 38;
pub(crate) const PEACEFUL_MISSION_1: u32 = 39;
pub(crate) const PEACEFUL_MISSION_2: u32 = 40;
pub(crate) const PEACEFUL_MISSION_3: u32 = 41;
pub(crate) const PEACEFUL_MISSION_4: u32 = 42;
pub(crate) const BYE_1: u32 = 43;
pub(crate) const BYE_2: u32 = 44;
pub(crate) const BYE_3: u32 = 45;
pub(crate) const BYE_4: u32 = 46;
pub(crate) const GOODBYE_1: u32 = 47;
pub(crate) const GOODBYE_2: u32 = 48;
pub(crate) const GOODBYE_3: u32 = 49;
pub(crate) const GOODBYE_4: u32 = 50;
pub(crate) const HOSTILE: u32 = 51;
pub(crate) const DESTRUCT_SEQUENCE: u32 = 52;
pub(crate) const DESTRUCT_CODE: u32 = 53;
pub(crate) const COORD_PLUS: u32 = 54;
pub(crate) const COORD_MINUS: u32 = 55;
pub(crate) const COORD_POINT: u32 = 56;
pub(crate) const ENUMERATE_HUNDRED: u32 = 57;
pub(crate) const ENUMERATE_THOUSAND: u32 = 58;
pub(crate) const ENUMERATE_ZERO: u32 = 59;
pub(crate) const ENUMERATE_ONE: u32 = 60;
pub(crate) const ENUMERATE_TWO: u32 = 61;
pub(crate) const ENUMERATE_THREE: u32 = 62;
pub(crate) const ENUMERATE_FOUR: u32 = 63;
pub(crate) const ENUMERATE_FIVE: u32 = 64;
pub(crate) const ENUMERATE_SIX: u32 = 65;
pub(crate) const ENUMERATE_SEVEN: u32 = 66;
pub(crate) const ENUMERATE_EIGHT: u32 = 67;
pub(crate) const ENUMERATE_NINE: u32 = 68;
pub(crate) const ENUMERATE_TEN: u32 = 69;
pub(crate) const ENUMERATE_ELEVEN: u32 = 70;
pub(crate) const ENUMERATE_TWELVE: u32 = 71;
pub(crate) const ENUMERATE_THIRTEEN: u32 = 72;
pub(crate) const ENUMERATE_FOURTEEN: u32 = 73;
pub(crate) const ENUMERATE_FIFTEEN: u32 = 74;
pub(crate) const ENUMERATE_SIXTEEN: u32 = 75;
pub(crate) const ENUMERATE_SEVENTEEN: u32 = 76;
pub(crate) const ENUMERATE_EIGHTEEN: u32 = 77;
pub(crate) const ENUMERATE_NINETEEN: u32 = 78;
pub(crate) const ENUMERATE_TWENTY: u32 = 79;
pub(crate) const ENUMERATE_THIRTY: u32 = 80;
pub(crate) const ENUMERATE_FOURTY: u32 = 81;
pub(crate) const ENUMERATE_FIFTY: u32 = 82;
pub(crate) const ENUMERATE_SIXTY: u32 = 83;
pub(crate) const ENUMERATE_SEVENTY: u32 = 84;
pub(crate) const ENUMERATE_EIGHTY: u32 = 85;
pub(crate) const ENUMERATE_NINETY: u32 = 86;

pub(crate) mod api;
pub(crate) use api::SlylandApi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Combat,
}

pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}

mod logic;
pub(crate) mod numbers;

// ---------------------------------------------------------------------------
// Resource keys (from resinst.h)
// ---------------------------------------------------------------------------

pub(crate) const RACE_PMAP_ANIM: &std::ffi::CStr = c"comm.probe.graphics";
pub(crate) const RACE_FONT: &std::ffi::CStr = c"comm.probe.font";
pub(crate) const RACE_COLOR_MAP: &std::ffi::CStr = c"comm.probe.colortable";
pub(crate) const RACE_MUSIC: &std::ffi::CStr = c"comm.probe.music";
pub(crate) const RACE_CONVERSATION_PHRASES: &std::ffi::CStr = c"comm.probe.dialogue";

#[cfg(test)]
#[path = "slyland/tests.rs"]
mod branch_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_keys_are_null_terminated() {
        assert_eq!(RACE_PMAP_ANIM.to_bytes_with_nul().last(), Some(&0));
        assert_eq!(RACE_FONT.to_bytes_with_nul().last(), Some(&0));
        assert_eq!(RACE_COLOR_MAP.to_bytes_with_nul().last(), Some(&0));
        assert_eq!(RACE_MUSIC.to_bytes_with_nul().last(), Some(&0));
    }

    #[test]
    fn test_game_state_keys_exist() {
        assert!(
            crate::engine::StoryKey::from_name("DESTRUCT_CODE_ON_SHIP").is_some(),
            "missing game state key: DESTRUCT_CODE_ON_SHIP"
        );
        assert!(
            crate::engine::StoryKey::from_name("PROBE_EXHIBITED_BUG").is_some(),
            "missing game state key: PROBE_EXHIBITED_BUG"
        );
        assert!(
            crate::engine::StoryKey::from_name("SLYLANDRO_PROBE_EXIT").is_some(),
            "missing game state key: SLYLANDRO_PROBE_EXIT"
        );
        assert!(
            crate::engine::StoryKey::from_name("SLYLANDRO_PROBE_ID").is_some(),
            "missing game state key: SLYLANDRO_PROBE_ID"
        );
        assert!(
            crate::engine::StoryKey::from_name("SLYLANDRO_PROBE_INFO").is_some(),
            "missing game state key: SLYLANDRO_PROBE_INFO"
        );
    }
}

pub(crate) mod descriptor;
