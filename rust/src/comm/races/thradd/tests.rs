use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler>;
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().iter().map(|x| x.0).collect()
}
#[test]
fn helix_permission_and_thief_hostility() {
    let mut f = Api::default();
    f.put("GLOBAL_FLAGS_AND_DATA", 64);
    run(&mut f, Handler::ThraddAllies, WHY_YOU_HERE_ALLY);
    assert_eq!(
        choices(&f),
        [
            WHATS_HELIX_ALLY,
            MAY_I_LAND,
            WHATS_UP_ALLY,
            HOW_GOES_CULTURE,
            BYE_ALLY
        ]
    );
    run(&mut f, Handler::ExitConversation, MAY_I_LAND);
    assert_eq!(f.get("HELIX_UNPROTECTED"), 1);
    assert_eq!(f.segue, 0);
    f.put("AQUA_HELIX", 1);
    f.put("THRADD_MANNER", 1);
    for phrase in [DIE_THIEF_1, DIE_THIEF_2, DIE_THIEF_2] {
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [i32::from(phrase)]);
        assert_eq!(f.segue, 1);
        assert!(f.offers().is_empty());
    }
}
#[test]
fn culture_name_and_demeanor_sequence() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.thradd_worship(0);
    assert_eq!(f.get("THRADD_MANNER"), 1);
    assert!(f.effects.contains(&Effect::Allied(8, true)));
    assert_eq!(
        choices(&f),
        [BE_POLITE, SPEAK_PIG_LATIN, USE_RHYMES, JUST_THE_WAY_WE_DO]
    );
    for (r, intro) in [
        (BE_POLITE, 0),
        (USE_RHYMES, 1),
        (SPEAK_PIG_LATIN, 2),
        (JUST_THE_WAY_WE_DO, 3),
    ] {
        run(&mut f, Handler::ThraddCulture, r);
        assert_eq!(f.get("THRADD_INTRO"), intro);
        assert_eq!(choices(&f), [YOU_DECIDE, FAT, THE_SLAVE_EMPIRE0]);
        assert!(f.effects.contains(&Effect::Offer(
            THE_SLAVE_EMPIRE0,
            Handler::ThraddDemeanor,
            vec![
                i32::from(THE_SLAVE_EMPIRE0),
                GLOBAL_PLAYER_NAME,
                i32::from(THE_SLAVE_EMPIRE1)
            ]
        )));
    }
    for (r, culture, name) in [
        (YOU_DECIDE, 1, CULTURE),
        (FAT, 2, FAT_JERKS),
        (THE_SLAVE_EMPIRE0, 3, SLAVE_EMPIRE),
    ] {
        run(&mut f, Handler::ThraddDemeanor, r);
        assert_eq!(f.get("THRADD_CULTURE"), culture);
        assert_eq!(Dialogue { api: &mut f }.get_culture_name(), i32::from(name));
    }
    for (r, value) in [
        (CONTEMPLATIVE, 0),
        (FRIENDLY, 1),
        (WACKY, 2),
        (JUST_LIKE_US, 3),
    ] {
        run(&mut f, Handler::ExitConversation, r);
        assert_eq!(f.get("THRADD_DEMEANOR"), value);
        assert_eq!(f.segue, 0);
        run(&mut f, Handler::ThraddAllies, HOW_GOES_CULTURE);
        assert_eq!(f.get("THRADD_DEMEANOR"), value | 4);
    }
}
#[test]
fn mission_departure_and_reports() {
    let mut f = Api::default();
    run(&mut f, Handler::ExitConversation, GOT_IDEA);
    assert!(f.effects.contains(&Effect::Event(1, 0, 0, 0, 5)));
    assert_eq!(f.get("THRADD_STACK_1"), 5);
    assert_eq!(f.segue, 0);
    for mission in [1, 2, 3] {
        let mut f = Api::default();
        f.put("THRADD_MISSION", mission);
        for phrase in match mission {
            1 => [WE_GO_TO_IMPRESS_URQUAN_1, WE_GO_TO_IMPRESS_URQUAN_2],
            2 => [WE_IMPRESSING_URQUAN_1, WE_IMPRESSING_URQUAN_2],
            _ => [WE_IMPRESSED_URQUAN_1, WE_IMPRESSED_URQUAN_2],
        }
        .iter()
        {
            f.effects.clear();
            Dialogue { api: &mut f }.intro();
            assert_eq!(f.spoken(), [i32::from(*phrase)]);
            assert_eq!(f.get("THRADD_MISSION_VISITS"), 1);
        }
    }
    f.put("THRADD_MISSION", 4);
    run(&mut f, Handler::ThraddHostile, HOW_IMPRESSED_URQUAN_1);
    assert!(choices(&f).contains(&HOW_IMPRESSED_URQUAN_2));
    run(&mut f, Handler::ThraddHostile, HOW_IMPRESSED_URQUAN_2);
    assert_eq!(f.get("THRADD_MISSION"), 5);
}
#[test]
fn body_count_threshold_and_frenzy() {
    for (count, level, phrase, next) in [
        (8, 0, ADEQUATE_PERFORMANCE, 1),
        (16, 1, IMPRESSIVE_PERFORMANCE, 2),
        (25, 0, AMAZING_PERFORMANCE, 0),
    ] {
        let mut f = Api::default();
        f.put("THRADDASH_BODY_COUNT", count);
        f.put("THRADD_BODY_LEVEL", level);
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken()[0], i32::from(phrase));
        assert_eq!(f.get("THRADD_BODY_LEVEL"), next);
        assert_eq!(f.get("THRADD_MANNER"), u8::from(count == 25));
    }
    let mut f = Api::default();
    f.put("THRADD_STACK_1", 4);
    f.put("KOHR_AH_FRENZY", 1);
    run(&mut f, Handler::ThraddHostile, 0);
    assert_eq!(f.get("THRADD_STACK_1"), 5);
    assert!(!choices(&f).contains(&GOT_IDEA));
    run(&mut f, Handler::HelixWorld, WHY_YOU_HERE_HOSTILE);
    assert!(choices(&f).contains(&DEMAND_TO_LAND));
}
#[test]
fn allied_greetings_names_and_outtakes() {
    for (intro, phrase) in [
        (0, HELLO_POLITE_1),
        (1, HELLO_RHYME_1),
        (2, HELLO_PIG_LATIN_1),
        (3, HELLO_LIKE_YOU_1),
    ] {
        for (flags, welcome0, welcome1) in [
            (0, WELCOME_SPACE0, WELCOME_SPACE1),
            (64, WELCOME_HELIX0, WELCOME_HELIX1),
            (128, WELCOME_HOMEWORLD0, WELCOME_HOMEWORLD1),
        ] {
            let mut f = Api::default();
            f.put("THRADD_MANNER", 1);
            f.put("THRADD_CULTURE", 1);
            f.put("THRADD_INTRO", intro);
            f.put("GLOBAL_FLAGS_AND_DATA", flags);
            Dialogue { api: &mut f }.initialize();
            assert_eq!(f.segue, 0);
            Dialogue { api: &mut f }.intro();
            assert_eq!(
                f.spoken(),
                [phrase, welcome0, CULTURE, welcome1].map(i32::from)
            );
        }
    }
    let mut f = Api {
        activity: 5,
        ..Api::default()
    };
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    Dialogue { api: &mut f }.post_thradd_enc();
    assert_eq!(descriptor().alien_ambient_array[4].block_mask, 41);
}
