//! Thradd dialogue, ported from thraddc.c.
pub(crate) use super::alliance::AllianceApi;
pub(crate) use super::script::GLOBAL_PLAYER_NAME;
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    ExitConversation,
    ThraddAllies,
    ThraddDemeanor,
    ThraddCulture,
    HelixWorld,
    ThraddHostile,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: AllianceApi<Handler>> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::ThraddAllies => self.thradd_allies(r),
            Handler::ThraddDemeanor => self.thradd_demeanor(r),
            Handler::ThraddCulture => self.thradd_culture(r),
            Handler::HelixWorld => self.helix_world(r),
            Handler::ThraddHostile => self.thradd_hostile(r),
        }
    }
    pub(crate) fn get_culture_name(&self) -> i32 {
        match self.api.get(StoryKey::THRADD_CULTURE) {
            1 => i32::from(CULTURE),
            2 => i32::from(FAT_JERKS),
            3 => i32::from(SLAVE_EMPIRE),
            _ => panic!("unknown Thraddash culture"),
        }
    }
    pub(crate) fn polite_phrase(&mut self, which_phrase: u8) {
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
    pub(crate) fn rhyme_phrase(&mut self, which_phrase: u8) {
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
    pub(crate) fn pig_latin_phrase(&mut self, which_phrase: u8) {
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
    pub(crate) fn like_you_phrase(&mut self, which_phrase: u8) {
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
    pub(crate) fn exit_conversation(&mut self, r: u16) {
        self.api.segue(1_u32);
        if r == BYE_HOSTILE_2 {
            self.api.npc(i32::from(GOODBYE_HOSTILE_2));
        } else if r == BYE_HOSTILE_1 {
            self.api.npc(i32::from(GOODBYE_HOSTILE_1));
            self.api.set(StoryKey::THRADD_HOSTILE_STACK_5, 1_u8);
        } else if r == SUBMIT_1 {
            self.api.npc(i32::from(NO_SUBMIT_1));
            self.api.set(StoryKey::THRADD_HOSTILE_STACK_2, 1_u8);
        } else if r == SUBMIT_2 {
            self.api.npc(i32::from(NO_SUBMIT_2));
        } else if r == GOT_IDEA {
            self.api.npc(i32::from(GOOD_IDEA));
            self.api.segue(0_u32);
            self.api.event(1, 0_u16, 0_u16, 0_u16, 5_u8);
            self.api.set(StoryKey::THRADD_STACK_1, 5_u8);
        } else if r == BYE_HOSTILE_HELIX {
            self.api.npc(i32::from(GOODBYE_HOSTILE_HELIX));
        } else if r == BYE_ALLY {
            let mut num_visits: u8;
            num_visits = self.api.get(StoryKey::THRADD_STACK_1);
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
            self.api.set(StoryKey::THRADD_STACK_1, num_visits);
            self.api.segue(0_u32);
        } else if r == MAY_I_LAND {
            self.api.npc(i32::from(SURE_LAND));
            self.api.set(StoryKey::HELIX_UNPROTECTED, 1_u8);
            self.api.segue(0_u32);
        } else if r == DEMAND_TO_LAND {
            self.api.npc(i32::from(NO_DEMAND));
        } else if r == I_NEED_TO_LAND_LIE {
            self.api.npc(i32::from(CAUGHT_LIE));
        } else {
            if r == CONTEMPLATIVE {
                self.api.npc(i32::from(OK_CONTEMPLATIVE));
                self.api.set(StoryKey::THRADD_DEMEANOR, 0_u8);
            } else if r == FRIENDLY {
                self.api.npc(i32::from(OK_FRIENDLY));
                self.api.set(StoryKey::THRADD_DEMEANOR, 1_u8);
            } else if r == WACKY {
                self.api.npc(i32::from(OK_WACKY));
                self.api.set(StoryKey::THRADD_DEMEANOR, 2_u8);
            } else if r == JUST_LIKE_US {
                self.api.npc(i32::from(OK_JUST_LIKE_YOU));
                self.api.set(StoryKey::THRADD_DEMEANOR, 3_u8);
            }
            self.api.npc(i32::from(WORK_TO_DO));
            self.api.segue(0_u32);
        }
    }
    pub(crate) fn thradd_allies(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHY_YOU_HERE_ALLY {
            self.api.npc(i32::from(GUARDING_HELIX_ALLY));
            self.api.disable(WHY_YOU_HERE_ALLY);
        } else if r == WHATS_HELIX_ALLY {
            self.api.npc(i32::from(HELIX_IS_ALLY));
            self.api.disable(WHATS_HELIX_ALLY);
        } else if r == WHATS_UP_ALLY {
            num_visits = self.api.get(StoryKey::THRADD_INFO);
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
            self.api.set(StoryKey::THRADD_INFO, num_visits);
            self.api.disable(WHATS_UP_ALLY);
        } else if r == HOW_GOES_CULTURE {
            num_visits = self.api.get(StoryKey::THRADD_DEMEANOR);
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
            self.api.set(StoryKey::THRADD_DEMEANOR, num_visits);
            self.api.disable(HOW_GOES_CULTURE);
        }
        if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 6)) != 0 {
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
    pub(crate) fn thradd_demeanor(&mut self, r: u16) {
        if r == YOU_DECIDE {
            self.api.npc(i32::from(OK_CULTURE_20));
            self.api.set(StoryKey::THRADD_CULTURE, 1_u8);
        } else if r == FAT {
            self.api.npc(i32::from(OK_FAT));
            self.api.set(StoryKey::THRADD_CULTURE, 2_u8);
        } else if r == THE_SLAVE_EMPIRE0 {
            self.api.set(StoryKey::THRADD_CULTURE, 3_u8);
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
    pub(crate) fn thradd_culture(&mut self, r: u16) {
        if r == BE_POLITE {
            self.api.npc(i32::from(OK_POLITE));
            self.api.set(StoryKey::THRADD_INTRO, 0_u8);
        } else if r == USE_RHYMES {
            self.api.npc(i32::from(OK_RHYMES));
            self.api.set(StoryKey::THRADD_INTRO, 1_u8);
        } else if r == SPEAK_PIG_LATIN {
            self.api.npc(i32::from(OK_PIG_LATIN));
            self.api.set(StoryKey::THRADD_INTRO, 2_u8);
        } else if r == JUST_THE_WAY_WE_DO {
            self.api.npc(i32::from(OK_WAY_YOU_DO));
            self.api.set(StoryKey::THRADD_INTRO, 3_u8);
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
    pub(crate) fn thradd_worship(&mut self, _r: u16) {
        self.api.set(StoryKey::THRADD_VISITS, 0_u8);
        self.api.set(StoryKey::THRADD_MANNER, 1_u8);
        self.api.set(StoryKey::THRADD_STACK_1, 0_u8);
        self.api
            .set_allied((crate::comm::dispatch::ship::THRADDASH_SHIP) as u16, true);
        self.api.response(BE_POLITE, Handler::ThraddCulture, &[]);
        self.api
            .response(SPEAK_PIG_LATIN, Handler::ThraddCulture, &[]);
        self.api.response(USE_RHYMES, Handler::ThraddCulture, &[]);
        self.api
            .response(JUST_THE_WAY_WE_DO, Handler::ThraddCulture, &[]);
    }
    pub(crate) fn helix_world(&mut self, r: u16) {
        if r == WHY_YOU_HERE_HOSTILE {
            self.api.npc(i32::from(NONE_OF_YOUR_CONCERN));
            self.api.set(StoryKey::THRADD_CULTURE, 1_u8);
        } else if r == WHAT_ABOUT_THIS_WORLD {
            self.api.npc(i32::from(BLUE_HELIX));
            self.api.set(StoryKey::THRADD_INTRO, 1_u8);
        } else if r == WHATS_HELIX_HOSTILE {
            self.api.npc(i32::from(HELIX_IS_HOSTILE));
            self.api.set(StoryKey::THRADD_INTRO, 2_u8);
        } else if r == I_NEED_TO_LAND_LIE {
            self.api.npc(i32::from(CAUGHT_LIE));
            self.api.set(StoryKey::THRADD_DEMEANOR, 1_u8);
        }
        if !(self.api.get(StoryKey::THRADD_CULTURE) != 0) {
            self.api
                .response(WHY_YOU_HERE_HOSTILE, Handler::HelixWorld, &[]);
        } else {
            self.api
                .response(DEMAND_TO_LAND, Handler::ExitConversation, &[]);
        }
        match self.api.get(StoryKey::THRADD_INTRO) {
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
        if !(self.api.get(StoryKey::THRADD_DEMEANOR) != 0) {
            self.api
                .response(I_NEED_TO_LAND_LIE, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_HOSTILE_HELIX, Handler::ExitConversation, &[]);
    }
    pub(crate) fn thradd_hostile(&mut self, r: u16) {
        if r == WHATS_UP_HOSTILE_1 {
            self.api.npc(i32::from(GENERAL_INFO_HOSTILE_1));
            self.api.set(StoryKey::THRADD_INFO, 1_u8);
            self.api.disable(WHATS_UP_HOSTILE_2);
        } else if r == WHATS_UP_HOSTILE_2 {
            let mut num_visits: u8;
            num_visits = self.api.get(StoryKey::THRADD_INFO);
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
            self.api.set(StoryKey::THRADD_INFO, num_visits);
            self.api.disable(WHATS_UP_HOSTILE_2);
        } else if r == WHAT_ABOUT_YOU_1 {
            self.api.npc(i32::from(ABOUT_US_1));
            self.api.set(StoryKey::THRADD_STACK_1, 1_u8);
        } else if r == WHAT_ABOUT_YOU_2 {
            self.api.npc(i32::from(ABOUT_US_2));
            self.api.set(StoryKey::THRADD_STACK_1, 2_u8);
        } else if r == WHAT_ABOUT_URQUAN_1 {
            self.api.npc(i32::from(ABOUT_URQUAN_1));
            self.api.set(StoryKey::THRADD_STACK_1, 3_u8);
        } else if r == WHAT_ABOUT_URQUAN_2 {
            self.api.npc(i32::from(ABOUT_URQUAN_2));
            self.api.set(StoryKey::THRADD_STACK_1, 4_u8);
        } else if r == BE_FRIENDS_1 {
            self.api.npc(i32::from(NO_FRIENDS_1));
            self.api.set(StoryKey::THRADD_HOSTILE_STACK_3, 1_u8);
        } else if r == BE_FRIENDS_2 {
            self.api.npc(i32::from(NO_FRIENDS_2));
            self.api.disable(BE_FRIENDS_2);
        } else if r == HOW_IMPRESSED_URQUAN_1 {
            self.api.npc(i32::from(IMPRESSED_LIKE_SO_1));
            self.api.set(StoryKey::THRADD_HOSTILE_STACK_4, 1_u8);
        } else if r == HOW_IMPRESSED_URQUAN_2 {
            self.api.npc(i32::from(IMPRESSED_LIKE_SO_2));
            self.api.set(StoryKey::THRADD_MISSION, 5_u8);
        }
        if i32::from(self.api.get(StoryKey::THRADD_INFO)) == 0 {
            self.api
                .response(WHATS_UP_HOSTILE_1, Handler::ThraddHostile, &[]);
        } else if self.api.enabled(WHATS_UP_HOSTILE_2) {
            self.api
                .response(WHATS_UP_HOSTILE_2, Handler::ThraddHostile, &[]);
        }
        match self.api.get(StoryKey::THRADD_STACK_1) {
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
                if !(self.api.get(StoryKey::KOHR_AH_FRENZY) != 0) {
                    self.api.response(GOT_IDEA, Handler::ExitConversation, &[]);
                } else {
                    self.api.set(StoryKey::THRADD_STACK_1, 5_u8);
                }
            }
            _ => {}
        }
        if i32::from(self.api.get(StoryKey::THRADD_HOSTILE_STACK_2)) == 0 {
            self.api.response(SUBMIT_1, Handler::ExitConversation, &[]);
        } else {
            self.api.response(SUBMIT_2, Handler::ExitConversation, &[]);
        }
        if i32::from(self.api.get(StoryKey::THRADD_HOSTILE_STACK_3)) == 0 {
            self.api.response(BE_FRIENDS_1, Handler::ThraddHostile, &[]);
        } else if self.api.enabled(BE_FRIENDS_2) {
            self.api.response(BE_FRIENDS_2, Handler::ThraddHostile, &[]);
        }
        if i32::from(self.api.get(StoryKey::THRADD_MISSION)) == 4 {
            if i32::from(self.api.get(StoryKey::THRADD_HOSTILE_STACK_4)) == 0 {
                self.api
                    .response(HOW_IMPRESSED_URQUAN_1, Handler::ThraddHostile, &[]);
            } else {
                self.api
                    .response(HOW_IMPRESSED_URQUAN_2, Handler::ThraddHostile, &[]);
            }
        }
        if i32::from(self.api.get(StoryKey::THRADD_HOSTILE_STACK_5)) == 0 {
            self.api
                .response(BYE_HOSTILE_1, Handler::ExitConversation, &[]);
        } else {
            self.api
                .response(BYE_HOSTILE_2, Handler::ExitConversation, &[]);
        }
    }
    pub(crate) fn intro(&mut self) {
        let mut visits: u8;
        if (self.api.activity() as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0_u32);
            return;
        }
        if self.api.get(StoryKey::AQUA_HELIX) != 0 {
            visits = self.api.get(StoryKey::HELIX_VISITS);
            let res = {
                let old = visits;
                visits = visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(DIE_THIEF_1));
                }
                1 => {
                    self.api.npc(i32::from(DIE_THIEF_2));
                    {
                        visits = visits.wrapping_sub(1);
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::HELIX_VISITS, visits);
            self.api.segue(1_u32);
        } else if self.api.get(StoryKey::ILWRATH_FIGHT_THRADDASH) != 0 {
            visits = self.api.get(StoryKey::THRADD_VISITS);
            if self.api.get(StoryKey::THRADD_MANNER) != 0 {
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(HAVING_FUN_WITH_ILWRATH_1));
                    }
                    1 => {
                        self.api.npc(i32::from(HAVING_FUN_WITH_ILWRATH_2));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
            } else {
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(GO_AWAY_FIGHTING_ILWRATH_1));
                    }
                    1 => {
                        self.api.npc(i32::from(GO_AWAY_FIGHTING_ILWRATH_2));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
            }
            self.api.set(StoryKey::THRADD_VISITS, visits);
            self.api.segue(0_u32);
        } else if self.api.get(StoryKey::THRADD_MANNER) != 0 {
            let p_str0: u16;
            let p_str1: u16;
            visits = self.api.get(StoryKey::THRADD_VISITS);
            match self.api.get(StoryKey::THRADD_INTRO) {
                0 => {
                    self.polite_phrase(visits);
                }
                1 => {
                    self.rhyme_phrase(visits);
                }
                2 => {
                    self.pig_latin_phrase(visits);
                }
                3 => {
                    self.like_you_phrase(visits);
                }
                _ => {}
            }
            if i32::from({
                visits = visits.wrapping_add(1);
                visits
            }) < 4
            {
                self.api.set(StoryKey::THRADD_VISITS, visits);
            }
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 6)) != 0 {
                p_str0 = WELCOME_HELIX0;
                p_str1 = WELCOME_HELIX1;
            } else if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
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
        } else if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 6)) != 0 {
            visits = self.api.get(StoryKey::HELIX_VISITS);
            let res = {
                let old = visits;
                visits = visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HOSTILE_HELIX_HELLO_1));
                }
                1 => {
                    self.api.npc(i32::from(HOSTILE_HELIX_HELLO_2));
                    {
                        visits = visits.wrapping_sub(1);
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::HELIX_VISITS, visits);
            self.helix_world(0_u16);
        } else if i32::from(self.api.get(StoryKey::THRADDASH_BODY_COUNT)) >= 25 {
            self.api.npc(i32::from(AMAZING_PERFORMANCE));
            self.thradd_worship(0_u16);
        } else {
            visits = self.api.get(StoryKey::THRADDASH_BODY_COUNT);
            if (i32::from(visits) >= 16)
                && (i32::from(self.api.get(StoryKey::THRADD_BODY_LEVEL)) == 1)
            {
                self.api.set(StoryKey::THRADD_BODY_LEVEL, 2_u8);
                self.api.npc(i32::from(IMPRESSIVE_PERFORMANCE));
            } else if (i32::from(visits) >= 8) && (self.api.get(StoryKey::THRADD_BODY_LEVEL) == 0) {
                self.api.set(StoryKey::THRADD_BODY_LEVEL, 1_u8);
                self.api.npc(i32::from(ADEQUATE_PERFORMANCE));
            }
            {
                if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                    visits = self.api.get(StoryKey::THRADD_HOME_VISITS);
                    let res = {
                        let old = visits;
                        visits = visits.wrapping_add(1);
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
                                visits = visits.wrapping_sub(1);
                                visits
                            };
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::THRADD_HOME_VISITS, visits);
                } else if (i32::from({
                    visits = self.api.get(StoryKey::THRADD_MISSION);
                    visits
                }) == 0)
                    || (i32::from(visits) > 3)
                {
                    visits = self.api.get(StoryKey::THRADD_VISITS);
                    let res = {
                        let old = visits;
                        visits = visits.wrapping_add(1);
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
                                visits = visits.wrapping_sub(1);
                                visits
                            };
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::THRADD_VISITS, visits);
                } else {
                    match visits {
                        1 => {
                            if self.api.get(StoryKey::THRADD_MISSION_VISITS) == 0 {
                                self.api.npc(i32::from(WE_GO_TO_IMPRESS_URQUAN_1));
                            } else {
                                self.api.npc(i32::from(WE_GO_TO_IMPRESS_URQUAN_2));
                            }
                        }
                        2 => {
                            if self.api.get(StoryKey::THRADD_MISSION_VISITS) == 0 {
                                self.api.npc(i32::from(WE_IMPRESSING_URQUAN_1));
                            } else {
                                self.api.npc(i32::from(WE_IMPRESSING_URQUAN_2));
                            }
                        }
                        3 => {
                            if self.api.get(StoryKey::THRADD_MISSION_VISITS) == 0 {
                                self.api.npc(i32::from(WE_IMPRESSED_URQUAN_1));
                            } else {
                                self.api.npc(i32::from(WE_IMPRESSED_URQUAN_2));
                            }
                        }
                        _ => {}
                    }
                    self.api.set(StoryKey::THRADD_MISSION_VISITS, 1_u8);
                }
                self.thradd_hostile(0_u16);
            }
        }
    }
    pub(crate) fn post_thradd_enc(&mut self) {}
    pub(crate) fn initialize(&mut self) {
        if (self.api.get(StoryKey::THRADD_MANNER) != 0) || ((self.api.activity() as u8) == 5) {
            self.api.segue(0_u32);
        } else {
            self.api.segue(1_u32);
        }
    }
}

#[cfg(test)]
use crate::comm::descriptors::thradd::descriptor;

#[cfg(test)]
mod tests;
