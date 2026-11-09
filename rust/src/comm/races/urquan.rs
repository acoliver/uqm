//! Urquan dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

pub(crate) use crate::comm::locdata::CPoint;

pub(crate) mod api;
pub(crate) mod dialogue;
mod phrases;

#[cfg(test)]
mod parity_tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Combat,
    Normal,
    Hypno,
    MentionCompulsion,
    DescribePet,
    DescribeCompulsion,
    Loser,
}
// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

pub(crate) const HELLO_SAMATRA: u32 = 1;
pub(crate) const SENSE_EVIL: u32 = 2;
pub(crate) const INIT_URQUAN_WAKE_UP: u32 = 3;
pub(crate) const WHERE_AM_I: u32 = 4;
pub(crate) const YOU_ARE_HERE: u32 = 5;
pub(crate) const WHY_DOES_MY_HEAD_HURT: u32 = 6;
pub(crate) const HURTS_BECAUSE: u32 = 7;
pub(crate) const WHAT_ABOUT_2_WEEKS: u32 = 8;
pub(crate) const ABOUT_2_WEEKS: u32 = 9;
pub(crate) const COMPULSION: u32 = 10;
pub(crate) const WHAT_COMPULSION: u32 = 11;
pub(crate) const WASCALLY_LITTLE_GUY: u32 = 12;
pub(crate) const WHAT_IT_LOOK_LIKE: u32 = 13;
pub(crate) const TERRAN_AMPHIBIAN: u32 = 14;
pub(crate) const TALKING_PET_ON_STEROIDS: u32 = 15;
pub(crate) const BAD_NEWS: u32 = 16;
pub(crate) const TURD_AND_TOAD: u32 = 17;
pub(crate) const WHAT_IS_TURD_AND_TOAD: u32 = 18;
pub(crate) const ALIEN_MIND_CONTROL: u32 = 19;
pub(crate) const WHAT_FELT_LIKE: u32 = 20;
pub(crate) const POSSESSED_BY_DEVIL: u32 = 21;
pub(crate) const STUPID_DEVIL: u32 = 22;
pub(crate) const FALLING_ASLEEP: u32 = 23;
pub(crate) const SOMEONE_ELSE_CONTROLLED: u32 = 24;
pub(crate) const SOUNDS_FAMILIAR: u32 = 25;
pub(crate) const BEFORE_COFFEE: u32 = 26;
pub(crate) const EXPLAIN: u32 = 27;
pub(crate) const WHY_EXPLAIN: u32 = 28;
pub(crate) const MUST_EXPLAIN: u32 = 29;
pub(crate) const BYE_INIT_HYPNO: u32 = 30;
pub(crate) const GOODBYE_AND_DIE_INIT_HYPNO: u32 = 31;
pub(crate) const SUBSEQUENT_URQUAN_WAKE_UP: u32 = 32;
pub(crate) const UH_OH: u32 = 33;
pub(crate) const NO_UH_OH: u32 = 34;
pub(crate) const STOP_MEETING: u32 = 35;
pub(crate) const NO_STOP_MEETING: u32 = 36;
pub(crate) const BYE_SUB_HYPNO: u32 = 37;
pub(crate) const GOODBYE_AND_DIE_SUB_HYPNO: u32 = 38;
pub(crate) const CAUGHT_YA: u32 = 39;
pub(crate) const INIT_FLEE_HUMAN: u32 = 40;
pub(crate) const SUBSEQUENT_FLEE_HUMAN: u32 = 41;
pub(crate) const WHY_FLEE: u32 = 42;
pub(crate) const FLEE_BECAUSE: u32 = 43;
pub(crate) const WHAT_HAPPENS_NOW: u32 = 44;
pub(crate) const HAPPENS_NOW: u32 = 45;
pub(crate) const WHAT_ABOUT_YOU: u32 = 46;
pub(crate) const ABOUT_US: u32 = 47;
pub(crate) const BYE_WARS_OVER: u32 = 48;
pub(crate) const GOODBYE_WARS_OVER: u32 = 49;
pub(crate) const SEND_MESSAGE: u32 = 50;
pub(crate) const INIT_HELLO: u32 = 51;
pub(crate) const SUBSEQUENT_HELLO_1: u32 = 52;
pub(crate) const SUBSEQUENT_HELLO_2: u32 = 53;
pub(crate) const SUBSEQUENT_HELLO_3: u32 = 54;
pub(crate) const SUBSEQUENT_HELLO_4: u32 = 55;
pub(crate) const YOU_MUST_SURRENDER: u32 = 56;
pub(crate) const NOPE: u32 = 57;
pub(crate) const I_SURRENDER: u32 = 58;
pub(crate) const DISOBEDIENCE_PUNISHED: u32 = 59;
pub(crate) const I_WONT_SURRENDER: u32 = 60;
pub(crate) const BAD_CHOICE: u32 = 61;
pub(crate) const I_WILL_SURRENDER: u32 = 62;
pub(crate) const GOOD_CHOICE: u32 = 63;
pub(crate) const KEY_PHRASE: u32 = 64;
pub(crate) const URQUAN_STORY: u32 = 65;
pub(crate) const LIKE_TO_LEAVE: u32 = 66;
pub(crate) const INDEPENDENCE_IS_BAD: u32 = 67;
pub(crate) const WHATS_UP_1: u32 = 68;
pub(crate) const GENERAL_INFO_1: u32 = 69;
pub(crate) const WHATS_UP_2: u32 = 70;
pub(crate) const GENERAL_INFO_2: u32 = 71;
pub(crate) const WHATS_UP_3: u32 = 72;
pub(crate) const GENERAL_INFO_3: u32 = 73;
pub(crate) const WHATS_UP_4: u32 = 74;
pub(crate) const GENERAL_INFO_4: u32 = 75;
pub(crate) const OUT_TAKES: u32 = 76;

pub(crate) fn text_layout(screen_width: i32) -> (CPoint, u16) {
    let text_width = screen_width - 80;
    (
        CPoint {
            x: (1 + (text_width >> 1)) as i16,
            y: 0,
        },
        (text_width - 16) as u16,
    )
}

#[cfg(test)]
use crate::comm::descriptors::urquan::descriptor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_matches_c_resource_and_animation_configuration() {
        let desc = descriptor(320);
        assert_eq!(desc.num_animations, 7);
        assert_eq!(desc.alien_text_baseline.x, 121);
        assert_eq!(desc.alien_text_width, 224);
        assert_eq!(desc.alien_ambient_array[0].start_index, 7);
        assert_eq!(desc.alien_ambient_array[0].num_frames, 6);
        assert_eq!(desc.alien_ambient_array[1].anim_flags, 1 << 2);
        assert_eq!(desc.alien_ambient_array[6].start_index, 36);
        assert_eq!(desc.alien_ambient_array[6].num_frames, 8);
        assert_eq!(desc.alien_transition_desc.base_frame_rate, 140);
        assert_eq!(desc.alien_talk_desc.base_restart_rate, 70);
    }

    #[test]
    fn urquan_state_keys_exist() {
        for key in [
            "AWARE_OF_SAMATRA",
            "KNOW_URQUAN_STORY",
            "KOHR_AH_FRENZY",
            "MENTIONED_PET_COMPULSION",
            "PLAYER_HYPNOTIZED",
            "URQUAN_VISITS",
            "URQUAN_HYPNO_VISITS",
            "URQUAN_INFO",
            "URQUAN_PROBE_GRPOFFS0",
            "PROBE_MESSAGE_DELIVERED",
        ] {
            assert!(
                crate::engine::StoryKey::from_name(key).is_some(),
                "missing game state key: {key}"
            );
        }
    }
}
