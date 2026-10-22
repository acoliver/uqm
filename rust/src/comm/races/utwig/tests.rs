use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler>;
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().into_iter().map(|(p, _)| p).collect()
}
#[test]
fn repaired_ultron_allies_both_races_and_advances_only_before_frenzy() {
    for frenzy in [0, 1] {
        let mut f = Api::default();
        f.put("ULTRON_CONDITION", 4);
        f.put("KOHR_AH_FRENZY", frenzy);
        let reset = [
            "UTWIG_VISITS",
            "SUPOX_VISITS",
            "UTWIG_HOME_VISITS",
            "SUPOX_HOME_VISITS",
            "BOMB_VISITS",
            "SUPOX_INFO",
            "UTWIG_INFO",
            "SUPOX_WAR_NEWS",
            "UTWIG_WAR_NEWS",
            "SUPOX_HOSTILE",
            "UTWIG_HOSTILE",
        ];
        for key in reset {
            f.put(key, 1);
        }
        run(&mut f, Handler::ExitConversation, GOT_ULTRON);
        assert_eq!(
            f.spoken(),
            [
                i32::from(DONT_WANT_TO_LOOK),
                i32::from(HAPPY_DAYS),
                i32::from(if frenzy == 0 {
                    OK_ATTACK_KOHRAH
                } else {
                    TOO_LATE
                })
            ]
        );
        assert_eq!(
            f.effects.contains(&Effect::Event(1, 0, 0, 0, 9)),
            frenzy == 0
        );
        assert_eq!(
            (f.get("UTWIG_HAVE_ULTRON"), f.get("ULTRON_CONDITION")),
            (1, 5)
        );
        for key in reset {
            assert_eq!(f.get(key), 0, "{key}");
        }
        assert_eq!(
            &f.effects[f.effects.len() - 2..],
            &[Effect::Allied(9, true), Effect::Allied(7, true)]
        );
        assert_eq!(f.segue, 0);
    }
}
#[test]
fn incomplete_ultron_and_bomb_refusal() {
    for response in [GOT_ULTRON, HEY_WAIT_GOT_ULTRON] {
        for info in [0, 1] {
            for condition in [1, 2, 3] {
                let mut f = Api::default();
                f.put("ULTRON_CONDITION", condition);
                f.put("UTWIG_INFO", info);
                run(&mut f, Handler::ExitConversation, response);
                let expected = if response == GOT_ULTRON {
                    if info == 0 {
                        SICK_TRICK_1
                    } else {
                        SICK_TRICK_2
                    }
                } else if info == 0 {
                    TRICKED_US_1
                } else {
                    TRICKED_US_2
                };
                assert_eq!(
                    f.spoken(),
                    [
                        i32::from(if response == GOT_ULTRON {
                            DONT_WANT_TO_LOOK
                        } else {
                            TAUNT_US_BUT_WE_LOOK
                        }),
                        i32::from(expected)
                    ]
                );
                assert_eq!(f.segue, u32::from(response == HEY_WAIT_GOT_ULTRON));
                assert_eq!(f.get("UTWIG_INFO"), 1);
            }
        }
    }
    let mut f = Api::default();
    f.put("GLOBAL_FLAGS_AND_DATA", 64);
    f.put("ULTRON_CONDITION", 4);
    run(&mut f, Handler::ExitConversation, GOT_ULTRON);
    assert_eq!(f.spoken(), [i32::from(NO_ULTRON_AT_BOMB)]);
    assert_eq!(f.get("REFUSED_ULTRON_AT_BOMB"), 1);
    assert_eq!(f.get("ULTRON_CONDITION"), 4);
    assert_eq!(f.alliances[9], 2);
}
#[test]
fn bomb_negotiation_and_combat_resets() {
    let mut f = Api::default();
    run(&mut f, Handler::BombWorld, 0);
    assert_eq!(choices(&f), [WHY_YOU_HERE, BYE_NEUTRAL]);
    run(&mut f, Handler::BombWorld, WHY_YOU_HERE);
    assert_eq!(f.spoken(), [i32::from(WE_GUARD_BOMB)]);
    assert_eq!(choices(&f), [WHAT_ABOUT_BOMB, BYE_NEUTRAL]);
    run(&mut f, Handler::BombWorld, WHAT_ABOUT_BOMB);
    assert_eq!(
        choices(&f),
        [
            GIVE_US_BOMB_OR_DIE,
            MAY_WE_HAVE_BOMB,
            WHATS_UP_BOMB,
            BYE_NEUTRAL
        ]
    );
    run(&mut f, Handler::BombWorld, MAY_WE_HAVE_BOMB);
    assert_eq!(choices(&f)[0], PLEASE);
    run(&mut f, Handler::BombWorld, PLEASE);
    assert_eq!(f.spoken(), [i32::from(SORRY_NO_BOMB)]);
    run(&mut f, Handler::BombWorld, WHATS_UP_BOMB);
    assert_eq!(f.spoken(), [i32::from(GENERAL_INFO_BOMB_1)]);
    assert_eq!(choices(&f).last(), Some(&BYE_BOMB));
    run(&mut f, Handler::BombWorld, WHATS_UP_BOMB);
    assert_eq!(f.spoken(), [i32::from(GENERAL_INFO_BOMB_2)]);
    run(&mut f, Handler::BombWorld, GIVE_US_BOMB_OR_DIE);
    assert_eq!(choices(&f)[0], DEMAND_BOMB);
    for key in [
        "UTWIG_INFO",
        "UTWIG_HOME_VISITS",
        "UTWIG_VISITS",
        "BOMB_VISITS",
    ] {
        f.put(key, 1);
    }
    run(&mut f, Handler::BombWorld, DEMAND_BOMB);
    assert_eq!(f.spoken(), [i32::from(GUARDS_FIGHT)]);
    assert_eq!(f.segue, 1);
    assert_eq!(f.get("UTWIG_HOSTILE"), 1);
    assert!(f.offers().is_empty());
    for key in [
        "UTWIG_INFO",
        "UTWIG_HOME_VISITS",
        "UTWIG_VISITS",
        "BOMB_VISITS",
    ] {
        assert_eq!(f.get(key), 0);
    }
}
#[test]
fn neutral_introduction_names_priority_and_insult() {
    for alliance in 0..4 {
        let mut f = Api::default();
        f.put("NEW_ALLIANCE_NAME", alliance);
        run(&mut f, Handler::NeutralUtwig, 0);
        let mut fragments = vec![
            i32::from(WE_ARE_VINDICATOR0),
            GLOBAL_PLAYER_NAME,
            i32::from(WE_ARE_VINDICATOR1),
            i32::from(NAME_1) + i32::from(alliance),
        ];
        if alliance == 3 {
            fragments.extend([GLOBAL_PLAYER_NAME, i32::from(NAME_1) + 4]);
        }
        fragments.push(i32::from(WE_ARE_VINDICATOR2));
        assert!(f.effects.contains(&Effect::Offer(
            WE_ARE_VINDICATOR0,
            Handler::NeutralUtwig,
            fragments
        )));
        assert_eq!(
            choices(&f),
            [WE_ARE_VINDICATOR0, WHAT_ABOUT_URQUAN_1, BYE_NEUTRAL]
        );
    }
    let mut f = Api::default();
    run(&mut f, Handler::NeutralUtwig, WE_ARE_VINDICATOR0);
    assert_eq!(f.get("UTWIG_STACK1"), 1);
    run(&mut f, Handler::NeutralUtwig, WHY_SAD);
    assert_eq!(f.get("UTWIG_STACK1"), 2);
    run(&mut f, Handler::NeutralUtwig, WHAT_ULTRON);
    assert_eq!(f.get("UTWIG_STACK1"), 3);
    assert_eq!(choices(&f)[..2], [DONT_BE_BABIES, REAL_SORRY_ABOUT_ULTRON]);
    f.put("UTWIG_VISITS", 1);
    f.put("UTWIG_HOME_VISITS", 1);
    f.put("UTWIG_INFO", 1);
    run(&mut f, Handler::NeutralUtwig, DONT_BE_BABIES);
    assert_eq!(f.get("UTWIG_HOSTILE"), 1);
    assert_eq!(f.segue, 1);
    assert!(f.offers().is_empty());
    for key in ["UTWIG_VISITS", "UTWIG_HOME_VISITS", "UTWIG_INFO"] {
        assert_eq!(f.get(key), 0);
    }
}
#[test]
fn mission_news_escort_and_introduction_dispatch() {
    let mut f = Api::default();
    f.put("UTWIG_SUPOX_MISSION", 1);
    run(&mut f, Handler::AlliedHome, HOW_GOES_WAR);
    assert_eq!(f.spoken(), [i32::from(FLEET_ON_WAY)]);
    assert_eq!(f.get("UTWIG_WAR_NEWS"), 0);
    for feasibility in [0, 1] {
        let mut f = Api {
            feasibility,
            ..Api::default()
        };
        run(&mut f, Handler::AlliedHome, 0);
        assert_eq!(choices(&f).contains(&CAN_YOU_HELP), feasibility != 0);
        run(&mut f, Handler::ExitConversation, CAN_YOU_HELP);
        if feasibility != 0 {
            assert_eq!(
                &f.effects[f.effects.len() - 2..],
                &[Effect::Talk(65535), Effect::AddEscorts(9, 4)]
            );
        }
    }
    for hostile in [0, 1] {
        for flags in [0, 64, 128] {
            for condition in [0, 1] {
                let mut f = Api::default();
                f.put("UTWIG_HOSTILE", hostile);
                f.put("GLOBAL_FLAGS_AND_DATA", flags);
                f.put("ULTRON_CONDITION", condition);
                for _ in 0..6 {
                    f.effects.clear();
                    Dialogue { api: &mut f }.intro();
                    assert!(!f.spoken().is_empty());
                }
                if hostile != 0 {
                    assert_eq!(
                        choices(&f),
                        if condition != 0 && flags != 64 {
                            vec![HEY_WAIT_GOT_ULTRON]
                        } else {
                            vec![]
                        }
                    );
                }
            }
        }
    }
    for mission in [1, 2, 4, 5] {
        let mut f = Api::default();
        f.alliances[9] = 1;
        f.put("UTWIG_SUPOX_MISSION", mission);
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.offers().is_empty(), (2..5).contains(&mission));
    }
    let mut f = Api::default();
    let mut d = descriptor();
    Dialogue { api: &mut f }.initialize(&mut d);
    assert_eq!(f.segue, 1);
    f.put("UTWIG_HAVE_ULTRON", 1);
    Dialogue { api: &mut f }.initialize(&mut d);
    assert_eq!(f.segue, 0);
    assert_eq!(d.alien_song_flags & 1, 1);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(d.alien_alt_song_res) },
        c"comm.utwig.ultron.music"
    );
    f.put("UTWIG_HAVE_ULTRON", 0);
    Dialogue { api: &mut f }.initialize(&mut d);
    assert_eq!(d.alien_song_flags & 1, 0);
    f.activity = 5;
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    assert_eq!(
        (
            d.num_animations,
            d.alien_text_baseline.x,
            d.alien_text_width,
            d.alien_text_valign
        ),
        (16, 121, 224, 1)
    );
}

#[test]
fn before_and_after_war_information_saturates_and_disables_each_question() {
    for (handler, question, first, second, now, answer, bye) in [
        (
            Handler::BeforeKohrAh,
            WHATS_UP_BEFORE_SPACE,
            GENERAL_INFO_BEFORE_SPACE_1,
            GENERAL_INFO_BEFORE_SPACE_2,
            WHAT_NOW_BEFORE_SPACE,
            DO_THIS_BEFORE_SPACE,
            BYE_BEFORE_SPACE,
        ),
        (
            Handler::AfterKohrAh,
            WHATS_UP_AFTER_SPACE,
            GENERAL_INFO_AFTER_SPACE_1,
            GENERAL_INFO_AFTER_SPACE_2,
            WHAT_NOW_AFTER_SPACE,
            DO_THIS_AFTER_SPACE,
            BYE_AFTER_SPACE,
        ),
    ] {
        for visit in [0, 1] {
            let mut f = Api::default();
            f.put("UTWIG_INFO", visit);
            run(&mut f, handler, question);
            assert_eq!(
                f.spoken(),
                [i32::from(if visit == 0 { first } else { second })]
            );
            assert_eq!(f.get("UTWIG_INFO"), 1);
            assert!(!f.enabled(question));
            assert_eq!(choices(&f), [now, bye]);
            run(&mut f, handler, now);
            assert_eq!(f.spoken(), [i32::from(answer)]);
            assert_eq!(choices(&f), [bye]);
        }
    }
}

#[test]
fn war_news_sequence_and_after_war_information() {
    for (news, phrase, next) in [(0, BATTLE_HAPPENS_1, 1), (1, BATTLE_HAPPENS_2, 2)] {
        let mut f = Api::default();
        f.put("UTWIG_SUPOX_MISSION", 3);
        f.put("UTWIG_WAR_NEWS", news);
        run(&mut f, Handler::AlliedHome, HOW_GOES_WAR);
        assert_eq!(f.spoken(), [i32::from(phrase)]);
        assert_eq!(f.get("UTWIG_WAR_NEWS"), next);
    }
    let mut f = Api::default();
    f.put("UTWIG_SUPOX_MISSION", 5);
    run(&mut f, Handler::AlliedHome, HOW_WENT_WAR);
    assert_eq!(f.spoken(), [i32::from(ABOUT_BATTLE)]);
    assert_eq!(f.get("UTWIG_WAR_NEWS"), 1);
    run(&mut f, Handler::AlliedHome, LEARN_NEW_INFO);
    assert_eq!(f.spoken(), [i32::from(SAMATRA)]);
    assert_eq!(f.get("UTWIG_WAR_NEWS"), 3);
    run(&mut f, Handler::AlliedHome, WHAT_NOW_HOMEWORLD);
    assert_eq!(f.spoken(), [i32::from(HOPE_KILL_EACH_OTHER)]);
    run(&mut f, Handler::AlliedHome, HOW_IS_ULTRON);
    assert_eq!(f.spoken(), [i32::from(ULTRON_IS_GREAT)]);
    assert_eq!(choices(&f), [BYE_ALLIED_HOMEWORLD]);
}
