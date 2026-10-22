use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;

type Api = Fake<Handler, ()>;
fn choices(f: &Api) -> Vec<i32> {
    f.offers().iter().map(|x| i32::from(x.0)).collect()
}

#[test]
fn all_exit_branches_preserve_speech_state_and_segue_order() {
    for (r, p, state) in [
        (DONT_ATTACK, TYPICAL_PLOY, ("SHOFIXTI_STACK1", 1)),
        (HEY_STOP, ONLY_STOP, ("SHOFIXTI_STACK1", 2)),
        (LOOK_YOU_ARE, TOO_BAD, ("SHOFIXTI_STACK1", 3)),
        (NO_ONE_INSULTS, YOU_LIMP, ("SHOFIXTI_STACK2", 1)),
        (MIGHTY_WORDS, HANG_YOUR, ("SHOFIXTI_STACK2", 2)),
        (DONT_KNOW, NEVER, ("SHOFIXTI_STACK3", 1)),
        (LOOK0, FOR_YOU, ("SHOFIXTI_STACK3", 2)),
        (NO_BLOODSHED, YES_BLOODSHED, ("SHOFIXTI_STACK3", 3)),
    ] {
        let mut f = Api::default();
        Dialogue { api: &mut f }.handle(Handler::Exit, r);
        assert_eq!(
            f.effects,
            [
                Effect::Segue(1),
                Effect::Npc(p),
                Effect::Set(state.0.into(), state.1)
            ]
        );
    }
    for (r, p) in [
        (BYE0, GOODBYE),
        (GO_AHEAD, ON_SECOND_THOUGHT),
        (NEED_YOU_FOR_DUTY, OK_WILL_BE_SENTRY),
    ] {
        let mut f = Api::default();
        Dialogue { api: &mut f }.exit(r);
        assert_eq!(
            f.effects,
            [Effect::Segue(1), Effect::Npc(p), Effect::Segue(0)]
        );
    }
    for r in [FEMALES, NUBILES, RAT_BABES] {
        let mut f = Api::default();
        f.put("MAIDENS_ON_SHIP", 1);
        Dialogue { api: &mut f }.exit(r);
        assert_eq!(
            f.effects,
            [
                Effect::Segue(1),
                Effect::Npc(LEAPING_HAPPINESS),
                Effect::Set("SHOFIXTI_RECRUITED".into(), 1),
                Effect::Set("MAIDENS_ON_SHIP".into(), 0),
                Effect::Segue(0),
                Effect::Event(1, 2, 0, 0, 8)
            ]
        );
    }
    let mut f = Api::default();
    Dialogue { api: &mut f }.exit(0);
    assert_eq!(f.effects, [Effect::Segue(1)]);
}

#[test]
fn refusal_counter_postincrements_for_unhandled_values_and_caps_only_case_three() {
    for n in 0..=u8::MAX {
        let mut f = Api::default();
        f.put("SHOFIXTI_STACK4", n);
        Dialogue { api: &mut f }.exit(DONT_WANT_TO_FIGHT);
        assert_eq!(
            f.spoken(),
            if n < 4 {
                vec![34 + i32::from(n)]
            } else {
                vec![]
            }
        );
        assert_eq!(
            f.get("SHOFIXTI_STACK4"),
            if n == 3 { n } else { n.wrapping_add(1) }
        );
    }
}

#[test]
fn maidens_explanation_and_console_branch_choices_keep_callbacks_and_order() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.handle(Handler::GiveMaidens, IMPORTANT_DUTY);
    assert_eq!(f.spoken(), [WHAT_DUTY]);
    assert_eq!(
        choices(&f),
        [PROCREATING_WILDLY, REPLENISHING_YOUR_SPECIES, HOPE_YOU_HAVE]
    );
    assert!(f.offers().iter().all(|(_, h)| *h == Handler::GiveMaidens));
    for r in [
        PROCREATING_WILDLY,
        REPLENISHING_YOUR_SPECIES,
        HOPE_YOU_HAVE,
        0,
    ] {
        let mut f = Api::default();
        Dialogue { api: &mut f }.give_maidens(r);
        assert_eq!(f.spoken(), [SOUNDS_GREAT_BUT_HOW]);
        assert_eq!(choices(&f), [FEMALES, NUBILES, RAT_BABES]);
        assert!(f.offers().iter().all(|(_, h)| *h == Handler::Exit));
    }
    for (r, p) in [
        (I_AM_CAPTAIN0, SO_SORRY),
        (I_AM_NICE, MUST_UNDERSTAND),
        (I_AM_GUY, NICE_BUT_WHAT_IS_DONKEY),
        (0, SO_SORRY),
    ] {
        let mut f = Api::default();
        Dialogue { api: &mut f }.handle(Handler::Explain, r);
        assert_eq!(f.spoken(), [p, IS_DEFEAT_TRUE]);
        assert_eq!(choices(&f), [YES_AND_NO, CLOBBERED, BUTT_BLASTED]);
        assert!(f.offers().iter().all(|(_, h)| *h == Handler::Console));
    }
    for maidens in [0, 1] {
        for r in [YES_AND_NO, CLOBBERED, BUTT_BLASTED, DONT_DO_IT, 0] {
            let mut f = Api::default();
            f.put("MAIDENS_ON_SHIP", maidens);
            Dialogue { api: &mut f }.handle(Handler::Console, r);
            assert_eq!(
                f.spoken(),
                [if r == DONT_DO_IT {
                    YES_I_DO_IT
                } else {
                    VERY_SAD_KILL_SELF
                }]
            );
            let mut expected = Vec::new();
            if maidens != 0 {
                expected.push(IMPORTANT_DUTY);
            }
            if r != DONT_DO_IT {
                expected.push(DONT_DO_IT);
            }
            expected.extend([NEED_YOU_FOR_DUTY, GO_AHEAD]);
            assert_eq!(choices(&f), expected);
            assert_eq!(f.enabled(DONT_DO_IT as u16), r != DONT_DO_IT);
        }
    }
}

#[test]
fn reconciliation_constructs_every_alliance_name_including_commander_and_ship_fragments() {
    for (name, alliance) in [
        (0, vec![1]),
        (1, vec![2]),
        (2, vec![3]),
        (3, vec![4, GLOBAL_PLAYER_NAME, 5]),
    ] {
        let mut f = Api::default();
        f.put("NEW_ALLIANCE_NAME", name);
        f.put("SHOFIXTI_STACK1", 3);
        f.put("SHOFIXTI_STACK3", 3);
        Dialogue { api: &mut f }.handle(Handler::Realize, DONKEY_BREATH);
        let mut fragments = vec![44, GLOBAL_PLAYER_NAME, 45];
        fragments.extend(alliance);
        fragments.extend([46, GLOBAL_SHIP_NAME, 47]);
        assert_eq!(
            f.effects,
            [
                Effect::Npc(DGRUNTI),
                Effect::Set("SHOFIXTI_STACK1".into(), 0),
                Effect::Set("SHOFIXTI_STACK3".into(), 0),
                Effect::Set("SHOFIXTI_STACK2".into(), 3),
                Effect::Offer(I_AM_CAPTAIN0 as u16, Handler::Explain, fragments),
                Effect::Offer(I_AM_NICE as u16, Handler::Explain, vec![]),
                Effect::Offer(I_AM_GUY as u16, Handler::Explain, vec![])
            ]
        );
    }
}

#[test]
fn hostile_stack_choices_keep_the_original_duplicate_look_response_and_name_fragments() {
    for kia in [0, 1] {
        for a in 0..=3 {
            for b in 0..=3 {
                for c in 0..=3 {
                    let mut f = Api::default();
                    f.put("SHOFIXTI_KIA", kia);
                    f.put("SHOFIXTI_STACK1", a);
                    f.put("SHOFIXTI_STACK2", b);
                    f.put("SHOFIXTI_STACK3", c);
                    Dialogue { api: &mut f }.hostile();
                    let mut expected = Vec::new();
                    if a < 3 {
                        expected.push([DONT_ATTACK, HEY_STOP, LOOK_YOU_ARE][usize::from(a)]);
                    }
                    if b < 3 {
                        expected
                            .push([NO_ONE_INSULTS, MIGHTY_WORDS, DONKEY_BREATH][usize::from(b)]);
                    }
                    if c < 3 {
                        expected.push([DONT_KNOW, LOOK0, LOOK_YOU_ARE][usize::from(c)]);
                    }
                    expected.push(DONT_WANT_TO_FIGHT);
                    assert_eq!(choices(&f), expected);
                    for e in &f.effects {
                        if let Effect::Offer(p, h, fragments) = e {
                            assert_eq!(
                                *h,
                                if i32::from(*p) == DONKEY_BREATH {
                                    Handler::Realize
                                } else {
                                    Handler::Exit
                                }
                            );
                            assert_eq!(
                                *fragments,
                                if i32::from(*p) == LOOK0 {
                                    vec![LOOK0, if kia == 0 { TANAKA } else { KATANA }, LOOK1]
                                } else {
                                    vec![]
                                }
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn friendly_prioritizes_the_last_stack_then_offers_other_stacks_and_goodbye() {
    for (r, spoken, stack1, stack3, order) in [
        (0, None, 0, 0, vec![REPORT0, WHY_HERE0, WHERE_WORLD]),
        (
            REPORT0,
            Some(NOTHING_NEW),
            0,
            0,
            vec![WHY_HERE0, WHERE_WORLD],
        ),
        (
            WHY_HERE0,
            Some(I_GUARD),
            1,
            0,
            vec![WHAT_HAPPENED, REPORT0, WHERE_WORLD],
        ),
        (
            WHAT_HAPPENED,
            Some(MET_VUX),
            2,
            0,
            vec![GLORY_DEVICE, REPORT0, WHERE_WORLD],
        ),
        (
            GLORY_DEVICE,
            Some(SWITCH_BROKE),
            3,
            0,
            vec![REPORT0, WHERE_WORLD],
        ),
        (
            WHERE_WORLD,
            Some(BLEW_IT_UP),
            0,
            1,
            vec![HOW_SURVIVE, REPORT0, WHY_HERE0],
        ),
        (HOW_SURVIVE, Some(NOT_HERE), 0, 2, vec![REPORT0, WHY_HERE0]),
    ] {
        for kia in [0, 1] {
            for maidens in [0, 1] {
                let mut f = Api::default();
                f.put("SHOFIXTI_KIA", kia);
                f.put("MAIDENS_ON_SHIP", maidens);
                Dialogue { api: &mut f }.handle(Handler::Friendly, r);
                assert_eq!(f.spoken(), spoken.into_iter().collect::<Vec<_>>());
                assert_eq!(f.get("SHOFIXTI_STACK1"), stack1);
                assert_eq!(f.get("SHOFIXTI_STACK3"), stack3);
                let mut expected = order.clone();
                if maidens != 0 {
                    expected.push(IMPORTANT_DUTY);
                }
                expected.push(BYE0);
                assert_eq!(choices(&f), expected, "response={r}");
                for e in &f.effects {
                    if let Effect::Offer(p, h, fragments) = e {
                        let p = i32::from(*p);
                        let name = if kia == 0 { TANAKA } else { KATANA };
                        let expected = match p {
                            REPORT0 => vec![REPORT0, name, REPORT1],
                            WHY_HERE0 => vec![WHY_HERE0, name, WHY_HERE1],
                            BYE0 => vec![BYE0, name, BYE1],
                            _ => vec![],
                        };
                        assert_eq!(*fragments, expected);
                        assert_eq!(
                            *h,
                            if p == BYE0 {
                                Handler::Exit
                            } else if p == IMPORTANT_DUTY {
                                Handler::GiveMaidens
                            } else {
                                Handler::Friendly
                            }
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn exhausted_friendly_stacks_and_disabled_report_are_omitted() {
    for stack1 in [0, 1, 2, 3, 255] {
        for stack3 in [0, 1, 2, 255] {
            let mut f = Api::default();
            f.put("SHOFIXTI_STACK1", stack1);
            f.put("SHOFIXTI_STACK3", stack3);
            f.disabled.insert(REPORT0 as u16);
            Dialogue { api: &mut f }.friendly(0);
            let mut expected = Vec::new();
            if stack1 < 3 {
                expected.push([WHY_HERE0, WHAT_HAPPENED, GLORY_DEVICE][usize::from(stack1)]);
            }
            if stack3 < 2 {
                expected.push([WHERE_WORLD, HOW_SURVIVE][usize::from(stack3)]);
            }
            expected.push(BYE0);
            assert_eq!(choices(&f), expected);
        }
    }
}

#[test]
fn all_hostile_greeting_counter_values_and_identities_preserve_byte_postincrement() {
    for kia in [0, 1] {
        for n in 0..=u8::MAX {
            let mut f = Api::default();
            f.put("SHOFIXTI_KIA", kia);
            f.put("SHOFIXTI_VISITS", n);
            Dialogue { api: &mut f }.initialize();
            assert_eq!(f.effects, [Effect::Segue(0)]);
            f.effects.clear();
            Dialogue { api: &mut f }.intro();
            let cap = if kia == 0 { 7 } else { 3 };
            assert_eq!(
                f.spoken(),
                if n <= cap {
                    vec![if kia == 0 { 12 } else { 8 } + i32::from(n)]
                } else {
                    vec![]
                }
            );
            assert_eq!(
                f.get("SHOFIXTI_VISITS"),
                if n == cap { n } else { n.wrapping_add(1) }
            );
            assert_eq!(
                choices(&f),
                [DONT_ATTACK, NO_ONE_INSULTS, DONT_KNOW, DONT_WANT_TO_FIGHT]
            );
        }
    }
}

#[test]
fn intro_outtakes_short_circuits_and_friendly_hello_does_not_advance_visits() {
    let mut f = Api {
        activity: crate::comm::dispatch::WON_LAST_BATTLE | 0x100,
        ..Api::default()
    };
    f.put("SHOFIXTI_STACK2", 3);
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.effects, [Effect::Npc(OUT_TAKES), Effect::Segue(0)]);
    for kia in [0, 1] {
        for stack2 in [3, 255] {
            let mut f = Api::default();
            f.put("SHOFIXTI_KIA", kia);
            f.put("SHOFIXTI_STACK2", stack2);
            f.put("SHOFIXTI_VISITS", 6);
            Dialogue { api: &mut f }.intro();
            assert_eq!(f.spoken(), [FRIENDLY_HELLO]);
            assert_eq!(f.get("SHOFIXTI_VISITS"), 6);
            assert_eq!(choices(&f), [REPORT0, WHY_HERE0, WHERE_WORLD, BYE0]);
        }
    }
}

#[test]
fn descriptor_matches_resources_text_layout_and_all_animation_fields() {
    let d = descriptor(320);
    assert_eq!(
        (
            d.alien_text_baseline.x,
            d.alien_text_baseline.y,
            d.alien_text_width
        ),
        (121, 0, 240)
    );
    let wide = descriptor(640);
    assert_eq!(
        (wide.alien_text_baseline.x, wide.alien_text_width),
        (281, 560)
    );
    for (ptr, expected) in [
        (d.alien_frame_res, "comm.shofixti.graphics"),
        (d.alien_font_res, "comm.shofixti.font"),
        (d.alien_colormap_res, "comm.shofixti.colortable"),
        (d.alien_song_res, "comm.shofixti.music"),
        (d.conversation_phrases_res, "comm.shofixti.dialogue"),
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
    assert_eq!(d.num_animations, 11);
    let expected = [
        (5, 15, 2, 28, 0, 28, 0, 0),
        (20, 3, 1, 84, 0, 420, 1260, 0),
        (23, 3, 1, 84, 0, 420, 1260, 0),
        (26, 3, 4, 28, 0, 840, 2520, 0),
        (29, 4, 4, 56, 0, 840, 2520, 0),
        (33, 6, 2, 42, 28, 42, 28, 0),
        (39, 7, 2, 42, 28, 42, 28, 128),
        (46, 6, 2, 42, 28, 42, 28, 64),
        (52, 4, 2, 42, 28, 42, 28, 0),
        (56, 7, 2, 42, 28, 42, 28, 1024),
        (63, 6, 2, 42, 28, 42, 28, 512),
        (0, 0, 0, 0, 0, 0, 0, 0),
        (1, 4, 0, 42, 0, 56, 0, 0),
    ];
    for (a, e) in d.alien_ambient_array[..11]
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
