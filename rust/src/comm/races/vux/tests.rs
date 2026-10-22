use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::state::game_state_keys::bit_range;

fn run(api: &mut Fake<Handler>, action: impl FnOnce(&mut dialogue::Dialogue<'_, Fake<Handler>>)) {
    action(&mut dialogue::Dialogue { api });
}

#[test]
fn descriptor_matches_c_resources_and_animation_setup() {
    let desc = descriptor(320);
    assert_eq!(desc.num_animations, 17);
    assert_eq!(desc.alien_text_baseline.x, 181);
    assert_eq!(desc.alien_text_width, 112);
    assert_eq!(desc.alien_ambient_array[0].start_index, 12);
    assert_eq!(desc.alien_ambient_array[0].anim_flags, 1 << 0);
    assert_eq!(desc.alien_ambient_array[10].base_frame_rate, 42);
    assert_eq!(desc.alien_ambient_array[12].block_mask, 1 << 14);
    assert_eq!(desc.alien_ambient_array[14].block_mask, 1 << 12);
    assert_eq!(desc.alien_talk_desc.start_index, 1);
    assert_eq!(desc.alien_talk_desc.num_frames, 11);
}

#[test]
fn initialization_sets_peace_for_menagerie_and_won_battle_only() {
    let mut api = Fake::<Handler>::default();
    run(&mut api, |dialogue| dialogue.initialize());
    assert_eq!(api.segue, 1);

    api.put("GLOBAL_FLAGS_AND_DATA", 1 << 6);
    run(&mut api, |dialogue| dialogue.initialize());
    assert_eq!(api.segue, 0);

    api.put("GLOBAL_FLAGS_AND_DATA", 0);
    api.activity = crate::comm::dispatch::WON_LAST_BATTLE;
    run(&mut api, |dialogue| dialogue.initialize());
    assert_eq!(api.segue, 0);
}

#[test]
fn normal_intro_advances_space_visits_and_offers_progressive_topics() {
    let mut api = Fake::<Handler>::default();
    run(&mut api, |dialogue| dialogue.intro());
    assert_eq!(api.state["VUX_VISITS"], 1);
    assert_eq!(api.spoken(), [SPACE_HELLO_1 as i32]);
    assert_eq!(
        api.offers(),
        [
            (WHY_SO_MEAN as u16, Handler::Normal),
            (KILL_YOU_SQUIDS_1 as u16, Handler::Combat),
            (WHATS_UP_HOSTILE as u16, Handler::Normal),
            (BYE_HOSTILE_SPACE as u16, Handler::Combat),
        ]
    );

    api.effects.clear();
    api.put("VUX_STACK_1", 3);
    run(&mut api, |dialogue| {
        dialogue.normal(WHATS_UP_HOSTILE as i32)
    });
    assert_eq!(api.spoken(), [GENERAL_INFO_HOSTILE_1 as i32]);
    assert!(api.disabled.contains(&(WHATS_UP_HOSTILE as u16)));
    assert_eq!(api.offers()[0], (TRY_ANY_WAY as u16, Handler::Combat));
}

#[test]
fn homeworld_and_menagerie_intro_follow_global_flags() {
    let mut api = Fake::<Handler>::default();
    api.put("GLOBAL_FLAGS_AND_DATA", 1 << 7);
    run(&mut api, |dialogue| dialogue.intro());
    assert_eq!(api.state["VUX_HOME_VISITS"], 1);
    assert_eq!(api.spoken()[0], HOMEWORLD_HELLO_1 as i32);

    api.effects.clear();
    api.put("GLOBAL_FLAGS_AND_DATA", 1 << 6);
    run(&mut api, |dialogue| dialogue.intro());
    assert_eq!(api.state["ZEX_VISITS"], 1);
    assert_eq!(api.spoken()[0], ZEX_HELLO_1 as i32);
    assert!(api
        .offers()
        .iter()
        .any(|offer| offer.0 == WHAT_YOU_DO_HERE as u16));
}

#[test]
fn menagerie_progression_orders_responses_and_disables_creature_repeats() {
    let mut api = Fake::<Handler>::default();
    run(&mut api, |dialogue| dialogue.menagerie(NULL_PHRASE as i32));
    api.effects.clear();
    run(&mut api, |dialogue| {
        dialogue.menagerie(WHAT_YOU_DO_HERE as i32)
    });
    assert_eq!(api.spoken(), [MY_MENAGERIE as i32]);
    assert_eq!(api.state["ZEX_STACK_1"], 1);
    assert_eq!(
        api.offers()[0],
        (WHAT_ABOUT_MENAGERIE as u16, Handler::Menagerie)
    );

    api.effects.clear();
    run(&mut api, |dialogue| {
        dialogue.menagerie(WHAT_ABOUT_MENAGERIE as i32)
    });
    assert_eq!(api.state["ZEX_STACK_1"], 2);
    api.effects.clear();
    run(&mut api, |dialogue| {
        dialogue.menagerie(WHAT_ABOUT_CREATURE as i32)
    });
    assert_eq!(api.state["KNOW_ZEX_WANTS_MONSTER"], 1);
    assert!(api.disabled.contains(&(WHAT_ABOUT_CREATURE as u16)));
    assert!(api
        .offers()
        .iter()
        .any(|offer| offer.0 == ABOUT_CREATURE_AGAIN as u16));
}

#[test]
fn combat_counter_branches_hold_final_lines_and_advance_refusal_state() {
    let mut api = Fake::<Handler>::default();
    api.put("VUX_STACK_3", 3);
    run(&mut api, |dialogue| {
        dialogue.combat(CANT_WE_BE_FRIENDS_4 as i32)
    });
    assert_eq!(api.spoken(), [NEVER_UGLY_HUMANS_4 as i32]);
    assert_eq!(api.state["VUX_STACK_3"], 3);

    api.effects.clear();
    api.put("VUX_STACK_4", 2);
    run(&mut api, |dialogue| {
        dialogue.combat(BYE_HOSTILE_SPACE as i32)
    });
    assert_eq!(api.spoken(), [GOODBYE_AND_DIE_HOSTILE_SPACE_3 as i32]);
    assert_eq!(api.state["VUX_STACK_4"], 3);

    api.effects.clear();
    api.put("VUX_STACK_1", 13);
    run(&mut api, |dialogue| dialogue.combat(APOLOGY_10 as i32));
    assert_eq!(api.spoken(), [TRUTH as i32]);
    assert_eq!(api.state["VUX_STACK_1"], 14);
}

#[test]
fn accepting_beast_applies_palette_effects_and_peaceful_ending() {
    let mut api = Fake::<Handler>::default();
    api.put("VUX_BEAST_ON_SHIP", 1);
    run(&mut api, |dialogue| dialogue.combat(OK_TAKE_BEAST as i32));
    assert_eq!(api.spoken(), [FOOL_AIEE0 as i32, FOOL_AIEE1 as i32]);
    assert_eq!(api.state["VUX_BEAST_ON_SHIP"], 0);
    assert_eq!(api.state["ZEX_IS_DEAD"], 1);
    assert_eq!(api.segue, 0);
    assert!(api.effects.contains(&Effect::Talk(1)));
    assert!(api.effects.contains(&Effect::FadePalette(1, 210)));
    assert!(api.effects.contains(&Effect::Talk(u16::MAX)));
}

#[test]
fn required_vux_game_state_keys_are_present() {
    for key in [
        "GLOBAL_FLAGS_AND_DATA",
        "KNOW_ZEX_WANTS_MONSTER",
        "VUX_BEAST_ON_SHIP",
        "VUX_HOME_VISITS",
        "VUX_INFO",
        "VUX_STACK_1",
        "VUX_STACK_2",
        "VUX_STACK_3",
        "VUX_STACK_4",
        "ZEX_STACK_1",
        "ZEX_STACK_2",
        "ZEX_STACK_3",
        "ZEX_VISITS",
        "ZEX_IS_DEAD",
    ] {
        assert!(bit_range(key).is_some(), "missing game state key: {key}");
    }
}
