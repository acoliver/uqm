use super::api::World;
use super::dialogue::Dialogue;
use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;

type Api = Fake<Handler, World>;

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
fn combat_endings_and_hypnosis_updates_match_each_response() {
    for (response, phrase, segue, hypno) in [
        (YOU_MUST_SURRENDER, NOPE, 1, 0),
        (I_WONT_SURRENDER, BAD_CHOICE, 1, 0),
        (I_WILL_SURRENDER, GOOD_CHOICE, 2, 0),
        (LIKE_TO_LEAVE, INDEPENDENCE_IS_BAD, 1, 0),
        (BYE_WARS_OVER, GOODBYE_WARS_OVER, 0, 0),
        (BYE_SUB_HYPNO, GOODBYE_AND_DIE_SUB_HYPNO, 1, 0),
        (BYE_INIT_HYPNO, GOODBYE_AND_DIE_INIT_HYPNO, 1, 1),
        (TERRAN_AMPHIBIAN, BAD_NEWS, 0, 1),
        (TALKING_PET_ON_STEROIDS, BAD_NEWS, 0, 1),
        (FALLING_ASLEEP, SOUNDS_FAMILIAR, 0, 1),
        (SOMEONE_ELSE_CONTROLLED, SOUNDS_FAMILIAR, 0, 1),
    ] {
        let mut api = Api::default();
        handle(&mut api, Handler::Combat, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.segue, segue);
        assert_eq!(api.get("URQUAN_HYPNO_VISITS"), hypno);
        assert!(api.offers().is_empty());
        assert_eq!(api.effects[0], Effect::Segue(1));
    }
}

#[test]
fn information_switch_and_normal_menu_cover_every_byte_value() {
    for visits in 0..=u8::MAX {
        for response in [WHATS_UP_1, WHATS_UP_2, WHATS_UP_3, WHATS_UP_4] {
            let mut api = Api::default();
            api.put("URQUAN_INFO", visits);
            handle(&mut api, Handler::Combat, response);
            let lines = [
                GENERAL_INFO_1,
                GENERAL_INFO_2,
                GENERAL_INFO_3,
                GENERAL_INFO_4,
            ];
            let expected: Vec<i32> = lines
                .get(visits as usize)
                .map(|p| *p as i32)
                .into_iter()
                .collect();
            assert_eq!(api.spoken(), expected);
            assert_eq!(
                api.get("URQUAN_INFO"),
                if visits == 3 {
                    3
                } else {
                    visits.wrapping_add(1)
                }
            );
        }
        let mut api = Api::default();
        api.put("URQUAN_INFO", visits);
        handle(&mut api, Handler::Normal, 0);
        let mut expected = vec![YOU_MUST_SURRENDER];
        if let Some(p) = [WHATS_UP_1, WHATS_UP_2, WHATS_UP_3, WHATS_UP_4].get(visits as usize) {
            expected.push(*p);
        }
        expected.extend([I_SURRENDER, LIKE_TO_LEAVE]);
        assert_eq!(offers(&api), expected);
    }
    let mut api = Api::default();
    api.put("KNOW_URQUAN_STORY", 1);
    handle(&mut api, Handler::Normal, KEY_PHRASE);
    assert_eq!(api.spoken(), [URQUAN_STORY as i32]);
    assert_eq!(api.get("KNOW_URQUAN_STORY"), 2);
    assert!(!offers(&api).contains(&KEY_PHRASE));
}

#[test]
fn all_question_branches_disable_only_the_selected_phrase() {
    for (handler, response, phrase, base_options) in [
        (
            Handler::MentionCompulsion,
            POSSESSED_BY_DEVIL,
            STUPID_DEVIL,
            vec![
                WASCALLY_LITTLE_GUY,
                ALIEN_MIND_CONTROL,
                POSSESSED_BY_DEVIL,
                WHY_EXPLAIN,
            ],
        ),
        (
            Handler::MentionCompulsion,
            WHY_EXPLAIN,
            MUST_EXPLAIN,
            vec![
                WASCALLY_LITTLE_GUY,
                ALIEN_MIND_CONTROL,
                POSSESSED_BY_DEVIL,
                WHY_EXPLAIN,
            ],
        ),
        (
            Handler::DescribePet,
            TURD_AND_TOAD,
            WHAT_IS_TURD_AND_TOAD,
            vec![TERRAN_AMPHIBIAN, TALKING_PET_ON_STEROIDS, TURD_AND_TOAD],
        ),
        (
            Handler::DescribeCompulsion,
            BEFORE_COFFEE,
            EXPLAIN,
            vec![FALLING_ASLEEP, SOMEONE_ELSE_CONTROLLED, BEFORE_COFFEE],
        ),
        (
            Handler::Loser,
            WHY_FLEE,
            FLEE_BECAUSE,
            vec![WHY_FLEE, WHAT_HAPPENS_NOW, WHAT_ABOUT_YOU, BYE_WARS_OVER],
        ),
        (
            Handler::Loser,
            WHAT_HAPPENS_NOW,
            HAPPENS_NOW,
            vec![WHY_FLEE, WHAT_HAPPENS_NOW, WHAT_ABOUT_YOU, BYE_WARS_OVER],
        ),
        (
            Handler::Loser,
            WHAT_ABOUT_YOU,
            ABOUT_US,
            vec![WHY_FLEE, WHAT_HAPPENS_NOW, WHAT_ABOUT_YOU, BYE_WARS_OVER],
        ),
    ] {
        let mut api = Api::default();
        handle(&mut api, handler, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(api.disabled.len(), 1);
        assert!(api.disabled.contains(&(response as u16)));
        assert_eq!(
            offers(&api),
            base_options
                .into_iter()
                .filter(|p| *p != response)
                .collect::<Vec<_>>()
        );
    }
    for (handler, response, phrase, expected) in [
        (
            Handler::DescribePet,
            WASCALLY_LITTLE_GUY,
            WHAT_IT_LOOK_LIKE,
            vec![TERRAN_AMPHIBIAN, TALKING_PET_ON_STEROIDS, TURD_AND_TOAD],
        ),
        (
            Handler::DescribeCompulsion,
            ALIEN_MIND_CONTROL,
            WHAT_FELT_LIKE,
            vec![FALLING_ASLEEP, SOMEONE_ELSE_CONTROLLED, BEFORE_COFFEE],
        ),
    ] {
        let mut api = Api::default();
        handle(&mut api, handler, response);
        assert_eq!(api.spoken(), [phrase as i32]);
        assert_eq!(offers(&api), expected);
        assert!(api.disabled.is_empty());
    }
}

#[test]
fn hypnosis_questions_cover_initial_and_subsequent_encounters() {
    for (visits, questions, phrases, goodbye) in [
        (
            0,
            vec![WHERE_AM_I, WHY_DOES_MY_HEAD_HURT, WHAT_ABOUT_2_WEEKS],
            vec![YOU_ARE_HERE, HURTS_BECAUSE, ABOUT_2_WEEKS],
            BYE_INIT_HYPNO,
        ),
        (
            1,
            vec![UH_OH, STOP_MEETING],
            vec![NO_UH_OH, NO_STOP_MEETING],
            BYE_SUB_HYPNO,
        ),
    ] {
        let mut api = Api::default();
        api.put("URQUAN_HYPNO_VISITS", visits);
        for (&response, &phrase) in questions.iter().zip(&phrases) {
            api.effects.clear();
            handle(&mut api, Handler::Hypno, response);
            assert_eq!(api.spoken(), [phrase as i32]);
            let mut expected: Vec<_> = questions
                .iter()
                .copied()
                .filter(|p| !api.disabled.contains(&(*p as u16)))
                .collect();
            expected.extend([COMPULSION, goodbye]);
            assert_eq!(offers(&api), expected);
        }
    }
    let mut api = Api::default();
    api.put("URQUAN_HYPNO_VISITS", 1);
    api.put("MENTIONED_PET_COMPULSION", 1);
    handle(&mut api, Handler::Hypno, 0);
    assert_eq!(api.spoken(), [SUBSEQUENT_URQUAN_WAKE_UP as i32]);
    assert_eq!(offers(&api), [UH_OH, STOP_MEETING, BYE_SUB_HYPNO]);
}

#[test]
fn greeting_counters_preserve_final_case_decrement_and_unmatched_postincrement() {
    for frenzy in [0, 1] {
        for visits in 0..=u8::MAX {
            let mut api = Api::default();
            api.put("KOHR_AH_FRENZY", frenzy);
            api.put("URQUAN_VISITS", visits);
            Dialogue { api: &mut api }.intro();
            let lines: &[u32] = if frenzy == 0 {
                &[
                    INIT_HELLO,
                    SUBSEQUENT_HELLO_1,
                    SUBSEQUENT_HELLO_2,
                    SUBSEQUENT_HELLO_3,
                    SUBSEQUENT_HELLO_4,
                ]
            } else {
                &[INIT_FLEE_HUMAN, SUBSEQUENT_FLEE_HUMAN]
            };
            let expected: Vec<_> = lines
                .get(visits as usize)
                .map(|p| *p as i32)
                .into_iter()
                .collect();
            assert_eq!(api.spoken(), expected);
            assert_eq!(
                api.get("URQUAN_VISITS"),
                if visits as usize == lines.len() - 1 {
                    visits
                } else {
                    visits.wrapping_add(1)
                }
            );
            if frenzy != 0 && visits != 0 {
                assert!(api.offers().is_empty());
            }
            if frenzy == 0 {
                assert!(
                    matches!(api.effects.last(), Some(Effect::Set(key, _)) if key == "URQUAN_VISITS")
                );
            }
        }
    }
}

#[test]
fn intro_priority_and_initialization_use_activity_low_byte() {
    for activity in [
        0x100,
        0x100 | crate::comm::dispatch::IN_INTERPLANETARY,
        0x100 | crate::comm::dispatch::WON_LAST_BATTLE,
    ] {
        for hypno in [0, 1] {
            let mut api = Api {
                activity,
                ..Api::default()
            };
            api.put("PLAYER_HYPNOTIZED", hypno);
            api.put("TALKING_PET_ON_SHIP", 1);
            api.world.group_battles.extend(
                [
                    "URQUAN_PROBE_GRPOFFS0",
                    "COLONY_GRPOFFS0",
                    "SAMATRA_GRPOFFS0",
                ]
                .map(String::from),
            );
            Dialogue { api: &mut api }.initialize();
            let low = activity as u8;
            assert_eq!(
                api.segue,
                u32::from(
                    hypno == 0
                        && low != crate::comm::dispatch::WON_LAST_BATTLE as u8
                        && low != crate::comm::dispatch::IN_INTERPLANETARY as u8
                )
            );
            api.effects.clear();
            Dialogue { api: &mut api }.intro();
            let expected = if low == crate::comm::dispatch::WON_LAST_BATTLE as u8 {
                OUT_TAKES
            } else if low == crate::comm::dispatch::IN_INTERPLANETARY as u8 {
                SEND_MESSAGE
            } else if hypno != 0 {
                INIT_URQUAN_WAKE_UP
            } else {
                SENSE_EVIL
            };
            assert_eq!(api.spoken()[0], expected as i32);
        }
    }
    let mut api = Api {
        activity: crate::comm::dispatch::IN_INTERPLANETARY,
        ..Api::default()
    };
    api.put("TALKING_PET_ON_SHIP", 1);
    api.world
        .group_battles
        .extend(["COLONY_GRPOFFS0", "SAMATRA_GRPOFFS0"].map(String::from));
    Dialogue { api: &mut api }.intro();
    assert_eq!(api.spoken(), [SENSE_EVIL as i32, CAUGHT_YA as i32]);
    assert_eq!(api.get("URQUAN_VISITS"), 0);
    assert_eq!(api.get("AWARE_OF_SAMATRA"), 0);
}

#[test]
fn full_descriptor_uses_c_flags_rates_masks_resources_and_dynamic_geometry() {
    for (screen_width, x, width) in [(320, 121, 224), (640, 281, 544), (321, 121, 225)] {
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
        (7, 6, 2, 84, 0, 84, 0),
        (13, 3, 4, 42, 0, 840, 2520),
        (16, 3, 4, 42, 0, 840, 2520),
        (19, 3, 4, 42, 0, 840, 2520),
        (22, 7, 2, 56, 0, 840, 2520),
        (29, 7, 2, 56, 0, 840, 2520),
        (36, 8, 2, 56, 0, 840, 2520),
    ];
    for (a, expected) in d.alien_ambient_array.iter().zip(specs) {
        assert_eq!(
            (
                a.start_index,
                a.num_frames,
                a.anim_flags,
                a.base_frame_rate,
                a.random_frame_rate,
                a.base_restart_rate,
                a.random_restart_rate
            ),
            expected
        );
        assert_eq!(a.block_mask, 0);
    }
    for (a, expected) in [
        (d.alien_transition_desc, (1, 2, 140, 0)),
        (d.alien_talk_desc, (2, 5, 56, 70)),
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
        (d.alien_frame_res, c"comm.urquan.graphics"),
        (d.alien_font_res, c"comm.urquan.font"),
        (d.alien_colormap_res, c"comm.urquan.colortable"),
        (d.alien_song_res, c"comm.urquan.music"),
        (d.conversation_phrases_res, c"comm.urquan.dialogue"),
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
