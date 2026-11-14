use super::*;
use crate::engine::species::conv;

#[test]
fn conversation_table_preserves_every_first_pass_and_unknown_id() {
    use DialogueKind::*;
    let expected = [
        Some(Arilou),
        Some(Chmmr),
        Some(Comandr),
        Some(Orz),
        Some(Pkunk),
        Some(Shofixt),
        Some(Spathi),
        Some(Supox),
        Some(Thradd),
        Some(Utwig),
        Some(Vux),
        Some(Yehat),
        Some(Melnorm),
        Some(Druuge),
        Some(Ilwrath),
        Some(Mycon),
        Some(Slyland),
        Some(Umgah),
        Some(Urquan),
        Some(Zoqfot),
        Some(Syreen),
        Some(Blackur),
        Some(Talkpet),
        Some(Slyhome),
        None,
        Some(Rebel),
    ];
    for (conversation, kind) in expected.into_iter().enumerate() {
        assert_eq!(for_conversation(conversation as u32), kind);
    }
    for unknown in [conv::INVALID, conv::INVALID + 1, u32::MAX] {
        assert_eq!(for_conversation(unknown), None);
    }
}

#[test]
#[serial_test::serial]
fn host_lifecycle_runs_each_registered_race() {
    crate::comm::with_test_context(|cx| {
        cx.world.story = Some(crate::engine::world::story::StoryState::new());
        let mut comm = Comm::new();
        for kind in (0..=25)
            .filter_map(for_conversation)
            .chain([DialogueKind::Starbas, DialogueKind::Spahome])
        {
            initialize(&mut comm, kind, cx);
            assert!(!comm.comm_data().unwrap().alien_frame_res.is_null());
            let first = session(&mut comm, cx, kind);
            let second = session(&mut comm, cx, kind);
            assert!(first.definition.alien_frame_res.is_some());
            assert_eq!(
                first.definition.alien_frame_res,
                second.definition.alien_frame_res
            );
            assert!(first.alien_frame.is_null());
            let entry = &LIFECYCLES[kind as usize];
            assert!(std::ptr::fn_addr_eq(
                first.init_encounter_func.unwrap(),
                entry.intro
            ));
            assert!(std::ptr::fn_addr_eq(
                first.post_encounter_func.unwrap(),
                entry.post
            ));
            assert_eq!((entry.uninit)(&mut comm, cx), 0);
        }
    });
}

#[test]
fn initialization_skips_unknown_ids_without_replacing_the_session() {
    crate::comm::with_test_context(|cx| {
        cx.world.story = Some(crate::engine::world::story::StoryState::new());
        let mut comm = Comm::new();
        initialize_conversation(&mut comm, cx, u32::MAX);
        assert!(comm.comm_data().is_none());
        initialize_conversation(&mut comm, cx, conv::ARILOU);
        let frame = comm.comm_data().unwrap().alien_frame_res;
        assert!(!frame.is_null());
        initialize_conversation(&mut comm, cx, conv::URQUAN_DRONE);
        assert_eq!(comm.comm_data().unwrap().alien_frame_res, frame);
    });
}

#[test]
fn commander_first_pass_configures_content_before_the_selected_script_pass() {
    crate::comm::with_test_context(|cx| {
        cx.world.story = Some(crate::engine::world::story::StoryState::new());
        let mut comm = Comm::new();
        for radios in [0, 1] {
            cx.world
                .set_story_state(StoryKey::RADIOACTIVES_PROVIDED, radios);
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 1);
            initialize_conversation(&mut comm, cx, conv::COMMANDER);
            assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 1);
            let first = comm.comm_data().unwrap();
            assert!(first.init_encounter_func.is_none());
            assert!(first.post_encounter_func.is_none());
            assert!(first.uninit_encounter_func.is_none());
            let width = first.alien_text_width;
            let selected = session(&mut comm, cx, DialogueKind::Comandr);
            assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 0);
            assert_eq!(selected.definition.alien_text_width, width);
            assert_eq!(
                selected.definition.conversation_phrases_res,
                Some(c"comm.commander.dialogue")
            );
            assert!(std::ptr::fn_addr_eq(
                selected.init_encounter_func.unwrap(),
                live::comandr::intro as fn(&mut Comm, &mut Cx<'_>)
            ));
        }
    });
}
