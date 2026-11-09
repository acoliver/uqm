//! Supox dialogue, ported from the original C script.
pub(crate) use super::alliance::AllianceApi;
pub(crate) use super::script::{GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
pub(crate) use crate::comm::locdata::CLocData;
pub(crate) use crate::engine::StoryKey;
mod phrases;
pub(crate) use super::alliance::{SUPOX_SHIP, UTWIG_SHIP};
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    AlliedHome,
    ExitConversation,
    BeforeKohrAh,
    AfterKohrAh,
    NeutralSupox,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: AllianceApi<Handler>> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::AlliedHome => self.allied_home(r),
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::BeforeKohrAh => self.before_kohr_ah(r),
            Handler::AfterKohrAh => self.after_kohr_ah(r),
            Handler::NeutralSupox => self.neutral_supox(r),
        }
    }
    pub(crate) fn initialize(&mut self, _desc: &mut CLocData) {
        self.api.segue(u32::from(
            self.api.get(StoryKey::SUPOX_HOSTILE) != 0 && self.api.activity() as u8 != 5,
        ));
    }
    pub(crate) fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0);
        if r == BYE_NEUTRAL {
            self.api.npc(i32::from(GOODBYE_NEUTRAL));
        } else if r == WHAT_DO_I_DO_NOW {
            self.api.npc(i32::from(FIX_IT));
        } else if r == THANKS_NOW_WE_EAT_YOU {
            self.api.npc(i32::from(HIDEOUS_MONSTERS));
            self.api.set(StoryKey::SUPOX_HOSTILE, 1_u8);
            self.api.set(StoryKey::SUPOX_HOME_VISITS, 0_u8);
            self.api.set(StoryKey::SUPOX_VISITS, 0_u8);
        } else if r == BYE_AFTER_SPACE {
            self.api.npc(i32::from(GOODBYE_AFTER_SPACE));
        } else if r == BYE_BEFORE_SPACE {
            self.api.npc(i32::from(GOODBYE_BEFORE_SPACE));
        } else if r == BYE_ALLIED_HOMEWORLD {
            self.api.npc(i32::from(GOODBYE_ALLIED_HOMEWORLD));
        } else if r == CAN_YOU_HELP {
            self.api.npc(i32::from(HOW_HELP));
            if self.api.escort_feasibility(SUPOX_SHIP) == 0 {
                self.api.npc(i32::from(DONT_NEED));
            } else {
                self.api.npc(i32::from(HAVE_4_SHIPS));
                self.api.talk_segue(!0u16);
                self.api.add_escorts(SUPOX_SHIP, 4);
            }
        }
    }
    pub(crate) fn allied_home(&mut self, r: u16) {
        let mut news: u8;
        news = self.api.get(StoryKey::SUPOX_WAR_NEWS);
        let num_visits: u8 = self.api.get(StoryKey::UTWIG_SUPOX_MISSION);
        if r == HOW_WENT_WAR {
            self.api.npc(i32::from(ABOUT_BATTLE));
            news |= 1 << 0;
        } else if r == HOW_GOES_WAR {
            if num_visits == 1 {
                self.api.npc(i32::from(FLEET_ON_WAY));
                self.api.set(StoryKey::SUPOX_WAR_NEWS, 1_u8);
            } else {
                match self.api.get(StoryKey::SUPOX_WAR_NEWS) {
                    0 => {
                        self.api.npc(i32::from(BATTLE_HAPPENS_1));
                        news = 1;
                    }
                    1 => {
                        self.api.npc(i32::from(BATTLE_HAPPENS_2));
                        news = 2;
                    }
                    _ => {}
                }
            }
            self.api.disable(HOW_GOES_WAR);
        } else if r == LEARN_NEW_INFO {
            if num_visits < 5 {
                self.api.npc(i32::from(NO_NEW_INFO));
            } else {
                self.api.npc(i32::from(SAMATRA));
                news |= 1 << 1;
            }
            self.api.disable(LEARN_NEW_INFO);
        } else if r == WHAT_NOW_HOMEWORLD {
            if num_visits < 5 {
                self.api.npc(i32::from(UP_TO_YOU));
            } else {
                self.api.npc(i32::from(HOPE_KILL_EACH_OTHER));
            }
            self.api.disable(WHAT_NOW_HOMEWORLD);
        } else if r == HOW_IS_ULTRON {
            self.api.npc(i32::from(ULTRON_IS_GREAT));
            self.api.disable(HOW_IS_ULTRON);
        }
        self.api.set(StoryKey::SUPOX_WAR_NEWS, news);
        if num_visits >= 5 {
            if !((news & (1 << 0)) != 0) {
                self.api.response(HOW_WENT_WAR, Handler::AlliedHome, &[]);
            }
        } else if self.api.enabled(HOW_GOES_WAR)
            && (((num_visits == 1) && (news == 0)) || (num_visits != 0 && (news < 2)))
        {
            self.api.response(HOW_GOES_WAR, Handler::AlliedHome, &[]);
        }
        if self.api.enabled(LEARN_NEW_INFO) {
            self.api.response(LEARN_NEW_INFO, Handler::AlliedHome, &[]);
        }
        if self.api.enabled(WHAT_NOW_HOMEWORLD) {
            self.api
                .response(WHAT_NOW_HOMEWORLD, Handler::AlliedHome, &[]);
        }
        if self.api.enabled(HOW_IS_ULTRON) {
            self.api.response(HOW_IS_ULTRON, Handler::AlliedHome, &[]);
        }
        if num_visits == 0 {
            self.api
                .response(CAN_YOU_HELP, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_ALLIED_HOMEWORLD, Handler::ExitConversation, &[]);
    }
    pub(crate) fn before_kohr_ah(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_BEFORE_SPACE {
            num_visits = self.api.get(StoryKey::SUPOX_INFO);
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_BEFORE_SPACE_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_BEFORE_SPACE_2));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set(StoryKey::SUPOX_INFO, num_visits);
            self.api.disable(WHATS_UP_BEFORE_SPACE);
        } else if r == WHAT_NOW_BEFORE_SPACE {
            self.api.npc(i32::from(DO_THIS_BEFORE_SPACE));
            self.api.disable(WHAT_NOW_BEFORE_SPACE);
        }
        if self.api.enabled(WHATS_UP_BEFORE_SPACE) {
            self.api
                .response(WHATS_UP_BEFORE_SPACE, Handler::BeforeKohrAh, &[]);
        }
        if self.api.enabled(WHAT_NOW_BEFORE_SPACE) {
            self.api
                .response(WHAT_NOW_BEFORE_SPACE, Handler::BeforeKohrAh, &[]);
        }
        self.api
            .response(BYE_BEFORE_SPACE, Handler::ExitConversation, &[]);
    }
    pub(crate) fn after_kohr_ah(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_AFTER_SPACE {
            num_visits = self.api.get(StoryKey::SUPOX_INFO);
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_AFTER_SPACE_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_AFTER_SPACE_2));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set(StoryKey::SUPOX_INFO, num_visits);
            self.api.disable(WHATS_UP_AFTER_SPACE);
        } else if r == WHAT_NOW_AFTER_SPACE {
            self.api.npc(i32::from(DO_THIS_AFTER_SPACE));
            self.api.disable(WHAT_NOW_AFTER_SPACE);
        }
        if self.api.enabled(WHATS_UP_AFTER_SPACE) {
            self.api
                .response(WHATS_UP_AFTER_SPACE, Handler::AfterKohrAh, &[]);
        }
        if self.api.enabled(WHAT_NOW_AFTER_SPACE) {
            self.api
                .response(WHAT_NOW_AFTER_SPACE, Handler::AfterKohrAh, &[]);
        }
        self.api
            .response(BYE_AFTER_SPACE, Handler::ExitConversation, &[]);
    }
    pub(crate) fn neutral_supox(&mut self, r: u16) {
        let mut fragments: Vec<i32> = Vec::new();
        let _i: u8;
        let mut last_stack: u8;

        let mut p_str: [u16; 3] = [0; 3];
        last_stack = 0;
        p_str[2_usize] = 0;
        p_str[1_usize] = p_str[2_usize];
        p_str[0_usize] = p_str[1_usize];
        if r == I_AM0 {
            self.api.npc(i32::from(WE_ARE_SUPOX));
            self.api.set(StoryKey::SUPOX_STACK1, 1_u8);
            self.api.disable(I_AM0);
        } else if r == MY_SHIP0 {
            self.api.npc(i32::from(OUR_SHIP));
            self.api.set(StoryKey::SUPOX_STACK1, 2_u8);
            self.api.disable(MY_SHIP0);
        } else if r == FROM_ALLIANCE0 {
            self.api.npc(i32::from(FROM_SUPOX));
            self.api.set(StoryKey::SUPOX_STACK1, 3_u8);
            self.api.disable(FROM_ALLIANCE0);
        } else if r == ARE_YOU_COPYING {
            self.api.npc(i32::from(YEAH_SORRY));
            self.api.set(StoryKey::SUPOX_STACK1, 4_u8);
        } else if r == WHY_COPY {
            self.api.npc(i32::from(SYMBIOTS));
            self.api.set(StoryKey::SUPOX_STACK1, 5_u8);
        } else if r == TELL_US_OF_YOUR_SPECIES {
            self.api.npc(i32::from(OUR_SPECIES));
            last_stack = 1;
            self.api.set(StoryKey::SUPOX_STACK2, 1_u8);
        } else if r == PLANTS_ARENT_INTELLIGENT {
            self.api.npc(i32::from(PROVES_WERE_SPECIAL));
            self.api.set(StoryKey::SUPOX_STACK2, 2_u8);
        } else if r == ANYONE_AROUND_HERE {
            self.api.npc(i32::from(UTWIG_NEARBY));
            last_stack = 2;
            self.api.set(StoryKey::SUPOX_WAR_NEWS, 1_u8);
            self.api.track(UTWIG_SHIP);
        } else if r == WHAT_RELATION_TO_UTWIG {
            self.api.npc(i32::from(UTWIG_ALLIES));
            last_stack = 2;
            self.api.set(StoryKey::SUPOX_WAR_NEWS, 1_u8);
        } else if r == WHATS_WRONG_WITH_UTWIG {
            self.api.npc(i32::from(BROKE_ULTRON));
            last_stack = 2;
            self.api.set(StoryKey::SUPOX_WAR_NEWS, 2_u8);
        } else if r == WHATS_ULTRON {
            self.api.npc(i32::from(TAKE_ULTRON));
            self.api.set(StoryKey::SUPOX_WAR_NEWS, 0_u8);
            self.api.set(StoryKey::ULTRON_CONDITION, 1_u8);
            self.api
                .response(WHAT_DO_I_DO_NOW, Handler::ExitConversation, &[]);
            self.api
                .response(THANKS_NOW_WE_EAT_YOU, Handler::ExitConversation, &[]);
            return;
        } else if r == GOT_FIXED_ULTRON {
            self.api.npc(i32::from(GOOD_GIVE_TO_UTWIG));
            self.api.set(StoryKey::SUPOX_ULTRON_HELP, 1_u8);
        } else if r == LOOK_I_REPAIRED_LOTS {
            self.api.npc(i32::from(ALMOST_THERE));
            self.api.set(StoryKey::SUPOX_ULTRON_HELP, 1_u8);
        } else if r == LOOK_I_SLIGHTLY_REPAIRED {
            self.api.npc(i32::from(GREAT_DO_MORE));
            self.api.set(StoryKey::SUPOX_ULTRON_HELP, 1_u8);
        } else if r == WHERE_GET_REPAIRS {
            self.api.npc(i32::from(ANCIENT_RHYME));
            self.api.set(StoryKey::SUPOX_ULTRON_HELP, 1_u8);
        }
        match self.api.get(StoryKey::SUPOX_STACK2) {
            0 => p_str[1_usize] = TELL_US_OF_YOUR_SPECIES,
            1 => p_str[1_usize] = PLANTS_ARENT_INTELLIGENT,
            _ => {}
        }
        match self.api.get(StoryKey::SUPOX_STACK1) {
            0 => {
                fragments.clear();
                fragments.push(I_AM0 as i32);
                fragments.push(GLOBAL_PLAYER_NAME);
                fragments.push(I_AM1 as i32);
                p_str[0_usize] = I_AM0;
                p_str[1_usize] = 0;
            }
            1 => {
                fragments.clear();
                fragments.push(MY_SHIP0 as i32);
                fragments.push(GLOBAL_SHIP_NAME);
                fragments.push(MY_SHIP1 as i32);
                p_str[0_usize] = MY_SHIP0;
                p_str[1_usize] = 0;
            }
            2 => {
                {
                    fragments.clear();
                    fragments.push(FROM_ALLIANCE0 as i32);
                    fragments.extend(self.api.alliance_fragments(NAME_1));
                    fragments.push(FROM_ALLIANCE1 as i32);
                }
                p_str[0_usize] = FROM_ALLIANCE0;
                p_str[1_usize] = 0;
            }
            3 => {
                p_str[0_usize] = ARE_YOU_COPYING;
                p_str[1_usize] = 0;
            }
            4 => {
                p_str[0_usize] = WHY_COPY;
                p_str[1_usize] = 0;
            }
            _ => {}
        }
        let visits: u8 = self.api.get(StoryKey::ULTRON_CONDITION);
        if visits == 0 {
            match self.api.get(StoryKey::SUPOX_WAR_NEWS) {
                0 => {
                    if (self.api.get(StoryKey::UTWIG_VISITS) != 0
                        || self.api.get(StoryKey::UTWIG_HOME_VISITS) != 0)
                        || self.api.get(StoryKey::BOMB_VISITS) != 0
                    {
                        p_str[2_usize] = WHAT_RELATION_TO_UTWIG;
                    } else {
                        p_str[2_usize] = ANYONE_AROUND_HERE;
                    }
                }
                1 => p_str[2_usize] = WHATS_WRONG_WITH_UTWIG,
                2 => p_str[2_usize] = WHATS_ULTRON,
                _ => {}
            }
        }
        if p_str[last_stack as usize] != 0 {
            if (last_stack != 0) || (self.api.get(StoryKey::SUPOX_STACK1) > 2) {
                self.api
                    .response(p_str[last_stack as usize], Handler::NeutralSupox, &[]);
            } else {
                self.api.response(
                    p_str[last_stack as usize],
                    Handler::NeutralSupox,
                    &fragments,
                );
            }
        }
        for i in 0..3 {
            if (i != last_stack) && p_str[i as usize] != 0 {
                if (i != 0) || (self.api.get(StoryKey::SUPOX_STACK1) > 2) {
                    self.api
                        .response(p_str[i as usize], Handler::NeutralSupox, &[]);
                } else {
                    self.api
                        .response(p_str[i as usize], Handler::NeutralSupox, &fragments);
                }
            }
        }
        if !(self.api.get(StoryKey::SUPOX_ULTRON_HELP) != 0) {
            match visits {
                1 => {
                    self.api
                        .response(WHERE_GET_REPAIRS, Handler::NeutralSupox, &[]);
                }
                2 => {
                    self.api
                        .response(LOOK_I_SLIGHTLY_REPAIRED, Handler::NeutralSupox, &[]);
                }
                3 => {
                    self.api
                        .response(LOOK_I_REPAIRED_LOTS, Handler::NeutralSupox, &[]);
                }
                4 => {
                    self.api
                        .response(GOT_FIXED_ULTRON, Handler::NeutralSupox, &[]);
                }
                _ => {}
            }
        }
        self.api
            .response(BYE_NEUTRAL, Handler::ExitConversation, &[]);
    }
    pub(crate) fn intro(&mut self) {
        let mut num_visits: u8;
        if (self.api.activity() as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0);
            return;
        }
        if self.api.get(StoryKey::SUPOX_HOSTILE) != 0 {
            num_visits = self.api.get(StoryKey::SUPOX_VISITS);
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HOSTILE_SPACE_HELLO_1));
                }
                1 => {
                    self.api.npc(i32::from(HOSTILE_SPACE_HELLO_2));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set(StoryKey::SUPOX_VISITS, num_visits);
            self.api.segue(0);
        } else if self.api.alliance(SUPOX_SHIP) == 1 {
            if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::SUPOX_HOME_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits += 1;
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
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set(StoryKey::SUPOX_HOME_VISITS, num_visits);
                self.allied_home(0_u16);
            } else {
                num_visits = self.api.get(StoryKey::UTWIG_SUPOX_MISSION);
                if num_visits == 1 {
                    num_visits = self.api.get(StoryKey::SUPOX_VISITS);
                    let res = {
                        let old = num_visits;
                        num_visits += 1;
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(HELLO_BEFORE_KOHRAH_SPACE_1));
                        }
                        1 => {
                            self.api.npc(i32::from(HELLO_BEFORE_KOHRAH_SPACE_2));
                            num_visits -= 1;
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::SUPOX_VISITS, num_visits);
                    self.before_kohr_ah(0_u16);
                } else if num_visits < 5 {
                    num_visits = self.api.get(StoryKey::SUPOX_VISITS);
                    let res = {
                        let old = num_visits;
                        num_visits += 1;
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(HELLO_DURING_KOHRAH_SPACE_1));
                        }
                        1 => {
                            self.api.npc(i32::from(HELLO_DURING_KOHRAH_SPACE_2));
                            num_visits -= 1;
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::SUPOX_VISITS, num_visits);
                } else {
                    num_visits = self.api.get(StoryKey::SUPOX_VISITS);
                    let res = {
                        let old = num_visits;
                        num_visits += 1;
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(HELLO_AFTER_KOHRAH_SPACE_1));
                        }
                        1 => {
                            self.api.npc(i32::from(HELLO_AFTER_KOHRAH_SPACE_2));
                            num_visits -= 1;
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::SUPOX_VISITS, num_visits);
                    self.after_kohr_ah(0_u16);
                }
            }
        } else {
            if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::SUPOX_HOME_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits += 1;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_2));
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set(StoryKey::SUPOX_HOME_VISITS, num_visits);
            } else {
                num_visits = self.api.get(StoryKey::SUPOX_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits += 1;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(NEUTRAL_SPACE_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(NEUTRAL_SPACE_HELLO_2));
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set(StoryKey::SUPOX_VISITS, num_visits);
            }
            self.neutral_supox(0_u16);
        }
    }
}

#[cfg(test)]
use crate::comm::descriptors::supox::descriptor;

#[cfg(test)]
mod tests;
