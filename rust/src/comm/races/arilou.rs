//! Arilou dialogue, ported from arilouc.c.
use super::alliance::AllianceApi;
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    ArilouHome,
    AngryHomeArilou,
    AngrySpaceArilou,
    FriendlySpaceArilou,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: AllianceApi<Handler>> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::ArilouHome => self.arilou_home(r),
            Handler::AngryHomeArilou => self.angry_home_arilou(r),
            Handler::AngrySpaceArilou => self.angry_space_arilou(r),
            Handler::FriendlySpaceArilou => self.friendly_space_arilou(r),
        }
    }
    fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0_u32);
        if r == BYE_ANGRY_SPACE {
            self.api.npc(i32::from(GOODBYE_ANGRY_SPACE));
        } else if r == BYE_FRIENDLY_SPACE {
            self.api.npc(i32::from(GOODBYE_FRIENDLY_SPACE));
        } else if r == BYE_FRIENDLY_HOMEWORLD {
            self.api.npc(i32::from(GOODBYE_FRDLY_HOMEWORLD));
        } else if r == LETS_FIGHT {
            self.api.npc(i32::from(NO_FIGHT));
        } else if r == BUG_EYED_FRUITCAKES {
            self.api.npc(i32::from(WE_NEVER_FRIENDS));
            self.api.set("ARILOU_MANNER", 2_u8);
        } else if r == BEST_IF_I_KILLED_YOU {
            self.api.npc(i32::from(WICKED_HUMAN));
            self.api.set("ARILOU_MANNER", 2_u8);
        }
    }
    fn arilou_home(&mut self, r: u16) {
        let mut last_stack: u8;
        let mut p_str: [u16; 4] = [0; 4];
        last_stack = 0_u8;
        if r == CONFUSED_BY_HELLO {
            self.api.npc(i32::from(CONFUSED_RESPONSE));
        } else if r == HAPPY_BY_HELLO {
            self.api.npc(i32::from(HAPPY_RESPONSE));
        } else if r == MIFFED_BY_HELLO {
            self.api.npc(i32::from(MIFFED_RESPONSE));
        } else if r == OK_LETS_BE_FRIENDS {
            self.api.npc(i32::from(NO_ALLY_BUT_MUCH_GIVE));
        } else if r == WHAT_ABOUT_WAR {
            self.api.npc(i32::from(ABOUT_WAR));
            self.api.set("ARILOU_STACK_1", 1_u8);
        } else if r == WHAT_ABOUT_URQUAN {
            self.api.npc(i32::from(ABOUT_URQUAN));
            self.api.set("ARILOU_STACK_1", 2_u8);
        } else if r == TELL_ARILOU_ABOUT_TPET {
            self.api.npc(i32::from(BAD_NEWS_ABOUT_TPET));
            last_stack = 1_u8;
            self.api.set("ARILOU_STACK_2", 1_u8);
        } else if r == WHAT_DO_ABOUT_TPET {
            self.api.npc(i32::from(DANGEROUS_BUT_USEFUL));
            last_stack = 1_u8;
            self.api.set("ARILOU_STACK_2", 2_u8);
        } else if r == LEARNED_ABOUT_UMGAH {
            if i32::from(self.api.get("ARILOU_CHECKED_UMGAH")) != 2 {
                self.api.npc(i32::from(NO_NEWS_YET));
            } else {
                self.api.npc(i32::from(UMGAH_UNDER_COMPULSION));
                last_stack = 1_u8;
            }
            self.api.disable(LEARNED_ABOUT_UMGAH);
        } else if r == UMGAH_ACTING_WEIRD {
            self.api.npc(i32::from(WELL_GO_CHECK));
            self.api.set("ARILOU_CHECKED_UMGAH", 1_u8);
            self.api.event(1, 0_u16, 10_u16, 0_u16, 14_u8);
            self.api.disable(UMGAH_ACTING_WEIRD);
        } else if r == WHAT_DO_NOW {
            self.api.npc(i32::from(GO_FIND_OUT));
            self.api.set("ARILOU_CHECKED_UMGAH", 3_u8);
        } else if r == WHAT_DID_ON_EARTH {
            self.api.npc(i32::from(DID_THIS));
            last_stack = 2_u8;
            self.api.set("ARILOU_STACK_3", 1_u8);
        } else if r == WHY_DID_THIS {
            self.api.npc(i32::from(IDF_PARASITES));
            last_stack = 2_u8;
            self.api.set("ARILOU_STACK_3", 2_u8);
        } else if r == TELL_MORE {
            self.api.npc(i32::from(NOT_NOW));
            last_stack = 2_u8;
            self.api.set("ARILOU_STACK_3", 3_u8);
        } else if r == WHAT_GIVE_ME {
            self.api.npc(i32::from(ABOUT_PORTAL));
            last_stack = 3_u8;
            self.api.set("KNOW_ARILOU_WANT_WRECK", 1_u8);
            self.api.disable(WHAT_GIVE_ME);
        } else if r == WHAT_ABOUT_TPET {
            self.api.npc(i32::from(ABOUT_TPET));
            self.api.set("ARILOU_STACK_4", 1_u8);
        } else if r == ABOUT_PORTAL_AGAIN {
            self.api.npc(i32::from(PORTAL_AGAIN));
            self.api.disable(ABOUT_PORTAL_AGAIN);
        } else if r == GOT_IT {
            if i32::from(self.api.get("ARILOU_HOME_VISITS")) == 1 {
                self.api.npc(i32::from(CLEVER_HUMAN));
            }
            self.api.npc(i32::from(GIVE_PORTAL));
            self.api.set("PORTAL_KEY_ON_SHIP", 0_u8);
            self.api.set("PORTAL_SPAWNER", 1_u8);
            self.api.set("PORTAL_SPAWNER_ON_SHIP", 1_u8);
        }
        match self.api.get("ARILOU_STACK_1") {
            0 => {
                p_str[0_usize] = WHAT_ABOUT_WAR;
            }
            1 => {
                p_str[0_usize] = WHAT_ABOUT_URQUAN;
            }
            _ => {}
        }
        if self.api.get("TALKING_PET") != 0 {
        } else {
            if self.api.get("TALKING_PET_VISITS") != 0 {
                match self.api.get("ARILOU_STACK_2") {
                    0 => {
                        p_str[1_usize] = TELL_ARILOU_ABOUT_TPET;
                    }
                    1 => {
                        p_str[1_usize] = WHAT_DO_ABOUT_TPET;
                    }
                    _ => {}
                }
            } else if self.api.get("KNOW_UMGAH_ZOMBIES") != 0 {
                if !(self.api.get("ARILOU_CHECKED_UMGAH") != 0) {
                    p_str[1_usize] = UMGAH_ACTING_WEIRD;
                } else if self.api.enabled(LEARNED_ABOUT_UMGAH)
                    && self.api.enabled(UMGAH_ACTING_WEIRD)
                {
                    p_str[1_usize] = LEARNED_ABOUT_UMGAH;
                } else if i32::from(self.api.get("ARILOU_CHECKED_UMGAH")) == 2 {
                    p_str[1_usize] = WHAT_DO_NOW;
                }
            }
        }
        match self.api.get("ARILOU_STACK_3") {
            0 => {
                p_str[2_usize] = WHAT_DID_ON_EARTH;
            }
            1 => {
                p_str[2_usize] = WHY_DID_THIS;
            }
            2 => {
                p_str[2_usize] = TELL_MORE;
            }
            _ => {}
        }
        if !(self.api.get("KNOW_ARILOU_WANT_WRECK") != 0) {
            p_str[3_usize] = WHAT_GIVE_ME;
        } else if !(self.api.get("ARILOU_STACK_4") != 0) {
            p_str[3_usize] = WHAT_ABOUT_TPET;
        }
        if p_str[(last_stack) as usize] != 0 {
            self.api
                .response(p_str[(last_stack) as usize], Handler::ArilouHome, &[]);
        }
        for i in 0..4 {
            if (i != i32::from(last_stack)) && (p_str[(i) as usize] != 0) {
                self.api
                    .response(p_str[(i) as usize], Handler::ArilouHome, &[]);
            }
        }
        if self.api.get("KNOW_ARILOU_WANT_WRECK") != 0 {
            if self.api.get("PORTAL_KEY_ON_SHIP") != 0 {
                self.api.response(GOT_IT, Handler::ArilouHome, &[]);
            } else if self.api.enabled(ABOUT_PORTAL_AGAIN) && !(self.api.get("PORTAL_SPAWNER") != 0)
            {
                self.api
                    .response(ABOUT_PORTAL_AGAIN, Handler::ArilouHome, &[]);
            }
        }
        if i32::from(self.api.get("ARILOU_MANNER")) != 3 {
            self.api
                .response(BEST_IF_I_KILLED_YOU, Handler::ExitConversation, &[]);
        }
        self.api
            .response(BYE_FRIENDLY_HOMEWORLD, Handler::ExitConversation, &[]);
    }
    fn angry_home_arilou(&mut self, r: u16) {
        if r == INVADERS_FROM_MARS {
            self.api.npc(i32::from(HAD_OUR_REASONS));
            self.api.disable(INVADERS_FROM_MARS);
        } else if r == WHY_SHOULD_I_TRUST {
            self.api.npc(i32::from(TRUST_BECAUSE));
            self.api.disable(WHY_SHOULD_I_TRUST);
        } else if r == WHAT_ABOUT_INTERFERENCE {
            self.api.npc(i32::from(INTERFERENCE_NECESSARY));
            self.api.disable(WHAT_ABOUT_INTERFERENCE);
        } else if r == I_JUST_LIKE_TO_LEAVE {
            self.api.npc(i32::from(SORRY_NO_LEAVE));
            self.api.disable(I_JUST_LIKE_TO_LEAVE);
        }
        if self.api.enabled(INVADERS_FROM_MARS) {
            self.api
                .response(INVADERS_FROM_MARS, Handler::AngryHomeArilou, &[]);
        } else {
            self.api
                .response(BUG_EYED_FRUITCAKES, Handler::ExitConversation, &[]);
        }
        if self.api.enabled(WHY_SHOULD_I_TRUST) {
            self.api
                .response(WHY_SHOULD_I_TRUST, Handler::AngryHomeArilou, &[]);
        } else if self.api.enabled(WHAT_ABOUT_INTERFERENCE) {
            self.api
                .response(WHAT_ABOUT_INTERFERENCE, Handler::AngryHomeArilou, &[]);
        }
        self.api
            .response(OK_LETS_BE_FRIENDS, Handler::ArilouHome, &[]);
        self.api
            .response(I_JUST_LIKE_TO_LEAVE, Handler::AngryHomeArilou, &[]);
    }
    fn angry_space_arilou(&mut self, r: u16) {
        if r == IM_SORRY {
            self.api.npc(i32::from(APOLOGIZE_AT_HOMEWORLD));
            self.api.disable(IM_SORRY);
        }
        self.api
            .response(LETS_FIGHT, Handler::ExitConversation, &[]);
        if self.api.enabled(IM_SORRY) {
            self.api.response(IM_SORRY, Handler::AngrySpaceArilou, &[]);
        }
        self.api
            .response(BYE_ANGRY_SPACE, Handler::ExitConversation, &[]);
    }
    fn friendly_space_arilou(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == CONFUSED_BY_HELLO {
            self.api.npc(i32::from(CONFUSED_RESPONSE));
        } else if r == HAPPY_BY_HELLO {
            self.api.npc(i32::from(HAPPY_RESPONSE));
        } else if r == MIFFED_BY_HELLO {
            self.api.npc(i32::from(MIFFED_RESPONSE));
        } else if (r == WHATS_UP_1) || (r == WHATS_UP_2) {
            num_visits = self.api.get("ARILOU_INFO");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_2));
                }
                2 => {
                    self.api.npc(i32::from(GENERAL_INFO_3));
                }
                3 => {
                    self.api.npc(i32::from(GENERAL_INFO_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("ARILOU_INFO", num_visits);
            self.api.disable(WHATS_UP_2);
        } else if r == WHY_YOU_HERE {
            self.api.npc(i32::from(LEARN_THINGS));
            self.api.set("ARILOU_STACK_5", 1_u8);
        } else if r == WHAT_THINGS {
            self.api.npc(i32::from(THESE_THINGS));
            self.api.set("ARILOU_STACK_5", 2_u8);
        } else if r == WHY_DO_IT {
            self.api.npc(i32::from(DO_IT_BECAUSE));
            self.api.set("ARILOU_STACK_5", 3_u8);
        } else if (r == GIVE_ME_INFO_1) || (r == GIVE_ME_INFO_2) {
            num_visits = self.api.get("ARILOU_HINTS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(ARILOU_HINTS_1));
                }
                1 => {
                    self.api.npc(i32::from(ARILOU_HINTS_2));
                    if i32::from(self.api.get("KNOW_ABOUT_SHATTERED")) < 2 {
                        self.api.set("KNOW_ABOUT_SHATTERED", 2_u8);
                    }
                }
                2 => {
                    self.api.npc(i32::from(ARILOU_HINTS_3));
                    self.api.set("KNOW_URQUAN_STORY", 1_u8);
                    self.api.set("KNOW_KOHR_AH_STORY", 1_u8);
                }
                3 => {
                    self.api.npc(i32::from(ARILOU_HINTS_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("ARILOU_HINTS", num_visits);
            self.api.disable(GIVE_ME_INFO_2);
        }
        match self.api.get("ARILOU_STACK_5") {
            0 => {
                self.api
                    .response(WHY_YOU_HERE, Handler::FriendlySpaceArilou, &[]);
            }
            1 => {
                self.api
                    .response(WHAT_THINGS, Handler::FriendlySpaceArilou, &[]);
            }
            2 => {
                self.api
                    .response(WHY_DO_IT, Handler::FriendlySpaceArilou, &[]);
            }
            _ => {}
        }
        if self.api.enabled(WHATS_UP_2) {
            if i32::from(self.api.get("ARILOU_INFO")) == 0 {
                self.api
                    .response(WHATS_UP_1, Handler::FriendlySpaceArilou, &[]);
            } else {
                self.api
                    .response(WHATS_UP_2, Handler::FriendlySpaceArilou, &[]);
            }
        }
        if self.api.enabled(GIVE_ME_INFO_2) {
            if i32::from(self.api.get("ARILOU_HINTS")) == 0 {
                self.api
                    .response(GIVE_ME_INFO_1, Handler::FriendlySpaceArilou, &[]);
            } else {
                self.api
                    .response(GIVE_ME_INFO_2, Handler::FriendlySpaceArilou, &[]);
            }
        }
        self.api
            .response(BYE_FRIENDLY_SPACE, Handler::ExitConversation, &[]);
    }
    fn intro(&mut self) {
        let mut num_visits: u8;
        if i32::from((self.api.activity()) as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0_u32);
            return;
        } else if !(self.api.get("MET_ARILOU") != 0) {
            let resp_func: Handler;
            if i32::from(self.api.get("ARILOU_SPACE_SIDE")) <= 1 {
                self.api.npc(i32::from(INIT_HELLO));
                resp_func = Handler::FriendlySpaceArilou;
            } else {
                self.api.npc(i32::from(FRDLY_HOMEWORLD_HELLO_1));
                resp_func = Handler::ArilouHome;
                self.api.set("ARILOU_HOME_VISITS", 1_u8);
            }
            self.api.response(CONFUSED_BY_HELLO, resp_func, &[]);
            self.api.response(HAPPY_BY_HELLO, resp_func, &[]);
            self.api.response(MIFFED_BY_HELLO, resp_func, &[]);
            self.api.set("MET_ARILOU", 1_u8);
            return;
        }
        let manner: u8 = self.api.get("ARILOU_MANNER");
        if i32::from(manner) == 2 {
            num_visits = self.api.get("ARILOU_VISITS");
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HOSTILE_GOODBYE_1));
                }
                1 => {
                    self.api.npc(i32::from(HOSTILE_GOODBYE_2));
                }
                2 => {
                    self.api.npc(i32::from(HOSTILE_GOODBYE_3));
                }
                3 => {
                    self.api.npc(i32::from(HOSTILE_GOODBYE_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("ARILOU_VISITS", num_visits);
            self.api.segue(0_u32);
        } else if i32::from(manner) == 1 {
            if i32::from(self.api.get("ARILOU_SPACE_SIDE")) > 1 {
                self.api.npc(i32::from(INIT_ANGRY_HWLD_HELLO));
                self.api.set("ARILOU_HOME_VISITS", 1_u8);
                self.angry_home_arilou(0_u16);
            } else {
                num_visits = self.api.get("ARILOU_VISITS");
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
                self.api.set("ARILOU_VISITS", num_visits);
                self.angry_space_arilou(0_u16);
            }
        } else {
            if i32::from(self.api.get("ARILOU_SPACE_SIDE")) <= 1 {
                num_visits = self.api.get("ARILOU_VISITS");
                let res = {
                    let old = num_visits;
                    num_visits = num_visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_1));
                    }
                    1 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_2));
                    }
                    2 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_3));
                    }
                    3 => {
                        self.api.npc(i32::from(FRIENDLY_SPACE_HELLO_4));
                        {
                            num_visits = num_visits.wrapping_sub(1);
                            num_visits
                        };
                    }
                    _ => {}
                }
                self.api.set("ARILOU_VISITS", num_visits);
                self.friendly_space_arilou(0_u16);
            } else {
                if !(self.api.get("PORTAL_SPAWNER") != 0)
                    && (self.api.get("KNOW_ARILOU_WANT_WRECK") != 0)
                {
                    num_visits = self.api.get("NO_PORTAL_VISITS");
                    let res = {
                        let old = num_visits;
                        num_visits = num_visits.wrapping_add(1);
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(GOT_PART_YET_1));
                        }
                        1 => {
                            self.api.npc(i32::from(GOT_PART_YET_1));
                            {
                                num_visits = num_visits.wrapping_sub(1);
                                num_visits
                            };
                        }
                        _ => {}
                    }
                    self.api.set("NO_PORTAL_VISITS", num_visits);
                } else {
                    num_visits = self.api.get("ARILOU_HOME_VISITS");
                    let res = {
                        let old = num_visits;
                        num_visits = num_visits.wrapping_add(1);
                        old
                    };
                    match res {
                        0 => {
                            self.api.npc(i32::from(FRDLY_HOMEWORLD_HELLO_1));
                        }
                        1 => {
                            self.api.npc(i32::from(FRDLY_HOMEWORLD_HELLO_2));
                        }
                        2 => {
                            self.api.npc(i32::from(FRDLY_HOMEWORLD_HELLO_3));
                        }
                        3 => {
                            self.api.npc(i32::from(FRDLY_HOMEWORLD_HELLO_4));
                            {
                                num_visits = num_visits.wrapping_sub(1);
                                num_visits
                            };
                        }
                        _ => {}
                    }
                    self.api.set("ARILOU_HOME_VISITS", num_visits);
                }
                self.arilou_home(0_u16);
            }
        }
    }
    fn post_arilou_enc(&mut self) {
        let manner: u8;
        if ((self.api.get_segue()) as i32 == 1)
            && (i32::from({
                manner = self.api.get("ARILOU_MANNER");
                manner
            }) != 2)
        {
            self.api.set("ARILOU_MANNER", 1_u8);
            if i32::from(manner) != 1 {
                self.api.set("ARILOU_VISITS", 0_u8);
                self.api.set("ARILOU_HOME_VISITS", 0_u8);
            }
        }
        if (i32::from(self.api.get("ARILOU_SPACE_SIDE")) > 1)
            && (i32::from(self.api.get("ARILOU_HOME_VISITS")) <= 1)
        {
            self.api.set("UMGAH_ZOMBIE_BLOBBIES", 1_u8);
            self.api.set("UMGAH_VISITS", 0_u8);
            self.api.set("UMGAH_HOME_VISITS", 0_u8);
            if i32::from(self.api.get("ARILOU_MANNER")) < 2 {
                self.api.set("ARILOU_MANNER", 3_u8);
            }
        }
    }
    fn initialize(&mut self) {
        if ((i32::from(self.api.get("ARILOU_SPACE_SIDE")) > 1)
            || (i32::from(self.api.get("ARILOU_MANNER")) == 3))
            || (i32::from((self.api.activity()) as u8) == 5)
        {
            self.api.segue(0_u32);
        } else {
            self.api.segue(1_u32);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.arilou.graphics".as_ptr(),
        alien_font_res: c"comm.arilou.font".as_ptr(),
        alien_colormap_res: c"comm.arilou.colortable".as_ptr(),
        alien_song_res: c"comm.arilou.music".as_ptr(),
        conversation_phrases_res: c"comm.arilou.dialogue".as_ptr(),
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
        num_animations: 20,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = {
        let mut a = animation(4, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 65538;
        a
    };
    d.alien_ambient_array[1] = {
        let mut a = animation(13, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 65537;
        a
    };
    d.alien_ambient_array[2] = {
        let mut a = animation(22, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 65536;
        a
    };
    d.alien_ambient_array[3] = {
        let mut a = animation(31, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[4] = {
        let mut a = animation(40, 10, 2, (28, 0), (840, 2520));
        a.block_mask = 8;
        a
    };
    d.alien_ambient_array[5] = {
        let mut a = animation(50, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 128;
        a
    };
    d.alien_ambient_array[6] = animation(59, 8, 2, (28, 0), (840, 2520));
    d.alien_ambient_array[7] = {
        let mut a = animation(67, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 32;
        a
    };
    d.alien_ambient_array[8] = {
        let mut a = animation(76, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 512;
        a
    };
    d.alien_ambient_array[9] = {
        let mut a = animation(85, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 256;
        a
    };
    d.alien_ambient_array[10] = animation(94, 9, 2, (28, 0), (840, 2520));
    d.alien_ambient_array[11] = {
        let mut a = animation(103, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 8192;
        a
    };
    d.alien_ambient_array[12] = animation(112, 9, 2, (28, 0), (840, 2520));
    d.alien_ambient_array[13] = {
        let mut a = animation(121, 8, 2, (28, 0), (840, 2520));
        a.block_mask = 2048;
        a
    };
    d.alien_ambient_array[14] = {
        let mut a = animation(129, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 32768;
        a
    };
    d.alien_ambient_array[15] = {
        let mut a = animation(138, 8, 2, (28, 0), (840, 2520));
        a.block_mask = 16384;
        a
    };
    d.alien_ambient_array[16] = {
        let mut a = animation(146, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 7;
        a
    };
    d.alien_ambient_array[17] = {
        let mut a = animation(155, 2, 10, (56, 0), (840, 2520));
        a.block_mask = 524288;
        a
    };
    d.alien_ambient_array[18] = {
        let mut a = animation(157, 2, 10, (56, 0), (840, 2520));
        a.block_mask = 524288;
        a
    };
    d.alien_ambient_array[19] = {
        let mut a = animation(159, 4, 2, (70, 0), (840, 2520));
        a.block_mask = 393216;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}
#[cfg(not(test))]
mod live {
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
    fn arilou_home(r: u32) {
        run(|s| {
            s.handle(
                Handler::ArilouHome,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn angry_home_arilou(r: u32) {
        run(|s| {
            s.handle(
                Handler::AngryHomeArilou,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn angry_space_arilou(r: u32) {
        run(|s| {
            s.handle(
                Handler::AngrySpaceArilou,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn friendly_space_arilou(r: u32) {
        run(|s| {
            s.handle(
                Handler::FriendlySpaceArilou,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::ExitConversation => exit_conversation,
            Handler::ArilouHome => arilou_home,
            Handler::AngryHomeArilou => angry_home_arilou,
            Handler::AngrySpaceArilou => angry_space_arilou,
            Handler::FriendlySpaceArilou => friendly_space_arilou,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_arilou_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize this encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_arilou_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize());
        desc
    }
}
pub struct ArilouDialogue;
impl super::RaceDialogue for ArilouDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_arilou_comm()) }
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
