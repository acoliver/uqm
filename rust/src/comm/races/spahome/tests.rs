use super::super::encounter::testing::{Effect, Fake as BaseFake};
use super::super::script::{GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
use super::*;
use api::World;
type Fake = BaseFake<Handler, World>;

fn select(api: &mut Fake, phrase: u32) {
    let handler = api
        .offers()
        .into_iter()
        .find(|(p, _)| u32::from(*p) == phrase)
        .unwrap_or_else(|| panic!("phrase {phrase} missing from {:?}", api.offers()))
        .1;
    api.effects.clear();
    Dialogue { api }.handle(handler, phrase as i32);
}

#[test]
fn offered_callbacks_complete_the_quest_party_and_alliance() {
    let mut api = Fake::default();
    api.put("FOUND_PLUTO_SPATHI", 1);
    Dialogue { api: &mut api }.initialize();
    Dialogue { api: &mut api }.intro();
    for phrase in [
        GOOD_PASSWORD,
        WE_COME_IN_PEACE,
        HAND_IN_FRIENDSHIP,
        STRONGER,
        YES_WE_ARE,
        HOW_PROVE,
        WHAT_TEST,
        TELL_EVIL,
        I_ACCEPT,
    ] {
        select(&mut api, phrase);
    }
    assert_eq!(api.state["SPATHI_QUEST"], 1);
    Dialogue { api: &mut api }.post();
    api.effects.clear();
    Dialogue { api: &mut api }.intro();
    api.put("SPATHI_CREATURES_ELIMINATED", 1);
    select(&mut api, KILLED_THEM_ALL_1);
    assert_party(&api);
    api.effects.clear();
    Dialogue { api: &mut api }.intro();
    for phrase in [
        DEALS_A_DEAL,
        HOW_LONG,
        RENEGING,
        RETURN_BEASTS,
        DO_AS_WE_SAY,
    ] {
        select(&mut api, phrase);
    }
    assert_eq!(
        api.alliances[crate::comm::dispatch::ship::SPATHI_SHIP as usize],
        1
    );
    api.effects.clear();
    Dialogue { api: &mut api }.intro();
    for phrase in [
        WHATS_UP,
        LIKE_SOME_INFO,
        WHAT_ABOUT_HIERARCHY,
        ENOUGH_INFO,
        BYE_ALLY,
    ] {
        select(&mut api, phrase);
    }
    assert_eq!(api.spoken(), [GOODBYE_ALLY as i32]);
    assert_eq!(api.segue, 0);
}

#[test]
fn council_fwiffo_menu_respects_every_disabled_phrase_combination() {
    let topics = [SPATHI_ON_PLUTO, HOSTAGE, KILLED_FWIFFO, FWIFFO_FINE];
    for mask in 0..16 {
        for escort in [false, true] {
            let mut api = Fake::default();
            api.put("FOUND_PLUTO_SPATHI", 2);
            api.world.have_escort = escort;
            for (i, p) in topics.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    api.disabled.insert(*p as u16);
                }
            }
            Dialogue { api: &mut api }.council(0);
            let mut expected = vec![WE_COME_IN_PEACE];
            if mask & 1 == 0 {
                expected.push(SPATHI_ON_PLUTO);
            } else if escort {
                if mask & 2 == 0 {
                    expected.push(HOSTAGE);
                }
            } else if mask & 4 == 0 {
                expected.push(KILLED_FWIFFO);
                if mask & 8 == 0 {
                    expected.push(FWIFFO_FINE);
                }
            }
            expected.extend([SURRENDER, BYE_NO_ALLY_OFFER]);
            assert_eq!(choices(&api), expected);
        }
    }
}

fn choices(api: &Fake) -> Vec<u32> {
    api.offers()
        .into_iter()
        .map(|(p, _)| u32::from(p))
        .collect()
}

#[test]
fn initialization_and_post_encounter_use_the_live_logic_for_all_manners() {
    for manner in 0..=3 {
        let mut api = Fake::default();
        api.put("SPATHI_MANNER", manner);
        Dialogue { api: &mut api }.initialize();
        assert_eq!(api.segue, if manner == 3 { 0 } else { 1 });
        for segue in 0..=2 {
            api.effects.clear();
            api.segue = segue;
            api.put("SPATHI_MANNER", manner);
            api.put("SPATHI_HOME_VISITS", 5);
            api.put("SPATHI_VISITS", 4);
            Dialogue { api: &mut api }.post();
            let changed = segue == 1 && manner != 2;
            assert_eq!(api.state["SPATHI_MANNER"], if changed { 1 } else { manner });
            assert_eq!(
                api.state["SPATHI_HOME_VISITS"],
                if changed && manner != 1 { 0 } else { 5 }
            );
            assert_eq!(
                api.state["SPATHI_VISITS"],
                if changed && manner != 1 { 0 } else { 4 }
            );
            if !changed {
                assert!(api.effects.is_empty());
            }
        }
    }
}

#[test]
fn every_exit_speaks_the_correct_phrase_and_selects_the_correct_segue() {
    for (r, line, hostile) in [
        (WE_ATTACK_AGAIN, WE_FIGHT_AGAIN, true),
        (SURRENDER_OR_DIE, DEFEND_OURSELVES, true),
        (WE_ARE_VINDICATOR0, NO_PASSWORD, true),
        (GORT_MERENGA, WRONG_PASSWORD, true),
        (GUPH_FLORP, WRONG_PASSWORD, true),
        (WAGNGL_FTHAGN, WRONG_PASSWORD, true),
        (PLEEESE, WRONG_PASSWORD, true),
        (SCREW_PASSWORD, NO_PASSWORD, true),
        (BYE_NO_ALLY_OFFER, GOODBYE_NO_ALLY_OFFER, false),
        (BYE_ANGRY_SPATHI, GOODBYE_ANGRY_SPATHI, false),
        (BYE_ALLY, GOODBYE_ALLY, false),
        (ALREADY_GOT_THEM, EARLY_BIRD_CHECK, false),
        (TOO_DANGEROUS, WE_AGREE, false),
        (THINK_MORE, COWARD, false),
        (I_ACCEPT, AWAIT_RETURN, false),
        (DO_AS_WE_SAY, DEPART_FOR_EARTH, false),
        (KILLED_THEM_ALL_1, WILL_CHECK_1, false),
        (KILLED_THEM_ALL_2, WILL_CHECK_2, false),
        (BYE_BEFORE_PARTY, GOODBYE_BEFORE_PARTY, false),
        (BYE_FROM_PARTY_1, GOODBYE_FROM_PARTY, false),
        (BYE_FROM_PARTY_2, GOODBYE_FROM_PARTY, false),
        (BYE_FROM_PARTY_3, GOODBYE_FROM_PARTY, false),
    ] {
        let mut api = Fake::default();
        Dialogue { api: &mut api }.handle(Handler::Exit, r as i32);
        assert_eq!(api.effects[0], Effect::Segue(0));
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(api.segue, u32::from(hostile));
        assert!(api.offers().is_empty());
        match r {
            ALREADY_GOT_THEM => assert_party(&api),
            I_ACCEPT => {
                assert_eq!(api.state["SPATHI_QUEST"], 1);
                assert_eq!(api.state["SPATHI_MANNER"], 3);
                assert_eq!(api.state["SPATHI_VISITS"], 0);
            }
            DO_AS_WE_SAY => {
                assert!(api.effects.contains(&Effect::Allied(
                    crate::comm::dispatch::ship::SPATHI_SHIP as u16,
                    true
                )));
                assert!(api.effects.contains(&Effect::Event(1, 6, 0, 0, 11)));
                assert_eq!(api.state["SPATHI_HOME_VISITS"], 0);
                assert_eq!(api.state["SPATHI_VISITS"], 0);
            }
            KILLED_THEM_ALL_1 => assert_eq!(api.state["LIED_ABOUT_CREATURES"], 1),
            KILLED_THEM_ALL_2 => assert_eq!(api.state["LIED_ABOUT_CREATURES"], 2),
            _ => {}
        }
    }
    for r in [KILLED_THEM_ALL_1, KILLED_THEM_ALL_2] {
        let mut api = Fake::default();
        api.put("SPATHI_CREATURES_ELIMINATED", 1);
        Dialogue { api: &mut api }.exit(r as i32);
        assert_party(&api);
        assert!(!api.state.contains_key("LIED_ABOUT_CREATURES"));
    }
}
fn assert_party(api: &Fake) {
    assert_eq!(api.state["SPATHI_HOME_VISITS"], 0);
    assert_eq!(api.state["SPATHI_VISITS"], 0);
    assert_eq!(api.state["SPATHI_PARTY"], 1);
    assert_eq!(api.state["SPATHI_MANNER"], 3);
}

#[test]
fn password_visits_constructed_fragments_and_known_password_branches() {
    for n in 0..=255u8 {
        let mut api = Fake::default();
        api.put("SPATHI_HOME_VISITS", n);
        Dialogue { api: &mut api }.handle(Handler::Password, 0);
        assert_eq!(
            api.spoken(),
            [if n == 1 {
                WHAT_IS_PASSWORD_AGAIN
            } else {
                WHAT_IS_PASSWORD
            } as i32]
        );
        assert_eq!(api.state["SPATHI_HOME_VISITS"], 1);
        assert_eq!(
            choices(&api),
            [
                WE_ARE_VINDICATOR0,
                GORT_MERENGA,
                GUPH_FLORP,
                WAGNGL_FTHAGN,
                PLEEESE,
                SCREW_PASSWORD
            ]
        );
        assert!(api.effects.contains(&Effect::Offer(
            WE_ARE_VINDICATOR0 as u16,
            Handler::Exit,
            vec![
                WE_ARE_VINDICATOR0 as i32,
                GLOBAL_PLAYER_NAME,
                WE_ARE_VINDICATOR1 as i32,
                GLOBAL_SHIP_NAME,
                WE_ARE_VINDICATOR2 as i32
            ]
        )));
    }
    for key in ["FOUND_PLUTO_SPATHI", "KNOW_SPATHI_PASSWORD"] {
        let mut api = Fake::default();
        api.put(key, 1);
        Dialogue { api: &mut api }.password(0);
        assert_eq!(
            api.offers(),
            [
                (GOOD_PASSWORD as u16, Handler::Council),
                (WHAT_DO_I_GET as u16, Handler::Password)
            ]
        );
        api.effects.clear();
        Dialogue { api: &mut api }.password(WHAT_DO_I_GET as i32);
        assert_eq!(api.spoken(), [YOU_GET_TO_LIVE as i32]);
        assert!(api.disabled.contains(&(WHAT_DO_I_GET as u16)));
        assert_eq!(choices(&api), [GOOD_PASSWORD]);
    }
}

#[test]
fn angry_and_alliance_conversations_preserve_choices_and_palette_fade() {
    let mut api = Fake::default();
    Dialogue { api: &mut api }.handle(Handler::Angry, 0);
    assert_eq!(api.spoken(), [MEAN_GUYS_RETURN as i32]);
    assert_eq!(
        choices(&api),
        [WE_APOLOGIZE, WE_ATTACK_AGAIN, BYE_ANGRY_SPATHI]
    );
    api.effects.clear();
    Dialogue { api: &mut api }.angry(WE_APOLOGIZE as i32);
    assert_eq!(api.spoken(), [DONT_BELIEVE as i32]);
    assert_eq!(
        api.offers()[0],
        (MISUNDERSTANDING as u16, Handler::Alliance)
    );
    api.effects.clear();
    Dialogue { api: &mut api }.handle(Handler::Alliance, MISUNDERSTANDING as i32);
    assert_eq!(api.spoken(), [JUST_MISUNDERSTANDING as i32]);
    assert!(api.effects.contains(&Effect::FadePalette(1, 210)));
    assert_eq!(api.state["SPATHI_MANNER"], 3);
    assert_eq!(api.state["SPATHI_VISITS"], 0);
    assert_eq!(
        choices(&api),
        [HAND_IN_FRIENDSHIP, SHARE_INFO, GIVE_US_RESOURCES]
    );
    for (r, line, next) in [
        (WE_COME_IN_PEACE, OF_COURSE, HAND_IN_FRIENDSHIP),
        (HAND_IN_FRIENDSHIP, TOO_AFRAID, STRONGER),
        (STRONGER, YOURE_NOT, YES_WE_ARE),
        (YES_WE_ARE, NO_YOURE_NOT, HOW_PROVE),
        (SHARE_INFO, NO_INFO, HOW_PROVE),
        (GIVE_US_RESOURCES, NO_RESOURCES, HOW_PROVE),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.alliance(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(choices(&api)[0], next);
        if r != WE_COME_IN_PEACE {
            assert!(api.disabled.contains(&(r as u16)));
        }
    }
    assert_eq!(api.offers(), [(HOW_PROVE as u16, Handler::Learn)]);
}

#[test]
fn learn_quest_restores_known_topics_and_handles_all_answers() {
    for evil in 0..=1 {
        let mut api = Fake::default();
        api.put("KNOW_SPATHI_QUEST", 1);
        api.put("KNOW_SPATHI_EVIL", evil);
        Dialogue { api: &mut api }.handle(Handler::Learn, 0);
        assert_eq!(api.spoken(), [QUEST_AGAIN as i32]);
        assert!(api.disabled.contains(&(WHAT_TEST as u16)));
        assert_eq!(api.disabled.contains(&(TELL_EVIL as u16)), evil != 0);
        assert_eq!(
            choices(&api),
            if evil == 0 {
                vec![TOO_DANGEROUS, TELL_EVIL, WHY_DONT_YOU_DO_IT]
            } else {
                vec![TOO_DANGEROUS, THINK_MORE, I_ACCEPT, WHY_DONT_YOU_DO_IT]
            }
        );
    }
    let mut api = Fake::default();
    for (r, line) in [
        (HOW_PROVE, BETTER_IDEA),
        (PROVE_STRENGTH, YOUR_BEHAVIOR),
        (WHAT_TEST, WIPE_EVIL),
        (TELL_EVIL, BEFORE_ACCEPT),
        (WHY_DONT_YOU_DO_IT, WE_WONT_BECAUSE),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.learn(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        if r != HOW_PROVE {
            assert!(api.disabled.contains(&(r as u16)));
        }
    }
    assert_eq!(api.state["KNOW_SPATHI_QUEST"], 1);
    assert_eq!(api.state["KNOW_SPATHI_EVIL"], 1);
    assert_eq!(choices(&api), [TOO_DANGEROUS, THINK_MORE, I_ACCEPT]);
    api.effects.clear();
    api.put("SPATHI_CREATURES_ELIMINATED", 1);
    Dialogue { api: &mut api }.learn(0);
    assert_eq!(choices(&api), [ALREADY_GOT_THEM]);
}

#[test]
fn quest_claims_and_optional_replies_follow_creature_state() {
    for lied in 0..=1 {
        for eliminated in 0..=1 {
            for examined in 0..=1 {
                let mut api = Fake::default();
                api.put("LIED_ABOUT_CREATURES", lied);
                api.put("SPATHI_CREATURES_ELIMINATED", eliminated);
                api.put("SPATHI_CREATURES_EXAMINED", examined);
                Dialogue { api: &mut api }.handle(Handler::Quest, 0);
                assert_eq!(
                    api.spoken(),
                    [if lied == 0 {
                        HOW_GO_EFFORTS
                    } else {
                        YOU_LIED_1
                    } as i32]
                );
                let mut expected = if lied == 0 {
                    vec![KILLED_THEM_ALL_1]
                } else {
                    vec![LITTLE_MISTAKE, KILLED_THEM_ALL_2]
                };
                if eliminated == 0 {
                    expected.push(TALK_TEST);
                    if examined != 0 {
                        expected.push(ZAPPED_A_FEW);
                    }
                    expected.push(BYE_BEFORE_PARTY);
                }
                assert_eq!(choices(&api), expected);
            }
        }
    }
    for (r, line) in [
        (LITTLE_MISTAKE, BIG_MISTAKE),
        (TALK_TEST, TEST_AGAIN),
        (ZAPPED_A_FEW, YOU_FORTUNATE),
    ] {
        let mut api = Fake::default();
        Dialogue { api: &mut api }.quest(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert!(api.disabled.contains(&(r as u16)));
    }
}

#[test]
fn council_answers_and_escort_dependent_fwiffo_choices() {
    for (r, line) in [
        (GOOD_PASSWORD, YES_GOOD_PASSWORD),
        (WE_COME_IN_PEACE, KILLED_SPATHI),
        (SPATHI_ON_PLUTO, WHERE_SPATHI),
        (HOSTAGE, GUN_TO_HEAD),
        (KILLED_FWIFFO, POOR_FWIFFO),
        (FWIFFO_FINE, NOT_LIKELY),
        (SURRENDER, NO_SURRENDER),
    ] {
        let mut api = Fake::default();
        api.put("FOUND_PLUTO_SPATHI", 1);
        Dialogue { api: &mut api }.handle(Handler::Council, r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        if r == GOOD_PASSWORD {
            assert!(api.effects.contains(&Effect::FadePalette(1, 210)));
            assert_eq!(api.state["KNOW_SPATHI_PASSWORD"], 1);
            assert_eq!(api.state["SPATHI_HOME_VISITS"], 0);
        } else {
            assert!(api.disabled.contains(&(r as u16)));
        }
        if r == HOSTAGE || r == KILLED_FWIFFO {
            assert_eq!(api.state["FOUND_PLUTO_SPATHI"], 3);
        }
        if r == FWIFFO_FINE {
            assert_eq!(api.state["FOUND_PLUTO_SPATHI"], 1);
            assert!(!api.disabled.contains(&(KILLED_FWIFFO as u16)));
        }
    }
    for found in 0..=3 {
        for escort in [false, true] {
            for pluto in [false, true] {
                let mut api = Fake::default();
                api.put("FOUND_PLUTO_SPATHI", found);
                api.world.have_escort = escort;
                if !pluto {
                    api.disabled.insert(SPATHI_ON_PLUTO as u16);
                }
                Dialogue { api: &mut api }.council(0);
                let mut expected = vec![WE_COME_IN_PEACE];
                if found > 0 && found < 3 {
                    if pluto {
                        expected.push(SPATHI_ON_PLUTO);
                    } else if escort {
                        expected.push(HOSTAGE);
                    } else {
                        expected.extend([KILLED_FWIFFO, FWIFFO_FINE]);
                    }
                }
                expected.extend([SURRENDER, BYE_NO_ALLY_OFFER]);
                assert_eq!(choices(&api), expected);
            }
        }
    }
    let mut api = Fake::default();
    api.put("SPATHI_MANNER", 1);
    Dialogue { api: &mut api }.council(0);
    assert_eq!(api.offers()[0], (WE_COME_IN_PEACE as u16, Handler::Council));
    api.effects.clear();
    Dialogue { api: &mut api }.council(WE_COME_IN_PEACE as i32);
    assert_eq!(
        api.offers()[0],
        (MISUNDERSTANDING as u16, Handler::Alliance)
    );
}

#[test]
fn party_visits_and_final_parallel_responses_match_c() {
    for n in 0..=255u8 {
        let mut api = Fake::default();
        api.put("SPATHI_HOME_VISITS", n);
        Dialogue { api: &mut api }.handle(Handler::Party, 0);
        assert_eq!(
            api.spoken(),
            if n <= 2 {
                vec![(MUST_PARTY_1 + u32::from(n)) as i32]
            } else {
                vec![]
            }
        );
        let next = if n == 2 { n } else { n.wrapping_add(1) };
        assert_eq!(api.state["SPATHI_HOME_VISITS"], next);
        assert_eq!(
            choices(&api),
            [
                DEALS_A_DEAL,
                match next {
                    1 => BYE_FROM_PARTY_1,
                    2 => BYE_FROM_PARTY_2,
                    _ => BYE_FROM_PARTY_3,
                }
            ]
        );
    }
    let mut api = Fake::default();
    for (r, line, next) in [
        (DEALS_A_DEAL, WAIT_A_WHILE, HOW_LONG),
        (HOW_LONG, TEN_YEARS, RENEGING),
        (RENEGING, ADULT_VIEW, RETURN_BEASTS),
        (RETURN_BEASTS, WHAT_RELATIONSHIP, MINDS_AND_MIGHT),
        (MINDS_AND_MIGHT, HUH, FELLOWSHIP),
        (FELLOWSHIP, WHAT, DO_AS_WE_SAY),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.party(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(choices(&api)[0], next);
        assert!(api.disabled.contains(&(r as u16)));
        if r == RETURN_BEASTS {
            assert_eq!(choices(&api), [MINDS_AND_MIGHT, FELLOWSHIP, DO_AS_WE_SAY]);
        }
    }
    assert_eq!(choices(&api), [DO_AS_WE_SAY]);
}

#[test]
fn allied_counters_information_story_flags_and_fishy_followup() {
    for n in 0..=255u8 {
        let mut api = Fake::default();
        api.put("SPATHI_HOME_VISITS", n);
        Dialogue { api: &mut api }.handle(Handler::Allies, 0);
        assert_eq!(
            api.spoken(),
            if n <= 2 {
                vec![(HELLO_ALLIES_1 + u32::from(n)) as i32]
            } else {
                vec![]
            }
        );
        assert_eq!(
            api.state["SPATHI_HOME_VISITS"],
            if n == 2 { n } else { n.wrapping_add(1) }
        );
        api.effects.clear();
        api.put("SPATHI_INFO", n);
        Dialogue { api: &mut api }.allies(WHATS_UP as i32);
        assert_eq!(
            api.spoken(),
            if n <= 5 {
                vec![(GENERAL_INFO_1 + u32::from(n.min(4))) as i32]
            } else {
                vec![]
            }
        );
        assert_eq!(
            api.state["SPATHI_INFO"],
            if n == 4 || n == 5 {
                n
            } else {
                n.wrapping_add(1)
            }
        );
        assert_eq!(
            api.state.get("KNOW_URQUAN_STORY"),
            if n == 2 { Some(&1) } else { None }
        );
        assert_eq!(
            api.state.get("KNOW_KOHR_AH_STORY"),
            if n == 2 { Some(&1) } else { None }
        );
        assert!(api.disabled.contains(&(WHATS_UP as u16)));
        assert_eq!(
            choices(&api).contains(&SOMETHING_FISHY),
            api.state["SPATHI_INFO"] == 4
        );
    }
    let mut api = Fake::default();
    for (r, line) in [
        (RESOURCES_PLEASE, SORRY_NO_RESOURCES),
        (SOMETHING_FISHY, NOTHING_FISHY),
        (ENOUGH_INFO, OK_ENOUGH_INFO),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.allies(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
    }
    assert_eq!(api.state["SPATHI_INFO"], 5);
    assert!(api.disabled.contains(&(RESOURCES_PLEASE as u16)));
}

#[test]
fn information_exhaustion_disables_the_parent_topic() {
    let mut api = Fake::default();
    Dialogue { api: &mut api }.handle(Handler::Info, LIKE_SOME_INFO as i32);
    assert_eq!(api.spoken(), [WHAT_ABOUT as i32]);
    for (r, line) in [
        (WHAT_ABOUT_HIERARCHY, ABOUT_HIERARCHY),
        (WHAT_ABOUT_HISTORY, ABOUT_HISTORY),
        (WHAT_ABOUT_ALLIANCE, ABOUT_ALLIANCE),
        (WHAT_ABOUT_OTHER, ABOUT_OTHER),
        (WHAT_ABOUT_PRECURSORS, ABOUT_PRECURSORS),
    ] {
        api.effects.clear();
        Dialogue { api: &mut api }.info(r as i32);
        assert_eq!(api.spoken(), [line as i32]);
        assert!(api.disabled.contains(&(r as u16)));
    }
    assert_eq!(api.offers(), [(ENOUGH_INFO as u16, Handler::Allies)]);
    assert!(api.disabled.contains(&(LIKE_SOME_INFO as u16)));
}

#[test]
fn intro_dispatches_every_gate_in_c_priority_order() {
    let ship = crate::comm::dispatch::ship::SPATHI_SHIP as usize;
    for (states, allied, line, palette) in [
        (vec![("SPATHI_MANNER", 2)], true, HATE_YOU_FOREVER, false),
        (
            vec![("SPATHI_MANNER", 1), ("KNOW_SPATHI_PASSWORD", 1)],
            true,
            MEAN_GUYS_RETURN,
            false,
        ),
        (vec![], true, HELLO_ALLIES_1, true),
        (
            vec![("SPATHI_PARTY", 1), ("SPATHI_QUEST", 1)],
            false,
            MUST_PARTY_1,
            true,
        ),
        (vec![("SPATHI_QUEST", 1)], false, HOW_GO_EFFORTS, true),
        (
            vec![("SPATHI_QUEST", 1), ("LIED_ABOUT_CREATURES", 1)],
            false,
            YOU_LIED_1,
            true,
        ),
        (
            vec![("SPATHI_QUEST", 1), ("LIED_ABOUT_CREATURES", 2)],
            false,
            YOU_LIED_2,
            false,
        ),
        (vec![("KNOW_SPATHI_QUEST", 1)], false, QUEST_AGAIN, true),
        (vec![("KNOW_SPATHI_PASSWORD", 1)], false, HELLO_AGAIN, true),
        (
            vec![("KNOW_SPATHI_PASSWORD", 1), ("SPATHI_HOME_VISITS", 7)],
            false,
            WHAT_IS_PASSWORD,
            false,
        ),
        (
            vec![
                ("KNOW_SPATHI_PASSWORD", 1),
                ("SPATHI_HOME_VISITS", 7),
                ("FOUND_PLUTO_SPATHI", 1),
            ],
            false,
            HELLO_AGAIN,
            true,
        ),
        (
            vec![
                ("SPATHI_MANNER", 1),
                ("KNOW_SPATHI_PASSWORD", 1),
                ("SPATHI_HOME_VISITS", 7),
            ],
            false,
            WHAT_IS_PASSWORD,
            false,
        ),
        (vec![], false, WHAT_IS_PASSWORD, false),
    ] {
        let mut api = Fake::default();
        for (key, value) in states {
            api.put(key, value);
        }
        if allied {
            api.alliances[ship] = 1;
        }
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [line as i32]);
        assert_eq!(api.effects.contains(&Effect::Palette(1)), palette);
        if [HATE_YOU_FOREVER, YOU_LIED_2].contains(&line) {
            assert_eq!(api.segue, 1);
            assert!(api.offers().is_empty());
        }
        if line == YOU_LIED_2 {
            assert_eq!(api.state["SPATHI_MANNER"], 2);
        }
    }
}

#[test]
fn full_descriptor_matches_c_resources_layout_and_animations() {
    let expected = [
        [1, 3, 2, 42, 0, 840, 2520, 0],
        [4, 5, 2, 42, 0, 840, 2520, 0],
        [9, 4, 2, 42, 0, 840, 2520, 3072],
        [13, 6, 2, 42, 0, 42, 0, 48],
        [19, 3, 4, 42, 0, 840, 2520, 40],
        [22, 4, 4, 84, 28, 84, 28, 1048],
        [26, 3, 4, 84, 28, 8400, 2520, 1024],
        [29, 3, 4, 84, 28, 8400, 2520, 0],
        [32, 7, 2, 42, 0, 42, 0, 1536],
        [39, 3, 4, 42, 0, 840, 2520, 1280],
        [42, 4, 4, 84, 28, 28, 0, 2916],
        [46, 4, 4, 84, 28, 84, 28, 1028],
        [50, 6, 2, 42, 0, 42, 0, 8192],
        [56, 3, 4, 42, 0, 840, 2520, 4096],
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
            (
                (1 + ((width - 80) >> 1)) as i16,
                0,
                (width - 80 - 16) as u16
            )
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
        assert_eq!(d.alien_song_flags, 1);
        unsafe {
            assert_eq!(
                std::ffi::CStr::from_ptr(d.alien_alt_song_res).to_bytes_with_nul(),
                RACE_ALT_MUSIC
            );
        }
        assert!(d.alien_frame.is_null());
        assert!(d.alien_font.is_null());
        assert!(d.alien_colormap.is_null());
        assert!(d.alien_song.is_null());
        assert!(d.conversation_phrases.is_null());
    }
}
