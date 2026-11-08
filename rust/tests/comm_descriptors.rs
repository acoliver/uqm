use std::ffi::CStr;

use uqm_rust::{
    comm::{dispatch::race_descriptor, locdata::CLocData},
    engine::{
        battle::{Battle, Cx},
        Game, StoryKey,
    },
};

#[test]
#[serial_test::serial]
fn descriptor_factories_return_independent_values_and_reselect_variants() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    uqm_rust::comm::state::init_communication().unwrap();
    // SAFETY: the serialized fixture owns the initialized conversation and game thread.
    unsafe {
        let mut first: CLocData = race_descriptor(&mut cx, 16);
        let width = first.alien_text_width;
        let talk = first.alien_talk_desc.start_index;
        first.alien_text_width = width.wrapping_add(1);
        first.alien_talk_desc.start_index = talk.wrapping_add(1);
        first.alien_frame = std::ptr::dangling_mut();
        let second = race_descriptor(&mut cx, 16);
        assert_eq!(second.alien_text_width, width);
        assert_eq!(second.alien_talk_desc.start_index, talk);
        assert!(second.alien_frame.is_null());
        assert_eq!(first.alien_text_width, width.wrapping_add(1));
        assert_eq!(
            CStr::from_ptr(second.alien_frame_res).to_bytes(),
            b"comm.probe.graphics"
        );

        cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 0);
        let commander = race_descriptor(&mut cx, 2);
        cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);
        let starbase = race_descriptor(&mut cx, 2);
        assert_eq!(
            CStr::from_ptr(commander.conversation_phrases_res).to_bytes(),
            b"comm.commander.dialogue"
        );
        assert_eq!(
            CStr::from_ptr(starbase.conversation_phrases_res).to_bytes(),
            b"comm.starbase.dialogue"
        );
    }
    uqm_rust::comm::state::uninit_communication();
}
