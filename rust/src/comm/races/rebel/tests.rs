use super::api::Date;
use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;

type Api = Fake<Handler, Date>;
fn choices(f: &Api) -> Vec<i32> {
    f.offers().iter().map(|x| i32::from(x.0)).collect()
}

#[test]
fn information_questions_disable_in_order_until_the_submenu_is_exhausted() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.handle(Handler::Info, GIVE_INFO_REBELS);
    assert_eq!(f.spoken(), [WHAT_INFO]);
    let questions = [
        WHAT_ABOUT_URQUAN,
        WHAT_ABOUT_ROYALTY,
        WHAT_ABOUT_WAR,
        WHAT_ABOUT_VUX,
        WHAT_ABOUT_CLUE,
    ];
    assert_eq!(choices(&f), [21, 17, 19, 23, 25, 27]);
    for (i, phrase) in [
        ABOUT_URQUAN,
        ABOUT_ROYALTY,
        ABOUT_WAR,
        ABOUT_VUX,
        ABOUT_CLUE,
    ]
    .into_iter()
    .enumerate()
    {
        f.effects.clear();
        Dialogue { api: &mut f }.handle(Handler::Info, questions[i]);
        assert_eq!(f.spoken(), [phrase]);
        assert!(f.disabled.contains(&(questions[i] as u16)));
        let mut expected = questions[i + 1..].to_vec();
        expected.push(ENOUGH_INFO);
        assert_eq!(choices(&f), expected);
        assert_eq!(
            f.offers().last(),
            Some(&(ENOUGH_INFO as u16, Handler::Rebels))
        );
        assert_eq!(f.enabled(GIVE_INFO_REBELS as u16), i != 4);
    }
    f.effects.clear();
    Dialogue { api: &mut f }.handle(Handler::Rebels, ENOUGH_INFO);
    assert_eq!(f.spoken(), [OK_ENOUGH_INFO]);
    assert_eq!(choices(&f), [HOW_GOES_REVOLUTION, ANY_SHIPS, BYE_REBEL]);
}

#[test]
fn rebel_questions_and_pkunk_visit_gates_keep_response_order() {
    for visits in [0, 1] {
        for home in [0, 1] {
            for told in [0, 1] {
                let mut f = Api::default();
                f.put("PKUNK_VISITS", visits);
                f.put("PKUNK_HOME_VISITS", home);
                f.put("YEHAT_REBEL_TOLD_PKUNK", told);
                Dialogue { api: &mut f }.rebels(0);
                let mut expected = vec![HOW_GOES_REVOLUTION];
                if visits != 0 && home != 0 && told == 0 {
                    expected.push(WHAT_ABOUT_PKUNK_REBEL);
                }
                expected.extend([ANY_SHIPS, GIVE_INFO_REBELS, BYE_REBEL]);
                assert_eq!(choices(&f), expected);
                assert_eq!(f.offers().last(), Some(&(BYE_REBEL as u16, Handler::Exit)));
            }
        }
    }
    for absorbed in [0, 1] {
        let mut f = Api::default();
        f.put("YEHAT_ABSORBED_PKUNK", absorbed);
        Dialogue { api: &mut f }.rebels(WHAT_ABOUT_PKUNK_REBEL);
        assert_eq!(
            f.spoken(),
            [if absorbed == 0 {
                HATE_PKUNK_REBEL
            } else {
                PKUNK_ABSORBED_REBEL
            }]
        );
        assert_eq!(f.get("YEHAT_REBEL_TOLD_PKUNK"), 1);
    }
    let mut f = Api::default();
    Dialogue { api: &mut f }.handle(Handler::Exit, BYE_REBEL);
    assert_eq!(f.effects, [Effect::Npc(GOODBYE_REBEL)]);
    f.effects.clear();
    Dialogue { api: &mut f }.handle(Handler::Exit, 0);
    assert!(f.effects.is_empty());
}

#[test]
fn all_visit_and_revolution_counters_preserve_postincrement_without_zero_phrases() {
    for n in 0..=u8::MAX {
        let mut f = Api::default();
        f.put("YEHAT_REBEL_VISITS", n);
        Dialogue { api: &mut f }.intro();
        assert_eq!(
            f.spoken(),
            if n < 4 {
                vec![1 + i32::from(n)]
            } else {
                vec![]
            }
        );
        assert_eq!(
            f.get("YEHAT_REBEL_VISITS"),
            if n == 3 { n } else { n.wrapping_add(1) }
        );
        assert_eq!(f.effects.first(), Some(&Effect::Segue(0)));
        f.effects.clear();
        f.put("YEHAT_REBEL_INFO", n);
        Dialogue { api: &mut f }.rebels(HOW_GOES_REVOLUTION);
        assert_eq!(
            f.spoken(),
            if n < 4 {
                vec![6 + i32::from(n)]
            } else {
                vec![]
            }
        );
        assert_eq!(
            f.get("YEHAT_REBEL_INFO"),
            if n == 3 { n } else { n.wrapping_add(1) }
        );
        assert!(!f.enabled(HOW_GOES_REVOLUTION as u16));
        assert!(!choices(&f).contains(&HOW_GOES_REVOLUTION));
    }
}

#[test]
fn ships_use_the_current_month_and_signed_byte_date_differences() {
    for (date, ready) in [
        (
            Date {
                year: 2155,
                month: 4,
                day: 29,
            },
            false,
        ),
        (
            Date {
                year: 2156,
                month: 3,
                day: 29,
            },
            false,
        ),
        (
            Date {
                year: 2156,
                month: 4,
                day: 27,
            },
            false,
        ),
        (
            Date {
                year: 2156,
                month: 4,
                day: 28,
            },
            true,
        ),
        (
            Date {
                year: 2156,
                month: 4,
                day: 29,
            },
            true,
        ),
        (
            Date {
                year: 2156,
                month: 5,
                day: 1,
            },
            true,
        ),
        (
            Date {
                year: 2157,
                month: 1,
                day: 1,
            },
            true,
        ),
        (
            Date {
                year: 2154,
                month: 4,
                day: 29,
            },
            false,
        ),
        (
            Date {
                year: 2284,
                month: 4,
                day: 29,
            },
            false,
        ),
        (
            Date {
                year: 2412,
                month: 4,
                day: 28,
            },
            true,
        ),
    ] {
        let mut f = Api {
            world: date,
            ..Api::default()
        };
        f.put("YEHAT_SHIP_YEAR", 1);
        f.put("YEHAT_SHIP_MONTH", 4);
        f.put("YEHAT_SHIP_DAY", 28);
        assert_eq!(
            f.ship_not_ready(),
            !ready,
            "{}-{}-{}",
            date.year,
            date.month,
            date.day
        );
        Dialogue { api: &mut f }.rebels(ANY_SHIPS);
        assert_eq!(
            f.spoken(),
            [if ready { HAVE_FEW_SHIPS } else { NO_SHIPS_YET }]
        );
        assert!(!f.enabled(ANY_SHIPS as u16));
        if !ready {
            assert!(!f
                .effects
                .iter()
                .any(|e| matches!(e, Effect::AddEscorts(..))));
        }
    }
    let mut f = Api {
        world: Date {
            year: 2156,
            month: 1,
            day: 1,
        },
        ..Api::default()
    };
    f.put("YEHAT_SHIP_YEAR", 1);
    f.put("YEHAT_SHIP_MONTH", 255);
    f.put("YEHAT_SHIP_DAY", 28);
    assert!(!f.ship_not_ready()); // 1 - 255 becomes +2 in SBYTE.
}

#[test]
fn prepare_ship_preserves_month_rollover_day_cap_and_byte_year_conversion() {
    for (date, expected) in [
        (
            Date {
                year: 2155,
                month: 1,
                day: 31,
            },
            (2, 28, 0),
        ),
        (
            Date {
                year: 2155,
                month: 11,
                day: 27,
            },
            (12, 27, 0),
        ),
        (
            Date {
                year: 2155,
                month: 12,
                day: 30,
            },
            (1, 28, 1),
        ),
        (
            Date {
                year: 2154,
                month: 1,
                day: 1,
            },
            (2, 1, 255),
        ),
        (
            Date {
                year: 2154,
                month: 12,
                day: 1,
            },
            (1, 1, 0),
        ),
        (
            Date {
                year: 2411,
                month: 1,
                day: 1,
            },
            (2, 1, 0),
        ),
    ] {
        let mut f = Api {
            world: date,
            ..Api::default()
        };
        f.prepare_ship();
        assert_eq!(
            f.effects,
            [
                Effect::Set("YEHAT_SHIP_MONTH".into(), expected.0),
                Effect::Set("YEHAT_SHIP_DAY".into(), expected.1),
                Effect::Set("YEHAT_SHIP_YEAR".into(), expected.2),
            ]
        );
    }
}

#[test]
fn ship_capacity_signed_conversion_and_grant_effect_order() {
    for (capacity, phrase, count) in [
        (0, NO_ROOM, None),
        (1, HAVE_FEW_SHIPS, Some(1)),
        (3, HAVE_FEW_SHIPS, Some(3)),
        (4, HAVE_ALL_SHIPS, Some(4)),
        (12, HAVE_ALL_SHIPS, Some(4)),
        (128, HAVE_FEW_SHIPS, Some(-128)),
        (255, HAVE_FEW_SHIPS, Some(-1)),
        (256, NO_ROOM, None),
        (257, HAVE_FEW_SHIPS, Some(1)),
    ] {
        let mut f = Api {
            feasibility: capacity,
            ..Api::default()
        };
        Dialogue { api: &mut f }.rebels(ANY_SHIPS);
        assert_eq!(f.spoken(), [phrase]);
        if let Some(count) = count {
            assert_eq!(
                &f.effects[..6],
                [
                    Effect::Npc(phrase),
                    Effect::Talk(u16::MAX),
                    Effect::AddEscorts(crate::comm::dispatch::ship::YEHAT_SHIP as u16, count),
                    Effect::Set("YEHAT_SHIP_MONTH".into(), 2),
                    Effect::Set("YEHAT_SHIP_DAY".into(), 1),
                    Effect::Set("YEHAT_SHIP_YEAR".into(), 0),
                ]
            );
        } else {
            assert!(!f
                .effects
                .iter()
                .any(|e| matches!(e, Effect::AddEscorts(..))));
        }
        assert!(!choices(&f).contains(&ANY_SHIPS));
    }
}

#[test]
fn final_battle_cavalry_splits_every_capacity_after_byte_conversion() {
    for capacity in [0, 1, 2, 3, 7, 8, 9, 255, 256, 257] {
        let mut f = Api {
            activity: crate::comm::dispatch::IN_LAST_BATTLE | 0x100,
            feasibility: capacity,
            ..Api::default()
        };
        Dialogue { api: &mut f }.initialize();
        assert_eq!(f.effects, [Effect::Segue(0)]);
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        let n = (capacity as u8).min(8);
        assert_eq!(
            f.effects,
            [
                Effect::Segue(0),
                Effect::Npc(YEHAT_CAVALRY),
                Effect::Talk(u16::MAX),
                Effect::AddEscorts(
                    crate::comm::dispatch::ship::YEHAT_REBEL_SHIP as u16,
                    i16::from(n - (n >> 1))
                ),
                Effect::AddEscorts(
                    crate::comm::dispatch::ship::PKUNK_SHIP as u16,
                    i16::from(n >> 1)
                ),
            ]
        );
        assert_eq!(f.get("YEHAT_REBEL_VISITS"), 0);
    }
}

#[test]
fn descriptor_matches_all_animation_fields_resources_and_resolution_dependent_layout() {
    let d = descriptor(320);
    assert_eq!(
        (
            d.alien_text_baseline.x,
            d.alien_text_baseline.y,
            d.alien_text_width
        ),
        (161, 60, 149)
    );
    let wide = descriptor(640);
    assert_eq!(
        (wide.alien_text_baseline.x, wide.alien_text_width),
        (374, 362)
    );
    for (ptr, expected) in [
        (d.alien_frame_res, "comm.yehat.graphics"),
        (d.alien_font_res, "comm.yehat.font"),
        (d.alien_colormap_res, "comm.yehat.colortable"),
        (d.alien_song_res, "comm.yehat.music"),
        (d.alien_alt_song_res, "comm.yehat.rebel.music"),
        (d.conversation_phrases_res, "comm.yehat.rebel.dialogue"),
    ] {
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap(),
            expected
        );
    }
    assert_eq!(d.alien_song_flags, 1);
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
    assert_eq!(d.num_animations, 15);
    let expected = [
        (4, 3, 12, 84, 84, 840, 2520, 192),
        (7, 3, 12, 84, 84, 840, 2520, 192),
        (10, 3, 4, 42, 0, 840, 2520, 16400),
        (13, 3, 4, 42, 0, 840, 2520, 32),
        (16, 5, 4, 56, 56, 8400, 2520, 16388),
        (21, 5, 4, 56, 56, 8400, 2520, 8),
        (26, 2, 12, 56, 56, 8400, 2520, 3),
        (28, 2, 12, 56, 56, 8400, 2520, 3),
        (30, 3, 4, 28, 28, 28, 28, 0),
        (33, 3, 4, 28, 28, 28, 28, 0),
        (36, 3, 4, 28, 28, 28, 28, 0),
        (39, 3, 4, 28, 28, 28, 28, 0),
        (42, 3, 4, 28, 28, 28, 28, 0),
        (45, 3, 4, 28, 28, 28, 28, 0),
        (48, 4, 12, 28, 0, 840, 2520, 20),
        (0, 0, 0, 0, 0, 0, 0, 0),
        (1, 3, 0, 56, 0, 70, 0, 0),
    ];
    for (a, e) in d.alien_ambient_array[..15]
        .iter()
        .chain([&d.alien_transition_desc, &d.alien_talk_desc])
        .zip(expected)
    {
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
            e
        );
    }
    assert!(d.alien_number_speech.is_null());
}
