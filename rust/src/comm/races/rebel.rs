//! Yehat Rebel dialogue, ported from rebel.c.
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
mod api;
use api::RebelApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Exit,
    Info,
    Rebels,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: RebelApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::Exit => {
                if r == BYE_REBEL {
                    self.api.npc(GOODBYE_REBEL)
                }
            }
            Handler::Info => self.info(r),
            Handler::Rebels => self.rebels(r),
        }
    }
    fn info(&mut self, r: i32) {
        if r == GIVE_INFO_REBELS {
            self.api.npc(WHAT_INFO)
        } else if r == WHAT_ABOUT_URQUAN {
            self.api.npc(ABOUT_URQUAN);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_ROYALTY {
            self.api.npc(ABOUT_ROYALTY);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_WAR {
            self.api.npc(ABOUT_WAR);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_VUX {
            self.api.npc(ABOUT_VUX);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_CLUE {
            self.api.npc(ABOUT_CLUE);
            self.api.disable(r as u16)
        }
        let mut any = false;
        for p in [
            WHAT_ABOUT_URQUAN,
            WHAT_ABOUT_ROYALTY,
            WHAT_ABOUT_WAR,
            WHAT_ABOUT_VUX,
            WHAT_ABOUT_CLUE,
        ] {
            if self.api.enabled(p as u16) {
                self.api.response(p as u16, Handler::Info, &[]);
                any = true
            }
        }
        self.api.response(ENOUGH_INFO as u16, Handler::Rebels, &[]);
        if !any {
            self.api.disable(GIVE_INFO_REBELS as u16)
        }
    }
    fn rebels(&mut self, r: i32) {
        if r == HOW_GOES_REVOLUTION {
            let n = self.api.get("YEHAT_REBEL_INFO");
            if let Some(&phrase) = [
                REBEL_REVOLUTION_1,
                REBEL_REVOLUTION_2,
                REBEL_REVOLUTION_3,
                REBEL_REVOLUTION_4,
            ]
            .get(usize::from(n))
            {
                self.api.npc(phrase);
            }
            self.api.set(
                "YEHAT_REBEL_INFO",
                if n == 3 { n } else { n.wrapping_add(1) },
            );
            self.api.disable(r as u16)
        } else if r == ANY_SHIPS {
            if self.api.get("YEHAT_SHIP_MONTH") != 0 && self.api.ship_not_ready() {
                self.api.npc(NO_SHIPS_YET)
            } else {
                let n = self
                    .api
                    .escort_feasibility(crate::comm::dispatch::ship::YEHAT_SHIP as u16)
                    as i8;
                if n == 0 {
                    self.api.npc(NO_ROOM)
                } else {
                    let count = n.min(4);
                    self.api.npc(if n < 4 {
                        HAVE_FEW_SHIPS
                    } else {
                        HAVE_ALL_SHIPS
                    });
                    self.api.talk_segue(u16::MAX);
                    self.api
                        .add_escorts(crate::comm::dispatch::ship::YEHAT_SHIP as u16, count as i16);
                    self.api.prepare_ship()
                }
            }
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_PKUNK_REBEL {
            self.api.npc(if self.api.get("YEHAT_ABSORBED_PKUNK") != 0 {
                PKUNK_ABSORBED_REBEL
            } else {
                HATE_PKUNK_REBEL
            });
            self.api.set("YEHAT_REBEL_TOLD_PKUNK", 1)
        } else if r == ENOUGH_INFO {
            self.api.npc(OK_ENOUGH_INFO)
        }
        if self.api.enabled(HOW_GOES_REVOLUTION as u16) {
            self.api
                .response(HOW_GOES_REVOLUTION as u16, Handler::Rebels, &[])
        }
        if self.api.get("YEHAT_REBEL_TOLD_PKUNK") == 0
            && self.api.get("PKUNK_VISITS") != 0
            && self.api.get("PKUNK_HOME_VISITS") != 0
        {
            self.api
                .response(WHAT_ABOUT_PKUNK_REBEL as u16, Handler::Rebels, &[])
        }
        if self.api.enabled(ANY_SHIPS as u16) {
            self.api.response(ANY_SHIPS as u16, Handler::Rebels, &[])
        }
        if self.api.enabled(GIVE_INFO_REBELS as u16) {
            self.api
                .response(GIVE_INFO_REBELS as u16, Handler::Info, &[])
        }
        self.api.response(BYE_REBEL as u16, Handler::Exit, &[])
    }
    fn intro(&mut self) {
        self.api.segue(0);
        if self.api.activity() as u8 == crate::comm::dispatch::IN_LAST_BATTLE as u8 {
            self.api.npc(YEHAT_CAVALRY);
            self.api.talk_segue(u16::MAX);
            let n = (self
                .api
                .escort_feasibility(crate::comm::dispatch::ship::YEHAT_REBEL_SHIP as u16)
                as u8)
                .min(8);
            self.api.add_escorts(
                crate::comm::dispatch::ship::YEHAT_REBEL_SHIP as u16,
                (n - (n >> 1)) as i16,
            );
            self.api.add_escorts(
                crate::comm::dispatch::ship::PKUNK_SHIP as u16,
                (n >> 1) as i16,
            )
        } else {
            let n = self.api.get("YEHAT_REBEL_VISITS");
            if let Some(&phrase) =
                [REBEL_HELLO_1, REBEL_HELLO_2, REBEL_HELLO_3, REBEL_HELLO_4].get(usize::from(n))
            {
                self.api.npc(phrase);
            }
            self.api.set(
                "YEHAT_REBEL_VISITS",
                if n == 3 { n } else { n.wrapping_add(1) },
            );
            self.rebels(0)
        }
    }
    fn initialize(&mut self) {
        self.api.segue(0)
    }
}
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
    let mut d = CLocData {
        alien_frame_res: c"comm.yehat.graphics".as_ptr(),
        alien_font_res: c"comm.yehat.font".as_ptr(),
        alien_colormap_res: c"comm.yehat.colortable".as_ptr(),
        alien_song_res: c"comm.yehat.music".as_ptr(),
        alien_alt_song_res: c"comm.yehat.rebel.music".as_ptr(),
        conversation_phrases_res: c"comm.yehat.rebel.dialogue".as_ptr(),
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
        alien_song_flags: 1,
        num_animations: 15,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(4, 3, 0x0c, (84, 84), (840, 2520));
    d.alien_ambient_array[0].block_mask = (1 << 6) | (1 << 7);
    d.alien_ambient_array[1] = animation(7, 3, 0x0c, (84, 84), (840, 2520));
    d.alien_ambient_array[1].block_mask = (1 << 6) | (1 << 7);
    d.alien_ambient_array[2] = animation(10, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[2].block_mask = (1 << 4) | (1 << 14);
    d.alien_ambient_array[3] = animation(13, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[3].block_mask = 1 << 5;
    d.alien_ambient_array[4] = animation(16, 5, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[4].block_mask = (1 << 2) | (1 << 14);
    d.alien_ambient_array[5] = animation(21, 5, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[5].block_mask = 1 << 3;
    d.alien_ambient_array[6] = animation(26, 2, 0x0c, (56, 56), (8400, 2520));
    d.alien_ambient_array[6].block_mask = (1 << 0) | (1 << 1);
    d.alien_ambient_array[7] = animation(28, 2, 0x0c, (56, 56), (8400, 2520));
    d.alien_ambient_array[7].block_mask = (1 << 0) | (1 << 1);
    for (i, start) in (30..=45).step_by(3).enumerate() {
        d.alien_ambient_array[i + 8] = animation(start, 3, 4, (28, 28), (28, 28));
    }
    d.alien_ambient_array[14] = animation(48, 4, 0x0c, (28, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = (1 << 2) | (1 << 4);
    d.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}
#[cfg(not(test))]
pub(crate) mod live {
    use super::super::script::live::LiveScript;
    use super::*;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut a = LiveScript { callback };
        f(&mut Dialogue { api: &mut a })
    }
    fn exit(r: u32) {
        run(|s| s.handle(Handler::Exit, r as i32))
    }
    fn info(r: u32) {
        run(|s| s.handle(Handler::Info, r as i32))
    }
    fn rebels(r: u32) {
        run(|s| s.handle(Handler::Rebels, r as i32))
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::Exit => exit,
            Handler::Info => info,
            Handler::Rebels => rebels,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro())
    }
    pub(super) fn post() {}
    pub(super) fn uninit() -> u16 {
        0
    }
    #[no_mangle]
    pub unsafe extern "C" fn init_rebel_yehat_comm() -> *mut CLocData {
        static mut D: Option<CLocData> = None;
        extern "C" {
            static ScreenWidth: i32;
        }
        let d = (&mut *std::ptr::addr_of_mut!(D)).get_or_insert_with(|| descriptor(ScreenWidth));
        (d.alien_text_baseline, d.alien_text_width) = text_layout(ScreenWidth);
        run(|s| s.initialize());
        d
    }
}
pub struct RebelDialogue;
impl super::RaceDialogue for RebelDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_rebel_yehat_comm()) }
        }
        #[cfg(test)]
        {
            as_comm_data(&descriptor(320))
        }
    }
    fn intro(&self) {
        #[cfg(not(test))]
        live::intro()
    }
    fn post_encounter(&self) {
        #[cfg(not(test))]
        live::post()
    }
    fn uninit(&self) -> u32 {
        0
    }
}
#[cfg(test)]
mod tests;
