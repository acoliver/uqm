use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler>;
fn run(f: &mut Api, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}

#[test]
fn init_segue_and_descriptor() {
    for (probe, flags, activity, want) in [
        (0, 0, 0, 1),
        (1, 0, 0, 0),
        (0, 16, 0, 0),
        (0, 32, 0, 0),
        (0, 128, 0, 1),
        (0, 0, 0x105, 0),
    ] {
        let mut f = Api::default();
        f.put("PROBE_ILWRATH_ENCOUNTER", probe);
        f.put("GLOBAL_FLAGS_AND_DATA", flags);
        f.activity = activity;
        let mut d = descriptor();
        Dialogue { api: &mut f }.initialize(&mut d);
        assert_eq!(f.segue, want);
        Dialogue { api: &mut f }.post_ilwrath_enc();
        assert_eq!(
            (
                d.alien_text_baseline.x,
                d.alien_text_baseline.y,
                d.alien_text_width
            ),
            (164, 70, 288)
        );
        assert_eq!(d.num_animations, 4);
        assert_eq!(d.alien_ambient_array[3].num_frames, 6);
        assert_eq!(d.alien_talk_desc.base_frame_rate, 56);
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(d.alien_frame_res) },
            c"comm.ilwrath.graphics"
        );
    }
}
#[test]
fn probe_intro_and_every_combat_exit() {
    let mut f = Api::default();
    f.put("PROBE_ILWRATH_ENCOUNTER", 1);
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), vec![23]);
    assert_eq!(
        f.offers(),
        [78, 79, 81, 80].map(|r| (r, Handler::CombatIsInevitable))
    );
    for (r, p) in [
        (YOU_ARE_WEAK, 47),
        (SLAY_BY_THOUSANDS, 48),
        (EASE_UP, 49),
        (BYE_SPACE, 50),
        (BYE_HOMEWORLD, 52),
        (WANT_PEACE, 32),
        (WANT_ALLIANCE, 33),
        (BUT_EVIL_IS_DEFINED, 65),
        (BYE_GODS, 36),
        (BYE, 30),
        (WHERE_YOU_COME_FROM, 24),
        (IT_WILL_BE_A_PLEASURE, 25),
        (SURRENDER, 26),
        (BE_REASONABLE, 27),
    ] {
        run(&mut f, Handler::CombatIsInevitable, r);
        assert_eq!(f.spoken(), vec![p]);
        assert_eq!(f.segue, if r == BYE_GODS { 0 } else { 1 });
        assert!(f.offers().is_empty());
    }
    run(&mut f, Handler::CombatIsInevitable, WHATS_UP);
    assert_eq!(f.spoken(), vec![29]);
    assert_eq!(f.offers(), vec![(83, Handler::CombatIsInevitable)]);
}
#[test]
fn intro_routing_priority_and_broadcaster() {
    for (flags, fight, deceived, spoken, offers) in [
        (128, 0, 0, 51, 5),
        (64, 0, 0, 20, 2),
        (32, 0, 0, 13, 0),
        (32, 0, 1, 7, 0),
        (32, 1, 1, 6, 0),
        (16, 0, 0, 34, 4),
        (16, 0, 1, 9, 3),
        (16, 1, 1, 8, 3),
        (0, 0, 0, 37, 5),
        (0, 0, 1, 66, 0),
        (0, 1, 1, 67, 0),
        (128 | 64 | 32 | 16, 0, 0, 51, 5),
    ] {
        let mut f = Api::default();
        f.put("GLOBAL_FLAGS_AND_DATA", flags);
        f.put("ILWRATH_FIGHT_THRADDASH", fight);
        f.put("ILWRATH_DECEIVED", deceived);
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), vec![spoken]);
        assert_eq!(f.offers().len(), offers);
        if flags == 32 || flags == 0 {
            assert_eq!(f.segue, 0);
        }
    }
}
#[test]
fn visit_and_information_counters_saturate_with_exact_phrases() {
    for visit in 0..=4 {
        let mut f = Api::default();
        f.put("ILWRATH_VISITS", visit);
        f.put("ILWRATH_INFO", visit);
        Dialogue { api: &mut f }.ilwrath_space(0);
        assert_eq!(f.spoken(), vec![37 + i32::from(visit)]);
        assert_eq!(f.get("ILWRATH_VISITS"), (visit + 1).min(4));
        assert_eq!(
            f.offers(),
            [87 + u16::from(visit), 92, 93, 94, 95].map(|r| (r, Handler::CombatIsInevitable))
        );
        run(
            &mut f,
            Handler::CombatIsInevitable,
            WHATS_UP_SPACE_1 + u16::from(visit),
        );
        assert_eq!(f.spoken(), vec![42 + i32::from(visit)]);
        assert_eq!(f.get("ILWRATH_INFO"), (visit + 1).min(4));
    }
    for (key, flags, last, phrases) in [
        ("ILWRATH_HOME_VISITS", 128, 1, vec![51, 28]),
        ("ILWRATH_CHMMR_VISITS", 64, 1, vec![20, 19]),
        ("ILWRATH_GODS_SPOKEN", 16, 3, vec![34, 10, 11, 12]),
    ] {
        for visit in 0..=last {
            let mut f = Api::default();
            f.put(key, visit);
            f.put("GLOBAL_FLAGS_AND_DATA", flags);
            Dialogue { api: &mut f }.intro();
            assert_eq!(f.spoken(), vec![phrases[usize::from(visit)]]);
            assert_eq!(f.get(key), (visit + 1).min(last));
        }
    }
}
#[test]
fn gods_questions_exhaust_in_original_order() {
    let mut f = Api::default();
    for (r, p, choices) in [
        (WANT_INFO_ON_GODS, 53, vec![98, 99, 100, 77]),
        (WHEN_START_WORSHIP, 54, vec![99, 100, 77]),
        (ANY_GOOD_GODS, 55, vec![100, 77]),
        (HOW_TALK_WITH_GODS, 56, vec![101, 77]),
        (WHY_44, 57, vec![77]),
    ] {
        run(&mut f, Handler::IlwrathGods, r);
        assert_eq!(f.spoken(), vec![p]);
        assert_eq!(
            f.offers().iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            choices
        );
    }
    assert!(!f.enabled(WANT_INFO_ON_GODS));
    run(&mut f, Handler::IlwrathHome, ENOUGH_GODS);
    assert_eq!(f.spoken(), vec![22]);
    assert_eq!(f.offers().len(), 4);
}
#[test]
fn information_keeps_evil_challenge_and_home_exits() {
    let mut f = Api::default();
    for (r, p) in [
        (WANT_INFO_ON_ILWRATH, 58),
        (WHAT_ABOUT_PHYSIO, 59),
        (WHAT_ABOUT_HISTORY, 60),
        (WHAT_ABOUT_CULTURE, 61),
        (WHAT_ABOUT_URQUAN, 63),
        (ARE_YOU_EVIL, 64),
    ] {
        run(&mut f, Handler::IlwrathInfo, r);
        assert_eq!(f.spoken(), vec![p]);
    }
    assert_eq!(
        f.offers(),
        vec![
            (108, Handler::CombatIsInevitable),
            (76, Handler::IlwrathHome)
        ]
    );
    assert!(f.enabled(WANT_INFO_ON_ILWRATH));
    run(&mut f, Handler::IlwrathHome, ENOUGH_ILWRATH);
    assert_eq!(f.spoken(), vec![21]);
    assert_eq!(f.offers().last(), Some(&(96, Handler::CombatIsInevitable)));
}
#[test]
fn gods_orders_worship_and_mission_effect_order() {
    let mut f = Api::default();
    run(&mut f, Handler::GodsSpeak, GO_KILL_THRADDASH);
    assert_eq!(
        &f.effects[..3],
        &[
            Effect::Npc(35),
            Effect::Set("ILWRATH_DECEIVED".into(), 1),
            Effect::AdvanceIlwrath
        ]
    );
    assert!(!f.offers().iter().any(|(p, _)| *p == 86));
    for visit in 0..=2 {
        f.put("ILWRATH_WORSHIP", visit);
        run(&mut f, Handler::GodsSpeak, WORSHIP_US);
        assert_eq!(f.spoken(), vec![16 + i32::from(visit)]);
        assert_eq!(f.get("ILWRATH_WORSHIP"), (visit + 1).min(2));
        assert!(!f.enabled(WORSHIP_US));
    }
    for (r, p) in [
        (OTHER_DIVINE_ORDERS, 15),
        (SAY_WARSHIP, 2),
        (SAY_DWE, 3),
        (SAY_YOUBOO, 4),
        (SAY_DILLRAT, 5),
    ] {
        run(&mut f, Handler::GodsOrder, r);
        assert_eq!(f.spoken(), vec![p]);
    }
    assert_eq!(f.offers(), vec![(72, Handler::GodsSpeak)]);
    assert!(!f.enabled(OTHER_DIVINE_ORDERS));
    run(&mut f, Handler::GodsSpeak, ENOUGH_ORDERS);
    assert_eq!(f.spoken(), vec![1]);
    assert_eq!(f.offers(), vec![(75, Handler::CombatIsInevitable)]);
}
