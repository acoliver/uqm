//! Supox dialogue, ported from the original C script.
use super::alliance::AllianceApi;
use super::script::{animation, as_comm_data, GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
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
    NeutralSupox,
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
            Handler::NeutralSupox => self.neutral_supox(r),
        }
    }
    fn initialize(&mut self, _desc: &mut CLocData) {
        self.api.segue(u32::from(
            self.api.get("SUPOX_HOSTILE") != 0 && self.api.activity() as u8 != 5,
        ));
    }
    fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0);
        if r == BYE_NEUTRAL {
            self.api.npc(i32::from(GOODBYE_NEUTRAL));
        } else if r == WHAT_DO_I_DO_NOW {
            self.api.npc(i32::from(FIX_IT));
        } else if r == THANKS_NOW_WE_EAT_YOU {
            self.api.npc(i32::from(HIDEOUS_MONSTERS));
            self.api.set("SUPOX_HOSTILE", 1_u8);
            self.api.set("SUPOX_HOME_VISITS", 0_u8);
            self.api.set("SUPOX_VISITS", 0_u8);
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
    fn allied_home(&mut self, r: u16) {
        let mut news: u8;
        news = self.api.get("SUPOX_WAR_NEWS");
        let num_visits: u8 = self.api.get("UTWIG_SUPOX_MISSION");
        if r == HOW_WENT_WAR {
            self.api.npc(i32::from(ABOUT_BATTLE));
            news |= 1 << 0;
        } else if r == HOW_GOES_WAR {
            if num_visits == 1 {
                self.api.npc(i32::from(FLEET_ON_WAY));
                self.api.set("SUPOX_WAR_NEWS", 1_u8);
            } else {
                match self.api.get("SUPOX_WAR_NEWS") {
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
        self.api.set("SUPOX_WAR_NEWS", news);
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
    fn before_kohr_ah(&mut self, r: u16) {
        let mut num_visits: u8;
        if r == WHATS_UP_BEFORE_SPACE {
            num_visits = self.api.get("SUPOX_INFO");
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
            self.api.set("SUPOX_INFO", num_visits);
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
            num_visits = self.api.get("SUPOX_INFO");
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
            self.api.set("SUPOX_INFO", num_visits);
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
    fn neutral_supox(&mut self, r: u16) {
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
            self.api.set("SUPOX_STACK1", 1_u8);
            self.api.disable(I_AM0);
        } else if r == MY_SHIP0 {
            self.api.npc(i32::from(OUR_SHIP));
            self.api.set("SUPOX_STACK1", 2_u8);
            self.api.disable(MY_SHIP0);
        } else if r == FROM_ALLIANCE0 {
            self.api.npc(i32::from(FROM_SUPOX));
            self.api.set("SUPOX_STACK1", 3_u8);
            self.api.disable(FROM_ALLIANCE0);
        } else if r == ARE_YOU_COPYING {
            self.api.npc(i32::from(YEAH_SORRY));
            self.api.set("SUPOX_STACK1", 4_u8);
        } else if r == WHY_COPY {
            self.api.npc(i32::from(SYMBIOTS));
            self.api.set("SUPOX_STACK1", 5_u8);
        } else if r == TELL_US_OF_YOUR_SPECIES {
            self.api.npc(i32::from(OUR_SPECIES));
            last_stack = 1;
            self.api.set("SUPOX_STACK2", 1_u8);
        } else if r == PLANTS_ARENT_INTELLIGENT {
            self.api.npc(i32::from(PROVES_WERE_SPECIAL));
            self.api.set("SUPOX_STACK2", 2_u8);
        } else if r == ANYONE_AROUND_HERE {
            self.api.npc(i32::from(UTWIG_NEARBY));
            last_stack = 2;
            self.api.set("SUPOX_WAR_NEWS", 1_u8);
            self.api.track(UTWIG_SHIP);
        } else if r == WHAT_RELATION_TO_UTWIG {
            self.api.npc(i32::from(UTWIG_ALLIES));
            last_stack = 2;
            self.api.set("SUPOX_WAR_NEWS", 1_u8);
        } else if r == WHATS_WRONG_WITH_UTWIG {
            self.api.npc(i32::from(BROKE_ULTRON));
            last_stack = 2;
            self.api.set("SUPOX_WAR_NEWS", 2_u8);
        } else if r == WHATS_ULTRON {
            self.api.npc(i32::from(TAKE_ULTRON));
            self.api.set("SUPOX_WAR_NEWS", 0_u8);
            self.api.set("ULTRON_CONDITION", 1_u8);
            self.api
                .response(WHAT_DO_I_DO_NOW, Handler::ExitConversation, &[]);
            self.api
                .response(THANKS_NOW_WE_EAT_YOU, Handler::ExitConversation, &[]);
            return;
        } else if r == GOT_FIXED_ULTRON {
            self.api.npc(i32::from(GOOD_GIVE_TO_UTWIG));
            self.api.set("SUPOX_ULTRON_HELP", 1_u8);
        } else if r == LOOK_I_REPAIRED_LOTS {
            self.api.npc(i32::from(ALMOST_THERE));
            self.api.set("SUPOX_ULTRON_HELP", 1_u8);
        } else if r == LOOK_I_SLIGHTLY_REPAIRED {
            self.api.npc(i32::from(GREAT_DO_MORE));
            self.api.set("SUPOX_ULTRON_HELP", 1_u8);
        } else if r == WHERE_GET_REPAIRS {
            self.api.npc(i32::from(ANCIENT_RHYME));
            self.api.set("SUPOX_ULTRON_HELP", 1_u8);
        }
        match self.api.get("SUPOX_STACK2") {
            0 => {
                p_str[1_usize] = TELL_US_OF_YOUR_SPECIES;
            }
            1 => {
                p_str[1_usize] = PLANTS_ARENT_INTELLIGENT;
            }
            _ => {}
        }
        match self.api.get("SUPOX_STACK1") {
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
        let num_visits: u8 = self.api.get("ULTRON_CONDITION");
        if num_visits == 0 {
            match self.api.get("SUPOX_WAR_NEWS") {
                0 => {
                    if (self.api.get("UTWIG_VISITS") != 0 || self.api.get("UTWIG_HOME_VISITS") != 0)
                        || self.api.get("BOMB_VISITS") != 0
                    {
                        p_str[2_usize] = WHAT_RELATION_TO_UTWIG;
                    } else {
                        p_str[2_usize] = ANYONE_AROUND_HERE;
                    }
                }
                1 => {
                    p_str[2_usize] = WHATS_WRONG_WITH_UTWIG;
                }
                2 => {
                    p_str[2_usize] = WHATS_ULTRON;
                }
                _ => {}
            }
        }
        if p_str[last_stack as usize] != 0 {
            if (last_stack != 0) || (self.api.get("SUPOX_STACK1") > 2) {
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
                if (i != 0) || (self.api.get("SUPOX_STACK1") > 2) {
                    self.api
                        .response(p_str[i as usize], Handler::NeutralSupox, &[]);
                } else {
                    self.api
                        .response(p_str[i as usize], Handler::NeutralSupox, &fragments);
                }
            }
        }
        if !(self.api.get("SUPOX_ULTRON_HELP") != 0) {
            match num_visits {
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
    fn intro(&mut self) {
        let mut num_visits: u8;
        if (self.api.activity() as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0);
            return;
        }
        if self.api.get("SUPOX_HOSTILE") != 0 {
            num_visits = self.api.get("SUPOX_VISITS");
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
            self.api.set("SUPOX_VISITS", num_visits);
            self.api.segue(0);
        } else if self.api.alliance(SUPOX_SHIP) == 1 {
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                num_visits = self.api.get("SUPOX_HOME_VISITS");
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
                self.api.set("SUPOX_HOME_VISITS", num_visits);
                self.allied_home(0_u16);
            } else {
                num_visits = self.api.get("UTWIG_SUPOX_MISSION");
                if num_visits == 1 {
                    num_visits = self.api.get("SUPOX_VISITS");
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
                    self.api.set("SUPOX_VISITS", num_visits);
                    self.before_kohr_ah(0_u16);
                } else if num_visits < 5 {
                    num_visits = self.api.get("SUPOX_VISITS");
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
                    self.api.set("SUPOX_VISITS", num_visits);
                } else {
                    num_visits = self.api.get("SUPOX_VISITS");
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
                    self.api.set("SUPOX_VISITS", num_visits);
                    self.after_kohr_ah(0_u16);
                }
            }
        } else {
            if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
                num_visits = self.api.get("SUPOX_HOME_VISITS");
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
                self.api.set("SUPOX_HOME_VISITS", num_visits);
            } else {
                num_visits = self.api.get("SUPOX_VISITS");
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
                self.api.set("SUPOX_VISITS", num_visits);
            }
            self.neutral_supox(0_u16);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.supox.graphics".as_ptr(),
        alien_font_res: c"comm.supox.font".as_ptr(),
        alien_colormap_res: c"comm.supox.colortable".as_ptr(),
        alien_song_res: c"comm.supox.music".as_ptr(),
        conversation_phrases_res: c"comm.supox.dialogue".as_ptr(),
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
        num_animations: 4,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(4, 5, 12, (84, 84), (840, 2520));
    d.alien_ambient_array[1] = animation(9, 10, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[2] = animation(19, 10, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[3] = animation(29, 13, 2, (56, 0), (840, 2520));
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
    fn neutral_supox(r: u32) {
        run(|s| {
            s.handle(
                Handler::NeutralSupox,
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
            Handler::NeutralSupox => neutral_supox,
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
    pub unsafe extern "C" fn init_supox_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct SupoxDialogue;
impl super::RaceDialogue for SupoxDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_supox_comm()) }
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
