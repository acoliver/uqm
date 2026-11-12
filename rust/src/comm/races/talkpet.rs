//! Talkpet dialogue, ported from talkpet.c.
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
pub(crate) mod api;
pub(crate) use api::TalkpetApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    ExitConversation,
    MindFuckUrquan,
    MindControl,
    PetInfo,
    PetDevice,
    CompelPlayer,
    KillPet,
    PetDeal,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: TalkpetApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::MindFuckUrquan => self.mind_fuck_urquan(r),
            Handler::MindControl => self.mind_control(r),
            Handler::PetInfo => self.pet_info(r),
            Handler::PetDevice => self.pet_device(r),
            Handler::CompelPlayer => self.compel_player(r),
            Handler::KillPet => self.kill_pet(r),
            Handler::PetDeal => self.pet_deal(r),
        }
    }
    pub(crate) fn exit_conversation(&mut self, r: i32) {
        self.api.segue(0);
        self.api.set(StoryKey::SHIP_TO_COMPEL, 0_u8);
        if r == COMPEL_URQUAN {
            self.api.npc(HERE_WE_GO);
            self.api.set(StoryKey::URQUAN_MESSED_UP, 1_u8);
        } else if r == WISH_TO_GO_NOW {
            self.api.npc(EXCELLENT_IDEA);
        } else if r == BYE_ONBOARD {
            self.api.npc(GOODBYE_ONBOARD);
        } else if r == COMPEL_THAT_SHIP {
            self.api.npc(SAVING_MY_POWER);
        } else if (((r == OK_LETS_DO_IT) || (r == WANT_KILL_1)) || (r == WANT_KILL_2))
            || (r == WANT_KILL_3)
        {
            if r == OK_LETS_DO_IT {
                self.api.npc(COMING_ABOARD);
            } else {
                self.api.npc(GLAD_YOU_WONT_KILL);
            }
            self.api.set(StoryKey::TALKING_PET, 1_u8);
            self.api.set(StoryKey::TALKING_PET_ON_SHIP, 1_u8);
            self.api.set(StoryKey::UMGAH_ZOMBIE_BLOBBIES, 0_u8);
            self.api.set(StoryKey::UMGAH_VISITS, 0_u8);
            self.api.set(StoryKey::UMGAH_HOME_VISITS, 0_u8);
            self.api.set(StoryKey::ARILOU_STACK_2, 0_u8);
        }
    }
    pub(crate) fn mind_fuck_urquan(&mut self, r: i32) {
        if r == WHAT_NOW {
            self.api.npc(DO_THIS);
            self.api.disable((WHAT_NOW) as u16);
        } else if r == IM_SCARED {
            self.api.npc(STUPID_FOP);
            self.api.disable((IM_SCARED) as u16);
        }
        if self.api.enabled((WHAT_NOW) as u16) {
            self.api
                .response((WHAT_NOW) as u16, Handler::MindFuckUrquan, &[]);
        }
        if self.api.enabled((IM_SCARED) as u16) {
            self.api
                .response((IM_SCARED) as u16, Handler::MindFuckUrquan, &[]);
        }
        self.api
            .response((COMPEL_URQUAN) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn mind_control_strobe(&mut self) {
        self.api.strobe();
    }
    pub(crate) fn mind_control(&mut self, mut r: i32) {
        let resp_func: Handler;
        if r == WHAT_ABOUT_POWERS {
            self.api.npc(NOT_POWERS_BUT_FLOWERS);
            resp_func = Handler::MindControl;
            r = YES_FLOWERS;
        } else {
            self.api.npc(GOOD_HUMAN);
            resp_func = Handler::ExitConversation;
            r = WISH_TO_GO_NOW;
        }
        self.api.talk_segue((!0_u16 as i32) as u16);
        self.mind_control_strobe();
        self.api.response((r) as u16, resp_func, &[]);
    }
    pub(crate) fn pet_info(&mut self, r: i32) {
        if r == ABOUT_YOUR_RACE {
            self.api.npc(WHAT_ABOUT_RACE);
        } else if r == WHAT_ABOUT_PHYSIOLOGY {
            self.api.npc(NO_TALK_ABOUT_SELF);
            self.api.disable((WHAT_ABOUT_PHYSIOLOGY) as u16);
        } else if r == WHAT_ABOUT_YOUR_HISTORY {
            self.api.npc(ABOUT_HISTORY);
            self.api.disable((WHAT_ABOUT_YOUR_HISTORY) as u16);
        } else if r == SENTIENT_MILIEU {
            self.api.npc(ABOUT_SENTIENT_MILIEU);
            self.api.disable((SENTIENT_MILIEU) as u16);
        } else if r == WHAT_ABOUT_WAR {
            self.api.npc(ABOUT_WAR);
            self.api.disable((WHAT_ABOUT_WAR) as u16);
        }
        if self.api.enabled((WHAT_ABOUT_PHYSIOLOGY) as u16) {
            self.api
                .response((WHAT_ABOUT_PHYSIOLOGY) as u16, Handler::PetInfo, &[]);
        } else {
            self.api
                .response((WHAT_ABOUT_POWERS) as u16, Handler::MindControl, &[]);
        }
        if self.api.enabled((WHAT_ABOUT_YOUR_HISTORY) as u16) {
            self.api
                .response((WHAT_ABOUT_YOUR_HISTORY) as u16, Handler::PetInfo, &[]);
        } else if self.api.enabled((SENTIENT_MILIEU) as u16) {
            self.api
                .response((SENTIENT_MILIEU) as u16, Handler::PetInfo, &[]);
        } else if self.api.enabled((WHAT_ABOUT_WAR) as u16) {
            self.api
                .response((WHAT_ABOUT_WAR) as u16, Handler::PetInfo, &[]);
        }
        self.api
            .response((ENOUGH_INFO) as u16, Handler::PetDevice, &[]);
    }
    pub(crate) fn pet_device(&mut self, r: i32) {
        let mut visits: i32;
        if r == WHATS_UP_ONBOARD {
            visits = i32::from(self.api.get(StoryKey::TALKING_PET_INFO));
            let res = {
                let old = visits;
                visits = (visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => self.api.npc(GENERAL_INFO_ONBOARD_1),
                1 => self.api.npc(GENERAL_INFO_ONBOARD_2),
                2 => self.api.npc(GENERAL_INFO_ONBOARD_3),
                3 => self.api.npc(GENERAL_INFO_ONBOARD_4),
                4 => self.api.npc(GENERAL_INFO_ONBOARD_5),
                5 => self.api.npc(GENERAL_INFO_ONBOARD_6),
                6 => self.api.npc(GENERAL_INFO_ONBOARD_7),
                7 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_8);
                    {
                        visits = (visits - 1) as u8 as i32;
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::TALKING_PET_INFO, visits as u8);
            self.api.disable((WHATS_UP_ONBOARD) as u16);
        } else if r == ANY_SUGGESTIONS {
            visits = i32::from(self.api.get(StoryKey::TALKING_PET_SUGGESTIONS));
            let res = {
                let old = visits;
                visits = (visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => self.api.npc(SUGGESTION_1),
                1 => self.api.npc(SUGGESTION_2),
                2 => self.api.npc(SUGGESTION_3),
                3 => self.api.npc(SUGGESTION_4),
                4 => self.api.npc(SUGGESTION_5),
                5 => self.api.npc(SUGGESTION_6),
                6 => self.api.npc(SUGGESTION_7),
                7 => {
                    self.api.npc(SUGGESTION_8);
                    {
                        visits = (visits - 1) as u8 as i32;
                        visits
                    };
                }
                _ => {}
            }
            self.api
                .set(StoryKey::TALKING_PET_SUGGESTIONS, visits as u8);
            self.api.disable((ANY_SUGGESTIONS) as u16);
        } else if r == ENOUGH_INFO {
            self.api.npc(OK_ENOUGH_INFO);
        } else if r == YOU_LIED {
            self.api.npc(SO_WHAT);
            self.api.set(StoryKey::DNYARRI_LIED, 0_u8);
        }
        if self.api.get(StoryKey::SHIP_TO_COMPEL) != 0 {
            self.api
                .response((COMPEL_THAT_SHIP) as u16, Handler::ExitConversation, &[]);
        }
        if self.api.enabled((WHATS_UP_ONBOARD) as u16) {
            self.api
                .response((WHATS_UP_ONBOARD) as u16, Handler::PetDevice, &[]);
        }
        if self.api.enabled((ANY_SUGGESTIONS) as u16) {
            self.api
                .response((ANY_SUGGESTIONS) as u16, Handler::PetDevice, &[]);
        }
        self.api
            .response((ABOUT_YOUR_RACE) as u16, Handler::PetInfo, &[]);
        if (self.api.get(StoryKey::DNYARRI_LIED) != 0)
            && (self.api.get(StoryKey::LEARNED_TALKING_PET) != 0)
        {
            self.api
                .response((YOU_LIED) as u16, Handler::PetDevice, &[]);
        }
        self.api
            .response((BYE_ONBOARD) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn compel_player(&mut self, r: i32) {
        let mut i: i32;
        let mut last_stack: i32;
        let mut p_str = [0i32; 3];
        last_stack = 0;
        p_str[2_usize] = 0;
        p_str[1_usize] = p_str[2_usize];
        p_str[0_usize] = p_str[1_usize];
        if r == WHAT_ARE_YOU {
            self.api.npc(JUST_TALKING_PET);
            self.api.disable((WHAT_ARE_YOU) as u16);
        } else if r == WHAT_DO_TO_UMGAH {
            self.api.npc(DID_NOTHING);
            self.api.disable((WHAT_DO_TO_UMGAH) as u16);
            last_stack = 1;
        } else if r == WE_ARE_VINDICATOR0 {
            self.api.npc(GOOD_FOR_YOU);
            self.api.disable((WE_ARE_VINDICATOR0) as u16);
            last_stack = 2;
        } else if r != 0 {
            if r == BYE_AT_UMGAH {
                self.api.npc(GOODBYE_AT_UMGAH);
            } else if r == MUST_EXPLAIN_PRESENCE {
                self.api.npc(EXPLAIN_NOTHING_MONKEY_BOY);
            } else if r == UMGAH_ZOMBIES {
                self.api.npc(WORKS_LIKE_THIS);
            } else if r == TALKING_PETS_DUMB {
                self.api.npc(OH_NO_YOU_DONT);
            }
            self.api.set(StoryKey::KNOW_UMGAH_ZOMBIES, 1_u8);
            if !(i32::from(self.api.get(StoryKey::TAALO_PROTECTOR_ON_SHIP)) != 0) {
                self.api.set(StoryKey::PLAYER_HYPNOTIZED, 1_u8);
            } else {
                self.api.npc(CANT_COMPEL);
                self.api.segue(1);
            }
            return;
        }
        if self.api.enabled((WHAT_ARE_YOU) as u16) {
            p_str[0_usize] = WHAT_ARE_YOU;
        } else {
            p_str[0_usize] = TALKING_PETS_DUMB;
        }
        if i32::from(self.api.get(StoryKey::KNOW_UMGAH_ZOMBIES)) != 0 {
            if self.api.enabled((WHAT_DO_TO_UMGAH) as u16) {
                p_str[1_usize] = WHAT_DO_TO_UMGAH;
            } else {
                p_str[1_usize] = UMGAH_ZOMBIES;
            }
        }
        if self.api.enabled((WE_ARE_VINDICATOR0) as u16) {
            p_str[2_usize] = WE_ARE_VINDICATOR0;
        } else {
            p_str[2_usize] = MUST_EXPLAIN_PRESENCE;
        }
        if p_str[last_stack as usize] != 0 {
            if p_str[last_stack as usize] != WE_ARE_VINDICATOR0 {
                self.api.response(
                    (p_str[last_stack as usize]) as u16,
                    Handler::CompelPlayer,
                    &[],
                );
            } else {
                self.api.response(
                    (p_str[last_stack as usize]) as u16,
                    Handler::CompelPlayer,
                    &[
                        WE_ARE_VINDICATOR0,
                        super::script::GLOBAL_SHIP_NAME,
                        WE_ARE_VINDICATOR1,
                    ],
                );
            }
        }
        {
            i = 0;
            while i < 3 {
                {
                    if (i != last_stack) && (p_str[i as usize] != 0) {
                        if p_str[i as usize] != WE_ARE_VINDICATOR0 {
                            self.api.response(
                                (p_str[i as usize]) as u16,
                                Handler::CompelPlayer,
                                &[],
                            );
                        } else {
                            self.api.response(
                                (p_str[i as usize]) as u16,
                                Handler::CompelPlayer,
                                &[
                                    WE_ARE_VINDICATOR0,
                                    super::script::GLOBAL_SHIP_NAME,
                                    WE_ARE_VINDICATOR1,
                                ],
                            );
                        }
                    }
                }
                {
                    i = (i + 1) as u8 as i32;
                    i
                };
            }
        }
        self.api
            .response((BYE_AT_UMGAH) as u16, Handler::CompelPlayer, &[]);
    }
    pub(crate) fn kill_pet(&mut self, r: i32) {
        if r == MUST_KILL {
            self.api.npc(DONT_KILL);
            self.api.talk_segue((!0_u16 as i32) as u16);
            self.mind_control_strobe();
        }
        self.api
            .response((WANT_KILL_1) as u16, Handler::ExitConversation, &[]);
        self.api
            .response((WANT_KILL_2) as u16, Handler::ExitConversation, &[]);
        self.api
            .response((WANT_KILL_3) as u16, Handler::ExitConversation, &[]);
    }
    pub(crate) fn pet_deal(&mut self, r: i32) {
        if r == WHAT_KIND_OF_DEAL {
            self.api.npc(HELP_DEFEAT_URQUAN);
            self.api.disable((WHAT_KIND_OF_DEAL) as u16);
        } else if r == HOW_TRUST {
            self.api.npc(TRUST);
            self.api.disable((HOW_TRUST) as u16);
        } else if r == BONELESS_DWEEB {
            self.api.npc(YOUR_BONELESS_DWEEB);
            self.api.disable((BONELESS_DWEEB) as u16);
        } else if r == WHAT_ARE_YOU_REALLY {
            self.api.npc(POOR_DNYARRI);
            self.api.disable((WHAT_ARE_YOU_REALLY) as u16);
        } else if r == HARD_TO_BELIEVE {
            self.api.npc(ITS_TRUE);
            self.api.set(StoryKey::DNYARRI_LIED, 1_u8);
            self.api.disable((HARD_TO_BELIEVE) as u16);
        } else if r == BULLSHIT {
            self.api.npc(WORTH_A_TRY);
            self.api.disable((BULLSHIT) as u16);
        } else if r == KILL_YOU {
            self.api.npc(PLEASE_DONT);
            self.api.disable((KILL_YOU) as u16);
        }
        if self.api.enabled((WHAT_KIND_OF_DEAL) as u16) {
            self.api
                .response((WHAT_KIND_OF_DEAL) as u16, Handler::PetDeal, &[]);
        } else {
            if self.api.enabled((HOW_TRUST) as u16) {
                self.api.response((HOW_TRUST) as u16, Handler::PetDeal, &[]);
            } else if self.api.enabled((BONELESS_DWEEB) as u16) {
                self.api
                    .response((BONELESS_DWEEB) as u16, Handler::PetDeal, &[]);
            }
            self.api
                .response((OK_LETS_DO_IT) as u16, Handler::ExitConversation, &[]);
        }
        if self.api.enabled((WHAT_ARE_YOU_REALLY) as u16) {
            self.api
                .response((WHAT_ARE_YOU_REALLY) as u16, Handler::PetDeal, &[]);
        } else {
            if self.api.enabled((HARD_TO_BELIEVE) as u16)
                && !(i32::from(self.api.get(StoryKey::LEARNED_TALKING_PET)) != 0)
            {
                self.api
                    .response((HARD_TO_BELIEVE) as u16, Handler::PetDeal, &[]);
            } else if self.api.enabled((BULLSHIT) as u16)
                && (i32::from(self.api.get(StoryKey::LEARNED_TALKING_PET)) != 0)
            {
                self.api.response((BULLSHIT) as u16, Handler::PetDeal, &[]);
            }
        }
        if self.api.enabled((KILL_YOU) as u16) {
            self.api.response((KILL_YOU) as u16, Handler::PetDeal, &[]);
        } else if self.api.enabled((MUST_KILL) as u16) {
            self.api.response((MUST_KILL) as u16, Handler::KillPet, &[]);
        }
    }
    pub(crate) fn intro(&mut self) {
        let mut visits: i32;
        if (i32::from(self.api.activity()) as u8 as i32) == 5 {
            self.api.npc(OUT_TAKES);
            self.api.segue(0);
            return;
        }
        if (i32::from(self.api.activity()) as u8 as i32) == 1 {
            self.api.set(StoryKey::SHIP_TO_COMPEL, 0_u8);
            self.api.segue(1);
            if !((i32::from(self.api.flags()) & 32) != 0) {
                self.api.npc(HUMAN_PEP_TALK);
            } else {
                self.api.npc(CYBORG_PEP_TALK);
            }
        } else if self.api.get(StoryKey::READY_TO_CONFUSE_URQUAN) != 0 {
            self.api.set(StoryKey::SHIP_TO_COMPEL, 0_u8);
            self.api.set(StoryKey::READY_TO_CONFUSE_URQUAN, 0_u8);
            self.api.set(StoryKey::AWARE_OF_SAMATRA, 1_u8);
            if i32::from(self.api.get(StoryKey::CHMMR_BOMB_STATE)) != 3 {
                self.api.npc(HAVENT_GOT_EVERYTHING);
                if !(self.api.get(StoryKey::UTWIG_BOMB_ON_SHIP) != 0) {
                    self.api.npc(NEED_BOMB);
                } else {
                    self.api.npc(SOUP_UP_BOMB);
                }
                self.api.segue(0);
            } else if self.api.get(StoryKey::URQUAN_MESSED_UP) != 0 {
                self.api.npc(HELLO_AFTER_COMPEL_URQUAN);
                self.api.segue(0);
            } else {
                self.api.npc(I_SENSE_MY_SLAVES);
                self.mind_fuck_urquan(0_u16 as i32);
            }
        } else if self.api.get(StoryKey::TALKING_PET_ON_SHIP) != 0 {
            visits = i32::from(self.api.get(StoryKey::TALKING_PET_VISITS));
            if !(self.api.get(StoryKey::TAALO_PROTECTOR_ON_SHIP) != 0) && (visits == 0) {
                {
                    visits = (visits + 1) as u8 as i32;
                    visits
                };
            }
            let res = {
                let old = visits;
                visits = (visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => self.api.npc(HELLO_AS_DEVICE_1),
                1 => self.api.npc(HELLO_AS_DEVICE_2),
                2 => self.api.npc(HELLO_AS_DEVICE_3),
                3 => self.api.npc(HELLO_AS_DEVICE_4),
                4 => self.api.npc(HELLO_AS_DEVICE_5),
                5 => self.api.npc(HELLO_AS_DEVICE_6),
                6 => self.api.npc(HELLO_AS_DEVICE_7),
                7 => {
                    self.api.npc(HELLO_AS_DEVICE_8);
                    {
                        visits = (visits - 1) as u8 as i32;
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::TALKING_PET_VISITS, visits as u8);
            self.pet_device(0_u16 as i32);
        } else if self.api.npc_ships() {
            visits = i32::from(self.api.get(StoryKey::TALKING_PET_HOME_VISITS));
            let res = {
                let old = visits;
                visits = (visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.set(StoryKey::UMGAH_VISITS, 0_u8);
                    self.api.npc(HELLO_AT_UMGAH);
                }
                1 => self.api.npc(HYPNOTIZE_AGAIN_1),
                2 => self.api.npc(HYPNOTIZE_AGAIN_2),
                3 => self.api.npc(HYPNOTIZE_AGAIN_3),
                4 => {
                    self.api.npc(HYPNOTIZE_AGAIN_4);
                    {
                        visits = (visits - 1) as u8 as i32;
                        visits
                    };
                }
                _ => {}
            }
            self.api
                .set(StoryKey::TALKING_PET_HOME_VISITS, visits as u8);
            if visits == 1 {
                self.compel_player(0_u16 as i32);
            } else if !(self.api.get(StoryKey::TAALO_PROTECTOR_ON_SHIP) != 0) {
                self.api.set(StoryKey::PLAYER_HYPNOTIZED, 1_u8);
                self.api.segue(0);
            } else {
                self.api.npc(CANT_COMPEL);
                self.api.segue(1);
            }
        } else {
            if self.api.umgah_alive() {
                self.api.npc(LETS_MAKE_A_DEAL);
            } else {
                self.api.npc(UMGAH_ALL_GONE);
                if (self.api.get(StoryKey::TALKING_PET_HOME_VISITS) == 0)
                    || !(self.api.get(StoryKey::TAALO_PROTECTOR_ON_SHIP) != 0)
                {
                    self.api.disable((HOW_TRUST) as u16);
                }
            }
            self.pet_deal(0_u16 as i32);
        }
    }
    pub(crate) fn post_talkpet_enc(&mut self) {}
    pub(crate) fn initialize(&mut self) {
        if (i32::from(self.api.activity()) as u8 as i32) != 1 {
            self.api.segue(0);
        } else {
            self.api.segue(1);
        }
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
