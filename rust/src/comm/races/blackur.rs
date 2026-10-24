//! Kohr-Ah dialogue, ported from blackurq.c.
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
mod api;
use api::BlackurApi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Combat,
    Story,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: BlackurApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::Combat => self.combat(r),
            Handler::Story => self.story(r),
        }
    }
    fn sequence(&mut self, key: &str, phrases: &[i32]) -> u8 {
        let n = self.api.get(key);
        if let Some(&p) = phrases.get(usize::from(n)) {
            self.api.npc(p);
        }
        if usize::from(n) + 1 == phrases.len() {
            n
        } else {
            n.wrapping_add(1)
        }
    }
    fn advance_sequence(&mut self, key: &str, phrases: &[i32]) {
        let next = self.sequence(key, phrases);
        self.api.set(key, next);
    }
    fn combat(&mut self, r: i32) {
        self.api.segue(1);
        match r {
            x if x == BYE => {
                self.api.npc(if self.api.get("KOHR_AH_BYES") == 0 {
                    GOODBYE_AND_DIE
                } else {
                    DIE_HUMAN
                });
                self.api.set("KOHR_AH_BYES", 1);
            }
            x if x == GUESS_THATS_ALL => self.api.npc(THEN_DIE),
            x if x == WHAT_ARE_YOU_HOVERING_OVER => {
                self.api.npc(BONE_PILE);
                self.api.set("KOHR_AH_INFO", 1);
            }
            x if x == YOU_SURE_ARE_CREEPY => {
                self.api.npc(YES_CREEPY);
                self.api.set("KOHR_AH_INFO", 2);
            }
            x if x == STOP_THAT_GROSS_BLINKING => {
                self.api.npc(DIE_HUMAN);
                self.api.set("KOHR_AH_INFO", 3);
            }
            x if [THREAT_1, THREAT_2, THREAT_3, THREAT_4].contains(&x) => self.advance_sequence(
                "KOHR_AH_REASONS",
                &[
                    RESISTANCE_IS_USELESS_1,
                    RESISTANCE_IS_USELESS_2,
                    RESISTANCE_IS_USELESS_3,
                    RESISTANCE_IS_USELESS_4,
                ],
            ),
            x if [PLEAD_1, PLEAD_2, PLEAD_3, PLEAD_4].contains(&x) => {
                let n = self.api.get("KOHR_AH_PLEAD");
                match n {
                    0 => self.api.npc(PLEADING_IS_USELESS_1),
                    1 => self.api.npc(PLEADING_IS_USELESS_2),
                    2 | 3 => self.api.npc(PLEADING_IS_USELESS_4),
                    _ => {}
                }
                self.api.set(
                    "KOHR_AH_PLEAD",
                    if matches!(n, 2 | 3) {
                        n
                    } else {
                        n.wrapping_add(1)
                    },
                );
            }
            x if [
                WHY_KILL_ALL_1,
                WHY_KILL_ALL_2,
                WHY_KILL_ALL_3,
                WHY_KILL_ALL_4,
            ]
            .contains(&x) =>
            {
                self.advance_sequence(
                    "KOHR_AH_REASONS",
                    &[
                        KILL_BECAUSE_1,
                        KILL_BECAUSE_2,
                        KILL_BECAUSE_3,
                        KILL_BECAUSE_4,
                    ],
                )
            }
            x if [
                PLEASE_DONT_KILL_1,
                PLEASE_DONT_KILL_2,
                PLEASE_DONT_KILL_3,
                PLEASE_DONT_KILL_4,
            ]
            .contains(&x) =>
            {
                self.advance_sequence(
                    "KOHR_AH_PLEAD",
                    &[WILL_KILL_1, WILL_KILL_2, WILL_KILL_3, WILL_KILL_4],
                )
            }
            x if [BYE_FRENZY_1, BYE_FRENZY_2, BYE_FRENZY_3, BYE_FRENZY_4].contains(&x) => self
                .advance_sequence(
                    "KOHR_AH_INFO",
                    &[
                        GOODBYE_AND_DIE_FRENZY_1,
                        GOODBYE_AND_DIE_FRENZY_2,
                        GOODBYE_AND_DIE_FRENZY_3,
                        GOODBYE_AND_DIE_FRENZY_4,
                    ],
                ),
            _ => {}
        }
    }
    fn frenzy(&mut self, _: i32) {
        let n = self.api.get("KOHR_AH_REASONS") as usize;
        if n < 4 {
            self.api
                .response((WHY_KILL_ALL_1 + n as i32) as u16, Handler::Combat, &[]);
        }
        let n = self.api.get("KOHR_AH_PLEAD") as usize;
        if n < 4 {
            self.api.response(
                (PLEASE_DONT_KILL_1 + n as i32 * 2) as u16,
                Handler::Combat,
                &[],
            );
        }
        let n = self.api.get("KOHR_AH_INFO") as usize;
        if n < 4 {
            self.api
                .response((BYE_FRENZY_1 + n as i32) as u16, Handler::Combat, &[]);
        }
    }
    fn story(&mut self, r: i32) {
        match r {
            x if x == KEY_PHRASE => {
                self.api.npc(RESPONSE_TO_KEY_PHRASE);
                self.api.set("KNOW_KOHR_AH_STORY", 2);
            }
            x if x == WHY_DO_YOU_DESTROY => {
                self.api.npc(WE_WERE_SLAVES);
                self.api.disable(r as u16);
            }
            x if x == RELATIONSHIP_WITH_URQUAN => {
                self.api.npc(WE_ARE_URQUAN_TOO);
                self.api.disable(r as u16);
            }
            x if x == WHAT_ABOUT_CULTURE => {
                self.api.npc(BONE_GARDENS);
                self.api.disable(r as u16);
            }
            x if x == HOW_LEAVE_ME_ALONE => {
                self.api.npc(YOU_DIE);
                self.api.disable(r as u16);
            }
            _ => {}
        }
        for p in [
            WHY_DO_YOU_DESTROY,
            RELATIONSHIP_WITH_URQUAN,
            WHAT_ABOUT_CULTURE,
            HOW_LEAVE_ME_ALONE,
        ] {
            if self.api.enabled(p as u16) {
                self.api.response(p as u16, Handler::Story, &[]);
            }
        }
        self.api
            .response(GUESS_THATS_ALL as u16, Handler::Combat, &[]);
    }
    fn die_human(&mut self, _: i32) {
        let n = self.api.get("KOHR_AH_REASONS") as usize;
        if n < 4 {
            self.api
                .response((THREAT_1 + n as i32 * 2) as u16, Handler::Combat, &[]);
        }
        if self.api.get("KNOW_KOHR_AH_STORY") == 1 {
            self.api.response(KEY_PHRASE as u16, Handler::Story, &[]);
        }
        match self.api.get("KOHR_AH_INFO") {
            0 => self
                .api
                .response(WHAT_ARE_YOU_HOVERING_OVER as u16, Handler::Combat, &[]),
            1 => self
                .api
                .response(YOU_SURE_ARE_CREEPY as u16, Handler::Combat, &[]),
            2 => self
                .api
                .response(STOP_THAT_GROSS_BLINKING as u16, Handler::Combat, &[]),
            _ => {}
        }
        match self.api.get("KOHR_AH_PLEAD") {
            0 => self.api.response(PLEAD_1 as u16, Handler::Combat, &[]),
            1 => self.api.response(PLEAD_2 as u16, Handler::Combat, &[]),
            2 | 3 => self.api.response(PLEAD_4 as u16, Handler::Combat, &[]),
            _ => {}
        }
        self.api.response(BYE as u16, Handler::Combat, &[]);
    }
    fn intro(&mut self) {
        let activity = self.api.activity() as u8;
        if activity == crate::comm::dispatch::WON_LAST_BATTLE as u8 {
            self.api.npc(OUT_TAKES);
            self.api.segue(0);
            return;
        }
        if self.api.get("KOHR_AH_KILLED_ALL") != 0 {
            self.api.npc(GAME_OVER_DUDE);
            self.api.segue(0);
            return;
        }
        if self.api.get("KOHR_AH_SENSES_EVIL") == 0 && self.api.get("TALKING_PET_ON_SHIP") != 0 {
            self.api.npc(SENSE_EVIL);
            self.api.set("KOHR_AH_SENSES_EVIL", 1);
        }
        if activity == crate::comm::dispatch::IN_INTERPLANETARY as u8
            && self.api.in_samatra_battle()
        {
            self.api.npc(HELLO_SAMATRA);
            self.api.set("AWARE_OF_SAMATRA", 1);
            self.api.segue(1);
            return;
        }
        if self.api.get("KOHR_AH_FRENZY") != 0 {
            let next = self.sequence(
                "KOHR_AH_VISITS",
                &[WE_KILL_ALL_1, WE_KILL_ALL_2, WE_KILL_ALL_3, WE_KILL_ALL_4],
            );
            self.frenzy(0);
            self.api.set("KOHR_AH_VISITS", next);
        } else {
            let next = self.sequence(
                "KOHR_AH_VISITS",
                &[
                    HELLO_AND_DIE_1,
                    HELLO_AND_DIE_2,
                    HELLO_AND_DIE_3,
                    HELLO_AND_DIE_4,
                ],
            );
            self.die_human(0);
            self.api.set("KOHR_AH_VISITS", next);
        }
    }
    fn initialize(&mut self) {
        self.api.segue(
            if self.api.get("KOHR_AH_KILLED_ALL") == 0
                && self.api.activity() as u8 != crate::comm::dispatch::WON_LAST_BATTLE as u8
            {
                1
            } else {
                0
            },
        );
    }
}
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
        alien_frame_res: c"comm.kohrah.graphics".as_ptr(),
        alien_font_res: c"comm.kohrah.font".as_ptr(),
        alien_colormap_res: c"comm.kohrah.colortable".as_ptr(),
        alien_song_res: c"comm.kohrah.music".as_ptr(),
        conversation_phrases_res: c"comm.kohrah.dialogue".as_ptr(),
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
        num_animations: 8,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(7, 6, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[1] = animation(13, 7, 2, (56, 0), (840, 2520));
    for (i, s) in [20, 23, 26].into_iter().enumerate() {
        d.alien_ambient_array[i + 2] = animation(s, 3, 4, (56, 0), (840, 2520));
    }
    d.alien_ambient_array[5] = animation(29, 4, 1, (84, 0), (84, 0));
    d.alien_ambient_array[6] = animation(33, 5, 0x0a, (84, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(38, 4, 2, (84, 0), (840, 2520));
    d.alien_transition_desc = animation(1, 2, 0, (140, 0), (0, 0));
    d.alien_talk_desc = animation(2, 5, 0, (56, 0), (70, 0));
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
        f(&mut Dialogue { api: &mut a });
    }
    fn combat(r: u32) {
        run(|s| s.handle(Handler::Combat, r as i32));
    }
    fn story(r: u32) {
        run(|s| s.handle(Handler::Story, r as i32));
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::Combat => combat,
            Handler::Story => story,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {}
    pub(super) fn uninit() -> u16 {
        0
    }
    #[no_mangle]
    pub unsafe extern "C" fn init_blackurq_comm() -> *mut CLocData {
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
pub struct BlackurDialogue;
impl super::RaceDialogue for BlackurDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_blackurq_comm()) }
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
        0
    }
}
#[cfg(test)]
mod tests;
