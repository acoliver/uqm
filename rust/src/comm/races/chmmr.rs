//! Chmmr dialogue, ported from chmmrc.c.
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
pub(crate) mod api;
pub(crate) use api::ChmmrApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    ExitConversation,
    NotReady,
    ImproveBomb,
    ChmmrFree,
    ChmmrAdvice,
    ChmmrShielded,
    AfterBomb,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: ChmmrApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::NotReady => self.not_ready(r),
            Handler::ImproveBomb => self.improve_bomb(r),
            Handler::ChmmrFree => self.chmmr_free(r),
            Handler::ChmmrAdvice => self.chmmr_advice(r),
            Handler::ChmmrShielded => self.chmmr_shielded(r),
            Handler::AfterBomb => self.after_bomb(r),
        }
    }
    pub(crate) fn exit_conversation(&mut self, r: i32) {
        self.api.segue(0);
        if r == BYE {
            self.api.npc(GOODBYE);
        } else if r == BYE_SHIELDED {
            self.api.npc(GOODBYE_SHIELDED);
        } else if r == BYE_AFTER_BOMB {
            self.api.npc(GOODBYE_AFTER_BOMB);
        } else if r == PROCEED {
            self.api.npc(TAKE_2_WEEKS);
            self.api.set_allied(
                (crate::engine::species::ship::CHMMR_SHIP as i32) as u16,
                1 != 0,
            );
            self.api.set(StoryKey::CHMMR_HOME_VISITS, 0_u8);
            self.api.set(StoryKey::CHMMR_STACK, 0_u8);
            self.api.set(StoryKey::CHMMR_BOMB_STATE, 2_u8);
            self.api.set(StoryKey::UTWIG_BOMB_ON_SHIP, 0_u8);
            self.api.refit();
        }
    }
    pub(crate) fn not_ready(&mut self, r: i32) {
        if r == 0 {
            self.api.npc(RETURN_WHEN_READY);
        } else if r == FURTHER_ASSISTANCE {
            self.api.npc(NO_FURTHER_ASSISTANCE);
            self.api.disable((FURTHER_ASSISTANCE) as u16);
        } else if r == TECH_HELP {
            self.api.npc(USE_OUR_SHIPS_BEFORE);
            self.api.set_allied(
                (crate::engine::species::ship::CHMMR_SHIP as i32) as u16,
                1 != 0,
            );
        } else if r == WHERE_WEAPON {
            self.api.npc(PRECURSOR_WEAPON);
            self.api.disable((WHERE_WEAPON) as u16);
        } else if r == WHERE_DISTRACTION {
            self.api.npc(PSYCHIC_WEAPONRY);
            self.api.disable((WHERE_DISTRACTION) as u16);
        }
        if i32::from(
            self.api
                .alliance((crate::engine::species::ship::CHMMR_SHIP as i32) as u16),
        ) != 1
        {
            self.api
                .response((TECH_HELP) as u16, Handler::NotReady, &[]);
        } else if self.api.enabled((FURTHER_ASSISTANCE) as u16) {
            self.api
                .response((FURTHER_ASSISTANCE) as u16, Handler::NotReady, &[]);
        }
        if self.api.enabled((WHERE_WEAPON) as u16)
            && !(i32::from(self.api.get(StoryKey::UTWIG_BOMB_ON_SHIP)) != 0)
        {
            self.api
                .response((WHERE_WEAPON) as u16, Handler::NotReady, &[]);
        }
        if self.api.enabled((WHERE_DISTRACTION) as u16)
            && !(i32::from(self.api.get(StoryKey::TALKING_PET_ON_SHIP)) != 0)
        {
            self.api
                .response((WHERE_DISTRACTION) as u16, Handler::NotReady, &[]);
        }
        self.api
            .response((BYE) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn improve_bomb(&mut self, r: i32) {
        if r == 0 {
            self.api.npc(WE_WILL_IMPROVE_BOMB);
        } else if r == WHAT_NOW {
            self.api.npc(MODIFY_VESSEL);
            self.api.disable((WHAT_NOW) as u16);
        } else if r == WONT_HURT_MY_SHIP {
            self.api.npc(WILL_DESTROY_IT);
            self.api.disable((WONT_HURT_MY_SHIP) as u16);
        } else if r == BUMMER_ABOUT_MY_SHIP {
            self.api.npc(DEAD_SILENCE);
            self.api.disable((BUMMER_ABOUT_MY_SHIP) as u16);
        } else if r == OTHER_ASSISTANCE {
            self.api.npc(USE_OUR_SHIPS_AFTER);
            self.api.set_allied(
                (crate::engine::species::ship::CHMMR_SHIP as i32) as u16,
                1 != 0,
            );
        }
        if self.api.enabled((WHAT_NOW) as u16) {
            self.api
                .response((WHAT_NOW) as u16, Handler::ImproveBomb, &[]);
        } else if self.api.enabled((WONT_HURT_MY_SHIP) as u16) {
            self.api
                .response((WONT_HURT_MY_SHIP) as u16, Handler::ImproveBomb, &[]);
        } else if self.api.enabled((BUMMER_ABOUT_MY_SHIP) as u16) {
            self.api
                .response((BUMMER_ABOUT_MY_SHIP) as u16, Handler::ImproveBomb, &[]);
        }
        if i32::from(
            self.api
                .alliance((crate::engine::species::ship::CHMMR_SHIP as i32) as u16),
        ) != 1
        {
            self.api
                .response((OTHER_ASSISTANCE) as u16, Handler::ImproveBomb, &[]);
        }
        self.api
            .response((PROCEED) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn chmmr_free(&mut self, r: i32) {
        if (((r == 0) || (r == I_AM_CAPTAIN0)) || (r == I_AM_SAVIOR)) || (r == I_AM_SILLY) {
            self.api.npc(WHY_HAVE_YOU_FREED_US);
            self.api.talk_segue((!0_u16 as i32) as u16);
            self.api.set(StoryKey::CHMMR_EMERGING, 0_u8);
            self.api
                .response((SERIOUS_1) as u16, Handler::ChmmrFree, &[]);
            self.api
                .response((SERIOUS_2) as u16, Handler::ChmmrFree, &[]);
            self.api.response((SILLY) as u16, Handler::ChmmrFree, &[]);
        } else {
            self.api.npc(WILL_HELP_ANALYZE_LOGS);
            if i32::from(self.api.get(StoryKey::AWARE_OF_SAMATRA)) != 0 {
                self.api.npc(YOU_KNOW_SAMATRA);
            } else {
                self.api.npc(DONT_KNOW_ABOUT_SAMATRA);
                self.api.set(StoryKey::AWARE_OF_SAMATRA, 1_u8);
            }
            if i32::from(self.api.get(StoryKey::TALKING_PET_ON_SHIP)) != 0 {
                self.api.npc(HAVE_TALKING_PET);
            } else {
                self.api.npc(NEED_DISTRACTION);
            }
            if i32::from(self.api.get(StoryKey::UTWIG_BOMB_ON_SHIP)) != 0 {
                self.api.npc(HAVE_BOMB);
            } else {
                self.api.npc(NEED_WEAPON);
            }
            if !(i32::from(self.api.get(StoryKey::TALKING_PET_ON_SHIP)) != 0)
                || !(i32::from(self.api.get(StoryKey::UTWIG_BOMB_ON_SHIP)) != 0)
            {
                self.not_ready(0_u16 as i32);
            } else {
                self.improve_bomb(0_u16 as i32);
            }
        }
    }
    pub(crate) fn chmmr_advice(&mut self, r: i32) {
        let mut advice_left: i32;
        if r == NEED_ADVICE {
            self.api.npc(WHAT_ADVICE);
        } else if r == HOW_DEFEAT_URQUAN {
            self.api.npc(DEFEAT_LIKE_SO);
            self.api.set(StoryKey::CHMMR_BOMB_STATE, 1_u8);
            self.api.disable((HOW_DEFEAT_URQUAN) as u16);
        } else if r == WHAT_ABOUT_TPET {
            self.api.npc(SCARY_BUT_USEFUL);
            self.api.disable((WHAT_ABOUT_TPET) as u16);
        } else if r == WHAT_ABOUT_BOMB {
            self.api.npc(ABOUT_BOMB);
            self.api.disable((WHAT_ABOUT_BOMB) as u16);
        } else if r == WHAT_ABOUT_SUN_DEVICE {
            self.api.npc(ABOUT_SUN_DEVICE);
            self.api.disable((WHAT_ABOUT_SUN_DEVICE) as u16);
        } else if r == WHAT_ABOUT_SAMATRA {
            self.api.npc(ABOUT_SAMATRA);
            self.api.disable((WHAT_ABOUT_SAMATRA) as u16);
        }
        advice_left = 0;
        if self.api.enabled((HOW_DEFEAT_URQUAN) as u16) {
            self.api
                .response((HOW_DEFEAT_URQUAN) as u16, Handler::ChmmrAdvice, &[]);
            advice_left = 1;
        }
        if self.api.enabled((WHAT_ABOUT_TPET) as u16)
            && (i32::from(self.api.get(StoryKey::TALKING_PET_ON_SHIP)) != 0)
        {
            self.api
                .response((WHAT_ABOUT_TPET) as u16, Handler::ChmmrAdvice, &[]);
            advice_left = 1;
        }
        if self.api.enabled((WHAT_ABOUT_BOMB) as u16)
            && (i32::from(self.api.get(StoryKey::UTWIG_BOMB_ON_SHIP)) != 0)
        {
            self.api
                .response((WHAT_ABOUT_BOMB) as u16, Handler::ChmmrAdvice, &[]);
            advice_left = 1;
        }
        if self.api.enabled((WHAT_ABOUT_SUN_DEVICE) as u16)
            && (i32::from(self.api.get(StoryKey::SUN_DEVICE_ON_SHIP)) != 0)
        {
            self.api
                .response((WHAT_ABOUT_SUN_DEVICE) as u16, Handler::ChmmrAdvice, &[]);
            advice_left = 1;
        }
        if self.api.enabled((WHAT_ABOUT_SAMATRA) as u16)
            && (i32::from(self.api.get(StoryKey::AWARE_OF_SAMATRA)) != 0)
        {
            self.api
                .response((WHAT_ABOUT_SAMATRA) as u16, Handler::ChmmrAdvice, &[]);
            advice_left = 1;
        }
        self.api
            .response((ENOUGH_ADVICE) as u16, Handler::ChmmrShielded, &[]);
        if !(advice_left != 0) {
            self.api.disable((NEED_ADVICE) as u16);
        }
    }
    pub(crate) fn chmmr_shielded(&mut self, r: i32) {
        if r == FIND_OUT_WHATS_UP {
            self.api.npc(HYBRID_PROCESS);
            self.api.disable((FIND_OUT_WHATS_UP) as u16);
        } else if r == NEED_HELP {
            self.api.npc(CANT_HELP);
            self.api.set(StoryKey::CHMMR_STACK, 1_u8);
        } else if r == WHY_NO_HELP {
            self.api.npc(LONG_TIME);
            self.api.set(StoryKey::CHMMR_STACK, 2_u8);
        } else if r == WHAT_IF_MORE_ENERGY {
            self.api.npc(DANGER_TO_US);
            self.api.set(StoryKey::CHMMR_STACK, 3_u8);
        } else if r == ENOUGH_ADVICE {
            self.api.npc(OK_ENOUGH_ADVICE);
        }
        match i32::from(self.api.get(StoryKey::CHMMR_STACK)) {
            0 => {
                self.api
                    .response((NEED_HELP) as u16, Handler::ChmmrShielded, &[]);
            }
            1 => {
                self.api
                    .response((WHY_NO_HELP) as u16, Handler::ChmmrShielded, &[]);
            }
            2 => {
                self.api
                    .response((WHAT_IF_MORE_ENERGY) as u16, Handler::ChmmrShielded, &[]);
            }
            _ => {}
        }
        if self.api.enabled((FIND_OUT_WHATS_UP) as u16) {
            self.api
                .response((FIND_OUT_WHATS_UP) as u16, Handler::ChmmrShielded, &[]);
        }
        if self.api.enabled((NEED_ADVICE) as u16) {
            self.api
                .response((NEED_ADVICE) as u16, Handler::ChmmrAdvice, &[]);
        }
        self.api
            .response((BYE_SHIELDED) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn after_bomb(&mut self, r: i32) {
        if r == WHATS_UP_AFTER_BOMB {
            if i32::from(self.api.get(StoryKey::CHMMR_STACK)) != 0 {
                self.api.npc(GENERAL_INFO_AFTER_BOMB_2);
            } else {
                self.api.npc(GENERAL_INFO_AFTER_BOMB_1);
                self.api.set(StoryKey::CHMMR_STACK, 1_u8);
            }
            self.api.disable((WHATS_UP_AFTER_BOMB) as u16);
        } else if r == WHAT_DO_AFTER_BOMB {
            self.api.npc(DO_AFTER_BOMB);
            self.api.disable((WHAT_DO_AFTER_BOMB) as u16);
        }
        if self.api.enabled((WHATS_UP_AFTER_BOMB) as u16) {
            self.api
                .response((WHATS_UP_AFTER_BOMB) as u16, Handler::AfterBomb, &[]);
        }
        if self.api.enabled((WHAT_DO_AFTER_BOMB) as u16) {
            self.api
                .response((WHAT_DO_AFTER_BOMB) as u16, Handler::AfterBomb, &[]);
        }
        self.api
            .response((BYE_AFTER_BOMB) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn intro(&mut self) {
        let mut visits: i32;
        if i32::from(self.api.get(StoryKey::CHMMR_BOMB_STATE)) >= 2 {
            visits = i32::from(self.api.get(StoryKey::CHMMR_HOME_VISITS));
            let res = {
                let old = visits;
                visits = (visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_AFTER_BOMB_1);
                }
                1 => {
                    self.api.npc(HELLO_AFTER_BOMB_2);
                    {
                        visits = (visits - 1) as u8 as i32;
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::CHMMR_HOME_VISITS, visits as u8);
            self.after_bomb(0_u16 as i32);
        } else if self.api.get(StoryKey::CHMMR_UNLEASHED) != 0 {
            if !(self.api.get(StoryKey::TALKING_PET_ON_SHIP) != 0)
                || !(self.api.get(StoryKey::UTWIG_BOMB_ON_SHIP) != 0)
            {
                self.not_ready(0_u16 as i32);
            } else {
                self.api.npc(YOU_ARE_READY);
                self.improve_bomb(0_u16 as i32);
            }
        } else {
            visits = i32::from(self.api.get(StoryKey::CHMMR_HOME_VISITS));
            if !(self.api.get(StoryKey::CHMMR_EMERGING) != 0) {
                self.api.palette(1);
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(WHY_YOU_HERE_1);
                    }
                    1 => {
                        self.api.npc(WHY_YOU_HERE_2);
                    }
                    2 => {
                        self.api.npc(WHY_YOU_HERE_3);
                    }
                    3 => {
                        self.api.npc(WHY_YOU_HERE_4);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.chmmr_shielded(0_u16 as i32);
            } else {
                self.api.intro_mode(3, 840 * 2);
                self.api.npc(WE_ARE_FREE);
                if visits != 0 {
                    self.chmmr_free(0_u16 as i32);
                    visits = 0;
                } else {
                    self.api.npc(WHO_ARE_YOU);

                    self.api.response(
                        (I_AM_CAPTAIN0) as u16,
                        Handler::ChmmrFree,
                        &[
                            I_AM_CAPTAIN0,
                            super::script::GLOBAL_PLAYER_NAME,
                            I_AM_CAPTAIN1,
                            super::script::GLOBAL_SHIP_NAME,
                            I_AM_CAPTAIN2,
                        ],
                    );
                    self.api
                        .response((I_AM_SAVIOR) as u16, Handler::ChmmrFree, &[]);
                    self.api
                        .response((I_AM_SILLY) as u16, Handler::ChmmrFree, &[]);
                }
                self.api.set(StoryKey::CHMMR_UNLEASHED, 1_u8);
            }
            self.api.set(StoryKey::CHMMR_HOME_VISITS, visits as u8);
        }
    }
    pub(crate) fn post_chmmr_enc(&mut self) {}
    pub(crate) fn initialize(&mut self) {
        self.api.segue(0);
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
