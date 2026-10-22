//! Urquan dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;

mod api;
mod dialogue;
mod phrases;

#[cfg(test)]
mod parity_tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
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

const HELLO_SAMATRA: u32 = 1;
const SENSE_EVIL: u32 = 2;
const INIT_URQUAN_WAKE_UP: u32 = 3;
const WHERE_AM_I: u32 = 4;
const YOU_ARE_HERE: u32 = 5;
const WHY_DOES_MY_HEAD_HURT: u32 = 6;
const HURTS_BECAUSE: u32 = 7;
const WHAT_ABOUT_2_WEEKS: u32 = 8;
const ABOUT_2_WEEKS: u32 = 9;
const COMPULSION: u32 = 10;
const WHAT_COMPULSION: u32 = 11;
const WASCALLY_LITTLE_GUY: u32 = 12;
const WHAT_IT_LOOK_LIKE: u32 = 13;
const TERRAN_AMPHIBIAN: u32 = 14;
const TALKING_PET_ON_STEROIDS: u32 = 15;
const BAD_NEWS: u32 = 16;
const TURD_AND_TOAD: u32 = 17;
const WHAT_IS_TURD_AND_TOAD: u32 = 18;
const ALIEN_MIND_CONTROL: u32 = 19;
const WHAT_FELT_LIKE: u32 = 20;
const POSSESSED_BY_DEVIL: u32 = 21;
const STUPID_DEVIL: u32 = 22;
const FALLING_ASLEEP: u32 = 23;
const SOMEONE_ELSE_CONTROLLED: u32 = 24;
const SOUNDS_FAMILIAR: u32 = 25;
const BEFORE_COFFEE: u32 = 26;
const EXPLAIN: u32 = 27;
const WHY_EXPLAIN: u32 = 28;
const MUST_EXPLAIN: u32 = 29;
const BYE_INIT_HYPNO: u32 = 30;
const GOODBYE_AND_DIE_INIT_HYPNO: u32 = 31;
const SUBSEQUENT_URQUAN_WAKE_UP: u32 = 32;
const UH_OH: u32 = 33;
const NO_UH_OH: u32 = 34;
const STOP_MEETING: u32 = 35;
const NO_STOP_MEETING: u32 = 36;
const BYE_SUB_HYPNO: u32 = 37;
const GOODBYE_AND_DIE_SUB_HYPNO: u32 = 38;
const CAUGHT_YA: u32 = 39;
const INIT_FLEE_HUMAN: u32 = 40;
const SUBSEQUENT_FLEE_HUMAN: u32 = 41;
const WHY_FLEE: u32 = 42;
const FLEE_BECAUSE: u32 = 43;
const WHAT_HAPPENS_NOW: u32 = 44;
const HAPPENS_NOW: u32 = 45;
const WHAT_ABOUT_YOU: u32 = 46;
const ABOUT_US: u32 = 47;
const BYE_WARS_OVER: u32 = 48;
const GOODBYE_WARS_OVER: u32 = 49;
const SEND_MESSAGE: u32 = 50;
const INIT_HELLO: u32 = 51;
const SUBSEQUENT_HELLO_1: u32 = 52;
const SUBSEQUENT_HELLO_2: u32 = 53;
const SUBSEQUENT_HELLO_3: u32 = 54;
const SUBSEQUENT_HELLO_4: u32 = 55;
const YOU_MUST_SURRENDER: u32 = 56;
const NOPE: u32 = 57;
const I_SURRENDER: u32 = 58;
const DISOBEDIENCE_PUNISHED: u32 = 59;
const I_WONT_SURRENDER: u32 = 60;
const BAD_CHOICE: u32 = 61;
const I_WILL_SURRENDER: u32 = 62;
const GOOD_CHOICE: u32 = 63;
const KEY_PHRASE: u32 = 64;
const URQUAN_STORY: u32 = 65;
const LIKE_TO_LEAVE: u32 = 66;
const INDEPENDENCE_IS_BAD: u32 = 67;
const WHATS_UP_1: u32 = 68;
const GENERAL_INFO_1: u32 = 69;
const WHATS_UP_2: u32 = 70;
const GENERAL_INFO_2: u32 = 71;
const WHATS_UP_3: u32 = 72;
const GENERAL_INFO_3: u32 = 73;
const WHATS_UP_4: u32 = 74;
const GENERAL_INFO_4: u32 = 75;
const OUT_TAKES: u32 = 76;

fn text_layout(screen_width: i32) -> (CPoint, u16) {
    let text_width = screen_width - 80;
    (
        CPoint {
            x: (1 + (text_width >> 1)) as i16,
            y: 0,
        },
        (text_width - 16) as u16,
    )
}

fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CLocData {
        alien_frame_res: c"comm.urquan.graphics".as_ptr(),
        alien_font_res: c"comm.urquan.font".as_ptr(),
        alien_colormap_res: c"comm.urquan.colortable".as_ptr(),
        alien_song_res: c"comm.urquan.music".as_ptr(),
        conversation_phrases_res: c"comm.urquan.dialogue".as_ptr(),
        alien_text_fcolor: CColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        alien_text_bcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: baseline,
        alien_text_width: width,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 7,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(7, 6, 1 << 1, (84, 0), (84, 0));
    for (index, start) in [13, 16, 19].into_iter().enumerate() {
        d.alien_ambient_array[index + 1] = animation(start, 3, 1 << 2, (42, 0), (840, 2520));
    }
    for (index, (start, frames)) in [(22, 7), (29, 7), (36, 8)].into_iter().enumerate() {
        d.alien_ambient_array[index + 4] = animation(start, frames, 1 << 1, (56, 0), (840, 2520));
    }
    d.alien_transition_desc = animation(1, 2, 0, (140, 0), (0, 0));
    d.alien_talk_desc = animation(2, 5, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = live::intro as *const () as *const _;
        d.post_encounter_func = live::post as *const () as *const _;
        d.uninit_encounter_func = live::uninit as *const () as *const _;
    }
    d
}

#[cfg(not(test))]
mod live {
    use super::super::script::live::LiveScript;
    use super::*;

    fn run(f: impl FnOnce(&mut dialogue::Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut dialogue::Dialogue { api: &mut api });
    }

    extern "C" fn combat(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Combat, response as i32));
    }
    extern "C" fn normal(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Normal, response as i32));
    }
    extern "C" fn hypno(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Hypno, response as i32));
    }
    extern "C" fn mention_compulsion(response: u32) {
        run(|dialogue| dialogue.handle(Handler::MentionCompulsion, response as i32));
    }
    extern "C" fn describe_pet(response: u32) {
        run(|dialogue| dialogue.handle(Handler::DescribePet, response as i32));
    }
    extern "C" fn describe_compulsion(response: u32) {
        run(|dialogue| dialogue.handle(Handler::DescribeCompulsion, response as i32));
    }
    extern "C" fn loser(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Loser, response as i32));
    }

    fn callback(handler: Handler) -> extern "C" fn(u32) {
        match handler {
            Handler::Combat => combat,
            Handler::Normal => normal,
            Handler::Hypno => hypno,
            Handler::MentionCompulsion => mention_compulsion,
            Handler::DescribePet => describe_pet,
            Handler::DescribeCompulsion => describe_compulsion,
            Handler::Loser => loser,
        }
    }

    pub(super) extern "C" fn intro() {
        run(|dialogue| dialogue.intro());
    }
    pub(super) extern "C" fn post() {
        run(|dialogue| dialogue.post());
    }
    pub(super) extern "C" fn uninit() -> u16 {
        0
    }

    #[no_mangle]
    pub unsafe extern "C" fn init_urquan_comm() -> *mut CLocData {
        static mut DESCRIPTOR: Option<CLocData> = None;
        extern "C" {
            static ScreenWidth: i32;
        }
        let descriptor = (&mut *std::ptr::addr_of_mut!(DESCRIPTOR))
            .get_or_insert_with(|| super::descriptor(ScreenWidth));
        (descriptor.alien_text_baseline, descriptor.alien_text_width) = text_layout(ScreenWidth);
        run(|dialogue| dialogue.initialize());
        descriptor
    }
}

pub struct UrquanDialogue;

impl super::RaceDialogue for UrquanDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_urquan_comm()) }
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

    fn post_encounter(&self) {
        #[cfg(not(test))]
        live::post();
    }

    fn uninit(&self) -> u32 {
        #[cfg(not(test))]
        return live::uninit() as u32;
        #[cfg(test)]
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::state::game_state_keys::bit_range;

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
            assert!(bit_range(key).is_some(), "missing game state key: {key}");
        }
    }
}
