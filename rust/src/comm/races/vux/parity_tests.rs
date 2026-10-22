use super::dialogue::Dialogue;
use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;

type Api = Fake<Handler>;

fn handle(api: &mut Api, handler: Handler, response: u32) {
    Dialogue { api }.handle(handler, response as i32);
}

fn offers(api: &Api) -> Vec<u32> {
    api.offers()
        .into_iter()
        .map(|(p, _)| u32::from(p))
        .collect()
}

#[test]
fn every_combat_counter_preserves_byte_switch_and_final_repeat() {
    for (key, response, lines) in [
        (
            "VUX_STACK_3",
            CANT_WE_BE_FRIENDS_1,
            [
                NEVER_UGLY_HUMANS_1,
                NEVER_UGLY_HUMANS_2,
                NEVER_UGLY_HUMANS_3,
                NEVER_UGLY_HUMANS_4,
            ],
        ),
        (
            "VUX_STACK_4",
            BYE_HOSTILE_SPACE,
            [
                GOODBYE_AND_DIE_HOSTILE_SPACE_1,
                GOODBYE_AND_DIE_HOSTILE_SPACE_2,
                GOODBYE_AND_DIE_HOSTILE_SPACE_3,
                GOODBYE_AND_DIE_HOSTILE_SPACE_4,
            ],
        ),
    ] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put(key, visits);
            handle(&mut api, Handler::Combat, response);
            assert_eq!(
                api.spoken(),
                lines
                    .get(visits as usize)
                    .map(|p| *p as i32)
                    .into_iter()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                api.get(key),
                if visits == 3 {
                    3
                } else {
                    visits.wrapping_add(1)
                }
            );
            assert_eq!(api.segue, 1);
        }
    }
    for response in [
        CANT_WE_BE_FRIENDS_2,
        CANT_WE_BE_FRIENDS_3,
        CANT_WE_BE_FRIENDS_4,
    ] {
        let mut api = Api::default();
        handle(&mut api, Handler::Combat, response);
        assert_eq!(api.spoken(), [NEVER_UGLY_HUMANS_1 as i32]);
    }
    for response in [
        KILL_YOU_SQUIDS_1,
        KILL_YOU_SQUIDS_2,
        KILL_YOU_SQUIDS_3,
        KILL_YOU_SQUIDS_4,
    ] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put("VUX_STACK_2", visits);
            handle(&mut api, Handler::Combat, response);
            let next = visits.wrapping_add(1);
            assert_eq!(
                api.get("VUX_STACK_2"),
                if next <= 3 { next } else { visits }
            );
            assert_eq!(api.spoken(), [WE_FIGHT as i32]);
        }
    }
    let rejections = [
        NOT_ACCEPTED_1,
        NOT_ACCEPTED_2,
        NOT_ACCEPTED_3,
        NOT_ACCEPTED_4,
        NOT_ACCEPTED_5,
        NOT_ACCEPTED_6,
        NOT_ACCEPTED_7,
        NOT_ACCEPTED_8,
        NOT_ACCEPTED_9,
        TRUTH,
    ];
    for visits in 0..=u8::MAX {
        let mut api = Api::default();
        api.put("VUX_STACK_1", visits);
        handle(&mut api, Handler::Combat, APOLOGY_1);
        let expected = if (4..=13).contains(&visits) {
            vec![rejections[(visits - 4) as usize] as i32]
        } else {
            vec![]
        };
        assert_eq!(api.spoken(), expected);
        assert_eq!(api.get("VUX_STACK_1"), visits.wrapping_add(1));
    }
}

#[test]
fn normal_topics_and_each_menu_stack_match_c() {
    for (response, phrase, state, option, handler) in [
        (
            WHY_SO_MEAN,
            URQUAN_SLAVES,
            1,
            DEEPER_REASON,
            Handler::Normal,
        ),
        (
            DEEPER_REASON,
            OLD_INSULT,
            2,
            IF_WE_APOLOGIZE,
            Handler::Normal,
        ),
        (
            IF_WE_APOLOGIZE,
            PROBABLY_NOT,
            3,
            TRY_ANY_WAY,
            Handler::Combat,
        ),
    ] {
        let mut api = Api::default();
        handle(&mut api, Handler::Normal, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.get("VUX_STACK_1"), state);
        assert_eq!(api.offers()[0], (option as u16, handler));
    }
    let topics = [
        WHY_SO_MEAN,
        DEEPER_REASON,
        IF_WE_APOLOGIZE,
        TRY_ANY_WAY,
        APOLOGY_1,
        APOLOGY_2,
        APOLOGY_3,
        APOLOGY_4,
        APOLOGY_5,
        APOLOGY_6,
        APOLOGY_7,
        APOLOGY_8,
        APOLOGY_9,
        APOLOGY_10,
    ];
    for stack in 0..=u8::MAX {
        let mut api = Api::default();
        api.put("VUX_STACK_1", stack);
        api.put("VUX_STACK_2", stack);
        api.put("VUX_STACK_3", stack);
        handle(&mut api, Handler::Normal, 0);
        let mut expected = vec![];
        if let Some(p) = topics.get(stack as usize) {
            expected.push(*p);
        }
        if let Some(p) = [
            KILL_YOU_SQUIDS_1,
            KILL_YOU_SQUIDS_2,
            KILL_YOU_SQUIDS_3,
            KILL_YOU_SQUIDS_4,
        ]
        .get(stack as usize)
        {
            expected.push(*p);
        }
        expected.extend([WHATS_UP_HOSTILE, BYE_HOSTILE_SPACE]);
        assert_eq!(offers(&api), expected);
    }
    for (stack, friend) in [
        CANT_WE_BE_FRIENDS_1,
        CANT_WE_BE_FRIENDS_2,
        CANT_WE_BE_FRIENDS_3,
        CANT_WE_BE_FRIENDS_4,
    ]
    .into_iter()
    .enumerate()
    {
        let mut api = Api::default();
        api.put("VUX_STACK_1", 14);
        api.put("VUX_STACK_3", stack as u8);
        handle(&mut api, Handler::Normal, 0);
        assert_eq!(
            offers(&api),
            [
                KILL_YOU_SQUIDS_1,
                WHATS_UP_HOSTILE,
                friend,
                BYE_HOSTILE_SPACE
            ]
        );
    }
    for visits in 0..=u8::MAX {
        let mut api = Api::default();
        api.put("VUX_INFO", visits);
        handle(&mut api, Handler::Normal, WHATS_UP_HOSTILE);
        let lines = [
            GENERAL_INFO_HOSTILE_1,
            GENERAL_INFO_HOSTILE_2,
            GENERAL_INFO_HOSTILE_3,
            GENERAL_INFO_HOSTILE_4,
        ];
        assert_eq!(
            api.spoken(),
            lines
                .get(visits as usize)
                .map(|p| *p as i32)
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            api.get("VUX_INFO"),
            if visits == 3 {
                3
            } else {
                visits.wrapping_add(1)
            }
        );
        assert!(!offers(&api).contains(&WHATS_UP_HOSTILE));
    }
}

#[test]
fn zex_topics_prioritize_selected_stack_and_apply_exact_transitions() {
    for (response, phrase, key, state, expected) in [
        (
            WHAT_YOU_DO_HERE,
            MY_MENAGERIE,
            "ZEX_STACK_1",
            1,
            vec![
                WHAT_ABOUT_MENAGERIE,
                WHY_DONT_YOU_ATTACK,
                TAKE_BY_FORCE,
                BYE_ZEX,
            ],
        ),
        (
            WHAT_ABOUT_MENAGERIE,
            NEED_NEW_CREATURE,
            "ZEX_STACK_1",
            2,
            vec![
                WHAT_ABOUT_CREATURE,
                WHY_DONT_YOU_ATTACK,
                TAKE_BY_FORCE,
                BYE_ZEX,
            ],
        ),
        (
            WHAT_ABOUT_CREATURE,
            ABOUT_CREATURE,
            "ZEX_STACK_1",
            3,
            vec![
                ABOUT_CREATURE_AGAIN,
                WHY_DONT_YOU_ATTACK,
                TAKE_BY_FORCE,
                BYE_ZEX,
            ],
        ),
        (
            ABOUT_CREATURE_AGAIN,
            CREATURE_AGAIN,
            "ZEX_STACK_1",
            3,
            vec![WHY_DONT_YOU_ATTACK, TAKE_BY_FORCE, BYE_ZEX],
        ),
        (
            WHY_DONT_YOU_ATTACK,
            LIKE_YOU,
            "ZEX_STACK_2",
            1,
            vec![WHY_LIKE_ME, WHAT_YOU_DO_HERE, TAKE_BY_FORCE, BYE_ZEX],
        ),
        (
            WHY_LIKE_ME,
            LIKE_BECAUSE,
            "ZEX_STACK_2",
            2,
            vec![ARE_YOU_A_PERVERT, WHAT_YOU_DO_HERE, TAKE_BY_FORCE, BYE_ZEX],
        ),
        (
            ARE_YOU_A_PERVERT,
            CALL_ME_WHAT_YOU_WISH,
            "ZEX_STACK_2",
            3,
            vec![WHAT_YOU_DO_HERE, TAKE_BY_FORCE, BYE_ZEX],
        ),
        (
            TAKE_BY_FORCE,
            PRECURSOR_DEVICE,
            "ZEX_STACK_3",
            1,
            vec![REGARDLESS, WHAT_YOU_DO_HERE, WHY_DONT_YOU_ATTACK, BYE_ZEX],
        ),
        (
            YOU_LIED,
            YUP_LIED,
            "ZEX_STACK_3",
            3,
            vec![KILL_YOU, WHAT_YOU_DO_HERE, WHY_DONT_YOU_ATTACK, BYE_ZEX],
        ),
    ] {
        let mut api = Api::default();
        if response == ABOUT_CREATURE_AGAIN {
            api.put("ZEX_STACK_1", 3);
        }
        handle(&mut api, Handler::Menagerie, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.get(key), state);
        assert_eq!(offers(&api), expected);
        if response == WHAT_ABOUT_CREATURE {
            assert_eq!(api.get("KNOW_ZEX_WANTS_MONSTER"), 1);
            assert!(api.disabled.contains(&(WHAT_ABOUT_CREATURE as u16)));
            assert!(!api.disabled.contains(&(ABOUT_CREATURE_AGAIN as u16)));
        }
    }
    for knows in [0, 1] {
        for beast in [0, 1] {
            let mut api = Api::default();
            api.put("KNOW_ZEX_WANTS_MONSTER", knows);
            api.put("VUX_BEAST_ON_SHIP", beast);
            handle(&mut api, Handler::Menagerie, 0);
            assert_eq!(
                offers(&api)[0],
                if knows != 0 && beast != 0 {
                    I_HAVE_BEAST
                } else {
                    WHAT_YOU_DO_HERE
                }
            );
        }
    }
    let mut api = Api::default();
    for key in ["ZEX_STACK_1", "ZEX_STACK_2", "ZEX_STACK_3"] {
        api.put(key, 255);
    }
    handle(&mut api, Handler::Menagerie, 0);
    assert_eq!(offers(&api), [BYE_ZEX]);
}

#[test]
fn zex_trust_sequence_and_terminal_branches_preserve_effect_order() {
    let mut api = Api::default();
    for (response, phrase, next) in [
        (I_HAVE_BEAST, GIVE_BEAST, Some(WHY_TRUST_1)),
        (WHY_TRUST_1, TRUST_1, Some(WHY_TRUST_2)),
        (WHY_TRUST_2, TRUST_2, Some(WHY_TRUST_3)),
        (WHY_TRUST_3, TRUST_3, None),
    ] {
        api.effects.clear();
        handle(&mut api, Handler::Menagerie, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        let mut expected: Vec<_> = next.into_iter().collect();
        expected.push(OK_TAKE_BEAST);
        assert_eq!(offers(&api), expected);
    }
    for (response, phrase) in [(KILL_YOU, FIGHT_AGAIN), (REGARDLESS, THEN_FIGHT)] {
        let mut api = Api::default();
        api.put("ZEX_VISITS", 3);
        handle(&mut api, Handler::Menagerie, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.segue, 1);
        assert!(api.offers().is_empty());
        if response == REGARDLESS {
            assert_eq!(api.get("ZEX_STACK_3"), 2);
            assert_eq!(api.get("ZEX_VISITS"), 0);
        }
    }
    let mut api = Api::default();
    handle(&mut api, Handler::Combat, OK_TAKE_BEAST);
    assert_eq!(
        api.effects,
        [
            Effect::Segue(1),
            Effect::Npc(FOOL_AIEE0 as i32),
            Effect::Npc(FOOL_AIEE1 as i32),
            Effect::Talk(1),
            Effect::FadePalette(1, 210),
            Effect::Talk(u16::MAX),
            Effect::Set("VUX_BEAST_ON_SHIP".into(), 0),
            Effect::Set("ZEX_IS_DEAD".into(), 1),
            Effect::Segue(0),
        ]
    );
    api.effects.clear();
    handle(&mut api, Handler::Combat, TRY_ANY_WAY);
    assert_eq!(api.spoken(), [NOPE as i32]);
    assert_eq!(api.get("VUX_STACK_1"), 4);
    api.effects.clear();
    handle(&mut api, Handler::Combat, BYE_ZEX);
    assert_eq!(api.spoken(), [GOODBYE_ZEX as i32]);
    assert_eq!(api.segue, 0);
}

#[test]
fn all_greeting_paths_hold_last_case_and_increment_unmatched_byte_values() {
    for (flags, hostile_zex, key, lines) in [
        (
            0,
            0,
            "VUX_VISITS",
            vec![SPACE_HELLO_1, SPACE_HELLO_2, SPACE_HELLO_3, SPACE_HELLO_4],
        ),
        (
            128,
            0,
            "VUX_HOME_VISITS",
            vec![
                HOMEWORLD_HELLO_1,
                HOMEWORLD_HELLO_2,
                HOMEWORLD_HELLO_3,
                HOMEWORLD_HELLO_4,
            ],
        ),
        (
            192,
            0,
            "ZEX_VISITS",
            vec![ZEX_HELLO_1, ZEX_HELLO_2, ZEX_HELLO_3, ZEX_HELLO_4],
        ),
        (
            64,
            2,
            "ZEX_VISITS",
            vec![FIGHT_OR_TRADE_1, FIGHT_OR_TRADE_2],
        ),
    ] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put("GLOBAL_FLAGS_AND_DATA", flags);
            api.put("ZEX_STACK_3", hostile_zex);
            api.put(key, visits);
            Dialogue { api: &mut api }.intro();
            assert_eq!(
                api.spoken(),
                lines
                    .get(visits as usize)
                    .map(|p| *p as i32)
                    .into_iter()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                api.get(key),
                if visits as usize == lines.len() - 1 {
                    visits
                } else {
                    visits.wrapping_add(1)
                }
            );
        }
    }
    let mut api = Api {
        activity: 0x100 | crate::comm::dispatch::WON_LAST_BATTLE,
        ..Api::default()
    };
    api.put("GLOBAL_FLAGS_AND_DATA", 192);
    Dialogue { api: &mut api }.intro();
    assert_eq!(
        api.effects,
        [Effect::Npc(OUT_TAKES as i32), Effect::Segue(0)]
    );
}

#[test]
fn full_descriptor_matches_c_including_opaque_black_and_dynamic_layout() {
    for (screen_width, x, width) in [(320, 181, 112), (640, 421, 272), (323, 182, 113)] {
        let d = descriptor(screen_width);
        assert_eq!(
            (
                d.alien_text_baseline.x,
                d.alien_text_baseline.y,
                d.alien_text_width
            ),
            (x, 0, width)
        );
    }
    let d = descriptor(320);
    let specs = [
        (12, 3, 1, 28, 28, 28, 28, 0),
        (15, 5, 1, 28, 28, 28, 28, 0),
        (20, 14, 2, 28, 0, 28, 0, 0),
        (34, 7, 2, 28, 28, 28, 28, 0),
        (41, 6, 1, 28, 28, 28, 28, 0),
        (47, 11, 2, 28, 28, 28, 28, 0),
        (58, 3, 1, 28, 28, 28, 28, 0),
        (61, 4, 1, 28, 28, 28, 28, 0),
        (65, 4, 1, 28, 28, 28, 28, 0),
        (69, 2, 1, 28, 28, 28, 28, 0),
        (71, 3, 4, 42, 0, 840, 2520, 0),
        (74, 6, 4, 56, 56, 56, 56, 0),
        (80, 5, 4, 56, 56, 56, 56, 16384),
        (85, 5, 4, 56, 56, 56, 56, 0),
        (90, 5, 4, 56, 56, 56, 56, 4096),
        (95, 4, 4, 56, 56, 4200, 4200, 0),
        (99, 4, 4, 56, 56, 4200, 4200, 0),
    ];
    assert_eq!(usize::from(d.num_animations), specs.len());
    for (a, expected) in d.alien_ambient_array.iter().zip(specs) {
        assert_eq!(
            (
                a.start_index,
                a.num_frames,
                a.anim_flags,
                a.base_frame_rate,
                a.random_frame_rate,
                a.base_restart_rate,
                a.random_restart_rate,
                a.block_mask
            ),
            expected
        );
    }
    for (a, expected) in [
        (d.alien_transition_desc, (0, 0, 0, 0)),
        (d.alien_talk_desc, (1, 11, 56, 70)),
    ] {
        assert_eq!(
            (
                a.start_index,
                a.num_frames,
                a.base_frame_rate,
                a.base_restart_rate
            ),
            expected
        );
        assert_eq!(
            (
                a.anim_flags,
                a.random_frame_rate,
                a.random_restart_rate,
                a.block_mask
            ),
            (0, 0, 0, 0)
        );
    }
    for (resource, expected) in [
        (d.alien_frame_res, c"comm.vux.graphics"),
        (d.alien_font_res, c"comm.vux.font"),
        (d.alien_colormap_res, c"comm.vux.colortable"),
        (d.alien_song_res, c"comm.vux.music"),
        (d.conversation_phrases_res, c"comm.vux.dialogue"),
    ] {
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(resource) }, expected);
    }
    assert_eq!((d.alien_text_align, d.alien_text_valign), (1, 0));
    assert_eq!(
        (
            d.alien_text_fcolor.r,
            d.alien_text_fcolor.g,
            d.alien_text_fcolor.b,
            d.alien_text_fcolor.a
        ),
        (255, 255, 255, 255)
    );
    assert_eq!(
        (
            d.alien_text_bcolor.r,
            d.alien_text_bcolor.g,
            d.alien_text_bcolor.b,
            d.alien_text_bcolor.a
        ),
        (0, 0, 0, 255)
    );
    assert!(d.alien_alt_song_res.is_null());
    assert_eq!(d.alien_song_flags, 0);
    assert!(d.alien_number_speech.is_null());
}
