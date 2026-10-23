//! Ilwrath dialogue, ported from the original C script.

use super::encounter::EncounterApi;
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    CombatIsInevitable,
    IlwrathGods,
    IlwrathInfo,
    IlwrathHome,
    GodsOrder,
    GodsSpeak,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: EncounterApi<Handler>> Dialogue<'_, A> {
    fn handle(&mut self, handler: Handler, r: u16) {
        match handler {
            Handler::CombatIsInevitable => self.combat_is_inevitable(r),
            Handler::IlwrathGods => self.ilwrath_gods(r),
            Handler::IlwrathInfo => self.ilwrath_info(r),
            Handler::IlwrathHome => self.ilwrath_home(r),
            Handler::GodsOrder => self.gods_order(r),
            Handler::GodsSpeak => self.gods_speak(r),
        }
    }
    fn initialize(&mut self, _desc: &mut CLocData) {
        let peaceful = self.api.get("PROBE_ILWRATH_ENCOUNTER") != 0
            || self.api.get("GLOBAL_FLAGS_AND_DATA") & ((1 << 4) | (1 << 5)) != 0
            || self.api.activity() as u8 == 5;
        self.api.segue(u32::from(!peaceful));
    }
    fn combat_is_inevitable(&mut self, r: u16) {
        self.api.segue(1);
        if r == YOU_ARE_WEAK {
            self.api.npc(i32::from(STRENGTH_NOT_ALL));
        } else if r == SLAY_BY_THOUSANDS {
            self.api.npc(i32::from(NO_SLAY_BY_THOUSANDS));
        } else if r == EASE_UP {
            self.api.npc(i32::from(NO_EASE_UP));
        } else if r == BYE_SPACE {
            self.api.npc(i32::from(GOODBYE_AND_DIE_SPACE));
        } else if r == BYE_HOMEWORLD {
            self.api.npc(i32::from(GOODBYE_AND_DIE_HOMEWORLD));
        } else if r == WANT_PEACE {
            self.api.npc(i32::from(NO_PEACE));
        } else if r == WANT_ALLIANCE {
            self.api.npc(i32::from(NO_ALLIANCE));
        } else if r == BUT_EVIL_IS_DEFINED {
            self.api.npc(i32::from(DONT_CONFUSE_US));
        } else if r == BYE_GODS {
            self.api.npc(i32::from(GOODBYE_GODS));
            self.api.segue(0);
        }
        if r == WHATS_UP {
            self.api.npc(i32::from(GENERAL_INFO));
            self.api.response(BYE, Handler::CombatIsInevitable, &[]);
        } else if ((((r == WHATS_UP_SPACE_1) || (r == WHATS_UP_SPACE_2))
            || (r == WHATS_UP_SPACE_3))
            || (r == WHATS_UP_SPACE_4))
            || (r == WHATS_UP_SPACE_5)
        {
            let mut num_visits: u8;
            num_visits = self.api.get("ILWRATH_INFO");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_2));
                }
                2 => {
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_3));
                }
                3 => {
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_4));
                }
                4 => {
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_5));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ILWRATH_INFO", num_visits);
        } else {
            if r == BYE {
                self.api.npc(i32::from(GOODBYE_AND_DIE));
            } else if r == WHERE_YOU_COME_FROM {
                self.api.npc(i32::from(CAME_FROM));
            }
            if r == IT_WILL_BE_A_PLEASURE {
                self.api.npc(i32::from(WHO_BLASTS_WHO));
            }
            if r == SURRENDER {
                self.api.npc(i32::from(NO_SURRENDER));
            }
            if r == BE_REASONABLE {
                self.api.npc(i32::from(NOT_REASONABLE));
            }
        }
    }
    fn ilwrath_gods(&mut self, r: u16) {
        let mut gods_left: bool;
        gods_left = false;
        if r == WANT_INFO_ON_GODS {
            self.api.npc(i32::from(SO_MUCH_TO_KNOW));
        } else if r == WHEN_START_WORSHIP {
            self.api.npc(i32::from(LONG_AGO));
            self.api.disable(WHEN_START_WORSHIP);
        } else if r == ANY_GOOD_GODS {
            self.api.npc(i32::from(KILLED_GOOD_GODS));
            self.api.disable(ANY_GOOD_GODS);
        } else if r == HOW_TALK_WITH_GODS {
            self.api.npc(i32::from(CHANNEL_44));
            self.api.disable(HOW_TALK_WITH_GODS);
        } else if r == WHY_44 {
            self.api.npc(i32::from(BECAUSE_44));
            self.api.disable(WHY_44);
        }
        if self.api.enabled(WHEN_START_WORSHIP) {
            self.api
                .response(WHEN_START_WORSHIP, Handler::IlwrathGods, &[]);
            gods_left = true;
        }
        if self.api.enabled(ANY_GOOD_GODS) {
            self.api.response(ANY_GOOD_GODS, Handler::IlwrathGods, &[]);
            gods_left = true;
        }
        if self.api.enabled(HOW_TALK_WITH_GODS) {
            self.api
                .response(HOW_TALK_WITH_GODS, Handler::IlwrathGods, &[]);
            gods_left = true;
        } else if self.api.enabled(WHY_44) {
            self.api.response(WHY_44, Handler::IlwrathGods, &[]);
            gods_left = true;
        }
        self.api.response(ENOUGH_GODS, Handler::IlwrathHome, &[]);
        if !(gods_left) {
            self.api.disable(WANT_INFO_ON_GODS);
        }
    }
    fn ilwrath_info(&mut self, r: u16) {
        if r == WANT_INFO_ON_ILWRATH {
            self.api.npc(i32::from(WHAT_ABOUT_ILWRATH));
        } else if r == WHAT_ABOUT_PHYSIO {
            self.api.npc(i32::from(ABOUT_PHYSIO));
            self.api.disable(WHAT_ABOUT_PHYSIO);
        } else if r == WHAT_ABOUT_HISTORY {
            self.api.npc(i32::from(ABOUT_HISTORY));
            self.api.disable(WHAT_ABOUT_HISTORY);
        } else if r == WHAT_ABOUT_CULTURE {
            self.api.npc(i32::from(ABOUT_CULTURE));
            self.api.disable(WHAT_ABOUT_CULTURE);
        } else if r == WHAT_ABOUT_URQUAN {
            self.api.npc(i32::from(URQUAN_TOO_NICE));
            self.api.disable(WHAT_ABOUT_URQUAN);
        } else if r == ARE_YOU_EVIL {
            self.api.npc(i32::from(OF_COURSE_WERE_EVIL));
            self.api.disable(ARE_YOU_EVIL);
        }
        if self.api.enabled(WHAT_ABOUT_PHYSIO) {
            self.api
                .response(WHAT_ABOUT_PHYSIO, Handler::IlwrathInfo, &[]);
        }
        if self.api.enabled(WHAT_ABOUT_HISTORY) {
            self.api
                .response(WHAT_ABOUT_HISTORY, Handler::IlwrathInfo, &[]);
        }
        if self.api.enabled(WHAT_ABOUT_CULTURE) {
            self.api
                .response(WHAT_ABOUT_CULTURE, Handler::IlwrathInfo, &[]);
        }
        if self.api.enabled(WHAT_ABOUT_URQUAN) {
            self.api
                .response(WHAT_ABOUT_URQUAN, Handler::IlwrathInfo, &[]);
        }
        if self.api.enabled(ARE_YOU_EVIL) {
            self.api.response(ARE_YOU_EVIL, Handler::IlwrathInfo, &[]);
        } else {
            self.api
                .response(BUT_EVIL_IS_DEFINED, Handler::CombatIsInevitable, &[]);
        }
        self.api.response(ENOUGH_ILWRATH, Handler::IlwrathHome, &[]);
    }
    fn ilwrath_home(&mut self, r: u16) {
        if r == 0 {
            let mut num_visits: u8;
            num_visits = self.api.get("ILWRATH_HOME_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_HOME_HELLO));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_HOME_HELLO));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ILWRATH_HOME_VISITS", num_visits);
        } else if r == ENOUGH_GODS {
            self.api.npc(i32::from(OK_ENOUGH_GODS));
        } else if r == ENOUGH_ILWRATH {
            self.api.npc(i32::from(OK_ENOUGH_ILWRATH));
        }
        if self.api.enabled(WANT_INFO_ON_GODS) {
            self.api
                .response(WANT_INFO_ON_GODS, Handler::IlwrathGods, &[]);
        }
        if self.api.enabled(WANT_INFO_ON_ILWRATH) {
            self.api
                .response(WANT_INFO_ON_ILWRATH, Handler::IlwrathInfo, &[]);
        }
        self.api
            .response(WANT_PEACE, Handler::CombatIsInevitable, &[]);
        self.api
            .response(WANT_ALLIANCE, Handler::CombatIsInevitable, &[]);
        self.api
            .response(BYE_HOMEWORLD, Handler::CombatIsInevitable, &[]);
    }
    fn gods_order(&mut self, r: u16) {
        let mut orders_left: bool;
        orders_left = false;
        if r == OTHER_DIVINE_ORDERS {
            self.api.npc(i32::from(WHAT_ORDERS));
        } else if r == SAY_WARSHIP {
            self.api.npc(i32::from(OK_WARSHIP));
            self.api.disable(SAY_WARSHIP);
        } else if r == SAY_DWE {
            self.api.npc(i32::from(OK_DWE));
            self.api.disable(SAY_DWE);
        } else if r == SAY_YOUBOO {
            self.api.npc(i32::from(OK_YOUBOO));
            self.api.disable(SAY_YOUBOO);
        } else if r == SAY_DILLRAT {
            self.api.npc(i32::from(OK_DILRAT));
            self.api.disable(SAY_DILLRAT);
        }
        if self.api.enabled(SAY_WARSHIP) {
            self.api.response(SAY_WARSHIP, Handler::GodsOrder, &[]);
            orders_left = true;
        }
        if self.api.enabled(SAY_DWE) {
            self.api.response(SAY_DWE, Handler::GodsOrder, &[]);
            orders_left = true;
        }
        if self.api.enabled(SAY_YOUBOO) {
            self.api.response(SAY_YOUBOO, Handler::GodsOrder, &[]);
            orders_left = true;
        }
        if self.api.enabled(SAY_DILLRAT) {
            self.api.response(SAY_DILLRAT, Handler::GodsOrder, &[]);
            orders_left = true;
        }
        self.api.response(ENOUGH_ORDERS, Handler::GodsSpeak, &[]);
        if !(orders_left) {
            self.api.disable(OTHER_DIVINE_ORDERS);
        }
    }
    fn gods_speak(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == 0 {
            if self.api.get("ILWRATH_FIGHT_THRADDASH") != 0 {
                self.api.npc(i32::from(GLORIOUS_WORSHIP));
            } else if self.api.get("ILWRATH_DECEIVED") != 0 {
                self.api.npc(i32::from(ON_WAY));
            } else {
                num_visits = self.api.get("ILWRATH_GODS_SPOKEN");
                let res = {
                    let old = num_visits;
                    num_visits += 1;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(ILWRATH_BELIEVE));
                    }
                    1 => {
                        self.api.npc(i32::from(GODS_RETURN_1));
                    }
                    2 => {
                        self.api.npc(i32::from(GODS_RETURN_2));
                    }
                    3 => {
                        self.api.npc(i32::from(GODS_RETURN_3));
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set("ILWRATH_GODS_SPOKEN", num_visits);
            }
        } else if r == GO_KILL_THRADDASH {
            self.api.npc(i32::from(OK_KILL_THRADDASH));
            self.api.set("ILWRATH_DECEIVED", 1_u8);
            self.api.advance_ilwrath();
        } else if r == WORSHIP_US {
            num_visits = self.api.get("ILWRATH_WORSHIP");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(WE_WORSHIP_1));
                }
                1 => {
                    self.api.npc(i32::from(WE_WORSHIP_2));
                }
                2 => {
                    self.api.npc(i32::from(WE_WORSHIP_3));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ILWRATH_WORSHIP", num_visits);
            self.api.disable(WORSHIP_US);
        } else if r == ENOUGH_ORDERS {
            self.api.npc(i32::from(NEVER_ENOUGH));
        }
        if !(self.api.get("ILWRATH_DECEIVED") != 0) {
            self.api
                .response(GO_KILL_THRADDASH, Handler::GodsSpeak, &[]);
        }
        if self.api.enabled(WORSHIP_US) {
            self.api.response(WORSHIP_US, Handler::GodsSpeak, &[]);
        }
        if self.api.enabled(OTHER_DIVINE_ORDERS) {
            self.api
                .response(OTHER_DIVINE_ORDERS, Handler::GodsOrder, &[]);
        }
        self.api
            .response(BYE_GODS, Handler::CombatIsInevitable, &[]);
    }
    fn ilwrath_space(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == 0 {
            num_visits = self.api.get("ILWRATH_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_HELLO_SPACE));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_HELLO_SPACE_1));
                }
                2 => {
                    self.api.npc(i32::from(SUBSEQUENT_HELLO_SPACE_2));
                }
                3 => {
                    self.api.npc(i32::from(SUBSEQUENT_HELLO_SPACE_3));
                }
                4 => {
                    self.api.npc(i32::from(SUBSEQUENT_HELLO_SPACE_4));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ILWRATH_VISITS", num_visits);
        }
        num_visits = self.api.get("ILWRATH_INFO");
        match num_visits {
            0 => {
                self.api
                    .response(WHATS_UP_SPACE_1, Handler::CombatIsInevitable, &[]);
            }
            1 => {
                self.api
                    .response(WHATS_UP_SPACE_2, Handler::CombatIsInevitable, &[]);
            }
            2 => {
                self.api
                    .response(WHATS_UP_SPACE_3, Handler::CombatIsInevitable, &[]);
            }
            3 => {
                self.api
                    .response(WHATS_UP_SPACE_4, Handler::CombatIsInevitable, &[]);
            }
            4 => {
                self.api
                    .response(WHATS_UP_SPACE_5, Handler::CombatIsInevitable, &[]);
            }
            _ => {}
        }
        self.api
            .response(YOU_ARE_WEAK, Handler::CombatIsInevitable, &[]);
        self.api
            .response(SLAY_BY_THOUSANDS, Handler::CombatIsInevitable, &[]);
        self.api.response(EASE_UP, Handler::CombatIsInevitable, &[]);
        self.api
            .response(BYE_SPACE, Handler::CombatIsInevitable, &[]);
    }
    fn intro(&mut self) {
        let mut num_visits: u8;
        if self.api.get("PROBE_ILWRATH_ENCOUNTER") != 0 {
            self.api.npc(i32::from(SEND_MESSAGE));
            self.api
                .response(WHERE_YOU_COME_FROM, Handler::CombatIsInevitable, &[]);
            self.api
                .response(IT_WILL_BE_A_PLEASURE, Handler::CombatIsInevitable, &[]);
            self.api
                .response(SURRENDER, Handler::CombatIsInevitable, &[]);
            self.api
                .response(BE_REASONABLE, Handler::CombatIsInevitable, &[]);
        } else if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
            self.ilwrath_home(0_u16);
        } else if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0 {
            num_visits = self.api.get("ILWRATH_CHMMR_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_CHMMR_HELLO));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_CHMMR_HELLO));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ILWRATH_CHMMR_VISITS", num_visits);
            self.api
                .response(WHATS_UP, Handler::CombatIsInevitable, &[]);
            self.api.response(BYE, Handler::CombatIsInevitable, &[]);
        } else if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 5)) != 0 {
            if self.api.get("ILWRATH_FIGHT_THRADDASH") != 0 {
                self.api.npc(i32::from(BIG_FUN));
            } else if self.api.get("ILWRATH_DECEIVED") != 0 {
                self.api.npc(i32::from(FAST_AS_CAN));
            } else {
                self.api.npc(i32::from(JUST_GRUNTS));
            }
            self.api.segue(0);
        } else if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 4)) != 0 {
            self.gods_speak(0_u16);
        } else {
            self.api.segue(0);
            if self.api.get("ILWRATH_FIGHT_THRADDASH") != 0 {
                self.api.npc(i32::from(HAPPY_FIGHTING_THRADDASH));
            } else if self.api.get("ILWRATH_DECEIVED") != 0 {
                self.api.npc(i32::from(ON_WAY_TO_THRADDASH));
            } else {
                self.ilwrath_space(0_u16);
            }
        }
    }
    fn post_ilwrath_enc(&mut self) {}
}
fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.ilwrath.graphics".as_ptr(),
        alien_font_res: c"comm.ilwrath.font".as_ptr(),
        alien_colormap_res: c"comm.ilwrath.colortable".as_ptr(),
        alien_song_res: c"comm.ilwrath.music".as_ptr(),
        conversation_phrases_res: c"comm.ilwrath.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 164, y: 70 },
        alien_text_width: 288,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 4,
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] = animation(6, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[1] = animation(11, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[2] = animation(16, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[3] = animation(21, 6, 2, (42, 0), (840, 2520));
    desc.alien_talk_desc = animation(1, 5, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}
#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    fn combat_is_inevitable(r: u32) {
        run(|s| {
            s.handle(
                Handler::CombatIsInevitable,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn ilwrath_gods(r: u32) {
        run(|s| {
            s.handle(
                Handler::IlwrathGods,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn ilwrath_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::IlwrathInfo,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn ilwrath_home(r: u32) {
        run(|s| {
            s.handle(
                Handler::IlwrathHome,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn gods_order(r: u32) {
        run(|s| {
            s.handle(
                Handler::GodsOrder,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn gods_speak(r: u32) {
        run(|s| {
            s.handle(
                Handler::GodsSpeak,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
        match handler {
            Handler::CombatIsInevitable => combat_is_inevitable,
            Handler::IlwrathGods => ilwrath_gods,
            Handler::IlwrathInfo => ilwrath_info,
            Handler::IlwrathHome => ilwrath_home,
            Handler::GodsOrder => gods_order,
            Handler::GodsSpeak => gods_speak,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_ilwrath_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter descriptor and initial battle segue.
    ///
    /// # Safety
    /// Game thread only. The caller must copy the descriptor before another encounter.
    #[no_mangle]
    pub unsafe extern "C" fn init_ilwrath_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct IlwrathDialogue;
impl super::RaceDialogue for IlwrathDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_ilwrath_comm()) }
        }
        #[cfg(test)]
        {
            as_comm_data(&descriptor())
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
