use super::Host;
use crate::comm::{races, Comm, CommIntroMode};

fn local_effect<H>(comm: &mut Comm, action: impl FnOnce(&mut Host<'_, '_, H>)) {
    crate::comm::with_test_context(|cx| {
        let mut host = Host::new(comm, cx, |_| panic!("unexpected response registration"));
        action(&mut host);
    });
}

#[test]
fn phrase_and_ambient_controls_need_only_the_supplied_conversation() {
    use races::script::{DialogueHost, ANIM_DISABLED};
    let mut first = Comm::new();
    let second = Comm::new();
    first.data.definition.alien_ambient_array[2].anim_flags = 3;
    local_effect::<()>(&mut first, |host| {
        assert!(host.enabled(7));
        host.disable(7);
        assert!(!host.enabled(7));
        assert!(host.enabled(8));
        host.disable_animation(2);
    });
    assert_eq!(
        first.data.definition.alien_ambient_array[2].anim_flags,
        3 | ANIM_DISABLED
    );
    assert!(second.phrase_enabled(7));
    assert_eq!(second.data.definition.alien_ambient_array[2].anim_flags, 0);
}

#[test]
fn enabled_query_does_not_need_an_exclusive_context_or_conversation() {
    use races::script::DialogueHost;
    let mut comm = Comm::new();
    crate::comm::with_test_context(|cx| {
        let host = Host::<()>::new(&mut comm, cx, |_| panic!("unexpected response"));
        let _context = host.cx.borrow();
        let _conversation = host.comm.borrow();
        assert!(host.enabled(9));
    });
}

#[test]
fn chmmr_and_urquan_intro_controls_keep_native_modes_and_durations() {
    use races::{chmmr::api::ChmmrApi, urquan::api::UrquanApi};
    let mut first = Comm::new();
    let second = Comm::new();
    for (mode, expected) in [
        (0, CommIntroMode::Default),
        (1, CommIntroMode::FadeIn),
        (2, CommIntroMode::CrossFade),
        (3, CommIntroMode::Immediate),
        (77, CommIntroMode::Default),
    ] {
        local_effect::<races::chmmr::Handler>(&mut first, |host| host.intro_mode(mode, -1));
        assert_eq!(first.intro_mode(), expected);
        assert_eq!(first.fade_time(), u32::MAX);
        local_effect::<races::urquan::Handler>(&mut first, |host| host.intro_mode(mode, 1680));
        assert_eq!(first.intro_mode(), expected);
        assert_eq!(first.fade_time(), 1680);
    }
    first.reset_for_encounter();
    assert_eq!(first.fade_time(), 1680);
    assert_eq!(second.intro_mode(), CommIntroMode::Default);
    assert_eq!(second.fade_time(), 0);
}

#[test]
fn talking_strobe_and_probe_choices_do_not_borrow_a_campaign() {
    use races::{
        orz::api::OrzApi, slyland::api::SlylandApi, starbas::api::StarbaseApi,
        talkpet::api::TalkpetApi,
    };
    let mut first = Comm::new();
    let second = Comm::new();
    first.data.definition.alien_talk_desc.anim_flags = 3;
    first.data.definition.alien_ambient_array[16].anim_flags = 0x83;
    for enabled in [false, true] {
        local_effect::<races::orz::Handler>(&mut first, |host| host.talking(enabled));
        assert_eq!(
            first.data.definition.alien_talk_desc.anim_flags,
            if enabled { 3 } else { 19 }
        );
        local_effect::<races::starbas::Handler>(&mut first, |host| host.talking_animation(enabled));
        assert_eq!(
            first.data.definition.alien_talk_desc.anim_flags,
            if enabled { 3 } else { 19 }
        );
    }
    local_effect::<races::talkpet::Handler>(&mut first, |host| host.strobe());
    assert_eq!(first.data.definition.alien_ambient_array[16].anim_flags, 3);
    local_effect::<races::slyland::Handler>(&mut first, |host| {
        host.remember_choices([5, 4, 3, 2, 1]);
        assert_eq!(host.choices(), [5, 4, 3, 2, 1]);
    });
    assert_eq!(second.data.definition.alien_talk_desc.anim_flags, 0);
    assert_eq!(second.data.definition.alien_ambient_array[16].anim_flags, 0);
    assert_eq!(second.race_scratch.slyland_choices, [0; 5]);
}

