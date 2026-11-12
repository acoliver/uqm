//! Druuge dialogue, ported from druugec.c.
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    ExitConversation,
    Buy,
    Trade,
    Sell,
    ExplainSlaveTrade,
    TradeWorld,
    BombAmbush,
    Space,
}
#[derive(Default)]
pub(crate) struct Session {
    slavery_count: u16,
    attempted_salvage: bool,
    last_response: u16,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
    pub(crate) session: &'a mut Session,
}
pub(crate) mod api;
pub(crate) use api::DruugeApi;
impl<A: DruugeApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: u16) {
        match h {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::Buy => self.buy(r),
            Handler::Trade => self.trade(r),
            Handler::Sell => self.sell(r),
            Handler::ExplainSlaveTrade => self.explain_slave_trade(r),
            Handler::TradeWorld => self.trade_world(r),
            Handler::BombAmbush => self.bomb_ambush(r),
            Handler::Space => self.space(r),
        }
    }
    pub(crate) fn exit_conversation(&mut self, r: u16) {
        if r == BYE {
            self.api.segue(0_u32);
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                if self.session.slavery_count != 0 {
                    let previous_slaves: u16 =
                        u16::from(self.api.get(StoryKey::CREW_SOLD_TO_DRUUGE0))
                            | (u16::from(self.api.get(StoryKey::CREW_SOLD_TO_DRUUGE1)) << 8);
                    self.session.slavery_count =
                        self.session.slavery_count.wrapping_add(previous_slaves);
                    if (i32::from(self.session.slavery_count) > 250)
                        && (i32::from(previous_slaves) <= 250)
                    {
                        if i32::from(previous_slaves) > 100 {
                            self.api
                                .set_crew_cost(self.api.crew_cost().wrapping_add((22 - 7) as u8));
                        } else {
                            self.api
                                .set_crew_cost(self.api.crew_cost().wrapping_add(22_u8));
                        }
                    } else if (i32::from(self.session.slavery_count) > 100)
                        && (i32::from(previous_slaves) <= 100)
                    {
                        self.api
                            .set_crew_cost(self.api.crew_cost().wrapping_add(7_u8));
                    }
                    self.api.set(
                        StoryKey::CREW_SOLD_TO_DRUUGE0,
                        (self.session.slavery_count) as u8,
                    );
                    self.api.set(
                        StoryKey::CREW_SOLD_TO_DRUUGE1,
                        (self.session.slavery_count >> 8) as u8,
                    );
                }
                match self.api.get(StoryKey::DRUUGE_HOME_VISITS) {
                    1 => {
                        self.api.npc(i32::from(BYE_FROM_TRADE_WORLD_1));
                    }
                    _ => {
                        self.api.npc(i32::from(BYE_FROM_TRADE_WORLD_2));
                    }
                }
            } else if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 6)) != 0 {
                self.api.npc(i32::from(GOODBYE_FROM_BOMB_PLANET));
            } else {
                self.api.npc(i32::from(GOODBYE_FROM_SPACE));
            }
        } else {
            self.api.segue(1_u32);
            self.api.npc(i32::from(FIGHT_FOR_BOMB));
        }
    }
    pub(crate) fn buy(&mut self, r: u16) {
        if (r == WANT_TO_BUY) || (r == IM_READY_TO_BUY) {
            self.api.npc(i32::from(READY_TO_SELL));
            if !(self.api.get(StoryKey::ROSY_SPHERE) != 0) {
                self.api.npc(i32::from(HAVE_SPHERE));
            }
            if !(self.api.get(StoryKey::ARTIFACT_2_ON_SHIP) != 0) {
                self.api.npc(i32::from(HAVE_ART_1));
            }
            if !(self.api.get(StoryKey::ARTIFACT_3_ON_SHIP) != 0) {
                self.api.npc(i32::from(HAVE_ART_2));
            }
            self.api.npc(i32::from(SHIPS_AND_FUEL));
            self.api.set(StoryKey::KNOW_DRUUGE_SLAVERS, 3_u8);
        } else if r == BUY_DRUUGE_SHIP {
            if i32::from(self.api.ship().crew_enlisted) < 100 {
                self.api.npc(i32::from(NOT_ENOUGH_CREW));
            } else if i32::from(
                self.api
                    .escort_feasibility((crate::engine::species::ship::DRUUGE_SHIP) as u16),
            ) == 0
            {
                self.api.npc(i32::from(NOT_ENOUGH_ROOM));
            } else {
                self.api.gauges(-100_i16, 0_i16, 0);
                self.session.slavery_count = self.session.slavery_count.wrapping_add(100_u16);
                self.api
                    .add_escorts((crate::engine::species::ship::DRUUGE_SHIP) as u16, 1_i16);
                self.api.npc(i32::from(BOUGHT_SHIP));
            }
        } else if r == BUY_ROSY_SPHERE {
            if i32::from(self.api.ship().crew_enlisted) < 100 {
                self.api.npc(i32::from(NOT_ENOUGH_CREW));
            } else {
                self.api.gauges(-100_i16, 0_i16, 0);
                self.session.slavery_count = self.session.slavery_count.wrapping_add(100_u16);
                self.api.set(StoryKey::ROSY_SPHERE_ON_SHIP, 1_u8);
                self.api.set(StoryKey::ROSY_SPHERE, 1_u8);
                self.api.npc(i32::from(BOUGHT_SPHERE));
            }
        } else if r == BUY_ART_1 {
            if i32::from(self.api.ship().crew_enlisted) < 100 {
                self.api.npc(i32::from(NOT_ENOUGH_CREW));
            } else {
                self.api.gauges(-100_i16, 0_i16, 0);
                self.session.slavery_count = self.session.slavery_count.wrapping_add(100_u16);
                self.api.set(StoryKey::ARTIFACT_2_ON_SHIP, 1_u8);
                self.api.npc(i32::from(BOUGHT_ART_1));
            }
        } else if r == BUY_ART_2 {
            if i32::from(self.api.ship().crew_enlisted) < 100 {
                self.api.npc(i32::from(NOT_ENOUGH_CREW));
            } else {
                self.api.gauges(-100_i16, 0_i16, 0);
                self.session.slavery_count = self.session.slavery_count.wrapping_add(100_u16);
                self.api.set(StoryKey::ARTIFACT_3_ON_SHIP, 1_u8);
                self.api.npc(i32::from(BOUGHT_ART_2));
            }
        } else if r == BUY_FUEL {
            if i32::from(self.api.ship().crew_enlisted) < 10 {
                self.api.npc(i32::from(NOT_ENOUGH_CREW));
            } else {
                self.api.gauges(-10_i16, (10 * 100) as i16, 0);
                self.session.slavery_count = self.session.slavery_count.wrapping_add(10_u16);
                self.api.npc(i32::from(BOUGHT_FUEL));
            }
        }
        self.api.response(BUY_DRUUGE_SHIP, Handler::Buy, &[]);
        if !(self.api.get(StoryKey::ROSY_SPHERE) != 0) {
            self.api.response(BUY_ROSY_SPHERE, Handler::Buy, &[]);
        }
        if !(self.api.get(StoryKey::ARTIFACT_2_ON_SHIP) != 0) {
            self.api.response(BUY_ART_1, Handler::Buy, &[]);
        }
        if !(self.api.get(StoryKey::ARTIFACT_3_ON_SHIP) != 0) {
            self.api.response(BUY_ART_2, Handler::Buy, &[]);
        }
        self.api.response(BUY_FUEL, Handler::Buy, &[]);
        self.api.response(DONE_BUYING, Handler::TradeWorld, &[]);
    }
    pub(crate) fn trade(&mut self, r: u16) {
        if !(r == WHATS_THE_SPHERE_AGAIN) {
            self.api.npc(i32::from(TRADE_FOR_SPHERE));
            self.session.last_response = r;
        } else {
            self.api.npc(i32::from(SPHERE_IS));
            self.api.disable(WHATS_THE_SPHERE_AGAIN);
        }
        self.api.response(NO_WAY, Handler::Sell, &[]);
        self.api.response(WAY, Handler::Sell, &[]);
        if self.api.enabled(WHATS_THE_SPHERE_AGAIN) {
            self.api
                .response(WHATS_THE_SPHERE_AGAIN, Handler::Trade, &[]);
        }
    }
    pub(crate) fn do_transaction(&mut self, r: u16) {
        if r == SELL_MAIDENS {
            self.api.set(StoryKey::MAIDENS_ON_SHIP, 0);
        } else if r == SELL_FRAGMENTS {
            for key in api::EGG_CASES {
                if self.api.get(key) != 0 {
                    self.api.set(key, 0);
                    break;
                }
            }
            let num_frags = self.api.get(StoryKey::FRAGMENTS_BOUGHT).wrapping_add(1);
            self.api.set(StoryKey::FRAGMENTS_BOUGHT, num_frags);
        } else if r == SELL_CASTER {
            self.api.set(StoryKey::BURV_BROADCASTERS_ON_SHIP, 0);
        } else if r == SELL_SPAWNER {
            self.api.set(StoryKey::PORTAL_SPAWNER_ON_SHIP, 0);
        }
        if self.api.get(StoryKey::ROSY_SPHERE) == 0
            && self.api.get(StoryKey::ROSY_SPHERE_ON_SHIP) != 0
        {
            self.api.set(StoryKey::ROSY_SPHERE, 1);
        } else {
            let mut trade_gas = false;
            let mut ships_to_trade = 0;
            let ship_slots = self
                .api
                .escort_feasibility(crate::engine::species::ship::DRUUGE_SHIP as u16);
            if r == SELL_MAIDENS {
                self.api.npc(i32::from(BOUGHT_MAIDENS));
                ships_to_trade = 6;
            } else if r == SELL_FRAGMENTS {
                self.api.npc(i32::from(BOUGHT_FRAGMENTS));
                ships_to_trade = 1;
            } else if r == SELL_CASTER {
                self.api.npc(i32::from(BOUGHT_CASTER));
                trade_gas = true;
            } else if r == SELL_SPAWNER {
                self.api.npc(i32::from(BOUGHT_SPAWNER));
                ships_to_trade = 3;
                trade_gas = true;
            }
            self.api.npc(i32::from(YOU_GET));
            if ships_to_trade != 0 {
                self.api.add_escorts(
                    crate::engine::species::ship::DRUUGE_SHIP as u16,
                    ships_to_trade,
                );
                self.api
                    .npc(i32::from(if ship_slots >= ships_to_trade as u16 {
                        DEAL_FOR_STATED_SHIPS
                    } else if ship_slots == 0 {
                        DEAL_FOR_NO_SHIPS
                    } else {
                        DEAL_FOR_LESS_SHIPS
                    }));
                if trade_gas {
                    self.api.npc(i32::from(YOU_ALSO_GET));
                }
            }
            if trade_gas {
                let ship = self.api.ship();
                let mut capacity = 1000u32;
                for module in ship.module_slots.iter().rev() {
                    if *module == 5 || *module == 6 {
                        capacity += if *module == 5 { 5000 } else { 10000 };
                    }
                }
                capacity = capacity.wrapping_sub(ship.fuel_on_board);
                let f = ((capacity + 50) / 100) as u16;
                while capacity > 0x3FFF {
                    self.api.gauges(0, 0x3FFF, 0);
                    capacity -= 0x3FFF;
                }
                self.api.gauges(0, capacity as i16, 0);
                self.api.npc(i32::from(FUEL0));
                self.api.number(i32::from(f));
                self.api.npc(i32::from(FUEL1));
                self.api.npc(i32::from(if f >= 250 {
                    HIDEOUS_DEAL
                } else if f >= 100 {
                    BAD_DEAL
                } else if f >= 50 {
                    FAIR_DEAL
                } else if f >= 10 {
                    GOOD_DEAL
                } else {
                    FINE_DEAL
                }));
            }
        }
    }

    pub(crate) fn sell(&mut self, r: u16) {
        if r == WANT_TO_SELL {
            self.api.npc(i32::from(READY_TO_BUY));
        } else if (r == NO_WAY) || (r == WAY) {
            if r == NO_WAY {
                self.api.npc(i32::from(OK_REGULAR_DEAL));
            } else {
                self.api.npc(i32::from(OK_HERES_SPHERE));
                self.api.set(StoryKey::ROSY_SPHERE_ON_SHIP, 1_u8);
            }
            self.do_transaction(self.session.last_response);
        } else if (((r == SELL_MAIDENS) || (r == SELL_FRAGMENTS)) || (r == SELL_CASTER))
            || (r == SELL_SPAWNER)
        {
            self.do_transaction(r);
        }
        let resp_func: Handler = if !(self.api.get(StoryKey::ROSY_SPHERE) != 0) {
            Handler::Trade
        } else {
            Handler::Sell
        };
        if self.api.get(StoryKey::MAIDENS_ON_SHIP) != 0 {
            self.api.response(SELL_MAIDENS, resp_func, &[]);
        }
        if (((self.api.get(StoryKey::EGG_CASE0_ON_SHIP) != 0)
            || (self.api.get(StoryKey::EGG_CASE1_ON_SHIP) != 0))
            || (self.api.get(StoryKey::EGG_CASE2_ON_SHIP) != 0))
            && (i32::from(self.api.get(StoryKey::FRAGMENTS_BOUGHT)) < 2)
        {
            self.api.response(SELL_FRAGMENTS, resp_func, &[]);
        }
        if self.api.get(StoryKey::BURV_BROADCASTERS_ON_SHIP) != 0 {
            self.api.response(SELL_CASTER, resp_func, &[]);
        }
        if self.api.get(StoryKey::PORTAL_SPAWNER_ON_SHIP) != 0 {
            self.api.response(SELL_SPAWNER, resp_func, &[]);
        }
        self.api.response(DONE_SELLING, Handler::TradeWorld, &[]);
    }
    pub(crate) fn explain_slave_trade(&mut self, r: u16) {
        if r == WANT_TO_BUY {
            self.api.npc(i32::from(WE_SELL_FOR_CREW));
        } else if r == ISNT_THIS_SLAVE_TRADING {
            self.api.npc(i32::from(NO_SLAVE_TRADE));
            self.api.set(StoryKey::KNOW_DRUUGE_SLAVERS, 1_u8);
        } else if r == WHAT_DO_WITH_CREW {
            self.api.npc(i32::from(HAVE_FUN));
            self.api.set(StoryKey::KNOW_DRUUGE_SLAVERS, 2_u8);
        }
        match self.api.get(StoryKey::KNOW_DRUUGE_SLAVERS) {
            0 => {
                self.api
                    .response(ISNT_THIS_SLAVE_TRADING, Handler::ExplainSlaveTrade, &[]);
            }
            1 => {
                self.api
                    .response(WHAT_DO_WITH_CREW, Handler::ExplainSlaveTrade, &[]);
            }
            _ => {}
        }
        self.api
            .response(I_WILL_NEVER_TRADE_CREW, Handler::TradeWorld, &[]);
        self.api.response(IM_READY_TO_BUY, Handler::Buy, &[]);
    }
    pub(crate) fn trade_world(&mut self, r: u16) {
        if r == WHATS_UP_AT_TRADE_WORLD {
            let mut num_visits: u8;
            num_visits = self.api.get(StoryKey::DRUUGE_HOME_INFO);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GEN_INFO_AT_TRADE_WORLD_1));
                }
                1 => {
                    self.api.npc(i32::from(GEN_INFO_AT_TRADE_WORLD_2));
                }
                2 => {
                    self.api.npc(i32::from(GEN_INFO_AT_TRADE_WORLD_3));
                    if i32::from(self.api.get(StoryKey::KNOW_ABOUT_SHATTERED)) < 2 {
                        self.api.set(StoryKey::KNOW_ABOUT_SHATTERED, 2_u8);
                    }
                }
                3 => {
                    self.api.npc(i32::from(GEN_INFO_AT_TRADE_WORLD_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::DRUUGE_HOME_INFO, num_visits);
            self.api.disable(WHATS_UP_AT_TRADE_WORLD);
        } else if r == DONE_SELLING {
            self.api.npc(i32::from(OK_DONE_SELLING));
        } else if r == DONE_BUYING {
            self.api.npc(i32::from(OK_DONE_BUYING));
        } else if r == I_WILL_NEVER_TRADE_CREW {
            self.api.npc(i32::from(YOUR_LOSS));
        }
        if self.api.enabled(WHATS_UP_AT_TRADE_WORLD) {
            self.api
                .response(WHATS_UP_AT_TRADE_WORLD, Handler::TradeWorld, &[]);
        }
        self.api.response(WANT_TO_SELL, Handler::Sell, &[]);
        if i32::from(self.api.get(StoryKey::KNOW_DRUUGE_SLAVERS)) == 3 {
            self.api.response(WANT_TO_BUY, Handler::Buy, &[]);
        } else {
            self.api
                .response(WANT_TO_BUY, Handler::ExplainSlaveTrade, &[]);
        }
        self.api.response(BYE, Handler::ExitConversation, &[]);
    }
    pub(crate) fn bomb_ambush(&mut self, r: u16) {
        if r == WHATS_UP_AT_BOMB_PLANET {
            self.api.npc(i32::from(GEN_INFO_AT_BOMB_PLANET));
            self.api.set(StoryKey::BOMB_VISITS, 2_u8);
        } else if r == WE_GET_BOMB {
            self.api.npc(i32::from(NOT_GET_BOMB));
            self.api.set(StoryKey::BOMB_VISITS, 3_u8);
        }
        match self.api.get(StoryKey::BOMB_VISITS) {
            1 => {
                self.api
                    .response(WHATS_UP_AT_BOMB_PLANET, Handler::BombAmbush, &[]);
            }
            2 => {
                self.api.response(WE_GET_BOMB, Handler::BombAmbush, &[]);
            }
            _ => {
                self.api
                    .response(THEN_WE_TAKE_BOMB, Handler::ExitConversation, &[]);
            }
        }
        self.api.response(BYE, Handler::ExitConversation, &[]);
    }
    pub(crate) fn space(&mut self, r: u16) {
        if r == WHATS_UP_IN_SPACE {
            let mut num_visits: u8;
            num_visits = self.api.get(StoryKey::DRUUGE_SPACE_INFO);
            let res = {
                let old = num_visits;
                num_visits = num_visits.wrapping_add(1);
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(GENERAL_INFO_IN_SPACE_1));
                }
                1 => {
                    self.api.npc(i32::from(GENERAL_INFO_IN_SPACE_2));
                }
                2 => {
                    self.api.npc(i32::from(GENERAL_INFO_IN_SPACE_3));
                }
                3 => {
                    self.api.npc(i32::from(GENERAL_INFO_IN_SPACE_4));
                    {
                        num_visits = num_visits.wrapping_sub(1);
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::DRUUGE_SPACE_INFO, num_visits);
            self.api.disable(WHATS_UP_IN_SPACE);
        }
        if self.api.enabled(WHATS_UP_IN_SPACE) {
            self.api.response(WHATS_UP_IN_SPACE, Handler::Space, &[]);
        }
        self.api.response(BYE, Handler::ExitConversation, &[]);
    }
    pub(crate) fn intro(&mut self) {
        let mut visits: u8;
        if i32::from((self.api.activity()) as u8) == 5 {
            self.api.npc(i32::from(OUT_TAKES));
            self.api.segue(0_u32);
            return;
        }
        if self.api.get(StoryKey::DRUUGE_MANNER) != 0 {
            if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
                visits = self.api.get(StoryKey::DRUUGE_HOME_VISITS);
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => self.api.npc(i32::from(HSTL_TRADE_WORLD_HELLO_1)),
                    1 => {
                        self.api.npc(i32::from(HSTL_TRADE_WORLD_HELLO_2));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::DRUUGE_HOME_VISITS, visits);
            } else {
                visits = self.api.get(StoryKey::DRUUGE_VISITS);
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => self.api.npc(i32::from(HOSTILE_SPACE_HELLO_1)),
                    1 => {
                        self.api.npc(i32::from(HOSTILE_SPACE_HELLO_2));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::DRUUGE_VISITS, visits);
            }
            self.api.segue(1_u32);
        } else if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0 {
            {
                visits = self.api.get(StoryKey::DRUUGE_HOME_VISITS);
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => self.api.npc(i32::from(INITIAL_TRADE_WORLD_HELLO)),
                    1 => self.api.npc(i32::from(SSQ_TRADE_WORLD_HELLO_1)),
                    2 => self.api.npc(i32::from(SSQ_TRADE_WORLD_HELLO_2)),
                    3 => self.api.npc(i32::from(SSQ_TRADE_WORLD_HELLO_3)),
                    4 => {
                        self.api.npc(i32::from(SSQ_TRADE_WORLD_HELLO_4));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::DRUUGE_HOME_VISITS, visits);
            }
            if (self.api.get(StoryKey::ATTACKED_DRUUGE) != 0)
                && !(self.api.get(StoryKey::DRUUGE_DISCLAIMER) != 0)
            {
                self.api.set(StoryKey::DRUUGE_DISCLAIMER, 1_u8);
            }
            if (self.api.get(StoryKey::MAIDENS_ON_SHIP) != 0)
                && !(self.api.get(StoryKey::SCANNED_MAIDENS) != 0)
            {
                self.api.npc(i32::from(SCAN_MAIDENS));
                self.api.set(StoryKey::SCANNED_MAIDENS, 1_u8);
            }
            if (((self.api.get(StoryKey::EGG_CASE0_ON_SHIP) != 0)
                || (self.api.get(StoryKey::EGG_CASE1_ON_SHIP) != 0))
                || (self.api.get(StoryKey::EGG_CASE2_ON_SHIP) != 0))
                && !(self.api.get(StoryKey::SCANNED_FRAGMENTS) != 0)
            {
                if i32::from(self.api.get(StoryKey::FRAGMENTS_BOUGHT)) < 2 {
                    self.api.npc(i32::from(SCAN_FRAGMENTS));
                } else {
                    self.api.npc(i32::from(ENOUGH_FRAGMENTS));
                }
                self.api.set(StoryKey::SCANNED_FRAGMENTS, 1_u8);
            }
            if (self.api.get(StoryKey::BURV_BROADCASTERS_ON_SHIP) != 0)
                && !(self.api.get(StoryKey::SCANNED_CASTER) != 0)
            {
                self.api.npc(i32::from(SCAN_DRUUGE_CASTER));
                self.api.set(StoryKey::SCANNED_CASTER, 1_u8);
            }
            if (self.api.get(StoryKey::PORTAL_SPAWNER_ON_SHIP) != 0)
                && !(self.api.get(StoryKey::SCANNED_SPAWNER) != 0)
            {
                self.api.npc(i32::from(SCAN_ARILOU_SPAWNER));
                self.api.set(StoryKey::SCANNED_SPAWNER, 1_u8);
            }
            self.trade_world(0_u16);
        } else if (i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 6)) != 0 {
            if self.api.get(StoryKey::BOMB_VISITS) != 0 {
                self.api.npc(i32::from(SUBSEQ_BOMB_WORLD_HELLO));
            } else {
                self.api.npc(i32::from(INIT_BOMB_WORLD_HELLO));
                self.api.set(StoryKey::BOMB_VISITS, 1_u8);
            }
            self.bomb_ambush(0_u16);
        } else if self.api.get(StoryKey::ATTACKED_DRUUGE) != 0 {
            visits = self.api.get(StoryKey::DRUUGE_VISITS);
            let res = {
                let old = visits;
                visits = visits.wrapping_add(1);
                old
            };
            match res {
                0 => self.api.npc(i32::from(HOSTILE_SPACE_HELLO_1)),
                1 => {
                    self.api.npc(i32::from(HOSTILE_SPACE_HELLO_2));
                    {
                        visits = visits.wrapping_sub(1);
                        visits
                    };
                }
                _ => {}
            }
            self.api.set(StoryKey::DRUUGE_VISITS, visits);
            self.api.segue(1_u32);
        } else {
            visits = 0_u8;
            if !self.api.has_escorts() {
                visits = 16;
                for module in self.api.ship().module_slots {
                    if (9..=11).contains(&module) {
                        visits = 0;
                        break;
                    }
                }
            }
            if visits != 0 {
                visits = self.api.get(StoryKey::DRUUGE_SALVAGE);
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => self.api.npc(i32::from(SALVAGE_YOUR_SHIP_1)),
                    1 => {
                        self.api.npc(i32::from(SALVAGE_YOUR_SHIP_2));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::DRUUGE_SALVAGE, visits);
                self.api.segue(1_u32);
                self.session.attempted_salvage = true;
            } else {
                visits = self.api.get(StoryKey::DRUUGE_VISITS);
                let res = {
                    let old = visits;
                    visits = visits.wrapping_add(1);
                    old
                };
                match res {
                    0 => self.api.npc(i32::from(INIT_SPACE_HELLO)),
                    1 => {
                        self.api.npc(i32::from(SUBSEQUENT_SPACE_HELLO));
                        {
                            visits = visits.wrapping_sub(1);
                            visits
                        };
                    }
                    _ => {}
                }
                self.api.set(StoryKey::DRUUGE_VISITS, visits);
                self.space(0_u16);
            }
        }
    }
    pub(crate) fn post_druuge_enc(&mut self) {
        if (((self.api.get_segue()) as i32 == 1) && !self.session.attempted_salvage)
            && !(self.api.get(StoryKey::DRUUGE_MANNER) != 0)
            && !(self.api.get(StoryKey::ATTACKED_DRUUGE) != 0)
        {
            self.api.set(StoryKey::ATTACKED_DRUUGE, 1_u8);
            self.api.set(StoryKey::DRUUGE_VISITS, 0_u8);
        }
    }
    pub(crate) fn initialize(&mut self) {
        self.session.slavery_count = 0;
        self.session.attempted_salvage = false;
        if ((i32::from(self.api.get(StoryKey::DRUUGE_MANNER)) == 0)
            && ((i32::from(self.api.get(StoryKey::GLOBAL_FLAGS_AND_DATA)) & (1 << 7)) != 0))
            || (i32::from((self.api.activity()) as u8) == 5)
        {
            self.api.segue(0_u32);
        } else {
            self.api.segue(1_u32);
        }
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
