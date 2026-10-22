use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler, api::World>;
fn run(f: &mut Api, s: &mut Session, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f, session: s }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().iter().map(|x| x.0).collect()
}
#[test]
fn purchases_enforce_crew_and_escort_capacity() {
    for (r, cost, phrase, key) in [
        (BUY_DRUUGE_SHIP, 100, BOUGHT_SHIP, None),
        (BUY_ROSY_SPHERE, 100, BOUGHT_SPHERE, Some("ROSY_SPHERE")),
        (BUY_ART_1, 100, BOUGHT_ART_1, Some("ARTIFACT_2_ON_SHIP")),
        (BUY_ART_2, 100, BOUGHT_ART_2, Some("ARTIFACT_3_ON_SHIP")),
        (BUY_FUEL, 10, BOUGHT_FUEL, None),
    ] {
        for crew in [cost - 1, cost] {
            let mut f = Api::default();
            let mut s = Session::default();
            f.world.ship.crew_enlisted = crew;
            run(&mut f, &mut s, Handler::Buy, r);
            assert_eq!(s.slavery_count, if crew < cost { 0 } else { cost });
            assert_eq!(
                f.spoken(),
                [i32::from(if crew < cost {
                    NOT_ENOUGH_CREW
                } else {
                    phrase
                })]
            );
            if crew >= cost {
                assert!(f.effects.contains(&Effect::Gauges(
                    -(cost as i16),
                    if r == BUY_FUEL { 1000 } else { 0 },
                    0
                )));
                if let Some(key) = key {
                    assert_eq!(f.get(key), 1);
                }
            }
        }
    }
    let mut f = Api {
        feasibility: 0,
        ..Api::default()
    };
    let mut s = Session::default();
    f.world.ship.crew_enlisted = 100;
    run(&mut f, &mut s, Handler::Buy, BUY_DRUUGE_SHIP);
    assert_eq!(f.spoken(), [i32::from(NOT_ENOUGH_ROOM)]);
    assert_eq!(s.slavery_count, 0);
}
#[test]
fn crew_cost_thresholds_and_persisted_sale_total() {
    for (previous, sold, extra) in [
        (0, 100, 0),
        (100, 1, 7),
        (0, 251, 22),
        (101, 150, 15),
        (250, 1, 15),
        (251, 100, 0),
        (99, 1, 0),
    ] {
        let mut f = Api::default();
        let mut s = Session {
            slavery_count: sold,
            ..Session::default()
        };
        f.put("GLOBAL_FLAGS_AND_DATA", 128);
        f.put("CREW_SOLD_TO_DRUUGE0", previous as u8);
        f.put("CREW_SOLD_TO_DRUUGE1", (previous >> 8) as u8);
        run(&mut f, &mut s, Handler::ExitConversation, BYE);
        assert_eq!(f.world.crew_cost, 3 + extra);
        assert_eq!(
            u16::from(f.get("CREW_SOLD_TO_DRUUGE0"))
                | (u16::from(f.get("CREW_SOLD_TO_DRUUGE1")) << 8),
            previous + sold
        );
    }
}
#[test]
fn sphere_trade_remembers_original_artifact_and_withholds_regular_reward() {
    for (r, key) in [
        (SELL_MAIDENS, "MAIDENS_ON_SHIP"),
        (SELL_CASTER, "BURV_BROADCASTERS_ON_SHIP"),
        (SELL_SPAWNER, "PORTAL_SPAWNER_ON_SHIP"),
        (SELL_FRAGMENTS, "EGG_CASE0_ON_SHIP"),
    ] {
        let mut f = Api::default();
        let mut s = Session::default();
        f.put(key, 1);
        run(&mut f, &mut s, Handler::Trade, r);
        assert_eq!(s.last_response, r);
        assert_eq!(choices(&f), [NO_WAY, WAY, WHATS_THE_SPHERE_AGAIN]);
        run(&mut f, &mut s, Handler::Trade, WHATS_THE_SPHERE_AGAIN);
        assert_eq!(s.last_response, r);
        assert_eq!(choices(&f), [NO_WAY, WAY]);
        run(&mut f, &mut s, Handler::Sell, WAY);
        assert_eq!(f.get(key), 0);
        assert_eq!((f.get("ROSY_SPHERE"), f.get("ROSY_SPHERE_ON_SHIP")), (1, 1));
        assert_eq!(f.spoken(), [i32::from(OK_HERES_SPHERE)]);
        assert!(!f
            .effects
            .iter()
            .any(|e| matches!(e, Effect::AddEscorts(..) | Effect::Gauges(..))));
    }
}
#[test]
fn regular_trades_consume_fragments_in_order_and_report_available_slots() {
    let mut f = Api {
        feasibility: 0,
        ..Api::default()
    };
    let mut s = Session::default();
    f.put("ROSY_SPHERE", 1);
    f.put("EGG_CASE0_ON_SHIP", 1);
    f.put("EGG_CASE1_ON_SHIP", 1);
    run(&mut f, &mut s, Handler::Sell, SELL_FRAGMENTS);
    assert_eq!(
        (
            f.get("EGG_CASE0_ON_SHIP"),
            f.get("EGG_CASE1_ON_SHIP"),
            f.get("FRAGMENTS_BOUGHT")
        ),
        (0, 1, 1)
    );
    assert!(f.spoken().contains(&i32::from(DEAL_FOR_NO_SHIPS)));
    run(&mut f, &mut s, Handler::Sell, SELL_FRAGMENTS);
    assert_eq!(f.get("FRAGMENTS_BOUGHT"), 2);
    assert!(!choices(&f).contains(&SELL_FRAGMENTS));
    f.put("MAIDENS_ON_SHIP", 1);
    f.feasibility = 3;
    run(&mut f, &mut s, Handler::Sell, SELL_MAIDENS);
    assert!(f.effects.contains(&Effect::AddEscorts(13, 6)));
    assert!(f.spoken().contains(&i32::from(DEAL_FOR_LESS_SHIPS)));
}
#[test]
fn caster_fuel_trade_uses_reserve_modules_rounding_and_signed_gauge_chunks() {
    for (modules, fuel, amount, phrase) in [
        ([0, 0], 50, 10, GOOD_DEAL),
        ([5, 0], 1000, 50, FAIR_DEAL),
        ([6, 0], 1000, 100, BAD_DEAL),
        ([6, 6], 0, 210, BAD_DEAL),
    ] {
        let mut f = Api::default();
        let mut s = Session::default();
        f.world.ship.module_slots[..2].copy_from_slice(&modules);
        f.world.ship.fuel_on_board = fuel;
        f.put("ROSY_SPHERE", 1);
        f.put("BURV_BROADCASTERS_ON_SHIP", 1);
        run(&mut f, &mut s, Handler::Sell, SELL_CASTER);
        assert_eq!(f.get("BURV_BROADCASTERS_ON_SHIP"), 0);
        assert_eq!(f.world.numbers, [amount]);
        assert_eq!(
            f.spoken(),
            [BOUGHT_CASTER, YOU_GET, FUEL0, FUEL1, phrase].map(i32::from)
        );
        let gauges: Vec<_> = f
            .effects
            .iter()
            .filter_map(|e| {
                if let Effect::Gauges(0, n, 0) = e {
                    Some(*n)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            gauges.iter().map(|n| i32::from(*n)).sum::<i32>(),
            amount * 100 - if fuel == 50 { 50 } else { 0 }
        );
        assert!(gauges.iter().all(|n| *n <= 0x3FFF));
    }
    let mut f = Api::default();
    let mut s = Session::default();
    f.world.ship.module_slots.fill(6);
    f.put("ROSY_SPHERE", 1);
    f.put("PORTAL_SPAWNER_ON_SHIP", 1);
    run(&mut f, &mut s, Handler::Sell, SELL_SPAWNER);
    assert!(f.spoken().contains(&i32::from(HIDEOUS_DEAL)));
    assert!(f.effects.contains(&Effect::AddEscorts(13, 3)));
    assert!(f.spoken().contains(&i32::from(YOU_ALSO_GET)));
}
#[test]
fn salvage_requires_no_weapons_or_escorts_and_does_not_mark_player_attack() {
    for (escorts, module, salvage) in [
        (false, 0, true),
        (false, 9, false),
        (false, 10, false),
        (false, 11, false),
        (true, 0, false),
        (false, 12, true),
    ] {
        let mut f = Api::default();
        let mut s = Session::default();
        f.world.escorts = escorts;
        f.world.ship.module_slots[15] = module;
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .intro();
        assert_eq!(s.attempted_salvage, salvage);
        f.segue = 1;
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .post_druuge_enc();
        assert_eq!(f.get("ATTACKED_DRUUGE"), u8::from(!salvage));
    }
}
#[test]
fn bomb_trade_world_outtakes_and_encounter_reset() {
    let mut f = Api::default();
    let mut s = Session::default();
    f.put("GLOBAL_FLAGS_AND_DATA", 64);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .intro();
    assert_eq!(choices(&f), [WHATS_UP_AT_BOMB_PLANET, BYE]);
    run(&mut f, &mut s, Handler::BombAmbush, WHATS_UP_AT_BOMB_PLANET);
    assert_eq!(choices(&f), [WE_GET_BOMB, BYE]);
    run(&mut f, &mut s, Handler::BombAmbush, WE_GET_BOMB);
    assert_eq!(choices(&f), [THEN_WE_TAKE_BOMB, BYE]);
    f.put("GLOBAL_FLAGS_AND_DATA", 128);
    f.effects.clear();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .intro();
    assert_eq!(
        choices(&f),
        [WHATS_UP_AT_TRADE_WORLD, WANT_TO_SELL, WANT_TO_BUY, BYE]
    );
    run(
        &mut f,
        &mut s,
        Handler::ExplainSlaveTrade,
        WHAT_DO_WITH_CREW,
    );
    assert_eq!(f.get("KNOW_DRUUGE_SLAVERS"), 2);
    s.slavery_count = 50;
    s.attempted_salvage = true;
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .initialize();
    assert_eq!(s.slavery_count, 0);
    assert!(!s.attempted_salvage);
    assert_eq!(f.segue, 0);
    f.activity = 5;
    f.effects.clear();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    let d = descriptor();
    assert_eq!(d.alien_text_baseline.y, 70);
    assert_eq!(d.alien_text_valign, 1);
}
