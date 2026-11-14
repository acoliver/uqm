//! Orz dialogue, ported from orzc.c.
pub(crate) use super::script::{GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    ExitConversation,
    TaaloWorld,
    OrzAllied,
    OfferAlliance,
    OrzNeutral,
    OrzAngry,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
pub(crate) mod api;
pub(crate) use api::OrzApi;
impl<A: OrzApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::TaaloWorld => self.taalo_world(r),
            Handler::OrzAllied => self.orz_allied(r),
            Handler::OfferAlliance => self.offer_alliance(r),
            Handler::OrzNeutral => self.orz_neutral(r),
            Handler::OrzAngry => self.orz_angry(r),
        }
    }
    pub(crate) fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0_u32);
        if r == BYE_ALLY {
            self.api.npc(i32::from(GOODBYE_ALLY));
        } else if r == BYE_NEUTRAL {
            self.api.npc(i32::from(GOODBYE_NEUTRAL));
        } else if r == BYE_ANGRY {
            self.api.npc(i32::from(GOODBYE_ANGRY));
        } else if r == BYE_TAALO {
            if i32::from(self.api.get(StoryKey::ORZ_MANNER)) == 1 {
                self.api.npc(i32::from(ANGRY_TAALO_GOODBYE));
            } else {
                self.api.npc(i32::from(FRIENDLY_TAALO_GOODBYE));
            }
        } else if r == HOSTILE_2 {
            self.api.npc(i32::from(HOSTILITY_IS_BAD_2));
            self.api.segue(1_u32);
        } else if r == MAY_WE_LAND {
            self.api.npc(i32::from(SURE_LAND));
            self.api.set(StoryKey::TAALO_UNPROTECTED, 1_u8);
        } else if (r == YES_ALLIANCE) || (r == WERE_SORRY) {
            if r == YES_ALLIANCE {
                self.api.npc(i32::from(GREAT));
            } else {
                self.api.npc(i32::from(APOLOGY_ACCEPTED));
            }
            self.api.set(StoryKey::ORZ_ANDRO_STATE, 0_u8);
            self.api.set(StoryKey::ORZ_GENERAL_INFO, 0_u8);
            self.api.set(StoryKey::ORZ_PERSONAL_INFO, 0_u8);
            self.api.set(StoryKey::ORZ_MANNER, 3_u8);
            self.api
                .set_allied((crate::engine::species::ship::ORZ_SHIP) as u16, true);
        } else if r == DEMAND_TO_LAND {
            self.api.npc(i32::from(NO_DEMAND));
            self.api.segue(1_u32);
        } else if (r == ABOUT_ANDRO_3) || (r == MUST_KNOW_ABOUT_ANDROSYN) {
            if r == ABOUT_ANDRO_3 {
                self.api.npc(i32::from(BLEW_IT));
            } else {
                self.api.npc(i32::from(KNOW_TOO_MUCH));
            }
            self.api.set(StoryKey::ORZ_VISITS, 0_u8);
            self.api.set(StoryKey::ORZ_MANNER, 2_u8);
            self.api.segue(1_u32);
            if r == ABOUT_ANDRO_3 {
                self.api
                    .set_allied((crate::engine::species::ship::ORZ_SHIP) as u16, false);
                self.api
                    .remove_escorts((crate::engine::species::ship::ORZ_SHIP) as u16);
            }
            self.api.fade_palette(1, 420);
        } else {
            let mut num_visits: u8;
            num_visits = self.api.get(StoryKey::ORZ_PERSONAL_INFO);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INSULTED_1));
                }
                1 => {
                    self.api.npc(i32::from(INSULTED_2));
                }
                2 => {
                    self.api.npc(i32::from(INSULTED_3));
                    self.api.segue(1_u32);
                }
                7 => {
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                    self.api.npc(i32::from(INSULTED_4));
                    self.api.segue(1_u32);
                }
                _ => {
                    self.api.npc(i32::from(INSULTED_4));
                    self.api.segue(1_u32);
                }
            }
            self.api.set(StoryKey::ORZ_PERSONAL_INFO, num_visits);
        }
    }
    pub(crate) fn taalo_world(&mut self, r: u16) {
        let manner: u8 = self.api.get(StoryKey::ORZ_MANNER);
        if r == DEMAND_TO_LAND {
            self.api.npc(i32::from(ASK_NICELY));
            self.api.disable(DEMAND_TO_LAND);
        } else if r == WHY_YOU_HERE {
            if i32::from(manner) != 1 {
                self.api.npc(i32::from(FRIENDLY_EXPLANATION));
            } else {
                self.api.npc(i32::from(ANGRY_EXPLANATION));
            }
            self.api.disable(WHY_YOU_HERE);
        } else if r == WHAT_IS_THIS_PLACE {
            if i32::from(manner) != 1 {
                self.api.npc(i32::from(FRIENDLY_PLACE));
            } else {
                self.api.npc(i32::from(ANGRY_PLACE));
            }
            self.api.disable(WHAT_IS_THIS_PLACE);
        } else if r == MAY_WE_LAND {
            self.api.npc(i32::from(ALLIES_CAN_VISIT));
            self.api.disable(MAY_WE_LAND);
        } else if r == MAKE_ALLIANCE {
            self.api.npc(i32::from(CANT_ALLY_HERE));
            self.api.disable(MAKE_ALLIANCE);
        } else if r == WHY_BUSY {
            self.api.npc(i32::from(BUSY_BECAUSE));
            self.api.disable(WHY_BUSY);
        }
        if self.api.enabled(MAY_WE_LAND) {
            if (i32::from(manner) == 3)
                && (i32::from(
                    self.api
                        .alliance((crate::engine::species::ship::ORZ_SHIP) as u16),
                ) == 1)
            {
                self.api
                    .response(MAY_WE_LAND, Handler::ExitConversation, &[]);
            } else {
                self.api.response(MAY_WE_LAND, Handler::TaaloWorld, &[]);
            }
        } else if self.api.enabled(MAKE_ALLIANCE) {
            self.api.response(MAKE_ALLIANCE, Handler::TaaloWorld, &[]);
        } else if self.api.enabled(WHY_BUSY) {
            self.api.response(WHY_BUSY, Handler::TaaloWorld, &[]);
        }
        if self.api.enabled(DEMAND_TO_LAND) {
            if i32::from(manner) == 1 {
                self.api
                    .response(DEMAND_TO_LAND, Handler::ExitConversation, &[]);
            } else {
                self.api.response(DEMAND_TO_LAND, Handler::TaaloWorld, &[]);
            }
        }
        if self.api.enabled(WHY_YOU_HERE) {
            self.api.response(WHY_YOU_HERE, Handler::TaaloWorld, &[]);
        }
        if self.api.enabled(WHAT_IS_THIS_PLACE) {
            self.api
                .response(WHAT_IS_THIS_PLACE, Handler::TaaloWorld, &[]);
        }
        self.api.response(BYE_TAALO, Handler::ExitConversation, &[]);
    }
    pub(crate) fn orz_allied(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_ALLY {
            num_visits = self.api.get(StoryKey::ORZ_GENERAL_INFO);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_ALLY_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_ALLY_2));
                }
                2 => {
                    self.api.npc(i32::from(GENERAL_INFO_ALLY_3));
                }
                3 => {
                    self.api.npc(i32::from(GENERAL_INFO_ALLY_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::ORZ_GENERAL_INFO, num_visits);
            self.api.disable(WHATS_UP_ALLY);
        } else if r == MORE_ABOUT_YOU {
            num_visits = self.api.get(StoryKey::ORZ_PERSONAL_INFO);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(ABOUT_US_1));
                }
                1 => {
                    self.api.npc(i32::from(ABOUT_US_2));
                }
                2 => {
                    self.api.npc(i32::from(ABOUT_US_3));
                }
                3 => {
                    self.api.npc(i32::from(ABOUT_US_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::ORZ_PERSONAL_INFO, num_visits);
            self.api.disable(MORE_ABOUT_YOU);
        } else if r == ABOUT_ANDRO_1 {
            self.api.npc(i32::from(FORGET_ANDRO_1));
            self.api.set(StoryKey::ORZ_ANDRO_STATE, 1_u8);
        } else if r == ABOUT_ANDRO_2 {
            self.api.npc(i32::from(FORGET_ANDRO_2));
            self.api.set(StoryKey::ORZ_ANDRO_STATE, 2_u8);
        }
        if i32::from(self.api.get(StoryKey::ORZ_ANDRO_STATE)) == 0 {
            self.api.response(ABOUT_ANDRO_1, Handler::OrzAllied, &[]);
        } else if i32::from(self.api.get(StoryKey::ORZ_ANDRO_STATE)) == 1 {
            self.api.response(ABOUT_ANDRO_2, Handler::OrzAllied, &[]);
        } else {
            self.api
                .response(ABOUT_ANDRO_3, Handler::ExitConversation, &[]);
        }
        if self.api.enabled(WHATS_UP_ALLY) {
            self.api.response(WHATS_UP_ALLY, Handler::OrzAllied, &[]);
        }
        if self.api.enabled(MORE_ABOUT_YOU) {
            self.api.response(MORE_ABOUT_YOU, Handler::OrzAllied, &[]);
        }
        self.api.response(BYE_ALLY, Handler::ExitConversation, &[]);
    }
    pub(crate) fn where_androsyn(&mut self, _r: u16) {
        self.api.npc(i32::from(DISEMBLE_ABOUT_ANDROSYN));
        self.api.disable(WHERE_ANDROSYN);
        self.api
            .response(MUST_KNOW_ABOUT_ANDROSYN, Handler::ExitConversation, &[]);
        self.api
            .response(DONT_REALLY_CARE, Handler::OrzNeutral, &[]);
    }
    pub(crate) fn offer_alliance(&mut self, r: u16) {
        if r == SEEM_LIKE_NICE_GUYS {
            self.api.npc(i32::from(ARE_NICE_WANT_ALLY));
        } else if r == TALK_ABOUT_ALLIANCE {
            self.api.npc(i32::from(OK_TALK_ALLIANCE));
        } else if r == WHY_SO_TRUSTING {
            self.api.npc(i32::from(TRUSTING_BECAUSE));
            self.api.set(StoryKey::ORZ_STACK1, 1_u8);
        }
        self.api.response(NO_ALLIANCE, Handler::OrzNeutral, &[]);
        self.api.response(DECIDE_LATER, Handler::OrzNeutral, &[]);
        if i32::from(self.api.get(StoryKey::ORZ_STACK1)) == 0 {
            self.api
                .response(WHY_SO_TRUSTING, Handler::OfferAlliance, &[]);
        }
        self.api
            .response(YES_ALLIANCE, Handler::ExitConversation, &[]);
    }
    pub(crate) fn orz_neutral(&mut self, r: u16) {
        let mut fragments: Vec<i32> = Vec::new();
        let mut last_stack: u8;
        let mut p_str: [u16; 3] = [0; 3];
        last_stack = 0_u8;
        if r == HOSTILE_1 {
            self.api.npc(i32::from(HOSTILITY_IS_BAD_1));
            self.api.disable(HOSTILE_1);
            last_stack = 2_u8;
        } else if r == WE_ARE_VINDICATOR0 {
            self.api.npc(i32::from(NICE_TO_MEET_YOU));
            self.api.set(StoryKey::ORZ_STACK0, 1_u8);
            last_stack = 1_u8;
        } else if r == WHO_YOU {
            self.api.npc(i32::from(WE_ARE_ORZ));
            self.api.set(StoryKey::ORZ_ANDRO_STATE, 1_u8);
        } else if r == WHY_HERE {
            self.api.npc(i32::from(HERE_BECAUSE));
            self.api.set(StoryKey::ORZ_ANDRO_STATE, 2_u8);
        } else if r == NO_ALLIANCE {
            self.api.npc(i32::from(MAYBE_LATER));
            self.api.disable(TALK_ABOUT_ALLIANCE);
            self.api.set(StoryKey::REFUSED_ORZ_ALLIANCE, 1_u8);
        } else if r == DECIDE_LATER {
            self.api.npc(i32::from(OK_LATER));
            self.api.disable(TALK_ABOUT_ALLIANCE);
            self.api.set(StoryKey::REFUSED_ORZ_ALLIANCE, 1_u8);
        } else if r == DONT_REALLY_CARE {
            self.api.npc(i32::from(YOU_ARE_OUR_FRIENDS));
        } else if r == WHERE_ANDROSYN {
            self.where_androsyn(r);
            return;
        } else if (r == TALK_ABOUT_ALLIANCE) || (r == SEEM_LIKE_NICE_GUYS) {
            self.offer_alliance(r);
            return;
        } else if r == HOSTILE_2 {
            self.exit_conversation(r);
            return;
        }
        if i32::from(self.api.get(StoryKey::ORZ_ANDRO_STATE)) == 0 {
            p_str[0_usize] = WHO_YOU;
        } else if i32::from(self.api.get(StoryKey::ORZ_ANDRO_STATE)) == 1 {
            p_str[0_usize] = WHY_HERE;
        } else if self.api.enabled(WHERE_ANDROSYN)
            && (i32::from(self.api.get(StoryKey::ORZ_ANDRO_STATE)) == 2)
        {
            p_str[0_usize] = WHERE_ANDROSYN;
        }
        if self.api.get(StoryKey::REFUSED_ORZ_ALLIANCE) != 0 {
            if self.api.enabled(TALK_ABOUT_ALLIANCE) {
                p_str[1_usize] = TALK_ABOUT_ALLIANCE;
            }
        } else if i32::from(self.api.get(StoryKey::ORZ_STACK0)) == 0 {
            fragments = vec![
                i32::from(WE_ARE_VINDICATOR0),
                GLOBAL_PLAYER_NAME,
                i32::from(WE_ARE_VINDICATOR1),
                GLOBAL_SHIP_NAME,
                i32::from(WE_ARE_VINDICATOR2),
            ];
            p_str[1_usize] = WE_ARE_VINDICATOR0;
        } else {
            p_str[1_usize] = SEEM_LIKE_NICE_GUYS;
        }
        if self.api.enabled(HOSTILE_1) {
            p_str[2_usize] = HOSTILE_1;
        } else {
            p_str[2_usize] = HOSTILE_2;
        }
        if p_str[(last_stack) as usize] != 0 {
            if i32::from(p_str[(last_stack) as usize]) != i32::from(WE_ARE_VINDICATOR0) {
                self.api
                    .response(p_str[(last_stack) as usize], Handler::OrzNeutral, &[]);
            } else {
                self.api.response(
                    p_str[(last_stack) as usize],
                    Handler::OrzNeutral,
                    &fragments,
                );
            }
        }
        for i in 0..3 {
            if (i != i32::from(last_stack)) && (p_str[(i) as usize] != 0) {
                if i32::from(p_str[(i) as usize]) != i32::from(WE_ARE_VINDICATOR0) {
                    self.api
                        .response(p_str[(i) as usize], Handler::OrzNeutral, &[]);
                } else {
                    self.api
                        .response(p_str[(i) as usize], Handler::OrzNeutral, &fragments);
                }
            }
        }
        self.api
            .response(BYE_NEUTRAL, Handler::ExitConversation, &[]);
    }
    pub(crate) fn orz_angry(&mut self, r: u16) {
        if r == WHATS_UP_ANGRY {
            let mut num_visits: u8;
            num_visits = self.api.get(StoryKey::ORZ_GENERAL_INFO);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_ANGRY_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_ANGRY_2));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::ORZ_GENERAL_INFO, num_visits);
            self.api.disable(WHATS_UP_ANGRY);
        }
        if self.api.enabled(WHATS_UP_ANGRY) {
            self.api.response(WHATS_UP_ANGRY, Handler::OrzAngry, &[]);
        }
        self.api
            .response(WERE_SORRY, Handler::ExitConversation, &[]);
        match self.api.get(StoryKey::ORZ_PERSONAL_INFO) {
            0 => {
                self.api.response(INSULT_1, Handler::ExitConversation, &[]);
            }
            1 => {
                self.api.response(INSULT_2, Handler::ExitConversation, &[]);
            }
            2 => {
                self.api.response(INSULT_3, Handler::ExitConversation, &[]);
            }
            3 => {
                self.api.response(INSULT_4, Handler::ExitConversation, &[]);
            }
            4 => {
                self.api.response(INSULT_5, Handler::ExitConversation, &[]);
            }
            5 => {
                self.api.response(INSULT_6, Handler::ExitConversation, &[]);
            }
            6 => {
                self.api.response(INSULT_7, Handler::ExitConversation, &[]);
            }
            7 => {
                self.api.response(INSULT_8, Handler::ExitConversation, &[]);
            }
            _ => {}
        }
        self.api.response(BYE_ANGRY, Handler::ExitConversation, &[]);
    }
    pub(crate) fn intro(&mut self) {
        let mut num_visits: u8;
        if i32::from((self.api.activity()) as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0_u32);
            return;
        }
        if !(self.api.get(StoryKey::MET_ORZ_BEFORE) != 0) {
            self.api.npc(i32::from(INIT_HELLO));
        }
        let manner: u8 = self.api.get(StoryKey::ORZ_MANNER);
        if i32::from(manner) == 2 {
            self.api.palette(1);
            num_visits = self.api.get(StoryKey::ORZ_VISITS);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HOSTILE_HELLO_1));
                }
                1 => {
                    self.api.npc(i32::from(HOSTILE_HELLO_2));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::ORZ_VISITS, num_visits);
            self.api.segue(1_u32);
        } else if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 6)) != 0 {
            num_visits = self.api.get(StoryKey::TAALO_VISITS);
            if i32::from(manner) != 1 {
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(FRIENDLY_ALLIED_TAALO_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(FRIENDLY_ALLIED_TAALO_HELLO_2));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
            } else {
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(ANGRY_TAALO_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(ANGRY_TAALO_HELLO_2));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
            }
            self.api.set(StoryKey::TAALO_VISITS, num_visits);
            self.taalo_world(0_u16);
        } else if (i32::from(manner) == 3)
            && (i32::from(
                self.api
                    .alliance((crate::engine::species::ship::ORZ_SHIP) as u16),
            ) == 1)
        {
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::ORZ_HOME_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(ALLIED_HOMEWORLD_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(ALLIED_HOMEWORLD_HELLO_2));
                    }
                    2 => {
                        self.api.npc(i32::from(ALLIED_HOMEWORLD_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(ALLIED_HOMEWORLD_HELLO_4));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::ORZ_HOME_VISITS, num_visits);
            } else {
                num_visits = self.api.get(StoryKey::ORZ_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(ALLIED_SPACE_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(ALLIED_SPACE_HELLO_2));
                    }
                    2 => {
                        self.api.npc(i32::from(ALLIED_SPACE_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(ALLIED_SPACE_HELLO_4));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::ORZ_VISITS, num_visits);
            }
            self.orz_allied(0_u16);
        } else if i32::from(manner) != 1 {
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::ORZ_HOME_VISITS);
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
                self.api.set(StoryKey::ORZ_HOME_VISITS, num_visits);
            } else {
                num_visits = self.api.get(StoryKey::ORZ_VISITS);
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
                self.api.set(StoryKey::ORZ_VISITS, num_visits);
            }
            self.orz_neutral(0_u16);
        } else {
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::ORZ_HOME_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(ANGRY_HOMEWORLD_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(ANGRY_HOMEWORLD_HELLO_2));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::ORZ_HOME_VISITS, num_visits);
            } else {
                num_visits = self.api.get(StoryKey::ORZ_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(ANGRY_SPACE_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(ANGRY_SPACE_HELLO_2));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::ORZ_VISITS, num_visits);
            }
            self.orz_angry(0_u16);
        }
        if !(self.api.get(StoryKey::MET_ORZ_BEFORE) != 0) {
            self.api.set(StoryKey::MET_ORZ_BEFORE, 1_u8);
            self.api.talking(false);
            self.api.talk_segue(1_u16);
            self.api.talking(true);
        }
    }
    pub(crate) fn post_orz_enc(&mut self) {
        let manner: u8;
        if ((self.api.get_segue()) as i32 == 1)
            && (i32::from({
                manner = self.api.get(StoryKey::ORZ_MANNER);
                manner
            }) != 2)
        {
            self.api.set(StoryKey::ORZ_MANNER, 1_u8);
            if i32::from(manner) != 1 {
                self.api.set(StoryKey::ORZ_VISITS, 0_u8);
                self.api.set(StoryKey::ORZ_HOME_VISITS, 0_u8);
                self.api.set(StoryKey::TAALO_VISITS, 0_u8);
            }
        }
    }
    pub(crate) fn initialize(&mut self) {
        if (i32::from(self.api.get(StoryKey::ORZ_MANNER)) == 3)
            || (i32::from((self.api.activity()) as u8) == 5)
        {
            self.api.segue(0_u32);
        } else {
            self.api.segue(1_u32);
        }
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
