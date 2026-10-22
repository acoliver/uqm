use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::state::game_state_keys::bit_range;

fn run(
    api: &mut Fake<Handler, api::World>,
    action: impl FnOnce(&mut dialogue::Dialogue<'_, Fake<Handler, api::World>>),
) {
    action(&mut dialogue::Dialogue { api });
}

#[test]
fn descriptor_matches_c_animation_and_text_geometry() {
    let desc = descriptor(320);
    assert_eq!(desc.num_animations, 15);
    assert_eq!(desc.alien_text_baseline.x, 161);
    assert_eq!(desc.alien_text_baseline.y, 60);
    assert_eq!(desc.alien_text_width, 149);
    assert_eq!(desc.alien_text_valign, 1);
    assert_eq!(desc.alien_ambient_array[0].anim_flags, (1 << 2) | (1 << 3));
    assert_eq!(desc.alien_ambient_array[2].block_mask, (1 << 4) | (1 << 14));
    assert_eq!(desc.alien_ambient_array[14].start_index, 48);
    assert_eq!(desc.alien_talk_desc.num_frames, 3);
}

#[test]
fn ordinary_space_intro_advances_greeting_and_offers_space_topics() {
    let mut api = Fake::<Handler, api::World>::default();
    run(&mut api, |dialogue| dialogue.intro());
    assert_eq!(api.spoken(), [SPACE_HELLO_1 as i32]);
    assert_eq!(api.state["YEHAT_VISITS"], 1);
    assert_eq!(
        api.offers(),
        [
            (WHATS_UP_SPACE_1 as u16, Handler::Space),
            (AT_LEAST_HELP_US_SPACE as u16, Handler::Space),
            (I_DEMAND_YOU_ALLY_SPACE0 as u16, Handler::Space),
            (BYE_SPACE as u16, Handler::Exit),
        ]
    );
}

#[test]
fn space_information_counter_disables_followup_and_caps_final_phrase() {
    let mut api = Fake::<Handler, api::World>::default();
    api.put("YEHAT_REBEL_INFO", 3);
    run(&mut api, |dialogue| dialogue.space(WHATS_UP_SPACE_1 as i32));
    assert_eq!(api.spoken(), [GENERAL_INFO_SPACE_4 as i32]);
    assert_eq!(api.state["YEHAT_REBEL_INFO"], 3);
    assert!(api.disabled.contains(&(WHATS_UP_SPACE_1 as u16)));
    assert!(!api
        .offers()
        .iter()
        .any(|(phrase, _)| *phrase == WHATS_UP_SPACE_1 as u16));
}

#[test]
fn homeworld_topics_respect_one_time_info_help_and_alliance_states() {
    let mut api = Fake::<Handler, api::World>::default();
    api.put("GLOBAL_FLAGS_AND_DATA", 1 << 7);
    run(&mut api, |dialogue| dialogue.intro());
    assert_eq!(api.spoken()[0], HOMEWORLD_HELLO_1 as i32);
    assert!(api
        .offers()
        .iter()
        .any(|(phrase, _)| *phrase == GIVE_INFO as u16));

    api.effects.clear();
    run(&mut api, |dialogue| dialogue.home(GIVE_INFO as i32));
    assert_eq!(api.spoken(), [NO_INFO_FOR_ENEMY as i32]);
    assert_eq!(api.state["NO_YEHAT_INFO"], 1);
    assert!(!api
        .offers()
        .iter()
        .any(|(phrase, _)| *phrase == GIVE_INFO as u16));

    api.effects.clear();
    run(&mut api, |dialogue| {
        dialogue.exit(I_DEMAND_YOU_ALLY_HOMEWORLD0 as i32)
    });
    assert_eq!(api.spoken(), [ENEMY_MUST_DIE as i32]);
    assert_eq!(api.state["NO_YEHAT_ALLY_HOME"], 1);
}

#[test]
fn rebellion_offer_and_confirmation_reset_states_and_schedule_event() {
    let mut api = Fake::<Handler, api::World>::default();
    api.put("SHOFIXTI_VISITS", 1);
    run(&mut api, |dialogue| dialogue.space(NULL_PHRASE as i32));
    assert!(api.offers().iter().any(|(phrase, handler)| {
        *phrase == SHOFIXTI_ALIVE_1 as u16 && *handler == Handler::StartRevolt
    }));

    api.effects.clear();
    api.world.have_shofixti = true;
    run(&mut api, |dialogue| {
        dialogue.start_revolt(SHOFIXTI_ALIVE_1 as i32)
    });
    assert_eq!(api.spoken(), [SEND_HIM_OVER_1 as i32]);
    assert_eq!(api.state["YEHAT_REBEL_TOLD_PKUNK"], 1);
    assert!(api
        .offers()
        .iter()
        .any(|(phrase, _)| *phrase == OK_SEND as u16));

    api.effects.clear();
    run(&mut api, |dialogue| dialogue.exit(OK_SEND as i32));
    assert_eq!(api.spoken(), [WE_REVOLT as i32]);
    assert_eq!(api.state["YEHAT_CIVIL_WAR"], 1);
    assert_eq!(api.state["YEHAT_VISITS"], 0);
    assert_eq!(api.state["YEHAT_REBEL_INFO"], 0);
    assert!(api.effects.contains(&Effect::Event(1, 0, 0, 0, 15)));
}

#[test]
fn space_exit_uses_random_choice_and_obligation_finishes_peacefully() {
    let mut api = Fake::<Handler, api::World>::default();
    api.world.random_bit = 1;
    run(&mut api, |dialogue| dialogue.exit(BYE_SPACE as i32));
    assert_eq!(api.spoken(), [GOODBYE_AND_DIE_SPACE as i32]);
    assert_eq!(api.segue, 1);

    api.effects.clear();
    api.world.random_bit = 0;
    run(&mut api, |dialogue| dialogue.exit(BYE_SPACE as i32));
    assert_eq!(api.spoken(), [GO_IN_PEACE as i32]);
    assert_eq!(api.segue, 0);

    api.effects.clear();
    run(&mut api, |dialogue| dialogue.space(OBLIGATION as i32));
    assert_eq!(api.spoken(), [WE_CANNOT_2 as i32]);
    assert_eq!(api.state["NO_YEHAT_ALLY_SPACE"], 2);
    assert_eq!(api.segue, 0);
}

#[test]
fn civil_war_intro_selects_royalist_greeting_and_question() {
    let mut api = Fake::<Handler, api::World>::default();
    api.put("YEHAT_CIVIL_WAR", 1);
    run(&mut api, |dialogue| dialogue.intro());
    assert_eq!(api.spoken(), [ROYALIST_SPACE_HELLO_1 as i32]);
    assert_eq!(api.state["YEHAT_VISITS"], 1);
    assert!(api
        .offers()
        .iter()
        .any(|(phrase, _)| *phrase == HOW_IS_REBELLION as u16));

    api.effects.clear();
    run(&mut api, |dialogue| {
        dialogue.royalists(HOW_IS_REBELLION as i32)
    });
    assert_eq!(api.spoken(), [ROYALIST_REBELLION_1 as i32]);
    assert!(api.disabled.contains(&(HOW_IS_REBELLION as u16)));
}

#[test]
fn required_yehat_game_state_keys_are_present() {
    for key in [
        "GLOBAL_FLAGS_AND_DATA",
        "YEHAT_CIVIL_WAR",
        "YEHAT_VISITS",
        "YEHAT_HOME_VISITS",
        "YEHAT_REBEL_VISITS",
        "YEHAT_REBEL_INFO",
        "YEHAT_REBEL_TOLD_PKUNK",
        "YEHAT_ROYALIST_INFO",
        "YEHAT_ROYALIST_TOLD_PKUNK",
        "YEHAT_ABSORBED_PKUNK",
        "NO_YEHAT_INFO",
        "NO_YEHAT_ALLY_HOME",
        "NO_YEHAT_ALLY_SPACE",
        "NO_YEHAT_HELP_HOME",
        "NO_YEHAT_HELP_SPACE",
        "SHOFIXTI_VISITS",
        "PKUNK_VISITS",
        "PKUNK_HOME_VISITS",
    ] {
        assert!(bit_range(key).is_some(), "missing game state key: {key}");
    }
}
