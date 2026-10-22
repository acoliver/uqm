use super::super::encounter::testing::{Effect, Fake};
use super::super::script::{GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
use super::*;

#[test]
fn offered_callbacks_reach_the_recall_code_from_a_new_game() {
    let mut api = Fake::<Handler>::default();
    api.put("PROBE_EXHIBITED_BUG", 1);
    Dialogue { api: &mut api }.intro();
    for phrase in [
        WHO_ARE_YOU,
        LIKE_MORE_ABOUT_YOU,
        WHAT_ABOUT_HOME,
        WHAT_ABOUT_CULTURE,
        WHAT_ABOUT_HISTORY,
        WHAT_ABOUT_BIOLOGY,
        ENOUGH_INFO,
        WE_ARE_US0,
        WHAT_OTHER_VISITORS,
        WHAT_ARE_PROBES,
        WHATS_PROBES_MISSION,
        IF_ONLY_ONE,
        WHY_PROBE_ALWAYS_ATTACK,
        TALK_MORE_PROBE_ATTACK,
        TELL_ME_ABOUT_BASICS,
        TELL_ME_ABOUT_REP_1,
        WHAT_SET_PRIORITY,
        WHAT_EFFECT,
        PROBE_HAS_BUG,
        TELL_ME_ABOUT_REP_2,
        WHAT_ABOUT_REP_PRIORITIES,
        THINK_ABOUT_REP_PRIORITIES,
        HUNT_THEM_DOWN,
        SUE_MELNORME,
        RECALL_SIGNAL,
        MEGA_SELF_DESTRUCT,
        BYE,
    ] {
        let handler = api
            .offers()
            .into_iter()
            .find(|(p, _)| u32::from(*p) == phrase)
            .unwrap_or_else(|| panic!("phrase {phrase} missing from {:?}", api.offers()))
            .1;
        api.effects.clear();
        Dialogue { api: &mut api }.handle(handler, phrase as i32);
    }
    assert_eq!(api.state["DESTRUCT_CODE_ON_SHIP"], 1);
    assert_eq!(api.state["SLYLANDRO_KNOW_BROKEN"], 1);
    assert_eq!(api.segue, 0);
}

#[test]
fn probe_function_menu_covers_all_knowledge_and_stack_gates() {
    for stack8 in 0..=4 {
        for stack9 in 0..=4 {
            for mask in 0..32 {
                let mut api = Fake::<Handler>::default();
                api.put("SLYLANDRO_STACK8", stack8);
                api.put("SLYLANDRO_STACK9", stack9);
                api.put("PLAYER_KNOWS_PROGRAM", u8::from(mask & 1 != 0));
                api.put("PLAYER_KNOWS_PRIORITY", u8::from(mask & 2 != 0));
                api.put("PLAYER_KNOWS_EFFECTS", u8::from(mask & 4 != 0));
                if mask & 8 != 0 {
                    api.disabled.insert(WHAT_SET_PRIORITY as u16);
                }
                if mask & 16 != 0 {
                    api.disabled.insert(WHAT_EFFECT as u16);
                    api.disabled.insert(TELL_BASICS_AGAIN as u16);
                }
                Dialogue { api: &mut api }.function(0);
                let mut expected = Vec::new();
                if mask & 1 == 0 {
                    expected.push((TELL_ME_ABOUT_BASICS as u16, Handler::Function));
                } else {
                    if mask & 2 != 0 {
                        if mask & 4 != 0 {
                            expected.push((PROBE_HAS_BUG as u16, Handler::Bug));
                        }
                        if mask & 16 == 0 {
                            expected.push((WHAT_EFFECT as u16, Handler::Function));
                        }
                    }
                    if mask & 16 == 0 {
                        expected.push((TELL_BASICS_AGAIN as u16, Handler::Function));
                    }
                }
                if stack9 < 3 {
                    expected.push((
                        [
                            HOW_DOES_PROBE_DEFEND,
                            COMBAT_BEHAVIOR,
                            WHAT_MISSILE_BATTERIES,
                        ][stack9 as usize] as u16,
                        Handler::Function,
                    ));
                }
                if stack8 == 2 {
                    expected.push((TELL_ME_ABOUT_REP_1 as u16, Handler::Function));
                }
                if stack8 == 3 && mask & 8 == 0 {
                    expected.push((WHAT_SET_PRIORITY as u16, Handler::Function));
                }
                expected.push((ENOUGH_PROBLEM as u16, Handler::Probe));
                assert_eq!(api.offers(), expected);
            }
        }
    }
}

#[test]
fn probe_attack_gate_requires_an_observed_bug_and_unknown_solution() {
    for broken in 0..=1 {
        for observed in 0..=1 {
            for stack6 in 0..=2 {
                let mut api = Fake::<Handler>::default();
                api.put("SLYLANDRO_KNOW_BROKEN", broken);
                api.put("PROBE_EXHIBITED_BUG", observed);
                api.put("SLYLANDRO_STACK6", stack6);
                api.put("SLYLANDRO_STACK7", 3);
                api.put("SLYLANDRO_STACK8", 2);
                Dialogue { api: &mut api }.probe(0);
                let mut expected = Vec::new();
                if broken == 0 && observed != 0 && stack6 < 2 {
                    expected
                        .push([WHY_PROBE_ALWAYS_ATTACK, TALK_MORE_PROBE_ATTACK][stack6 as usize]);
                }
                let exhausted = expected.is_empty();
                expected.push(ENOUGH_PROBE);
                assert_eq!(choices(&api), expected);
                assert_eq!(api.disabled.contains(&(KNOW_MORE_PROBE as u16)), exhausted);
            }
        }
    }
}

fn choices(api: &Fake<Handler>) -> Vec<u32> {
    api.offers()
        .into_iter()
        .map(|(p, _)| u32::from(p))
        .collect()
}

#[test]
fn greetings_recall_and_goodbyes_preserve_byte_counters() {
    for n in 0..=u8::MAX {
        let mut api = Fake::<Handler>::default();
        api.put("SLYLANDRO_HOME_VISITS", n);
        Dialogue { api: &mut api }.initialize();
        Dialogue { api: &mut api }.intro();
        assert_eq!(
            api.spoken(),
            if n <= 3 {
                vec![i32::from(n) + 1]
            } else {
                vec![]
            }
        );
        assert_eq!(
            api.state["SLYLANDRO_HOME_VISITS"],
            if n == 3 { 3 } else { n.wrapping_add(1) }
        );
        api.effects.clear();
        Dialogue { api: &mut api }.handle(Handler::Exit, BYE as i32);
        assert_eq!(
            api.spoken(),
            [if api.state["SLYLANDRO_HOME_VISITS"] == 1 {
                GOODBYE_1
            } else {
                GOODBYE_2
            } as i32]
        );
        assert_eq!(api.segue, 0);
    }
    let mut api = Fake::<Handler>::default();
    api.put("SLYLANDRO_KNOW_BROKEN", 1);
    Dialogue { api: &mut api }.intro();
    assert_eq!(api.spoken(), [RECALL_PROGRAM_1 as i32]);
    assert_eq!(api.state["RECALL_VISITS"], 1);
    assert!(!api.state.contains_key("SLYLANDRO_HOME_VISITS"));
    api.effects.clear();
    Dialogue { api: &mut api }.intro();
    assert_eq!(api.spoken(), [HELLO_1 as i32]);
}

#[test]
fn home_answers_set_stacks_and_prioritize_the_active_topic() {
    for (r, line, key, value, first) in [
        (
            WE_ARE_US0,
            TERRIBLY_EXCITING,
            "SLYLANDRO_STACK1",
            1,
            WHAT_OTHER_VISITORS,
        ),
        (
            WHAT_OTHER_VISITORS,
            VISITORS,
            "SLYLANDRO_STACK1",
            2,
            ANY_OTHER_VISITORS,
        ),
        (
            ANY_OTHER_VISITORS,
            LONG_AGO,
            "SLYLANDRO_STACK1",
            3,
            WHAT_ABOUT_SENTIENT_MILIEU,
        ),
        (
            WHAT_ABOUT_SENTIENT_MILIEU,
            MET_TAALO_THEY_ARE_FROM,
            "SLYLANDRO_STACK1",
            4,
            WHO_ELSE,
        ),
        (WHO_ELSE, PRECURSORS, "SLYLANDRO_STACK1", 5, PRECURSORS_YOW),
        (
            PRECURSORS_YOW,
            ABOUT_PRECURSORS,
            "SLYLANDRO_STACK1",
            6,
            MUST_KNOW_MORE,
        ),
        (
            MUST_KNOW_MORE,
            ALL_WE_KNOW,
            "SLYLANDRO_STACK1",
            7,
            WHO_ARE_YOU,
        ),
        (
            WHO_ARE_YOU,
            WE_ARE_SLY,
            "SLYLANDRO_STACK2",
            1,
            LIKE_MORE_ABOUT_YOU,
        ),
        (
            WHERE_ARE_YOU,
            DOWN_HERE,
            "SLYLANDRO_STACK3",
            1,
            THATS_IMPOSSIBLE_1,
        ),
        (
            THATS_IMPOSSIBLE_1,
            NO_ITS_NOT_1,
            "SLYLANDRO_STACK3",
            2,
            THATS_IMPOSSIBLE_2,
        ),
        (
            THATS_IMPOSSIBLE_2,
            NO_ITS_NOT_2,
            "SLYLANDRO_STACK3",
            3,
            WE_ARE_US0,
        ),
    ] {
        let mut api = Fake::<Handler>::default();
        Dialogue { api: &mut api }.handle(Handler::Home, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(api.state[key], value);
        assert_eq!(choices(&api)[0], first, "response {r}");
        assert!(api
            .offers()
            .iter()
            .filter(|(p, _)| [WHO_ARE_YOU as u16, LIKE_MORE_ABOUT_YOU as u16].contains(p))
            .all(|(_, h)| *h == Handler::Home));
        if r == WHAT_OTHER_VISITORS {
            assert_eq!(api.state["PLAYER_KNOWS_PROBE"], 1);
        }
        if r == WE_ARE_US0 {
            assert!(api.disabled.contains(&(WE_ARE_US0 as u16)));
        }
    }
    for (r, line) in [
        (ENOUGH_ABOUT_ME, OK_ENOUGH_YOU),
        (ENOUGH_INFO, OK_ENOUGH_INFO),
        (ENOUGH_PROBE, OK_ENOUGH_PROBE),
    ] {
        let mut api = Fake::<Handler>::default();
        Dialogue { api: &mut api }.home(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
    }
}

#[test]
fn home_menu_obeys_each_stack_and_constructs_the_introduction() {
    let mut api = Fake::<Handler>::default();
    Dialogue { api: &mut api }.home(0);
    assert_eq!(choices(&api), [WE_ARE_US0, WHO_ARE_YOU, WHERE_ARE_YOU, BYE]);
    assert!(api.effects.contains(&Effect::Offer(
        WE_ARE_US0 as u16,
        Handler::Home,
        vec![
            WE_ARE_US0 as i32,
            GLOBAL_PLAYER_NAME,
            WE_ARE_US1 as i32,
            GLOBAL_SHIP_NAME,
            WE_ARE_US2 as i32
        ]
    )));
    for (s4, human) in [
        (0, Some(HAPPY_TO_TELL_MORE)),
        (1, Some(WOULD_YOU_LIKE_TO_KNOW_MORE)),
        (2, None),
    ] {
        for (s5, probe) in [
            (0, Some(WHAT_ARE_PROBES)),
            (1, Some(KNOW_MORE_PROBE)),
            (2, None),
        ] {
            api.effects.clear();
            api.put("SLYLANDRO_STACK1", 7);
            api.put("SLYLANDRO_STACK2", 2);
            api.put("SLYLANDRO_STACK3", 3);
            api.put("SLYLANDRO_STACK4", s4);
            api.put("SLYLANDRO_STACK5", s5);
            api.put("PLAYER_KNOWS_PROBE", 1);
            Dialogue { api: &mut api }.home(0);
            assert_eq!(
                choices(&api),
                human
                    .into_iter()
                    .chain(probe)
                    .chain([BYE])
                    .collect::<Vec<_>>()
            );
        }
    }
    api.effects.clear();
    api.disabled.insert(KNOW_MORE_PROBE as u16);
    api.put("SLYLANDRO_STACK5", 1);
    Dialogue { api: &mut api }.home(0);
    assert_eq!(choices(&api), [BYE]);
}

#[test]
fn human_information_branches_and_exhaustion() {
    for (r, line, key, value) in [
        (HAPPY_TO_TELL_MORE, TELL_MORE, "SLYLANDRO_STACK4", 1),
        (WE_COME_FROM_EARTH, OK_EARTH, "SLYLANDRO_KNOW_EARTH", 1),
        (WE_EXPLORE, OK_EXPLORE, "SLYLANDRO_KNOW_EXPLORE", 1),
        (
            WE_FIGHT_URQUAN,
            URQUAN_NICE_GUYS,
            "SLYLANDRO_KNOW_URQUAN",
            1,
        ),
        (
            NOT_SAME_URQUAN,
            PERSONALITY_CHANGE,
            "SLYLANDRO_KNOW_URQUAN",
            2,
        ),
        (WE_GATHER, MAYBE_INTERESTED, "SLYLANDRO_KNOW_GATHER", 1),
    ] {
        let mut api = Fake::<Handler>::default();
        Dialogue { api: &mut api }.handle(Handler::Human, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(api.state[key], value);
        if r == WE_FIGHT_URQUAN {
            assert_eq!(choices(&api)[0], NOT_SAME_URQUAN);
        }
        assert_eq!(*choices(&api).last().unwrap(), ENOUGH_ABOUT_ME);
    }
    let mut api = Fake::<Handler>::default();
    for key in [
        "SLYLANDRO_KNOW_EARTH",
        "SLYLANDRO_KNOW_EXPLORE",
        "SLYLANDRO_KNOW_GATHER",
    ] {
        api.put(key, 1);
    }
    api.put("SLYLANDRO_KNOW_URQUAN", 2);
    Dialogue { api: &mut api }.human(WOULD_YOU_LIKE_TO_KNOW_MORE as i32);
    assert_eq!(api.spoken(), [YES_TELL_MORE as i32]);
    assert_eq!(choices(&api), [ENOUGH_ABOUT_ME]);
    assert_eq!(api.state["SLYLANDRO_STACK4"], 2);
}

#[test]
fn alien_topics_disable_individually_and_close_when_exhausted() {
    let mut api = Fake::<Handler>::default();
    Dialogue { api: &mut api }.handle(Handler::Home, LIKE_MORE_ABOUT_YOU as i32);
    assert_eq!(api.spoken(), [SURE_KNOW_WHAT as i32]);
    for (r, line) in [
        (WHAT_ABOUT_HOME, ABOUT_HOME),
        (WHAT_ABOUT_CULTURE, ABOUT_CULTURE),
        (WHAT_ABOUT_HISTORY, ABOUT_HISTORY),
        (WHAT_ABOUT_BIOLOGY, ABOUT_BIOLOGY),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.handle(Handler::Alien, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert!(api.disabled.contains(&(r as u16)));
    }
    assert_eq!(choices(&api), [ENOUGH_INFO]);
    assert!(api.disabled.contains(&(LIKE_MORE_ABOUT_YOU as u16)));
}

#[test]
fn probe_topics_include_attack_in_c_stack_order() {
    for (r, line, key, value, expected) in [
        (
            WHAT_ARE_PROBES,
            PROBES_ARE,
            "SLYLANDRO_STACK5",
            1,
            vec![
                WHY_PROBE_ALWAYS_ATTACK,
                WHERE_PROBES_FROM,
                WHATS_PROBES_MISSION,
            ],
        ),
        (
            WHY_PROBE_ALWAYS_ATTACK,
            ONLY_DEFEND,
            "SLYLANDRO_STACK6",
            1,
            vec![
                TALK_MORE_PROBE_ATTACK,
                WHERE_PROBES_FROM,
                WHATS_PROBES_MISSION,
            ],
        ),
        (
            WHERE_PROBES_FROM,
            PROBES_FROM_MELNORME,
            "SLYLANDRO_STACK7",
            1,
            vec![WHY_SELL, WHY_PROBE_ALWAYS_ATTACK, WHATS_PROBES_MISSION],
        ),
        (
            WHY_SELL,
            SELL_FOR_INFO,
            "SLYLANDRO_STACK7",
            2,
            vec![HOW_LONG_AGO, WHY_PROBE_ALWAYS_ATTACK, WHATS_PROBES_MISSION],
        ),
        (
            HOW_LONG_AGO,
            FIFTY_THOUSAND_ROTATIONS,
            "SLYLANDRO_STACK7",
            3,
            vec![WHY_PROBE_ALWAYS_ATTACK, WHATS_PROBES_MISSION],
        ),
        (
            WHATS_PROBES_MISSION,
            SEEK_OUT_NEW_LIFE,
            "SLYLANDRO_STACK8",
            1,
            vec![IF_ONLY_ONE, WHY_PROBE_ALWAYS_ATTACK, WHERE_PROBES_FROM],
        ),
        (
            IF_ONLY_ONE,
            THEY_REPLICATE,
            "SLYLANDRO_STACK8",
            2,
            vec![WHY_PROBE_ALWAYS_ATTACK, WHERE_PROBES_FROM],
        ),
    ] {
        let mut api = Fake::<Handler>::default();
        api.put("PROBE_EXHIBITED_BUG", 1);
        Dialogue { api: &mut api }.handle(Handler::Probe, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(api.state[key], value);
        assert_eq!(
            choices(&api),
            expected
                .into_iter()
                .chain([ENOUGH_PROBE])
                .collect::<Vec<_>>()
        );
    }
    for r in [KNOW_MORE_PROBE, ENOUGH_PROBLEM] {
        let mut api = Fake::<Handler>::default();
        api.put("SLYLANDRO_STACK7", 3);
        api.put("SLYLANDRO_STACK8", 2);
        Dialogue { api: &mut api }.probe(r as i32);
        assert_eq!(
            api.spoken(),
            [if r == KNOW_MORE_PROBE {
                OK_WHAT_MORE_PROBE
            } else {
                OK_ENOUGH_PROBLEM
            } as i32]
        );
        assert_eq!(choices(&api), [ENOUGH_PROBE]);
        assert!(api.disabled.contains(&(KNOW_MORE_PROBE as u16)));
    }
}

#[test]
fn probe_function_exposes_both_progression_stacks_and_prioritizes_answers() {
    let mut api = Fake::<Handler>::default();
    api.put("SLYLANDRO_STACK8", 2);
    Dialogue { api: &mut api }.probe(TALK_MORE_PROBE_ATTACK as i32);
    assert_eq!(api.spoken(), [NO_PROBLEM_BUT_SURE as i32]);
    assert_eq!(
        choices(&api),
        [
            TELL_ME_ABOUT_BASICS,
            HOW_DOES_PROBE_DEFEND,
            TELL_ME_ABOUT_REP_1,
            ENOUGH_PROBLEM
        ]
    );
    for (r, line, first) in [
        (TELL_ME_ABOUT_REP_1, ABOUT_REP, WHAT_SET_PRIORITY),
        (HOW_DOES_PROBE_DEFEND, ONLY_SELF_DEFENSE, COMBAT_BEHAVIOR),
        (COMBAT_BEHAVIOR, MISSILE_BATTERIES, WHAT_MISSILE_BATTERIES),
        (
            WHAT_MISSILE_BATTERIES,
            LIGHTNING_ONLY_FOR_HARVESTING,
            TELL_ME_ABOUT_BASICS,
        ),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.handle(Handler::Function, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(choices(&api)[0], first);
    }
    for (r, line, key) in [
        (TELL_ME_ABOUT_BASICS, BASIC_COMMANDS, "PLAYER_KNOWS_PROGRAM"),
        (WHAT_SET_PRIORITY, MAXIMUM, "PLAYER_KNOWS_PRIORITY"),
        (WHAT_EFFECT, AFFECTS_BEHAVIOR, "PLAYER_KNOWS_EFFECTS"),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.function(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(api.state[key], 1);
    }
    assert_eq!(choices(&api), [PROBE_HAS_BUG, ENOUGH_PROBLEM]);
    api.effects.clear();
    Dialogue { api: &mut api }.function(TELL_BASICS_AGAIN as i32);
    assert_eq!(api.spoken(), [OK_BASICS_AGAIN as i32]);
    assert!(api.disabled.contains(&(TELL_BASICS_AGAIN as u16)));
}

#[test]
fn bug_conversation_requires_replication_answers_before_fixing() {
    let mut api = Fake::<Handler>::default();
    for (r, line, expected) in [
        (
            PROBE_HAS_BUG,
            NO_IT_DOESNT,
            vec![TELL_ME_ABOUT_REP_2, TELL_ME_ABOUT_ATTACK],
        ),
        (
            TELL_ME_ABOUT_REP_2,
            REP_NO_PROBLEM,
            vec![WHAT_ABOUT_REP_PRIORITIES, TELL_ME_ABOUT_ATTACK],
        ),
        (
            WHAT_ABOUT_REP_PRIORITIES,
            MAXIMUM_SO_WHAT,
            vec![THINK_ABOUT_REP_PRIORITIES, TELL_ME_ABOUT_ATTACK],
        ),
        (
            TELL_ME_ABOUT_ATTACK,
            ATTACK_NO_PROBLEM,
            vec![THINK_ABOUT_REP_PRIORITIES],
        ),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.handle(Handler::Bug, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(choices(&api), expected);
    }
    for (r, line) in [
        (THINK_ABOUT_REP_PRIORITIES, UH_OH),
        (HUNT_THEM_DOWN, GROW_TOO_FAST),
        (SUE_MELNORME, SIGNED_WAIVER),
        (RECALL_SIGNAL, NOT_THIS_MODEL),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.handle(Handler::Fix, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(*choices(&api).last().unwrap(), MEGA_SELF_DESTRUCT);
        if r != THINK_ABOUT_REP_PRIORITIES {
            assert!(api.disabled.contains(&(r as u16)));
        }
    }
    assert_eq!(choices(&api), [MEGA_SELF_DESTRUCT]);
}

#[test]
fn self_destruct_preserves_byte_wrapping_and_event_selector() {
    let mut api = Fake::<Handler>::default();
    api.put("SLYLANDRO_MULTIPLIER", 255);
    Dialogue { api: &mut api }.home(MEGA_SELF_DESTRUCT as i32);
    assert_eq!(api.spoken(), [WHY_YES_THERE_IS as i32]);
    assert_eq!(api.state["DESTRUCT_CODE_ON_SHIP"], 1);
    assert_eq!(api.state["SLYLANDRO_KNOW_BROKEN"], 1);
    assert_eq!(api.state["SLYLANDRO_MULTIPLIER"], 0);
    assert!(api.effects.contains(&Effect::Event(1, 0, 0, 0, 17)));
}

#[test]
fn full_descriptor_matches_c_resources_layout_and_animations() {
    let expected = [
        [0, 5, 17, 105, 525, 105, 525, 0],
        [1, 5, 4, 56, 56, 840, 2520, 0],
        [6, 5, 4, 56, 56, 840, 2520, 0],
        [11, 5, 4, 56, 56, 840, 2520, 0],
        [16, 6, 4, 56, 56, 840, 2520, 0],
        [22, 8, 2, 56, 0, 56, 0, 0],
        [30, 9, 2, 56, 56, 840, 2520, 768],
        [39, 4, 2, 56, 0, 840, 2520, 0],
        [43, 5, 2, 56, 0, 840, 2520, 64],
        [48, 6, 2, 56, 0, 840, 2520, 64],
        [54, 6, 2, 56, 0, 840, 2520, 4096],
        [60, 7, 2, 56, 0, 840, 2520, 4096],
        [67, 8, 2, 56, 0, 840, 2520, 3072],
    ];
    for width in [320, 640, 321] {
        let d = descriptor(width);
        assert_eq!(d.num_animations as usize, expected.len());
        assert_eq!(
            (
                d.alien_text_baseline.x,
                d.alien_text_baseline.y,
                d.alien_text_width
            ),
            ((1 + ((width - 80) >> 1)) as i16, 0, (width - 80) as u16)
        );
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
        let row = |a: crate::comm::locdata::CAnimationDesc| {
            [
                u32::from(a.start_index),
                u32::from(a.num_frames),
                u32::from(a.anim_flags),
                u32::from(a.base_frame_rate),
                u32::from(a.random_frame_rate),
                u32::from(a.base_restart_rate),
                u32::from(a.random_restart_rate),
                a.block_mask,
            ]
        };
        for (a, expected) in d.alien_ambient_array.iter().zip(expected) {
            assert_eq!(row(*a), expected);
        }
        for a in &d.alien_ambient_array[expected.len()..] {
            assert_eq!(row(*a), [0; 8]);
        }
        assert_eq!(row(d.alien_transition_desc), [0; 8]);
        assert_eq!(row(d.alien_talk_desc), [0; 8]);
        unsafe {
            for (pointer, key) in [
                (d.alien_frame_res, RACE_PMAP_ANIM),
                (d.alien_font_res, RACE_FONT),
                (d.alien_colormap_res, RACE_COLOR_MAP),
                (d.alien_song_res, RACE_MUSIC),
                (d.conversation_phrases_res, RACE_CONVERSATION_PHRASES),
            ] {
                assert_eq!(std::ffi::CStr::from_ptr(pointer).to_bytes_with_nul(), key);
            }
        }
        assert!(d.alien_number_speech.is_null());
        assert!(d.alien_alt_song_res.is_null());
        assert_eq!(d.alien_song_flags, 0);
        assert!(d.alien_frame.is_null());
        assert!(d.alien_font.is_null());
        assert!(d.alien_colormap.is_null());
        assert!(d.alien_song.is_null());
        assert!(d.conversation_phrases.is_null());
    }
}
