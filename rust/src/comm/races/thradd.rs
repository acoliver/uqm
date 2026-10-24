//! Thradd dialogue, ported from thraddc.c.
use super::alliance::AllianceApi;
use super::script::{animation, as_comm_data, GLOBAL_PLAYER_NAME};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    ThraddAllies,
    ThraddDemeanor,
    ThraddCulture,
    HelixWorld,
    ThraddHostile,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: AllianceApi<Handler>> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::ThraddAllies => self.thradd_allies(r),
            Handler::ThraddDemeanor => self.thradd_demeanor(r),
            Handler::ThraddCulture => self.thradd_culture(r),
            Handler::HelixWorld => self.helix_world(r),
            Handler::ThraddHostile => self.thradd_hostile(r),
        }
    }
    fn get_culture_name(&self) -> i32 {
        match self.api.get("THRADD_CULTURE") {
            1 => i32::from(CULTURE),
            2 => i32::from(FAT_JERKS),
            3 => i32::from(SLAVE_EMPIRE),
            _ => panic!("unknown Thraddash culture"),
        }
    }
    fn polite_phrase(&mut self, which_phrase: u8) {
        match which_phrase {
            0 => {
                self.api.npc(i32::from(HELLO_POLITE_1));
            }
            1 => {
                self.api.npc(i32::from(HELLO_POLITE_2));
            }
            2 => {
                self.api.npc(i32::from(HELLO_POLITE_3));
            }
            3 => {
                self.api.npc(i32::from(HELLO_POLITE_4));
            }
            _ => {}
        }
    }
    fn rhyme_phrase(&mut self, which_phrase: u8) {
        match which_phrase {
            0 => {
                self.api.npc(i32::from(HELLO_RHYME_1));
            }
            1 => {
                self.api.npc(i32::from(HELLO_RHYME_2));
            }
            2 => {
                self.api.npc(i32::from(HELLO_RHYME_3));
            }
            3 => {
                self.api.npc(i32::from(HELLO_RHYME_4));
            }
            _ => {}
        }
    }
    fn pig_latin_phrase(&mut self, which_phrase: u8) {
        match which_phrase {
            0 => {
                self.api.npc(i32::from(HELLO_PIG_LATIN_1));
            }
            1 => {
                self.api.npc(i32::from(HELLO_PIG_LATIN_2));
            }
            2 => {
                self.api.npc(i32::from(HELLO_PIG_LATIN_3));
            }
            3 => {
                self.api.npc(i32::from(HELLO_PIG_LATIN_4));
            }
            _ => {}
        }
    }
    fn like_you_phrase(&mut self, which_phrase: u8) {
        match which_phrase {
            0 => {
                self.api.npc(i32::from(HELLO_LIKE_YOU_1));
            }
            1 => {
                self.api.npc(i32::from(HELLO_LIKE_YOU_2));
            }
            2 => {
                self.api.npc(i32::from(HELLO_LIKE_YOU_3));
            }
            3 => {
                self.api.npc(i32::from(HELLO_LIKE_YOU_4));
            }
            _ => {}
        }
    }
    fn exit_conversation(&mut self, r: u16) {
        self.api.segue(1_u32);
        if r == BYE_HOSTILE_2 {
            self.api.npc(i32::from(GOODBYE_HOSTILE_2));
        } else if r == BYE_HOSTILE_1 {
            self.api.npc(i32::from(GOODBYE_HOSTILE_1));
            self.api.set("THRADD_HOSTILE_STACK_5", 1_u8);
        } else if r == SUBMIT_1 {
            self.api.npc(i32::from(NO_SUBMIT_1));
            self.api.set("THRADD_HOSTILE_STACK_2", 1_u8);
        } else if r == SUBMIT_2 {
            self.api.npc(i32::from(NO_SUBMIT_2));
        } else if r == GOT_IDEA {
            self.api.npc(i32::from(GOOD_IDEA));
            self.api.segue(0_u32);
            self.api.event(1, 0_u16, 0_u16, 0_u16, 5_u8);
            self.api.set("THRADD_STACK_1", 5_u8);
        } else if r == BYE_HOSTILE_HELIX {
            self.api.npc(i32::from(GOODBYE_HOSTILE_HELIX));
        } else if r == BYE_ALLY {
            let mut num_visits: u8;
            num_visits = self.api.get("THRADD_STACK_1");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GOODBYE_ALLY_1));
                }
                1 => {
                    self.api.npc(i32::from(GOODBYE_ALLY_2));
                }
                2 => {
                    self.api.npc(i32::from(GOODBYE_ALLY_3));
                }
                3 => {
                    self.api.npc(i32::from(GOODBYE_ALLY_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("THRADD_STACK_1", num_visits);
            self.api.segue(0_u32);
        } else if r == MAY_I_LAND {
            self.api.npc(i32::from(SURE_LAND));
            self.api.set("HELIX_UNPROTECTED", 1_u8);
            self.api.segue(0_u32);
        } else if r == DEMAND_TO_LAND {
            self.api.npc(i32::from(NO_DEMAND));
        } else if r == I_NEED_TO_LAND_LIE {
            self.api.npc(i32::from(CAUGHT_LIE));
        } else {
            if r == CONTEMPLATIVE {
                self.api.npc(i32::from(OK_CONTEMPLATIVE));
                self.api.set("THRADD_DEMEANOR", 0_u8);
            } else if r == FRIENDLY {
                self.api.npc(i32::from(OK_FRIENDLY));
                self.api.set("THRADD_DEMEANOR", 1_u8);
            } else if r == WACKY {
                self.api.npc(i32::from(OK_WACKY));
                self.api.set("THRADD_DEMEANOR", 2_u8);
            } else if r == JUST_LIKE_US {
                self.api.npc(i32::from(OK_JUST_LIKE_YOU));
                self.api.set("THRADD_DEMEANOR", 3_u8);
            }
            self.api.npc(i32::from(WORK_TO_DO));
            self.api.segue(0_u32);
        }
    }
    fn thradd_allies(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHY_YOU_HERE_ALLY {
            self.api.npc(i32::from(GUARDING_HELIX_ALLY));
            self.api.disable(WHY_YOU_HERE_ALLY);
        } else if r == WHATS_HELIX_ALLY {
            self.api.npc(i32::from(HELIX_IS_ALLY));
            self.api.disable(WHATS_HELIX_ALLY);
        } else if r == WHATS_UP_ALLY {
            num_visits = self.api.get("THRADD_INFO");
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
            self.api.set("THRADD_INFO", num_visits);
            self.api.disable(WHATS_UP_ALLY);
        } else if r == HOW_GOES_CULTURE {
            num_visits = self.api.get("THRADD_DEMEANOR");
            match i32::from(num_visits) & ((1 << 2) - 1) {
                0 => {
                    if !((i32::from(num_visits) & !((1 << 2) - 1)) != 0) {
                        self.api.npc(i32::from(CONTEMP_GOES_1));
                    } else {
                        self.api.npc(i32::from(CONTEMP_GOES_2));
                    }
                }
                1 => {
                    if !((i32::from(num_visits) & !((1 << 2) - 1)) != 0) {
                        self.api.npc(i32::from(FRIENDLY_GOES_1));
                    } else {
                        self.api.npc(i32::from(FRIENDLY_GOES_2));
                    }
                }
                2 => {
                    if !((i32::from(num_visits) & !((1 << 2) - 1)) != 0) {
                        self.api.npc(i32::from(WACKY_GOES_1));
                    } else {
                        self.api.npc(i32::from(WACKY_GOES_2));
                    }
                }
                3 => {
                    if !((i32::from(num_visits) & !((1 << 2) - 1)) != 0) {
                        self.api.npc(i32::from(LIKE_YOU_GOES_1));
                    } else {
                        self.api.npc(i32::from(LIKE_YOU_GOES_2));
                    }
                }
                _ => {}
            }
            num_visits |= (1 << 2) as u8;
            self.api.set("THRADD_DEMEANOR", num_visits);
            self.api.disable(HOW_GOES_CULTURE);
        }
        if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 6)) != 0 {
            if self.api.enabled(WHY_YOU_HERE_ALLY) {
                self.api
                    .response(WHY_YOU_HERE_ALLY, Handler::ThraddAllies, &[]);
            } else {
                if self.api.enabled(WHATS_HELIX_ALLY) {
                    self.api
                        .response(WHATS_HELIX_ALLY, Handler::ThraddAllies, &[]);
                }
                self.api
                    .response(MAY_I_LAND, Handler::ExitConversation, &[]);
            }
        }
        if self.api.enabled(WHATS_UP_ALLY) {
            self.api.response(WHATS_UP_ALLY, Handler::ThraddAllies, &[]);
        }
        if self.api.enabled(HOW_GOES_CULTURE) {
            self.api
                .response(HOW_GOES_CULTURE, Handler::ThraddAllies, &[]);
        }
        self.api.response(BYE_ALLY, Handler::ExitConversation, &[]);
    }
    fn thradd_demeanor(&mut self, r: u16) {
        if r == YOU_DECIDE {
            self.api.npc(i32::from(OK_CULTURE_20));
            self.api.set("THRADD_CULTURE", 1_u8);
        } else if r == FAT {
            self.api.npc(i32::from(OK_FAT));
            self.api.set("THRADD_CULTURE", 2_u8);
        } else if r == THE_SLAVE_EMPIRE0 {
            self.api.set("THRADD_CULTURE", 3_u8);
            self.api.npc(i32::from(OK_SLAVE));
        }
        self.api.npc(i32::from(HOW_SHOULD_WE_ACT));
        self.api
            .response(CONTEMPLATIVE, Handler::ExitConversation, &[]);
        self.api.response(FRIENDLY, Handler::ExitConversation, &[]);
        self.api.response(WACKY, Handler::ExitConversation, &[]);
        self.api
            .response(JUST_LIKE_US, Handler::ExitConversation, &[]);
    }
    fn thradd_culture(&mut self, r: u16) {
        if r == BE_POLITE {
            self.api.npc(i32::from(OK_POLITE));
            self.api.set("THRADD_INTRO", 0_u8);
        } else if r == USE_RHYMES {
            self.api.npc(i32::from(OK_RHYMES));
            self.api.set("THRADD_INTRO", 1_u8);
        } else if r == SPEAK_PIG_LATIN {
            self.api.npc(i32::from(OK_PIG_LATIN));
            self.api.set("THRADD_INTRO", 2_u8);
        } else if r == JUST_THE_WAY_WE_DO {
            self.api.npc(i32::from(OK_WAY_YOU_DO));
            self.api.set("THRADD_INTRO", 3_u8);
        }
        self.api.npc(i32::from(WHAT_NAME_FOR_CULTURE));
        let fragments: Vec<i32> = vec![
            i32::from(THE_SLAVE_EMPIRE0),
            GLOBAL_PLAYER_NAME,
            i32::from(THE_SLAVE_EMPIRE1),
        ];
        self.api.response(YOU_DECIDE, Handler::ThraddDemeanor, &[]);
        self.api.response(FAT, Handler::ThraddDemeanor, &[]);
        self.api
            .response(THE_SLAVE_EMPIRE0, Handler::ThraddDemeanor, &fragments);
    }
    fn thradd_worship(&mut self, _r: u16) {
        self.api.set("THRADD_VISITS", 0_u8);
        self.api.set("THRADD_MANNER", 1_u8);
        self.api.set("THRADD_STACK_1", 0_u8);
        self.api
            .set_allied((crate::comm::dispatch::ship::THRADDASH_SHIP) as u16, true);
        self.api.response(BE_POLITE, Handler::ThraddCulture, &[]);
        self.api
            .response(SPEAK_PIG_LATIN, Handler::ThraddCulture, &[]);
        self.api.response(USE_RHYMES, Handler::ThraddCulture, &[]);
        self.api
            .response(JUST_THE_WAY_WE_DO, Handler::ThraddCulture, &[]);
    }
    fn helix_world(&mut self, r: u16) {
        if r == WHY_YOU_HERE_HOSTILE {
            self.api.npc(i32::from(NONE_OF_YOUR_CONCERN));
            self.api.set("THRADD_CULTURE", 1_u8);
        } else if r == WHAT_ABOUT_THIS_WORLD {
            self.api.npc(i32::from(BLUE_HELIX));
            self.api.set("THRADD_INTRO", 1_u8);
        } else if r == WHATS_HELIX_HOSTILE {
            self.api.npc(i32::from(HELIX_IS_HOSTILE));
            self.api.set("THRADD_INTRO", 2_u8);
        } else if r == I_NEED_TO_LAND_LIE {
            self.api.npc(i32::from(CAUGHT_LIE));
            self.api.set("THRADD_DEMEANOR", 1_u8);
        }
        if !(self.api.get("THRADD_CULTURE") != 0) {
            self.api
                .response(WHY_YOU_HERE_HOSTILE, Handler::HelixWorld, &[]);
        } else {
            self.api
                .response(DEMAND_TO_LAND, Handler::ExitConversation, &[]);
        }
        match self.api.get("THRADD_INTRO") {
            0 => {
                self.api
                    .response(WHAT_ABOUT_THIS_WORLD, Handler::HelixWorld, &[]);
            }
            1 => {
                self.api
                    .response(WHATS_HELIX_HOSTILE, Handler::HelixWorld, &[]);
            }
            _ => {}
        }
        if !(self.api.get("THRADD_DEMEANOR") != 0) {
            self.api
                .response(I_NEED_TO_LAND_LIE, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_HOSTILE_HELIX, Handler::ExitConversation, &[]);
    }
    fn thradd_hostile(&mut self, r: u16) {
        if r == WHATS_UP_HOSTILE_1 {
            self.api.npc(i32::from(GENERAL_INFO_HOSTILE_1));
            self.api.set("THRADD_INFO", 1_u8);
            self.api.disable(WHATS_UP_HOSTILE_2);
        } else if r == WHATS_UP_HOSTILE_2 {
            let mut num_visits: u8;
            num_visits = self.api.get("THRADD_INFO");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_HOSTILE_2));
                }
                2 => {
                    self.api.npc(i32::from(GENERAL_INFO_HOSTILE_3));
                }
                3 => {
                    self.api.npc(i32::from(GENERAL_INFO_HOSTILE_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("THRADD_INFO", num_visits);
            self.api.disable(WHATS_UP_HOSTILE_2);
        } else if r == WHAT_ABOUT_YOU_1 {
            self.api.npc(i32::from(ABOUT_US_1));
            self.api.set("THRADD_STACK_1", 1_u8);
        } else if r == WHAT_ABOUT_YOU_2 {
            self.api.npc(i32::from(ABOUT_US_2));
            self.api.set("THRADD_STACK_1", 2_u8);
        } else if r == WHAT_ABOUT_URQUAN_1 {
            self.api.npc(i32::from(ABOUT_URQUAN_1));
            self.api.set("THRADD_STACK_1", 3_u8);
        } else if r == WHAT_ABOUT_URQUAN_2 {
            self.api.npc(i32::from(ABOUT_URQUAN_2));
            self.api.set("THRADD_STACK_1", 4_u8);
        } else if r == BE_FRIENDS_1 {
            self.api.npc(i32::from(NO_FRIENDS_1));
            self.api.set("THRADD_HOSTILE_STACK_3", 1_u8);
        } else if r == BE_FRIENDS_2 {
            self.api.npc(i32::from(NO_FRIENDS_2));
            self.api.disable(BE_FRIENDS_2);
        } else if r == HOW_IMPRESSED_URQUAN_1 {
            self.api.npc(i32::from(IMPRESSED_LIKE_SO_1));
            self.api.set("THRADD_HOSTILE_STACK_4", 1_u8);
        } else if r == HOW_IMPRESSED_URQUAN_2 {
            self.api.npc(i32::from(IMPRESSED_LIKE_SO_2));
            self.api.set("THRADD_MISSION", 5_u8);
        }
        if i32::from(self.api.get("THRADD_INFO")) == 0 {
            self.api
                .response(WHATS_UP_HOSTILE_1, Handler::ThraddHostile, &[]);
        } else if self.api.enabled(WHATS_UP_HOSTILE_2) {
            self.api
                .response(WHATS_UP_HOSTILE_2, Handler::ThraddHostile, &[]);
        }
        match self.api.get("THRADD_STACK_1") {
            0 => {
                self.api
                    .response(WHAT_ABOUT_YOU_1, Handler::ThraddHostile, &[]);
            }
            1 => {
                self.api
                    .response(WHAT_ABOUT_YOU_2, Handler::ThraddHostile, &[]);
            }
            2 => {
                self.api
                    .response(WHAT_ABOUT_URQUAN_1, Handler::ThraddHostile, &[]);
            }
            3 => {
                self.api
                    .response(WHAT_ABOUT_URQUAN_2, Handler::ThraddHostile, &[]);
            }
            4 => {
                if !(self.api.get("KOHR_AH_FRENZY") != 0) {
                    self.api.response(GOT_IDEA, Handler::ExitConversation, &[]);
                } else {
                    self.api.set("THRADD_STACK_1", 5_u8);
                }
            }
            _ => {}
        }
        if i32::from(self.api.get("THRADD_HOSTILE_STACK_2")) == 0 {
            self.api.response(SUBMIT_1, Handler::ExitConversation, &[]);
        } else {
            self.api.response(SUBMIT_2, Handler::ExitConversation, &[]);
        }
        if i32::from(self.api.get("THRADD_HOSTILE_STACK_3")) == 0 {
            self.api.response(BE_FRIENDS_1, Handler::ThraddHostile, &[]);
        } else if self.api.enabled(BE_FRIENDS_2) {
            self.api.response(BE_FRIENDS_2, Handler::ThraddHostile, &[]);
        }
        if i32::from(self.api.get("THRADD_MISSION")) == 4 {
            if i32::from(self.api.get("THRADD_HOSTILE_STACK_4")) == 0 {
                self.api
                    .response(HOW_IMPRESSED_URQUAN_1, Handler::ThraddHostile, &[]);
            } else {
                self.api
                    .response(HOW_IMPRESSED_URQUAN_2, Handler::ThraddHostile, &[]);
            }
        }
        if i32::from(self.api.get("THRADD_HOSTILE_STACK_5")) == 0 {
            self.api
                .response(BYE_HOSTILE_1, Handler::ExitConversation, &[]);
        } else {
            self.api
                .response(BYE_HOSTILE_2, Handler::ExitConversation, &[]);
        }
    }
    fn intro(&mut self) {
        let mut num_visits: u8;
        if i32::from((self.api.activity()) as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0_u32);
            return;
        }
        if self.api.get("AQUA_HELIX") != 0 {
            num_visits = self.api.get("HELIX_VISITS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(DIE_THIEF_1));
                }
                1 => {
                    self.api.npc(i32::from(DIE_THIEF_2));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("HELIX_VISITS", num_visits);
            self.api.segue(1_u32);
        } else if self.api.get("ILWRATH_FIGHT_THRADDASH") != 0 {
            num_visits = self.api.get("THRADD_VISITS");
            if self.api.get("THRADD_MANNER") != 0 {
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(HAVING_FUN_WITH_ILWRATH_1));
                    }
                    1 => {
                        self.api.npc(i32::from(HAVING_FUN_WITH_ILWRATH_2));
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
                        self.api.npc(i32::from(GO_AWAY_FIGHTING_ILWRATH_1));
                    }
                    1 => {
                        self.api.npc(i32::from(GO_AWAY_FIGHTING_ILWRATH_2));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
            }
            self.api.set("THRADD_VISITS", num_visits);
            self.api.segue(0_u32);
        } else if self.api.get("THRADD_MANNER") != 0 {
            let p_str0: u16;
            let p_str1: u16;
            num_visits = self.api.get("THRADD_VISITS");
            match self.api.get("THRADD_INTRO") {
                0 => {
                    self.polite_phrase(num_visits);
                }
                1 => {
                    self.rhyme_phrase(num_visits);
                }
                2 => {
                    self.pig_latin_phrase(num_visits);
                }
                3 => {
                    self.like_you_phrase(num_visits);
                }
                _ => {}
            }
            if i32::from({
                num_visits = num_visits.wrapping_add(1);
                num_visits
            }) < 4
            {
                self.api.set("THRADD_VISITS", num_visits);
            }
            if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 6)) != 0 {
                p_str0 = WELCOME_HELIX0;
                p_str1 = WELCOME_HELIX1;
            } else if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0 {
                p_str0 = WELCOME_HOMEWORLD0;
                p_str1 = WELCOME_HOMEWORLD1;
            } else {
                p_str0 = WELCOME_SPACE0;
                p_str1 = WELCOME_SPACE1;
            }
            self.api.npc(i32::from(p_str0));
            let culture = self.get_culture_name();
            self.api.npc(culture);
            self.api.npc(i32::from(p_str1));
            self.thradd_allies(0_u16);
        } else if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 6)) != 0 {
            num_visits = self.api.get("HELIX_VISITS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HOSTILE_HELIX_HELLO_1));
                }
                1 => {
                    self.api.npc(i32::from(HOSTILE_HELIX_HELLO_2));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("HELIX_VISITS", num_visits);
            self.helix_world(0_u16);
        } else if i32::from(self.api.get("THRADDASH_BODY_COUNT")) >= 25 {
            self.api.npc(i32::from(AMAZING_PERFORMANCE));
            self.thradd_worship(0_u16);
        } else {
            num_visits = self.api.get("THRADDASH_BODY_COUNT");
            if (i32::from(num_visits) >= 16) && (i32::from(self.api.get("THRADD_BODY_LEVEL")) == 1)
            {
                self.api.set("THRADD_BODY_LEVEL", 2_u8);
                self.api.npc(i32::from(IMPRESSIVE_PERFORMANCE));
            } else if (i32::from(num_visits) >= 8)
                && (i32::from(self.api.get("THRADD_BODY_LEVEL")) == 0)
            {
                self.api.set("THRADD_BODY_LEVEL", 1_u8);
                self.api.npc(i32::from(ADEQUATE_PERFORMANCE));
            }
            {
                if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0 {
                    num_visits = self.api.get("THRADD_HOME_VISITS");
                    let res = {
                        let old = num_visits;
                        num_visits = num_visits.wrapping_add(1);
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(HOSTILE_HOMEWORLD_HELLO_1));
                        }
                        1 => {
                            self.api.npc(i32::from(HOSTILE_HOMEWORLD_HELLO_2));
                        }
                        2 => {
                            self.api.npc(i32::from(HOSTILE_HOMEWORLD_HELLO_3));
                        }
                        3 => {
                            self.api.npc(i32::from(HOSTILE_HOMEWORLD_HELLO_4));
                            {
                                num_visits = num_visits.wrapping_sub(1);
                                num_visits
                            };
                        }
                        _ => {}
                    }
                    self.api.set("THRADD_HOME_VISITS", num_visits);
                } else if (i32::from({
                    num_visits = self.api.get("THRADD_MISSION");
                    num_visits
                }) == 0)
                    || (i32::from(num_visits) > 3)
                {
                    num_visits = self.api.get("THRADD_VISITS");
                    let res = {
                        let old = num_visits;
                        num_visits = num_visits.wrapping_add(1);
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(HOSTILE_SPACE_HELLO_1));
                        }
                        1 => {
                            self.api.npc(i32::from(HOSTILE_SPACE_HELLO_2));
                        }
                        2 => {
                            self.api.npc(i32::from(HOSTILE_SPACE_HELLO_3));
                        }
                        3 => {
                            self.api.npc(i32::from(HOSTILE_SPACE_HELLO_4));
                            {
                                num_visits = num_visits.wrapping_sub(1);
                                num_visits
                            };
                        }
                        _ => {}
                    }
                    self.api.set("THRADD_VISITS", num_visits);
                } else {
                    match num_visits {
                        1 => {
                            if i32::from(self.api.get("THRADD_MISSION_VISITS")) == 0 {
                                self.api.npc(i32::from(WE_GO_TO_IMPRESS_URQUAN_1));
                            } else {
                                self.api.npc(i32::from(WE_GO_TO_IMPRESS_URQUAN_2));
                            }
                        }
                        2 => {
                            if i32::from(self.api.get("THRADD_MISSION_VISITS")) == 0 {
                                self.api.npc(i32::from(WE_IMPRESSING_URQUAN_1));
                            } else {
                                self.api.npc(i32::from(WE_IMPRESSING_URQUAN_2));
                            }
                        }
                        3 => {
                            if i32::from(self.api.get("THRADD_MISSION_VISITS")) == 0 {
                                self.api.npc(i32::from(WE_IMPRESSED_URQUAN_1));
                            } else {
                                self.api.npc(i32::from(WE_IMPRESSED_URQUAN_2));
                            }
                        }
                        _ => {}
                    }
                    self.api.set("THRADD_MISSION_VISITS", 1_u8);
                }
                self.thradd_hostile(0_u16);
            }
        }
    }
    fn post_thradd_enc(&mut self) {}
    fn initialize(&mut self) {
        if (self.api.get("THRADD_MANNER") != 0) || (i32::from((self.api.activity()) as u8) == 5) {
            self.api.segue(0_u32);
        } else {
            self.api.segue(1_u32);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.thraddash.graphics".as_ptr(),
        alien_font_res: c"comm.thraddash.font".as_ptr(),
        alien_colormap_res: c"comm.thraddash.colortable".as_ptr(),
        alien_song_res: c"comm.thraddash.music".as_ptr(),
        conversation_phrases_res: c"comm.thraddash.dialogue".as_ptr(),
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
        num_animations: 8,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = {
        let mut a = animation(8, 4, 1, (56, 56), (56, 56));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[1] = animation(12, 9, 2, (42, 0), (840, 2520));
    d.alien_ambient_array[2] = animation(21, 6, 1, (42, 0), (840, 2520));
    d.alien_ambient_array[3] = {
        let mut a = animation(27, 3, 4, (42, 0), (840, 2520));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[4] = {
        let mut a = animation(30, 12, 10, (70, 0), (840, 840));
        a.block_mask = 41;
        a
    };
    d.alien_ambient_array[5] = {
        let mut a = animation(42, 5, 4, (84, 28), (840, 2520));
        a.block_mask = 80;
        a
    };
    d.alien_ambient_array[6] = {
        let mut a = animation(47, 5, 4, (84, 28), (840, 2520));
        a.block_mask = 32;
        a
    };
    d.alien_ambient_array[7] = animation(52, 4, 1, (42, 0), (42, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 7, 0, (56, 0), (70, 0));
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
    fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn thradd_allies(r: u32) {
        run(|s| {
            s.handle(
                Handler::ThraddAllies,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn thradd_demeanor(r: u32) {
        run(|s| {
            s.handle(
                Handler::ThraddDemeanor,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn thradd_culture(r: u32) {
        run(|s| {
            s.handle(
                Handler::ThraddCulture,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn helix_world(r: u32) {
        run(|s| {
            s.handle(
                Handler::HelixWorld,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn thradd_hostile(r: u32) {
        run(|s| {
            s.handle(
                Handler::ThraddHostile,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::ExitConversation => exit_conversation,
            Handler::ThraddAllies => thradd_allies,
            Handler::ThraddDemeanor => thradd_demeanor,
            Handler::ThraddCulture => thradd_culture,
            Handler::HelixWorld => helix_world,
            Handler::ThraddHostile => thradd_hostile,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_thradd_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize this encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_thradd_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize());
        desc
    }
}
pub struct ThraddDialogue;
impl super::RaceDialogue for ThraddDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_thradd_comm()) }
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
