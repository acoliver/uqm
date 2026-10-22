use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler, api::World>;
fn run(f: &mut Api, h: Handler, r: i32) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
fn choices(f: &Api) -> Vec<i32> {
    f.offers().iter().map(|x| i32::from(x.0)).collect()
}
#[test]
fn caster_and_pet_information_order_and_biomass_reward_are_preserved() {
    let mut f = Api::default();
    run(&mut f, Handler::UmgahReward, 0);
    assert_eq!(
        choices(&f),
        [
            WHAT_BEFORE_TPET,
            OWE_ME_BIG_TIME,
            OUR_LARGESSE,
            WHAT_DO_WITH_TPET,
            SO_WHAT_FOR_NOW,
            BYE_POST_ZOMBIE
        ]
    );
    run(&mut f, Handler::UmgahReward, WHAT_BEFORE_TPET);
    assert_eq!(f.spoken(), [TRKD_SPATHI_AND_ILWRATH]);
    assert_eq!(choices(&f)[0], WHERE_CASTER);
    run(&mut f, Handler::UmgahReward, WHERE_CASTER);
    assert_eq!(f.spoken(), [SPATHI_TOOK_THEM]);
    assert!(!choices(&f).contains(&WHERE_CASTER));
    for (r, phrase) in [(OWE_ME_BIG_TIME, THANKS), (OUR_LARGESSE, GIVE_LIFEDATA)] {
        let mut f = Api::default();
        f.world.biomass = u16::MAX - 100;
        run(&mut f, Handler::UmgahReward, r);
        assert_eq!(f.spoken(), [phrase]);
        assert_eq!(f.world.biomass, 399);
        assert!(f.disabled.contains(&(OWE_ME_BIG_TIME as u16)));
        assert!(f.disabled.contains(&(OUR_LARGESSE as u16)));
    }
    run(&mut f, Handler::UmgahReward, WHAT_DO_WITH_TPET);
    assert!(choices(&f).contains(&ANY_JOKES));
    f.put("UMGAH_MENTIONED_TRICKS", 2);
    run(&mut f, Handler::UmgahReward, 0);
    assert!(!choices(&f).contains(&ANY_JOKES));
}
#[test]
fn zombie_revelation_and_reward_exit_keep_combat_and_escort_effects() {
    let mut f = Api::default();
    f.put("MET_NORMAL_UMGAH", 1);
    run(&mut f, Handler::Zombies, HOW_GOES_TPET);
    assert_eq!(
        choices(&f),
        [YOU_TOLD_US, WHATS_UP_ZOMBIES, THREAT, BYE_UNKNOWN]
    );
    run(&mut f, Handler::Zombies, YOU_TOLD_US);
    assert_eq!(
        choices(&f),
        [DONT_BELIEVE, WHATS_UP_ZOMBIES, THREAT, BYE_UNKNOWN]
    );
    f.put("UMGAH_VISITS", 3);
    run(&mut f, Handler::CombatIsInevitable, DONT_BELIEVE);
    assert_eq!(f.get("KNOW_UMGAH_ZOMBIES"), 1);
    assert_eq!(f.get("UMGAH_VISITS"), 0);
    assert_eq!(f.segue, 1);
    run(&mut f, Handler::CombatIsInevitable, BYE_UNKNOWN);
    assert_eq!(f.segue, 0);
    run(&mut f, Handler::CombatIsInevitable, BYE_POST_ZOMBIE);
    assert_eq!(f.segue, 1);
    assert_eq!(f.get("UMGAH_HOSTILE"), 1);
    assert_eq!(
        f.effects,
        [
            Effect::Segue(1),
            Effect::Npc(FUNNY_IDEA),
            Effect::Talk(u16::MAX),
            Effect::AddEscorts(crate::comm::dispatch::ship::UMGAH_SHIP as u16, 4),
            Effect::Set("UMGAH_HOSTILE".into(), 1)
        ]
    );
}
#[test]
fn introductions_follow_hostility_zombie_pet_and_home_priority() {
    for (keys, phrase, hostile) in [
        (vec![("UMGAH_HOSTILE", 1)], HOSTILE_HELLO_1, true),
        (
            vec![("UMGAH_ZOMBIE_BLOBBIES", 1), ("TALKING_PET_VISITS", 1)],
            DESTROY_INTERFERER_1,
            true,
        ),
        (
            vec![("UMGAH_ZOMBIE_BLOBBIES", 1), ("KNOW_UMGAH_ZOMBIES", 1)],
            REVEALED_ZOMBIE_HELLO_1,
            true,
        ),
        (
            vec![("UMGAH_ZOMBIE_BLOBBIES", 1)],
            UNKNOWN_ZOMBIE_HELLO_1,
            false,
        ),
        (
            vec![("GLOBAL_FLAGS_AND_DATA", 128)],
            HWLD_PRE_ZOMBIE_HELLO_1,
            false,
        ),
        (vec![], SPACE_PRE_ZOMBIE_HELLO_1, false),
        (
            vec![("TALKING_PET", 1), ("GLOBAL_FLAGS_AND_DATA", 128)],
            POST_ZOMBIE_HWLD_HELLO,
            false,
        ),
        (vec![("TALKING_PET", 1)], REWARD_AT_HOMEWORLD_1, false),
    ] {
        let mut f = Api::default();
        for (k, v) in keys {
            f.put(k, v);
        }
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [phrase]);
        if hostile {
            assert_eq!(f.segue, 1);
            assert!(choices(&f).is_empty());
        }
    }
}
#[test]
fn post_encounter_marks_only_uncontrolled_umgah_and_initialize_respects_outtakes() {
    for zombie in [0, 1] {
        let mut f = Api::default();
        f.put("UMGAH_ZOMBIE_BLOBBIES", zombie);
        Dialogue { api: &mut f }.post_umgah_enc();
        assert_eq!(f.get("MET_NORMAL_UMGAH"), 1 - zombie);
    }
    for (pet, hostile, activity, segue) in [(0, 0, 0, 1), (1, 0, 0, 0), (1, 1, 0, 1), (0, 1, 5, 0)]
    {
        let mut f = Api {
            activity,
            ..Api::default()
        };
        f.put("TALKING_PET", pet);
        f.put("UMGAH_HOSTILE", hostile);
        Dialogue { api: &mut f }.initialize();
        assert_eq!(f.segue, segue);
    }
    assert_eq!(descriptor().num_animations, 16);
}
