use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler, api::World>;
fn run(f: &mut Api, s: &mut Session, h: Handler, r: i32) {
    f.effects.clear();
    Dialogue { api: f, session: s }.handle(h, r);
}
fn choices(f: &Api) -> Vec<i32> {
    f.offers().iter().map(|x| i32::from(x.0)).collect()
}
#[test]
fn organon_clues_schedule_ambush_only_after_all_three_in_order() {
    let mut f = Api::default();
    let mut s = Session::default();
    run(&mut f, &mut s, Handler::TrickMycon, I_HAVE_A_CUNNING_PLAN);
    assert_eq!(f.spoken(), [TELL_US_ABOUT_WORLD, AMBUSH_TAIL]);
    assert_eq!(choices(&f), [CLUE_1, CLUE_2, CLUE_3]);
    for (r, phrase, remaining) in [
        (CLUE_2, RESPONSE_2, vec![CLUE_1, CLUE_3]),
        (CLUE_1, RESPONSE_1, vec![CLUE_3]),
        (CLUE_3, RESPONSE_3, vec![]),
    ] {
        run(&mut f, &mut s, Handler::TrickMycon, r);
        assert_eq!(choices(&f), remaining);
        if r == CLUE_3 {
            assert_eq!(f.spoken(), [phrase, WE_GO_TO_IMPLANT]);
            assert_eq!(f.get("MYCON_FELL_FOR_AMBUSH"), 1);
            assert_eq!(f.segue, 0);
            assert!(f.effects.contains(&Effect::Event(1, 0, 0, 0, 13)));
        } else {
            assert_eq!(f.spoken(), [phrase, AMBUSH_TAIL]);
            assert_eq!(f.get("MYCON_FELL_FOR_AMBUSH"), 0);
        }
    }
}
#[test]
fn sun_device_cannot_be_tricked_but_landing_opens_after_ambush() {
    let mut f = Api::default();
    let mut s = Session::default();
    f.put("MYCON_AMBUSH", 1);
    run(&mut f, &mut s, Handler::SunDevice, 0);
    assert_eq!(
        choices(&f),
        [WHATS_UP_SUN_DEVICE, I_HAVE_A_CUNNING_PLAN, BYE_SUN_DEVICE]
    );
    run(&mut f, &mut s, Handler::SunDevice, I_HAVE_A_CUNNING_PLAN);
    assert_eq!(f.get("NO_TRICK_AT_SUN"), 1);
    assert_eq!(choices(&f), [WHATS_UP_SUN_DEVICE, BYE_SUN_DEVICE]);
    f.put("MYCON_FELL_FOR_AMBUSH", 1);
    run(&mut f, &mut s, Handler::SunDevice, WHATS_UP_SUN_DEVICE);
    assert_eq!(
        choices(&f),
        [HOW_GOES_IMPLANTING, LIKE_TO_LAND, BYE_SUN_DEVICE]
    );
    run(&mut f, &mut s, Handler::CombatIsInevitable, BYE_SUN_DEVICE);
    assert_eq!(f.segue, 0);
    run(&mut f, &mut s, Handler::CombatIsInevitable, LIKE_TO_LAND);
    assert_eq!(f.segue, 1);
    assert_eq!(f.spoken(), [NEVER_LET_LAND]);
}
#[test]
fn rng_low_byte_boundary_and_choice_suppression_match_space_and_home() {
    for (random, handler) in [
        (75, Handler::CombatIsInevitable),
        (76, Handler::NormalMycon),
        (256 + 75, Handler::CombatIsInevitable),
    ] {
        for home in [false, true] {
            let mut f = Api::default();
            let mut s = Session::default();
            f.world.random.push_back(random);
            f.put("GLOBAL_FLAGS_AND_DATA", if home { 128 } else { 0 });
            run(&mut f, &mut s, Handler::NormalMycon, 0);
            assert_eq!(f.world.random_calls, 1);
            assert_eq!(f.offers()[0].1, handler);
            f.world.random.push_back(100);
            run(
                &mut f,
                &mut s,
                Handler::NormalMycon,
                if home { QUESTION_1 } else { INSULT_1 },
            );
            assert_eq!(s.made_choice, 1);
            assert_eq!(f.get(if home { "MYCON_INFO" } else { "MYCON_INSULTS" }), 1);
            assert!(!choices(&f).contains(&if home { QUESTION_2 } else { INSULT_2 }));
        }
    }
}
#[test]
fn ramble_wrap_and_shattered_knowledge_and_intro_priority() {
    let mut f = Api::default();
    let mut s = Session::default();
    f.put("MYCON_RAMBLE", 24);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .do_ramble(COME_IN_PEACE);
    assert_eq!(f.spoken(), [RAMBLE_25]);
    assert_eq!(f.get("KNOW_ABOUT_SHATTERED"), 2);
    f.put("MYCON_RAMBLE", 31);
    f.effects.clear();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .do_ramble(COME_IN_PEACE);
    assert_eq!(f.spoken(), [RAMBLE_32]);
    assert_eq!(f.get("MYCON_RAMBLE"), 0);
    for (key, phrase) in [("SUN_DEVICE", DIE_THIEF), ("MYCON_KNOW_AMBUSH", DIE_LIAR)] {
        f.state.clear();
        f.put(key, 1);
        f.effects.clear();
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .intro();
        assert_eq!(f.spoken(), [phrase]);
        assert_eq!(f.segue, 1);
    }
    for (flags, key, phrase) in [
        (64, "MYCON_SUN_VISITS", HELLO_SUN_DEVICE_WORLD_1),
        (128, "MYCON_HOME_VISITS", HELLO_HOMEWORLD_1),
        (0, "MYCON_VISITS", HELLO_SPACE_1),
    ] {
        f.state.clear();
        f.put("GLOBAL_FLAGS_AND_DATA", flags);
        f.effects.clear();
        f.world.random.push_back(100);
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .intro();
        assert_eq!(f.spoken(), [phrase]);
        assert_eq!(f.get(key), 1);
    }
    s.made_choice = 1;
    f.activity = 5;
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .initialize();
    assert_eq!(s.made_choice, 0);
    assert_eq!(f.segue, 0);
}

#[test]
fn lifecycle_and_descriptor() {
    let mut f = Api::default();
    let mut s = Session::default();
    let mut d = Dialogue {
        api: &mut f,
        session: &mut s,
    };
    d.initialize();
    d.post_mycon_enc();
    assert_eq!(descriptor().alien_text_width, 224);
}

#[test]
fn animation_flags_and_masks_match_the_portrait() {
    let d = descriptor();
    assert_eq!(
        d.alien_ambient_array[..5]
            .iter()
            .map(|a| a.anim_flags)
            .collect::<Vec<_>>(),
        [2, 2, 2, 2, 4]
    );
    assert_eq!(d.alien_ambient_array[0].block_mask, 2);
    assert_eq!(d.alien_ambient_array[1].block_mask, 1);
}
