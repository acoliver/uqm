use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::{ScriptApi, GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
type Api = Fake<Handler>;
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().into_iter().map(|(p, _)| p).collect()
}

#[test]
fn neutral_names_and_stack_order() {
    let mut f = Api::default();
    run(&mut f, Handler::NeutralSupox, 0);
    assert_eq!(choices(&f), [I_AM0, ANYONE_AROUND_HERE, BYE_NEUTRAL]);
    assert!(f.effects.contains(&Effect::Offer(
        I_AM0,
        Handler::NeutralSupox,
        vec![i32::from(I_AM0), GLOBAL_PLAYER_NAME, i32::from(I_AM1)]
    )));
    run(&mut f, Handler::NeutralSupox, I_AM0);
    assert_eq!(f.spoken(), [i32::from(WE_ARE_SUPOX)]);
    assert_eq!(f.get("SUPOX_STACK1"), 1);
    assert!(!f.enabled(I_AM0));
    assert!(f.effects.contains(&Effect::Offer(
        MY_SHIP0,
        Handler::NeutralSupox,
        vec![i32::from(MY_SHIP0), GLOBAL_SHIP_NAME, i32::from(MY_SHIP1)]
    )));
    for alliance in 0..4 {
        f.put("NEW_ALLIANCE_NAME", alliance);
        run(&mut f, Handler::NeutralSupox, MY_SHIP0);
        let mut want = vec![
            i32::from(FROM_ALLIANCE0),
            i32::from(NAME_1) + i32::from(alliance),
        ];
        if alliance == 3 {
            want.extend([GLOBAL_PLAYER_NAME, i32::from(NAME_1) + 4]);
        }
        want.push(i32::from(FROM_ALLIANCE1));
        assert!(f
            .effects
            .contains(&Effect::Offer(FROM_ALLIANCE0, Handler::NeutralSupox, want)));
    }
    for (r, p, stack) in [
        (FROM_ALLIANCE0, FROM_SUPOX, 3),
        (ARE_YOU_COPYING, YEAH_SORRY, 4),
        (WHY_COPY, SYMBIOTS, 5),
    ] {
        run(&mut f, Handler::NeutralSupox, r);
        assert_eq!(f.spoken(), [i32::from(p)]);
        assert_eq!(f.get("SUPOX_STACK1"), stack);
    }
    assert_eq!(
        choices(&f),
        [TELL_US_OF_YOUR_SPECIES, ANYONE_AROUND_HERE, BYE_NEUTRAL]
    );
    run(&mut f, Handler::NeutralSupox, TELL_US_OF_YOUR_SPECIES);
    assert_eq!(choices(&f)[0], PLANTS_ARENT_INTELLIGENT);
    run(&mut f, Handler::NeutralSupox, PLANTS_ARENT_INTELLIGENT);
    assert_eq!(f.get("SUPOX_STACK2"), 2);
    run(&mut f, Handler::NeutralSupox, ANYONE_AROUND_HERE);
    assert_eq!(choices(&f)[0], WHATS_WRONG_WITH_UTWIG);
    assert!(f.effects.contains(&Effect::Track(9)));
    run(&mut f, Handler::NeutralSupox, WHATS_WRONG_WITH_UTWIG);
    assert_eq!(choices(&f)[0], WHATS_ULTRON);
    run(&mut f, Handler::NeutralSupox, WHATS_ULTRON);
    assert_eq!(f.spoken(), [i32::from(TAKE_ULTRON)]);
    assert_eq!(f.get("ULTRON_CONDITION"), 1);
    assert_eq!(choices(&f), [WHAT_DO_I_DO_NOW, THANKS_NOW_WE_EAT_YOU]);
}
#[test]
fn ultron_repair_help_and_existing_utwig_contact() {
    for (condition, response, phrase) in [
        (1, WHERE_GET_REPAIRS, ANCIENT_RHYME),
        (2, LOOK_I_SLIGHTLY_REPAIRED, GREAT_DO_MORE),
        (3, LOOK_I_REPAIRED_LOTS, ALMOST_THERE),
        (4, GOT_FIXED_ULTRON, GOOD_GIVE_TO_UTWIG),
    ] {
        let mut f = Api::default();
        f.put("ULTRON_CONDITION", condition);
        run(&mut f, Handler::NeutralSupox, 0);
        assert!(choices(&f).contains(&response));
        run(&mut f, Handler::NeutralSupox, response);
        assert_eq!(f.spoken(), [i32::from(phrase)]);
        assert_eq!(f.get("SUPOX_ULTRON_HELP"), 1);
        assert!(!choices(&f).contains(&response));
    }
    for key in ["UTWIG_VISITS", "UTWIG_HOME_VISITS", "BOMB_VISITS"] {
        let mut f = Api::default();
        f.put(key, 1);
        run(&mut f, Handler::NeutralSupox, 0);
        assert_eq!(choices(&f)[1], WHAT_RELATION_TO_UTWIG);
    }
}
#[test]
fn war_news_preserves_write_order_and_mission_branches() {
    let mut f = Api::default();
    f.put("UTWIG_SUPOX_MISSION", 1);
    run(&mut f, Handler::AlliedHome, HOW_GOES_WAR);
    assert_eq!(f.spoken(), [i32::from(FLEET_ON_WAY)]);
    assert_eq!(f.get("SUPOX_WAR_NEWS"), 0);
    assert_eq!(
        &f.effects[..4],
        &[
            Effect::Npc(i32::from(FLEET_ON_WAY)),
            Effect::Set("SUPOX_WAR_NEWS".into(), 1),
            Effect::Disable(HOW_GOES_WAR),
            Effect::Set("SUPOX_WAR_NEWS".into(), 0)
        ]
    );
    for (news, phrase, next) in [(0, BATTLE_HAPPENS_1, 1), (1, BATTLE_HAPPENS_2, 2)] {
        f.put("UTWIG_SUPOX_MISSION", 3);
        f.put("SUPOX_WAR_NEWS", news);
        run(&mut f, Handler::AlliedHome, HOW_GOES_WAR);
        assert_eq!(f.spoken(), [i32::from(phrase)]);
        assert_eq!(f.get("SUPOX_WAR_NEWS"), next);
    }
    for mission in [0, 1, 4, 5] {
        let mut f = Api::default();
        f.put("UTWIG_SUPOX_MISSION", mission);
        run(&mut f, Handler::AlliedHome, LEARN_NEW_INFO);
        assert_eq!(
            f.spoken(),
            [i32::from(if mission < 5 { NO_NEW_INFO } else { SAMATRA })]
        );
        run(&mut f, Handler::AlliedHome, WHAT_NOW_HOMEWORLD);
        assert_eq!(
            f.spoken(),
            [i32::from(if mission < 5 {
                UP_TO_YOU
            } else {
                HOPE_KILL_EACH_OTHER
            })]
        );
        assert_eq!(choices(&f).contains(&CAN_YOU_HELP), mission == 0);
    }
}
#[test]
fn exits_escort_effects_and_insult() {
    for feasible in [0, 1] {
        let mut f = Api {
            feasibility: feasible,
            ..Api::default()
        };
        run(&mut f, Handler::ExitConversation, CAN_YOU_HELP);
        assert_eq!(
            f.spoken(),
            [
                i32::from(HOW_HELP),
                i32::from(if feasible == 0 {
                    DONT_NEED
                } else {
                    HAVE_4_SHIPS
                })
            ]
        );
        if feasible != 0 {
            assert_eq!(
                &f.effects[f.effects.len() - 2..],
                &[Effect::Talk(65535), Effect::AddEscorts(7, 4)]
            );
        }
    }
    for (r, p) in [
        (BYE_NEUTRAL, GOODBYE_NEUTRAL),
        (WHAT_DO_I_DO_NOW, FIX_IT),
        (BYE_AFTER_SPACE, GOODBYE_AFTER_SPACE),
        (BYE_BEFORE_SPACE, GOODBYE_BEFORE_SPACE),
        (BYE_ALLIED_HOMEWORLD, GOODBYE_ALLIED_HOMEWORLD),
    ] {
        let mut f = Api::default();
        run(&mut f, Handler::ExitConversation, r);
        assert_eq!(f.spoken(), [i32::from(p)]);
        assert_eq!(f.segue, 0);
    }
    let mut f = Api::default();
    f.put("SUPOX_VISITS", 3);
    f.put("SUPOX_HOME_VISITS", 3);
    run(&mut f, Handler::ExitConversation, THANKS_NOW_WE_EAT_YOU);
    assert_eq!(
        (
            f.get("SUPOX_HOSTILE"),
            f.get("SUPOX_VISITS"),
            f.get("SUPOX_HOME_VISITS")
        ),
        (1, 0, 0)
    );
    assert_eq!(f.segue, 0);
}
#[test]
fn encounter_routing_and_saturated_greetings() {
    for allied in [false, true] {
        for flags in [0, 128] {
            for mission in [0, 1, 2, 4, 5] {
                let mut f = Api::default();
                f.alliances[7] = if allied { 1 } else { 2 };
                f.put("GLOBAL_FLAGS_AND_DATA", flags);
                f.put("UTWIG_SUPOX_MISSION", mission);
                let key = if flags == 128 {
                    "SUPOX_HOME_VISITS"
                } else {
                    "SUPOX_VISITS"
                };
                for visit in 0..6 {
                    f.effects.clear();
                    Dialogue { api: &mut f }.intro();
                    assert!(!f.spoken().is_empty());
                    let cap = if flags == 128 && allied { 3 } else { 1 };
                    assert_eq!(f.get(key), (visit + 1).min(cap));
                }
                if allied && flags == 0 && (2..5).contains(&mission) {
                    assert!(f.offers().is_empty());
                }
            }
        }
    }
    let mut f = Api::default();
    f.put("SUPOX_HOSTILE", 1);
    let mut d = descriptor();
    Dialogue { api: &mut f }.initialize(&mut d);
    assert_eq!(f.segue, 1);
    for _ in 0..4 {
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert!(f.offers().is_empty());
    }
    f.activity = 5;
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    assert_eq!(
        (
            d.alien_text_baseline.x,
            d.alien_text_width,
            d.alien_text_valign
        ),
        (121, 224, 0)
    );
    assert_eq!(d.alien_ambient_array[0].anim_flags, 12);
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
            f.put("SUPOX_INFO", visit);
            run(&mut f, handler, question);
            assert_eq!(
                f.spoken(),
                [i32::from(if visit == 0 { first } else { second })]
            );
            assert_eq!(f.get("SUPOX_INFO"), 1);
            assert!(!f.enabled(question));
            assert_eq!(choices(&f), [now, bye]);
            run(&mut f, handler, now);
            assert_eq!(f.spoken(), [i32::from(answer)]);
            assert_eq!(choices(&f), [bye]);
        }
    }
}
