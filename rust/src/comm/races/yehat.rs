//! Yehat dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;

mod api;
mod dialogue;
#[cfg(test)]
mod parity_tests;
mod phrases;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Exit,
    Royalists,
    StartRevolt,
    Home,
    Space,
}

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

const NULL_PHRASE: u32 = 0;
const HOMEWORLD_HELLO_1: u32 = 1;
const HOMEWORLD_HELLO_2: u32 = 2;
const WHATS_UP_HOMEWORLD: u32 = 3;
const GENERAL_INFO_HOMEWORLD_1: u32 = 4;
const GENERAL_INFO_HOMEWORLD_2: u32 = 5;
const I_DEMAND_YOU_ALLY_HOMEWORLD0: u32 = 6;
const I_DEMAND_YOU_ALLY_HOMEWORLD1: u32 = 7;
const I_DEMAND_YOU_ALLY_HOMEWORLD2: u32 = 8;
const I_DEMAND_YOU_ALLY_HOMEWORLD3: u32 = 9;
const ENEMY_MUST_DIE: u32 = 10;
const AT_LEAST_HELP_US_HOMEWORLD: u32 = 11;
const NO_HELP_ENEMY: u32 = 12;
const GIVE_INFO: u32 = 13;
const NO_INFO_FOR_ENEMY: u32 = 14;
const WHAT_ABOUT_PKUNK_ROYALIST: u32 = 15;
const PKUNK_ABSORBED_ROYALIST: u32 = 16;
const HATE_PKUNK_ROYALIST: u32 = 17;
const BYE_HOMEWORLD: u32 = 18;
const GOODBYE_AND_DIE_HOMEWORLD: u32 = 19;
const SPACE_HELLO_1: u32 = 20;
const SPACE_HELLO_2: u32 = 21;
const SPACE_HELLO_3: u32 = 22;
const SPACE_HELLO_4: u32 = 23;
const WHATS_UP_SPACE_1: u32 = 24;
const GENERAL_INFO_SPACE_1: u32 = 25;
const WHATS_UP_SPACE_2: u32 = 26;
const GENERAL_INFO_SPACE_2: u32 = 27;
const WHATS_UP_SPACE_3: u32 = 28;
const GENERAL_INFO_SPACE_3: u32 = 29;
const WHATS_UP_SPACE_4: u32 = 30;
const GENERAL_INFO_SPACE_4: u32 = 31;
const I_DEMAND_YOU_ALLY_SPACE0: u32 = 32;
const I_DEMAND_YOU_ALLY_SPACE1: u32 = 33;
const I_DEMAND_YOU_ALLY_SPACE2: u32 = 34;
const I_DEMAND_YOU_ALLY_SPACE3: u32 = 35;
const WE_CANNOT_1: u32 = 36;
const OBLIGATION: u32 = 37;
const WE_CANNOT_2: u32 = 38;
const AT_LEAST_HELP_US_SPACE: u32 = 39;
const SORRY_CANNOT: u32 = 40;
const DISHONOR: u32 = 41;
const HERES_A_HINT: u32 = 42;
const BYE_SPACE: u32 = 46;
const GO_IN_PEACE: u32 = 47;
const GOODBYE_AND_DIE_SPACE: u32 = 48;
const SHOFIXTI_ALIVE_1: u32 = 49;
const SHOFIXTI_ALIVE_2: u32 = 50;
const SEND_HIM_OVER_1: u32 = 51;
const SEND_HIM_OVER_2: u32 = 52;
const NOT_HERE: u32 = 53;
const NOT_SEND: u32 = 54;
const JUST_A_TRICK_1: u32 = 55;
const JUST_A_TRICK_2: u32 = 56;
const OK_SEND: u32 = 57;
const WE_REVOLT: u32 = 58;
const ROYALIST_SPACE_HELLO_1: u32 = 59;
const ROYALIST_SPACE_HELLO_2: u32 = 60;
const ROYALIST_HOMEWORLD_HELLO_1: u32 = 61;
const ROYALIST_HOMEWORLD_HELLO_2: u32 = 62;
const HOW_IS_REBELLION: u32 = 63;
const ROYALIST_REBELLION_1: u32 = 64;
const ROYALIST_REBELLION_2: u32 = 65;
const SORRY_ABOUT_REVOLUTION: u32 = 66;
const ALL_YOUR_FAULT: u32 = 67;
const BYE_ROYALIST: u32 = 68;
const GOODBYE_AND_DIE_ROYALIST: u32 = 69;
const NAME_1: u32 = 70;
const OUT_TAKES: u32 = 75;

fn text_layout(screen_width: i32) -> (CPoint, u16) {
    (
        CPoint {
            x: ((screen_width - 78) * 2 / 3) as i16,
            y: 60,
        },
        ((screen_width - 96) * 2 / 3) as u16,
    )
}

fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut desc = CLocData {
        alien_frame_res: c"comm.yehat.graphics".as_ptr(),
        alien_font_res: c"comm.yehat.font".as_ptr(),
        alien_colormap_res: c"comm.yehat.colortable".as_ptr(),
        alien_song_res: c"comm.yehat.music".as_ptr(),
        conversation_phrases_res: c"comm.yehat.dialogue".as_ptr(),
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
        alien_text_valign: 1,
        num_animations: 15,
        ..CLocData::default()
    };
    let specs = [
        (
            4,
            3,
            (1 << 2) | (1 << 3),
            (84, 84),
            (210, 420),
            (1 << 6) | (1 << 7),
        ),
        (
            7,
            3,
            (1 << 2) | (1 << 3),
            (84, 84),
            (210, 420),
            (1 << 6) | (1 << 7),
        ),
        (10, 3, 1 << 2, (42, 0), (840, 2520), (1 << 4) | (1 << 14)),
        (13, 3, 1 << 2, (42, 0), (840, 2520), 1 << 5),
        (16, 5, 1 << 2, (56, 56), (5040, 2520), (1 << 2) | (1 << 14)),
        (21, 5, 1 << 2, (56, 56), (5040, 2520), 1 << 3),
        (
            26,
            2,
            (1 << 2) | (1 << 3),
            (56, 56),
            (5040, 2520),
            (1 << 0) | (1 << 1),
        ),
        (
            28,
            2,
            (1 << 2) | (1 << 3),
            (56, 56),
            (5040, 2520),
            (1 << 0) | (1 << 1),
        ),
        (30, 3, 1 << 2, (28, 28), (28, 28), 0),
        (33, 3, 1 << 2, (28, 28), (28, 28), 0),
        (36, 3, 1 << 2, (28, 28), (28, 28), 0),
        (39, 3, 1 << 2, (28, 28), (28, 28), 0),
        (42, 3, 1 << 2, (28, 28), (28, 28), 0),
        (45, 3, 1 << 2, (28, 28), (28, 28), 0),
        (
            48,
            4,
            (1 << 2) | (1 << 3),
            (28, 0),
            (840, 2520),
            (1 << 2) | (1 << 4),
        ),
    ];
    for (index, (start, frames, flags, frame_rate, restart_rate, block_mask)) in
        specs.into_iter().enumerate()
    {
        desc.alien_ambient_array[index] = animation(start, frames, flags, frame_rate, restart_rate);
        desc.alien_ambient_array[index].block_mask = block_mask;
    }
    desc.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::super::script::live::LiveScript;
    use super::*;

    fn run(f: impl FnOnce(&mut dialogue::Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut dialogue::Dialogue { api: &mut api });
    }

    fn exit(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Exit, response as i32));
    }
    fn royalists(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Royalists, response as i32));
    }
    fn start_revolt(response: u32) {
        run(|dialogue| dialogue.handle(Handler::StartRevolt, response as i32));
    }
    fn home(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Home, response as i32));
    }
    fn space(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Space, response as i32));
    }

    fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
        match handler {
            Handler::Exit => exit,
            Handler::Royalists => royalists,
            Handler::StartRevolt => start_revolt,
            Handler::Home => home,
            Handler::Space => space,
        }
    }

    pub(super) fn intro() {
        run(|dialogue| dialogue.intro());
    }
    pub(super) fn post() {}
    pub(super) fn uninit() -> u16 {
        0
    }

    #[no_mangle]
    pub unsafe extern "C" fn init_yehat_comm() -> *mut CLocData {
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

pub struct YehatDialogue;

impl super::RaceDialogue for YehatDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_yehat_comm()) }
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
mod tests;
