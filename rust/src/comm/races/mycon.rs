//! Mycon dialogue, ported from myconc.c.
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
mod api;
use api::MyconApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    CombatIsInevitable,
    SunDevice,
    TrickMycon,
    NormalMycon,
}
#[derive(Default)]
struct Session {
    made_choice: i32,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
    session: &'a mut Session,
}
impl<A: MyconApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::CombatIsInevitable => self.combat_is_inevitable(r),
            Handler::SunDevice => self.sun_device(r),
            Handler::TrickMycon => self.trick_mycon(r),
            Handler::NormalMycon => self.normal_mycon(r),
        }
    }
    fn do_ramble(&mut self, r: i32) {
        let mut counter: i32;
        counter = i32::from(self.api.get("MYCON_RAMBLE"));
        let res = {
            let old = counter;
            counter = (counter + 1) as u8 as i32;
            old
        };
        match res {
            0 => {
                self.api.npc(RAMBLE_1);
            }
            1 => {
                self.api.npc(RAMBLE_2);
            }
            2 => {
                self.api.npc(RAMBLE_3);
            }
            3 => {
                self.api.npc(RAMBLE_4);
            }
            4 => {
                self.api.npc(RAMBLE_5);
            }
            5 => {
                self.api.npc(RAMBLE_6);
            }
            6 => {
                self.api.npc(RAMBLE_7);
            }
            7 => {
                self.api.npc(RAMBLE_8);
            }
            8 => {
                self.api.npc(RAMBLE_9);
            }
            9 => {
                self.api.npc(RAMBLE_10);
            }
            10 => {
                self.api.npc(RAMBLE_11);
            }
            11 => {
                self.api.npc(RAMBLE_12);
            }
            12 => {
                self.api.npc(RAMBLE_13);
            }
            13 => {
                self.api.npc(RAMBLE_14);
            }
            14 => {
                self.api.npc(RAMBLE_15);
            }
            15 => {
                self.api.npc(RAMBLE_16);
            }
            16 => {
                self.api.npc(RAMBLE_17);
            }
            17 => {
                self.api.npc(RAMBLE_18);
            }
            18 => {
                self.api.npc(RAMBLE_19);
            }
            19 => {
                self.api.npc(RAMBLE_20);
            }
            20 => {
                self.api.npc(RAMBLE_21);
            }
            21 => {
                self.api.npc(RAMBLE_22);
            }
            22 => {
                self.api.npc(RAMBLE_23);
            }
            23 => {
                self.api.npc(RAMBLE_24);
            }
            24 => {
                self.api.npc(RAMBLE_25);
                if i32::from(self.api.get("KNOW_ABOUT_SHATTERED")) < 2 {
                    self.api.set("KNOW_ABOUT_SHATTERED", 2_u8);
                }
            }
            25 => {
                self.api.npc(RAMBLE_26);
            }
            26 => {
                self.api.npc(RAMBLE_27);
            }
            27 => {
                self.api.npc(RAMBLE_28);
            }
            28 => {
                self.api.npc(RAMBLE_29);
            }
            29 => {
                self.api.npc(RAMBLE_30);
            }
            30 => {
                self.api.npc(RAMBLE_31);
            }
            31 => {
                self.api.npc(RAMBLE_32);
                counter = 0;
            }
            _ => {}
        }
        self.api.set("MYCON_RAMBLE", (counter) as u8);
        if !((i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0) {
            if !(r == COME_IN_PEACE) && !(r == GONNA_DIE) {
                counter = (i32::from(self.api.get("MYCON_INSULTS")) + 1) & 7;
                self.api.set("MYCON_INSULTS", (counter) as u8);
                self.session.made_choice = 1;
            }
        } else if (!(r == LETS_BE_FRIENDS) && !(r == CAME_TO_HOMEWORLD)) && !(r == SUBMIT_TO_US) {
            counter = (i32::from(self.api.get("MYCON_INFO")) + 1) & 15;
            self.api.set("MYCON_INFO", (counter) as u8);
            self.session.made_choice = 1;
        }
    }
    fn combat_is_inevitable(&mut self, r: i32) {
        self.api.segue(1);
        if r == BYE_SPACE {
            self.api.npc(BYE_AND_DIE_SPACE);
        } else if r == BYE_HOMEWORLD {
            self.api.npc(BYE_AND_DIE_HOMEWORLD);
        } else if r == LIKE_TO_LAND {
            self.api.npc(NEVER_LET_LAND);
        } else if r == BYE_SUN_DEVICE {
            self.api.npc(GOODBYE_SUN_DEVICE);
            self.api.segue(0);
        } else {
            self.do_ramble(r);
            if !((i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0) {
                self.api.npc(BYE_AND_DIE_SPACE);
            } else {
                self.api.npc(BYE_AND_DIE_HOMEWORLD);
            }
        }
        self.session.made_choice = 0;
    }
    fn sun_device(&mut self, r: i32) {
        if r == WHATS_UP_SUN_DEVICE {
            self.api.npc(GENERAL_INFO_SUN_DEVICE);
            self.api.disable((WHATS_UP_SUN_DEVICE) as u16);
        } else if r == HOW_GOES_IMPLANTING {
            self.api.npc(UNFORSEEN_DELAYS);
            self.api.disable((HOW_GOES_IMPLANTING) as u16);
        } else if r == I_HAVE_A_CUNNING_PLAN {
            self.api.npc(WONT_FALL_FOR_TRICK);
            self.api.set("NO_TRICK_AT_SUN", 1_u8);
        }
        if self.api.enabled((WHATS_UP_SUN_DEVICE) as u16) {
            self.api
                .response((WHATS_UP_SUN_DEVICE) as u16, Handler::SunDevice, &[]);
        }
        if i32::from(self.api.get("MYCON_FELL_FOR_AMBUSH")) != 0 {
            if self.api.enabled((HOW_GOES_IMPLANTING) as u16)
                && (i32::from(self.api.get("MYCON_FELL_FOR_AMBUSH")) != 0)
            {
                self.api
                    .response((HOW_GOES_IMPLANTING) as u16, Handler::SunDevice, &[]);
            }
            self.api
                .response((LIKE_TO_LAND) as u16, Handler::CombatIsInevitable, &[]);
        } else if (i32::from(self.api.get("MYCON_AMBUSH")) != 0)
            && !(i32::from(self.api.get("NO_TRICK_AT_SUN")) != 0)
        {
            self.api
                .response((I_HAVE_A_CUNNING_PLAN) as u16, Handler::SunDevice, &[]);
        }
        self.api
            .response((BYE_SUN_DEVICE) as u16, Handler::CombatIsInevitable, &[]);
    }
    fn trick_mycon(&mut self, r: i32) {
        if r == I_HAVE_A_CUNNING_PLAN {
            self.api.npc(TELL_US_ABOUT_WORLD);
            self.api.disable((I_HAVE_A_CUNNING_PLAN) as u16);
        } else if r == CLUE_1 {
            self.api.npc(RESPONSE_1);
            self.api.disable((CLUE_1) as u16);
        } else if r == CLUE_2 {
            self.api.npc(RESPONSE_2);
            self.api.disable((CLUE_2) as u16);
        } else if r == CLUE_3 {
            self.api.npc(RESPONSE_3);
            self.api.disable((CLUE_3) as u16);
        }
        if (!self.api.enabled((CLUE_1) as u16) && !self.api.enabled((CLUE_2) as u16))
            && !self.api.enabled((CLUE_3) as u16)
        {
            self.api.npc(WE_GO_TO_IMPLANT);
            self.api.segue(0);
            self.api.set("MYCON_FELL_FOR_AMBUSH", 1_u8);
            self.api.event(1, 0, 0, 0, 13);
        } else {
            self.api.npc(AMBUSH_TAIL);
        }
        if self.api.enabled((CLUE_1) as u16) {
            self.api.response((CLUE_1) as u16, Handler::TrickMycon, &[]);
        }
        if self.api.enabled((CLUE_2) as u16) {
            self.api.response((CLUE_2) as u16, Handler::TrickMycon, &[]);
        }
        if self.api.enabled((CLUE_3) as u16) {
            self.api.response((CLUE_3) as u16, Handler::TrickMycon, &[]);
        }
    }
    fn normal_mycon(&mut self, r: i32) {
        if r == WHAT_ABOUT_SHATTERED {
            self.api.npc(ABOUT_SHATTERED);
            self.api.set("KNOW_ABOUT_SHATTERED", 2_u8);
        } else if r != 0 {
            self.do_ramble(r);
            self.api.npc(RAMBLE_TAIL);
            self.api.disable((r) as u16);
        }
        let resp_func: Handler = if (self.api.random() as u8 as i32) < ((256 * 30) / 100) {
            Handler::CombatIsInevitable
        } else {
            Handler::NormalMycon
        };
        if !((i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0) {
            if self.api.enabled((COME_IN_PEACE) as u16) {
                self.api.response((COME_IN_PEACE) as u16, resp_func, &[]);
            }
            if self.api.enabled((GONNA_DIE) as u16) {
                self.api.response((GONNA_DIE) as u16, resp_func, &[]);
            }
            if !(self.session.made_choice != 0) {
                match i32::from(self.api.get("MYCON_INSULTS")) {
                    0 => {
                        self.api.response((INSULT_1) as u16, resp_func, &[]);
                    }
                    1 => {
                        self.api.response((INSULT_2) as u16, resp_func, &[]);
                    }
                    2 => {
                        self.api.response((INSULT_3) as u16, resp_func, &[]);
                    }
                    3 => {
                        self.api.response((INSULT_4) as u16, resp_func, &[]);
                    }
                    4 => {
                        self.api.response((INSULT_5) as u16, resp_func, &[]);
                    }
                    5 => {
                        self.api.response((INSULT_6) as u16, resp_func, &[]);
                    }
                    6 => {
                        self.api.response((INSULT_7) as u16, resp_func, &[]);
                    }
                    7 => {
                        self.api.response((INSULT_8) as u16, resp_func, &[]);
                    }
                    _ => {}
                }
            }
            self.api
                .response((BYE_SPACE) as u16, Handler::CombatIsInevitable, &[]);
        } else {
            if !(self.session.made_choice != 0) {
                match i32::from(self.api.get("MYCON_INFO")) {
                    0 => {
                        self.api.response((QUESTION_1) as u16, resp_func, &[]);
                    }
                    1 => {
                        self.api.response((QUESTION_2) as u16, resp_func, &[]);
                    }
                    2 => {
                        self.api.response((QUESTION_3) as u16, resp_func, &[]);
                    }
                    3 => {
                        self.api.response((QUESTION_4) as u16, resp_func, &[]);
                    }
                    4 => {
                        self.api.response((QUESTION_5) as u16, resp_func, &[]);
                    }
                    5 => {
                        self.api.response((QUESTION_6) as u16, resp_func, &[]);
                    }
                    6 => {
                        self.api.response((QUESTION_7) as u16, resp_func, &[]);
                    }
                    7 => {
                        self.api.response((QUESTION_8) as u16, resp_func, &[]);
                    }
                    8 => {
                        self.api.response((QUESTION_9) as u16, resp_func, &[]);
                    }
                    9 => {
                        self.api.response((QUESTION_10) as u16, resp_func, &[]);
                    }
                    10 => {
                        self.api.response((QUESTION_11) as u16, resp_func, &[]);
                    }
                    11 => {
                        self.api.response((QUESTION_12) as u16, resp_func, &[]);
                    }
                    12 => {
                        self.api.response((QUESTION_13) as u16, resp_func, &[]);
                    }
                    13 => {
                        self.api.response((QUESTION_14) as u16, resp_func, &[]);
                    }
                    14 => {
                        self.api.response((QUESTION_15) as u16, resp_func, &[]);
                    }
                    15 => {
                        self.api.response((QUESTION_16) as u16, resp_func, &[]);
                    }
                    _ => {}
                }
            }
            if self.api.enabled((LETS_BE_FRIENDS) as u16) {
                self.api.response((LETS_BE_FRIENDS) as u16, resp_func, &[]);
            }
            if self.api.enabled((CAME_TO_HOMEWORLD) as u16) {
                self.api
                    .response((CAME_TO_HOMEWORLD) as u16, resp_func, &[]);
            }
            if self.api.enabled((SUBMIT_TO_US) as u16) {
                self.api.response((SUBMIT_TO_US) as u16, resp_func, &[]);
            }
            if !(i32::from(self.api.get("MYCON_FELL_FOR_AMBUSH")) != 0) {
                if i32::from(self.api.get("KNOW_ABOUT_SHATTERED")) == 1 {
                    self.api
                        .response((WHAT_ABOUT_SHATTERED) as u16, Handler::NormalMycon, &[]);
                }
                if i32::from(self.api.get("MYCON_AMBUSH")) != 0 {
                    self.api
                        .response((I_HAVE_A_CUNNING_PLAN) as u16, Handler::TrickMycon, &[]);
                }
            }
            self.api
                .response((BYE_HOMEWORLD) as u16, Handler::CombatIsInevitable, &[]);
        }
    }
    fn intro(&mut self) {
        let mut num_visits: i32;
        if i32::from(self.api.get("SUN_DEVICE")) != 0 {
            num_visits = i32::from(self.api.get("MYCON_VISITS"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(DIE_THIEF);
                }
                1 => {
                    self.api.npc(DIE_THIEF_AGAIN);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("MYCON_VISITS", (num_visits) as u8);
            self.api.segue(1);
        } else if i32::from(self.api.get("MYCON_KNOW_AMBUSH")) != 0 {
            self.api.npc(DIE_LIAR);
            self.api.segue(1);
        } else if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 6)) != 0 {
            num_visits = i32::from(self.api.get("MYCON_SUN_VISITS"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_SUN_DEVICE_WORLD_1);
                }
                1 => {
                    self.api.npc(HELLO_SUN_DEVICE_WORLD_2);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("MYCON_SUN_VISITS", (num_visits) as u8);
            self.sun_device(0_u16 as i32);
        } else if (i32::from(self.api.get("GLOBAL_FLAGS_AND_DATA")) & (1 << 7)) != 0 {
            num_visits = i32::from(self.api.get("MYCON_HOME_VISITS"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_HOMEWORLD_1);
                }
                1 => {
                    self.api.npc(HELLO_HOMEWORLD_2);
                }
                2 => {
                    self.api.npc(HELLO_HOMEWORLD_3);
                }
                3 => {
                    self.api.npc(HELLO_HOMEWORLD_4);
                }
                4 => {
                    self.api.npc(HELLO_HOMEWORLD_5);
                }
                5 => {
                    self.api.npc(HELLO_HOMEWORLD_6);
                }
                6 => {
                    self.api.npc(HELLO_HOMEWORLD_7);
                }
                7 => {
                    self.api.npc(HELLO_HOMEWORLD_8);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("MYCON_HOME_VISITS", (num_visits) as u8);
            self.normal_mycon(0_u16 as i32);
        } else {
            num_visits = i32::from(self.api.get("MYCON_VISITS"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_SPACE_1);
                }
                1 => {
                    self.api.npc(HELLO_SPACE_2);
                }
                2 => {
                    self.api.npc(HELLO_SPACE_3);
                }
                3 => {
                    self.api.npc(HELLO_SPACE_4);
                }
                4 => {
                    self.api.npc(HELLO_SPACE_5);
                }
                5 => {
                    self.api.npc(HELLO_SPACE_6);
                }
                6 => {
                    self.api.npc(HELLO_SPACE_7);
                }
                7 => {
                    self.api.npc(HELLO_SPACE_8);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("MYCON_VISITS", (num_visits) as u8);
            self.normal_mycon(0_u16 as i32);
        }
    }
    fn post_mycon_enc(&mut self) {}
    fn initialize(&mut self) {
        self.session.made_choice = 0;
        if (i32::from(self.api.activity()) as u8 as i32) != 5 {
            self.api.segue(1);
        } else {
            self.api.segue(0);
        }
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.mycon.graphics".as_ptr(),
        alien_font_res: c"comm.mycon.font".as_ptr(),
        alien_colormap_res: c"comm.mycon.colortable".as_ptr(),
        alien_song_res: c"comm.mycon.music".as_ptr(),
        conversation_phrases_res: c"comm.mycon.dialogue".as_ptr(),
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
        num_animations: 5,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(12, 6, 2, (63, 0), (63, 0));
    d.alien_ambient_array[0].block_mask = 2;
    d.alien_ambient_array[1] = animation(18, 4, 2, (63, 0), (63, 0));
    d.alien_ambient_array[1].block_mask = 1;
    d.alien_ambient_array[2] = animation(22, 6, 2, (63, 0), (63, 0));
    d.alien_ambient_array[3] = animation(28, 5, 2, (63, 0), (63, 0));
    d.alien_ambient_array[4] = animation(33, 4, 4, (63, 0), (63, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 11, 0, (56, 0), (70, 0));
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
        static mut SESSION: Session = Session { made_choice: 0 };
        unsafe {
            f(&mut Dialogue {
                api: &mut api,
                session: &mut *std::ptr::addr_of_mut!(SESSION),
            });
        }
    }
    fn combat_is_inevitable(r: u32) {
        run(|s| {
            s.handle(
                Handler::CombatIsInevitable,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn sun_device(r: u32) {
        run(|s| {
            s.handle(
                Handler::SunDevice,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn trick_mycon(r: u32) {
        run(|s| {
            s.handle(
                Handler::TrickMycon,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn normal_mycon(r: u32) {
        run(|s| {
            s.handle(
                Handler::NormalMycon,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::CombatIsInevitable => combat_is_inevitable,
            Handler::SunDevice => sun_device,
            Handler::TrickMycon => trick_mycon,
            Handler::NormalMycon => normal_mycon,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_mycon_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_mycon_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize());
        desc
    }
}
pub struct MyconDialogue;
impl super::RaceDialogue for MyconDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_mycon_comm()) }
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
