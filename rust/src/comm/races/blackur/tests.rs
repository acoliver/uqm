use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;

type Api = Fake<Handler, (u32, u32)>;
fn choices(f: &Api) -> Vec<i32> {
    f.offers().iter().map(|x| i32::from(x.0)).collect()
}

#[test]
fn pet_warning_is_emitted_once_only_when_the_pet_is_aboard() {
    for pet in [0, 1] {
        for sensed in [0, 1] {
            let mut f = Api::default();
            f.put("TALKING_PET_ON_SHIP", pet);
            f.put("KOHR_AH_SENSES_EVIL", sensed);
            Dialogue { api: &mut f }.intro();
            let mut expected = Vec::new();
            if pet != 0 && sensed == 0 {
                expected.push(SENSE_EVIL);
            }
            expected.push(HELLO_AND_DIE_1);
            assert_eq!(f.spoken(), expected);
            assert_eq!(f.get("KOHR_AH_SENSES_EVIL"), pet | sensed);
        }
    }
    let mut f = Api::default();
    Dialogue { api: &mut f }.story(0);
    assert!(f.spoken().is_empty());
    assert_eq!(choices(&f), [49, 51, 53, 55, 57]);
}

#[test]
fn every_combat_sequence_preserves_byte_postincrement_and_pleading_fallthrough() {
    for (responses, key, phrases, cap) in [
        (
            vec![THREAT_1, THREAT_2, THREAT_3, THREAT_4],
            "KOHR_AH_REASONS",
            vec![40, 42, 44, 46],
            3,
        ),
        (
            vec![PLEAD_1, PLEAD_2, PLEAD_3, PLEAD_4],
            "KOHR_AH_PLEAD",
            vec![66, 68, 72, 72],
            2,
        ),
        (
            vec![
                WHY_KILL_ALL_1,
                WHY_KILL_ALL_2,
                WHY_KILL_ALL_3,
                WHY_KILL_ALL_4,
            ],
            "KOHR_AH_REASONS",
            vec![19, 20, 21, 22],
            3,
        ),
        (
            vec![
                PLEASE_DONT_KILL_1,
                PLEASE_DONT_KILL_2,
                PLEASE_DONT_KILL_3,
                PLEASE_DONT_KILL_4,
            ],
            "KOHR_AH_PLEAD",
            vec![24, 26, 28, 30],
            3,
        ),
        (
            vec![BYE_FRENZY_1, BYE_FRENZY_2, BYE_FRENZY_3, BYE_FRENZY_4],
            "KOHR_AH_INFO",
            vec![35, 36, 37, 38],
            3,
        ),
    ] {
        for r in responses {
            for n in 0..=u8::MAX {
                let mut f = Api::default();
                f.put(key, n);
                Dialogue { api: &mut f }.handle(Handler::Combat, r);
                let spoken = phrases
                    .get(usize::from(n))
                    .copied()
                    .into_iter()
                    .collect::<Vec<_>>();
                assert_eq!(f.spoken(), spoken, "response={r}, counter={n}");
                let next = if n >= cap && n <= 3 {
                    n
                } else {
                    n.wrapping_add(1)
                };
                assert_eq!(f.get(key), next, "response={r}, counter={n}");
                assert_eq!(f.effects.first(), Some(&Effect::Segue(1)));
                assert!(f.offers().is_empty());
            }
        }
    }
}

#[test]
fn combat_information_goodbyes_and_unknown_response() {
    for (r, p, info) in [
        (GUESS_THATS_ALL, THEN_DIE, None),
        (WHAT_ARE_YOU_HOVERING_OVER, BONE_PILE, Some(1)),
        (YOU_SURE_ARE_CREEPY, YES_CREEPY, Some(2)),
        (STOP_THAT_GROSS_BLINKING, DIE_HUMAN, Some(3)),
    ] {
        let mut f = Api::default();
        Dialogue { api: &mut f }.combat(r);
        assert_eq!(f.spoken(), [p]);
        assert_eq!(f.get("KOHR_AH_INFO"), info.unwrap_or(0));
    }
    for n in [0, 1, 255] {
        let mut f = Api::default();
        f.put("KOHR_AH_BYES", n);
        Dialogue { api: &mut f }.combat(BYE);
        assert_eq!(
            f.spoken(),
            [if n == 0 { GOODBYE_AND_DIE } else { DIE_HUMAN }]
        );
        assert_eq!(f.get("KOHR_AH_BYES"), 1);
    }
    let mut f = Api::default();
    Dialogue { api: &mut f }.combat(0);
    assert_eq!(f.effects, [Effect::Segue(1)]);
}

#[test]
fn hostile_and_frenzy_choices_preserve_order_and_callbacks() {
    for n in 0..=u8::MAX {
        let mut f = Api::default();
        for k in ["KOHR_AH_REASONS", "KOHR_AH_PLEAD", "KOHR_AH_INFO"] {
            f.put(k, n);
        }
        f.put("KNOW_KOHR_AH_STORY", 1);
        Dialogue { api: &mut f }.die_human(0);
        let mut expected = Vec::new();
        if n < 4 {
            expected.push(39 + i32::from(n) * 2);
        }
        expected.push(KEY_PHRASE);
        if n < 3 {
            expected.push(59 + i32::from(n) * 2);
        }
        if n < 4 {
            expected.push([65, 67, 71, 71][usize::from(n)]);
        }
        expected.push(BYE);
        assert_eq!(choices(&f), expected);
        for (p, h) in f.offers() {
            assert_eq!(
                h,
                if i32::from(p) == KEY_PHRASE {
                    Handler::Story
                } else {
                    Handler::Combat
                }
            );
        }
        f.effects.clear();
        Dialogue { api: &mut f }.frenzy(0);
        assert_eq!(
            choices(&f),
            if n < 4 {
                vec![15 + i32::from(n), 23 + i32::from(n) * 2, 31 + i32::from(n)]
            } else {
                vec![]
            }
        );
        assert!(f.offers().iter().all(|(_, h)| *h == Handler::Combat));
    }
    for known in [0, 2, 255] {
        let mut f = Api::default();
        f.put("KNOW_KOHR_AH_STORY", known);
        Dialogue { api: &mut f }.die_human(0);
        assert!(!choices(&f).contains(&KEY_PHRASE));
    }
}

#[test]
fn story_questions_disable_individually_and_exhaust_in_order() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.handle(Handler::Story, KEY_PHRASE);
    assert_eq!(f.spoken(), [RESPONSE_TO_KEY_PHRASE]);
    assert_eq!(f.get("KNOW_KOHR_AH_STORY"), 2);
    let questions = [
        WHY_DO_YOU_DESTROY,
        RELATIONSHIP_WITH_URQUAN,
        WHAT_ABOUT_CULTURE,
        HOW_LEAVE_ME_ALONE,
    ];
    assert_eq!(choices(&f), [49, 51, 53, 55, 57]);
    for (i, p) in [WE_WERE_SLAVES, WE_ARE_URQUAN_TOO, BONE_GARDENS, YOU_DIE]
        .into_iter()
        .enumerate()
    {
        f.effects.clear();
        Dialogue { api: &mut f }.handle(Handler::Story, questions[i]);
        assert_eq!(f.spoken(), [p]);
        let mut expected = questions[i + 1..].to_vec();
        expected.push(GUESS_THATS_ALL);
        assert_eq!(choices(&f), expected);
        assert_eq!(
            f.offers().last(),
            Some(&(GUESS_THATS_ALL as u16, Handler::Combat))
        );
        assert!(!f.effects.contains(&Effect::Segue(1)));
    }
}

#[test]
fn intro_priorities_activity_low_byte_and_samatra_group_match() {
    for (activity, killed, expected) in [
        (crate::comm::dispatch::WON_LAST_BATTLE | 0x100, 1, OUT_TAKES),
        (0, 1, GAME_OVER_DUDE),
    ] {
        let mut f = Api {
            activity,
            ..Api::default()
        };
        f.put("KOHR_AH_KILLED_ALL", killed);
        f.put("TALKING_PET_ON_SHIP", 1);
        Dialogue { api: &mut f }.initialize();
        assert_eq!(f.segue, 0);
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.effects, [Effect::Npc(expected), Effect::Segue(0)]);
    }
    for (group, samatra, activity, matches) in [
        (0, 0, crate::comm::dispatch::IN_INTERPLANETARY, false),
        (1, 2, crate::comm::dispatch::IN_INTERPLANETARY, false),
        (
            0x12345678,
            0x12345678,
            crate::comm::dispatch::IN_INTERPLANETARY | 0x100,
            true,
        ),
        (1, 1, crate::comm::dispatch::IN_HYPERSPACE, false),
    ] {
        let mut f = Api {
            activity,
            world: (group, samatra),
            ..Api::default()
        };
        f.put("TALKING_PET_ON_SHIP", 1);
        Dialogue { api: &mut f }.initialize();
        assert_eq!(f.segue, 1);
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(
            f.spoken(),
            [
                SENSE_EVIL,
                if matches {
                    HELLO_SAMATRA
                } else {
                    HELLO_AND_DIE_1
                }
            ]
        );
        assert_eq!(f.get("KOHR_AH_SENSES_EVIL"), 1);
        assert_eq!(f.get("AWARE_OF_SAMATRA"), u8::from(matches));
        assert_eq!(f.get("KOHR_AH_VISITS"), u8::from(!matches));
        if matches {
            assert!(f.offers().is_empty());
        }
    }
}

#[test]
fn all_greeting_counter_values_and_pet_repeat() {
    for frenzy in [0, 1] {
        for n in 0..=u8::MAX {
            let mut f = Api::default();
            f.put("KOHR_AH_VISITS", n);
            f.put("KOHR_AH_FRENZY", frenzy);
            f.put("TALKING_PET_ON_SHIP", 1);
            f.put("KOHR_AH_SENSES_EVIL", 1);
            Dialogue { api: &mut f }.intro();
            assert_eq!(
                f.spoken(),
                if n < 4 {
                    vec![if frenzy == 0 { 2 } else { 11 } + i32::from(n)]
                } else {
                    vec![]
                }
            );
            let next = if n == 3 { 3 } else { n.wrapping_add(1) };
            assert_eq!(
                f.effects.last(),
                Some(&Effect::Set("KOHR_AH_VISITS".into(), next))
            );
        }
    }
}

#[test]
fn descriptor_matches_resources_layout_and_every_animation_field() {
    let d = descriptor(320);
    assert_eq!(
        (
            d.alien_text_baseline.x,
            d.alien_text_baseline.y,
            d.alien_text_width
        ),
        (121, 0, 224)
    );
    let wide = descriptor(640);
    assert_eq!(
        (wide.alien_text_baseline.x, wide.alien_text_width),
        (281, 544)
    );
    for (ptr, expected) in [
        (d.alien_frame_res, "comm.kohrah.graphics"),
        (d.alien_font_res, "comm.kohrah.font"),
        (d.alien_colormap_res, "comm.kohrah.colortable"),
        (d.alien_song_res, "comm.kohrah.music"),
        (d.conversation_phrases_res, "comm.kohrah.dialogue"),
    ] {
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap(),
            expected
        );
    }
    assert!(d.alien_alt_song_res.is_null());
    assert_eq!(d.alien_song_flags, 0);
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
    assert_eq!(d.num_animations, 8);
    let expected = [
        (7, 6, 2, 56, 0, 840, 2520, 0),
        (13, 7, 2, 56, 0, 840, 2520, 0),
        (20, 3, 4, 56, 0, 840, 2520, 0),
        (23, 3, 4, 56, 0, 840, 2520, 0),
        (26, 3, 4, 56, 0, 840, 2520, 0),
        (29, 4, 1, 84, 0, 84, 0, 0),
        (33, 5, 10, 84, 0, 840, 2520, 0),
        (38, 4, 2, 84, 0, 840, 2520, 0),
        (1, 2, 0, 140, 0, 0, 0, 0),
        (2, 5, 0, 56, 0, 70, 0, 0),
    ];
    for (a, e) in d.alien_ambient_array[..8]
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
