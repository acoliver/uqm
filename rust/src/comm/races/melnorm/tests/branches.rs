use super::*;
use crate::comm::races::encounter::testing::Effect;
#[test]
fn fuel_prices_capacity_shortfall_and_large_fill() {
    for (r, units) in [(137, 1), (139, 5), (140, 10), (141, 25)] {
        let mut f = Api::default();
        let mut s = Session::new();
        f.world.ship.module_slots[0] = 5;
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .set_credits(100);
        run(&mut f, &mut s, Handler::DoBuy, r);
        assert_eq!(f.world.ship.fuel_on_board, units * 100);
        assert_eq!(
            Dialogue {
                api: &mut f,
                session: &mut s
            }
            .credits(),
            100 - units as u16
        );
        assert_eq!(f.spoken(), [138, 136]);
        assert_eq!(
            f.offers().iter().map(|&(p, _)| p).collect::<Vec<_>>(),
            [137, 139, 140, 141, 198, 142]
        );
    }
    let mut f = Api::default();
    let mut s = Session::new();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .set_credits(4);
    run(&mut f, &mut s, Handler::DoBuy, 139);
    assert_eq!(f.world.ship.fuel_on_board, 0);
    assert_eq!(f.spoken(), [146, 147, 136]);
    assert_eq!(f.world.numbers, [1]);
    f.world.ship.fuel_on_board = 900;
    run(&mut f, &mut s, Handler::DoBuy, 139);
    assert_eq!(f.spoken(), [149, 136]);
    assert_eq!(
        Dialogue {
            api: &mut f,
            session: &mut s
        }
        .credits(),
        4
    );
    f.world.ship.module_slots = [6; 16];
    f.world.ship.fuel_on_board = 1;
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .set_credits(1610);
    run(&mut f, &mut s, Handler::DoBuy, 198);
    assert_eq!(f.world.ship.fuel_on_board, 161000);
    assert_eq!(
        Dialogue {
            api: &mut f,
            session: &mut s
        }
        .credits(),
        0
    );
    let deltas: Vec<_> = f
        .effects
        .iter()
        .filter_map(|e| {
            if let Effect::Gauges(0, fuel, 0) = e {
                Some(*fuel)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        deltas,
        [16383, 16383, 16383, 16383, 16383, 16383, 16383, 16383, 16383, 13553]
    );
    assert!(!f.offers().iter().any(|&(p, _)| p == 133));
}
#[test]
fn information_prices_exhaustion_and_knowledge_effects() {
    for (r, key, count, first) in [
        (152, "MELNORME_EVENTS_INFO_STACK", 8, 161),
        (153, "MELNORME_ALIEN_INFO_STACK", 16, 169),
        (154, "MELNORME_HISTORY_INFO_STACK", 9, 185),
    ] {
        let mut f = Api::default();
        let mut s = Session::new();
        f.put("MELNORME_INFO_PROCEDURE", 1);
        for i in 0..count {
            Dialogue {
                api: &mut f,
                session: &mut s,
            }
            .set_credits(75);
            run(&mut f, &mut s, Handler::DoBuy, r);
            assert_eq!(f.spoken(), [first + i as i32]);
            assert_eq!(f.get(key), i + 1);
            assert_eq!(
                Dialogue {
                    api: &mut f,
                    session: &mut s
                }
                .credits(),
                0
            );
        }
        if r == 153 {
            assert_eq!(f.get("KNOW_SPATHI_PASSWORD"), 1);
            assert_eq!(f.get("SPATHI_HOME_VISITS"), 7);
            assert_eq!(f.get("KNOW_ABOUT_SHATTERED"), 2);
            assert_eq!(f.get("KNOW_SYREEN_WORLD_SHATTERED"), 1);
        }
    }
    let mut f = Api::default();
    let mut s = Session::new();
    f.put("MELNORME_INFO_PROCEDURE", 1);
    f.put("MELNORME_EVENTS_INFO_STACK", 7);
    f.put("MELNORME_ALIEN_INFO_STACK", 16);
    f.put("MELNORME_HISTORY_INFO_STACK", 9);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .set_credits(75);
    run(&mut f, &mut s, Handler::DoBuy, 152);
    assert_eq!(f.spoken(), [168, 194]);
    assert!(!f.offers().iter().any(|&(p, _)| p == 150));
}
#[test]
fn biomass_sale_skip_and_incremental_paths_preserve_queue_order() {
    for skip in [false, true] {
        let mut f = Api::default();
        let mut s = Session::new();
        f.world.skip = skip;
        f.world.ship.total_bio_mass = 3;
        f.world.ship.fuel_on_board = 1;
        f.put("RAINBOW_WORLD0", 1);
        s.asked_to_buy = true;
        run(&mut f, &mut s, Handler::DoSell, 117);
        assert_eq!(f.world.ship.total_bio_mass, 0);
        assert_eq!(f.world.numbers, [3, 6]);
        assert_eq!(f.spoken(), [118, 119, 120, 115]);
        assert_eq!(
            &f.effects[..5],
            &[
                Effect::Npc(118),
                Effect::Npc(119),
                Effect::Npc(120),
                Effect::Npc(115),
                Effect::Talk(1)
            ]
        );
        assert_eq!(
            Dialogue {
                api: &mut f,
                session: &mut s
            }
            .credits(),
            6
        );
        assert!(!s.asked_to_buy);
        assert_eq!(
            f.offers(),
            [(121, Handler::DoSell), (128, Handler::NatureOfConversation)]
        );
    }
}
#[test]
fn first_meeting_order_names_business_and_status_restore() {
    let mut f = Api::default();
    let mut s = Session::new();
    let mut desc = descriptor();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .initialize(&mut desc);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .intro();
    assert_eq!(f.spoken(), [1]);
    assert_eq!(f.get("MET_MELNORME"), 1);
    assert_eq!(
        f.offers(),
        [
            (233, Handler::DoFirstMeeting),
            (7, Handler::DoFirstMeeting),
            (11, Handler::DoFirstMeeting),
            (20, Handler::ExitConversation)
        ]
    );
    for i in 0..4 {
        f.put("NEW_ALLIANCE_NAME", i);
        f.effects.clear();
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .yack0_respond();
        let expected = if i == 3 {
            vec![233, 250, GLOBAL_PLAYER_NAME, 251]
        } else {
            vec![233, 247 + i as i32]
        };
        assert_eq!(
            f.effects,
            [Effect::Offer(233, Handler::DoFirstMeeting, expected)]
        );
    }
    for (r, p, key, value, offers) in [
        (233, 4, "MELNORME_YACK_STACK0", 1, vec![5, 7, 11, 20]),
        (5, 6, "MELNORME_YACK_STACK0", 2, vec![7, 11, 20]),
        (7, 8, "MELNORME_YACK_STACK1", 1, vec![9, 17, 11, 20]),
        (9, 10, "MELNORME_YACK_STACK1", 2, vec![17, 11, 20]),
        (11, 12, "MELNORME_YACK_STACK2", 1, vec![13, 17, 20]),
        (13, 14, "MELNORME_YACK_STACK2", 2, vec![15, 17, 20]),
        (15, 16, "MELNORME_YACK_STACK2", 3, vec![18, 17, 20]),
    ] {
        run(&mut f, &mut s, Handler::DoFirstMeeting, r);
        assert_eq!(f.spoken(), [p]);
        assert_eq!(f.get(key), value);
        assert_eq!(
            f.offers().iter().map(|&(p, _)| p).collect::<Vec<_>>(),
            offers
        );
    }
    f.world.ship.fuel_on_board = 1;
    run(&mut f, &mut s, Handler::NatureOfConversation, 17);
    assert_eq!(f.spoken(), [22, 107, 108]);
    assert_eq!(f.get("MELNORME_YACK_STACK2"), 6);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .post_melnorme_enc();
    assert_eq!(f.world.status.last(), Some(&-1));
    assert_eq!(f.world.status[0], 3);
    assert_eq!(f.world.status[f.world.status.len() - 2], 1);
    assert_eq!(desc.alien_ambient_array[0].block_mask, 2);
    assert!(desc.alien_ambient_array[..4]
        .iter()
        .all(|a| a.anim_flags == 4));
    assert_eq!(desc.alien_text_baseline.x, 121);
    assert_eq!(desc.alien_text_width, 224);
    assert_eq!(desc.alien_text_valign, 0);
}
#[test]
fn anger_routes_bluster_apology_and_exits() {
    for (anger, first, key) in [
        (1, 33, "MELNORME_MIFFED_COUNT"),
        (2, 78, "MELNORME_PISSED_COUNT"),
        (3, 50, "MELNORME_HATE_COUNT"),
    ] {
        let mut f = Api::default();
        let mut s = Session::new();
        f.put("MET_MELNORME", 1);
        f.put("MELNORME_ANGER", anger);
        for n in 0..4 {
            f.effects.clear();
            Dialogue {
                api: &mut f,
                session: &mut s,
            }
            .intro();
            assert_eq!(f.spoken(), [first + n.min(2)]);
            assert_eq!(f.get(key), (n + 1).min(2) as u8);
        }
    }
    let mut f = Api::default();
    let mut s = Session::new();
    for (r, p, stack) in [(18, 19, 0), (95, 96, 1), (97, 98, 2), (99, 100, 3)] {
        run(&mut f, &mut s, Handler::DoBluster, r);
        assert_eq!(f.spoken(), [p]);
        assert_eq!(s.local_stack0, stack);
    }
    run(&mut f, &mut s, Handler::DoBluster, 101);
    assert_eq!(s.local_stack1, 1);
    assert!(f.offers().contains(&(103, Handler::DoFirstMeeting)));
    run(&mut f, &mut s, Handler::DoBluster, 93);
    assert_eq!(f.get("WHY_MELNORME_BLUE"), 1);
    for (h, r, p) in [
        (Handler::DoMelnormeMiffed, 36, 37),
        (Handler::DoMelnormeMiffed, 38, 39),
        (Handler::DoMelnormeMiffed, 40, 41),
        (Handler::DoMelnormeMiffed, 42, 43),
        (Handler::DoMelnormePissed, 81, 82),
        (Handler::DoMelnormePissed, 83, 84),
        (Handler::DoFirstMeeting, 103, 104),
        (Handler::DoFirstMeeting, 44, 45),
    ] {
        run(&mut f, &mut s, h, r);
        assert_eq!(f.spoken(), [p]);
    }
    for (r, p, anger) in [(105, 106, 1), (46, 47, 2), (91, 92, 3)] {
        run(&mut f, &mut s, Handler::ExitConversation, r);
        assert_eq!(f.spoken(), [p]);
        assert_eq!(f.get("MELNORME_ANGER"), anger);
        assert_eq!(f.segue, 1);
    }
    for (r, p) in [
        (20, 21),
        (48, 49),
        (89, 90),
        (53, 56),
        (54, 55),
        (73, 75),
        (68, 76),
        (64, 77),
        (135, 143),
        (109, 143),
    ] {
        run(&mut f, &mut s, Handler::ExitConversation, r);
        assert_eq!(f.spoken(), [p]);
        assert!(f.offers().is_empty());
    }
}
#[test]
fn rescue_offer_repeatability_protects_crew_and_cargo() {
    let mut f = Api::default();
    let mut s = Session::new();
    f.world.costs = [20; 20];
    f.world.ship.crew_enlisted = 51;
    f.world.ship.total_element_mass = 501;
    f.world.ship.module_slots = [3, 3, 4, 4, 5, 6, 10, 11, 12, 13, 7, 8, 9, 3, 4, 22];
    f.world.ship.num_landers = 2;
    f.world.ship.drive_slots = [1; 11];
    f.world.ship.jet_slots = [2; 8];
    assert_eq!(
        Dialogue {
            api: &mut f,
            session: &mut s
        }
        .ship_worth(),
        680
    );
    assert!(Dialogue {
        api: &mut f,
        session: &mut s
    }
    .strip_ship(900));
    let offer = f.spoken();
    let modules = s.sis_copy.module_slots;
    f.effects.clear();
    assert!(Dialogue {
        api: &mut f,
        session: &mut s
    }
    .strip_ship(900));
    assert_eq!(f.spoken(), offer);
    assert_eq!(s.sis_copy.module_slots, modules);
    assert!(s.sis_copy.module_slots.iter().filter(|&&m| m == 3).count() >= 2);
    assert!(s.sis_copy.module_slots.iter().filter(|&&m| m == 4).count() >= 2);
    assert_eq!(s.sis_copy.module_slots[4..6], [5, 6]);
    run(&mut f, &mut s, Handler::ExitConversation, 73);
    assert_eq!(f.get("MELNORME_RESCUE_REFUSED"), 1);
    run(&mut f, &mut s, Handler::ExitConversation, 72);
    assert_eq!(f.world.ship.module_slots, modules);
    assert_eq!(f.world.ship.fuel_on_board, 900);
    assert_eq!(f.get("MELNORME_RESCUE_REFUSED"), 0);
    run(&mut f, &mut s, Handler::ExitConversation, 85);
    assert_eq!(f.spoken(), [88]);
    assert_eq!(f.world.ship.drive_slots, [20; 11]);
    assert_eq!(f.world.ship.jet_slots, [21; 8]);
    assert_eq!(f.world.ship.num_landers, 0);
    assert_eq!(f.world.ship.total_element_mass, 0);
    assert_eq!(
        f.world
            .ship
            .module_slots
            .iter()
            .filter(|&&m| m == 3)
            .count(),
        2
    );
}
#[test]
fn charity_refusal_and_saturated_counters() {
    let mut f = Api::default();
    let mut s = Session::new();
    f.put("MET_MELNORME", 1);
    f.put("MELNORME_YACK_STACK2", 6);
    for n in 0..7 {
        f.world.ship.fuel_on_board = 0;
        f.effects.clear();
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .intro();
        assert_eq!(f.spoken()[1..], [57 + n.min(5), 66]);
        assert_eq!(f.get("MELNORME_RESCUE_COUNT"), (n + 1).min(5) as u8);
    }
    run(&mut f, &mut s, Handler::ExitConversation, 68);
    run(&mut f, &mut s, Handler::NatureOfConversation, 0);
    assert_eq!(f.spoken().last(), Some(&63));
    run(&mut f, &mut s, Handler::DoRescue, 65);
    assert_eq!(f.spoken(), [231]);
    assert!(f.offers().is_empty());
    assert_eq!(f.world.ship.fuel_on_board, 1000);
    run(&mut f, &mut s, Handler::ExitConversation, 85);
    assert_eq!(f.spoken(), [87]);
    for n in 0..12 {
        f.put("MELNORME_YACK_STACK2", 5 + n.min(9));
        run(&mut f, &mut s, Handler::NatureOfConversation, 0);
        assert_eq!(f.spoken()[0], 22 + i32::from(n.min(9)));
        assert_eq!(f.get("MELNORME_YACK_STACK2"), 5 + (n + 1).min(9));
    }
}
