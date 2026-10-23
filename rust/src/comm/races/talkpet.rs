//! Talkpet dialogue, ported from talkpet.c.
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
mod api;
use api::TalkpetApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    MindFuckUrquan,
    MindControl,
    PetInfo,
    PetDevice,
    CompelPlayer,
    KillPet,
    PetDeal,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: TalkpetApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: i32) {
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
    fn exit_conversation(&mut self, r: i32) {
        self.api.segue(0);
        self.api.set("SHIP_TO_COMPEL", 0_u8);
        if r == COMPEL_URQUAN {
            self.api.npc(HERE_WE_GO);
            self.api.set("URQUAN_MESSED_UP", 1_u8);
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
            self.api.set("TALKING_PET", 1_u8);
            self.api.set("TALKING_PET_ON_SHIP", 1_u8);
            self.api.set("UMGAH_ZOMBIE_BLOBBIES", 0_u8);
            self.api.set("UMGAH_VISITS", 0_u8);
            self.api.set("UMGAH_HOME_VISITS", 0_u8);
            self.api.set("ARILOU_STACK_2", 0_u8);
        }
    }
    fn mind_fuck_urquan(&mut self, r: i32) {
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
    fn mind_control_strobe(&mut self) {
        self.api.strobe();
    }
    fn mind_control(&mut self, mut r: i32) {
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
    fn pet_info(&mut self, r: i32) {
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
    fn pet_device(&mut self, r: i32) {
        let mut num_visits: i32;
        if r == WHATS_UP_ONBOARD {
            num_visits = i32::from(self.api.get("TALKING_PET_INFO"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_1);
                }
                1 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_2);
                }
                2 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_3);
                }
                3 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_4);
                }
                4 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_5);
                }
                5 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_6);
                }
                6 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_7);
                }
                7 => {
                    self.api.npc(GENERAL_INFO_ONBOARD_8);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("TALKING_PET_INFO", (num_visits) as u8);
            self.api.disable((WHATS_UP_ONBOARD) as u16);
        } else if r == ANY_SUGGESTIONS {
            num_visits = i32::from(self.api.get("TALKING_PET_SUGGESTIONS"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(SUGGESTION_1);
                }
                1 => {
                    self.api.npc(SUGGESTION_2);
                }
                2 => {
                    self.api.npc(SUGGESTION_3);
                }
                3 => {
                    self.api.npc(SUGGESTION_4);
                }
                4 => {
                    self.api.npc(SUGGESTION_5);
                }
                5 => {
                    self.api.npc(SUGGESTION_6);
                }
                6 => {
                    self.api.npc(SUGGESTION_7);
                }
                7 => {
                    self.api.npc(SUGGESTION_8);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("TALKING_PET_SUGGESTIONS", (num_visits) as u8);
            self.api.disable((ANY_SUGGESTIONS) as u16);
        } else if r == ENOUGH_INFO {
            self.api.npc(OK_ENOUGH_INFO);
        } else if r == YOU_LIED {
            self.api.npc(SO_WHAT);
            self.api.set("DNYARRI_LIED", 0_u8);
        }
        if i32::from(self.api.get("SHIP_TO_COMPEL")) != 0 {
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
        if (i32::from(self.api.get("DNYARRI_LIED")) != 0)
            && (i32::from(self.api.get("LEARNED_TALKING_PET")) != 0)
        {
            self.api
                .response((YOU_LIED) as u16, Handler::PetDevice, &[]);
        }
        self.api
            .response((BYE_ONBOARD) as u16, Handler::ExitConversation, &[]);
    }
    fn compel_player(&mut self, r: i32) {
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
            self.api.set("KNOW_UMGAH_ZOMBIES", 1_u8);
            if !(i32::from(self.api.get("TAALO_PROTECTOR_ON_SHIP")) != 0) {
                self.api.set("PLAYER_HYPNOTIZED", 1_u8);
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
        if i32::from(self.api.get("KNOW_UMGAH_ZOMBIES")) != 0 {
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
    fn kill_pet(&mut self, r: i32) {
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
    fn pet_deal(&mut self, r: i32) {
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
            self.api.set("DNYARRI_LIED", 1_u8);
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
                && !(i32::from(self.api.get("LEARNED_TALKING_PET")) != 0)
            {
                self.api
                    .response((HARD_TO_BELIEVE) as u16, Handler::PetDeal, &[]);
            } else if self.api.enabled((BULLSHIT) as u16)
                && (i32::from(self.api.get("LEARNED_TALKING_PET")) != 0)
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
    fn intro(&mut self) {
        let mut num_visits: i32;
        if (i32::from(self.api.activity()) as u8 as i32) == 5 {
            self.api.npc(OUT_TAKES);
            self.api.segue(0);
            return;
        }
        if (i32::from(self.api.activity()) as u8 as i32) == 1 {
            self.api.set("SHIP_TO_COMPEL", 0_u8);
            self.api.segue(1);
            if !((i32::from(self.api.flags()) & 32) != 0) {
                self.api.npc(HUMAN_PEP_TALK);
            } else {
                self.api.npc(CYBORG_PEP_TALK);
            }
        } else if i32::from(self.api.get("READY_TO_CONFUSE_URQUAN")) != 0 {
            self.api.set("SHIP_TO_COMPEL", 0_u8);
            self.api.set("READY_TO_CONFUSE_URQUAN", 0_u8);
            self.api.set("AWARE_OF_SAMATRA", 1_u8);
            if i32::from(self.api.get("CHMMR_BOMB_STATE")) != 3 {
                self.api.npc(HAVENT_GOT_EVERYTHING);
                if !(i32::from(self.api.get("UTWIG_BOMB_ON_SHIP")) != 0) {
                    self.api.npc(NEED_BOMB);
                } else {
                    self.api.npc(SOUP_UP_BOMB);
                }
                self.api.segue(0);
            } else if i32::from(self.api.get("URQUAN_MESSED_UP")) != 0 {
                self.api.npc(HELLO_AFTER_COMPEL_URQUAN);
                self.api.segue(0);
            } else {
                self.api.npc(I_SENSE_MY_SLAVES);
                self.mind_fuck_urquan(0_u16 as i32);
            }
        } else if i32::from(self.api.get("TALKING_PET_ON_SHIP")) != 0 {
            num_visits = i32::from(self.api.get("TALKING_PET_VISITS"));
            if !(i32::from(self.api.get("TAALO_PROTECTOR_ON_SHIP")) != 0) && (num_visits == 0) {
                {
                    num_visits = (num_visits + 1) as u8 as i32;
                    num_visits
                };
            }
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_AS_DEVICE_1);
                }
                1 => {
                    self.api.npc(HELLO_AS_DEVICE_2);
                }
                2 => {
                    self.api.npc(HELLO_AS_DEVICE_3);
                }
                3 => {
                    self.api.npc(HELLO_AS_DEVICE_4);
                }
                4 => {
                    self.api.npc(HELLO_AS_DEVICE_5);
                }
                5 => {
                    self.api.npc(HELLO_AS_DEVICE_6);
                }
                6 => {
                    self.api.npc(HELLO_AS_DEVICE_7);
                }
                7 => {
                    self.api.npc(HELLO_AS_DEVICE_8);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("TALKING_PET_VISITS", (num_visits) as u8);
            self.pet_device(0_u16 as i32);
        } else if self.api.npc_ships() {
            num_visits = i32::from(self.api.get("TALKING_PET_HOME_VISITS"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.set("UMGAH_VISITS", 0_u8);
                    self.api.npc(HELLO_AT_UMGAH);
                }
                1 => {
                    self.api.npc(HYPNOTIZE_AGAIN_1);
                }
                2 => {
                    self.api.npc(HYPNOTIZE_AGAIN_2);
                }
                3 => {
                    self.api.npc(HYPNOTIZE_AGAIN_3);
                }
                4 => {
                    self.api.npc(HYPNOTIZE_AGAIN_4);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("TALKING_PET_HOME_VISITS", (num_visits) as u8);
            if num_visits == 1 {
                self.compel_player(0_u16 as i32);
            } else if !(i32::from(self.api.get("TAALO_PROTECTOR_ON_SHIP")) != 0) {
                self.api.set("PLAYER_HYPNOTIZED", 1_u8);
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
                if (i32::from(self.api.get("TALKING_PET_HOME_VISITS")) == 0)
                    || !(i32::from(self.api.get("TAALO_PROTECTOR_ON_SHIP")) != 0)
                {
                    self.api.disable((HOW_TRUST) as u16);
                }
            }
            self.pet_deal(0_u16 as i32);
        }
    }
    fn post_talkpet_enc(&mut self) {}
    fn initialize(&mut self) {
        if (i32::from(self.api.activity()) as u8 as i32) != 1 {
            self.api.segue(0);
        } else {
            self.api.segue(1);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.talkingpet.graphics".as_ptr(),
        alien_font_res: c"comm.talkingpet.font".as_ptr(),
        alien_colormap_res: c"comm.talkingpet.colortable".as_ptr(),
        alien_song_res: c"comm.talkingpet.music".as_ptr(),
        conversation_phrases_res: c"comm.talkingpet.dialogue".as_ptr(),
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
        num_animations: 17,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(7, 3, 12, (56, 56), (840, 2520));
    d.alien_ambient_array[1] = animation(10, 3, 12, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(13, 3, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(16, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(18, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[4].block_mask = 8192;
    d.alien_ambient_array[5] = animation(21, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[5].block_mask = 20480;
    d.alien_ambient_array[6] = animation(24, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(26, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[8] = animation(28, 4, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[9] = animation(32, 3, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[10] = animation(35, 5, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[11] = animation(40, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[12] = animation(42, 6, 10, (84, 0), (840, 2520));
    d.alien_ambient_array[12].block_mask = 32;
    d.alien_ambient_array[13] = animation(48, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[13].block_mask = 16;
    d.alien_ambient_array[14] = animation(50, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = 32;
    d.alien_ambient_array[15] = animation(52, 2, 9, (84, 0), (840, 2520));
    d.alien_ambient_array[16] = animation(1, 30, 178, (42, 0), (0, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 6, 0, (56, 0), (70, 0));
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
    use super::super::script::live::LiveScript;
    use super::*;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn mind_fuck_urquan(r: u32) {
        run(|s| {
            s.handle(
                Handler::MindFuckUrquan,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn mind_control(r: u32) {
        run(|s| {
            s.handle(
                Handler::MindControl,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn pet_info(r: u32) {
        run(|s| {
            s.handle(
                Handler::PetInfo,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn pet_device(r: u32) {
        run(|s| {
            s.handle(
                Handler::PetDevice,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn compel_player(r: u32) {
        run(|s| {
            s.handle(
                Handler::CompelPlayer,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn kill_pet(r: u32) {
        run(|s| {
            s.handle(
                Handler::KillPet,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn pet_deal(r: u32) {
        run(|s| {
            s.handle(
                Handler::PetDeal,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::ExitConversation => exit_conversation,
            Handler::MindFuckUrquan => mind_fuck_urquan,
            Handler::MindControl => mind_control,
            Handler::PetInfo => pet_info,
            Handler::PetDevice => pet_device,
            Handler::CompelPlayer => compel_player,
            Handler::KillPet => kill_pet,
            Handler::PetDeal => pet_deal,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_talkpet_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_talkpet_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize());
        desc
    }
}
pub struct TalkpetDialogue;
impl super::RaceDialogue for TalkpetDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_talkpet_comm()) }
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
