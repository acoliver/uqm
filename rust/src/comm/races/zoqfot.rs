//! Zoqfot dialogue, ported from the original C script.
use super::alliance::AllianceApi;
use super::script::{animation, as_comm_data, GLOBAL_SHIP_NAME};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
mod api;
use super::alliance::ZOQFOTPIK_SHIP;
use api::{Speaker, ZoqApi};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    ZoqFotIntro,
    FormAlliance,
    AquaintZoqFot,
    ZoqFotInfo,
    ZoqFotHome,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: ZoqApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::ZoqFotIntro => self.zoq_fot_intro(r),
            Handler::FormAlliance => self.form_alliance(r),
            Handler::AquaintZoqFot => self.aquaint_zoq_fot(r),
            Handler::ZoqFotInfo => self.zoq_fot_info(r),
            Handler::ZoqFotHome => self.zoq_fot_home(r),
        }
    }
    fn initialize(&mut self, _desc: &mut CLocData) {
        self.api.segue(u32::from(
            self.api.alliance(ZOQFOTPIK_SHIP) != 1 && self.api.activity() as u8 != 5,
        ));
    }
    fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0);
        if r == BYE_HOMEWORLD {
            self.api.speak(GOODBYE_HOME0, Speaker::Zoq);
            self.api.speak(GOODBYE_HOME1, Speaker::Pik);
            self.api.begin_talk(!0u16);
        } else if r == DECIDE_LATER {
            self.api.speak(PLEASE_HURRY0, Speaker::Zoq);
            self.api.speak(PLEASE_HURRY1, Speaker::Pik);
            self.api.begin_talk(!0u16);
        } else if r == VALUABLE_INFO {
            self.api.speak(GOODBYE0, Speaker::Zoq);
            self.api.speak(GOODBYE1, Speaker::Pik);
            self.api.speak(GOODBYE2, Speaker::Zoq);
            self.api.speak(GOODBYE3, Speaker::Pik);
            self.api.begin_talk(!0u16);
        } else if r == HOW_CAN_I_HELP {
            self.api.speak(EMMISSARIES0, Speaker::Zoq);
            self.api.speak(EMMISSARIES1, Speaker::Pik);
            self.api.speak(EMMISSARIES2, Speaker::Zoq);
            self.api.speak(EMMISSARIES3, Speaker::Pik);
            self.api.speak(EMMISSARIES4, Speaker::Zoq);
            self.api.speak(EMMISSARIES5, Speaker::Pik);
            self.api.speak(EMMISSARIES6, Speaker::Zoq);
            self.api.speak(EMMISSARIES7, Speaker::Pik);
            self.api.begin_talk(!0u16);
        } else if r == SURE {
            self.api.speak(WE_ALLY0, Speaker::Zoq);
            self.api.speak(WE_ALLY1, Speaker::Pik);
            self.api.speak(WE_ALLY2, Speaker::Zoq);
            self.api.speak(WE_ALLY3, Speaker::Pik);
            self.api.speak(WE_ALLY4, Speaker::Zoq);
            self.api.speak(WE_ALLY5, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.set_allied(ZOQFOTPIK_SHIP, 1 != 0);
            self.api.event(1, 3, 0, 0, 6);
            self.api.set("ZOQFOT_HOME_VISITS", 0_u8);
        } else if r == ALL_VERY_INTERESTING {
            self.api.speak(SEE_TOLD_YOU0, Speaker::Zoq);
            self.api.speak(SEE_TOLD_YOU1, Speaker::Pik);
            self.api.speak(SEE_TOLD_YOU2, Speaker::Zoq);
            self.api.speak(SEE_TOLD_YOU3, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.set("ZOQFOT_HOSTILE", 1_u8);
            self.api.set("ZOQFOT_HOME_VISITS", 0_u8);
            self.api.segue(1);
        } else if r == NEVER {
            self.api.speak(WE_ENEMIES0, Speaker::Zoq);
            self.api.speak(WE_ENEMIES1, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.set("ZOQFOT_HOME_VISITS", 0_u8);
            self.api.set("ZOQFOT_HOSTILE", 1_u8);
            self.api.segue(1);
        }
    }
    fn form_alliance(&mut self, r: u16) {
        let _ = r;
        self.api.speak(ALLY_WITH_US0, Speaker::Zoq);
        self.api.speak(ALLY_WITH_US1, Speaker::Pik);
        self.api.speak(ALLY_WITH_US2, Speaker::Zoq);
        self.api.speak(ALLY_WITH_US3, Speaker::Pik);
        self.api.speak(ALLY_WITH_US4, Speaker::Zoq);
        self.api.speak(ALLY_WITH_US5, Speaker::Pik);
        self.api.begin_talk(!0u16);
        self.api.response(SURE, Handler::ExitConversation, &[]);
        self.api.response(NEVER, Handler::ExitConversation, &[]);
        self.api
            .response(DECIDE_LATER, Handler::ExitConversation, &[]);
    }
    fn zoq_fot_intro(&mut self, r: u16) {
        if r == WE_ARE_VINDICATOR0 {
            self.api.speak(WE_GLAD0, Speaker::Zoq);
            self.api.speak(WE_GLAD1, Speaker::Pik);
            self.api.speak(WE_GLAD2, Speaker::Zoq);
            self.api.speak(WE_GLAD3, Speaker::Pik);
            self.api.speak(WE_GLAD4, Speaker::Zoq);
            self.api.speak(WE_GLAD5, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WE_ARE_VINDICATOR0);
        } else if r == YOUR_RACE {
            self.api.speak(YEARS_AGO0, Speaker::Zoq);
            self.api.speak(YEARS_AGO1, Speaker::Pik);
            self.api.speak(YEARS_AGO2, Speaker::Zoq);
            self.api.speak(YEARS_AGO3, Speaker::Pik);
            self.api.speak(YEARS_AGO4, Speaker::Zoq);
            self.api.speak(YEARS_AGO5, Speaker::Pik);
            self.api.speak(YEARS_AGO6, Speaker::Zoq);
            self.api.speak(YEARS_AGO7, Speaker::Pik);
            self.api.speak(YEARS_AGO8, Speaker::Zoq);
            self.api.speak(YEARS_AGO9, Speaker::Pik);
            self.api.speak(YEARS_AGO10, Speaker::Zoq);
            self.api.speak(YEARS_AGO11, Speaker::Pik);
            self.api.speak(YEARS_AGO12, Speaker::Zoq);
            self.api.speak(YEARS_AGO13, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(YOUR_RACE);
        } else if r == WHERE_FROM {
            self.api.speak(TRAVELED_FAR0, Speaker::Zoq);
            self.api.speak(TRAVELED_FAR1, Speaker::Pik);
            self.api.speak(TRAVELED_FAR2, Speaker::Zoq);
            self.api.speak(TRAVELED_FAR3, Speaker::Pik);
            self.api.speak(TRAVELED_FAR4, Speaker::Zoq);
            self.api.speak(TRAVELED_FAR5, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHERE_FROM);
        } else if r == WHAT_EMERGENCY {
            self.api.speak(UNDER_ATTACK0, Speaker::Zoq);
            self.api.speak(UNDER_ATTACK1, Speaker::Pik);
            self.api.speak(UNDER_ATTACK2, Speaker::Zoq);
            self.api.speak(UNDER_ATTACK3, Speaker::Pik);
            self.api.speak(UNDER_ATTACK4, Speaker::Zoq);
            self.api.speak(UNDER_ATTACK5, Speaker::Pik);
            self.api.speak(UNDER_ATTACK6, Speaker::Zoq);
            self.api.speak(UNDER_ATTACK7, Speaker::Pik);
            self.api.speak(UNDER_ATTACK8, Speaker::Zoq);
            self.api.speak(UNDER_ATTACK9, Speaker::Pik);
            self.api.speak(UNDER_ATTACK10, Speaker::Zoq);
            self.api.speak(UNDER_ATTACK11, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_EMERGENCY);
        } else if r == TOUGH_LUCK {
            self.api.speak(NOT_HELPFUL0, Speaker::Zoq);
            self.api.speak(NOT_HELPFUL1, Speaker::Pik);
            self.api.speak(NOT_HELPFUL2, Speaker::Zoq);
            self.api.speak(NOT_HELPFUL3, Speaker::Pik);
            self.api.speak(NOT_HELPFUL4, Speaker::Zoq);
            self.api.speak(NOT_HELPFUL5, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(TOUGH_LUCK);
        } else if r == WHAT_LOOK_LIKE {
            self.api.speak(LOOK_LIKE0, Speaker::Zoq);
            self.api.speak(LOOK_LIKE1, Speaker::Pik);
            self.api.speak(LOOK_LIKE2, Speaker::Zoq);
            self.api.speak(LOOK_LIKE3, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_LOOK_LIKE);
        }
        if (self.api.enabled(YOUR_RACE) || self.api.enabled(WHERE_FROM))
            || self.api.enabled(WHAT_EMERGENCY)
        {
            if self.api.enabled(YOUR_RACE) {
                self.api.response(YOUR_RACE, Handler::ZoqFotIntro, &[]);
            }
            if self.api.enabled(WHERE_FROM) {
                self.api.response(WHERE_FROM, Handler::ZoqFotIntro, &[]);
            }
            if self.api.enabled(WHAT_EMERGENCY) {
                self.api.response(WHAT_EMERGENCY, Handler::ZoqFotIntro, &[]);
            }
        } else {
            if self.api.enabled(TOUGH_LUCK) {
                self.api.response(TOUGH_LUCK, Handler::ZoqFotIntro, &[]);
            }
            if self.api.enabled(WHAT_LOOK_LIKE) {
                self.api.response(WHAT_LOOK_LIKE, Handler::ZoqFotIntro, &[]);
            }
            self.api
                .response(ALL_VERY_INTERESTING, Handler::ExitConversation, &[]);
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                self.api
                    .response(HOW_CAN_I_HELP, Handler::FormAlliance, &[]);
            } else {
                self.api
                    .response(HOW_CAN_I_HELP, Handler::ExitConversation, &[]);
            }
            self.api
                .response(VALUABLE_INFO, Handler::ExitConversation, &[]);
        }
    }
    fn aquaint_zoq_fot(&mut self, r: u16) {
        let mut fragments: Vec<i32> = Vec::new();
        if r == WHICH_FOT {
            self.api.speak(HE_IS0, Speaker::Zoq);
            self.api.speak(HE_IS1, Speaker::Pik);
            self.api.speak(HE_IS2, Speaker::Zoq);
            self.api.speak(HE_IS3, Speaker::Pik);
            self.api.speak(HE_IS4, Speaker::Zoq);
            self.api.speak(HE_IS5, Speaker::Pik);
            self.api.speak(HE_IS6, Speaker::Zoq);
            self.api.speak(HE_IS7, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHICH_FOT);
        } else if r == QUIET_TOADIES {
            self.api.speak(TOLD_YOU0, Speaker::Zoq);
            self.api.speak(TOLD_YOU1, Speaker::Pik);
            self.api.speak(TOLD_YOU2, Speaker::Zoq);
            self.api.speak(TOLD_YOU3, Speaker::Pik);
            self.api.speak(TOLD_YOU4, Speaker::Zoq);
            self.api.speak(TOLD_YOU5, Speaker::Pik);
            self.api.speak(TOLD_YOU6, Speaker::Zoq);
            self.api.speak(TOLD_YOU7, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(QUIET_TOADIES);
        }
        if self.api.enabled(WE_ARE_VINDICATOR0) {
            fragments.clear();
            fragments.push(WE_ARE_VINDICATOR0 as i32);
            fragments.extend(self.api.alliance_fragments(NAME_1));
            fragments.push(WE_ARE_VINDICATOR1 as i32);
            fragments.push(GLOBAL_SHIP_NAME);
            fragments.push(WE_ARE_VINDICATOR2 as i32);
        }
        if self.api.enabled(WHICH_FOT) {
            self.api.response(WHICH_FOT, Handler::AquaintZoqFot, &[]);
        }
        if self.api.enabled(WE_ARE_VINDICATOR0) {
            self.api
                .response(WE_ARE_VINDICATOR0, Handler::ZoqFotIntro, &fragments);
        }
        if self.api.enabled(QUIET_TOADIES) {
            self.api
                .response(QUIET_TOADIES, Handler::AquaintZoqFot, &[]);
        }
        self.api
            .response(ALL_VERY_INTERESTING, Handler::ExitConversation, &[]);
        self.api
            .response(VALUABLE_INFO, Handler::ExitConversation, &[]);
    }
    fn zoq_fot_info(&mut self, r: u16) {
        let mut info_left: u8;
        if r == WANT_SPECIFIC_INFO {
            self.api.speak(WHAT_SPECIFIC_INFO0, Speaker::Zoq);
            self.api.speak(WHAT_SPECIFIC_INFO1, Speaker::Pik);
            self.api.begin_talk(!0u16);
        } else if r == WHAT_ABOUT_OTHERS {
            self.api.speak(ABOUT_OTHERS0, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS1, Speaker::Pik);
            self.api.speak(ABOUT_OTHERS2, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS3, Speaker::Pik);
            self.api.speak(ABOUT_OTHERS4, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS5, Speaker::Pik);
            self.api.speak(ABOUT_OTHERS6, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS7, Speaker::Pik);
            self.api.speak(ABOUT_OTHERS8, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS9, Speaker::Pik);
            self.api.speak(ABOUT_OTHERS10, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS11, Speaker::Pik);
            self.api.speak(ABOUT_OTHERS12, Speaker::Zoq);
            self.api.speak(ABOUT_OTHERS13, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_ABOUT_OTHERS);
        } else if r == WHAT_ABOUT_ZEBRANKY {
            self.api.speak(ABOUT_ZEBRANKY0, Speaker::Zoq);
            self.api.speak(ABOUT_ZEBRANKY1, Speaker::Pik);
            self.api.speak(ABOUT_ZEBRANKY2, Speaker::Zoq);
            self.api.speak(ABOUT_ZEBRANKY3, Speaker::Pik);
            self.api.speak(ABOUT_ZEBRANKY4, Speaker::Zoq);
            self.api.speak(ABOUT_ZEBRANKY5, Speaker::Pik);
            self.api.speak(ABOUT_ZEBRANKY6, Speaker::Zoq);
            self.api.speak(ABOUT_ZEBRANKY7, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_ABOUT_ZEBRANKY);
        } else if r == WHAT_ABOUT_STINGER {
            self.api.speak(ABOUT_STINGER0, Speaker::Zoq);
            self.api.speak(ABOUT_STINGER1, Speaker::Pik);
            self.api.speak(ABOUT_STINGER2, Speaker::Zoq);
            self.api.speak(ABOUT_STINGER3, Speaker::Pik);
            self.api.speak(ABOUT_STINGER4, Speaker::Zoq);
            self.api.speak(ABOUT_STINGER5, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_ABOUT_STINGER);
        } else if r == WHAT_ABOUT_GUY_IN_BACK {
            self.api.speak(ABOUT_GUY0, Speaker::Zoq);
            self.api.speak(ABOUT_GUY1, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_ABOUT_GUY_IN_BACK);
        } else if r == WHAT_ABOUT_PAST {
            self.api.speak(ABOUT_PAST0, Speaker::Zoq);
            self.api.speak(ABOUT_PAST1, Speaker::Pik);
            self.api.speak(ABOUT_PAST2, Speaker::Zoq);
            self.api.speak(ABOUT_PAST3, Speaker::Pik);
            self.api.speak(ABOUT_PAST4, Speaker::Zoq);
            self.api.speak(ABOUT_PAST5, Speaker::Pik);
            self.api.speak(ABOUT_PAST6, Speaker::Zoq);
            self.api.speak(ABOUT_PAST7, Speaker::Pik);
            self.api.speak(ABOUT_PAST8, Speaker::Zoq);
            self.api.speak(ABOUT_PAST9, Speaker::Pik);
            self.api.speak(ABOUT_PAST10, Speaker::Zoq);
            self.api.speak(ABOUT_PAST11, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.disable(WHAT_ABOUT_PAST);
        }
        info_left = 0;
        if self.api.enabled(WHAT_ABOUT_OTHERS) {
            self.api
                .response(WHAT_ABOUT_OTHERS, Handler::ZoqFotInfo, &[]);
            info_left = 1;
        }
        if self.api.enabled(WHAT_ABOUT_ZEBRANKY) {
            self.api
                .response(WHAT_ABOUT_ZEBRANKY, Handler::ZoqFotInfo, &[]);
            info_left = 1;
        }
        if self.api.enabled(WHAT_ABOUT_STINGER) {
            self.api
                .response(WHAT_ABOUT_STINGER, Handler::ZoqFotInfo, &[]);
            info_left = 1;
        }
        if self.api.enabled(WHAT_ABOUT_GUY_IN_BACK) {
            self.api
                .response(WHAT_ABOUT_GUY_IN_BACK, Handler::ZoqFotInfo, &[]);
            info_left = 1;
        }
        if self.api.enabled(WHAT_ABOUT_PAST) {
            self.api.response(WHAT_ABOUT_PAST, Handler::ZoqFotInfo, &[]);
            info_left = 1;
        }
        self.api.response(ENOUGH_INFO, Handler::ZoqFotHome, &[]);
        if !(info_left != 0) {
            self.api.disable(WANT_SPECIFIC_INFO);
        }
    }
    fn zoq_fot_home(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_HOMEWORLD {
            num_visits = self.api.get("ZOQFOT_INFO");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.speak(GENERAL_INFO_10, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_11, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_12, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_13, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
                1 => {
                    self.api.speak(GENERAL_INFO_20, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_21, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_22, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_23, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_24, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_25, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_26, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_27, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
                2 => {
                    self.api.speak(GENERAL_INFO_30, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_31, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_32, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_33, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_34, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_35, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
                3 => {
                    self.api.speak(GENERAL_INFO_40, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_41, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_42, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_43, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_44, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_45, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_46, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_47, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_48, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_49, Speaker::Pik);
                    self.api.speak(GENERAL_INFO_410, Speaker::Zoq);
                    self.api.speak(GENERAL_INFO_411, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ZOQFOT_INFO", num_visits);
            self.api.disable(WHATS_UP_HOMEWORLD);
        } else if r == ANY_WAR_NEWS {
            let mut know_mask: u8;
            num_visits = self.api.get("UTWIG_SUPOX_MISSION");
            know_mask = self.api.get("ZOQFOT_KNOW_MASK");
            if !((know_mask & 8) != 0) && self.api.get("KOHR_AH_FRENZY") != 0 {
                self.api.speak(KOHRAH_FRENZY0, Speaker::Zoq);
                self.api.speak(KOHRAH_FRENZY1, Speaker::Pik);
                self.api.speak(KOHRAH_FRENZY2, Speaker::Zoq);
                self.api.speak(KOHRAH_FRENZY3, Speaker::Pik);
                self.api.speak(KOHRAH_FRENZY4, Speaker::Zoq);
                self.api.speak(KOHRAH_FRENZY5, Speaker::Pik);
                self.api.speak(KOHRAH_FRENZY6, Speaker::Zoq);
                self.api.speak(KOHRAH_FRENZY7, Speaker::Pik);
                self.api.speak(KOHRAH_FRENZY8, Speaker::Zoq);
                self.api.speak(KOHRAH_FRENZY9, Speaker::Pik);
                self.api.speak(KOHRAH_FRENZY10, Speaker::Zoq);
                self.api.speak(KOHRAH_FRENZY11, Speaker::Pik);
                self.api.begin_talk(!0u16);
                know_mask = 15;
            } else if (!((know_mask & 1) != 0) && (num_visits > 0)) && (num_visits < 5) {
                self.api.speak(UTWIG_DELAY0, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY1, Speaker::Pik);
                self.api.speak(UTWIG_DELAY2, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY3, Speaker::Pik);
                self.api.speak(UTWIG_DELAY4, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY5, Speaker::Pik);
                self.api.speak(UTWIG_DELAY6, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY7, Speaker::Pik);
                self.api.speak(UTWIG_DELAY8, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY9, Speaker::Pik);
                self.api.speak(UTWIG_DELAY10, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY11, Speaker::Pik);
                self.api.speak(UTWIG_DELAY12, Speaker::Zoq);
                self.api.speak(UTWIG_DELAY13, Speaker::Pik);
                self.api.begin_talk(!0u16);
                know_mask |= 1;
            } else {
                let mut i: i16;
                i = 2155 + 4;
                if num_visits != 0 {
                    i += 1;
                }
                if ({
                    i -= self.api.date().0;
                    i
                } == 1)
                    && (self.api.date().1 > 2)
                {
                    i = 0;
                }
                if !((know_mask & 4) != 0) && (i <= 0) {
                    self.api.speak(URQUAN_NEARLY_GONE0, Speaker::Zoq);
                    self.api.speak(URQUAN_NEARLY_GONE1, Speaker::Pik);
                    self.api.speak(URQUAN_NEARLY_GONE2, Speaker::Zoq);
                    self.api.speak(URQUAN_NEARLY_GONE3, Speaker::Pik);
                    self.api.speak(URQUAN_NEARLY_GONE4, Speaker::Zoq);
                    self.api.speak(URQUAN_NEARLY_GONE5, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                    know_mask |= 2 | 4;
                } else if !((know_mask & 2) != 0) && (i == 1) {
                    self.api.speak(KOHRAH_WINNING0, Speaker::Zoq);
                    self.api.speak(KOHRAH_WINNING1, Speaker::Pik);
                    self.api.speak(KOHRAH_WINNING2, Speaker::Zoq);
                    self.api.speak(KOHRAH_WINNING3, Speaker::Pik);
                    self.api.speak(KOHRAH_WINNING4, Speaker::Zoq);
                    self.api.speak(KOHRAH_WINNING5, Speaker::Pik);
                    self.api.speak(KOHRAH_WINNING6, Speaker::Zoq);
                    self.api.speak(KOHRAH_WINNING7, Speaker::Pik);
                    self.api.speak(KOHRAH_WINNING8, Speaker::Zoq);
                    self.api.speak(KOHRAH_WINNING9, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                    know_mask |= 2;
                } else {
                    self.api.speak(NO_WAR_NEWS0, Speaker::Zoq);
                    self.api.speak(NO_WAR_NEWS1, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
            }
            self.api.set("ZOQFOT_KNOW_MASK", know_mask);
            self.api.disable(ANY_WAR_NEWS);
        } else if r == I_WANT_ALLIANCE {
            self.api.speak(GOOD0, Speaker::Zoq);
            self.api.speak(GOOD1, Speaker::Pik);
            self.api.speak(GOOD2, Speaker::Zoq);
            self.api.speak(GOOD3, Speaker::Pik);
            self.api.speak(GOOD4, Speaker::Zoq);
            self.api.speak(GOOD5, Speaker::Pik);
            self.api.speak(GOOD6, Speaker::Zoq);
            self.api.speak(GOOD7, Speaker::Pik);
            self.api.speak(GOOD8, Speaker::Zoq);
            self.api.speak(GOOD9, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.set_allied(ZOQFOTPIK_SHIP, 1 != 0);
            self.api.event(1, 3, 0, 0, 6);
        } else if r == ENOUGH_INFO {
            self.api.speak(OK_ENOUGH_INFO, Speaker::Zoq);
            self.api.begin_talk(!0u16);
        }
        if self.api.enabled(WHATS_UP_HOMEWORLD) {
            self.api
                .response(WHATS_UP_HOMEWORLD, Handler::ZoqFotHome, &[]);
        }
        if self.api.enabled(ANY_WAR_NEWS) {
            self.api.response(ANY_WAR_NEWS, Handler::ZoqFotHome, &[]);
        }
        if self.api.alliance(ZOQFOTPIK_SHIP) != 1 {
            self.api.response(I_WANT_ALLIANCE, Handler::ZoqFotHome, &[]);
        } else if self.api.enabled(WANT_SPECIFIC_INFO) {
            self.api
                .response(WANT_SPECIFIC_INFO, Handler::ZoqFotInfo, &[]);
        }
        self.api
            .response(BYE_HOMEWORLD, Handler::ExitConversation, &[]);
    }
    fn intro(&mut self) {
        let mut num_visits: u8;
        if (self.api.activity() as u8) == 5 {
            self.api.speak(OUT_TAKES0, Speaker::Zoq);
            self.api.speak(OUT_TAKES1, Speaker::Pik);
            self.api.speak(OUT_TAKES2, Speaker::Zoq);
            self.api.speak(OUT_TAKES3, Speaker::Pik);
            self.api.speak(OUT_TAKES4, Speaker::Zoq);
            self.api.speak(OUT_TAKES5, Speaker::Pik);
            self.api.speak(OUT_TAKES6, Speaker::Zoq);
            self.api.speak(OUT_TAKES7, Speaker::Pik);
            self.api.speak(OUT_TAKES8, Speaker::Zoq);
            self.api.speak(OUT_TAKES9, Speaker::Pik);
            self.api.speak(OUT_TAKES10, Speaker::Zoq);
            self.api.speak(OUT_TAKES11, Speaker::Pik);
            self.api.speak(OUT_TAKES12, Speaker::Zoq);
            self.api.speak(OUT_TAKES13, Speaker::Pik);
            self.api.begin_talk(!0u16);
            self.api.segue(0);
            return;
        }
        if self.api.get("ZOQFOT_HOSTILE") != 0 {
            num_visits = self.api.get("ZOQFOT_HOME_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.speak(HOSTILE_HELLO_10, Speaker::Zoq);
                    self.api.speak(HOSTILE_HELLO_11, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
                1 => {
                    self.api.speak(HOSTILE_HELLO_20, Speaker::Zoq);
                    self.api.speak(HOSTILE_HELLO_21, Speaker::Pik);
                    self.api.speak(HOSTILE_HELLO_22, Speaker::Zoq);
                    self.api.speak(HOSTILE_HELLO_23, Speaker::Pik);
                    self.api.speak(HOSTILE_HELLO_24, Speaker::Zoq);
                    self.api.speak(HOSTILE_HELLO_25, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
                2 => {
                    self.api.speak(HOSTILE_HELLO_30, Speaker::Zoq);
                    self.api.speak(HOSTILE_HELLO_31, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                }
                3 => {
                    self.api.speak(HOSTILE_HELLO_40, Speaker::Zoq);
                    self.api.speak(HOSTILE_HELLO_41, Speaker::Pik);
                    self.api.begin_talk(!0u16);
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("ZOQFOT_HOME_VISITS", num_visits);
            self.api.segue(1);
        } else if !(self.api.get("MET_ZOQFOT") != 0) {
            self.api.set("MET_ZOQFOT", 1_u8);
            self.api.speak(WE_ARE0, Speaker::Zoq);
            self.api.speak(WE_ARE1, Speaker::Pik);
            self.api.speak(WE_ARE2, Speaker::Zoq);
            self.api.speak(WE_ARE3, Speaker::Pik);
            self.api.speak(WE_ARE4, Speaker::Zoq);
            self.api.speak(WE_ARE5, Speaker::Pik);
            self.api.speak(WE_ARE6, Speaker::Zoq);
            self.api.speak(WE_ARE7, Speaker::Pik);
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                self.api.speak(INIT_HOME_HELLO0, Speaker::Zoq);
                self.api.speak(INIT_HOME_HELLO1, Speaker::Pik);
                self.api.speak(INIT_HOME_HELLO2, Speaker::Zoq);
                self.api.speak(INIT_HOME_HELLO3, Speaker::Pik);
            } else {
                self.api.speak(SCOUT_HELLO0, Speaker::Zoq);
                self.api.speak(SCOUT_HELLO1, Speaker::Pik);
                self.api.speak(SCOUT_HELLO2, Speaker::Zoq);
                self.api.speak(SCOUT_HELLO3, Speaker::Pik);
            }
            self.api.begin_talk(!0u16);
            self.aquaint_zoq_fot(0);
        } else {
            if self.api.get("ZOQFOT_DISTRESS") != 0 {
                self.api.speak(THANKS_FOR_RESCUE0, Speaker::Zoq);
                self.api.speak(THANKS_FOR_RESCUE1, Speaker::Pik);
                self.api.speak(THANKS_FOR_RESCUE2, Speaker::Zoq);
                self.api.speak(THANKS_FOR_RESCUE3, Speaker::Pik);
                self.api.speak(THANKS_FOR_RESCUE4, Speaker::Zoq);
                self.api.speak(THANKS_FOR_RESCUE5, Speaker::Pik);
                self.api.speak(THANKS_FOR_RESCUE6, Speaker::Zoq);
                self.api.speak(THANKS_FOR_RESCUE7, Speaker::Pik);
                self.api.speak(THANKS_FOR_RESCUE8, Speaker::Zoq);
                self.api.speak(THANKS_FOR_RESCUE9, Speaker::Pik);
                self.api.speak(THANKS_FOR_RESCUE10, Speaker::Zoq);
                self.api.speak(THANKS_FOR_RESCUE11, Speaker::Pik);
                self.api.begin_talk(!0u16);
                self.api.set("ZOQFOT_DISTRESS", 0_u8);
                self.api.add_escorts(ZOQFOTPIK_SHIP, 4);
            } else {
                num_visits = self.api.get("ZOQFOT_HOME_VISITS");
                if self.api.alliance(ZOQFOTPIK_SHIP) != 1 {
                    let res = {
                        let old = num_visits;
                        num_visits += 1;
                        old
                    };
                    match res {
                        0 => {
                            self.api.speak(NEUTRAL_HOME_HELLO_10, Speaker::Zoq);
                            self.api.speak(NEUTRAL_HOME_HELLO_11, Speaker::Pik);
                            self.api.speak(NEUTRAL_HOME_HELLO_12, Speaker::Zoq);
                            self.api.speak(NEUTRAL_HOME_HELLO_13, Speaker::Pik);
                        }
                        1 => {
                            self.api.speak(NEUTRAL_HOME_HELLO_20, Speaker::Zoq);
                            self.api.speak(NEUTRAL_HOME_HELLO_21, Speaker::Pik);
                            self.api.speak(NEUTRAL_HOME_HELLO_22, Speaker::Zoq);
                            self.api.speak(NEUTRAL_HOME_HELLO_23, Speaker::Pik);
                            num_visits -= 1;
                        }
                        _ => {}
                    }
                    self.api.begin_talk(!0u16);
                } else {
                    let res = {
                        let old = num_visits;
                        num_visits += 1;
                        old
                    };
                    match res {
                        0 => {
                            self.api.speak(ALLIED_HOME_HELLO_10, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_11, Speaker::Pik);
                            self.api.speak(ALLIED_HOME_HELLO_12, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_13, Speaker::Pik);
                            self.api.begin_talk(!0u16);
                        }
                        1 => {
                            self.api.speak(ALLIED_HOME_HELLO_20, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_21, Speaker::Pik);
                            self.api.speak(ALLIED_HOME_HELLO_22, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_23, Speaker::Pik);
                            self.api.speak(ALLIED_HOME_HELLO_24, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_25, Speaker::Pik);
                            self.api.speak(ALLIED_HOME_HELLO_26, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_27, Speaker::Pik);
                            self.api.begin_talk(!0u16);
                        }
                        2 => {
                            self.api.speak(ALLIED_HOME_HELLO_30, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_31, Speaker::Pik);
                            self.api.begin_talk(!0u16);
                        }
                        3 => {
                            self.api.speak(ALLIED_HOME_HELLO_40, Speaker::Zoq);
                            self.api.speak(ALLIED_HOME_HELLO_41, Speaker::Pik);
                            self.api.begin_talk(!0u16);
                            num_visits -= 1;
                        }
                        _ => {}
                    }
                }
                self.api.set("ZOQFOT_HOME_VISITS", num_visits);
            }
            self.zoq_fot_home(0);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.zoqfotpik.graphics".as_ptr(),
        alien_font_res: c"comm.zoqfotpik.font".as_ptr(),
        alien_colormap_res: c"comm.zoqfotpik.colortable".as_ptr(),
        alien_song_res: c"comm.zoqfotpik.music".as_ptr(),
        conversation_phrases_res: c"comm.zoqfotpik.dialogue".as_ptr(),
        alien_text_fcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_bcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: CPoint { x: 0, y: 0 },
        alien_text_width: 119,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 3,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(1, 4, 12, (35, 0), (0, 8400));
    d.alien_ambient_array[1] = animation(5, 5, 2, (49, 0), (1680, 0));
    d.alien_ambient_array[2] = animation(10, 8, 2, (56, 0), (0, 8400));
    d.alien_transition_desc = animation(23, 3, 0, (28, 0), (0, 0));
    d.alien_talk_desc = animation(18, 5, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;

    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn zoq_fot_intro(r: u32) {
        run(|s| {
            s.handle(
                Handler::ZoqFotIntro,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn form_alliance(r: u32) {
        run(|s| {
            s.handle(
                Handler::FormAlliance,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn aquaint_zoq_fot(r: u32) {
        run(|s| {
            s.handle(
                Handler::AquaintZoqFot,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn zoq_fot_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::ZoqFotInfo,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn zoq_fot_home(r: u32) {
        run(|s| {
            s.handle(
                Handler::ZoqFotHome,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }

    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::ExitConversation => exit_conversation,
            Handler::ZoqFotIntro => zoq_fot_intro,
            Handler::FormAlliance => form_alliance,
            Handler::AquaintZoqFot => aquaint_zoq_fot,
            Handler::ZoqFotInfo => zoq_fot_info,
            Handler::ZoqFotHome => zoq_fot_home,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {}
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_zoqfot_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct ZoqfotDialogue;
impl super::RaceDialogue for ZoqfotDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_zoqfot_comm()) }
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
