mod branches;
use super::*;
use crate::comm::races::encounter::testing::Fake;
use crate::comm::races::script::ScriptApi;

#[derive(Default)]
struct Api {
    base: Fake<Handler>,
    world: World,
}
impl std::ops::Deref for Api {
    type Target = Fake<Handler>;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl std::ops::DerefMut for Api {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
impl ScriptApi<Handler> for Api {
    fn get(&self, k: &str) -> u8 {
        self.base.get(k)
    }
    fn set(&mut self, k: &str, v: u8) {
        self.base.set(k, v);
    }
    fn npc(&mut self, p: i32) {
        self.base.npc(p);
    }
    fn response(&mut self, p: u16, h: Handler, f: &[i32]) {
        self.base.response(p, h, f);
    }
    fn enabled(&self, p: u16) -> bool {
        self.base.enabled(p)
    }
    fn disable(&mut self, p: u16) {
        self.base.disable(p);
    }
    fn supplies(&self) -> super::super::script::Supplies {
        panic!("unexpected supplies")
    }
    fn add_lander(&mut self) {
        panic!("unexpected lander")
    }
    fn gauges(&mut self, c: i16, f: i16, u: i32) {
        self.base.gauges(c, f, u);
        if f != 0x7fff {
            self.world.ship.fuel_on_board =
                (i64::from(self.world.ship.fuel_on_board) + i64::from(f)).min(i64::from(
                    1000 + self
                        .world
                        .ship
                        .module_slots
                        .iter()
                        .map(|&m| match m {
                            5 => 5000,
                            6 => 10000,
                            _ => 0,
                        })
                        .sum::<u32>(),
                )) as u32;
        }
    }
    fn palette(&mut self, _: u16) {}
    fn talk_segue(&mut self, t: u16) {
        self.base.talk_segue(t);
    }
    fn disable_animation(&mut self, _: usize) {
        panic!("unexpected animation")
    }
    fn fade_palette(&mut self, _: u16, _: i16) {}
}
impl EncounterApi<Handler> for Api {
    fn segue(&mut self, v: u32) {
        self.base.segue(v);
    }
    fn get_segue(&self) -> u32 {
        self.base.get_segue()
    }
    fn activity(&self) -> u16 {
        self.base.activity()
    }
    fn alliance(&self, r: u16) -> u16 {
        self.base.alliance(r)
    }
    fn escort_feasibility(&self, r: u16) -> u16 {
        self.base.escort_feasibility(r)
    }
    fn add_escorts(&mut self, r: u16, n: i16) {
        self.base.add_escorts(r, n);
    }
    fn fwiffo_crew(&mut self) {
        panic!("unexpected fwiffo")
    }
    fn advance_ilwrath(&mut self) {
        panic!("unexpected ilwrath")
    }
}
struct World {
    ship: CSisState,
    costs: [u8; 20],
    numbers: Vec<i32>,
    status: Vec<i32>,
    skip: bool,
}
impl Default for World {
    fn default() -> Self {
        Self {
            ship: empty_ship(),
            costs: [0; 20],
            numbers: vec![],
            status: vec![],
            skip: true,
        }
    }
}
impl MelnormApi for Api {
    fn ship(&self) -> CSisState {
        self.world.ship
    }
    fn set_ship(&mut self, ship: CSisState) {
        self.world.ship = ship;
    }
    fn module_cost(&self, id: usize) -> u8 {
        self.world.costs[id]
    }
    fn set_module_cost(&mut self, id: usize, cost: u8) {
        self.world.costs[id] = cost;
    }
    fn number(&mut self, n: i32) {
        self.world.numbers.push(n);
    }
    fn status_mode(&mut self, mode: i32) -> i32 {
        self.world.status.push(mode);
        1
    }
    fn draw_status(&mut self) {
        self.world.status.push(-1);
    }
    fn draw_cargo(&mut self, _: u8, _: u8) {}
    fn sleep(&mut self, _: u32) {}
    fn now(&mut self) -> u32 {
        0
    }
    fn skip_sale(&mut self) -> bool {
        self.world.skip
    }
    fn clear_display(&mut self) {}
}
fn run(f: &mut Api, s: &mut Session, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f, session: s }.handle(h, r);
}
#[test]
fn credit_purchase_is_atomic_and_reports_exact_shortfall() {
    let mut f = Api::default();
    let mut s = Session::new();
    let mut d = Dialogue {
        api: &mut f,
        session: &mut s,
    };
    d.set_credits(149);
    assert!(!d.delta_credit(-150));
    assert_eq!(d.credits(), 149);
    assert_eq!(d.api.spoken(), [146, 147]);
    assert_eq!(d.api.world.numbers, [1]);
    assert!(d.delta_credit(1));
    assert!(d.delta_credit(-150));
    assert_eq!(d.credits(), 0);
    d.set_credits(65535);
    assert!(d.delta_credit(2));
    assert_eq!(d.credits(), 1);
}
#[test]
fn technology_catalogue_order_prices_and_grants() {
    let mut f = Api::default();
    let mut s = Session::new();
    for i in 0..13 {
        {
            let mut d = Dialogue {
                api: &mut f,
                session: &mut s,
            };
            d.set_credits(149);
            assert_eq!(d.next_tech(), Some(i));
        }
        run(&mut f, &mut s, Handler::DoBuy, BUY_NEW_TECH);
        assert_eq!(
            Dialogue {
                api: &mut f,
                session: &mut s
            }
            .next_tech(),
            Some(i)
        );
        assert_eq!(f.world.numbers.last(), Some(&1));
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .set_credits(150);
        run(&mut f, &mut s, Handler::DoBuy, BUY_NEW_TECH);
        assert!(f.spoken().contains(&(218 + i as i32)));
        let d = Dialogue {
            api: &mut f,
            session: &mut s,
        };
        assert_eq!(d.credits(), 0);
        assert!(d.has_tech(i));
        assert_eq!(d.next_tech(), if i == 12 { None } else { Some(i + 1) });
    }
    assert_eq!(f.world.costs[10], 80);
    assert_eq!(f.world.costs[13], 80);
    assert_eq!(f.world.costs[6], 20);
    assert_eq!(f.world.costs[12], 100);
    assert_eq!(f.world.costs[11], 120);
    assert_eq!(f.world.costs[8], 80);
    assert_eq!(f.get("LANDER_SHIELDS"), 15);
}
#[test]
fn rainbow_sale_credits_each_new_world_once() {
    let mut f = Api::default();
    let mut s = Session::new();
    f.put("RAINBOW_WORLD0", 0b1011);
    f.put("RAINBOW_WORLD1", 1);
    f.put("MELNORME_RAINBOW_COUNT", 1);
    f.world.ship.fuel_on_board = 1;
    run(&mut f, &mut s, Handler::DoSell, SELL_RAINBOW_LOCATIONS);
    assert_eq!(&f.spoken()[..3], [122, 123, 124]);
    assert_eq!(f.world.numbers, [3, 1500]);
    assert_eq!(f.get("MELNORME_RAINBOW_COUNT"), 4);
    assert_eq!(
        Dialogue {
            api: &mut f,
            session: &mut s
        }
        .credits(),
        1500
    );
    assert!(!f.enabled(SELL));
}
