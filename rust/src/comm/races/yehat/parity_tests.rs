use super::dialogue::Dialogue;
use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::{ScriptApi, GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};

type Api = Fake<Handler, api::World>;

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
fn alliance_demands_construct_every_fragment_in_c_order_for_all_names() {
    for (name, alliance) in [
        (0, vec![70]),
        (1, vec![71]),
        (2, vec![72]),
        (3, vec![73, GLOBAL_PLAYER_NAME, 74]),
    ] {
        for handler in [Handler::Home, Handler::Space] {
            let mut api = Api::default();
            api.put("NEW_ALLIANCE_NAME", name);
            handle(&mut api, handler, 0);
            let (phrase, callback, expected) = if handler == Handler::Home {
                let mut expected = vec![6, GLOBAL_PLAYER_NAME, 7];
                expected.extend_from_slice(&alliance);
                expected.extend([8, GLOBAL_SHIP_NAME, 9]);
                (I_DEMAND_YOU_ALLY_HOMEWORLD0, Handler::Exit, expected)
            } else {
                let mut expected = vec![32, GLOBAL_PLAYER_NAME, 33, GLOBAL_SHIP_NAME, 34];
                expected.extend_from_slice(&alliance);
                expected.push(35);
                (I_DEMAND_YOU_ALLY_SPACE0, Handler::Space, expected)
            };
            assert!(api
                .effects
                .contains(&Effect::Offer(phrase as u16, callback, expected)));
        }
    }
}

#[test]
fn all_greeting_switches_keep_c_byte_postincrement_and_final_decrement() {
    for (civil_war, homeworld, key, lines, handler) in [
        (
            0,
            0,
            "YEHAT_VISITS",
            vec![SPACE_HELLO_1, SPACE_HELLO_2, SPACE_HELLO_3, SPACE_HELLO_4],
            Handler::Space,
        ),
        (
            0,
            128,
            "YEHAT_HOME_VISITS",
            vec![HOMEWORLD_HELLO_1, HOMEWORLD_HELLO_2],
            Handler::Home,
        ),
        (
            1,
            0,
            "YEHAT_VISITS",
            vec![ROYALIST_SPACE_HELLO_1, ROYALIST_SPACE_HELLO_2],
            Handler::Royalists,
        ),
        (
            1,
            128,
            "YEHAT_HOME_VISITS",
            vec![ROYALIST_HOMEWORLD_HELLO_1, ROYALIST_HOMEWORLD_HELLO_2],
            Handler::Royalists,
        ),
    ] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put("YEHAT_CIVIL_WAR", civil_war);
            api.put("GLOBAL_FLAGS_AND_DATA", homeworld);
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
            assert_eq!(api.offers()[0].1, handler);
        }
    }
    let mut api = Api {
        activity: 0x100 | crate::comm::dispatch::WON_LAST_BATTLE,
        ..Api::default()
    };
    api.put("YEHAT_CIVIL_WAR", 1);
    api.put("GLOBAL_FLAGS_AND_DATA", 128);
    Dialogue { api: &mut api }.initialize();
    assert_eq!(api.segue, 0);
    api.effects.clear();
    Dialogue { api: &mut api }.intro();
    assert_eq!(
        api.effects,
        [Effect::Npc(OUT_TAKES as i32), Effect::Segue(0)]
    );
    api.activity = 0x100;
    Dialogue { api: &mut api }.initialize();
    assert_eq!(api.segue, 1);
}

#[test]
fn royalist_home_and_space_information_switches_cover_every_byte() {
    for (handler, response, key, lines, disabled) in [
        (
            Handler::Royalists,
            HOW_IS_REBELLION,
            "YEHAT_ROYALIST_INFO",
            vec![ROYALIST_REBELLION_1, ROYALIST_REBELLION_2],
            HOW_IS_REBELLION,
        ),
        (
            Handler::Home,
            WHATS_UP_HOMEWORLD,
            "YEHAT_ROYALIST_INFO",
            vec![GENERAL_INFO_HOMEWORLD_1, GENERAL_INFO_HOMEWORLD_2],
            WHATS_UP_HOMEWORLD,
        ),
        (
            Handler::Space,
            WHATS_UP_SPACE_1,
            "YEHAT_REBEL_INFO",
            vec![
                GENERAL_INFO_SPACE_1,
                GENERAL_INFO_SPACE_2,
                GENERAL_INFO_SPACE_3,
                GENERAL_INFO_SPACE_4,
            ],
            WHATS_UP_SPACE_1,
        ),
    ] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put(key, visits);
            handle(&mut api, handler, response);
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
            assert_eq!(api.disabled.len(), 1);
            assert!(api.disabled.contains(&(disabled as u16)));
            assert!(!offers(&api).contains(&disabled));
        }
    }
    for response in [WHATS_UP_SPACE_2, WHATS_UP_SPACE_3, WHATS_UP_SPACE_4] {
        let mut api = Api::default();
        handle(&mut api, Handler::Space, response);
        assert_eq!(api.spoken(), [GENERAL_INFO_SPACE_1 as i32]);
        assert!(api.disabled.contains(&(WHATS_UP_SPACE_1 as u16)));
    }
}

#[test]
fn pkunk_offers_require_both_visits_and_use_royalist_phrases_in_all_contexts() {
    for handler in [Handler::Royalists, Handler::Home, Handler::Space] {
        for visits in [0, 1] {
            for home in [0, 1] {
                for told in [0, 1] {
                    let mut api = Api::default();
                    api.put("PKUNK_VISITS", visits);
                    api.put("PKUNK_HOME_VISITS", home);
                    api.put("YEHAT_ROYALIST_TOLD_PKUNK", told);
                    handle(&mut api, handler, 0);
                    assert_eq!(
                        offers(&api).contains(&WHAT_ABOUT_PKUNK_ROYALIST),
                        visits != 0 && home != 0 && told == 0
                    );
                }
            }
        }
        for absorbed in [0, 1] {
            let mut api = Api::default();
            api.put("YEHAT_ABSORBED_PKUNK", absorbed);
            handle(&mut api, handler, WHAT_ABOUT_PKUNK_ROYALIST);
            assert_eq!(
                api.spoken(),
                [if absorbed != 0 {
                    PKUNK_ABSORBED_ROYALIST
                } else {
                    HATE_PKUNK_ROYALIST
                } as i32]
            );
            assert_eq!(api.get("YEHAT_ROYALIST_TOLD_PKUNK"), 1);
            assert!(!offers(&api).contains(&WHAT_ABOUT_PKUNK_ROYALIST));
        }
    }
}

#[test]
fn home_and_royalist_topics_remove_only_their_own_options() {
    for (handler, response, phrase, key, removed) in [
        (
            Handler::Royalists,
            SORRY_ABOUT_REVOLUTION,
            ALL_YOUR_FAULT,
            "NO_YEHAT_INFO",
            SORRY_ABOUT_REVOLUTION,
        ),
        (
            Handler::Home,
            GIVE_INFO,
            NO_INFO_FOR_ENEMY,
            "NO_YEHAT_INFO",
            GIVE_INFO,
        ),
        (
            Handler::Home,
            AT_LEAST_HELP_US_HOMEWORLD,
            NO_HELP_ENEMY,
            "NO_YEHAT_HELP_HOME",
            AT_LEAST_HELP_US_HOMEWORLD,
        ),
    ] {
        let mut api = Api::default();
        handle(&mut api, handler, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.get(key), 1);
        assert!(!offers(&api).contains(&removed));
    }
    let mut api = Api::default();
    for key in ["NO_YEHAT_INFO", "NO_YEHAT_HELP_HOME", "NO_YEHAT_ALLY_HOME"] {
        api.put(key, 1);
    }
    api.disabled.insert(WHATS_UP_HOMEWORLD as u16);
    handle(&mut api, Handler::Home, 0);
    assert_eq!(offers(&api), [BYE_HOMEWORLD]);
    api.effects.clear();
    api.disabled.insert(HOW_IS_REBELLION as u16);
    handle(&mut api, Handler::Royalists, 0);
    assert_eq!(offers(&api), [BYE_ROYALIST]);
}

#[test]
fn space_stack_order_and_early_return_match_each_topic() {
    for (response, phrase, key, value, expected) in [
        (
            I_DEMAND_YOU_ALLY_SPACE0,
            WE_CANNOT_1,
            "NO_YEHAT_ALLY_SPACE",
            1,
            vec![
                OBLIGATION,
                WHATS_UP_SPACE_1,
                AT_LEAST_HELP_US_SPACE,
                BYE_SPACE,
            ],
        ),
        (
            AT_LEAST_HELP_US_SPACE,
            SORRY_CANNOT,
            "NO_YEHAT_HELP_SPACE",
            1,
            vec![
                DISHONOR,
                WHATS_UP_SPACE_1,
                I_DEMAND_YOU_ALLY_SPACE0,
                BYE_SPACE,
            ],
        ),
        (
            DISHONOR,
            HERES_A_HINT,
            "NO_YEHAT_HELP_SPACE",
            2,
            vec![WHATS_UP_SPACE_1, I_DEMAND_YOU_ALLY_SPACE0, BYE_SPACE],
        ),
        (OBLIGATION, WE_CANNOT_2, "NO_YEHAT_ALLY_SPACE", 2, vec![]),
    ] {
        let mut api = Api::default();
        handle(&mut api, Handler::Space, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.get(key), value);
        assert_eq!(offers(&api), expected);
        assert_eq!(api.world.random_calls, 0);
        if response == OBLIGATION {
            assert_eq!(
                api.effects,
                [
                    Effect::Npc(WE_CANNOT_2 as i32),
                    Effect::Segue(0),
                    Effect::Set("NO_YEHAT_ALLY_SPACE".into(), 2)
                ]
            );
        }
    }
    for info in 0..=u8::MAX {
        let mut api = Api::default();
        api.put("YEHAT_REBEL_INFO", info);
        api.put("NO_YEHAT_ALLY_SPACE", 2);
        api.put("NO_YEHAT_HELP_SPACE", 2);
        handle(&mut api, Handler::Space, 0);
        let mut expected: Vec<_> = [
            WHATS_UP_SPACE_1,
            WHATS_UP_SPACE_2,
            WHATS_UP_SPACE_3,
            WHATS_UP_SPACE_4,
        ]
        .get(info as usize)
        .copied()
        .into_iter()
        .collect();
        expected.push(BYE_SPACE);
        assert_eq!(offers(&api), expected);
    }
}

#[test]
fn shofixti_offers_and_proof_choices_follow_state_and_actual_escort() {
    for visited in [0, 1] {
        for told in [0, 1, 2] {
            let mut api = Api::default();
            api.put("SHOFIXTI_VISITS", visited);
            api.put("YEHAT_REBEL_TOLD_PKUNK", told);
            handle(&mut api, Handler::Space, 0);
            assert_eq!(
                offers(&api).contains(&SHOFIXTI_ALIVE_1),
                visited != 0 && told == 0
            );
            assert_eq!(
                offers(&api).contains(&SHOFIXTI_ALIVE_2),
                visited != 0 && told == 1
            );
        }
    }
    for response in [SHOFIXTI_ALIVE_1, SHOFIXTI_ALIVE_2] {
        for have_escort in [false, true] {
            let mut api = Api::default();
            api.world.have_shofixti = have_escort;
            handle(&mut api, Handler::StartRevolt, response);
            assert_eq!(
                api.spoken(),
                [if response == SHOFIXTI_ALIVE_1 {
                    SEND_HIM_OVER_1
                } else {
                    SEND_HIM_OVER_2
                } as i32]
            );
            assert_eq!(
                api.get("YEHAT_REBEL_TOLD_PKUNK"),
                u8::from(response == SHOFIXTI_ALIVE_1)
            );
            assert_eq!(
                api.offers(),
                [
                    (
                        if have_escort { OK_SEND } else { NOT_HERE } as u16,
                        Handler::Exit
                    ),
                    (NOT_SEND as u16, Handler::Exit)
                ]
            );
        }
    }
}

#[test]
fn exit_endings_rng_and_revolt_effects_are_exact() {
    for (response, phrase) in [
        (BYE_HOMEWORLD, GOODBYE_AND_DIE_HOMEWORLD),
        (BYE_ROYALIST, GOODBYE_AND_DIE_ROYALIST),
        (I_DEMAND_YOU_ALLY_HOMEWORLD0, ENEMY_MUST_DIE),
    ] {
        let mut api = Api::default();
        handle(&mut api, Handler::Exit, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.segue, 1);
        assert_eq!(api.world.random_calls, 0);
        assert_eq!(
            api.get("NO_YEHAT_ALLY_HOME"),
            u8::from(response == I_DEMAND_YOU_ALLY_HOMEWORLD0)
        );
    }
    for bit in [0, 1] {
        let mut api = Api::default();
        api.world.random_bit = bit;
        handle(&mut api, Handler::Exit, BYE_SPACE);
        assert_eq!(api.world.random_calls, 1);
        assert_eq!(api.segue, u32::from(bit));
        assert_eq!(
            api.spoken(),
            [if bit == 0 {
                GO_IN_PEACE
            } else {
                GOODBYE_AND_DIE_SPACE
            } as i32]
        );
    }
    for response in [NOT_HERE, NOT_SEND] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put("YEHAT_REBEL_VISITS", visits);
            handle(&mut api, Handler::Exit, response);
            assert_eq!(
                api.spoken(),
                [JUST_A_TRICK_1, JUST_A_TRICK_2]
                    .get(visits as usize)
                    .map(|p| *p as i32)
                    .into_iter()
                    .collect::<Vec<_>>()
            );
            assert_eq!(api.get("YEHAT_REBEL_VISITS"), 1);
            assert_eq!(api.segue, 1);
            assert_eq!(api.world.random_calls, 0);
        }
    }
    let mut api = Api::default();
    for key in [
        "YEHAT_VISITS",
        "YEHAT_HOME_VISITS",
        "YEHAT_REBEL_VISITS",
        "YEHAT_REBEL_INFO",
        "YEHAT_REBEL_TOLD_PKUNK",
        "NO_YEHAT_INFO",
        "YEHAT_ROYALIST_INFO",
        "NO_YEHAT_HELP_SPACE",
    ] {
        api.put(key, 1);
    }
    handle(&mut api, Handler::Exit, OK_SEND);
    assert_eq!(
        api.effects,
        [
            Effect::Segue(1),
            Effect::Npc(WE_REVOLT as i32),
            Effect::Segue(0),
            Effect::Set("YEHAT_CIVIL_WAR".into(), 1),
            Effect::Set("YEHAT_VISITS".into(), 0),
            Effect::Set("YEHAT_HOME_VISITS".into(), 0),
            Effect::Set("YEHAT_REBEL_VISITS".into(), 0),
            Effect::Set("YEHAT_REBEL_INFO".into(), 0),
            Effect::Set("YEHAT_REBEL_TOLD_PKUNK".into(), 0),
            Effect::Set("NO_YEHAT_INFO".into(), 0),
            Effect::Event(1, 0, 0, 0, 15),
        ]
    );
    assert_eq!(api.get("YEHAT_ROYALIST_INFO"), 1);
    assert_eq!(api.get("NO_YEHAT_HELP_SPACE"), 1);
    assert_eq!(api.world.random_calls, 0);
}

#[test]
fn descriptor_matches_every_c_animation_field_and_runtime_geometry() {
    for (screen_width, x, width) in [(320, 161, 149), (640, 374, 362), (322, 162, 150)] {
        let d = descriptor(screen_width);
        assert_eq!(
            (
                d.alien_text_baseline.x,
                d.alien_text_baseline.y,
                d.alien_text_width
            ),
            (x, 60, width)
        );
    }
    let d = descriptor(320);
    let specs = [
        (4, 3, 12, 84, 84, 210, 420, 192),
        (7, 3, 12, 84, 84, 210, 420, 192),
        (10, 3, 4, 42, 0, 840, 2520, 16400),
        (13, 3, 4, 42, 0, 840, 2520, 32),
        (16, 5, 4, 56, 56, 5040, 2520, 16388),
        (21, 5, 4, 56, 56, 5040, 2520, 8),
        (26, 2, 12, 56, 56, 5040, 2520, 3),
        (28, 2, 12, 56, 56, 5040, 2520, 3),
        (30, 3, 4, 28, 28, 28, 28, 0),
        (33, 3, 4, 28, 28, 28, 28, 0),
        (36, 3, 4, 28, 28, 28, 28, 0),
        (39, 3, 4, 28, 28, 28, 28, 0),
        (42, 3, 4, 28, 28, 28, 28, 0),
        (45, 3, 4, 28, 28, 28, 28, 0),
        (48, 4, 12, 28, 0, 840, 2520, 20),
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
        (d.alien_talk_desc, (1, 3, 56, 70)),
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
        (d.alien_frame_res, c"comm.yehat.graphics"),
        (d.alien_font_res, c"comm.yehat.font"),
        (d.alien_colormap_res, c"comm.yehat.colortable"),
        (d.alien_song_res, c"comm.yehat.music"),
        (d.conversation_phrases_res, c"comm.yehat.dialogue"),
    ] {
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(resource) }, expected);
    }
    assert_eq!((d.alien_text_align, d.alien_text_valign), (1, 1));
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
