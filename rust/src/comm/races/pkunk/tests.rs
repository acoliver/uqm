use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler, api::World>;
fn choices(f: &Api) -> Vec<u16> {
    f.offers().iter().map(|x| x.0).collect()
}
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
#[test]
fn fury_gift_dates_capacity_and_due_boundary() {
    for added in [0, 1, 4] {
        let mut f = Api::default();
        f.world.date = (2155, 12, 31);
        f.world.ships_added = added;
        Dialogue { api: &mut f }.prepare_ship();
        assert_eq!(
            (
                f.get("PKUNK_SHIP_YEAR"),
                f.get("PKUNK_SHIP_MONTH"),
                f.get("PKUNK_SHIP_DAY")
            ),
            if added != 0 { (1, 12, 28) } else { (0, 0, 0) }
        );
    }
    let mut f = Api::default();
    f.put("PKUNK_MANNER", 3);
    f.put("PKUNK_SHIP_YEAR", 1);
    f.put("PKUNK_SHIP_MONTH", 6);
    f.put("PKUNK_SHIP_DAY", 15);
    for (date, ready) in [
        ((2155, 12, 31), false),
        ((2156, 5, 31), false),
        ((2156, 6, 14), false),
        ((2156, 6, 15), true),
        ((2156, 6, 16), true),
        ((2157, 1, 1), true),
    ] {
        f.world.date = date;
        assert_eq!(Dialogue { api: &mut f }.ships_ready(), ready);
    }
    for room in [0, 1] {
        let mut f = Api {
            feasibility: room,
            ..Api::default()
        };
        f.world.date = (2155, 2, 15);
        f.world.ships_added = 4;
        run(&mut f, Handler::ExitConversation, WE_NEED_HELP);
        assert_eq!(f.get("PKUNK_MANNER"), 3);
        assert!(f.effects.contains(&Effect::Event(1, 6, 0, 0, 4)));
        assert_eq!(f.effects.contains(&Effect::Talk(65535)), room != 0);
        assert_eq!(f.get("PKUNK_SHIP_MONTH"), if room == 0 { 0 } else { 2 });
    }
}
#[test]
fn spindle_gift_home_and_reconciliation() {
    for reconcile in [false, true] {
        let mut f = Api::default();
        f.put("GLOBAL_FLAGS_AND_DATA", 128);
        if reconcile {
            run(&mut f, Handler::PkunkHome, TRY_TO_BE_NICER);
        } else {
            Dialogue { api: &mut f }.intro();
        }
        assert_eq!(
            (f.get("CLEAR_SPINDLE"), f.get("CLEAR_SPINDLE_ON_SHIP")),
            (1, 1)
        );
        assert!(f.spoken().contains(&i32::from(GIVE_SPINDLE)));
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert!(!f.spoken().contains(&i32::from(GIVE_SPINDLE)));
    }
}
#[test]
fn migration_reasons_and_response_order() {
    let mut f = Api::default();
    run(&mut f, Handler::PkunkMigrate, 0);
    assert_eq!(
        choices(&f),
        [
            GOOD_REASON_1,
            BAD_REASON_1,
            GOOD_REASON_2,
            BAD_REASON_2,
            SUIT_YOURSELF
        ]
    );
    for (question, mask, event) in [
        (GOOD_REASON_1, 1, true),
        (BAD_REASON_1, 4, false),
        (GOOD_REASON_2, 2, true),
        (BAD_REASON_2, 8, false),
    ] {
        run(&mut f, Handler::ExitConversation, question);
        assert_eq!(f.effects.contains(&Effect::Event(1, 0, 0, 0, 4)), event);
        assert_ne!(f.get("PKUNK_REASONS") & mask, 0);
    }
    run(&mut f, Handler::PkunkMigrate, 0);
    assert_eq!(choices(&f), [SUIT_YOURSELF]);
}
#[test]
fn home_name_response_and_war_information() {
    let mut f = Api::default();
    run(&mut f, Handler::PkunkHome, 0);
    assert!(f.effects.contains(&Effect::Offer(
        WE_ARE_VINDICATOR0,
        Handler::OfferAlliance,
        vec![
            i32::from(WE_ARE_VINDICATOR0),
            GLOBAL_PLAYER_NAME,
            i32::from(WE_ARE_VINDICATOR1),
            GLOBAL_SHIP_NAME,
            i32::from(WE_ARE_VINDICATOR2)
        ]
    )));
    for (visit, phrase) in [WAR_GOES_1, WAR_GOES_2, WAR_GOES_3, WAR_GOES_4, WAR_GOES_4]
        .iter()
        .enumerate()
    {
        run(&mut f, Handler::PkunkFriendlySpace, HOW_GOES_WAR);
        assert_eq!(f.spoken(), [i32::from(*phrase)]);
        assert_eq!(f.get("PKUNK_WAR"), (visit + 1).min(3) as u8);
    }
    assert_eq!(
        (
            f.get("KNOW_URQUAN_STORY"),
            f.get("KNOW_KOHR_AH_STORY"),
            f.get("PKUNK_DONE_WAR")
        ),
        (1, 1, 1)
    );
    assert!(!choices(&f).contains(&HOW_GOES_WAR));
}
#[test]
fn greeting_routes_hostility_and_outtakes() {
    for manner in [0, 1, 2, 3] {
        for flags in [0, 128] {
            for mission in [0, 1, 2] {
                let mut f = Api::default();
                f.put("PKUNK_MANNER", manner);
                f.put("GLOBAL_FLAGS_AND_DATA", flags);
                f.put("PKUNK_MISSION", mission);
                f.world.date = (2155, 1, 1);
                Dialogue { api: &mut f }.initialize();
                assert_eq!(f.segue, u32::from(manner != 3));
                Dialogue { api: &mut f }.intro();
                assert_eq!(f.offers().is_empty(), manner == 2);
            }
        }
    }
    let mut f = Api {
        segue: 1,
        ..Api::default()
    };
    f.put("PKUNK_HOME_VISITS", 3);
    Dialogue { api: &mut f }.post_pkunk_enc();
    assert_eq!((f.get("PKUNK_MANNER"), f.get("PKUNK_HOME_VISITS")), (1, 0));
    f.effects.clear();
    f.activity = 5;
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    assert_eq!(descriptor().alien_talk_desc.num_frames, 2);
}
