#[path = "support/campaign_display.rs"]
mod campaign_display_test;
#[path = "support/conversation_display.rs"]
mod conversation_display;
#[path = "support/native_process.rs"]
mod native_process;

use std::ffi::CStr;

use uqm_rust::{
    comm::{dispatch::race_descriptor, locdata::CLocData, Comm},
    engine::{
        self,
        battle::{Battle, Cx},
        Game, StoryKey,
    },
    graphics,
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
    let mut comm = Comm::new();
    comm.init().unwrap();
    // SAFETY: the serialized fixture owns the initialized conversation and game thread.
    unsafe {
        let mut first: CLocData = race_descriptor(&mut comm, &mut cx, 16);
        let width = first.alien_text_width;
        let talk = first.alien_talk_desc.start_index;
        first.alien_text_width = width.wrapping_add(1);
        first.alien_talk_desc.start_index = talk.wrapping_add(1);
        first.alien_frame = std::ptr::dangling_mut();
        let second = race_descriptor(&mut comm, &mut cx, 16);
        assert_eq!(second.alien_text_width, width);
        assert_eq!(second.alien_talk_desc.start_index, talk);
        assert!(second.alien_frame.is_null());
        assert_eq!(first.alien_text_width, width.wrapping_add(1));
        assert_eq!(
            CStr::from_ptr(second.alien_frame_res).to_bytes(),
            b"comm.probe.graphics"
        );

        cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 0);
        let commander = race_descriptor(&mut comm, &mut cx, 2);
        cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);
        let starbase = race_descriptor(&mut comm, &mut cx, 2);
        assert_eq!(
            CStr::from_ptr(commander.conversation_phrases_res).to_bytes(),
            b"comm.commander.dialogue"
        );
        assert_eq!(
            CStr::from_ptr(starbase.conversation_phrases_res).to_bytes(),
            b"comm.starbase.dialogue"
        );
    }
    comm.uninit();
}

#[test]
#[serial_test::serial]
fn all_descriptor_factories_initialize_without_loaded_resources() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    let mut comm = Comm::new();
    comm.init().unwrap();
    // SAFETY: the serialized fixture owns the initialized conversation and game thread.
    unsafe {
        for conversation in 0..=25 {
            let descriptor = race_descriptor(&mut comm, &mut cx, conversation);
            assert!(!descriptor.alien_frame_res.is_null());
            assert!(!descriptor.conversation_phrases_res.is_null());
            assert!(descriptor.alien_frame.is_null());
            assert!(descriptor.init_encounter_func.is_some());
        }
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 1 << 7);
        let home = race_descriptor(&mut comm, &mut cx, 6);
        assert_eq!(
            CStr::from_ptr(home.alien_frame_res).to_bytes(),
            b"comm.safeones.graphics"
        );
    }
    comm.uninit();
}

#[test]
#[serial_test::serial]
fn live_callbacks_keep_the_supplied_session_and_phrase_owner(
) -> Result<(), Box<dyn std::error::Error>> {
    if !native_process::own_graphics_process(
        "live_callbacks_keep_the_supplied_session_and_phrase_owner",
    ) {
        return Ok(());
    }
    let mut game = Game::new();
    let display = conversation_display::ConversationDisplay::install(&mut game)?;
    let runtime = uqm_rust::mainloop::input_abi::NativeInputRuntime::new(
        None,
        Default::default(),
        game.kernel.configuration.clone(),
    );
    game.kernel.install_input_runtime(std::rc::Rc::new(runtime));
    game.kernel.activity_frame = game.kernel.screen;
    game.kernel.star_con_font = game.kernel.tiny_font;
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    assert_eq!(
        uqm_rust::time::clock_bridge::rust_clock_init(&mut game.world),
        1
    );
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    for variant in [0, 1] {
        cx.world
            .set_story_state(StoryKey::STARBASE_AVAILABLE, variant);
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, variant << 7);
        cx.world.set_story_state(StoryKey::STARBASE_MONTH, 2);
        cx.world.set_story_state(StoryKey::STARBASE_DAY, 17);
        for conversation in 0..=25 {
            let mut comm = Comm::with_options(uqm_rust::comm::CommOptions {
                oscilloscope: false,
                slider: false,
                ..Default::default()
            });
            // SAFETY: the serialized child owns the display, initialized session and game context.
            let mut descriptor = unsafe { race_descriptor(&mut comm, &mut cx, conversation) };
            let phrases =
                uqm_rust::strings::owned_table::LoadedTextTable::fixture(vec![
                    b"A\0".to_vec();
                    2048
                ]);
            descriptor.conversation_phrases = phrases.as_raw().cast();
            descriptor.alien_font = cx.kernel.tiny_font.cast();
            let (intro, post, uninit) = (
                descriptor.init_encounter_func.ok_or("missing intro")?,
                descriptor.post_encounter_func.ok_or("missing post")?,
                descriptor.uninit_encounter_func.ok_or("missing uninit")?,
            );
            // SAFETY: the phrase owner and display outlive all callbacks and session teardown.
            unsafe { comm.install_descriptor(descriptor) };
            intro(&mut comm, &mut cx);
            post(&mut comm, &mut cx);
            assert_eq!(uninit(&mut comm, &mut cx), 0);
            assert_eq!(
                comm.comm_data()
                    .ok_or("session data missing")?
                    .conversation_phrases,
                phrases.as_raw().cast()
            );
            comm.uninit();
        }
    }
    drop(battle);
    display.finish(game);
    Ok(())
}
