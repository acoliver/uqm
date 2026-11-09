//! Utwig dialogue, ported from the original C script.
pub(crate) use super::alliance::AllianceApi;
pub(crate) use super::script::GLOBAL_PLAYER_NAME;
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
    NeutralUtwig,
    BombWorld,
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
            Handler::NeutralUtwig => self.neutral_utwig(r),
            Handler::BombWorld => self.bomb_world(r),
        }
    }
    pub(crate) fn initialize(&mut self, desc: &mut CLocData) {
        if self.api.get(StoryKey::UTWIG_HAVE_ULTRON) != 0 {
            desc.alien_alt_song_res = c"comm.utwig.ultron.music".as_ptr();
            desc.alien_song_flags |= 1;
        } else {
            desc.alien_song_flags &= !1;
        }
        self.api.segue(u32::from(
            self.api.get(StoryKey::UTWIG_HAVE_ULTRON) == 0 && self.api.activity() as u8 != 5,
        ));
    }
    pub(crate) fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0);
        if r == BYE_NEUTRAL {
            self.api.npc(i32::from(GOODBYE_NEUTRAL));
        } else if r == BYE_AFTER_SPACE {
            self.api.npc(i32::from(GOODBYE_AFTER_SPACE));
        } else if r == BYE_BEFORE_SPACE {
            self.api.npc(i32::from(GOODBYE_BEFORE_SPACE));
        } else if r == BYE_ALLIED_HOMEWORLD {
            self.api.npc(i32::from(GOODBYE_ALLIED_HOMEWORLD));
        } else if r == BYE_BOMB {
            self.api.npc(i32::from(GOODBYE_BOMB));
        } else if r == DEMAND_BOMB {
            self.api.npc(i32::from(GUARDS_FIGHT));
            self.api.segue(1);
        } else if r == GOT_ULTRON || r == HEY_WAIT_GOT_ULTRON {
            if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 6)) != 0 {
                self.api.npc(i32::from(NO_ULTRON_AT_BOMB));
                self.api.set(StoryKey::REFUSED_ULTRON_AT_BOMB, 1_u8);
            } else {
                if r == GOT_ULTRON {
                    self.api.npc(i32::from(DONT_WANT_TO_LOOK));
                } else {
                    self.api.npc(i32::from(TAUNT_US_BUT_WE_LOOK));
                }
                if self.api.get(StoryKey::ULTRON_CONDITION) < 4 {
                    match self.api.get(StoryKey::UTWIG_INFO) {
                        0 => {
                            if r == GOT_ULTRON {
                                self.api.npc(i32::from(SICK_TRICK_1));
                            } else {
                                self.api.npc(i32::from(TRICKED_US_1));
                                self.api.segue(1);
                            }
                        }
                        1 => {
                            if r == GOT_ULTRON {
                                self.api.npc(i32::from(SICK_TRICK_2));
                            } else {
                                self.api.npc(i32::from(TRICKED_US_2));
                                self.api.segue(1);
                            }
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::UTWIG_INFO, 1_u8);
                } else {
                    self.api.npc(i32::from(HAPPY_DAYS));
                    if self.api.get(StoryKey::KOHR_AH_FRENZY) != 0 {
                        self.api.npc(i32::from(TOO_LATE));
                    } else {
                        self.api.npc(i32::from(OK_ATTACK_KOHRAH));
                        self.api.event(1, 0, 0, 0, 9);
                    }
                    self.api.set(StoryKey::UTWIG_HAVE_ULTRON, 1_u8);
                    self.api.set(StoryKey::ULTRON_CONDITION, 5_u8);
                    self.api.set(StoryKey::UTWIG_VISITS, 0_u8);
                    self.api.set(StoryKey::SUPOX_VISITS, 0_u8);
                    self.api.set(StoryKey::UTWIG_HOME_VISITS, 0_u8);
                    self.api.set(StoryKey::SUPOX_HOME_VISITS, 0_u8);
                    self.api.set(StoryKey::BOMB_VISITS, 0_u8);
                    self.api.set(StoryKey::SUPOX_INFO, 0_u8);
                    self.api.set(StoryKey::UTWIG_INFO, 0_u8);
                    self.api.set(StoryKey::SUPOX_WAR_NEWS, 0_u8);
                    self.api.set(StoryKey::UTWIG_WAR_NEWS, 0_u8);
                    self.api.set(StoryKey::SUPOX_HOSTILE, 0_u8);
                    self.api.set(StoryKey::UTWIG_HOSTILE, 0_u8);
                    self.api.set_allied(UTWIG_SHIP, 1 != 0);
                    self.api.set_allied(SUPOX_SHIP, 1 != 0);
                }
            }
        } else if r == CAN_YOU_HELP {
            self.api.npc(i32::from(HOW_HELP));
            if self.api.escort_feasibility(UTWIG_SHIP) == 0 {
                self.api.npc(i32::from(DONT_NEED));
            } else {
                self.api.npc(i32::from(HAVE_4_SHIPS));
                self.api.talk_segue(!0u16);
                self.api.add_escorts(UTWIG_SHIP, 4);
            }
        }
    }
    pub(crate) fn allied_home(&mut self, r: u16) {
        let mut news: u8;
        news = self.api.get(StoryKey::UTWIG_WAR_NEWS);
        let num_visits: u8 = self.api.get(StoryKey::UTWIG_SUPOX_MISSION);
        if r == HOW_WENT_WAR {
            self.api.npc(i32::from(ABOUT_BATTLE));
            news |= 1 << 0;
        } else if r == HOW_GOES_WAR {
            if num_visits == 1 {
                self.api.npc(i32::from(FLEET_ON_WAY));
                self.api.set(StoryKey::UTWIG_WAR_NEWS, 1_u8);
            } else {
                match self.api.get(StoryKey::UTWIG_WAR_NEWS) {
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
        self.api.set(StoryKey::UTWIG_WAR_NEWS, news);
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
        if (num_visits == 0) && (self.api.escort_feasibility(UTWIG_SHIP) != 0) {
            self.api
                .response(CAN_YOU_HELP, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_ALLIED_HOMEWORLD, Handler::ExitConversation, &[]);
    }
    pub(crate) fn before_kohr_ah(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_BEFORE_SPACE {
            num_visits = self.api.get(StoryKey::UTWIG_INFO);
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
            self.api.set(StoryKey::UTWIG_INFO, num_visits);
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
            num_visits = self.api.get(StoryKey::UTWIG_INFO);
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
            self.api.set(StoryKey::UTWIG_INFO, num_visits);
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
    pub(crate) fn neutral_utwig(&mut self, r: u16) {
        let mut fragments: Vec<i32> = Vec::new();
        let _i: u8;
        let mut last_stack: u8;
        let mut p_str: [u16; 4] = [0; 4];
        last_stack = 0;
        p_str[3_usize] = 0;
        p_str[2_usize] = p_str[3_usize];
        p_str[1_usize] = p_str[2_usize];
        p_str[0_usize] = p_str[1_usize];
        if r == WE_ARE_VINDICATOR0 {
            self.api.npc(i32::from(WOULD_BE_HAPPY_BUT));
            self.api.set(StoryKey::UTWIG_STACK1, 1_u8);
        } else if r == WHY_SAD {
            self.api.npc(i32::from(ULTRON_BROKE));
            self.api.set(StoryKey::UTWIG_STACK1, 2_u8);
        } else if r == WHAT_ULTRON {
            self.api.npc(i32::from(GLORIOUS_ULTRON));
            self.api.set(StoryKey::UTWIG_STACK1, 3_u8);
        } else if r == DONT_BE_BABIES {
            self.api.npc(i32::from(MOCK_OUR_PAIN));
            self.api.segue(1);
            self.api.set(StoryKey::UTWIG_STACK1, 4_u8);
            self.api.set(StoryKey::UTWIG_HOSTILE, 1_u8);
            self.api.set(StoryKey::UTWIG_INFO, 0_u8);
            self.api.set(StoryKey::UTWIG_HOME_VISITS, 0_u8);
            self.api.set(StoryKey::UTWIG_VISITS, 0_u8);
            self.api.set(StoryKey::BOMB_VISITS, 0_u8);
            return;
        } else if r == REAL_SORRY_ABOUT_ULTRON {
            self.api.npc(i32::from(APPRECIATE_SYMPATHY));
            self.api.set(StoryKey::UTWIG_STACK1, 4_u8);
            return;
        } else if r == WHAT_ABOUT_YOU_1 {
            self.api.npc(i32::from(ABOUT_US_1));
            last_stack = 2;
            self.api.set(StoryKey::UTWIG_WAR_NEWS, 1_u8);
        } else if r == WHAT_ABOUT_YOU_2 {
            self.api.npc(i32::from(ABOUT_US_2));
            last_stack = 2;
            self.api.track(SUPOX_SHIP);
            self.api.set(StoryKey::UTWIG_WAR_NEWS, 2_u8);
        } else if r == WHAT_ABOUT_YOU_3 {
            self.api.npc(i32::from(ABOUT_US_3));
            self.api.set(StoryKey::UTWIG_WAR_NEWS, 3_u8);
        } else if r == WHAT_ABOUT_URQUAN_1 {
            self.api.npc(i32::from(ABOUT_URQUAN_1));
            last_stack = 3;
            self.api.set(StoryKey::UTWIG_STACK2, 1_u8);
        } else if r == WHAT_ABOUT_URQUAN_2 {
            self.api.npc(i32::from(ABOUT_URQUAN_2));
            self.api.set(StoryKey::UTWIG_STACK2, 2_u8);
        }
        match self.api.get(StoryKey::UTWIG_STACK1) {
            0 => {
                {
                    fragments.clear();
                    fragments.push(WE_ARE_VINDICATOR0 as i32);
                    fragments.push(GLOBAL_PLAYER_NAME);
                    fragments.push(WE_ARE_VINDICATOR1 as i32);
                    fragments.extend(self.api.alliance_fragments(NAME_1));
                    fragments.push(WE_ARE_VINDICATOR2 as i32);
                }
                p_str[0_usize] = WE_ARE_VINDICATOR0;
            }
            1 => {
                p_str[0_usize] = WHY_SAD;
            }
            2 => {
                p_str[0_usize] = WHAT_ULTRON;
            }
            3 => {
                p_str[0_usize] = DONT_BE_BABIES;
                p_str[1_usize] = REAL_SORRY_ABOUT_ULTRON;
            }
            _ => {}
        }
        match self.api.get(StoryKey::UTWIG_WAR_NEWS) {
            0 => {
                p_str[2_usize] = WHAT_ABOUT_YOU_1;
            }
            1 => {
                p_str[2_usize] = WHAT_ABOUT_YOU_2;
            }
            2 => {
                p_str[2_usize] = WHAT_ABOUT_YOU_3;
            }
            _ => {}
        }
        match self.api.get(StoryKey::UTWIG_STACK2) {
            0 => {
                p_str[2_usize] = WHAT_ABOUT_URQUAN_1;
            }
            1 => {
                p_str[2_usize] = WHAT_ABOUT_URQUAN_2;
            }
            _ => {}
        }
        if p_str[last_stack as usize] != 0 {
            if p_str[last_stack as usize] != WE_ARE_VINDICATOR0 {
                self.api
                    .response(p_str[last_stack as usize], Handler::NeutralUtwig, &[]);
            } else {
                self.api.response(
                    p_str[last_stack as usize],
                    Handler::NeutralUtwig,
                    &fragments,
                );
            }
        }
        for i in 0..4 {
            if (i != last_stack) && p_str[i as usize] != 0 {
                if p_str[i as usize] != WE_ARE_VINDICATOR0 {
                    self.api
                        .response(p_str[i as usize], Handler::NeutralUtwig, &[]);
                } else {
                    self.api
                        .response(p_str[i as usize], Handler::NeutralUtwig, &fragments);
                }
            }
        }
        if self.api.get(StoryKey::ULTRON_CONDITION) != 0 {
            self.api
                .response(GOT_ULTRON, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_NEUTRAL, Handler::ExitConversation, &[]);
    }
    pub(crate) fn bomb_world(&mut self, r: u16) {
        let mut last_stack: u8;
        let mut p_str: [u16; 2] = [0; 2];
        last_stack = 0;
        p_str[1_usize] = 0;
        p_str[0_usize] = p_str[1_usize];
        if r == WHY_YOU_HERE {
            self.api.npc(i32::from(WE_GUARD_BOMB));
            self.api.set(StoryKey::BOMB_STACK1, 1_u8);
        } else if r == WHAT_ABOUT_BOMB {
            self.api.npc(i32::from(ABOUT_BOMB));
            self.api.set(StoryKey::BOMB_STACK1, 2_u8);
        } else if r == GIVE_US_BOMB_OR_DIE {
            self.api.npc(i32::from(GUARDS_WARN));
            self.api.set(StoryKey::BOMB_STACK1, 3_u8);
        } else if r == DEMAND_BOMB {
            self.api.npc(i32::from(GUARDS_FIGHT));
            self.api.segue(1);
            self.api.set(StoryKey::UTWIG_HOSTILE, 1_u8);
            self.api.set(StoryKey::UTWIG_INFO, 0_u8);
            self.api.set(StoryKey::UTWIG_HOME_VISITS, 0_u8);
            self.api.set(StoryKey::UTWIG_VISITS, 0_u8);
            self.api.set(StoryKey::BOMB_VISITS, 0_u8);
            return;
        } else if r == MAY_WE_HAVE_BOMB {
            self.api.npc(i32::from(NO_BOMB));
            last_stack = 1;
            self.api.set(StoryKey::BOMB_STACK2, 1_u8);
        } else if r == PLEASE {
            self.api.npc(i32::from(SORRY_NO_BOMB));
            self.api.set(StoryKey::BOMB_STACK2, 2_u8);
        } else if r == WHATS_UP_BOMB {
            if self.api.get(StoryKey::BOMB_INFO) != 0 {
                self.api.npc(i32::from(GENERAL_INFO_BOMB_2));
            } else {
                self.api.npc(i32::from(GENERAL_INFO_BOMB_1));
                self.api.set(StoryKey::BOMB_INFO, 1_u8);
            }
            self.api.disable(WHATS_UP_BOMB);
        }
        match self.api.get(StoryKey::BOMB_STACK2) {
            0 => p_str[1_usize] = MAY_WE_HAVE_BOMB,
            1 => p_str[1_usize] = PLEASE,
            _ => {}
        }
        match self.api.get(StoryKey::BOMB_STACK1) {
            0 => {
                p_str[0_usize] = WHY_YOU_HERE;
                p_str[1_usize] = 0;
            }
            1 => {
                p_str[0_usize] = WHAT_ABOUT_BOMB;
                p_str[1_usize] = 0;
            }
            2 => p_str[0_usize] = GIVE_US_BOMB_OR_DIE,
            3 => p_str[0_usize] = DEMAND_BOMB,
            _ => {}
        }
        if p_str[last_stack as usize] != 0 {
            self.api
                .response(p_str[last_stack as usize], Handler::BombWorld, &[]);
        }
        last_stack ^= 1;
        if p_str[last_stack as usize] != 0 {
            self.api
                .response(p_str[last_stack as usize], Handler::BombWorld, &[]);
        }
        if self.api.enabled(WHATS_UP_BOMB) && (self.api.get(StoryKey::BOMB_STACK1) > 1) {
            self.api.response(WHATS_UP_BOMB, Handler::BombWorld, &[]);
        }
        if self.api.get(StoryKey::ULTRON_CONDITION) != 0
            && !(self.api.get(StoryKey::REFUSED_ULTRON_AT_BOMB) != 0)
        {
            self.api
                .response(GOT_ULTRON, Handler::ExitConversation, &[]);
        }
        if self.api.get(StoryKey::BOMB_INFO) != 0 {
            self.api.response(BYE_BOMB, Handler::ExitConversation, &[]);
        } else {
            self.api
                .response(BYE_NEUTRAL, Handler::ExitConversation, &[]);
        }
    }
    pub(crate) fn intro(&mut self) {
        let mut num_visits: u8;
        if (self.api.activity() as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0);
            return;
        }
        if self.api.get(StoryKey::UTWIG_HOSTILE) != 0 {
            if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 6)) != 0 {
                num_visits = self.api.get(StoryKey::BOMB_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits += 1;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(HOSTILE_BOMB_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(HOSTILE_BOMB_HELLO_2));
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set(StoryKey::BOMB_VISITS, num_visits);
            } else if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::UTWIG_HOME_VISITS);
                let res = {
                    let old = num_visits;
                    num_visits += 1;
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(HOSTILE_HOMEWORLD_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(HOSTILE_HOMEWORLD_HELLO_2));
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set(StoryKey::UTWIG_HOME_VISITS, num_visits);
            } else {
                num_visits = self.api.get(StoryKey::UTWIG_VISITS);
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
                self.api.set(StoryKey::UTWIG_VISITS, num_visits);
            }
            if !(self.api.get(StoryKey::ULTRON_CONDITION) != 0)
                || (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 6)) != 0
            {
                self.api.segue(1);
            } else {
                self.api
                    .response(HEY_WAIT_GOT_ULTRON, Handler::ExitConversation, &[]);
            }
        } else if self.api.alliance(UTWIG_SHIP) == 1 {
            if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::UTWIG_HOME_VISITS);
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
                self.api.set(StoryKey::UTWIG_HOME_VISITS, num_visits);
                self.allied_home(0_u16);
            } else {
                num_visits = self.api.get(StoryKey::UTWIG_SUPOX_MISSION);
                if num_visits == 1 {
                    num_visits = self.api.get(StoryKey::UTWIG_VISITS);
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
                    self.api.set(StoryKey::UTWIG_VISITS, num_visits);
                    self.before_kohr_ah(0_u16);
                } else if num_visits < 5 {
                    num_visits = self.api.get(StoryKey::UTWIG_VISITS);
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
                    self.api.set(StoryKey::UTWIG_VISITS, num_visits);
                } else {
                    num_visits = self.api.get(StoryKey::UTWIG_VISITS);
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
                    self.api.set(StoryKey::UTWIG_VISITS, num_visits);
                    self.after_kohr_ah(0_u16);
                }
            }
        } else if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 6)) != 0 {
            num_visits = self.api.get(StoryKey::BOMB_VISITS);
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(BOMB_WORLD_HELLO_1));
                }
                1 => {
                    self.api.npc(i32::from(BOMB_WORLD_HELLO_2));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set(StoryKey::BOMB_VISITS, num_visits);
            self.bomb_world(0_u16);
        } else {
            if (self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 7)) != 0 {
                num_visits = self.api.get(StoryKey::UTWIG_HOME_VISITS);
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
                    }
                    2 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(NEUTRAL_HOMEWORLD_HELLO_4));
                        num_visits -= 1;
                    }
                    _ => {}
                }
                self.api.set(StoryKey::UTWIG_HOME_VISITS, num_visits);
            } else {
                num_visits = self.api.get(StoryKey::UTWIG_VISITS);
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
                self.api.set(StoryKey::UTWIG_VISITS, num_visits);
            }
            self.neutral_utwig(0_u16);
        }
    }
}

#[cfg(test)]
use crate::comm::descriptors::utwig::descriptor;

#[cfg(test)]
mod tests;
