//! Vux dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;

mod dialogue;
#[cfg(test)]
mod parity_tests;
mod phrases;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Combat,
    Menagerie,
    Normal,
}

// ---------------------------------------------------------------------------
// String indices (from strings.h)
// ---------------------------------------------------------------------------

const NULL_PHRASE: u32 = 0;
const ZEX_HELLO_1: u32 = 1;
const ZEX_HELLO_2: u32 = 2;
const ZEX_HELLO_3: u32 = 3;
const ZEX_HELLO_4: u32 = 4;
const FIGHT_OR_TRADE_1: u32 = 5;
const FIGHT_OR_TRADE_2: u32 = 6;
const WHAT_YOU_DO_HERE: u32 = 7;
const MY_MENAGERIE: u32 = 8;
const WHAT_ABOUT_MENAGERIE: u32 = 9;
const NEED_NEW_CREATURE: u32 = 10;
const WHAT_ABOUT_CREATURE: u32 = 11;
const ABOUT_CREATURE: u32 = 12;
const ABOUT_CREATURE_AGAIN: u32 = 13;
const CREATURE_AGAIN: u32 = 14;
const I_HAVE_BEAST: u32 = 15;
const GIVE_BEAST: u32 = 16;
const OK_TAKE_BEAST: u32 = 17;
const FOOL_AIEE0: u32 = 18;
const FOOL_AIEE1: u32 = 19;
const WHY_TRUST_1: u32 = 20;
const TRUST_1: u32 = 21;
const WHY_TRUST_2: u32 = 22;
const TRUST_2: u32 = 23;
const WHY_TRUST_3: u32 = 24;
const TRUST_3: u32 = 25;
const WHY_DONT_YOU_ATTACK: u32 = 26;
const LIKE_YOU: u32 = 27;
const WHY_LIKE_ME: u32 = 28;
const LIKE_BECAUSE: u32 = 29;
const ARE_YOU_A_PERVERT: u32 = 30;
const CALL_ME_WHAT_YOU_WISH: u32 = 31;
const TAKE_BY_FORCE: u32 = 32;
const PRECURSOR_DEVICE: u32 = 33;
const REGARDLESS: u32 = 34;
const THEN_FIGHT: u32 = 35;
const YOU_LIED: u32 = 36;
const YUP_LIED: u32 = 37;
const KILL_YOU: u32 = 38;
const FIGHT_AGAIN: u32 = 39;
const BYE_ZEX: u32 = 40;
const GOODBYE_ZEX: u32 = 41;
const HOMEWORLD_HELLO_1: u32 = 42;
const HOMEWORLD_HELLO_2: u32 = 43;
const HOMEWORLD_HELLO_3: u32 = 44;
const HOMEWORLD_HELLO_4: u32 = 45;
const SPACE_HELLO_1: u32 = 46;
const SPACE_HELLO_2: u32 = 47;
const SPACE_HELLO_3: u32 = 48;
const SPACE_HELLO_4: u32 = 49;
const KILL_YOU_SQUIDS_1: u32 = 50;
const KILL_YOU_SQUIDS_2: u32 = 51;
const KILL_YOU_SQUIDS_3: u32 = 52;
const KILL_YOU_SQUIDS_4: u32 = 53;
const WE_FIGHT: u32 = 54;
const WHY_SO_MEAN: u32 = 55;
const URQUAN_SLAVES: u32 = 56;
const DEEPER_REASON: u32 = 57;
const OLD_INSULT: u32 = 58;
const IF_WE_APOLOGIZE: u32 = 59;
const PROBABLY_NOT: u32 = 60;
const TRY_ANY_WAY: u32 = 61;
const NOPE: u32 = 62;
const APOLOGY_1: u32 = 64;
const NOT_ACCEPTED_1: u32 = 65;
const APOLOGY_2: u32 = 66;
const NOT_ACCEPTED_2: u32 = 67;
const APOLOGY_3: u32 = 68;
const NOT_ACCEPTED_3: u32 = 69;
const APOLOGY_4: u32 = 70;
const NOT_ACCEPTED_4: u32 = 71;
const APOLOGY_5: u32 = 72;
const NOT_ACCEPTED_5: u32 = 73;
const APOLOGY_6: u32 = 74;
const NOT_ACCEPTED_6: u32 = 75;
const APOLOGY_7: u32 = 76;
const NOT_ACCEPTED_7: u32 = 77;
const APOLOGY_8: u32 = 78;
const NOT_ACCEPTED_8: u32 = 79;
const APOLOGY_9: u32 = 80;
const NOT_ACCEPTED_9: u32 = 81;
const APOLOGY_10: u32 = 82;
const TRUTH: u32 = 83;
const WHATS_UP_HOSTILE: u32 = 84;
const GENERAL_INFO_HOSTILE_1: u32 = 85;
const GENERAL_INFO_HOSTILE_2: u32 = 86;
const GENERAL_INFO_HOSTILE_3: u32 = 87;
const GENERAL_INFO_HOSTILE_4: u32 = 88;
const CANT_WE_BE_FRIENDS_1: u32 = 89;
const NEVER_UGLY_HUMANS_1: u32 = 90;
const CANT_WE_BE_FRIENDS_2: u32 = 91;
const NEVER_UGLY_HUMANS_2: u32 = 92;
const CANT_WE_BE_FRIENDS_3: u32 = 93;
const NEVER_UGLY_HUMANS_3: u32 = 94;
const CANT_WE_BE_FRIENDS_4: u32 = 95;
const NEVER_UGLY_HUMANS_4: u32 = 96;
const BYE_HOSTILE_SPACE: u32 = 97;
const GOODBYE_AND_DIE_HOSTILE_SPACE_1: u32 = 98;
const GOODBYE_AND_DIE_HOSTILE_SPACE_2: u32 = 99;
const GOODBYE_AND_DIE_HOSTILE_SPACE_3: u32 = 100;
const GOODBYE_AND_DIE_HOSTILE_SPACE_4: u32 = 101;
const OUT_TAKES: u32 = 102;

fn text_layout(screen_width: i32) -> (CPoint, u16) {
    let text_width = screen_width - 80;
    (
        CPoint {
            x: (1 + (text_width >> 1) + (text_width >> 2)) as i16,
            y: 0,
        },
        ((text_width - 16) >> 1) as u16,
    )
}

fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut desc = CLocData {
        alien_frame_res: c"comm.vux.graphics".as_ptr(),
        alien_font_res: c"comm.vux.font".as_ptr(),
        alien_colormap_res: c"comm.vux.colortable".as_ptr(),
        alien_song_res: c"comm.vux.music".as_ptr(),
        conversation_phrases_res: c"comm.vux.dialogue".as_ptr(),
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
        num_animations: 17,
        ..CLocData::default()
    };
    let specs = [
        (12, 3, 1 << 0, (28, 28), (28, 28), 0),
        (15, 5, 1 << 0, (28, 28), (28, 28), 0),
        (20, 14, 1 << 1, (28, 0), (28, 0), 0),
        (34, 7, 1 << 1, (28, 28), (28, 28), 0),
        (41, 6, 1 << 0, (28, 28), (28, 28), 0),
        (47, 11, 1 << 1, (28, 28), (28, 28), 0),
        (58, 3, 1 << 0, (28, 28), (28, 28), 0),
        (61, 4, 1 << 0, (28, 28), (28, 28), 0),
        (65, 4, 1 << 0, (28, 28), (28, 28), 0),
        (69, 2, 1 << 0, (28, 28), (28, 28), 0),
        (71, 3, 1 << 2, (42, 0), (840, 2520), 0),
        (74, 6, 1 << 2, (56, 56), (56, 56), 0),
        (80, 5, 1 << 2, (56, 56), (56, 56), 1 << 14),
        (85, 5, 1 << 2, (56, 56), (56, 56), 0),
        (90, 5, 1 << 2, (56, 56), (56, 56), 1 << 12),
        (95, 4, 1 << 2, (56, 56), (4200, 4200), 0),
        (99, 4, 1 << 2, (56, 56), (4200, 4200), 0),
    ];
    for (index, (start, frames, flags, frame_rate, restart_rate, block_mask)) in
        specs.into_iter().enumerate()
    {
        desc.alien_ambient_array[index] = animation(start, frames, flags, frame_rate, restart_rate);
        desc.alien_ambient_array[index].block_mask = block_mask;
    }
    desc.alien_talk_desc = animation(1, 11, 0, (56, 0), (70, 0));
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

    fn combat(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Combat, response as i32));
    }
    fn menagerie(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Menagerie, response as i32));
    }
    fn normal(response: u32) {
        run(|dialogue| dialogue.handle(Handler::Normal, response as i32));
    }

    fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
        match handler {
            Handler::Combat => combat,
            Handler::Menagerie => menagerie,
            Handler::Normal => normal,
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
    pub unsafe extern "C" fn init_vux_comm() -> *mut CLocData {
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

pub struct VuxDialogue;

impl super::RaceDialogue for VuxDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_vux_comm()) }
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
