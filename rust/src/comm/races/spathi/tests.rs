use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler>;
fn run(f: &mut Api, s: &mut Session, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f, session: s }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().iter().map(|(p, _)| *p).collect()
}

#[test]
fn descriptor_and_initial_segue_music_on_reentry() {
    let mut d = descriptor();
    assert_eq!(d.num_animations, 8);
    assert_eq!(d.alien_talk_desc.num_frames, 0);
    assert_eq!(
        (
            d.alien_text_baseline.x,
            d.alien_text_baseline.y,
            d.alien_text_width
        ),
        (164, 0, 288)
    );
    assert_eq!(d.alien_text_valign, 0);
    assert_eq!(d.alien_ambient_array[0].block_mask, 2);
    assert_eq!(d.alien_ambient_array[5].block_mask, 8);
    let mut s = Session::default();
    for (pluto, manner, activity, want) in [
        (1, 0, 0, 0),
        (0, 3, 0, 0),
        (0, 0, 5, 0),
        (0, 0, 0, 1),
        (0, 1, 0, 1),
    ] {
        let mut f = Api::default();
        f.put("FOUND_PLUTO_SPATHI", pluto);
        f.put("SPATHI_MANNER", manner);
        f.activity = activity;
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .initialize(&mut d);
        assert_eq!(f.segue, want);
        assert_eq!(d.alien_song_flags & 1, u32::from(pluto == 1));
        if pluto == 1 {
            assert_eq!(
                unsafe { std::ffi::CStr::from_ptr(d.alien_alt_song_res) },
                c"comm.fwiffo.music"
            );
        }
    }
}
#[test]
fn pluto_intro_resets_password_refusals_and_prioritizes_encounter() {
    let mut f = Api::default();
    let mut s = Session { refusals: 2 };
    f.put("FOUND_PLUTO_SPATHI", 1);
    f.put("SPATHI_HOME_VISITS", 7);
    f.put("SPATHI_MANNER", 2);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .intro();
    assert_eq!(s.refusals, 0);
    assert_eq!(f.get("SPATHI_HOME_VISITS"), 0);
    assert_eq!(f.get("KNOW_SPATHI_PASSWORD"), 0);
    assert_eq!(f.spoken(), vec![1]);
    assert_eq!(choices(&f), vec![2, 4, 6, 7]);
}
#[test]
fn grovelling_stages_keep_response_order_and_targets() {
    let mut f = Api::default();
    let mut s = Session::default();
    for (r, p, offers) in [
        (
            IDENTIFY,
            3,
            vec![
                (10, Handler::SpathiMustGrovel),
                (69, Handler::SpathiOnPluto),
                (12, Handler::ExitConversation),
            ],
        ),
        (
            DO_CULTURAL,
            11,
            vec![
                (13, Handler::SpathiMustGrovel),
                (15, Handler::SpathiOnPluto),
                (8, Handler::ExitConversation),
            ],
        ),
        (
            BEGIN_RITUAL,
            14,
            vec![
                (20, Handler::SpathiOnPluto),
                (18, Handler::ExitConversation),
                (22, Handler::SpathiMustGrovel),
            ],
        ),
        (
            WHAT_ARE_COORDINATES,
            23,
            vec![
                (27, Handler::SpathiOnPluto),
                (24, Handler::ExitConversation),
            ],
        ),
    ] {
        run(&mut f, &mut s, Handler::SpathiMustGrovel, r);
        assert_eq!(f.spoken(), vec![p]);
        assert_eq!(f.offers(), offers);
    }
}
#[test]
fn pluto_questions_disable_the_right_phrase_and_recruit_after_monsters() {
    let mut f = Api::default();
    let mut s = Session::default();
    for (r, p) in [
        (HI_THERE, 5),
        (DONT_KILL, 9),
        (YOURE_FORGIVEN, 70),
        (YOU_WONT_DIE_YET, 16),
        (YOU_MAY_LIVE, 67),
        (YOUVE_GOT_ME_ALL_WRONG, 28),
    ] {
        run(&mut f, &mut s, Handler::SpathiOnPluto, r);
        assert_eq!(f.spoken(), vec![p]);
    }
    for (r, p) in [
        (WHAT_DOING_ON_PLUTO_1, 30),
        (WHAT_DOING_ON_PLUTO_2, 32),
        (WHAT_DOING_ON_PLUTO_3, 40),
        (WHAT_ABOUT_ILWRATH, 46),
        (WHEN_ILWRATH, 53),
        (WHAT_ABOUT_MOONBASE, 44),
        (WHAT_ABOUT_OTHER_SPATHI, 49),
        (HOW_MANY_CREW, 58),
        (REALLY_THOUSANDS, 57),
        (FULL_OF_MONSTERS, 60),
        (WHAT_ENEMY, 51),
        (WHY_YOU_HERE, 55),
        (WHERE_ARE_URQUAN, 34),
        (WHAT_ABOUT_OTHER_RACES, 36),
        (WHAT_BLAZE_OF_GLORY, 42),
        (WHAT_ABOUT_YOURSELF, 39),
    ] {
        run(&mut f, &mut s, Handler::SpathiOnPluto, r);
        assert_eq!(f.spoken(), vec![p], "response {r}");
        assert!(!f.enabled(r));
    }
    assert_eq!(
        f.offers(),
        vec![
            (61, Handler::ExitConversation),
            (68, Handler::ExitConversation)
        ]
    );
    for (r, p) in [(JOIN_US, 63), (JOIN_US, 65), (JOIN_US, 66), (JOIN_US, 66)] {
        run(&mut f, &mut s, Handler::SpathiOnPluto, r);
        assert_eq!(f.spoken(), vec![p]);
    }
    assert_eq!(s.refusals, 2);
}
#[test]
fn recruitment_preserves_talk_add_ship_and_captain_order() {
    let mut f = Api::default();
    let mut s = Session::default();
    f.feasibility = 0;
    run(&mut f, &mut s, Handler::ExitConversation, JOIN_US);
    assert_eq!(f.effects, vec![Effect::Segue(0), Effect::Npc(26)]);
    f.feasibility = 1;
    run(&mut f, &mut s, Handler::ExitConversation, JOIN_US);
    assert_eq!(
        f.effects,
        vec![
            Effect::Segue(0),
            Effect::Npc(62),
            Effect::Talk(u16::MAX),
            Effect::AddEscorts(6, 1),
            Effect::FwiffoCrew
        ]
    );
}
#[test]
fn exits_preserve_combat_and_coordinate_phrase_order() {
    let mut f = Api::default();
    let mut s = Session::default();
    for (r, p) in [
        (BYE_ALLY_SPACE, 133),
        (BYE_FRIENDLY_SPACE, 101),
        (PART_IN_PEACE, 113),
        (WE_SORRY_SPACE, 83),
        (GIVE_INFO_SPACE, 81),
        (BYE_ANGRY_SPACE, 87),
        (ATTACK_YOU_NOW, 123),
        (WE_FIGHT_AGAIN_SPACE, 85),
    ] {
        run(&mut f, &mut s, Handler::ExitConversation, r);
        assert_eq!(f.spoken(), vec![p]);
        assert_eq!(
            f.segue,
            u32::from(matches!(r, ATTACK_YOU_NOW | WE_FIGHT_AGAIN_SPACE))
        );
    }
    for r in [
        DIE_SLUGBOY,
        WE_FIGHT_1,
        WE_FIGHT_2,
        PAY_FOR_CRIMES,
        TELL_ME_COORDINATES,
        CHANGED_MIND,
    ] {
        run(&mut f, &mut s, Handler::ExitConversation, r);
        assert_eq!(f.segue, 1);
        assert_eq!(
            f.spoken(),
            if r == TELL_ME_COORDINATES {
                vec![25, 134]
            } else {
                vec![134]
            }
        );
    }
}
#[test]
fn all_intro_modes_and_visit_saturation() {
    for (manner, allied, quest, first, second, offers) in [
        (2, false, 0, 71, 71, 0),
        (1, false, 0, 72, 73, 4),
        (0, true, 0, 78, 79, 5),
        (0, false, 1, 76, 77, 3),
        (0, false, 0, 74, 75, 4),
    ] {
        let mut f = Api::default();
        let mut s = Session::default();
        f.put("SPATHI_MANNER", manner);
        f.put("SPATHI_QUEST", quest);
        if allied {
            f.alliances[6] = 1;
        }
        for p in [first, second, second] {
            f.effects.clear();
            Dialogue {
                api: &mut f,
                session: &mut s,
            }
            .intro();
            assert_eq!(f.spoken(), vec![p]);
            assert_eq!(f.offers().len(), offers);
        }
        assert_eq!(f.get("SPATHI_VISITS"), u8::from(manner != 2));
    }
}
#[test]
fn friendly_allied_and_antagonistic_questions() {
    let mut f = Api::default();
    let mut s = Session::default();
    for (h, r, p) in [
        (Handler::SpathiAllies, WHATS_UP_SPACE_2, 125),
        (Handler::SpathiAllies, GIVE_US_INFO_FROM_SPACE, 127),
        (Handler::SpathiAllies, GIVE_US_RESOURCES_SPACE, 129),
        (Handler::SpathiAllies, WHAT_DO_FOR_FUN, 131),
        (Handler::SpathiFriendly, SINCE_FRIENDLY_GIVE_STUFF, 97),
        (Handler::SpathiFriendly, WHATS_UP_SPACE_1, 99),
        (Handler::SpathiAntagonize, HEARD_YOURE_COWARDS, 115),
        (Handler::SpathiAntagonize, WANNA_FIGHT, 117),
        (Handler::SpathiAntagonize, SO_LETS_FIGHT, 119),
        (Handler::SpathiAntagonize, SO_LETS_FIGHT_ALREADY, 121),
    ] {
        run(&mut f, &mut s, h, r);
        assert_eq!(f.spoken(), vec![p]);
        assert!(!f.enabled(r));
        assert!(!choices(&f).contains(&r));
    }
    run(&mut f, &mut s, Handler::SpathiAntagonize, PREPARE_TO_DIE);
    assert_eq!(f.get("SPATHI_STACK2"), 1);
    assert_eq!(f.spoken(), vec![95]);
    run(&mut f, &mut s, Handler::SpathiNeutral, LOOK_WEIRD);
    assert_eq!(f.get("SPATHI_STACK0"), 1);
    assert_eq!(choices(&f)[0], 90);
    run(&mut f, &mut s, Handler::SpathiNeutral, NO_LOOK_REALLY_WEIRD);
    assert_eq!(f.get("SPATHI_STACK0"), 2);
    assert_eq!(choices(&f), vec![92, 122, 112]);
}
#[test]
fn befriending_rotates_stacks_and_returns_to_neutral_only_when_exhausted() {
    let mut f = Api::default();
    let mut s = Session::default();
    for (r, p, offers) in [
        (COME_IN_PEACE, 93, vec![102, 106, 110]),
        (TELL_US_ABOUT_YOU, 107, vec![108, 102, 110]),
        (LOOKING_FOR_A_FEW_GOOD_SQUIDS, 103, vec![104, 108, 110]),
        (WHY_SLAVES, 105, vec![108, 110]),
        (WHAT_YOU_REALLY_WANT, 109, vec![110]),
        (HOW_ABOUT_ALLIANCE, 111, vec![88, 94, 112]),
    ] {
        run(&mut f, &mut s, Handler::SpathiBefriend, r);
        assert_eq!(f.spoken(), vec![p]);
        assert_eq!(choices(&f), offers);
    }
    assert_eq!(f.get("SPATHI_STACK1"), 1);
}
#[test]
fn post_encounter_handles_pluto_and_hostility_without_resetting_password_hint() {
    for (pluto, segue, manner, home, want_manner, want_visits, want_home) in [
        (1, 1, 0, 7, 0, 3, 7),
        (0, 0, 0, 4, 0, 3, 4),
        (0, 1, 2, 4, 2, 3, 4),
        (0, 1, 1, 4, 1, 3, 4),
        (0, 1, 0, 7, 1, 0, 7),
        (0, 1, 0, 4, 1, 0, 0),
    ] {
        let mut f = Api::default();
        let mut s = Session::default();
        f.put("FOUND_PLUTO_SPATHI", pluto);
        f.put("SPATHI_MANNER", manner);
        f.put("SPATHI_HOME_VISITS", home);
        f.put("SPATHI_VISITS", 3);
        f.segue = segue;
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .post_spathi_enc();
        assert_eq!(f.get("FOUND_PLUTO_SPATHI"), if pluto == 1 { 2 } else { 0 });
        assert_eq!(f.get("SPATHI_MANNER"), want_manner);
        assert_eq!(f.get("SPATHI_VISITS"), want_visits);
        assert_eq!(f.get("SPATHI_HOME_VISITS"), want_home);
    }
}
