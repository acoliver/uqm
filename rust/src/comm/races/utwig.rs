//! Utwig dialogue, ported from the original C script.
use super::alliance::AllianceApi;
use super::script::{animation, as_comm_data, GLOBAL_PLAYER_NAME};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use super::alliance::{SUPOX_SHIP, UTWIG_SHIP};
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    AlliedHome,
    ExitConversation,
    BeforeKohrAh,
    AfterKohrAh,
    NeutralUtwig,
    BombWorld,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: AllianceApi<Handler>> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::AlliedHome => self.allied_home(r),
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::BeforeKohrAh => self.before_kohr_ah(r),
            Handler::AfterKohrAh => self.after_kohr_ah(r),
            Handler::NeutralUtwig => self.neutral_utwig(r),
            Handler::BombWorld => self.bomb_world(r),
        }
    }
    fn initialize(&mut self, desc: &mut CLocData) {
        if self.api.get("UTWIG_HAVE_ULTRON") != 0 {
            desc.alien_alt_song_res = c"comm.utwig.ultron.music".as_ptr();
            desc.alien_song_flags |= 1;
        } else {
            desc.alien_song_flags &= !1;
        }
        self.api.segue(u32::from(
            self.api.get("UTWIG_HAVE_ULTRON") == 0 && self.api.activity() as u8 != 5,
        ));
    }
    fn exit_conversation(&mut self, r: u16) {
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
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0 {
                self.api.npc(i32::from(NO_ULTRON_AT_BOMB));
                self.api.set("REFUSED_ULTRON_AT_BOMB", 1_u8);
            } else {
                if r == GOT_ULTRON {
                    self.api.npc(i32::from(DONT_WANT_TO_LOOK));
                } else {
                    self.api.npc(i32::from(TAUNT_US_BUT_WE_LOOK));
                }
                if self.api.get("ULTRON_CONDITION") < 4 {
                    match self.api.get("UTWIG_INFO") {
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
                    self.api.set("UTWIG_INFO", 1_u8);
                } else {
                    self.api.npc(i32::from(HAPPY_DAYS));
                    if self.api.get("KOHR_AH_FRENZY") != 0 {
                        self.api.npc(i32::from(TOO_LATE));
                    } else {
                        self.api.npc(i32::from(OK_ATTACK_KOHRAH));
                        self.api.event(1, 0, 0, 0, 9);
                    }
                    self.api.set("UTWIG_HAVE_ULTRON", 1_u8);
                    self.api.set("ULTRON_CONDITION", 5_u8);
                    self.api.set("UTWIG_VISITS", 0_u8);
                    self.api.set("SUPOX_VISITS", 0_u8);
                    self.api.set("UTWIG_HOME_VISITS", 0_u8);
                    self.api.set("SUPOX_HOME_VISITS", 0_u8);
                    self.api.set("BOMB_VISITS", 0_u8);
                    self.api.set("SUPOX_INFO", 0_u8);
                    self.api.set("UTWIG_INFO", 0_u8);
                    self.api.set("SUPOX_WAR_NEWS", 0_u8);
                    self.api.set("UTWIG_WAR_NEWS", 0_u8);
                    self.api.set("SUPOX_HOSTILE", 0_u8);
                    self.api.set("UTWIG_HOSTILE", 0_u8);
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
    fn allied_home(&mut self, r: u16) {
        let mut news: u8;
        news = self.api.get("UTWIG_WAR_NEWS");
        let num_visits: u8 = self.api.get("UTWIG_SUPOX_MISSION");
        if r == HOW_WENT_WAR {
            self.api.npc(i32::from(ABOUT_BATTLE));
            news |= 1 << 0;
        } else if r == HOW_GOES_WAR {
            if num_visits == 1 {
                self.api.npc(i32::from(FLEET_ON_WAY));
                self.api.set("UTWIG_WAR_NEWS", 1_u8);
            } else {
                match self.api.get("UTWIG_WAR_NEWS") {
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
        self.api.set("UTWIG_WAR_NEWS", news);
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
    fn before_kohr_ah(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_BEFORE_SPACE {
            num_visits = self.api.get("UTWIG_INFO");
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
            self.api.set("UTWIG_INFO", num_visits);
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
    fn after_kohr_ah(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_AFTER_SPACE {
            num_visits = self.api.get("UTWIG_INFO");
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
            self.api.set("UTWIG_INFO", num_visits);
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
    fn neutral_utwig(&mut self, r: u16) {
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
            self.api.set("UTWIG_STACK1", 1_u8);
        } else if r == WHY_SAD {
            self.api.npc(i32::from(ULTRON_BROKE));
            self.api.set("UTWIG_STACK1", 2_u8);
        } else if r == WHAT_ULTRON {
            self.api.npc(i32::from(GLORIOUS_ULTRON));
            self.api.set("UTWIG_STACK1", 3_u8);
        } else if r == DONT_BE_BABIES {
            self.api.npc(i32::from(MOCK_OUR_PAIN));
            self.api.segue(1);
            self.api.set("UTWIG_STACK1", 4_u8);
            self.api.set("UTWIG_HOSTILE", 1_u8);
            self.api.set("UTWIG_INFO", 0_u8);
            self.api.set("UTWIG_HOME_VISITS", 0_u8);
            self.api.set("UTWIG_VISITS", 0_u8);
            self.api.set("BOMB_VISITS", 0_u8);
            return;
        } else if r == REAL_SORRY_ABOUT_ULTRON {
            self.api.npc(i32::from(APPRECIATE_SYMPATHY));
            self.api.set("UTWIG_STACK1", 4_u8);
            return;
        } else if r == WHAT_ABOUT_YOU_1 {
            self.api.npc(i32::from(ABOUT_US_1));
            last_stack = 2;
            self.api.set("UTWIG_WAR_NEWS", 1_u8);
        } else if r == WHAT_ABOUT_YOU_2 {
            self.api.npc(i32::from(ABOUT_US_2));
            last_stack = 2;
            self.api.track(SUPOX_SHIP);
            self.api.set("UTWIG_WAR_NEWS", 2_u8);
        } else if r == WHAT_ABOUT_YOU_3 {
            self.api.npc(i32::from(ABOUT_US_3));
            self.api.set("UTWIG_WAR_NEWS", 3_u8);
        } else if r == WHAT_ABOUT_URQUAN_1 {
            self.api.npc(i32::from(ABOUT_URQUAN_1));
            last_stack = 3;
            self.api.set("UTWIG_STACK2", 1_u8);
        } else if r == WHAT_ABOUT_URQUAN_2 {
            self.api.npc(i32::from(ABOUT_URQUAN_2));
            self.api.set("UTWIG_STACK2", 2_u8);
        }
        match self.api.get("UTWIG_STACK1") {
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
        match self.api.get("UTWIG_WAR_NEWS") {
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
        match self.api.get("UTWIG_STACK2") {
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
        if self.api.get("ULTRON_CONDITION") != 0 {
            self.api
                .response(GOT_ULTRON, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_NEUTRAL, Handler::ExitConversation, &[]);
    }
    fn bomb_world(&mut self, r: u16) {
        let mut last_stack: u8;
        let mut p_str: [u16; 2] = [0; 2];
        last_stack = 0;
        p_str[1_usize] = 0;
        p_str[0_usize] = p_str[1_usize];
        if r == WHY_YOU_HERE {
            self.api.npc(i32::from(WE_GUARD_BOMB));
            self.api.set("BOMB_STACK1", 1_u8);
        } else if r == WHAT_ABOUT_BOMB {
            self.api.npc(i32::from(ABOUT_BOMB));
            self.api.set("BOMB_STACK1", 2_u8);
        } else if r == GIVE_US_BOMB_OR_DIE {
            self.api.npc(i32::from(GUARDS_WARN));
            self.api.set("BOMB_STACK1", 3_u8);
        } else if r == DEMAND_BOMB {
            self.api.npc(i32::from(GUARDS_FIGHT));
            self.api.segue(1);
            self.api.set("UTWIG_HOSTILE", 1_u8);
            self.api.set("UTWIG_INFO", 0_u8);
            self.api.set("UTWIG_HOME_VISITS", 0_u8);
            self.api.set("UTWIG_VISITS", 0_u8);
            self.api.set("BOMB_VISITS", 0_u8);
            return;
        } else if r == MAY_WE_HAVE_BOMB {
            self.api.npc(i32::from(NO_BOMB));
            last_stack = 1;
            self.api.set("BOMB_STACK2", 1_u8);
        } else if r == PLEASE {
            self.api.npc(i32::from(SORRY_NO_BOMB));
            self.api.set("BOMB_STACK2", 2_u8);
        } else if r == WHATS_UP_BOMB {
            if self.api.get("BOMB_INFO") != 0 {
                self.api.npc(i32::from(GENERAL_INFO_BOMB_2));
            } else {
                self.api.npc(i32::from(GENERAL_INFO_BOMB_1));
                self.api.set("BOMB_INFO", 1_u8);
            }
            self.api.disable(WHATS_UP_BOMB);
        }
        match self.api.get("BOMB_STACK2") {
            0 => {
                p_str[1_usize] = MAY_WE_HAVE_BOMB;
            }
            1 => {
                p_str[1_usize] = PLEASE;
            }
            _ => {}
        }
        match self.api.get("BOMB_STACK1") {
            0 => {
                p_str[0_usize] = WHY_YOU_HERE;
                p_str[1_usize] = 0;
            }
            1 => {
                p_str[0_usize] = WHAT_ABOUT_BOMB;
                p_str[1_usize] = 0;
            }
            2 => {
                p_str[0_usize] = GIVE_US_BOMB_OR_DIE;
            }
            3 => {
                p_str[0_usize] = DEMAND_BOMB;
            }
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
        if self.api.enabled(WHATS_UP_BOMB) && (self.api.get("BOMB_STACK1") > 1) {
            self.api.response(WHATS_UP_BOMB, Handler::BombWorld, &[]);
        }
        if self.api.get("ULTRON_CONDITION") != 0 && !(self.api.get("REFUSED_ULTRON_AT_BOMB") != 0) {
            self.api
                .response(GOT_ULTRON, Handler::ExitConversation, &[]);
        }
        if self.api.get("BOMB_INFO") != 0 {
            self.api.response(BYE_BOMB, Handler::ExitConversation, &[]);
        } else {
            self.api
                .response(BYE_NEUTRAL, Handler::ExitConversation, &[]);
        }
    }
    fn intro(&mut self) {
        let mut num_visits: u8;
        if (self.api.activity() as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0);
            return;
        }
        if self.api.get("UTWIG_HOSTILE") != 0 {
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0 {
                num_visits = self.api.get("BOMB_VISITS");
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
                self.api.set("BOMB_VISITS", num_visits);
            } else if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                num_visits = self.api.get("UTWIG_HOME_VISITS");
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
                self.api.set("UTWIG_HOME_VISITS", num_visits);
            } else {
                num_visits = self.api.get("UTWIG_VISITS");
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
                self.api.set("UTWIG_VISITS", num_visits);
            }
            if !(self.api.get("ULTRON_CONDITION") != 0)
                || (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0
            {
                self.api.segue(1);
            } else {
                self.api
                    .response(HEY_WAIT_GOT_ULTRON, Handler::ExitConversation, &[]);
            }
        } else if self.api.alliance(UTWIG_SHIP) == 1 {
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                num_visits = self.api.get("UTWIG_HOME_VISITS");
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
                self.api.set("UTWIG_HOME_VISITS", num_visits);
                self.allied_home(0_u16);
            } else {
                num_visits = self.api.get("UTWIG_SUPOX_MISSION");
                if num_visits == 1 {
                    num_visits = self.api.get("UTWIG_VISITS");
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
                    self.api.set("UTWIG_VISITS", num_visits);
                    self.before_kohr_ah(0_u16);
                } else if num_visits < 5 {
                    num_visits = self.api.get("UTWIG_VISITS");
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
                    self.api.set("UTWIG_VISITS", num_visits);
                } else {
                    num_visits = self.api.get("UTWIG_VISITS");
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
                    self.api.set("UTWIG_VISITS", num_visits);
                    self.after_kohr_ah(0_u16);
                }
            }
        } else if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0 {
            num_visits = self.api.get("BOMB_VISITS");
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
            self.api.set("BOMB_VISITS", num_visits);
            self.bomb_world(0_u16);
        } else {
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                num_visits = self.api.get("UTWIG_HOME_VISITS");
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
                self.api.set("UTWIG_HOME_VISITS", num_visits);
            } else {
                num_visits = self.api.get("UTWIG_VISITS");
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
                self.api.set("UTWIG_VISITS", num_visits);
            }
            self.neutral_utwig(0_u16);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.utwig.graphics".as_ptr(),
        alien_font_res: c"comm.utwig.font".as_ptr(),
        alien_colormap_res: c"comm.utwig.colortable".as_ptr(),
        alien_song_res: c"comm.utwig.music".as_ptr(),
        conversation_phrases_res: c"comm.utwig.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 121, y: 70 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 16,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(4, 3, 4, (56, 28), (840, 2520));
    d.alien_ambient_array[1] = animation(7, 4, 4, (56, 28), (8400, 2520));
    d.alien_ambient_array[1].block_mask = 4;
    d.alien_ambient_array[2] = animation(11, 2, 12, (112, 28), (8400, 2520));
    d.alien_ambient_array[2].block_mask = 2;
    d.alien_ambient_array[3] = animation(13, 5, 2, (70, 0), (70, 0));
    d.alien_ambient_array[4] = animation(18, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[5] = animation(20, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[6] = animation(22, 3, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[7] = animation(25, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[8] = animation(27, 3, 4, (112, 28), (8400, 2520));
    d.alien_ambient_array[9] = animation(30, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[10] = animation(32, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[11] = animation(34, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[12] = animation(36, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[13] = animation(38, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[14] = animation(40, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[15] = animation(42, 3, 4, (112, 28), (8400, 2520));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (42, 42), (98, 420));
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
    use super::*;
    use crate::comm::races::script::live::LiveScript;

    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    fn allied_home(r: u32) {
        run(|s| {
            s.handle(
                Handler::AlliedHome,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn before_kohr_ah(r: u32) {
        run(|s| {
            s.handle(
                Handler::BeforeKohrAh,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn after_kohr_ah(r: u32) {
        run(|s| {
            s.handle(
                Handler::AfterKohrAh,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn neutral_utwig(r: u32) {
        run(|s| {
            s.handle(
                Handler::NeutralUtwig,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn bomb_world(r: u32) {
        run(|s| {
            s.handle(
                Handler::BombWorld,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }

    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::AlliedHome => allied_home,
            Handler::ExitConversation => exit_conversation,
            Handler::BeforeKohrAh => before_kohr_ah,
            Handler::AfterKohrAh => after_kohr_ah,
            Handler::NeutralUtwig => neutral_utwig,
            Handler::BombWorld => bomb_world,
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
    pub unsafe extern "C" fn init_utwig_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct UtwigDialogue;
impl super::RaceDialogue for UtwigDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_utwig_comm()) }
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
