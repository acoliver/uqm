use super::descriptor::descriptor;
use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::DialogueHost;
use uqm_engine::StoryKey;
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
            assert_eq!(f.get(StoryKey::MYCON_FELL_FOR_AMBUSH), 1);
            assert_eq!(f.segue, 0);
            assert!(f.effects.contains(&Effect::Event(1, 0, 0, 0, 13)));
        } else {
            assert_eq!(f.spoken(), [phrase, AMBUSH_TAIL]);
            assert_eq!(f.get(StoryKey::MYCON_FELL_FOR_AMBUSH), 0);
        }
    }
}
#[test]
fn sun_device_cannot_be_tricked_but_landing_opens_after_ambush() {
    let mut f = Api::default();
    let mut s = Session::default();
    f.put(StoryKey::MYCON_AMBUSH, 1);
    run(&mut f, &mut s, Handler::SunDevice, 0);
    assert_eq!(
        choices(&f),
        [WHATS_UP_SUN_DEVICE, I_HAVE_A_CUNNING_PLAN, BYE_SUN_DEVICE]
    );
    run(&mut f, &mut s, Handler::SunDevice, I_HAVE_A_CUNNING_PLAN);
    assert_eq!(f.get(StoryKey::NO_TRICK_AT_SUN), 1);
    assert_eq!(choices(&f), [WHATS_UP_SUN_DEVICE, BYE_SUN_DEVICE]);
    f.put(StoryKey::MYCON_FELL_FOR_AMBUSH, 1);
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
            f.put(StoryKey::GLOBAL_FLAGS_AND_DATA, if home { 128 } else { 0 });
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
            assert_eq!(
                f.get(if home {
                    StoryKey::MYCON_INFO
                } else {
                    StoryKey::MYCON_INSULTS
                }),
                1
            );
            assert!(!choices(&f).contains(&if home { QUESTION_2 } else { INSULT_2 }));
        }
    }
}
#[test]
fn ramble_wrap_and_shattered_knowledge_and_intro_priority() {
    let mut f = Api::default();
    let mut s = Session::default();
    f.put(StoryKey::MYCON_RAMBLE, 24);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .do_ramble(COME_IN_PEACE);
    assert_eq!(f.spoken(), [RAMBLE_25]);
    assert_eq!(f.get(StoryKey::KNOW_ABOUT_SHATTERED), 2);
    f.put(StoryKey::MYCON_RAMBLE, 31);
    f.effects.clear();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .do_ramble(COME_IN_PEACE);
    assert_eq!(f.spoken(), [RAMBLE_32]);
    assert_eq!(f.get(StoryKey::MYCON_RAMBLE), 0);
    for (key, phrase) in [
        (StoryKey::SUN_DEVICE, DIE_THIEF),
        (StoryKey::MYCON_KNOW_AMBUSH, DIE_LIAR),
    ] {
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
        (64, StoryKey::MYCON_SUN_VISITS, HELLO_SUN_DEVICE_WORLD_1),
        (128, StoryKey::MYCON_HOME_VISITS, HELLO_HOMEWORLD_1),
        (0, StoryKey::MYCON_VISITS, HELLO_SPACE_1),
    ] {
        f.state.clear();
        f.put(StoryKey::GLOBAL_FLAGS_AND_DATA, flags);
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

const RAMBLES: [i32; 32] = [
    RAMBLE_1, RAMBLE_2, RAMBLE_3, RAMBLE_4, RAMBLE_5, RAMBLE_6, RAMBLE_7, RAMBLE_8, RAMBLE_9,
    RAMBLE_10, RAMBLE_11, RAMBLE_12, RAMBLE_13, RAMBLE_14, RAMBLE_15, RAMBLE_16, RAMBLE_17,
    RAMBLE_18, RAMBLE_19, RAMBLE_20, RAMBLE_21, RAMBLE_22, RAMBLE_23, RAMBLE_24, RAMBLE_25,
    RAMBLE_26, RAMBLE_27, RAMBLE_28, RAMBLE_29, RAMBLE_30, RAMBLE_31, RAMBLE_32,
];
const INSULTS: [i32; 8] = [
    INSULT_1, INSULT_2, INSULT_3, INSULT_4, INSULT_5, INSULT_6, INSULT_7, INSULT_8,
];
const QUESTIONS: [i32; 16] = [
    QUESTION_1,
    QUESTION_2,
    QUESTION_3,
    QUESTION_4,
    QUESTION_5,
    QUESTION_6,
    QUESTION_7,
    QUESTION_8,
    QUESTION_9,
    QUESTION_10,
    QUESTION_11,
    QUESTION_12,
    QUESTION_13,
    QUESTION_14,
    QUESTION_15,
    QUESTION_16,
];
const HOME_GREETINGS: [i32; 8] = [
    HELLO_HOMEWORLD_1,
    HELLO_HOMEWORLD_2,
    HELLO_HOMEWORLD_3,
    HELLO_HOMEWORLD_4,
    HELLO_HOMEWORLD_5,
    HELLO_HOMEWORLD_6,
    HELLO_HOMEWORLD_7,
    HELLO_HOMEWORLD_8,
];
const SPACE_GREETINGS: [i32; 8] = [
    HELLO_SPACE_1,
    HELLO_SPACE_2,
    HELLO_SPACE_3,
    HELLO_SPACE_4,
    HELLO_SPACE_5,
    HELLO_SPACE_6,
    HELLO_SPACE_7,
    HELLO_SPACE_8,
];

fn dialogue<'a>(f: &'a mut Api, s: &'a mut Session) -> Dialogue<'a, Api> {
    Dialogue { api: f, session: s }
}
fn greet(f: &mut Api, s: &mut Session) -> Vec<i32> {
    f.effects.clear();
    f.world.random.push_back(100);
    dialogue(f, s).intro();
    f.spoken()
}

#[test]
fn rambling_walks_every_line_in_order_and_starts_over() {
    let mut f = Api::default();
    let mut s = Session::default();
    for _ in 0..2 {
        for (index, line) in RAMBLES.iter().enumerate() {
            f.effects.clear();
            dialogue(&mut f, &mut s).do_ramble(GONNA_DIE);
            assert_eq!(f.spoken(), [*line]);
            assert_eq!(f.get(StoryKey::MYCON_RAMBLE), (index as u8 + 1) % 32);
        }
    }
    assert_eq!(
        f.get(StoryKey::MYCON_INSULTS),
        0,
        "peaceful answers are not insults"
    );
}

#[test]
fn space_insults_and_homeworld_questions_advance_once_per_choice() {
    for (home, key, wheel) in [
        (false, StoryKey::MYCON_INSULTS, &INSULTS[..]),
        (true, StoryKey::MYCON_INFO, &QUESTIONS[..]),
    ] {
        for (index, next) in wheel.iter().enumerate() {
            let mut f = Api::default();
            let mut s = Session::default();
            f.put(StoryKey::GLOBAL_FLAGS_AND_DATA, if home { 128 } else { 0 });
            f.put(key, index as u8);
            f.world.random.push_back(100);
            run(&mut f, &mut s, Handler::NormalMycon, 0);
            assert!(choices(&f).contains(next), "{home} {index}");
            s.made_choice = 1;
            f.world.random.push_back(100);
            run(&mut f, &mut s, Handler::NormalMycon, 0);
            assert!(!choices(&f).contains(next), "a made choice hides the wheel");
        }
        let mut f = Api::default();
        let mut s = Session::default();
        f.put(StoryKey::GLOBAL_FLAGS_AND_DATA, if home { 128 } else { 0 });
        f.put(key, wheel.len() as u8);
        f.world.random.push_back(100);
        run(&mut f, &mut s, Handler::NormalMycon, 0);
        assert!(wheel.iter().all(|line| !choices(&f).contains(line)));
    }
}

#[test]
fn greetings_follow_the_visit_counter_and_stop_advancing_at_the_last_line() {
    let mut s = Session::default();
    for (flags, key, lines) in [
        (128, StoryKey::MYCON_HOME_VISITS, &HOME_GREETINGS),
        (0, StoryKey::MYCON_VISITS, &SPACE_GREETINGS),
    ] {
        let mut f = Api::default();
        f.put(StoryKey::GLOBAL_FLAGS_AND_DATA, flags);
        for (visit, line) in lines.iter().enumerate() {
            assert_eq!(greet(&mut f, &mut s), [*line]);
            assert_eq!(f.get(key), (visit as u8 + 1).min(7));
        }
        assert_eq!(greet(&mut f, &mut s), [lines[7]]);
        assert_eq!(f.get(key), 7);
        f.put(key, 8);
        assert!(greet(&mut f, &mut s).is_empty());
        assert_eq!(f.get(key), 9);
    }
}

#[test]
fn thief_and_sun_world_greetings_repeat_their_second_line() {
    let mut s = Session::default();
    for (setup, key, first, second) in [
        (
            (StoryKey::SUN_DEVICE, 1),
            StoryKey::MYCON_VISITS,
            DIE_THIEF,
            DIE_THIEF_AGAIN,
        ),
        (
            (StoryKey::GLOBAL_FLAGS_AND_DATA, 64),
            StoryKey::MYCON_SUN_VISITS,
            HELLO_SUN_DEVICE_WORLD_1,
            HELLO_SUN_DEVICE_WORLD_2,
        ),
    ] {
        let mut f = Api::default();
        f.put(setup.0, setup.1);
        assert_eq!(greet(&mut f, &mut s), [first]);
        assert_eq!(greet(&mut f, &mut s), [second]);
        assert_eq!(greet(&mut f, &mut s), [second]);
        assert_eq!(f.get(key), 1);
        f.put(key, 2);
        assert!(greet(&mut f, &mut s).is_empty());
        assert_eq!(f.get(key), 3);
    }
}
