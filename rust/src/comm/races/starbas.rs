//! Starbase and Hayes dialogue, ported from the original C script.

use super::encounter::EncounterApi;
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use super::script::{GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
use phrases::*;
mod api;
use api::*;
const SPATHI_SHIP: u16 = 6;
const GOOD_GUY: u16 = 1;

/// Resource units awarded when Hayes brings the Starbase online.
pub const STARBASE_READY_REWARD: u16 = 2500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ByeBye,
    AllianceInfo,
    HierarchyInfo,
    AlienRaces,
    WarInfo,
    AncientHistory,
    TellHistory,
    DefeatUrquan,
    TellMission,
    TellStarBase,
    NeedInfo,
    NormalStarbase,
    SellMinerals,
}
#[derive(Default)]
struct Session {
    bulletins: u32,
    stacks: [u8; 4],
    alliance: u8,
    hierarchy: u8,
    races: u8,
    war: u8,
    ancient: u8,
    defeat: u8,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
    session: &'a mut Session,
}
impl<A: StarbaseApi> Dialogue<'_, A> {
    fn handle(&mut self, handler: Handler, r: u16) {
        match handler {
            Handler::ByeBye => self.bye_bye(r),
            Handler::AllianceInfo => self.alliance_info(r),
            Handler::HierarchyInfo => self.hierarchy_info(r),
            Handler::AlienRaces => self.alien_races(r),
            Handler::WarInfo => self.war_info(r),
            Handler::AncientHistory => self.ancient_history(r),
            Handler::TellHistory => self.tell_history(r),
            Handler::DefeatUrquan => self.defeat_urquan(r),
            Handler::TellMission => self.tell_mission(r),
            Handler::TellStarBase => self.tell_star_base(r),
            Handler::NeedInfo => self.need_info(r),
            Handler::NormalStarbase => self.normal_starbase(r),
            Handler::SellMinerals => self.sell_minerals(r),
        }
    }
    fn get32(&self, first: &str) -> u32 {
        let prefix = first
            .strip_suffix('0')
            .expect("32-bit game-state key must end in 0");
        u32::from_le_bytes(std::array::from_fn(|i| {
            self.api.get(&format!("{prefix}{i}"))
        }))
    }

    fn set32(&mut self, first: &str, value: u32) {
        let prefix = first
            .strip_suffix('0')
            .expect("32-bit game-state key must end in 0");
        for (i, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.api.set(&format!("{prefix}{i}"), byte);
        }
    }

    fn check_timing(&mut self, mut month_index: u16, mut day_index: u16) -> bool {
        let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut mi = u16::from(self.api.get("STARBASE_MONTH"));
        let mut year_index = 2155;
        day_index += u16::from(self.api.get("STARBASE_DAY"));
        while day_index > days_in_month[usize::from(mi - 1)] {
            day_index -= days_in_month[usize::from(mi - 1)];
            mi += 1;
            if mi > 12 {
                mi = 1;
                year_index += 1;
            }
        }
        month_index += mi;
        year_index += (month_index - 1) / 12;
        month_index = ((month_index - 1) % 12) + 1;
        let now = self.api.date();
        (year_index, month_index, day_index) < (now.year, u16::from(now.month), u16::from(now.day))
    }

    fn tell_star_base(&mut self, r: u16) {
        match r {
            STARBASE_FUNCTIONS => {
                self.api.npc(i32::from(WHICH_FUNCTION));
                self.session.stacks = [0; 4];
            }
            TELL_ME_ABOUT_FUEL0 => {
                self.api.npc(i32::from(ABOUT_FUEL));
                self.session.stacks[1] = 1;
            }
            TELL_ME_ABOUT_CREW => {
                self.api.npc(i32::from(ABOUT_CREW0));
                if self.api.using_speech() {
                    self.api.npc(i32::from(YOUR_FLAGSHIP_3DO2));
                } else {
                    self.api.npc(i32::from(YOUR_FLAGSHIP_PC));
                    self.api.npc(GLOBAL_SHIP_NAME);
                }
                self.api.npc(i32::from(ABOUT_CREW1));
                self.session.stacks[2] = 2;
            }
            TELL_ME_ABOUT_MODULES0 => {
                self.api.npc(i32::from(ABOUT_MODULES));
                self.session.stacks[0] = 1;
            }
            TELL_ME_ABOUT_SHIPS => {
                self.api.npc(i32::from(ABOUT_SHIPS));
                self.session.stacks[2] = 1;
            }
            TELL_ME_ABOUT_RU => {
                self.api.npc(i32::from(ABOUT_RU));
                self.session.stacks[3] = 1;
            }
            TELL_ME_ABOUT_MINERALS => {
                self.api.npc(i32::from(ABOUT_MINERALS));
                self.session.stacks[3] = 2;
            }
            TELL_ME_ABOUT_LIFE => {
                self.api.npc(i32::from(ABOUT_LIFE));
                self.session.stacks[3] = 3;
            }
            _ => {}
        }
        if self.session.stacks[0] == 0 {
            self.api.response(
                TELL_ME_ABOUT_MODULES0,
                Handler::TellStarBase,
                &[
                    i32::from(TELL_ME_ABOUT_MODULES0),
                    GLOBAL_SHIP_NAME,
                    i32::from(TELL_ME_ABOUT_MODULES1),
                ],
            );
        }
        if self.session.stacks[1] == 0 {
            self.api.response(
                TELL_ME_ABOUT_FUEL0,
                Handler::TellStarBase,
                &[
                    i32::from(TELL_ME_ABOUT_FUEL0),
                    GLOBAL_SHIP_NAME,
                    i32::from(TELL_ME_ABOUT_FUEL1),
                ],
            );
        }
        match self.session.stacks[2] {
            0 => self
                .api
                .response(TELL_ME_ABOUT_SHIPS, Handler::TellStarBase, &[]),
            1 => self
                .api
                .response(TELL_ME_ABOUT_CREW, Handler::TellStarBase, &[]),
            _ => {}
        }
        match self.session.stacks[3] {
            0 => self
                .api
                .response(TELL_ME_ABOUT_RU, Handler::TellStarBase, &[]),
            1 => self
                .api
                .response(TELL_ME_ABOUT_MINERALS, Handler::TellStarBase, &[]),
            2 => self
                .api
                .response(TELL_ME_ABOUT_LIFE, Handler::TellStarBase, &[]),
            _ => {}
        }
        self.api.response(ENOUGH_STARBASE, Handler::NeedInfo, &[]);
    }

    fn sell_minerals(&mut self, r: u16) {
        let mut total = 0u16;
        let mut sleepy = true;
        for i in 0..8 {
            let mut time_in = 0;
            if i == 0 {
                self.api.draw_cargo(u8::MAX, u8::MAX);
                self.api.sleep(420);
                time_in = self.api.now();
                self.api.draw_cargo(0, 0);
            } else if sleepy {
                self.api.draw_cargo((i - 1) as u8, i as u8);
                time_in = self.api.now();
            }
            let mut amount = self.api.mineral_amount(i);
            if amount != 0 {
                total =
                    total.wrapping_add(amount.wrapping_mul(u16::from(self.api.mineral_worth(i))));
                loop {
                    if !sleepy || self.api.skip_sale() {
                        sleepy = false;
                        self.api.remove_minerals(i, amount);
                        self.api.gauges(
                            0,
                            0,
                            i32::from(amount) * i32::from(self.api.mineral_worth(i)),
                        );
                        break;
                    }
                    self.api.remove_minerals(i, 1);
                    self.api.task_switch();
                    time_in = self.api.now();
                    self.api.draw_cargo(i as u8, i as u8);
                    self.api.show_capacity();
                    self.api.gauges(0, 0, i32::from(self.api.mineral_worth(i)));
                    amount -= 1;
                    if amount == 0 {
                        break;
                    }
                }
            }
            if sleepy {
                self.api.sleep_until(time_in.wrapping_add(210));
                let _ = self.api.now();
            }
        }
        self.api.sleep(420);
        self.api.clear_display();
        let (key, pairs) = if total < 1000 {
            (
                "LIGHT_MINERAL_LOAD",
                [
                    (LIGHT_LOAD_A0, LIGHT_LOAD_A1),
                    (LIGHT_LOAD_B0, LIGHT_LOAD_B1),
                    (LIGHT_LOAD_C0, LIGHT_LOAD_C1),
                    (LIGHT_LOAD_D0, LIGHT_LOAD_D1),
                    (LIGHT_LOAD_E0, LIGHT_LOAD_E1),
                    (LIGHT_LOAD_F0, LIGHT_LOAD_F1),
                    (LIGHT_LOAD_G0, LIGHT_LOAD_G1),
                ],
            )
        } else if total < 2500 {
            (
                "MEDIUM_MINERAL_LOAD",
                [
                    (MEDIUM_LOAD_A0, MEDIUM_LOAD_A1),
                    (MEDIUM_LOAD_B0, MEDIUM_LOAD_B1),
                    (MEDIUM_LOAD_C0, MEDIUM_LOAD_C1),
                    (MEDIUM_LOAD_D0, MEDIUM_LOAD_D1),
                    (MEDIUM_LOAD_E0, MEDIUM_LOAD_E1),
                    (MEDIUM_LOAD_F0, MEDIUM_LOAD_F1),
                    (MEDIUM_LOAD_G0, MEDIUM_LOAD_G1),
                ],
            )
        } else {
            (
                "HEAVY_MINERAL_LOAD",
                [
                    (HEAVY_LOAD_A0, HEAVY_LOAD_A1),
                    (HEAVY_LOAD_B0, HEAVY_LOAD_B1),
                    (HEAVY_LOAD_C0, HEAVY_LOAD_C1),
                    (HEAVY_LOAD_D0, HEAVY_LOAD_D1),
                    (HEAVY_LOAD_E0, HEAVY_LOAD_E1),
                    (HEAVY_LOAD_F0, HEAVY_LOAD_F1),
                    (HEAVY_LOAD_G0, HEAVY_LOAD_G1),
                ],
            )
        };
        let visit = self.api.get(key);
        let (p1, p2) = pairs.get(usize::from(visit)).copied().unwrap_or((0, 0));
        self.api
            .set(key, if visit == 6 { 6 } else { visit.wrapping_add(1) });
        self.api.npc(i32::from(p1));
        if !self.api.using_speech() {
            self.api.npc(i32::from(SPACE));
            self.api.npc(GLOBAL_PLAYER_NAME);
        }
        self.api.npc(i32::from(p2));
        self.normal_starbase(r);
    }

    fn initialize(&mut self, desc: &mut CLocData) {
        desc.alien_alt_song_res = c"comm.starbase.music".as_ptr();
        desc.alien_song_flags |= 1;
        self.session.bulletins = 0;
        self.api.segue(0);
    }
    fn bye_bye(&mut self, _r: u16) {
        self.session.bulletins |= self.get32("STARBASE_BULLETS0");
        self.set32("STARBASE_BULLETS0", self.session.bulletins);
        if self.api.get("CHMMR_BOMB_STATE") >= 2 {
            self.api.npc(i32::from(GOOD_LUCK_AGAIN));
        } else {
            let mut p_str0: u16 = 0;
            let mut p_str1: u16 = 0;
            match (self.api.random() as u8) & 7 {
                0 => {
                    p_str0 = NORMAL_GOODBYE_A0;
                    p_str1 = NORMAL_GOODBYE_A1;
                }
                1 => {
                    p_str0 = NORMAL_GOODBYE_B0;
                    p_str1 = NORMAL_GOODBYE_B1;
                }
                2 => {
                    p_str0 = NORMAL_GOODBYE_C0;
                    p_str1 = NORMAL_GOODBYE_C1;
                }
                3 => {
                    p_str0 = NORMAL_GOODBYE_D0;
                    p_str1 = NORMAL_GOODBYE_D1;
                }
                4 => {
                    p_str0 = NORMAL_GOODBYE_E0;
                    p_str1 = NORMAL_GOODBYE_E1;
                }
                5 => {
                    p_str0 = NORMAL_GOODBYE_F0;
                    p_str1 = NORMAL_GOODBYE_F1;
                }
                6 => {
                    p_str0 = NORMAL_GOODBYE_G0;
                    p_str1 = NORMAL_GOODBYE_G1;
                }
                7 => {
                    p_str0 = NORMAL_GOODBYE_H0;
                    p_str1 = NORMAL_GOODBYE_H1;
                }
                _ => {}
            }
            self.api.npc(i32::from(p_str0));
            if !(self.api.using_speech()) {
                self.api.npc(i32::from(SPACE));
                self.api.npc(GLOBAL_PLAYER_NAME);
            }
            self.api.npc(i32::from(p_str1));
        }
    }
    fn alliance_info(&mut self, r: u16) {
        if r == WHAT_ABOUT_ALLIANCE {
            self.api.npc(i32::from(WHICH_ALLIANCE));
            self.session.alliance = 0;
        } else if r == SHOFIXTI {
            self.api.npc(i32::from(ABOUT_SHOFIXTI));
            self.session.alliance |= ALLIANCE_SHOFIXTI;
        } else if r == YEHAT {
            self.api.npc(i32::from(ABOUT_YEHAT));
            self.session.alliance |= ALLIANCE_YEHAT;
        } else if r == ARILOU {
            self.api.npc(i32::from(ABOUT_ARILOU));
            self.session.alliance |= ALLIANCE_ARILOU;
        } else if r == CHENJESU {
            self.api.npc(i32::from(ABOUT_CHENJESU));
            self.session.alliance |= ALLIANCE_CHENJESU;
        } else if r == MMRNMHRM {
            self.api.npc(i32::from(ABOUT_MMRNMHRM));
            self.session.alliance |= ALLIANCE_MMRNMHRM;
        } else if r == SYREEN {
            self.api.npc(i32::from(ABOUT_SYREEN));
            self.session.alliance |= ALLIANCE_SYREEN;
        }
        if !((self.session.alliance & ALLIANCE_SHOFIXTI) != 0) {
            self.api.response(SHOFIXTI, Handler::AllianceInfo, &[]);
        }
        if !((self.session.alliance & ALLIANCE_YEHAT) != 0) {
            self.api.response(YEHAT, Handler::AllianceInfo, &[]);
        }
        if !((self.session.alliance & ALLIANCE_ARILOU) != 0) {
            self.api.response(ARILOU, Handler::AllianceInfo, &[]);
        }
        if !((self.session.alliance & ALLIANCE_CHENJESU) != 0) {
            self.api.response(CHENJESU, Handler::AllianceInfo, &[]);
        }
        if !((self.session.alliance & ALLIANCE_MMRNMHRM) != 0) {
            self.api.response(MMRNMHRM, Handler::AllianceInfo, &[]);
        }
        if !((self.session.alliance & ALLIANCE_SYREEN) != 0) {
            self.api.response(SYREEN, Handler::AllianceInfo, &[]);
        }
        self.api.response(ENOUGH_ALLIANCE, Handler::AlienRaces, &[]);
    }
    fn hierarchy_info(&mut self, r: u16) {
        if r == WHAT_ABOUT_HIERARCHY {
            self.api.npc(i32::from(WHICH_HIERARCHY));
            self.session.hierarchy = 0;
        } else if r == URQUAN {
            self.api.npc(i32::from(ABOUT_URQUAN));
            self.session.hierarchy |= HIERARCHY_URQUAN;
        } else if r == MYCON {
            self.api.npc(i32::from(ABOUT_MYCON));
            self.session.hierarchy |= HIERARCHY_MYCON;
        } else if r == SPATHI {
            self.api.npc(i32::from(ABOUT_SPATHI));
            self.session.hierarchy |= HIERARCHY_SPATHI;
        } else if r == UMGAH {
            self.api.npc(i32::from(ABOUT_UMGAH));
            self.session.hierarchy |= HIERARCHY_UMGAH;
        } else if r == ANDROSYNTH {
            self.api.npc(i32::from(ABOUT_ANDROSYNTH));
            self.session.hierarchy |= HIERARCHY_ANDROSYNTH;
        } else if r == ILWRATH {
            self.api.npc(i32::from(ABOUT_ILWRATH));
            self.session.hierarchy |= HIERARCHY_ILWRATH;
        } else if r == VUX {
            self.api.npc(i32::from(ABOUT_VUX));
            self.session.hierarchy |= HIERARCHY_VUX;
        }
        if !((self.session.hierarchy & HIERARCHY_URQUAN) != 0) {
            self.api.response(URQUAN, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.hierarchy & HIERARCHY_MYCON) != 0) {
            self.api.response(MYCON, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.hierarchy & HIERARCHY_SPATHI) != 0) {
            self.api.response(SPATHI, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.hierarchy & HIERARCHY_UMGAH) != 0) {
            self.api.response(UMGAH, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.hierarchy & HIERARCHY_ANDROSYNTH) != 0) {
            self.api.response(ANDROSYNTH, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.hierarchy & HIERARCHY_ILWRATH) != 0) {
            self.api.response(ILWRATH, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.hierarchy & HIERARCHY_VUX) != 0) {
            self.api.response(VUX, Handler::HierarchyInfo, &[]);
        }
        self.api
            .response(ENOUGH_HIERARCHY, Handler::AlienRaces, &[]);
    }
    fn alien_races(&mut self, r: u16) {
        if r == ALIEN_RACES {
            self.api.npc(i32::from(WHICH_ALIEN));
            self.session.races = 0;
        } else if r == ENOUGH_ALLIANCE {
            self.api.npc(i32::from(OK_ENOUGH_ALLIANCE));
            self.session.races |= RACES_ALLIANCE;
        } else if r == ENOUGH_HIERARCHY {
            self.api.npc(i32::from(OK_ENOUGH_HIERARCHY));
            self.session.races |= RACES_HIERARCHY;
        } else if r == WHAT_ABOUT_OTHER {
            self.api.npc(i32::from(ABOUT_OTHER));
            self.session.races |= RACES_OTHER;
        }
        if !((self.session.races & RACES_ALLIANCE) != 0) {
            self.api
                .response(WHAT_ABOUT_ALLIANCE, Handler::AllianceInfo, &[]);
        }
        if !((self.session.races & RACES_HIERARCHY) != 0) {
            self.api
                .response(WHAT_ABOUT_HIERARCHY, Handler::HierarchyInfo, &[]);
        }
        if !((self.session.races & RACES_OTHER) != 0) {
            self.api
                .response(WHAT_ABOUT_OTHER, Handler::AlienRaces, &[]);
        }
        self.api.response(ENOUGH_ALIENS, Handler::TellHistory, &[]);
    }
    fn war_info(&mut self, r: u16) {
        if r == THE_WAR {
            self.api.npc(i32::from(WHICH_WAR));
            self.session.war = 0;
        } else if r == WHAT_STARTED_WAR {
            self.api.npc(i32::from(URQUAN_STARTED_WAR));
            self.session.war |= WAR_STARTED;
        } else if r == WHAT_WAS_WAR_LIKE {
            self.api.npc(i32::from(WAR_WAS_LIKE_SO));
            self.session.war |= WAR_WAS_LIKE;
        } else if r == WHY_LOSE_WAR {
            self.api.npc(i32::from(LOST_WAR_BECAUSE));
            self.session.war |= WAR_LOST;
        } else if r == WHAT_AFTER_WAR {
            self.api.npc(i32::from(AFTER_WAR));
            self.session.war |= WAR_AFTERMATH;
        }
        if !((self.session.war & WAR_STARTED) != 0) {
            self.api.response(WHAT_STARTED_WAR, Handler::WarInfo, &[]);
        }
        if !((self.session.war & WAR_WAS_LIKE) != 0) {
            self.api.response(WHAT_WAS_WAR_LIKE, Handler::WarInfo, &[]);
        }
        if !((self.session.war & WAR_LOST) != 0) {
            self.api.response(WHY_LOSE_WAR, Handler::WarInfo, &[]);
        }
        if !((self.session.war & WAR_AFTERMATH) != 0) {
            self.api.response(WHAT_AFTER_WAR, Handler::WarInfo, &[]);
        }
        self.api.response(ENOUGH_WAR, Handler::TellHistory, &[]);
    }
    fn ancient_history(&mut self, r: u16) {
        if r == ANCIENT_HISTORY {
            self.api.npc(i32::from(WHICH_ANCIENT));
            self.session.ancient = 0;
        } else if r == PRECURSORS {
            self.api.npc(i32::from(ABOUT_PRECURSORS));
            self.session.ancient |= ANCIENT_PRECURSORS;
        } else if r == OLD_RACES {
            self.api.npc(i32::from(ABOUT_OLD_RACES));
            self.session.ancient |= ANCIENT_RACES;
        } else if r == ALIENS_ON_EARTH {
            self.api.npc(i32::from(ABOUT_ALIENS_ON_EARTH));
            self.session.ancient |= ANCIENT_EARTH;
        }
        if !((self.session.ancient & ANCIENT_PRECURSORS) != 0) {
            self.api.response(PRECURSORS, Handler::AncientHistory, &[]);
        }
        if !((self.session.ancient & ANCIENT_RACES) != 0) {
            self.api.response(OLD_RACES, Handler::AncientHistory, &[]);
        }
        if !((self.session.ancient & ANCIENT_EARTH) != 0) {
            self.api
                .response(ALIENS_ON_EARTH, Handler::AncientHistory, &[]);
        }
        self.api.response(ENOUGH_ANCIENT, Handler::TellHistory, &[]);
    }
    fn tell_history(&mut self, r: u16) {
        let mut pstack: [u16; 3] = [0; 3];
        if r == HISTORY {
            self.api.npc(i32::from(WHICH_HISTORY));
            self.session.stacks[0] = 0;
            self.session.stacks[1] = 0;
            self.session.stacks[2] = 0;
        } else if r == ENOUGH_ALIENS {
            self.api.npc(i32::from(OK_ENOUGH_ALIENS));
            self.session.stacks[0] = 1;
        } else if r == ENOUGH_WAR {
            self.api.npc(i32::from(OK_ENOUGH_WAR));
            self.session.stacks[1] = 1;
        } else if r == ENOUGH_ANCIENT {
            self.api.npc(i32::from(OK_ENOUGH_ANCIENT));
            self.session.stacks[2] = 1;
        }
        match self.session.stacks[0] {
            0 => {
                pstack[0] = ALIEN_RACES;
            }
            _ => {
                pstack[0] = 0;
            }
        }
        match self.session.stacks[1] {
            0 => {
                pstack[1] = THE_WAR;
            }
            _ => {
                pstack[1] = 0;
            }
        }
        match self.session.stacks[2] {
            0 => {
                pstack[2] = ANCIENT_HISTORY;
            }
            _ => {
                pstack[2] = 0;
            }
        }
        if pstack[0] != 0 {
            self.api.response(pstack[0], Handler::AlienRaces, &[]);
        }
        if pstack[1] != 0 {
            self.api.response(pstack[1], Handler::WarInfo, &[]);
        }
        if pstack[2] != 0 {
            self.api.response(pstack[2], Handler::AncientHistory, &[]);
        }
        self.api.response(ENOUGH_HISTORY, Handler::NeedInfo, &[]);
    }
    fn defeat_urquan(&mut self, r: u16) {
        if r == HOW_DEFEAT {
            self.api.npc(i32::from(DEFEAT_LIKE_SO));
            self.session.defeat = 0;
        } else if r == HOW_FIND_URQUAN {
            self.api.npc(i32::from(FIND_URQUAN));
            self.session.defeat |= MASK_FIND_URQUAN;
        } else if r == HOW_FIGHT_URQUAN {
            self.api.npc(i32::from(FIGHT_URQUAN));
            self.session.defeat |= MASK_FIGHT_URQUAN;
        } else if r == HOW_ALLY {
            self.api.npc(i32::from(ALLY_LIKE_SO));
            self.session.defeat |= HOW_ALLY_AGAINST_URQUAN;
        } else if r == HOW_GET_STRONG {
            self.api.npc(i32::from(STRONG_LIKE_SO));
            self.session.defeat |= HOW_STRONG_AGAINST_URQUAN;
        }
        if !((self.session.defeat & MASK_FIND_URQUAN) != 0) {
            self.api
                .response(HOW_FIND_URQUAN, Handler::DefeatUrquan, &[]);
        }
        if !((self.session.defeat & MASK_FIGHT_URQUAN) != 0) {
            self.api
                .response(HOW_FIGHT_URQUAN, Handler::DefeatUrquan, &[]);
        }
        if !((self.session.defeat & HOW_ALLY_AGAINST_URQUAN) != 0) {
            self.api.response(HOW_ALLY, Handler::DefeatUrquan, &[]);
        }
        if !((self.session.defeat & HOW_STRONG_AGAINST_URQUAN) != 0) {
            self.api
                .response(HOW_GET_STRONG, Handler::DefeatUrquan, &[]);
        }
        self.api.response(ENOUGH_DEFEAT, Handler::TellMission, &[]);
    }
    fn analyze_condition(&mut self) {
        let mut i: u8;
        let mut num_thrusters: u8 = 0;
        let mut num_jets: u8 = 0;
        let mut num_guns: u8 = 0;
        let mut num_bays: u8 = 0;
        let mut num_batts: u8 = 0;
        let mut num_track: u8 = 0;
        let mut num_defense: u8 = 0;

        i = 0;
        while i < NUM_DRIVE_SLOTS {
            if self.api.ship().drives[i as usize] < EMPTY_SLOT {
                num_thrusters += 1;
            }
            i += 1;
        }
        i = 0;
        while i < NUM_JET_SLOTS {
            if self.api.ship().jets[i as usize] < EMPTY_SLOT {
                num_jets += 1;
            }
            i += 1;
        }
        i = 0;
        while i < NUM_MODULE_SLOTS {
            let which_piece: u8;
            let res = {
                which_piece = self.api.ship().modules[i as usize];
                which_piece
            };
            match res {
                STORAGE_BAY => {
                    num_bays += 1;
                }
                DYNAMO_UNIT | SHIVA_FURNACE => {
                    num_batts += 1 + (which_piece - DYNAMO_UNIT);
                }
                GUN_WEAPON | BLASTER_WEAPON | CANNON_WEAPON => {
                    num_guns += 1 + (which_piece - GUN_WEAPON);
                }
                TRACKING_SYSTEM => {
                    num_track += 1;
                }
                ANTIMISSILE_DEFENSE => {
                    num_defense += 1;
                }
                _ => {}
            }
            i += 1;
        }
        if (num_track != 0) && (num_guns != 0) {
            num_guns += 2;
        }
        let has_minimum: bool = (((((((num_thrusters >= 7) && (num_jets >= 5))
            && (self.api.ship().crew >= CREW_POD_CAPACITY))
            && (self.api.ship().fuel >= FUEL_TANK_CAPACITY))
            && (num_bays >= 1))
            && (self.api.ship().landers != 0))
            && (num_batts >= 1))
            && (num_guns >= 2);
        self.api.npc(i32::from(LETS_SEE));
        if !(has_minimum) && (self.api.get("CHMMR_BOMB_STATE") < 2) {
            self.api.npc(i32::from(IMPROVE_1));
            if num_thrusters < 7 {
                self.api.npc(i32::from(NEED_THRUSTERS_1));
            }
            if num_jets < 5 {
                self.api.npc(i32::from(NEED_TURN_1));
            }
            if num_guns < 2 {
                self.api.npc(i32::from(NEED_GUNS_1));
            }
            if self.api.ship().crew < CREW_POD_CAPACITY {
                self.api.npc(i32::from(NEED_CREW_1));
            }
            if self.api.ship().fuel < FUEL_TANK_CAPACITY {
                self.api.npc(i32::from(NEED_FUEL_1));
            }
            if num_bays < 1 {
                self.api.npc(i32::from(NEED_STORAGE_1));
            }
            if self.api.ship().landers == 0 {
                self.api.npc(i32::from(NEED_LANDERS_2));
            }
            if num_batts < 1 {
                self.api.npc(i32::from(NEED_DYNAMOS_1));
            }
            if self.api.ship().units >= 3000 {
                self.api.npc(i32::from(IMPROVE_FLAGSHIP_WITH_RU));
            } else {
                self.api.npc(i32::from(GO_GET_MINERALS));
            }
        } else {
            let mut num_aliens: u8 = 0;

            let fleet_strength: u16 = self.api.escorts_worth();
            i = 0;
            while i < NUM_AVAILABLE_RACES {
                if (i != HUMAN_SHIP) && (self.api.alliance(u16::from(i)) == GOOD_GUY) {
                    num_aliens += 1;
                }
                i += 1;
            }
            let has_maximum: bool = (((((((num_thrusters == NUM_DRIVE_SLOTS)
                && (num_jets == NUM_JET_SLOTS))
                && (self.api.ship().crew >= (CREW_POD_CAPACITY * 3)))
                && (self.api.ship().fuel >= (FUEL_TANK_CAPACITY * 3)))
                && (self.api.ship().landers >= 3))
                && (num_batts >= 4))
                && (num_guns >= 7))
                && (num_defense >= 2);
            if !(has_maximum) && (self.api.get("CHMMR_BOMB_STATE") < 2) {
                self.api.npc(i32::from(GOT_OK_FLAGSHIP));
            } else {
                self.api.npc(i32::from(GOT_AWESOME_FLAGSHIP));
            }
            if self.api.get("CHMMR_BOMB_STATE") >= 2 {
                self.api.npc(i32::from(CHMMR_IMPROVED_BOMB));
                if fleet_strength < 20000 {
                    self.api.npc(i32::from(MUST_ACQUIRE_AWESOME_FLEET));
                } else {
                    self.api.npc(i32::from(GOT_AWESOME_FLEET));
                    if !(self.api.get("TALKING_PET_ON_SHIP") != 0) {
                        self.api.npc(i32::from(MUST_ELIMINATE_URQUAN_GUARDS));
                    } else {
                        self.api.npc(i32::from(GO_DESTROY_SAMATRA));
                    }
                }
            } else if num_aliens < 2 {
                self.api.npc(i32::from(GO_ALLY_WITH_ALIENS));
            } else {
                self.api.npc(i32::from(MADE_SOME_ALLIES));
                if fleet_strength < 6000 {
                    if self.api.ship().units >= 3000 {
                        self.api.npc(i32::from(BUY_COMBAT_SHIPS));
                    } else {
                        self.api.npc(i32::from(GET_SHIPS_BY_MINING_OR_ALLIANCE));
                    }
                } else {
                    self.api.npc(i32::from(GOT_OK_FLEET));
                    if !(has_maximum) {
                        self.api.npc(i32::from(IMPROVE_2));
                        if num_thrusters < NUM_DRIVE_SLOTS {
                            self.api.npc(i32::from(NEED_THRUSTERS_2));
                        }
                        if num_jets < NUM_JET_SLOTS {
                            self.api.npc(i32::from(NEED_TURN_2));
                        }
                        if num_guns < 7 {
                            self.api.npc(i32::from(NEED_GUNS_2));
                        }
                        if self.api.ship().crew < (CREW_POD_CAPACITY * 3) {
                            self.api.npc(i32::from(NEED_CREW_2));
                        }
                        if self.api.ship().fuel < (FUEL_TANK_CAPACITY * 3) {
                            self.api.npc(i32::from(NEED_FUEL_2));
                        }
                        if self.api.ship().landers < 3 {
                            self.api.npc(i32::from(NEED_LANDERS_1));
                        }
                        if num_batts < 4 {
                            self.api.npc(i32::from(NEED_DYNAMOS_2));
                        }
                        if num_defense < 2 {
                            self.api.npc(i32::from(NEED_POINT));
                        }
                    } else if !(self.api.get("AWARE_OF_SAMATRA") != 0) {
                        self.api.npc(i32::from(GO_LEARN_ABOUT_URQUAN));
                    } else {
                        self.api.npc(i32::from(KNOW_ABOUT_SAMATRA));
                        if !(self.api.get("UTWIG_BOMB") != 0) {
                            self.api.npc(i32::from(FIND_WAY_TO_DESTROY_SAMATRA));
                        } else if self.api.get("UTWIG_BOMB_ON_SHIP") != 0 {
                            self.api.npc(i32::from(MUST_INCREASE_BOMB_STRENGTH));
                        }
                    }
                }
            }
        }
    }
    fn tell_mission(&mut self, r: u16) {
        let mut pstack: [u16; 4] = [0; 4];
        if r == OUR_MISSION {
            self.api.npc(i32::from(WHICH_MISSION));
            self.session.stacks[0] = 0;
            self.session.stacks[1] = 0;
            self.session.stacks[2] = 0;
            self.session.stacks[3] = 0;
        } else if r == WHERE_GET_MINERALS {
            self.api.npc(i32::from(GET_MINERALS));
            self.session.stacks[0] = 1;
        } else if r == WHAT_ABOUT_ALIENS {
            self.api.npc(i32::from(ABOUT_ALIENS));
            self.session.stacks[1] = 1;
        } else if r == WHAT_DO_NOW {
            self.analyze_condition();
            self.session.stacks[2] = 1;
        } else if r == WHAT_ABOUT_URQUAN {
            self.api.npc(i32::from(MUST_DEFEAT));
            self.session.stacks[3] = 1;
        } else if r == ENOUGH_DEFEAT {
            self.api.npc(i32::from(OK_ENOUGH_DEFEAT));
            self.session.stacks[3] = 2;
        }
        match self.session.stacks[0] {
            0 => {
                pstack[0] = WHERE_GET_MINERALS;
            }
            _ => {
                pstack[0] = 0;
            }
        }
        match self.session.stacks[1] {
            0 => {
                pstack[1] = WHAT_ABOUT_ALIENS;
            }
            _ => {
                pstack[1] = 0;
            }
        }
        match self.session.stacks[2] {
            0 => {
                pstack[2] = WHAT_DO_NOW;
            }
            _ => {
                pstack[2] = 0;
            }
        }
        match self.session.stacks[3] {
            0 => {
                pstack[3] = WHAT_ABOUT_URQUAN;
            }
            1 => {
                pstack[3] = HOW_DEFEAT;
            }
            _ => {
                pstack[3] = 0;
            }
        }
        if pstack[0] != 0 {
            self.api.response(pstack[0], Handler::TellMission, &[]);
        }
        if pstack[1] != 0 {
            self.api.response(pstack[1], Handler::TellMission, &[]);
        }
        if pstack[2] != 0 {
            self.api.response(pstack[2], Handler::TellMission, &[]);
        }
        if pstack[3] != 0 {
            if self.session.stacks[3] == 1 {
                self.api.response(pstack[3], Handler::DefeatUrquan, &[]);
            } else {
                self.api.response(pstack[3], Handler::TellMission, &[]);
            }
        }
        self.api.response(ENOUGH_MISSION, Handler::NeedInfo, &[]);
    }
    fn need_info(&mut self, r: u16) {
        if r == NEED_INFO {
            self.api.npc(i32::from(WHAT_KIND_OF_INFO));
        } else if r == ENOUGH_STARBASE {
            self.api.npc(i32::from(OK_ENOUGH_STARBASE));
        } else if r == ENOUGH_HISTORY {
            self.api.npc(i32::from(OK_ENOUGH_HISTORY));
        } else if r == ENOUGH_MISSION {
            self.api.npc(i32::from(OK_ENOUGH_MISSION));
        }
        self.api
            .response(STARBASE_FUNCTIONS, Handler::TellStarBase, &[]);
        self.api.response(HISTORY, Handler::TellHistory, &[]);
        self.api.response(OUR_MISSION, Handler::TellMission, &[]);
        self.api
            .response(NO_NEED_INFO, Handler::NormalStarbase, &[]);
    }
    fn discuss_devices(&mut self, talk_about: bool) -> bool {
        let mut i: u16;
        let mut vux_beast_index: u16;
        let mut phrase_index: u16;

        if talk_about {
            self.api.npc(i32::from(DEVICE_HEAD));
        }
        phrase_index = 2;
        vux_beast_index = 0;

        i = 0;
        while i < NUM_DEVICES {
            let mut p_str: u16;
            p_str = 0;
            match i {
                ROSY_SPHERE_DEVICE => {
                    if (self.api.get("ROSY_SPHERE_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_ROSY_SPHERE") != 0)
                    {
                        p_str = ABOUT_SPHERE;
                        self.api.set("DISCUSSED_ROSY_SPHERE", (talk_about) as u8);
                    }
                }
                ARTIFACT_2_DEVICE => {
                    if (self.api.get("ARTIFACT_2_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_ARTIFACT_2") != 0)
                    {
                        p_str = ABOUT_ARTIFACT_2;
                        self.api.set("DISCUSSED_ARTIFACT_2", (talk_about) as u8);
                    }
                }
                ARTIFACT_3_DEVICE => {
                    if (self.api.get("ARTIFACT_3_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_ARTIFACT_3") != 0)
                    {
                        p_str = ABOUT_ARTIFACT_3;
                        self.api.set("DISCUSSED_ARTIFACT_3", (talk_about) as u8);
                    }
                }
                SUN_EFFICIENCY_DEVICE => {
                    if (self.api.get("SUN_DEVICE_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_SUN_EFFICIENCY") != 0)
                    {
                        p_str = ABOUT_SUN;
                        self.api.set("DISCUSSED_SUN_EFFICIENCY", (talk_about) as u8);
                    }
                }
                UTWIG_BOMB_DEVICE => {
                    if (self.api.get("UTWIG_BOMB_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_UTWIG_BOMB") != 0)
                    {
                        p_str = ABOUT_BOMB;
                        self.api.set("DISCUSSED_UTWIG_BOMB", (talk_about) as u8);
                    }
                }
                ULTRON_0_DEVICE => {
                    if (self.api.get("ULTRON_CONDITION") == 1)
                        && !(self.api.get("DISCUSSED_ULTRON") != 0)
                    {
                        p_str = ABOUT_ULTRON_0;
                        self.api.set("DISCUSSED_ULTRON", (talk_about) as u8);
                    }
                }
                ULTRON_1_DEVICE => {
                    if (self.api.get("ULTRON_CONDITION") == 2)
                        && !(self.api.get("DISCUSSED_ULTRON") != 0)
                    {
                        p_str = ABOUT_ULTRON_1;
                        self.api.set("DISCUSSED_ULTRON", (talk_about) as u8);
                    }
                }
                ULTRON_2_DEVICE => {
                    if (self.api.get("ULTRON_CONDITION") == 3)
                        && !(self.api.get("DISCUSSED_ULTRON") != 0)
                    {
                        p_str = ABOUT_ULTRON_2;
                        self.api.set("DISCUSSED_ULTRON", (talk_about) as u8);
                    }
                }
                ULTRON_3_DEVICE => {
                    if (self.api.get("ULTRON_CONDITION") == 4)
                        && !(self.api.get("DISCUSSED_ULTRON") != 0)
                    {
                        p_str = ABOUT_ULTRON_3;
                        self.api.set("DISCUSSED_ULTRON", (talk_about) as u8);
                    }
                }
                MAIDENS_DEVICE => {
                    if (self.api.get("MAIDENS_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_MAIDENS") != 0)
                    {
                        p_str = ABOUT_MAIDENS;
                        self.api.set("DISCUSSED_MAIDENS", (talk_about) as u8);
                    }
                }
                TALKING_PET_DEVICE => {
                    if (self.api.get("TALKING_PET_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_TALKING_PET") != 0)
                    {
                        p_str = ABOUT_TALKPET;
                        self.api.set("DISCUSSED_TALKING_PET", (talk_about) as u8);
                    }
                }
                AQUA_HELIX_DEVICE => {
                    if (self.api.get("AQUA_HELIX_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_AQUA_HELIX") != 0)
                    {
                        p_str = ABOUT_HELIX;
                        self.api.set("DISCUSSED_AQUA_HELIX", (talk_about) as u8);
                    }
                }
                CLEAR_SPINDLE_DEVICE => {
                    if (self.api.get("CLEAR_SPINDLE_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_CLEAR_SPINDLE") != 0)
                    {
                        p_str = ABOUT_SPINDLE;
                        self.api.set("DISCUSSED_CLEAR_SPINDLE", (talk_about) as u8);
                    }
                }
                UMGAH_HYPERWAVE_DEVICE => {
                    if (self.api.get("UMGAH_BROADCASTERS_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_UMGAH_HYPERWAVE") != 0)
                    {
                        p_str = ABOUT_UCASTER;
                        self.api
                            .set("DISCUSSED_UMGAH_HYPERWAVE", (talk_about) as u8);
                    }
                }
                TAALO_PROTECTOR_DEVICE => {
                    if (self.api.get("TAALO_PROTECTOR_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_TAALO_PROTECTOR") != 0)
                    {
                        p_str = ABOUT_SHIELD;
                        self.api
                            .set("DISCUSSED_TAALO_PROTECTOR", (talk_about) as u8);
                    }
                }
                EGG_CASING0_DEVICE => {
                    if (self.api.get("EGG_CASE0_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_EGG_CASING0") != 0)
                    {
                        p_str = ABOUT_EGGCASE_0;
                        self.api.set("DISCUSSED_EGG_CASING0", (talk_about) as u8);
                        self.api.set("DISCUSSED_EGG_CASING1", (talk_about) as u8);
                        self.api.set("DISCUSSED_EGG_CASING2", (talk_about) as u8);
                    }
                }
                EGG_CASING1_DEVICE => {
                    if (self.api.get("EGG_CASE1_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_EGG_CASING1") != 0)
                    {
                        p_str = ABOUT_EGGCASE_0;
                        self.api.set("DISCUSSED_EGG_CASING0", (talk_about) as u8);
                        self.api.set("DISCUSSED_EGG_CASING1", (talk_about) as u8);
                        self.api.set("DISCUSSED_EGG_CASING2", (talk_about) as u8);
                    }
                }
                EGG_CASING2_DEVICE => {
                    if (self.api.get("EGG_CASE2_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_EGG_CASING2") != 0)
                    {
                        p_str = ABOUT_EGGCASE_0;
                        self.api.set("DISCUSSED_EGG_CASING0", (talk_about) as u8);
                        self.api.set("DISCUSSED_EGG_CASING1", (talk_about) as u8);
                        self.api.set("DISCUSSED_EGG_CASING2", (talk_about) as u8);
                    }
                }
                SYREEN_SHUTTLE_DEVICE => {
                    if (self.api.get("SYREEN_SHUTTLE_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_SYREEN_SHUTTLE") != 0)
                    {
                        p_str = ABOUT_SHUTTLE;
                        self.api.set("DISCUSSED_SYREEN_SHUTTLE", (talk_about) as u8);
                    }
                }
                VUX_BEAST_DEVICE => {
                    if (self.api.get("VUX_BEAST_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_VUX_BEAST") != 0)
                    {
                        p_str = ABOUT_VUXBEAST0;
                        self.api.set("DISCUSSED_VUX_BEAST", (talk_about) as u8);
                    }
                }
                DESTRUCT_CODE_DEVICE => {
                    if (self.api.get("DESTRUCT_CODE_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_DESTRUCT_CODE") != 0)
                    {
                        p_str = ABOUT_DESTRUCT;
                        self.api.set("DISCUSSED_DESTRUCT_CODE", (talk_about) as u8);
                    }
                }
                PORTAL_SPAWNER_DEVICE => {
                    if (self.api.get("PORTAL_SPAWNER_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_PORTAL_SPAWNER") != 0)
                    {
                        p_str = ABOUT_PORTAL;
                        self.api.set("DISCUSSED_PORTAL_SPAWNER", (talk_about) as u8);
                    }
                }
                URQUAN_WARP_DEVICE => {
                    if (self.api.get("PORTAL_KEY_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_URQUAN_WARP") != 0)
                    {
                        p_str = ABOUT_WARPPOD;
                        self.api.set("DISCUSSED_URQUAN_WARP", (talk_about) as u8);
                    }
                }
                BURVIX_HYPERWAVE_DEVICE
                    if (self.api.get("BURV_BROADCASTERS_ON_SHIP") != 0)
                        && !(self.api.get("DISCUSSED_BURVIX_HYPERWAVE") != 0) =>
                {
                    p_str = ABOUT_BCASTER;
                    self.api
                        .set("DISCUSSED_BURVIX_HYPERWAVE", (talk_about) as u8);
                }
                _ => {}
            }
            if p_str != 0 {
                if talk_about {
                    if phrase_index > 2 {
                        self.api.npc(i32::from(BETWEEN_DEVICES));
                    }
                    self.api.npc(i32::from(p_str));
                    if p_str == ABOUT_VUXBEAST0 {
                        vux_beast_index = {
                            phrase_index += 1;
                            phrase_index
                        };
                        self.api.npc(i32::from(ABOUT_VUXBEAST1));
                    }
                }
                phrase_index += 2;
            }
            i += 1;
        }
        if talk_about {
            self.api.npc(i32::from(DEVICE_TAIL));
            if vux_beast_index != 0 {
                self.api.talk_segue(vux_beast_index - 1);
                self.api.talking_animation(false);
                self.api.talk_segue(vux_beast_index);
                self.api.talking_animation(true);
                self.api.talk_segue(!0_u16);
            }
        }
        phrase_index > 2
    }
    fn check_bulletins(&mut self, repeat: bool) {
        let mut p_intro: u16;
        let mut b0: u8;
        let mut bulletin_mask: u32;
        if repeat {
            bulletin_mask = self.session.bulletins ^ 0xFFFFFFFF;
        } else {
            bulletin_mask = self.get32("STARBASE_BULLETS0");
        }
        p_intro = 0;
        {
            b0 = 0;
            b0
        };
        while b0 < 32 {
            if !((bulletin_mask & (1 << b0)) != 0) {
                let mut p_str: u16;
                p_str = 0;
                match b0 {
                    0 => {
                        if self.api.alliance(SPATHI_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_1;
                        }
                    }
                    1 => {
                        if self.api.alliance(ZOQFOTPIK_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_2;
                        }
                    }
                    2 => {
                        if self.api.alliance(SUPOX_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_3;
                        }
                    }
                    3 => {
                        if self.api.alliance(UTWIG_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_4;
                        }
                    }
                    4 => {
                        if self.api.alliance(ORZ_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_5;
                        }
                    }
                    5 => {
                        if self.api.get("ARILOU_MANNER") == 2 {
                            bulletin_mask |= 1 << b0;
                        } else if (self.api.get("PORTAL_SPAWNER") != 0)
                            && (repeat || (self.api.escort_feasibility(ARILOU_SHIP) != 0))
                        {
                            p_str = STARBASE_BULLETIN_6;
                            if !(repeat) {
                                self.api.add_escorts(ARILOU_SHIP, NUM_GIFT_ARILOUS);
                            }
                        }
                    }
                    6 => {
                        if self.api.get("ZOQFOT_DISTRESS") == 1 {
                            p_str = STARBASE_BULLETIN_7;
                        }
                    }
                    7 => {
                        if self.api.get("MET_MELNORME") != 0 {
                            bulletin_mask |= 1 << b0;
                        } else if self.check_timing(3, 0) {
                            p_str = STARBASE_BULLETIN_8;
                        }
                    }
                    8 => {
                        if self.api.get("MET_MELNORME") != 0 {
                            bulletin_mask |= 1 << b0;
                        } else if self.check_timing(6, 0) {
                            p_str = STARBASE_BULLETIN_9;
                        }
                    }
                    9 => {
                        if self.api.get("FOUND_PLUTO_SPATHI") != 0 {
                            bulletin_mask |= 1 << b0;
                        } else if self.check_timing(0, 7) {
                            p_str = STARBASE_BULLETIN_10;
                        }
                    }
                    10 => {
                        if self.api.get("SPATHI_SHIELDED_SELVES") != 0 {
                            p_str = STARBASE_BULLETIN_11;
                        }
                    }
                    11 => {
                        if (self.api.get("ZOQFOT_HOME_VISITS") != 0)
                            || (self.get32("ZOQFOT_GRPOFFS0") != 0)
                        {
                            bulletin_mask |= 1 << b0;
                        } else if self.check_timing(0, 42) {
                            p_str = STARBASE_BULLETIN_12;
                        }
                    }
                    12 => {
                        if self.api.alliance(CHMMR_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_13;
                        }
                    }
                    13 => {
                        if self.api.alliance(SHOFIXTI_SHIP) == GOOD_GUY {
                            p_str = STARBASE_BULLETIN_14;
                        }
                    }
                    14 => {
                        if self.api.get("PKUNK_MISSION") != 0 {
                            p_str = STARBASE_BULLETIN_15;
                        }
                    }
                    15 => {
                        if self.api.get("DESTRUCT_CODE_ON_SHIP") != 0 {
                            bulletin_mask |= 1 << b0;
                        } else if self.check_timing(7, 0) {
                            p_str = STARBASE_BULLETIN_16;
                        }
                    }
                    16 => {}
                    17 => {
                        if self.api.get("YEHAT_ABSORBED_PKUNK") != 0 {
                            p_str = STARBASE_BULLETIN_18;
                        }
                    }
                    18 => {
                        if self.api.get("CHMMR_BOMB_STATE") == 2 {
                            p_str = STARBASE_BULLETIN_19;
                        }
                    }
                    19 => {}
                    20 => {}
                    21 => {
                        if self.api.get("ZOQFOT_DISTRESS") == 2 {
                            p_str = STARBASE_BULLETIN_22;
                        }
                    }
                    22 => {}
                    23 => {}
                    24 => {}
                    25 => {}
                    26 => {
                        let crew_sold: u16 = u16::from_le_bytes([
                            self.api.get("CREW_SOLD_TO_DRUUGE0"),
                            self.api.get("CREW_SOLD_TO_DRUUGE1"),
                        ]);
                        if crew_sold > 100 {
                            bulletin_mask |= 1 << b0;
                        } else if crew_sold != 0 {
                            p_str = STARBASE_BULLETIN_27;
                        }
                    }
                    27 => {
                        let crew_sold: u16 = u16::from_le_bytes([
                            self.api.get("CREW_SOLD_TO_DRUUGE0"),
                            self.api.get("CREW_SOLD_TO_DRUUGE1"),
                        ]);
                        if crew_sold > 250 {
                            bulletin_mask |= 1 << b0;
                        } else if crew_sold > 100 {
                            p_str = STARBASE_BULLETIN_28;
                        }
                    }
                    28 => {
                        let crew_bought: u16 = u16::from_le_bytes([
                            self.api.get("CREW_PURCHASED0"),
                            self.api.get("CREW_PURCHASED1"),
                        ]);
                        if crew_bought >= CREW_EXPENSE_THRESHOLD {
                            p_str = STARBASE_BULLETIN_29;
                        }
                    }
                    29 => {
                        if u16::from_le_bytes([
                            self.api.get("CREW_SOLD_TO_DRUUGE0"),
                            self.api.get("CREW_SOLD_TO_DRUUGE1"),
                        ]) > 250
                        {
                            p_str = STARBASE_BULLETIN_30;
                        }
                    }
                    30 => {}
                    31 => {}
                    _ => {}
                }
                if p_str != 0 {
                    if p_intro != 0 {
                        self.api.npc(i32::from(BETWEEN_BULLETINS));
                    } else if repeat {
                        p_intro = BEFORE_WE_GO_ON_1;
                    } else {
                        match (self.api.random() as u8) % 7 {
                            0 => {
                                p_intro = BEFORE_WE_GO_ON_1;
                            }
                            1 => {
                                p_intro = BEFORE_WE_GO_ON_2;
                            }
                            2 => {
                                p_intro = BEFORE_WE_GO_ON_3;
                            }
                            3 => {
                                p_intro = BEFORE_WE_GO_ON_4;
                            }
                            4 => {
                                p_intro = BEFORE_WE_GO_ON_5;
                            }
                            5 => {
                                p_intro = BEFORE_WE_GO_ON_6;
                            }
                            _ => {
                                p_intro = BEFORE_WE_GO_ON_7;
                            }
                        }
                        self.api.npc(i32::from(p_intro));
                    }
                    self.api.npc(i32::from(p_str));
                    self.session.bulletins |= 1 << b0;
                }
            }
            b0 += 1;
        }
        if (p_intro == 0) && (self.api.get("STARBASE_VISITED") != 0) {
            self.api.npc(i32::from(RETURN_HELLO));
        } else if !(repeat) {
            self.set32("STARBASE_BULLETS0", bulletin_mask);
        }
    }
    fn normal_starbase(&mut self, r: u16) {
        if r == NO_NEED_INFO {
            self.api.npc(i32::from(OK_NO_NEED_INFO));
        } else if r == NEW_DEVICES {
            self.discuss_devices(true);
        } else if r == REPEAT_BULLETINS {
            self.check_bulletins(true);
        } else if r == 0 {
            if self.api.get("MOONBASE_ON_SHIP") != 0 {
                self.api.npc(i32::from(STARBASE_IS_READY_A));
                if self.api.using_speech() {
                    self.api.npc(i32::from(YOUR_FLAGSHIP_3DO1));
                } else {
                    self.api.npc(i32::from(YOUR_FLAGSHIP_PC));
                    self.api.npc(GLOBAL_SHIP_NAME);
                }
                self.api.npc(i32::from(STARBASE_IS_READY_B));
                if self.api.using_speech() {
                    self.api.npc(i32::from(YOUR_FLAGSHIP_3DO0));
                } else {
                    self.api.npc(GLOBAL_SHIP_NAME);
                }
                self.api.npc(i32::from(STARBASE_IS_READY_C));
                self.api.gauges(0, 0, i32::from(STARBASE_READY_REWARD));
                self.api.set("STARBASE_MONTH", self.api.date().month);
                self.api.set("STARBASE_DAY", self.api.date().day);
            } else if self.api.get("STARBASE_VISITED") != 0 {
                self.check_bulletins(false);
            } else {
                let mut p_str0: u16 = 0;
                let mut p_str1: u16 = 0;
                match (self.api.random() as u8) & 7 {
                    0 => {
                        p_str0 = NORMAL_HELLO_A0;
                        p_str1 = NORMAL_HELLO_A1;
                    }
                    1 => {
                        p_str0 = NORMAL_HELLO_B0;
                        p_str1 = NORMAL_HELLO_B1;
                    }
                    2 => {
                        p_str0 = NORMAL_HELLO_C0;
                        p_str1 = NORMAL_HELLO_C1;
                    }
                    3 => {
                        p_str0 = NORMAL_HELLO_D0;
                        p_str1 = NORMAL_HELLO_D1;
                    }
                    4 => {
                        p_str0 = NORMAL_HELLO_E0;
                        p_str1 = NORMAL_HELLO_E1;
                    }
                    5 => {
                        p_str0 = NORMAL_HELLO_F0;
                        p_str1 = NORMAL_HELLO_F1;
                    }
                    6 => {
                        p_str0 = NORMAL_HELLO_G0;
                        p_str1 = NORMAL_HELLO_G1;
                    }
                    7 => {
                        p_str0 = NORMAL_HELLO_H0;
                        p_str1 = NORMAL_HELLO_H1;
                    }
                    _ => {}
                }
                self.api.npc(i32::from(p_str0));
                if !(self.api.using_speech()) {
                    self.api.npc(i32::from(SPACE));
                    self.api.npc(GLOBAL_PLAYER_NAME);
                }
                self.api.npc(i32::from(p_str1));
                self.check_bulletins(false);
            }
            self.api.set("STARBASE_VISITED", 1_u8);
        }
        if self.api.ship().mineral_mass != 0 {
            self.api.response(HAVE_MINERALS, Handler::SellMinerals, &[]);
        }
        if self.discuss_devices(false) {
            self.api.response(NEW_DEVICES, Handler::NormalStarbase, &[]);
        }
        if self.session.bulletins != 0 {
            self.api
                .response(REPEAT_BULLETINS, Handler::NormalStarbase, &[]);
        }
        self.api.response(NEED_INFO, Handler::NeedInfo, &[]);
        self.api.response(GOODBYE_COMMANDER, Handler::ByeBye, &[]);
    }
    fn intro(&mut self) {
        self.normal_starbase(0);
    }
    fn post_starbase_enc(&mut self) {
        self.api.set("MOONBASE_ON_SHIP", 0_u8);
        if self.api.get("CHMMR_BOMB_STATE") == 2 {
            self.api.set("CHMMR_BOMB_STATE", 3_u8);
        }
    }
}
fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.commander.graphics".as_ptr(),
        alien_font_res: c"comm.commander.font".as_ptr(),
        alien_colormap_res: c"comm.commander.colortable".as_ptr(),
        alien_song_res: c"comm.commander.music".as_ptr(),
        conversation_phrases_res: c"comm.starbase.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 164, y: 20 },
        alien_text_width: 143,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 10,
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] = animation(1, 3, 4, (56, 0), (0, 6720));
    desc.alien_ambient_array[1] = animation(10, 30, 2, (21, 0), (1680, 0));
    desc.alien_ambient_array[2] = animation(40, 7, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[3] = animation(47, 8, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[4] = animation(55, 6, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[5] = animation(61, 6, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[6] = animation(67, 7, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[7] = animation(74, 11, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[8] = animation(85, 10, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[9] = animation(95, 1, 0, (0, 0), (0, 0));
    desc.alien_talk_desc = animation(4, 6, 0, (84, 56), (98, 70));
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}
#[cfg(not(test))]
pub(crate) mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        static mut SESSION: Option<Session> = None;
        // Only the game thread dispatches these callbacks.
        let session =
            unsafe { (&mut *std::ptr::addr_of_mut!(SESSION)).get_or_insert_with(Session::default) };
        f(&mut Dialogue {
            api: &mut api,
            session,
        });
    }
    fn bye_bye(r: u32) {
        run(|s| {
            s.handle(
                Handler::ByeBye,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn alliance_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::AllianceInfo,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn hierarchy_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::HierarchyInfo,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn alien_races(r: u32) {
        run(|s| {
            s.handle(
                Handler::AlienRaces,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn war_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::WarInfo,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn ancient_history(r: u32) {
        run(|s| {
            s.handle(
                Handler::AncientHistory,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn tell_history(r: u32) {
        run(|s| {
            s.handle(
                Handler::TellHistory,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn defeat_urquan(r: u32) {
        run(|s| {
            s.handle(
                Handler::DefeatUrquan,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn tell_mission(r: u32) {
        run(|s| {
            s.handle(
                Handler::TellMission,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn tell_star_base(r: u32) {
        run(|s| {
            s.handle(
                Handler::TellStarBase,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn need_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::NeedInfo,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn normal_starbase(r: u32) {
        run(|s| {
            s.handle(
                Handler::NormalStarbase,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn sell_minerals(r: u32) {
        run(|s| {
            s.handle(
                Handler::SellMinerals,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
        match handler {
            Handler::ByeBye => bye_bye,
            Handler::AllianceInfo => alliance_info,
            Handler::HierarchyInfo => hierarchy_info,
            Handler::AlienRaces => alien_races,
            Handler::WarInfo => war_info,
            Handler::AncientHistory => ancient_history,
            Handler::TellHistory => tell_history,
            Handler::DefeatUrquan => defeat_urquan,
            Handler::TellMission => tell_mission,
            Handler::TellStarBase => tell_star_base,
            Handler::NeedInfo => need_info,
            Handler::NormalStarbase => normal_starbase,
            Handler::SellMinerals => sell_minerals,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_starbase_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter descriptor and initial battle segue.
    ///
    /// # Safety
    /// Game thread only. The caller must copy the descriptor before another encounter.
    #[no_mangle]
    pub unsafe extern "C" fn init_starbase_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct StarbasDialogue;
impl super::RaceDialogue for StarbasDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_starbase_comm()) }
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
