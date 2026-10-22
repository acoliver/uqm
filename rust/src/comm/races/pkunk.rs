//! Pkunk dialogue, ported from pkunkc.c.
use super::script::{animation, as_comm_data, GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    PkunkAngry,
    DiscussConquer,
    OfferAlliance,
    AboutPkunk,
    AboutIlwrath,
    PkunkHome,
    PkunkFriendlySpace,
    PkunkNeutralSpace,
    PkunkMigrate,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
mod api;
use api::PkunkApi;
impl<A: PkunkApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::PkunkAngry => self.pkunk_angry(r),
            Handler::DiscussConquer => self.discuss_conquer(r),
            Handler::OfferAlliance => self.offer_alliance(r),
            Handler::AboutPkunk => self.about_pkunk(r),
            Handler::AboutIlwrath => self.about_ilwrath(r),
            Handler::PkunkHome => self.pkunk_home(r),
            Handler::PkunkFriendlySpace => self.pkunk_friendly_space(r),
            Handler::PkunkNeutralSpace => self.pkunk_neutral_space(r),
            Handler::PkunkMigrate => self.pkunk_migrate(r),
        }
    }
    fn ships_ready(&mut self) -> bool {
        let (year, month, day) = self.api.date();
        self.api.get("PKUNK_MANNER") == 3
            && (i32::from(year) - 2155, month, day)
                >= (
                    i32::from(self.api.get("PKUNK_SHIP_YEAR")),
                    u16::from(self.api.get("PKUNK_SHIP_MONTH")),
                    u16::from(self.api.get("PKUNK_SHIP_DAY")),
                )
    }
    fn prepare_ship(&mut self) {
        if self.api.gift_ships() != 0 {
            let (year, month, day) = self.api.date();
            self.api.set("PKUNK_SHIP_MONTH", month as u8);
            self.api.set("PKUNK_SHIP_DAY", day.min(28) as u8);
            self.api.set(
                "PKUNK_SHIP_YEAR",
                (year.wrapping_sub(2155) as u8).wrapping_add(1),
            );
        }
    }
    fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0_u32);
        if r == FRIENDLY_BYE_SPACE {
            self.api.npc(i32::from(FRIENDLY_GOODBYE_SPACE));
        } else if r == NEUTRAL_BYE_SPACE {
            self.api.npc(i32::from(NEUTRAL_GOODBYE_SPACE));
        } else if r == BYE_ANGRY {
            self.api.npc(i32::from(GOODBYE_ANGRY));
        } else if r == BYE_FRIENDLY {
            self.api.npc(i32::from(GOODBYE_FRIENDLY));
        } else if (r == WE_HERE_TO_HELP) || (r == WE_NEED_HELP) {
            if r == WE_HERE_TO_HELP {
                self.api.npc(i32::from(NEED_HELP));
            } else {
                self.api.npc(i32::from(GIVE_HELP));
            }
            self.api.npc(i32::from(ALMOST_ALLIANCE));
            self.api.set("PKUNK_MANNER", 3_u8);
            self.api.set("PKUNK_VISITS", 0_u8);
            self.api.set("PKUNK_HOME_VISITS", 0_u8);
            self.api.set("PKUNK_INFO", 0_u8);
            self.api.event(1, 6_u16, 0_u16, 0_u16, 4_u8);
            if i32::from(
                self.api
                    .escort_feasibility((crate::comm::dispatch::ship::PKUNK_SHIP) as u16),
            ) == 0
            {
                self.api.npc(i32::from(INIT_NO_ROOM));
            } else {
                self.api.npc(i32::from(INIT_SHIP_GIFT));
                self.api.talk_segue(!0_u16);
                self.prepare_ship();
            }
        } else if r == TRY_TO_BE_NICER {
            self.api.npc(i32::from(CANT_ASK_FOR_MORE));
            self.api.npc(i32::from(VISIT_OUR_HOMEWORLD));
            self.api.set("PKUNK_MANNER", 3_u8);
            self.api.set("PKUNK_VISITS", 0_u8);
            self.api.set("PKUNK_HOME_VISITS", 0_u8);
            self.api.set("PKUNK_INFO", 0_u8);
        } else if (r == MUST_CONQUER) || (r == OBEY) {
            if r == OBEY {
                self.api.npc(i32::from(NO_OBEY));
            } else {
                self.api.npc(i32::from(BAD_IDEA));
                self.api.set("PKUNK_MANNER", 2_u8);
            }
            self.api.segue(1_u32);
        } else if r == DIE_IDIOT_FOOLS {
            self.api.npc(i32::from(VERY_WELL));
            self.api.segue(1_u32);
        } else if r == SUIT_YOURSELF {
            self.api.npc(i32::from(GOODBYE_MIGRATION));
        } else {
            let mut reason_mask: u8;
            reason_mask = self.api.get("PKUNK_REASONS");
            if r == GOOD_REASON_1 {
                self.api.npc(i32::from(WE_GO_HOME_1));
                reason_mask |= (1 << 0) as u8;
                self.api.event(1, 0_u16, 0_u16, 0_u16, 4_u8);
            } else if r == GOOD_REASON_2 {
                self.api.npc(i32::from(WE_GO_HOME_2));
                reason_mask |= (1 << 1) as u8;
                self.api.event(1, 0_u16, 0_u16, 0_u16, 4_u8);
            } else if r == BAD_REASON_1 {
                self.api.npc(i32::from(NO_GO_HOME_1));
                reason_mask |= (1 << 2) as u8;
            } else if r == BAD_REASON_2 {
                self.api.npc(i32::from(NO_GO_HOME_2));
                reason_mask |= (1 << 3) as u8;
            }
            self.api.set("PKUNK_REASONS", reason_mask);
        }
    }
    fn pkunk_angry(&mut self, r: u16) {
        if r == WHY_INSULTS {
            self.api.npc(i32::from(RELEASE_TENSION));
            self.api.disable(WHY_INSULTS);
        } else if r == WHAT_ABOUT_YOU {
            self.api.npc(i32::from(ABOUT_US));
            self.api.disable(WHAT_ABOUT_YOU);
        } else if r == SHOULD_BE_FRIENDS {
            self.api.npc(i32::from(YES_FRIENDS));
            self.api.disable(SHOULD_BE_FRIENDS);
        }
        if self.api.enabled(SHOULD_BE_FRIENDS) {
            self.api
                .response(SHOULD_BE_FRIENDS, Handler::PkunkAngry, &[]);
        } else {
            if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0 {
                self.api.response(TRY_TO_BE_NICER, Handler::PkunkHome, &[]);
            } else {
                self.api
                    .response(TRY_TO_BE_NICER, Handler::ExitConversation, &[]);
            }
        }
        self.api
            .response(DIE_IDIOT_FOOLS, Handler::ExitConversation, &[]);
        if self.api.enabled(WHY_INSULTS) {
            self.api.response(WHY_INSULTS, Handler::PkunkAngry, &[]);
        }
        if self.api.enabled(WHAT_ABOUT_YOU) {
            self.api.response(WHAT_ABOUT_YOU, Handler::PkunkAngry, &[]);
        }
        self.api.response(BYE_ANGRY, Handler::ExitConversation, &[]);
    }
    fn discuss_conquer(&mut self, r: u16) {
        if r == WE_CONQUER {
            self.api.npc(i32::from(WHY_CONQUER));
            self.api.disable(WE_CONQUER);
        } else if r == CONQUER_BECAUSE_1 {
            self.api.npc(i32::from(NOT_CONQUER_1));
            self.api.disable(CONQUER_BECAUSE_1);
        } else if r == CONQUER_BECAUSE_2 {
            self.api.npc(i32::from(NOT_CONQUER_2));
            self.api.disable(CONQUER_BECAUSE_2);
        }
        if self.api.enabled(CONQUER_BECAUSE_1) {
            self.api
                .response(CONQUER_BECAUSE_1, Handler::DiscussConquer, &[]);
        }
        if self.api.enabled(CONQUER_BECAUSE_2) {
            self.api
                .response(CONQUER_BECAUSE_2, Handler::DiscussConquer, &[]);
        }
        self.api
            .response(MUST_CONQUER, Handler::ExitConversation, &[]);
        self.api.response(NO_CONQUEST, Handler::PkunkHome, &[]);
    }
    fn offer_alliance(&mut self, r: u16) {
        if r == WE_ARE_VINDICATOR0 {
            self.api.npc(i32::from(WHY_YOU_HERE));
        } else if r == EXPLORING_UNIVERSE {
            self.api.npc(i32::from(SENSE_DEEPER_CONFLICT));
            self.api.disable(EXPLORING_UNIVERSE);
        } else if r == FUN_CRUISE {
            self.api.npc(i32::from(REPRESS));
            self.api.disable(FUN_CRUISE);
        }
        self.api
            .response(WE_HERE_TO_HELP, Handler::ExitConversation, &[]);
        self.api
            .response(WE_NEED_HELP, Handler::ExitConversation, &[]);
        if self.api.enabled(EXPLORING_UNIVERSE) {
            self.api
                .response(EXPLORING_UNIVERSE, Handler::OfferAlliance, &[]);
        }
        if self.api.enabled(FUN_CRUISE) {
            self.api.response(FUN_CRUISE, Handler::OfferAlliance, &[]);
        }
    }
    fn about_pkunk(&mut self, r: u16) {
        let mut info_left: u8;
        info_left = u8::from(false);
        if r == WHAT_ABOUT_YOU {
            self.api.npc(i32::from(ABOUT_US));
        } else if r == WHAT_ABOUT_HISTORY {
            self.api.npc(i32::from(ABOUT_HISTORY));
            self.api.disable(WHAT_ABOUT_HISTORY);
        } else if r == WHAT_ABOUT_YEHAT {
            self.api.npc(i32::from(ABOUT_YEHAT));
            self.api.disable(WHAT_ABOUT_YEHAT);
        } else if r == WHAT_ABOUT_CULTURE {
            self.api.npc(i32::from(ABOUT_CULTURE));
            self.api.disable(WHAT_ABOUT_CULTURE);
        } else if r == ELABORATE_CULTURE {
            self.api.npc(i32::from(OK_ELABORATE_CULTURE));
            self.api.disable(ELABORATE_CULTURE);
        } else if r == WHAT_ABOUT_FUTURE {
            self.api.npc(i32::from(ABOUT_FUTURE));
            self.api.disable(WHAT_ABOUT_FUTURE);
        }
        if self.api.enabled(WHAT_ABOUT_HISTORY) {
            self.api
                .response(WHAT_ABOUT_HISTORY, Handler::AboutPkunk, &[]);
            info_left = u8::from(true);
        } else if self.api.enabled(WHAT_ABOUT_YEHAT) {
            self.api
                .response(WHAT_ABOUT_YEHAT, Handler::AboutPkunk, &[]);
            info_left = u8::from(true);
        }
        if self.api.enabled(WHAT_ABOUT_CULTURE) {
            self.api
                .response(WHAT_ABOUT_CULTURE, Handler::AboutPkunk, &[]);
            info_left = u8::from(true);
        } else if self.api.enabled(ELABORATE_CULTURE) {
            self.api
                .response(ELABORATE_CULTURE, Handler::AboutPkunk, &[]);
            info_left = u8::from(true);
        }
        if self.api.enabled(WHAT_ABOUT_FUTURE) {
            self.api
                .response(WHAT_ABOUT_FUTURE, Handler::AboutPkunk, &[]);
            info_left = u8::from(true);
        }
        self.api.response(ENOUGH_ABOUT_YOU, Handler::PkunkHome, &[]);
        if !(info_left != 0) {
            self.api.disable(WHAT_ABOUT_YOU);
        }
    }
    fn about_ilwrath(&mut self, r: u16) {
        let mut info_left: u8;
        info_left = u8::from(false);
        if r == WHAT_ABOUT_ILWRATH {
            self.api.npc(i32::from(ABOUT_ILWRATH));
        } else if r == WHY_ILWRATH_FIGHT {
            self.api.npc(i32::from(ILWRATH_FIGHT_BECAUSE));
            self.api.disable(WHY_ILWRATH_FIGHT);
        } else if r == WHEN_FIGHT_START {
            self.api.npc(i32::from(FIGHT_START_WHEN));
            self.api.disable(WHEN_FIGHT_START);
        } else if r == HOW_GOES_FIGHT {
            self.api.npc(i32::from(FIGHT_GOES));
            self.api.disable(HOW_GOES_FIGHT);
        } else if r == HOW_STOP_FIGHT {
            self.api.npc(i32::from(STOP_FIGHT_LIKE_SO));
            self.api.disable(HOW_STOP_FIGHT);
        }
        if self.api.enabled(WHY_ILWRATH_FIGHT) {
            self.api
                .response(WHY_ILWRATH_FIGHT, Handler::AboutIlwrath, &[]);
            info_left = u8::from(true);
        }
        if self.api.enabled(WHEN_FIGHT_START) {
            self.api
                .response(WHEN_FIGHT_START, Handler::AboutIlwrath, &[]);
            info_left = u8::from(true);
        }
        if self.api.enabled(HOW_GOES_FIGHT) {
            self.api
                .response(HOW_GOES_FIGHT, Handler::AboutIlwrath, &[]);
            info_left = u8::from(true);
        }
        if self.api.enabled(HOW_STOP_FIGHT) {
            self.api
                .response(HOW_STOP_FIGHT, Handler::AboutIlwrath, &[]);
            info_left = u8::from(true);
        }
        self.api.response(ENOUGH_ILWRATH, Handler::PkunkHome, &[]);
        if !(info_left != 0) {
            self.api.disable(WHAT_ABOUT_ILWRATH);
        }
    }
    fn pkunk_home(&mut self, r: u16) {
        let fragments: Vec<i32>;
        let mut num_visits: u8;
        if r == NO_CONQUEST {
            self.api.npc(i32::from(GOOD_IDEA));
        } else if r == ENOUGH_ILWRATH {
            self.api.npc(i32::from(OK_ENOUGH_ILWRATH));
        } else if r == ENOUGH_ABOUT_YOU {
            self.api.npc(i32::from(OK_ENOUGH_ABOUT_US));
        } else if ((r == WHERE_FLEET_1) || (r == WHERE_FLEET_2)) || (r == WHERE_FLEET_3) {
            self.api.set("PKUNK_SWITCH", 1_u8);
            if !((i32::from(self.api.get("PKUNK_MISSION")) & 1) != 0) {
                num_visits = self.api.get("PKUNK_RETURN");
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(RETURNING_FROM_YEHAT_1));
                    }
                    1 => {
                        self.api.npc(i32::from(RETURNING_FROM_YEHAT_2));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set("PKUNK_RETURN", num_visits);
            } else {
                num_visits = self.api.get("PKUNK_MIGRATE");
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(MIGRATING_HOMEWORLD_1));
                    }
                    1 => {
                        self.api.npc(i32::from(MIGRATING_HOMEWORLD_2));
                    }
                    2 => {
                        self.api.npc(i32::from(MIGRATING_HOMEWORLD_3));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set("PKUNK_MIGRATE", num_visits);
            }
            num_visits = (i32::from(self.api.get("PKUNK_FLEET")) + 1) as u8;
            self.api.set("PKUNK_FLEET", num_visits);
            self.api.disable(WHERE_FLEET_1);
        } else if ((r == AM_WORRIED_1) || (r == AM_WORRIED_2)) || (r == AM_WORRIED_3) {
            num_visits = self.api.get("PKUNK_WORRY");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(DONT_WORRY_1));
                }
                1 => {
                    self.api.npc(i32::from(DONT_WORRY_2));
                }
                2 => {
                    self.api.npc(i32::from(DONT_WORRY_3));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_WORRY", num_visits);
            self.api.disable(AM_WORRIED_1);
        } else if r == TRY_TO_BE_NICER {
            self.api.npc(i32::from(CANT_ASK_FOR_MORE));
            if !(self.api.get("CLEAR_SPINDLE") != 0) {
                self.api.npc(i32::from(GIVE_SPINDLE));
                self.api.set("CLEAR_SPINDLE", 1_u8);
                self.api.set("CLEAR_SPINDLE_ON_SHIP", 1_u8);
            }
            self.api.npc(i32::from(CAN_BE_FRIENDS));
            self.api.set("PKUNK_MANNER", 3_u8);
            self.api.set("PKUNK_VISITS", 0_u8);
            self.api.set("PKUNK_HOME_VISITS", 0_u8);
        } else if r == WHAT_ABOUT_ILWRATH {
            self.api.npc(i32::from(ABOUT_ILWRATH));
            self.api.disable(WHAT_ABOUT_ILWRATH);
        }
        if self.api.enabled(WE_CONQUER) && (i32::from(self.api.get("PKUNK_MANNER")) == 0) {
            self.api.response(WE_CONQUER, Handler::DiscussConquer, &[]);
        }
        if self.api.get("PKUNK_ON_THE_MOVE") != 0 {
            if self.api.enabled(WHERE_FLEET_1) && !(self.api.get("PKUNK_SWITCH") != 0) {
                match self.api.get("PKUNK_FLEET") {
                    0 => {
                        self.api.response(WHERE_FLEET_1, Handler::PkunkHome, &[]);
                    }
                    1 => {
                        self.api.response(WHERE_FLEET_2, Handler::PkunkHome, &[]);
                    }
                    2 => {
                        self.api.response(WHERE_FLEET_3, Handler::PkunkHome, &[]);
                    }
                    _ => {}
                }
            } else if (!self.api.enabled(WHERE_FLEET_1) && self.api.enabled(AM_WORRIED_1))
                && ((i32::from(self.api.get("PKUNK_MISSION")) & 1) != 0)
            {
                match self.api.get("PKUNK_WORRY") {
                    0 => {
                        self.api.response(AM_WORRIED_1, Handler::PkunkHome, &[]);
                    }
                    1 => {
                        self.api.response(AM_WORRIED_2, Handler::PkunkHome, &[]);
                    }
                    2 => {
                        self.api.response(AM_WORRIED_3, Handler::PkunkHome, &[]);
                    }
                    _ => {}
                }
            }
        }
        if !(self.api.get("PKUNK_SHIP_MONTH") != 0) {
            fragments = vec![
                i32::from(WE_ARE_VINDICATOR0),
                GLOBAL_PLAYER_NAME,
                i32::from(WE_ARE_VINDICATOR1),
                GLOBAL_SHIP_NAME,
                i32::from(WE_ARE_VINDICATOR2),
            ];
            self.api
                .response(WE_ARE_VINDICATOR0, Handler::OfferAlliance, &fragments);
        }
        if self.api.enabled(WHAT_ABOUT_YOU) {
            self.api.response(WHAT_ABOUT_YOU, Handler::AboutPkunk, &[]);
        }
        if self.api.enabled(WHAT_ABOUT_ILWRATH) {
            if !(self.api.get("ILWRATH_DECEIVED") != 0) {
                self.api
                    .response(WHAT_ABOUT_ILWRATH, Handler::AboutIlwrath, &[]);
            } else {
                self.api
                    .response(WHAT_ABOUT_ILWRATH, Handler::PkunkHome, &[]);
            }
        }
        self.api
            .response(BYE_FRIENDLY, Handler::ExitConversation, &[]);
    }
    fn pkunk_friendly_space(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_SPACE {
            if self.ships_ready() {
                self.api.npc(i32::from(SHIPS_AT_HOME));
            } else {
                num_visits = self.api.get("PKUNK_INFO");
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
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
                    }
                    5 => {
                        self.api.npc(i32::from(GENERAL_INFO_SPACE_6));
                    }
                    6 => {
                        self.api.npc(i32::from(GENERAL_INFO_SPACE_7));
                    }
                    7 => {
                        self.api.npc(i32::from(GENERAL_INFO_SPACE_8));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set("PKUNK_INFO", num_visits);
            }
            self.api.disable(WHATS_UP_SPACE);
        } else if r == HOW_GOES_WAR {
            num_visits = self.api.get("PKUNK_WAR");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(WAR_GOES_1));
                    self.api.set("KNOW_URQUAN_STORY", 1_u8);
                    self.api.set("KNOW_KOHR_AH_STORY", 1_u8);
                }
                1 => {
                    self.api.npc(i32::from(WAR_GOES_2));
                }
                2 => {
                    self.api.npc(i32::from(WAR_GOES_3));
                }
                3 => {
                    self.api.npc(i32::from(WAR_GOES_4));
                    self.api.set("PKUNK_DONE_WAR", 1_u8);
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_WAR", num_visits);
            self.api.disable(HOW_GOES_WAR);
        } else if r == TELL_MY_FORTUNE {
            num_visits = self.api.get("PKUNK_FORTUNE");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(FORTUNE_IS_1));
                }
                1 => {
                    self.api.npc(i32::from(FORTUNE_IS_2));
                }
                2 => {
                    self.api.npc(i32::from(FORTUNE_IS_3));
                }
                3 => {
                    self.api.npc(i32::from(FORTUNE_IS_4));
                }
                4 => {
                    self.api.npc(i32::from(FORTUNE_IS_5));
                }
                5 => {
                    self.api.npc(i32::from(FORTUNE_IS_6));
                }
                6 => {
                    self.api.npc(i32::from(FORTUNE_IS_7));
                }
                7 => {
                    self.api.npc(i32::from(FORTUNE_IS_8));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_FORTUNE", num_visits);
            self.api.disable(TELL_MY_FORTUNE);
        }
        if self.api.enabled(WHATS_UP_SPACE) {
            self.api
                .response(WHATS_UP_SPACE, Handler::PkunkFriendlySpace, &[]);
        }
        if !(self.api.get("PKUNK_DONE_WAR") != 0) && self.api.enabled(HOW_GOES_WAR) {
            self.api
                .response(HOW_GOES_WAR, Handler::PkunkFriendlySpace, &[]);
        }
        if self.api.enabled(TELL_MY_FORTUNE) {
            self.api
                .response(TELL_MY_FORTUNE, Handler::PkunkFriendlySpace, &[]);
        }
        self.api
            .response(FRIENDLY_BYE_SPACE, Handler::ExitConversation, &[]);
    }
    fn pkunk_neutral_space(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == FORM_ALLIANCE {
            self.api.npc(i32::from(GO_TO_HOMEWORLD));
            self.api.disable(FORM_ALLIANCE);
        } else if r == CAN_YOU_HELP {
            self.api.npc(i32::from(GO_TO_HOMEWORLD_AGAIN));
            self.api.disable(CAN_YOU_HELP);
        } else if r == HOSTILE_GREETING {
            self.api.npc(i32::from(DONT_BE_HOSTILE));
            self.api.disable(HOSTILE_GREETING);
        } else if r == WHATS_UP_NEUTRAL {
            num_visits = self.api.get("PKUNK_INFO");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
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
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_6));
                }
                3 => {
                    self.api.npc(i32::from(GENERAL_INFO_SPACE_7));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_INFO", num_visits);
            self.api.disable(WHATS_UP_NEUTRAL);
        }
        if self.api.enabled(FORM_ALLIANCE) {
            self.api
                .response(FORM_ALLIANCE, Handler::PkunkNeutralSpace, &[]);
        } else if self.api.enabled(CAN_YOU_HELP) {
            self.api
                .response(CAN_YOU_HELP, Handler::PkunkNeutralSpace, &[]);
        }
        if self.api.enabled(HOSTILE_GREETING) {
            self.api
                .response(HOSTILE_GREETING, Handler::PkunkNeutralSpace, &[]);
        } else {
            self.api.response(OBEY, Handler::ExitConversation, &[]);
        }
        if self.api.enabled(WHATS_UP_NEUTRAL) {
            self.api
                .response(WHATS_UP_NEUTRAL, Handler::PkunkNeutralSpace, &[]);
        }
        self.api
            .response(NEUTRAL_BYE_SPACE, Handler::ExitConversation, &[]);
    }
    fn pkunk_migrate(&mut self, _r: u16) {
        let reason_mask: u8 = self.api.get("PKUNK_REASONS");
        if !((i32::from(reason_mask) & (1 << 0)) != 0) {
            self.api
                .response(GOOD_REASON_1, Handler::ExitConversation, &[]);
        }
        if !((i32::from(reason_mask) & (1 << 2)) != 0) {
            self.api
                .response(BAD_REASON_1, Handler::ExitConversation, &[]);
        }
        if !((i32::from(reason_mask) & (1 << 1)) != 0) {
            self.api
                .response(GOOD_REASON_2, Handler::ExitConversation, &[]);
        }
        if !((i32::from(reason_mask) & (1 << 3)) != 0) {
            self.api
                .response(BAD_REASON_2, Handler::ExitConversation, &[]);
        }
        self.api
            .response(SUIT_YOURSELF, Handler::ExitConversation, &[]);
    }
    fn intro(&mut self) {
        let mut num_visits: u8;
        if i32::from((self.api.activity()) as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0_u32);
            return;
        }
        let manner: u8 = self.api.get("PKUNK_MANNER");
        if i32::from(manner) == 2 {
            num_visits = self.api.get("PKUNK_VISITS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HATE_YOU_FOREVER_1));
                }
                1 => {
                    self.api.npc(i32::from(HATE_YOU_FOREVER_2));
                }
                2 => {
                    self.api.npc(i32::from(HATE_YOU_FOREVER_3));
                }
                3 => {
                    self.api.npc(i32::from(HATE_YOU_FOREVER_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_VISITS", num_visits);
            self.api.segue(1_u32);
        } else if i32::from(manner) == 1 {
            num_visits = self.api.get("PKUNK_VISITS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(SPIRITUAL_PROBLEMS_1));
                }
                1 => {
                    self.api.npc(i32::from(SPIRITUAL_PROBLEMS_2));
                }
                2 => {
                    self.api.npc(i32::from(SPIRITUAL_PROBLEMS_3));
                }
                3 => {
                    self.api.npc(i32::from(SPIRITUAL_PROBLEMS_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_VISITS", num_visits);
            self.pkunk_angry(0_u16);
        } else if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0 {
            if !(self.api.get("CLEAR_SPINDLE") != 0) {
                self.api.npc(i32::from(GIVE_SPINDLE));
                self.api.set("CLEAR_SPINDLE", 1_u8);
                self.api.set("CLEAR_SPINDLE_ON_SHIP", 1_u8);
            } else if (!(self.api.get("PKUNK_SENSE_VICTOR") != 0)
                && (i32::from(self.api.date().0) > 2155))
                && !(self.api.get("KOHR_AH_FRENZY") != 0)
            {
                self.api.npc(i32::from(SENSE_KOHRAH_VICTORY));
                self.api.set("PKUNK_SENSE_VICTOR", 1_u8);
            }
            num_visits = self.api.get("PKUNK_HOME_VISITS");
            if i32::from(manner) == 0 {
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_2));
                    }
                    2 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_4));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
            } else {
                if (num_visits != 0) && self.ships_ready() {
                    if i32::from(
                        self.api
                            .escort_feasibility((crate::comm::dispatch::ship::PKUNK_SHIP) as u16),
                    ) == 0
                    {
                        self.api.npc(i32::from(NO_ROOM));
                    } else {
                        self.api.npc(i32::from(SHIP_GIFT));
                        self.prepare_ship();
                    }
                } else {
                    let res = {
                        let old = num_visits;
                        num_visits = num_visits.wrapping_add(1);
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_1));
                        }
                        1 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_2));
                        }
                        2 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_3));
                        }
                        3 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_4));
                        }
                        4 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_5));
                        }
                        5 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_6));
                        }
                        6 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_7));
                        }
                        7 => {
                            self.api.npc(i32::from(FRIENDLY_HOMEWORLD_HELLO_8));
                            {
                                num_visits = num_visits.wrapping_sub(1);
                                num_visits
                            };
                        }
                        _ => {}
                    }
                }
            }
            self.api.set("PKUNK_HOME_VISITS", num_visits);
            self.pkunk_home(0_u16);
        } else if (i32::from({
            num_visits = self.api.get("PKUNK_MISSION");
            num_visits
        }) == 0)
            || !((i32::from(num_visits) & 1) != 0)
        {
            num_visits = self.api.get("PKUNK_VISITS");
            if i32::from(manner) == 3 {
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_2));
                    }
                    2 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_4));
                    }
                    4 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_5));
                    }
                    5 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_6));
                    }
                    6 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_7));
                    }
                    7 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_8));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.pkunk_friendly_space(0_u16);
            } else {
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(NEUTRAL_SPACE_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(NEUTRAL_SPACE_HELLO_2));
                    }
                    2 => {
                        self.api.npc(i32::from(NEUTRAL_SPACE_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(NEUTRAL_SPACE_HELLO_4));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.pkunk_neutral_space(0_u16);
            }
            self.api.set("PKUNK_VISITS", num_visits);
        } else {
            num_visits = self.api.get("PKUNK_MIGRATE_VISITS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_1));
                }
                1 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_2));
                }
                2 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_3));
                }
                3 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_4));
                }
                4 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_5));
                }
                5 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_6));
                }
                6 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_7));
                }
                7 => {
                    self.api.npc(i32::from(MIGRATING_SPACE_8));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("PKUNK_MIGRATE_VISITS", num_visits);
            self.pkunk_migrate(0_u16);
        }
    }
    fn post_pkunk_enc(&mut self) {
        let manner: u8;
        if ((self.api.get_segue()) as i32 == 1)
            && (i32::from({
                manner = self.api.get("PKUNK_MANNER");
                manner
            }) != 2)
        {
            self.api.set("PKUNK_MANNER", 1_u8);
            if i32::from(manner) != 1 {
                self.api.set("PKUNK_VISITS", 0_u8);
                self.api.set("PKUNK_HOME_VISITS", 0_u8);
            }
        }
    }
    fn initialize(&mut self) {
        if (i32::from(self.api.get("PKUNK_MANNER")) == 3)
            || (i32::from((self.api.activity()) as u8) == 5)
        {
            self.api.segue(0_u32);
        } else {
            self.api.segue(1_u32);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.pkunk.graphics".as_ptr(),
        alien_font_res: c"comm.pkunk.font".as_ptr(),
        alien_colormap_res: c"comm.pkunk.colortable".as_ptr(),
        alien_song_res: c"comm.pkunk.music".as_ptr(),
        conversation_phrases_res: c"comm.pkunk.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 121, y: 0 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 3,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(3, 4, 10, (28, 0), (840, 2520));
    d.alien_ambient_array[1] = {
        let mut a = animation(7, 4, 12, (84, 0), (840, 2520));
        a.block_mask = 4;
        a
    };
    d.alien_ambient_array[2] = {
        let mut a = animation(11, 3, 12, (28, 0), (840, 2520));
        a.block_mask = 2;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 2, 0, (56, 140), (70, 420));
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
    use super::*;
    use crate::comm::races::script::live::LiveScript;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    extern "C" fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn pkunk_angry(r: u32) {
        run(|s| {
            s.handle(
                Handler::PkunkAngry,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn discuss_conquer(r: u32) {
        run(|s| {
            s.handle(
                Handler::DiscussConquer,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn offer_alliance(r: u32) {
        run(|s| {
            s.handle(
                Handler::OfferAlliance,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn about_pkunk(r: u32) {
        run(|s| {
            s.handle(
                Handler::AboutPkunk,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn about_ilwrath(r: u32) {
        run(|s| {
            s.handle(
                Handler::AboutIlwrath,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn pkunk_home(r: u32) {
        run(|s| {
            s.handle(
                Handler::PkunkHome,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn pkunk_friendly_space(r: u32) {
        run(|s| {
            s.handle(
                Handler::PkunkFriendlySpace,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn pkunk_neutral_space(r: u32) {
        run(|s| {
            s.handle(
                Handler::PkunkNeutralSpace,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn pkunk_migrate(r: u32) {
        run(|s| {
            s.handle(
                Handler::PkunkMigrate,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn callback(h: Handler) -> extern "C" fn(u32) {
        match h {
            Handler::ExitConversation => exit_conversation,
            Handler::PkunkAngry => pkunk_angry,
            Handler::DiscussConquer => discuss_conquer,
            Handler::OfferAlliance => offer_alliance,
            Handler::AboutPkunk => about_pkunk,
            Handler::AboutIlwrath => about_ilwrath,
            Handler::PkunkHome => pkunk_home,
            Handler::PkunkFriendlySpace => pkunk_friendly_space,
            Handler::PkunkNeutralSpace => pkunk_neutral_space,
            Handler::PkunkMigrate => pkunk_migrate,
        }
    }
    pub(super) extern "C" fn intro() {
        run(|s| s.intro());
    }
    pub(super) extern "C" fn post() {
        run(|s| s.post_pkunk_enc());
    }
    pub(super) extern "C" fn uninit() -> u16 {
        0
    }
    /// Initialize this encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_pkunk_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize());
        desc
    }
}
pub struct PkunkDialogue;
impl super::RaceDialogue for PkunkDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_pkunk_comm()) }
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
