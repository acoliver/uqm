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
fn first_contact_and_quasispace_routing() {
    for side in 0..4 {
        let mut f = Api::default();
        f.put("ARILOU_SPACE_SIDE", side);
        Dialogue { api: &mut f }.initialize();
        assert_eq!(f.segue, u32::from(side <= 1));
        Dialogue { api: &mut f }.intro();
        assert_eq!(
            f.spoken(),
            [i32::from(if side <= 1 {
                INIT_HELLO
            } else {
                FRDLY_HOMEWORLD_HELLO_1
            })]
        );
        assert_eq!(
            choices(&f),
            [CONFUSED_BY_HELLO, HAPPY_BY_HELLO, MIFFED_BY_HELLO]
        );
        assert!(f.offers().iter().all(|x| x.1
            == if side <= 1 {
                Handler::FriendlySpaceArilou
            } else {
                Handler::ArilouHome
            }));
        assert_eq!(f.get("MET_ARILOU"), 1);
        assert_eq!(f.get("ARILOU_HOME_VISITS"), u8::from(side > 1));
    }
}
#[test]
fn portal_exchange_and_response_stack_order() {
    let mut f = Api::default();
    run(&mut f, Handler::ArilouHome, WHAT_GIVE_ME);
    assert_eq!(f.spoken(), [i32::from(ABOUT_PORTAL)]);
    assert_eq!(f.get("KNOW_ARILOU_WANT_WRECK"), 1);
    assert_eq!(choices(&f)[0], WHAT_ABOUT_TPET);
    assert!(f.enabled(ABOUT_PORTAL_AGAIN));
    f.put("PORTAL_KEY_ON_SHIP", 1);
    f.put("ARILOU_HOME_VISITS", 1);
    run(&mut f, Handler::ArilouHome, GOT_IT);
    assert_eq!(
        f.spoken(),
        [i32::from(CLEVER_HUMAN), i32::from(GIVE_PORTAL)]
    );
    assert_eq!(
        (
            f.get("PORTAL_KEY_ON_SHIP"),
            f.get("PORTAL_SPAWNER"),
            f.get("PORTAL_SPAWNER_ON_SHIP")
        ),
        (0, 1, 1)
    );
    assert!(!choices(&f).contains(&ABOUT_PORTAL_AGAIN));
    run(&mut f, Handler::ArilouHome, WHAT_DID_ON_EARTH);
    assert_eq!(choices(&f)[0], WHY_DID_THIS);
}
#[test]
fn umgah_inquiry_and_talking_pet_branches() {
    let mut f = Api::default();
    f.put("KNOW_UMGAH_ZOMBIES", 1);
    run(&mut f, Handler::ArilouHome, UMGAH_ACTING_WEIRD);
    assert!(f.effects.contains(&Effect::Event(1, 0, 10, 0, 14)));
    assert_eq!(f.get("ARILOU_CHECKED_UMGAH"), 1);
    assert!(!choices(&f).contains(&LEARNED_ABOUT_UMGAH));
    f.disabled.clear();
    f.put("ARILOU_CHECKED_UMGAH", 2);
    run(&mut f, Handler::ArilouHome, LEARNED_ABOUT_UMGAH);
    assert_eq!(f.spoken(), [i32::from(UMGAH_UNDER_COMPULSION)]);
    assert_eq!(choices(&f)[0], WHAT_DO_NOW);
    run(&mut f, Handler::ArilouHome, WHAT_DO_NOW);
    assert_eq!(f.get("ARILOU_CHECKED_UMGAH"), 3);
    f.put("TALKING_PET_VISITS", 1);
    run(&mut f, Handler::ArilouHome, TELL_ARILOU_ABOUT_TPET);
    assert_eq!(choices(&f)[0], WHAT_DO_ABOUT_TPET);
}
#[test]
fn hint_counters_saturate_and_preserve_story_flags() {
    for (question, key, phrases) in [
        (
            WHATS_UP_1,
            "ARILOU_INFO",
            [
                GENERAL_INFO_1,
                GENERAL_INFO_2,
                GENERAL_INFO_3,
                GENERAL_INFO_4,
            ],
        ),
        (
            GIVE_ME_INFO_1,
            "ARILOU_HINTS",
            [
                ARILOU_HINTS_1,
                ARILOU_HINTS_2,
                ARILOU_HINTS_3,
                ARILOU_HINTS_4,
            ],
        ),
    ] {
        let mut f = Api::default();
        for visit in 0..6 {
            run(&mut f, Handler::FriendlySpaceArilou, question);
            assert_eq!(f.spoken(), [i32::from(phrases[visit.min(3)])]);
            assert_eq!(f.get(key), (visit + 1).min(3) as u8);
        }
        if key == "ARILOU_HINTS" {
            assert_eq!(
                (
                    f.get("KNOW_ABOUT_SHATTERED"),
                    f.get("KNOW_URQUAN_STORY"),
                    f.get("KNOW_KOHR_AH_STORY")
                ),
                (2, 1, 1)
            );
        }
    }
}
#[test]
fn post_encounter_manner_and_first_home_visit() {
    let mut f = Api {
        segue: 1,
        ..Api::default()
    };
    f.put("ARILOU_VISITS", 3);
    Dialogue { api: &mut f }.post_arilou_enc();
    assert_eq!((f.get("ARILOU_MANNER"), f.get("ARILOU_VISITS")), (1, 0));
    f.put("ARILOU_SPACE_SIDE", 2);
    Dialogue { api: &mut f }.post_arilou_enc();
    assert_eq!(
        (f.get("ARILOU_MANNER"), f.get("UMGAH_ZOMBIE_BLOBBIES")),
        (3, 1)
    );
    run(&mut f, Handler::ExitConversation, BUG_EYED_FRUITCAKES);
    assert_eq!(f.get("ARILOU_MANNER"), 2);
    f.activity = 5;
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [i32::from(OUT_TAKES)]);
    assert_eq!(f.segue, 0);
    let d = descriptor();
    assert_eq!(d.num_animations, 20);
    assert_eq!(d.alien_ambient_array[0].block_mask, 65538);
}
