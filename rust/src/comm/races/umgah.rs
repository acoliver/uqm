//! Umgah dialogue, ported from umgahc.c.
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
pub(crate) mod api;
pub(crate) use api::UmgahApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    CombatIsInevitable,
    Zombies,
    NormalUmgah,
    UmgahReward,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: UmgahApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::CombatIsInevitable => self.combat_is_inevitable(r),
            Handler::Zombies => self.zombies(r),
            Handler::NormalUmgah => self.normal_umgah(r),
            Handler::UmgahReward => self.umgah_reward(r),
        }
    }
    pub(crate) fn combat_is_inevitable(&mut self, r: i32) {
        self.api.segue(1);
        if r == BYE_ZOMBIE {
            self.api.npc(GOODBYE_ZOMBIE);
            self.api.segue(0);
        } else if r == BYE_PRE_ZOMBIE {
            self.api.npc(GOODBYE_PRE_ZOMBIE);
        } else if r == CAN_WE_BE_FRIENDS {
            self.api.npc(SURE_FRIENDS);
            self.api.set(StoryKey::UMGAH_MENTIONED_TRICKS, 1_u8);
        } else if r == EVIL_BLOBBIES_GIVE_UP {
            self.api.npc(NOT_EVIL_BLOBBIES);
            self.api.set(StoryKey::UMGAH_EVIL_BLOBBIES, 1_u8);
        } else if r == EVIL_BLOBBIES_MUST_DIE {
            self.api.npc(OH_NO_WE_WONT);
        } else if r == THREAT {
            self.api.npc(NO_THREAT);
        } else if r == DONT_BELIEVE {
            self.api.npc(THEN_DIE);
            self.api.set(StoryKey::KNOW_UMGAH_ZOMBIES, 1_u8);
            self.api.set(StoryKey::UMGAH_VISITS, 0_u8);
        } else if r == BYE_UNKNOWN {
            self.api.npc(GOODBYE_UNKNOWN);
            self.api.segue(0);
        } else if r == BYE_POST_ZOMBIE {
            self.api.npc(FUNNY_IDEA);
            self.api.talk_segue((!0_u16 as i32) as u16);
            self.api.add_escorts(
                (crate::engine::species::ship::UMGAH_SHIP as i32) as u16,
                4_i16,
            );
            self.api.set(StoryKey::UMGAH_HOSTILE, 1_u8);
        }
    }
    pub(crate) fn zombies(&mut self, r: i32) {
        if i32::from(self.api.get(StoryKey::MET_NORMAL_UMGAH)) != 0 {
            if r == WHATS_UP_ZOMBIES {
                self.api.npc(GENERAL_INFO_ZOMBIE);
                self.api.disable((WHATS_UP_ZOMBIES) as u16);
            } else if r == HOW_GOES_TPET {
                self.api.npc(WHAT_TPET);
                self.api.disable((HOW_GOES_TPET) as u16);
            } else if r == YOU_TOLD_US {
                self.api.npc(SADLY_IT_DIED);
                self.api.disable((YOU_TOLD_US) as u16);
            }
            if self.api.enabled((WHATS_UP_ZOMBIES) as u16)
                && self.api.enabled((HOW_GOES_TPET) as u16)
            {
                self.api
                    .response((WHATS_UP_ZOMBIES) as u16, Handler::Zombies, &[]);
            }
            if self.api.enabled((HOW_GOES_TPET) as u16) {
                self.api
                    .response((HOW_GOES_TPET) as u16, Handler::Zombies, &[]);
            } else if self.api.enabled((YOU_TOLD_US) as u16) {
                self.api
                    .response((YOU_TOLD_US) as u16, Handler::Zombies, &[]);
            } else {
                self.api
                    .response((DONT_BELIEVE) as u16, Handler::CombatIsInevitable, &[]);
            }
            if self.api.enabled((WHATS_UP_ZOMBIES) as u16)
                && !(self.api.enabled((HOW_GOES_TPET) as u16))
            {
                self.api
                    .response((WHATS_UP_ZOMBIES) as u16, Handler::Zombies, &[]);
            }
            self.api
                .response((THREAT) as u16, Handler::CombatIsInevitable, &[]);
            self.api
                .response((BYE_UNKNOWN) as u16, Handler::CombatIsInevitable, &[]);
        } else {
            let mut i: i32;
            let mut last_stack: i32;
            let mut p_str = [0i32; 4];
            last_stack = 0;
            p_str[3_usize] = 0;
            p_str[2_usize] = p_str[3_usize];
            p_str[1_usize] = p_str[2_usize];
            p_str[0_usize] = p_str[1_usize];
            if r == EVIL_BLOBBIES {
                self.api.npc(YES_VERY_EVIL);
                self.api.disable((EVIL_BLOBBIES) as u16);
                last_stack = 0;
            } else if r == WE_VINDICATOR0 {
                self.api.npc(GOOD_FOR_YOU_1);
                self.api.disable((WE_VINDICATOR0) as u16);
                last_stack = 1;
            } else if r == COME_IN_PEACE {
                self.api.npc(GOOD_FOR_YOU_2);
                self.api.disable((COME_IN_PEACE) as u16);
                last_stack = 1;
            } else if r == KNOW_ANY_JOKES {
                self.api.npc(JOKE_1);
                self.api.disable((KNOW_ANY_JOKES) as u16);
                last_stack = 2;
            } else if r == BETTER_JOKE {
                self.api.npc(JOKE_2);
                self.api.disable((BETTER_JOKE) as u16);
                last_stack = 2;
            } else if r == NOT_VERY_FUNNY {
                self.api.npc(YES_WE_ARE);
                self.api.disable((NOT_VERY_FUNNY) as u16);
                last_stack = 2;
            } else if r == WHAT_ABOUT_TPET {
                self.api.npc(WHAT_TPET);
                self.api.disable((WHAT_ABOUT_TPET) as u16);
                last_stack = 3;
            } else if r == GIVE_UP_OR_DIE {
                self.api.npc(NOT_GIVE_UP);
                self.api.segue(1);
                return;
            } else if r == ARILOU_TOLD_US {
                self.api.npc(THEN_DIE);
                self.api.segue(1);
                self.api.set(StoryKey::KNOW_UMGAH_ZOMBIES, 1_u8);
                self.api.set(StoryKey::UMGAH_VISITS, 0_u8);
                return;
            }
            if self.api.enabled((EVIL_BLOBBIES) as u16) {
                p_str[0_usize] = EVIL_BLOBBIES;
            } else {
                p_str[0_usize] = GIVE_UP_OR_DIE;
            }
            if self.api.enabled((WE_VINDICATOR0) as u16) {
                p_str[1_usize] = WE_VINDICATOR0;
            } else if self.api.enabled((COME_IN_PEACE) as u16) {
                p_str[1_usize] = COME_IN_PEACE;
            }
            if self.api.enabled((KNOW_ANY_JOKES) as u16) {
                p_str[2_usize] = KNOW_ANY_JOKES;
            } else if self.api.enabled((BETTER_JOKE) as u16) {
                p_str[2_usize] = BETTER_JOKE;
            } else if self.api.enabled((NOT_VERY_FUNNY) as u16) {
                p_str[2_usize] = NOT_VERY_FUNNY;
            }
            if self.api.enabled((WHAT_ABOUT_TPET) as u16) {
                p_str[3_usize] = WHAT_ABOUT_TPET;
            } else {
                p_str[3_usize] = ARILOU_TOLD_US;
            }
            if p_str[last_stack as usize] != 0 {
                if p_str[last_stack as usize] != WE_VINDICATOR0 {
                    self.api
                        .response((p_str[last_stack as usize]) as u16, Handler::Zombies, &[]);
                } else {
                    self.api.response(
                        (p_str[last_stack as usize]) as u16,
                        Handler::Zombies,
                        &[
                            WE_VINDICATOR0,
                            super::script::GLOBAL_PLAYER_NAME,
                            WE_VINDICATOR1,
                            super::script::GLOBAL_SHIP_NAME,
                            WE_VINDICATOR2,
                        ],
                    );
                }
            }
            {
                i = 0;
                while i < 4 {
                    {
                        if (i != last_stack) && (p_str[i as usize] != 0) {
                            if p_str[i as usize] != WE_VINDICATOR0 {
                                self.api.response(
                                    (p_str[i as usize]) as u16,
                                    Handler::Zombies,
                                    &[],
                                );
                            } else {
                                self.api.response(
                                    (p_str[i as usize]) as u16,
                                    Handler::Zombies,
                                    &[
                                        WE_VINDICATOR0,
                                        super::script::GLOBAL_PLAYER_NAME,
                                        WE_VINDICATOR1,
                                        super::script::GLOBAL_SHIP_NAME,
                                        WE_VINDICATOR2,
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
                .response((BYE_ZOMBIE) as u16, Handler::CombatIsInevitable, &[]);
        }
    }
    pub(crate) fn normal_umgah(&mut self, r: i32) {
        if r == WHATS_UP_PRE_ZOMBIE {
            self.api.npc(GENERAL_INFO_PRE_ZOMBIE);
            self.api.disable((WHATS_UP_PRE_ZOMBIE) as u16);
        } else if r == WANT_TO_DEFEAT_URQUAN {
            self.api.npc(FINE_BY_US);
            self.api.disable((WANT_TO_DEFEAT_URQUAN) as u16);
        }
        if !(i32::from(self.api.get(StoryKey::UMGAH_EVIL_BLOBBIES)) != 0) {
            self.api.response(
                (EVIL_BLOBBIES_GIVE_UP) as u16,
                Handler::CombatIsInevitable,
                &[],
            );
        } else {
            self.api.response(
                (EVIL_BLOBBIES_MUST_DIE) as u16,
                Handler::CombatIsInevitable,
                &[],
            );
        }
        if self.api.enabled((WHATS_UP_PRE_ZOMBIE) as u16) {
            self.api
                .response((WHATS_UP_PRE_ZOMBIE) as u16, Handler::NormalUmgah, &[]);
        }
        if self.api.enabled((WANT_TO_DEFEAT_URQUAN) as u16) {
            self.api
                .response((WANT_TO_DEFEAT_URQUAN) as u16, Handler::NormalUmgah, &[]);
        }
        if i32::from(self.api.get(StoryKey::UMGAH_MENTIONED_TRICKS)) == 0 {
            self.api
                .response((CAN_WE_BE_FRIENDS) as u16, Handler::CombatIsInevitable, &[]);
        }
        self.api
            .response((BYE_PRE_ZOMBIE) as u16, Handler::CombatIsInevitable, &[]);
    }
    pub(crate) fn umgah_reward(&mut self, r: i32) {
        if r == WHAT_BEFORE_TPET {
            self.api.npc(TRKD_SPATHI_AND_ILWRATH);
            self.api.disable((WHAT_BEFORE_TPET) as u16);
        } else if r == WHERE_CASTER {
            self.api.npc(SPATHI_TOOK_THEM);
            self.api.disable((WHERE_CASTER) as u16);
        } else if r == OWE_ME_BIG_TIME {
            self.api.npc(THANKS);
            self.api.add_biomass(1000 / 2);
            self.api.disable((OWE_ME_BIG_TIME) as u16);
            self.api.disable((OUR_LARGESSE) as u16);
        } else if r == OUR_LARGESSE {
            self.api.npc(GIVE_LIFEDATA);
            self.api.add_biomass(1000 / 2);
            self.api.disable((OUR_LARGESSE) as u16);
            self.api.disable((OWE_ME_BIG_TIME) as u16);
        } else if r == WHAT_DO_WITH_TPET {
            self.api.npc(TRICK_URQUAN);
            self.api.disable((WHAT_DO_WITH_TPET) as u16);
        } else if r == ANY_JOKES {
            self.api.npc(SURE);
            self.api.disable((ANY_JOKES) as u16);
        } else if r == SO_WHAT_FOR_NOW {
            self.api.npc(DO_THIS_NOW);
            self.api.disable((SO_WHAT_FOR_NOW) as u16);
        }
        if !(i32::from(self.api.get(StoryKey::MET_NORMAL_UMGAH)) != 0) {
            if self.api.enabled((WHAT_BEFORE_TPET) as u16) {
                self.api
                    .response((WHAT_BEFORE_TPET) as u16, Handler::UmgahReward, &[]);
            } else if self.api.enabled((WHERE_CASTER) as u16) {
                self.api
                    .response((WHERE_CASTER) as u16, Handler::UmgahReward, &[]);
            }
        }
        if self.api.enabled((OWE_ME_BIG_TIME) as u16) {
            self.api
                .response((OWE_ME_BIG_TIME) as u16, Handler::UmgahReward, &[]);
            self.api
                .response((OUR_LARGESSE) as u16, Handler::UmgahReward, &[]);
        }
        if self.api.enabled((WHAT_DO_WITH_TPET) as u16) {
            self.api
                .response((WHAT_DO_WITH_TPET) as u16, Handler::UmgahReward, &[]);
        } else if self.api.enabled((ANY_JOKES) as u16)
            && (i32::from(self.api.get(StoryKey::UMGAH_MENTIONED_TRICKS)) < 2)
        {
            self.api
                .response((ANY_JOKES) as u16, Handler::UmgahReward, &[]);
        }
        if self.api.enabled((SO_WHAT_FOR_NOW) as u16) {
            self.api
                .response((SO_WHAT_FOR_NOW) as u16, Handler::UmgahReward, &[]);
        }
        self.api
            .response((BYE_POST_ZOMBIE) as u16, Handler::CombatIsInevitable, &[]);
    }
    pub(crate) fn intro(&mut self) {
        let mut visits: i32;
        if self.api.get(StoryKey::UMGAH_HOSTILE) != 0 {
            visits = i32::from(self.api.get(StoryKey::UMGAH_VISITS));
            let res = {
                let old = visits;
                visits = (visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HOSTILE_HELLO_1);
                }
                1 => {
                    self.api.npc(HOSTILE_HELLO_2);
                }
                2 => {
                    self.api.npc(HOSTILE_HELLO_3);
                }
                3 => {
                    self.api.npc(HOSTILE_HELLO_4);
                    {
                        visits = (visits - 1) as u8 as i32;
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::UMGAH_VISITS, visits as u8);
            self.api.segue(1);
        } else if self.api.get(StoryKey::UMGAH_ZOMBIE_BLOBBIES) != 0 {
            visits = i32::from(self.api.get(StoryKey::UMGAH_VISITS));
            if self.api.get(StoryKey::TALKING_PET_VISITS) != 0 {
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(DESTROY_INTERFERER_1);
                    }
                    1 => {
                        self.api.npc(DESTROY_INTERFERER_2);
                    }
                    2 => {
                        self.api.npc(DESTROY_INTERFERER_3);
                    }
                    3 => {
                        self.api.npc(DESTROY_INTERFERER_4);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.segue(1);
            } else if self.api.get(StoryKey::KNOW_UMGAH_ZOMBIES) != 0 {
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(REVEALED_ZOMBIE_HELLO_1);
                    }
                    1 => {
                        self.api.npc(REVEALED_ZOMBIE_HELLO_2);
                    }
                    2 => {
                        self.api.npc(REVEALED_ZOMBIE_HELLO_3);
                    }
                    3 => {
                        self.api.npc(REVEALED_ZOMBIE_HELLO_4);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.segue(1);
            } else {
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(UNKNOWN_ZOMBIE_HELLO_1);
                    }
                    1 => {
                        self.api.npc(UNKNOWN_ZOMBIE_HELLO_2);
                    }
                    2 => {
                        self.api.npc(UNKNOWN_ZOMBIE_HELLO_3);
                    }
                    3 => {
                        self.api.npc(UNKNOWN_ZOMBIE_HELLO_4);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.zombies(0_u16 as i32);
            }
            self.api.set(StoryKey::UMGAH_VISITS, visits as u8);
        } else if !(self.api.get(StoryKey::TALKING_PET) != 0) {
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                visits = i32::from(self.api.get(StoryKey::UMGAH_HOME_VISITS));
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(HWLD_PRE_ZOMBIE_HELLO_1);
                    }
                    1 => {
                        self.api.npc(HWLD_PRE_ZOMBIE_HELLO_2);
                    }
                    2 => {
                        self.api.npc(HWLD_PRE_ZOMBIE_HELLO_3);
                    }
                    3 => {
                        self.api.npc(HWLD_PRE_ZOMBIE_HELLO_4);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::UMGAH_HOME_VISITS, visits as u8);
            } else {
                visits = i32::from(self.api.get(StoryKey::UMGAH_VISITS));
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(SPACE_PRE_ZOMBIE_HELLO_1);
                    }
                    1 => {
                        self.api.npc(SPACE_PRE_ZOMBIE_HELLO_2);
                    }
                    2 => {
                        self.api.npc(SPACE_PRE_ZOMBIE_HELLO_3);
                    }
                    3 => {
                        self.api.npc(SPACE_PRE_ZOMBIE_HELLO_4);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::UMGAH_VISITS, visits as u8);
            }
            self.normal_umgah(0_u16 as i32);
        } else {
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                self.api.npc(POST_ZOMBIE_HWLD_HELLO);
                self.umgah_reward(0_u16 as i32);
            } else {
                visits = i32::from(self.api.get(StoryKey::UMGAH_VISITS));
                let res = {
                    let old = visits;
                    visits = (visits + 1) as u8 as i32;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(REWARD_AT_HOMEWORLD_1);
                    }
                    1 => {
                        self.api.npc(REWARD_AT_HOMEWORLD_2);
                        {
                            visits = (visits - 1) as u8 as i32;
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::UMGAH_VISITS, visits as u8);
                self.api.segue(0);
            }
        }
    }
    pub(crate) fn post_umgah_enc(&mut self) {
        if !(i32::from(self.api.get(StoryKey::UMGAH_ZOMBIE_BLOBBIES)) != 0) {
            self.api.set(StoryKey::MET_NORMAL_UMGAH, 1_u8);
        }
    }
    pub(crate) fn initialize(&mut self) {
        if ((i32::from(self.api.get(StoryKey::TALKING_PET)) != 0)
            && !(i32::from(self.api.get(StoryKey::UMGAH_HOSTILE)) != 0))
            || ((i32::from(self.api.activity()) as u8 as i32) == 5)
        {
            self.api.segue(0);
        } else {
            self.api.segue(1);
        }
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
