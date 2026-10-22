//! Slyland dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

use super::script::as_comm_data;
use crate::comm::types::CommData;

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

const WE_COME_IN_PEACE_1: u32 = 1;
const WE_COME_IN_PEACE_2: u32 = 2;
const WE_COME_IN_PEACE_3: u32 = 3;
const WE_COME_IN_PEACE_4: u32 = 4;
const WE_COME_IN_PEACE_5: u32 = 5;
const WE_COME_IN_PEACE_6: u32 = 6;
const WE_COME_IN_PEACE_7: u32 = 7;
const WE_COME_IN_PEACE_8: u32 = 8;
const THREAT_1: u32 = 9;
const THREAT_2: u32 = 10;
const THREAT_3: u32 = 11;
const THREAT_4: u32 = 12;
const PROGRAMMED_TO_DEFEND_1: u32 = 13;
const PROGRAMMED_TO_DEFEND_2: u32 = 14;
const PROGRAMMED_TO_DEFEND_3: u32 = 15;
const PROGRAMMED_TO_DEFEND_4: u32 = 16;
const SOMETHING_WRONG_1: u32 = 17;
const SOMETHING_WRONG_2: u32 = 18;
const SOMETHING_WRONG_3: u32 = 19;
const SOMETHING_WRONG_4: u32 = 20;
const NOMINAL_FUNCTION_1: u32 = 21;
const NOMINAL_FUNCTION_2: u32 = 22;
const NOMINAL_FUNCTION_3: u32 = 23;
const NOMINAL_FUNCTION_4: u32 = 24;
const WE_ARE_US_1: u32 = 25;
const WE_ARE_US_2: u32 = 26;
const WE_ARE_US_3: u32 = 27;
const WE_ARE_US_4: u32 = 28;
const THIS_IS_PROBE_1: u32 = 29;
const THIS_IS_PROBE_2: u32 = 30;
const THIS_IS_PROBE_3: u32 = 31;
const THIS_IS_PROBE_40: u32 = 32;
const THIS_IS_PROBE_41: u32 = 33;
const THIS_IS_PROBE_42: u32 = 34;
const WHY_ATTACK_1: u32 = 35;
const WHY_ATTACK_2: u32 = 36;
const WHY_ATTACK_3: u32 = 37;
const WHY_ATTACK_4: u32 = 38;
const PEACEFUL_MISSION_1: u32 = 39;
const PEACEFUL_MISSION_2: u32 = 40;
const PEACEFUL_MISSION_3: u32 = 41;
const PEACEFUL_MISSION_4: u32 = 42;
const BYE_1: u32 = 43;
const BYE_2: u32 = 44;
const BYE_3: u32 = 45;
const BYE_4: u32 = 46;
const GOODBYE_1: u32 = 47;
const GOODBYE_2: u32 = 48;
const GOODBYE_3: u32 = 49;
const GOODBYE_4: u32 = 50;
const HOSTILE: u32 = 51;
const DESTRUCT_SEQUENCE: u32 = 52;
const DESTRUCT_CODE: u32 = 53;
const COORD_PLUS: u32 = 54;
const COORD_MINUS: u32 = 55;
const COORD_POINT: u32 = 56;
const ENUMERATE_HUNDRED: u32 = 57;
const ENUMERATE_THOUSAND: u32 = 58;
const ENUMERATE_ZERO: u32 = 59;
const ENUMERATE_ONE: u32 = 60;
const ENUMERATE_TWO: u32 = 61;
const ENUMERATE_THREE: u32 = 62;
const ENUMERATE_FOUR: u32 = 63;
const ENUMERATE_FIVE: u32 = 64;
const ENUMERATE_SIX: u32 = 65;
const ENUMERATE_SEVEN: u32 = 66;
const ENUMERATE_EIGHT: u32 = 67;
const ENUMERATE_NINE: u32 = 68;
const ENUMERATE_TEN: u32 = 69;
const ENUMERATE_ELEVEN: u32 = 70;
const ENUMERATE_TWELVE: u32 = 71;
const ENUMERATE_THIRTEEN: u32 = 72;
const ENUMERATE_FOURTEEN: u32 = 73;
const ENUMERATE_FIFTEEN: u32 = 74;
const ENUMERATE_SIXTEEN: u32 = 75;
const ENUMERATE_SEVENTEEN: u32 = 76;
const ENUMERATE_EIGHTEEN: u32 = 77;
const ENUMERATE_NINETEEN: u32 = 78;
const ENUMERATE_TWENTY: u32 = 79;
const ENUMERATE_THIRTY: u32 = 80;
const ENUMERATE_FOURTY: u32 = 81;
const ENUMERATE_FIFTY: u32 = 82;
const ENUMERATE_SIXTY: u32 = 83;
const ENUMERATE_SEVENTY: u32 = 84;
const ENUMERATE_EIGHTY: u32 = 85;
const ENUMERATE_NINETY: u32 = 86;

mod api;
use api::SlylandApi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Combat,
}

struct Dialogue<'a, A> {
    api: &'a mut A,
}

mod logic;
mod numbers;

// ---------------------------------------------------------------------------
// Resource keys (from resinst.h)
// ---------------------------------------------------------------------------

const RACE_PMAP_ANIM: &[u8] = b"comm.probe.graphics\0";
const RACE_FONT: &[u8] = b"comm.probe.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.probe.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.probe.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.probe.dialogue\0";
mod descriptor;
use descriptor::descriptor;

#[cfg(not(test))]
mod live {
    use super::super::script::live::LiveScript;
    use super::*;

    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }

    extern "C" fn combat(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Combat, response as i32));
    }

    fn callback(_: Handler) -> extern "C" fn(u32) {
        combat
    }

    pub(super) extern "C" fn intro() {
        run(|dialogue| dialogue.intro());
    }
    pub(super) extern "C" fn post() {}
    pub(super) extern "C" fn uninit() -> u16 {
        0
    }
    #[no_mangle]
    pub unsafe extern "C" fn init_slyland_comm() -> *mut crate::comm::locdata::CLocData {
        static mut DESC: Option<crate::comm::locdata::CLocData> = None;
        extern "C" {
            static ScreenWidth: i32;
        }
        let d = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(|| descriptor(ScreenWidth));
        d.alien_text_baseline.x = (1 + ((ScreenWidth - 80) >> 1)) as i16;
        d.alien_text_width = (ScreenWidth - 80 - 16) as u16;
        run(|s| s.initialize());
        d
    }
}

#[cfg(test)]
#[path = "slyland/tests.rs"]
mod branch_tests;

/// Slyland race dialogue implementation.
pub struct SlylandDialogue;

impl super::RaceDialogue for SlylandDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_slyland_comm()) }
        }
        #[cfg(test)]
        {
            as_comm_data(&descriptor(320))
        }
    }

    fn intro(&self) {
        #[cfg(not(test))]
        live::intro();
    }

    fn post_encounter(&self) {}

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
            crate::state::game_state_keys::bit_range("DESTRUCT_CODE_ON_SHIP").is_some(),
            "missing game state key: DESTRUCT_CODE_ON_SHIP"
        );
        assert!(
            crate::state::game_state_keys::bit_range("PROBE_EXHIBITED_BUG").is_some(),
            "missing game state key: PROBE_EXHIBITED_BUG"
        );
        assert!(
            crate::state::game_state_keys::bit_range("SLYLANDRO_PROBE_EXIT").is_some(),
            "missing game state key: SLYLANDRO_PROBE_EXIT"
        );
        assert!(
            crate::state::game_state_keys::bit_range("SLYLANDRO_PROBE_ID").is_some(),
            "missing game state key: SLYLANDRO_PROBE_ID"
        );
        assert!(
            crate::state::game_state_keys::bit_range("SLYLANDRO_PROBE_INFO").is_some(),
            "missing game state key: SLYLANDRO_PROBE_INFO"
        );
    }
}
