use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::{ScriptApi, GLOBAL_PLAYER_NAME};
type Api = Fake<Handler, api::Clock>;
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().into_iter().map(|(p, _)| p).collect()
}
fn speech(f: &Api) -> Vec<(u16, bool)> {
    f.effects
        .iter()
        .filter_map(|e| {
            if let Effect::Speaker(p, pik) = e {
                Some((*p, *pik))
            } else {
                None
            }
        })
        .collect()
}
fn alternating(first: u16, count: u16) -> Vec<(u16, bool)> {
    (0..count).map(|n| (first + n, n % 2 != 0)).collect()
}
#[test]
fn first_meeting_names_questions_and_home_alliance_routing() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.get("MET_ZOQFOT"), 1);
    let mut want = alternating(WE_ARE0, 8);
    want.extend(alternating(SCOUT_HELLO0, 4));
    assert_eq!(speech(&f), want);
    assert_eq!(
        choices(&f),
        [
            WHICH_FOT,
            WE_ARE_VINDICATOR0,
            QUIET_TOADIES,
            ALL_VERY_INTERESTING,
            VALUABLE_INFO
        ]
    );
    for alliance in 0..4 {
        f.put("NEW_ALLIANCE_NAME", alliance);
        run(&mut f, Handler::AquaintZoqFot, 0);
        let mut fragments = vec![
            i32::from(WE_ARE_VINDICATOR0),
            i32::from(NAME_1) + i32::from(alliance),
        ];
        if alliance == 3 {
            fragments.extend([GLOBAL_PLAYER_NAME, i32::from(NAME_1) + 4]);
        }
        fragments.extend([
            i32::from(WE_ARE_VINDICATOR1),
            GLOBAL_SHIP_NAME,
            i32::from(WE_ARE_VINDICATOR2),
        ]);
        assert!(f.effects.contains(&Effect::Offer(
            WE_ARE_VINDICATOR0,
            Handler::ZoqFotIntro,
            fragments
        )));
    }
    for (r, first, count) in [(WHICH_FOT, HE_IS0, 8), (QUIET_TOADIES, TOLD_YOU0, 8)] {
        run(&mut f, Handler::AquaintZoqFot, r);
        assert_eq!(speech(&f), alternating(first, count));
        assert!(!f.enabled(r));
    }
    run(&mut f, Handler::ZoqFotIntro, WE_ARE_VINDICATOR0);
    assert_eq!(speech(&f), alternating(WE_GLAD0, 6));
    for (r, first, count) in [
        (YOUR_RACE, YEARS_AGO0, 14),
        (WHERE_FROM, TRAVELED_FAR0, 6),
        (WHAT_EMERGENCY, UNDER_ATTACK0, 12),
    ] {
        run(&mut f, Handler::ZoqFotIntro, r);
        assert_eq!(speech(&f), alternating(first, count));
        assert!(!f.enabled(r));
    }
    assert_eq!(
        choices(&f),
        [
            TOUGH_LUCK,
            WHAT_LOOK_LIKE,
            ALL_VERY_INTERESTING,
            HOW_CAN_I_HELP,
            VALUABLE_INFO
        ]
    );
    assert_eq!(
        f.offers().iter().find(|(p, _)| *p == HOW_CAN_I_HELP),
        Some(&(HOW_CAN_I_HELP, Handler::ExitConversation))
    );
    f.put("GLOBAL_FLAGS_AND_DATA", 128);
    run(&mut f, Handler::ZoqFotIntro, 0);
    assert_eq!(
        f.offers().iter().find(|(p, _)| *p == HOW_CAN_I_HELP),
        Some(&(HOW_CAN_I_HELP, Handler::FormAlliance))
    );
}
#[test]
fn alliance_and_hostile_exits_have_exact_effects() {
    let mut f = Api::default();
    run(&mut f, Handler::FormAlliance, HOW_CAN_I_HELP);
    assert_eq!(speech(&f), alternating(ALLY_WITH_US0, 6));
    assert_eq!(choices(&f), [SURE, NEVER, DECIDE_LATER]);
    run(&mut f, Handler::ExitConversation, SURE);
    assert_eq!(speech(&f), alternating(WE_ALLY0, 6));
    assert_eq!(
        &f.effects[f.effects.len() - 3..],
        &[
            Effect::Allied(19, true),
            Effect::Event(1, 3, 0, 0, 6),
            Effect::Set("ZOQFOT_HOME_VISITS".into(), 0)
        ]
    );
    assert_eq!(f.segue, 0);
    for (r, first, count) in [
        (ALL_VERY_INTERESTING, SEE_TOLD_YOU0, 4),
        (NEVER, WE_ENEMIES0, 2),
    ] {
        f.put("ZOQFOT_HOME_VISITS", 3);
        run(&mut f, Handler::ExitConversation, r);
        assert_eq!(speech(&f), alternating(first, count));
        assert_eq!(
            (
                f.get("ZOQFOT_HOSTILE"),
                f.get("ZOQFOT_HOME_VISITS"),
                f.segue
            ),
            (1, 0, 1)
        );
    }
    for (r, first, count) in [
        (BYE_HOMEWORLD, GOODBYE_HOME0, 2),
        (DECIDE_LATER, PLEASE_HURRY0, 2),
        (VALUABLE_INFO, GOODBYE0, 4),
        (HOW_CAN_I_HELP, EMMISSARIES0, 8),
    ] {
        run(&mut f, Handler::ExitConversation, r);
        assert_eq!(speech(&f), alternating(first, count));
        assert_eq!(f.segue, 0);
    }
}
#[test]
fn information_exhaustion_and_speaker_order() {
    let mut f = Api::default();
    f.alliances[19] = 1;
    run(&mut f, Handler::ZoqFotInfo, WANT_SPECIFIC_INFO);
    assert_eq!(speech(&f), alternating(WHAT_SPECIFIC_INFO0, 2));
    for (r, first, count) in [
        (WHAT_ABOUT_OTHERS, ABOUT_OTHERS0, 14),
        (WHAT_ABOUT_ZEBRANKY, ABOUT_ZEBRANKY0, 8),
        (WHAT_ABOUT_STINGER, ABOUT_STINGER0, 6),
        (WHAT_ABOUT_GUY_IN_BACK, ABOUT_GUY0, 2),
        (WHAT_ABOUT_PAST, ABOUT_PAST0, 12),
    ] {
        run(&mut f, Handler::ZoqFotInfo, r);
        assert_eq!(speech(&f), alternating(first, count));
        assert!(!f.enabled(r));
    }
    assert_eq!(choices(&f), [ENOUGH_INFO]);
    assert!(!f.enabled(WANT_SPECIFIC_INFO));
    run(&mut f, Handler::ZoqFotHome, ENOUGH_INFO);
    assert_eq!(speech(&f), [(OK_ENOUGH_INFO, false)]);
    assert!(!choices(&f).contains(&WANT_SPECIFIC_INFO));
}
#[test]
fn war_news_priority_clock_boundaries_and_masks() {
    for (mission, year, month, frenzy, mask, first, count, next) in [
        (0, 2155, 2, 0, 0, NO_WAR_NEWS0, 2, 0),
        (1, 2160, 3, 1, 0, KOHRAH_FRENZY0, 12, 15),
        (1, 2160, 3, 0, 0, UTWIG_DELAY0, 14, 1),
        (0, 2158, 2, 0, 0, KOHRAH_WINNING0, 10, 2),
        (0, 2158, 3, 0, 0, URQUAN_NEARLY_GONE0, 6, 6),
        (0, 2159, 2, 0, 2, URQUAN_NEARLY_GONE0, 6, 6),
        (5, 2159, 2, 0, 1, KOHRAH_WINNING0, 10, 3),
        (5, 2159, 3, 0, 1, URQUAN_NEARLY_GONE0, 6, 7),
        (0, 2160, 3, 1, 15, NO_WAR_NEWS0, 2, 15),
    ] {
        let mut f = Api::default();
        f.put("UTWIG_SUPOX_MISSION", mission);
        f.put("KOHR_AH_FRENZY", frenzy);
        f.put("ZOQFOT_KNOW_MASK", mask);
        f.world.year = year;
        f.world.month = month;
        run(&mut f, Handler::ZoqFotHome, ANY_WAR_NEWS);
        assert_eq!(speech(&f), alternating(first, count));
        assert_eq!(f.get("ZOQFOT_KNOW_MASK"), next);
        assert!(!f.enabled(ANY_WAR_NEWS));
    }
}
#[test]
fn visits_rescue_outtakes_and_descriptor() {
    for allied in [false, true] {
        let mut f = Api::default();
        f.put("MET_ZOQFOT", 1);
        f.alliances[19] = if allied { 1 } else { 2 };
        let cap = if allied { 3 } else { 1 };
        for n in 0..6 {
            f.effects.clear();
            Dialogue { api: &mut f }.intro();
            assert_eq!(f.get("ZOQFOT_HOME_VISITS"), (n + 1).min(cap));
            assert!(!speech(&f).is_empty());
        }
    }
    let mut f = Api::default();
    f.put("ZOQFOT_HOSTILE", 1);
    for _ in 0..6 {
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert!(f.offers().is_empty());
        assert_eq!(f.segue, 1);
    }
    assert_eq!(f.get("ZOQFOT_HOME_VISITS"), 3);
    f.put("ZOQFOT_HOSTILE", 0);
    f.put("MET_ZOQFOT", 1);
    f.put("ZOQFOT_DISTRESS", 1);
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(speech(&f), alternating(THANKS_FOR_RESCUE0, 12));
    assert_eq!(f.get("ZOQFOT_DISTRESS"), 0);
    assert!(f.effects.contains(&Effect::AddEscorts(19, 4)));
    f.activity = 5;
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(speech(&f), alternating(OUT_TAKES0, 14));
    assert_eq!(f.segue, 0);
    let mut d = descriptor();
    assert_eq!(
        (d.num_animations, d.alien_text_width, d.alien_text_valign),
        (3, 119, 1)
    );
    assert_eq!(d.alien_ambient_array[0].anim_flags, 12);
    assert_eq!(
        (
            d.alien_transition_desc.start_index,
            d.alien_talk_desc.start_index
        ),
        (23, 18)
    );
    f.activity = 0;
    f.alliances[19] = 2;
    Dialogue { api: &mut f }.initialize(&mut d);
    assert_eq!(f.segue, 1);
    f.alliances[19] = 1;
    Dialogue { api: &mut f }.initialize(&mut d);
    assert_eq!(f.segue, 0);
}

#[test]
fn speaker_switches_transition_animation_and_text_geometry_at_callback_time() {
    let mut d = descriptor();
    let mut last = 1;
    api::select_speaker(&mut d, &mut last, Speaker::Zoq);
    assert_eq!(
        (
            last,
            d.alien_transition_desc.anim_flags,
            d.alien_transition_desc.start_index
        ),
        (0, 32, 23)
    );
    assert_eq!(
        (
            d.alien_talk_desc.start_index,
            d.alien_talk_desc.num_frames,
            d.alien_text_baseline.x,
            d.alien_text_baseline.y
        ),
        (18, 5, 61, 24)
    );
    assert_eq!(d.alien_ambient_array[1].anim_flags & 8, 0);
    d.alien_transition_desc.anim_flags = 0;
    api::select_speaker(&mut d, &mut last, Speaker::Zoq);
    assert_eq!(d.alien_transition_desc.anim_flags, 0);
    api::select_speaker(&mut d, &mut last, Speaker::Pik);
    assert_eq!(
        (
            last,
            d.alien_transition_desc.anim_flags,
            d.alien_transition_desc.start_index
        ),
        (2, 96, 26)
    );
    assert_eq!(
        (
            d.alien_talk_desc.start_index,
            d.alien_talk_desc.num_frames,
            d.alien_text_baseline.x
        ),
        (29, 2, 181)
    );
    assert_eq!(d.alien_ambient_array[1].anim_flags & 8, 8);
    api::select_speaker(&mut d, &mut last, Speaker::Zoq);
    assert_eq!(d.alien_ambient_array[1].anim_flags & 8, 0);
    assert_eq!(
        (
            d.alien_text_fcolor.r,
            d.alien_text_fcolor.a,
            d.alien_text_bcolor.r,
            d.alien_text_bcolor.a
        ),
        (255, 255, 0, 255)
    );
}
