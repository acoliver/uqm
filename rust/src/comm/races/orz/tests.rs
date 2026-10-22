use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler>;
fn choices(f: &Api) -> Vec<u16> {
    f.offers().iter().map(|x| x.0).collect()
}
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
#[test]
fn taalo_permission_requires_alliance_and_polite_request() {
    for manner in [0, 1, 3] {
        for allied in [1, 2] {
            let mut f = Api::default();
            f.put("ORZ_MANNER", manner);
            f.alliances[3] = allied;
            run(&mut f, Handler::TaaloWorld, 0);
            assert_eq!(
                choices(&f),
                [
                    MAY_WE_LAND,
                    DEMAND_TO_LAND,
                    WHY_YOU_HERE,
                    WHAT_IS_THIS_PLACE,
                    BYE_TAALO
                ]
            );
            assert_eq!(
                f.offers()[0].1,
                if manner == 3 && allied == 1 {
                    Handler::ExitConversation
                } else {
                    Handler::TaaloWorld
                }
            );
            run(&mut f, Handler::TaaloWorld, MAY_WE_LAND);
            assert_eq!(f.get("TAALO_UNPROTECTED"), 0);
            assert_eq!(choices(&f)[0], MAKE_ALLIANCE);
        }
    }
    let mut f = Api::default();
    run(&mut f, Handler::ExitConversation, MAY_WE_LAND);
    assert_eq!(f.get("TAALO_UNPROTECTED"), 1);
    assert_eq!(f.segue, 0);
    run(&mut f, Handler::ExitConversation, DEMAND_TO_LAND);
    assert_eq!(f.segue, 1);
}
#[test]
fn alliance_apology_and_androsynth_rupture_effects() {
    for response in [YES_ALLIANCE, WERE_SORRY] {
        let mut f = Api::default();
        f.put("ORZ_ANDRO_STATE", 2);
        f.put("ORZ_PERSONAL_INFO", 7);
        run(&mut f, Handler::ExitConversation, response);
        assert_eq!(
            (
                f.get("ORZ_MANNER"),
                f.get("ORZ_ANDRO_STATE"),
                f.get("ORZ_GENERAL_INFO"),
                f.get("ORZ_PERSONAL_INFO")
            ),
            (3, 0, 0, 0)
        );
        assert!(f.effects.contains(&Effect::Allied(3, true)));
    }
    for response in [ABOUT_ANDRO_3, MUST_KNOW_ABOUT_ANDROSYN] {
        let mut f = Api::default();
        run(&mut f, Handler::ExitConversation, response);
        assert_eq!(f.get("ORZ_MANNER"), 2);
        assert_eq!(f.segue, 1);
        assert_eq!(
            f.effects.contains(&Effect::RemoveEscorts(3)),
            response == ABOUT_ANDRO_3
        );
        assert_eq!(f.effects.last(), Some(&Effect::FadePalette(1, 420)));
    }
}
#[test]
fn names_stack_order_and_computer_report() {
    let mut f = Api::default();
    run(&mut f, Handler::OrzNeutral, 0);
    assert_eq!(
        choices(&f),
        [WHO_YOU, WE_ARE_VINDICATOR0, HOSTILE_1, BYE_NEUTRAL]
    );
    assert!(f.effects.contains(&Effect::Offer(
        WE_ARE_VINDICATOR0,
        Handler::OrzNeutral,
        vec![
            i32::from(WE_ARE_VINDICATOR0),
            GLOBAL_PLAYER_NAME,
            i32::from(WE_ARE_VINDICATOR1),
            GLOBAL_SHIP_NAME,
            i32::from(WE_ARE_VINDICATOR2)
        ]
    )));
    run(&mut f, Handler::OrzNeutral, WE_ARE_VINDICATOR0);
    assert_eq!(choices(&f)[0], SEEM_LIKE_NICE_GUYS);
    run(&mut f, Handler::OrzNeutral, HOSTILE_1);
    assert_eq!(choices(&f)[0], HOSTILE_2);
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(
        &f.effects[f.effects.len() - 3..],
        &[
            Effect::Talking(false),
            Effect::Talk(1),
            Effect::Talking(true)
        ]
    );
    assert_eq!(f.get("MET_ORZ_BEFORE"), 1);
}
#[test]
fn insult_and_allied_information_counters_saturate() {
    let mut f = Api::default();
    for visit in 0..10 {
        run(&mut f, Handler::ExitConversation, INSULT_1);
        assert_eq!(f.get("ORZ_PERSONAL_INFO"), (visit + 1).min(7));
        assert_eq!(f.segue, u32::from(visit >= 2));
    }
    for (question, key, phrases) in [
        (
            WHATS_UP_ALLY,
            "ORZ_GENERAL_INFO",
            [
                GENERAL_INFO_ALLY_1,
                GENERAL_INFO_ALLY_2,
                GENERAL_INFO_ALLY_3,
                GENERAL_INFO_ALLY_4,
            ],
        ),
        (
            MORE_ABOUT_YOU,
            "ORZ_PERSONAL_INFO",
            [ABOUT_US_1, ABOUT_US_2, ABOUT_US_3, ABOUT_US_4],
        ),
    ] {
        let mut f = Api::default();
        for visit in 0..6 {
            run(&mut f, Handler::OrzAllied, question);
            assert_eq!(f.spoken(), [i32::from(phrases[visit.min(3)])]);
            assert_eq!(f.get(key), (visit + 1).min(3) as u8);
        }
    }
}
#[test]
fn greeting_routes_post_battle_and_outtakes() {
    for manner in [0, 1, 2, 3] {
        for flags in [0, 64, 128] {
            let mut f = Api::default();
            f.put("ORZ_MANNER", manner);
            f.put("GLOBAL_FLAGS_AND_DATA", flags);
            f.alliances[3] = 1;
            Dialogue { api: &mut f }.initialize();
            assert_eq!(f.segue, u32::from(manner != 3));
            Dialogue { api: &mut f }.intro();
            assert_eq!(f.offers().is_empty(), manner == 2);
            if manner == 2 {
                assert!(f.effects.contains(&Effect::Palette(1)));
            }
        }
    }
    let mut f = Api {
        segue: 1,
        ..Api::default()
    };
    f.put("TAALO_VISITS", 1);
    Dialogue { api: &mut f }.post_orz_enc();
    assert_eq!((f.get("ORZ_MANNER"), f.get("TAALO_VISITS")), (1, 0));
    f.activity = 5;
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    let d = descriptor();
    assert_eq!(d.num_animations, 12);
    assert_eq!(d.alien_ambient_array[12].start_index, 118);
}
