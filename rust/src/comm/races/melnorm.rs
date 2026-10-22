//! Melnorme dialogue and trading, ported from melnorm.c.

use super::encounter::EncounterApi;
use super::script::{animation, as_comm_data, GLOBAL_PLAYER_NAME};
use crate::comm::locdata::{CColor, CLocData, CPoint, CSisState};
use crate::comm::types::CommData;
mod api;
mod numbers;
mod phrases;
mod rescue;
use api::MelnormApi;
use phrases::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    DoRescue,
    DoBuy,
    DoSell,
    NatureOfConversation,
    DoBluster,
    DoFirstMeeting,
    DoMelnormeMiffed,
    DoMelnormePissed,
}

const fn empty_ship() -> CSisState {
    CSisState {
        log_x: 0,
        log_y: 0,
        res_units: 0,
        fuel_on_board: 0,
        crew_enlisted: 0,
        total_element_mass: 0,
        total_bio_mass: 0,
        module_slots: [22; 16],
        drive_slots: [20; 11],
        jet_slots: [21; 8],
        num_landers: 0,
        element_amounts: [0; 8],
        ship_name: [0; 16],
        commander_name: [0; 16],
        planet_name: [0; 16],
    }
}
struct Session {
    local_stack0: u8,
    local_stack1: u8,
    asked_to_buy: bool,
    prev_msg_mode: i32,
    rescue_fuel: u16,
    sis_copy: CSisState,
}
impl Session {
    const fn new() -> Self {
        Self {
            local_stack0: 0,
            local_stack1: 0,
            asked_to_buy: false,
            prev_msg_mode: 0,
            rescue_fuel: 0,
            sis_copy: empty_ship(),
        }
    }
}
struct Dialogue<'a, A> {
    api: &'a mut A,
    session: &'a mut Session,
}

#[derive(Clone, Copy)]
enum Tech {
    Module(usize, u8),
    Shield(u8),
    State(&'static str),
}
const TECHS: [Tech; 13] = [
    Tech::Module(10, 80),
    Tech::State("IMPROVED_LANDER_SPEED"),
    Tech::Module(13, 80),
    Tech::Shield(0),
    Tech::State("IMPROVED_LANDER_CARGO"),
    Tech::Module(6, 20),
    Tech::State("IMPROVED_LANDER_SHOT"),
    Tech::Shield(1),
    Tech::Module(12, 100),
    Tech::Shield(2),
    Tech::Shield(3),
    Tech::Module(11, 120),
    Tech::Module(8, 80),
];
impl<A: MelnormApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::DoRescue => self.do_rescue(r),
            Handler::DoBuy => self.do_buy(r),
            Handler::DoSell => self.do_sell(r),
            Handler::NatureOfConversation => self.nature_of_conversation(r),
            Handler::DoBluster => self.do_bluster(r),
            Handler::DoFirstMeeting => self.do_first_meeting(r),
            Handler::DoMelnormeMiffed => self.do_melnorme_miffed(r),
            Handler::DoMelnormePissed => self.do_melnorme_pissed(r),
        }
    }
    fn initialize(&mut self, _desc: &mut CLocData) {
        self.session.local_stack0 = 0;
        self.session.local_stack1 = 0;
        self.session.prev_msg_mode = 0;
        self.api.segue(0);
        self.session.asked_to_buy = false;
    }
    fn has_tech(&self, i: usize) -> bool {
        match TECHS[i] {
            Tech::Module(id, _) => self.api.module_cost(id) != 0,
            Tech::Shield(bit) => self.api.get("LANDER_SHIELDS") & (1 << bit) != 0,
            Tech::State(key) => self.api.get(key) != 0,
        }
    }
    fn grant_tech(&mut self, i: usize) {
        match TECHS[i] {
            Tech::Module(id, cost) => self.api.set_module_cost(id, cost),
            Tech::Shield(bit) => self.api.set(
                "LANDER_SHIELDS",
                self.api.get("LANDER_SHIELDS") | (1 << bit),
            ),
            Tech::State(key) => self.api.set(key, 1),
        }
    }
    fn next_tech(&self) -> Option<usize> {
        (0..TECHS.len()).find(|&i| !self.has_tech(i))
    }
    fn credits(&self) -> u16 {
        u16::from_le_bytes([
            self.api.get("MELNORME_CREDIT0"),
            self.api.get("MELNORME_CREDIT1"),
        ])
    }
    fn set_credits(&mut self, credits: u16) {
        let [lo, hi] = credits.to_le_bytes();
        self.api.set("MELNORME_CREDIT0", lo);
        self.api.set("MELNORME_CREDIT1", hi);
    }
    fn delta_credit(&mut self, delta: i16) -> bool {
        let credit = self.credits();
        if delta >= 0 || -i32::from(delta) <= i32::from(credit) {
            self.set_credits(credit.wrapping_add(delta as u16));
            self.api.draw_status();
            true
        } else {
            self.api.npc(i32::from(NEED_MORE_CREDIT0));
            self.api.number(-i32::from(delta) - i32::from(credit));
            self.api.npc(i32::from(NEED_MORE_CREDIT1));
            false
        }
    }
    fn any_info(&self) -> bool {
        self.api.get("MELNORME_EVENTS_INFO_STACK") < 8
            || self.api.get("MELNORME_ALIEN_INFO_STACK") < 16
            || self.api.get("MELNORME_HISTORY_INFO_STACK") < 9
    }
    fn info(&mut self, r: u16) {
        let (key, first, count) = match r {
            BUY_CURRENT_EVENTS => ("MELNORME_EVENTS_INFO_STACK", OK_BUY_EVENT_1, 8),
            BUY_ALIEN_RACES => ("MELNORME_ALIEN_INFO_STACK", OK_BUY_ALIEN_RACE_1, 16),
            BUY_HISTORY => ("MELNORME_HISTORY_INFO_STACK", OK_BUY_HISTORY_1, 9),
            _ => unreachable!(),
        };
        let stack = self.api.get(key);
        let phrase = first + u16::from(stack.min(count - 1));
        if r == BUY_ALIEN_RACES {
            if phrase == OK_BUY_ALIEN_RACE_14 && self.api.get("FOUND_PLUTO_SPATHI") == 0 {
                self.api.set("KNOW_SPATHI_PASSWORD", 1);
                self.api.set("SPATHI_HOME_VISITS", 7);
            }
            if phrase == OK_BUY_ALIEN_RACE_15 {
                if self.api.get("KNOW_ABOUT_SHATTERED") < 2 {
                    self.api.set("KNOW_ABOUT_SHATTERED", 2);
                }
                self.api.set("KNOW_SYREEN_WORLD_SHATTERED", 1);
            }
        }
        self.api.npc(i32::from(phrase));
        self.api.set(key, stack.wrapping_add(1));
    }
    fn capacity(&self) -> u32 {
        1000 + self
            .api
            .ship()
            .module_slots
            .iter()
            .map(|&m| match m {
                5 => 5000,
                6 => 10000,
                _ => 0,
            })
            .sum::<u32>()
    }
    fn buy_menu(&mut self, capacity: u32) {
        if self.api.ship().fuel_on_board < capacity {
            self.api.response(BUY_FUEL, Handler::DoBuy, &[]);
        }
        if self.next_tech().is_some() {
            self.api.response(BUY_TECHNOLOGY, Handler::DoBuy, &[]);
        }
        if self.any_info() {
            self.api.response(BUY_INFO, Handler::DoBuy, &[]);
        }
        self.api
            .response(DONE_BUYING, Handler::NatureOfConversation, &[]);
        self.api
            .response(BE_LEAVING_NOW, Handler::ExitConversation, &[]);
    }
    fn do_buy(&mut self, r: u16) {
        let credit = self.credits();
        let capacity = self.capacity();
        if credit == 0 {
            self.session.asked_to_buy = true;
            self.api.npc(i32::from(NEED_CREDIT));
            self.nature_of_conversation(r);
        } else if [
            BUY_FUEL,
            BUY_1_FUEL,
            BUY_5_FUEL,
            BUY_10_FUEL,
            BUY_25_FUEL,
            FILL_ME_UP,
        ]
        .contains(&r)
        {
            let needed = match r {
                BUY_1_FUEL => 1,
                BUY_5_FUEL => 5,
                BUY_10_FUEL => 10,
                BUY_25_FUEL => 25,
                FILL_ME_UP => (capacity - self.api.ship().fuel_on_board).div_ceil(100) as i16,
                _ => 0,
            };
            let mut no_room = false;
            if needed == 0 {
                if self.api.get("MELNORME_FUEL_PROCEDURE") == 0 {
                    self.api.npc(i32::from(BUY_FUEL_INTRO));
                    self.api.set("MELNORME_FUEL_PROCEDURE", 1);
                }
            } else if self.api.ship().fuel_on_board / 100 + needed as u32 > capacity / 100 {
                self.api.npc(i32::from(NO_ROOM_FOR_FUEL));
                no_room = true;
            } else if i32::from(needed) <= i32::from(credit) {
                self.api.npc(i32::from(GOT_FUEL));
                let mut fuel = needed as u32 * 100;
                while fuel > 0x3fff {
                    self.api.gauges(0, 0x3fff, 0);
                    fuel -= 0x3fff;
                }
                self.api.gauges(0, fuel as i16, 0);
            }
            if !no_room && needed != 0 {
                self.delta_credit(-needed);
                if self.api.ship().fuel_on_board >= capacity {
                    self.buy_menu(capacity);
                    return;
                }
            }
            self.api.npc(i32::from(HOW_MUCH_FUEL));
            for p in [
                BUY_1_FUEL,
                BUY_5_FUEL,
                BUY_10_FUEL,
                BUY_25_FUEL,
                FILL_ME_UP,
                DONE_BUYING_FUEL,
            ] {
                self.api.response(p, Handler::DoBuy, &[]);
            }
        } else if r == BUY_TECHNOLOGY || r == BUY_NEW_TECH {
            if self.api.get("MELNORME_TECH_PROCEDURE") == 0 {
                self.api.npc(i32::from(BUY_NEW_TECH_INTRO));
                self.api.set("MELNORME_TECH_PROCEDURE", 1);
            }
            if r == BUY_NEW_TECH {
                let Some(i) = self.next_tech() else {
                    self.buy_menu(capacity);
                    return;
                };
                if !self.delta_credit(-150) {
                    self.buy_menu(capacity);
                    return;
                }
                self.grant_tech(i);
                self.api.npc(i32::from(OK_BUY_NEW_TECH_1) + i as i32);
            }
            let Some(i) = self.next_tech() else {
                self.api.npc(i32::from(NEW_TECH_ALL_GONE));
                self.buy_menu(capacity);
                return;
            };
            self.api.npc(i32::from(NEW_TECH_1) + i as i32);
            self.api.response(BUY_NEW_TECH, Handler::DoBuy, &[]);
            self.api.response(NO_BUY_NEW_TECH, Handler::DoBuy, &[]);
        } else if [BUY_INFO, BUY_CURRENT_EVENTS, BUY_ALIEN_RACES, BUY_HISTORY].contains(&r) {
            if self.api.get("MELNORME_INFO_PROCEDURE") == 0 {
                self.api.npc(i32::from(BUY_INFO_INTRO));
                self.api.set("MELNORME_INFO_PROCEDURE", 1);
            } else if r == BUY_INFO {
                self.api.npc(i32::from(OK_BUY_INFO));
            } else {
                if !self.delta_credit(-75) {
                    self.buy_menu(capacity);
                    return;
                }
                self.info(r);
            }
            if !self.any_info() {
                self.api.npc(i32::from(INFO_ALL_GONE));
                self.buy_menu(capacity);
                return;
            }
            for (key, n, p) in [
                ("MELNORME_EVENTS_INFO_STACK", 8, BUY_CURRENT_EVENTS),
                ("MELNORME_ALIEN_INFO_STACK", 16, BUY_ALIEN_RACES),
                ("MELNORME_HISTORY_INFO_STACK", 9, BUY_HISTORY),
            ] {
                if self.api.get(key) < n {
                    self.api.response(p, Handler::DoBuy, &[]);
                }
            }
            self.api.response(DONE_BUYING_INFO, Handler::DoBuy, &[]);
        } else {
            self.api.npc(i32::from(match r {
                DONE_BUYING_FUEL => OK_DONE_BUYING_FUEL,
                NO_BUY_NEW_TECH => OK_NO_BUY_NEW_TECH,
                DONE_BUYING_INFO => OK_DONE_BUYING_INFO,
                _ => WHAT_TO_BUY,
            }));
            self.buy_menu(capacity);
        }
    }
    fn new_rainbows(&self) -> u8 {
        let mask = u16::from_le_bytes([
            self.api.get("RAINBOW_WORLD0"),
            self.api.get("RAINBOW_WORLD1"),
        ]);
        (mask.count_ones() as u8).wrapping_sub(self.api.get("MELNORME_RAINBOW_COUNT"))
    }
    fn do_sell(&mut self, r: u16) {
        let mut rainbows = self.new_rainbows();
        let mut queued = false;
        if r != SELL {
            if r == SELL_LIFE_DATA {
                let bio = self.api.ship().total_bio_mass;
                let added = bio.wrapping_mul(2) as i16;
                self.api.npc(i32::from(SOLD_LIFE_DATA1));
                self.api.number(i32::from(bio));
                self.api.npc(i32::from(SOLD_LIFE_DATA2));
                self.api.number(i32::from(added));
                self.api.npc(i32::from(SOLD_LIFE_DATA3));
                if rainbows != 0 {
                    self.api.npc(i32::from(WHAT_TO_SELL));
                    queued = true;
                }
                self.api.talk_segue(1);
                self.api.draw_cargo(255, 255);
                self.api.sleep(420);
                self.api.now();
                self.api.draw_cargo(8, 8);
                loop {
                    self.api.now();
                    let mut ship = self.api.ship();
                    if self.api.skip_sale() {
                        self.delta_credit(ship.total_bio_mass.wrapping_mul(2) as i16);
                        ship.total_bio_mass = 0;
                    } else {
                        ship.total_bio_mass = ship.total_bio_mass.wrapping_sub(1);
                        self.api.set_ship(ship);
                        self.delta_credit(2);
                    }
                    self.api.set_ship(ship);
                    self.api.draw_cargo(8, 8);
                    if ship.total_bio_mass == 0 {
                        break;
                    }
                }
                self.api.sleep(420);
                self.api.clear_display();
            } else {
                let added = i16::from(rainbows) * 500;
                self.api.npc(i32::from(SOLD_RAINBOW_LOCATIONS1));
                self.api.number(i32::from(rainbows));
                self.api.npc(i32::from(SOLD_RAINBOW_LOCATIONS2));
                self.api.number(i32::from(added));
                self.api.npc(i32::from(SOLD_RAINBOW_LOCATIONS3));
                self.api.set(
                    "MELNORME_RAINBOW_COUNT",
                    rainbows.wrapping_add(self.api.get("MELNORME_RAINBOW_COUNT")),
                );
                rainbows = 0;
                self.delta_credit(added);
            }
            self.session.asked_to_buy = false;
        }
        if self.api.ship().total_bio_mass != 0 || rainbows != 0 {
            if !queued {
                self.api.npc(i32::from(WHAT_TO_SELL));
            }
            if self.api.ship().total_bio_mass != 0 {
                self.api.response(SELL_LIFE_DATA, Handler::DoSell, &[]);
            }
            if rainbows != 0 {
                self.api
                    .response(SELL_RAINBOW_LOCATIONS, Handler::DoSell, &[]);
            }
            self.api
                .response(DONE_SELLING, Handler::NatureOfConversation, &[]);
        } else {
            if r == SELL {
                self.api.npc(i32::from(NOTHING_TO_SELL));
            }
            self.api.disable(SELL);
            self.nature_of_conversation(r);
        }
    }
    fn nature_of_conversation(&mut self, mut r: u16) {
        if r == GET_ON_WITH_BUSINESS {
            self.api.set("MELNORME_YACK_STACK2", 5);
            r = 0;
        }
        self.delta_credit(0);
        let credit = self.credits();
        if r == 0 {
            let stack = self.api.get("MELNORME_YACK_STACK2").wrapping_sub(5);
            self.api.npc(i32::from(
                HELLO_AND_DOWN_TO_BUSINESS_1 + u16::from(stack.min(9)),
            ));
            self.api.set(
                "MELNORME_YACK_STACK2",
                if stack < 9 {
                    stack + 6
                } else {
                    stack.wrapping_add(5)
                },
            );
        }
        if self.api.ship().fuel_on_board > 0
            || self.api.ship().total_bio_mass != 0
            || credit != 0
            || self.new_rainbows() != 0
        {
            if self.api.get("TRADED_WITH_MELNORME") == 0 {
                self.api.set("TRADED_WITH_MELNORME", 1);
                self.api.npc(i32::from(TRADING_INFO));
            }
            if r == 0 {
                self.api.npc(i32::from(BUY_OR_SELL));
                self.api.talk_segue(1);
                self.api.fade_palette(1, 420);
                self.api.talk_segue(u16::MAX);
            } else if r == WHY_TURNED_PURPLE {
                self.api.set("WHY_MELNORME_PURPLE", 1);
                self.api.npc(i32::from(TURNED_PURPLE_BECAUSE));
            } else if r == DONE_SELLING {
                self.api.npc(i32::from(OK_DONE_SELLING));
            } else if r == DONE_BUYING {
                self.api.npc(i32::from(OK_DONE_BUYING));
            }
            if self.api.get("WHY_MELNORME_PURPLE") == 0 {
                self.api
                    .response(WHY_TURNED_PURPLE, Handler::NatureOfConversation, &[]);
            }
            if !self.session.asked_to_buy {
                self.api.response(BUY, Handler::DoBuy, &[]);
            }
            if self.api.enabled(SELL) {
                self.api.response(SELL, Handler::DoSell, &[]);
            }
            self.api.response(GOODBYE, Handler::ExitConversation, &[]);
        } else if self.api.get("MELNORME_RESCUE_REFUSED") != 0 {
            self.api.npc(i32::from(CHANGED_MIND));
            self.api.response(YES_CHANGED_MIND, Handler::DoRescue, &[]);
            self.api
                .response(NO_CHANGED_MIND, Handler::ExitConversation, &[]);
        } else {
            let n = self.api.get("MELNORME_RESCUE_COUNT");
            self.api
                .npc(i32::from(RESCUE_EXPLANATION + u16::from(n.min(5))));
            if n < 5 {
                self.api.set("MELNORME_RESCUE_COUNT", n + 1);
            }
            self.api.npc(i32::from(SHOULD_WE_HELP_YOU));
            self.api.response(YES_HELP, Handler::DoRescue, &[]);
            self.api.response(NO_HELP, Handler::ExitConversation, &[]);
        }
    }
    fn yack0_respond(&mut self) {
        match self.api.get("MELNORME_YACK_STACK0") {
            0 => {
                let i = self.api.get("NEW_ALLIANCE_NAME");
                let mut fragments = vec![
                    i32::from(WE_ARE_FROM_ALLIANCE0),
                    i32::from(NAME_1) + i32::from(i),
                ];
                if i == 3 {
                    fragments.extend_from_slice(&[GLOBAL_PLAYER_NAME, i32::from(NAME_41)]);
                }
                self.api
                    .response(WE_ARE_FROM_ALLIANCE0, Handler::DoFirstMeeting, &fragments);
            }
            1 => self.api.response(HOW_KNOW, Handler::DoFirstMeeting, &[]),
            _ => {}
        }
    }
    fn do_first_meeting(&mut self, r: u16) {
        let mut last_stack = 0;
        if r == 0 {
            let mut n = self.api.get("MELNORME_BUSINESS_COUNT");
            match n {
                0..=1 => {
                    self.api.npc(1 + i32::from(n));
                    n += 1;
                }
                2 => self.api.npc(3),
                _ => {
                    n = n.wrapping_add(1);
                }
            }
            self.api.set("MELNORME_BUSINESS_COUNT", n);
        } else if r == WE_ARE_FROM_ALLIANCE0 {
            self.api.set("MELNORME_YACK_STACK0", 1);
            self.api.npc(i32::from(KNOW_OF_YOU));
        } else if r == HOW_KNOW {
            self.api.set("MELNORME_YACK_STACK0", 2);
            self.api.npc(i32::from(KNOW_BECAUSE));
        } else if r == WHAT_ABOUT_YOURSELVES {
            last_stack = 1;
            self.api.set("MELNORME_YACK_STACK1", 1);
            self.api.npc(i32::from(NO_TALK_ABOUT_OURSELVES));
        } else if r == WHAT_FACTORS {
            last_stack = 1;
            self.api.set("MELNORME_YACK_STACK1", 2);
            self.api.npc(i32::from(FACTORS_ARE));
        } else if r == WHAT_ABOUT_UNIVERSE {
            last_stack = 2;
            self.api.set("MELNORME_YACK_STACK2", 1);
            self.api.npc(i32::from(NO_FREE_LUNCH));
        } else if r == GIVING_IS_GOOD_1 {
            last_stack = 2;
            self.api.set("MELNORME_YACK_STACK2", 2);
            self.api.npc(i32::from(GIVING_IS_BAD_1));
        } else if r == GIVING_IS_GOOD_2 {
            last_stack = 2;
            self.api.set("MELNORME_YACK_STACK2", 3);
            self.api.npc(i32::from(GIVING_IS_BAD_2));
        } else if r == YES_REALLY_TESTING {
            self.api.fade_palette(0, 420);
            self.api.npc(i32::from(TEST_RESULTS));
        } else if r == WE_APOLOGIZE {
            self.api.set("MELNORME_ANGER", 0);
            self.api.fade_palette(0, 420);
            self.api.npc(i32::from(APOLOGY_ACCEPTED));
        }
        let mut stacks = [0, 1, 2];
        stacks.swap(0, last_stack);
        for stack in stacks {
            match stack {
                0 => self.yack0_respond(),
                1 => self.yack1_respond(),
                _ => self.yack2_respond(),
            }
        }
        self.api
            .response(NO_TRADE_NOW, Handler::ExitConversation, &[]);
    }
    fn post_melnorme_enc(&mut self) {
        if self.session.prev_msg_mode != 0 {
            self.api.status_mode(self.session.prev_msg_mode);
        }
        self.api.draw_status();
    }
}

fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.melnorme.graphics".as_ptr(),
        alien_font_res: c"comm.melnorme.font".as_ptr(),
        alien_colormap_res: c"comm.melnorme.colortable".as_ptr(),
        alien_song_res: c"comm.melnorme.music".as_ptr(),
        conversation_phrases_res: c"comm.melnorme.dialogue".as_ptr(),
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
        alien_number_speech: numbers::descriptor(),
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(6, 5, 4, (70, 0), (3360, 3360));
    d.alien_ambient_array[0].block_mask = 2;
    d.alien_ambient_array[1] = animation(11, 9, 4, (42, 0), (3360, 3360));
    d.alien_ambient_array[1].block_mask = 1;
    d.alien_ambient_array[2] = animation(20, 2, 4, (84, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(22, 2, 4, (84, 56), (840, 2520));
    d.alien_talk_desc = animation(1, 5, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = live::intro as *const () as *const _;
        d.post_encounter_func = live::post as *const () as *const _;
        d.uninit_encounter_func = live::uninit as *const () as *const _;
    }
    d
}
impl<A: MelnormApi> Dialogue<'_, A> {
    fn exit_conversation(&mut self, r: u16) {
        if r == NO_TRADE_NOW {
            self.api.npc(i32::from(OK_NO_TRADE_NOW_BYE));
        } else if r == YOURE_ON {
            self.api.npc(i32::from(YOU_GIVE_US_NO_CHOICE));
            self.api.set("MELNORME_ANGER", 1_u8);
            self.api.segue(1);
        } else if r == SO_WE_CAN_ATTACK {
            self.api.npc(i32::from(DECEITFUL_HUMAN));
            self.api.set("MELNORME_ANGER", 2_u8);
            self.api.segue(1);
        } else if r == BYE_MELNORME_SLIGHTLY_ANGRY {
            self.api.npc(i32::from(MELNORME_SLIGHTLY_ANGRY_GOODBYE));
        } else if r == OK_STRIP_ME {
            if self.ship_worth() < 80 {
                self.api.npc(i32::from(NOT_WORTH_STRIPPING));
            } else {
                self.api.set("MELNORME_ANGER", 0_u8);
                self.strip_ship(65535);
                self.api.npc(i32::from(FAIR_JUSTICE));
            }
        } else if r == FIGHT_SOME_MORE {
            self.api.npc(i32::from(OK_FIGHT_SOME_MORE));
            self.api.set("MELNORME_ANGER", 3_u8);
            self.api.segue(1);
        } else if r == BYE_MELNORME_PISSED_OFF {
            self.api.npc(i32::from(MELNORME_PISSED_OFF_GOODBYE));
        } else if r == WELL_IF_THATS_THE_WAY_YOU_FEEL {
            self.api.npc(i32::from(WE_FIGHT_AGAIN));
            self.api.segue(1);
        } else if r == YOU_HATE_US_SO_WE_GO_AWAY {
            self.api.npc(i32::from(HATE_YOU_GOODBYE));
        } else if r == TAKE_IT {
            self.strip_ship(0);
            self.api.npc(i32::from(HAPPY_TO_HAVE_RESCUED));
        } else if r == LEAVE_IT {
            self.api.set("MELNORME_RESCUE_REFUSED", 1_u8);
            self.api.npc(i32::from(MAYBE_SEE_YOU_LATER));
        } else if r == NO_HELP {
            self.api.set("MELNORME_RESCUE_REFUSED", 1_u8);
            self.api.npc(i32::from(GOODBYE_AND_GOODLUCK));
        } else if r == NO_CHANGED_MIND {
            self.api.npc(i32::from(GOODBYE_AND_GOODLUCK_AGAIN));
        } else if r == BE_LEAVING_NOW || r == GOODBYE {
            self.api.npc(i32::from(FRIENDLY_GOODBYE));
        }
    }
    fn do_bluster(&mut self, r: u16) {
        if r == TRADE_IS_FOR_THE_WEAK {
            self.api.fade_palette(2, 420);
            self.api.set("MELNORME_YACK_STACK2", 4_u8);
            self.api.npc(i32::from(WERE_NOT_AFRAID));
        } else if r == WHY_BLUE_LIGHT {
            self.api.set("WHY_MELNORME_BLUE", 1_u8);
            self.api.npc(i32::from(BLUE_IS_MAD));
        } else if r == WE_STRONG_1 {
            self.session.local_stack0 = 1;
            self.api.npc(i32::from(YOU_NOT_STRONG_1));
        } else if r == WE_STRONG_2 {
            self.session.local_stack0 = 2;
            self.api.npc(i32::from(YOU_NOT_STRONG_2));
        } else if r == WE_STRONG_3 {
            self.session.local_stack0 = 3;
            self.api.npc(i32::from(YOU_NOT_STRONG_3));
        } else if r == JUST_TESTING {
            self.session.local_stack1 = 1;
            self.api.npc(i32::from(REALLY_TESTING));
        }
        if !(self.api.get("WHY_MELNORME_BLUE") != 0) {
            self.api.response(WHY_BLUE_LIGHT, Handler::DoBluster, &[]);
        }
        match self.session.local_stack0 {
            0 => {
                self.api.response(WE_STRONG_1, Handler::DoBluster, &[]);
            }
            1 => {
                self.api.response(WE_STRONG_2, Handler::DoBluster, &[]);
            }
            2 => {
                self.api.response(WE_STRONG_3, Handler::DoBluster, &[]);
            }
            _ => {}
        }
        match self.session.local_stack1 {
            0 => {
                self.api.response(JUST_TESTING, Handler::DoBluster, &[]);
            }
            1 => {
                self.api
                    .response(YES_REALLY_TESTING, Handler::DoFirstMeeting, &[]);
            }
            _ => {}
        }
        self.api.response(YOURE_ON, Handler::ExitConversation, &[]);
    }
    fn yack1_respond(&mut self) {
        match self.api.get("MELNORME_YACK_STACK1") {
            0 => {
                self.api
                    .response(WHAT_ABOUT_YOURSELVES, Handler::DoFirstMeeting, &[]);
            }
            1 => {
                self.api
                    .response(WHAT_FACTORS, Handler::DoFirstMeeting, &[]);
                self.api
                    .response(GET_ON_WITH_BUSINESS, Handler::NatureOfConversation, &[]);
            }
            2 => {
                self.api
                    .response(GET_ON_WITH_BUSINESS, Handler::NatureOfConversation, &[]);
            }
            _ => {}
        }
    }
    fn yack2_respond(&mut self) {
        match self.api.get("MELNORME_YACK_STACK2") {
            0 => {
                self.api
                    .response(WHAT_ABOUT_UNIVERSE, Handler::DoFirstMeeting, &[]);
            }
            1 => {
                self.api
                    .response(GIVING_IS_GOOD_1, Handler::DoFirstMeeting, &[]);
            }
            2 => {
                self.api
                    .response(GIVING_IS_GOOD_2, Handler::DoFirstMeeting, &[]);
            }
            3 => {
                self.api
                    .response(TRADE_IS_FOR_THE_WEAK, Handler::DoBluster, &[]);
            }
            _ => {}
        }
    }
    fn do_melnorme_miffed(&mut self, r: u16) {
        if r == 0 {
            let mut miffed_count: u8;
            miffed_count = self.api.get("MELNORME_MIFFED_COUNT");
            let res = {
                let old = miffed_count;
                miffed_count += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HELLO_SLIGHTLY_ANGRY_1));
                }
                1 => {
                    self.api.npc(i32::from(HELLO_SLIGHTLY_ANGRY_2));
                }
                _ => {
                    miffed_count -= 1;
                    self.api.npc(i32::from(HELLO_SLIGHTLY_ANGRY_3));
                }
            }
            self.api.set("MELNORME_MIFFED_COUNT", miffed_count);
            self.api.fade_palette(2, 420);
        } else if r == EXPLORE_RELATIONSHIP {
            self.api.set("MELNORME_YACK_STACK3", 1_u8);
            self.api.npc(i32::from(EXAMPLE_OF_RELATIONSHIP));
        } else if r == EXCUSE_1 {
            self.api.set("MELNORME_YACK_STACK3", 2_u8);
            self.api.npc(i32::from(NO_EXCUSE_1));
        } else if r == EXCUSE_2 {
            self.api.set("MELNORME_YACK_STACK3", 3_u8);
            self.api.npc(i32::from(NO_EXCUSE_2));
        } else if r == EXCUSE_3 {
            self.api.set("MELNORME_YACK_STACK3", 4_u8);
            self.api.npc(i32::from(NO_EXCUSE_3));
        }
        match self.api.get("MELNORME_YACK_STACK3") {
            0 => {
                self.api
                    .response(EXPLORE_RELATIONSHIP, Handler::DoMelnormeMiffed, &[]);
            }
            1 => {
                self.api.response(EXCUSE_1, Handler::DoMelnormeMiffed, &[]);
            }
            2 => {
                self.api.response(EXCUSE_2, Handler::DoMelnormeMiffed, &[]);
            }
            3 => {
                self.api.response(EXCUSE_3, Handler::DoMelnormeMiffed, &[]);
            }
            _ => {}
        }
        self.api
            .response(WE_APOLOGIZE, Handler::DoFirstMeeting, &[]);
        self.api
            .response(SO_WE_CAN_ATTACK, Handler::ExitConversation, &[]);
        self.api
            .response(BYE_MELNORME_SLIGHTLY_ANGRY, Handler::ExitConversation, &[]);
    }
    fn do_melnorme_pissed(&mut self, r: u16) {
        if r == 0 {
            let mut pissed_count: u8;
            pissed_count = self.api.get("MELNORME_PISSED_COUNT");
            let res = {
                let old = pissed_count;
                pissed_count += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(HELLO_PISSED_OFF_1));
                }
                1 => {
                    self.api.npc(i32::from(HELLO_PISSED_OFF_2));
                }
                _ => {
                    pissed_count -= 1;
                    self.api.npc(i32::from(HELLO_PISSED_OFF_3));
                }
            }
            self.api.set("MELNORME_PISSED_COUNT", pissed_count);
            self.api.fade_palette(2, 420);
        } else if r == BEG_FORGIVENESS {
            self.api.set("MELNORME_YACK_STACK4", 1_u8);
            self.api.npc(i32::from(LOTS_TO_MAKE_UP_FOR));
        } else if r == YOU_ARE_SO_RIGHT {
            self.api.set("MELNORME_YACK_STACK4", 2_u8);
            self.api.npc(i32::from(ONE_LAST_CHANCE));
        }
        match self.api.get("MELNORME_YACK_STACK4") {
            0 => {
                self.api
                    .response(BEG_FORGIVENESS, Handler::DoMelnormePissed, &[]);
            }
            1 => {
                self.api
                    .response(YOU_ARE_SO_RIGHT, Handler::DoMelnormePissed, &[]);
            }
            2 => {
                self.api
                    .response(OK_STRIP_ME, Handler::ExitConversation, &[]);
            }
            _ => {}
        }
        self.api
            .response(FIGHT_SOME_MORE, Handler::ExitConversation, &[]);
        self.api
            .response(BYE_MELNORME_PISSED_OFF, Handler::ExitConversation, &[]);
    }
    fn do_melnorme_hate(&mut self, r: u16) {
        let mut hate_count: u8;
        let _ = r;
        hate_count = self.api.get("MELNORME_HATE_COUNT");
        let res = {
            let old = hate_count;
            hate_count += 1;
            old
        };
        match res {
            0 => {
                self.api.npc(i32::from(HELLO_HATE_YOU_1));
            }
            1 => {
                self.api.npc(i32::from(HELLO_HATE_YOU_2));
            }
            _ => {
                hate_count -= 1;
                self.api.npc(i32::from(HELLO_HATE_YOU_3));
            }
        }
        self.api.set("MELNORME_HATE_COUNT", hate_count);
        self.api.fade_palette(2, 420);
        self.api.response(
            WELL_IF_THATS_THE_WAY_YOU_FEEL,
            Handler::ExitConversation,
            &[],
        );
        self.api
            .response(YOU_HATE_US_SO_WE_GO_AWAY, Handler::ExitConversation, &[]);
    }
    fn intro(&mut self) {
        self.session.prev_msg_mode = self.api.status_mode(3);
        if self.api.get("MET_MELNORME") == 0 {
            self.api.set("MET_MELNORME", 1_u8);
            self.do_first_meeting(0);
        } else {
            match self.api.get("MELNORME_ANGER") {
                0 => {
                    if self.api.get("MELNORME_YACK_STACK2") <= 5 {
                        self.do_first_meeting(0);
                    } else {
                        self.nature_of_conversation(0);
                    }
                }
                1 => {
                    self.do_melnorme_miffed(0);
                }
                2 => {
                    self.do_melnorme_pissed(0);
                }
                _ => {
                    self.do_melnorme_hate(0);
                }
            }
        }
    }
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;
    static mut SESSION: Session = Session::new();

    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        unsafe {
            f(&mut Dialogue {
                api: &mut api,
                session: &mut *std::ptr::addr_of_mut!(SESSION),
            });
        }
    }
    extern "C" fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_rescue(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoRescue,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_buy(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoBuy,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_sell(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoSell,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn nature_of_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::NatureOfConversation,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_bluster(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoBluster,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_first_meeting(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoFirstMeeting,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_melnorme_miffed(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoMelnormeMiffed,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }
    extern "C" fn do_melnorme_pissed(r: u32) {
        run(|s| {
            s.handle(
                Handler::DoMelnormePissed,
                u16::try_from(r).expect("response exceeds COUNT"),
            )
        });
    }

    fn callback(h: Handler) -> extern "C" fn(u32) {
        match h {
            Handler::ExitConversation => exit_conversation,
            Handler::DoRescue => do_rescue,
            Handler::DoBuy => do_buy,
            Handler::DoSell => do_sell,
            Handler::NatureOfConversation => nature_of_conversation,
            Handler::DoBluster => do_bluster,
            Handler::DoFirstMeeting => do_first_meeting,
            Handler::DoMelnormeMiffed => do_melnorme_miffed,
            Handler::DoMelnormePissed => do_melnorme_pissed,
        }
    }
    pub(super) extern "C" fn intro() {
        run(|s| s.intro());
    }
    pub(super) extern "C" fn post() {
        run(|s| s.post_melnorme_enc());
    }
    pub(super) extern "C" fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_melnorme_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct MelnormDialogue;
impl super::RaceDialogue for MelnormDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_melnorme_comm()) }
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
