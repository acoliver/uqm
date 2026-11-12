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

fn respond(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    path: &[u32],
) -> Result<(), Box<dyn std::error::Error>> {
    for &reference in path {
        let callback = comm
            .responses()
            .responses()
            .iter()
            .find(|entry| entry.response_ref == reference)
            .and_then(|entry| entry.response_func)
            .ok_or_else(|| format!("response {reference} was not offered"))?;
        comm.clear_responses();
        callback(comm, cx, reference);
    }
    Ok(())
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
    let frames = graphics::drawable_abi::LoadedDrawable::create(8, 1, 1, 100)
        .ok_or("missing panel frames")?;
    game.kernel.flag_stat_frame = frames.lease().frame(0).ok_or("missing panel frame")?;
    game.kernel.font_grad_frame = game.kernel.flag_stat_frame;
    game.kernel.misc_data_frame = game.kernel.flag_stat_frame;
    game.world.retain_fleet_resources(
        0,
        engine::battle::link_queue::ShipPickerResources::new(
            None,
            engine::battle::frames::FrameSelection::retain(game.kernel.flag_stat_frame),
        ),
    );
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    assert_eq!(
        uqm_rust::time::clock_bridge::rust_clock_init(&mut game.world),
        1
    );
    let mut fleets = game.world.avail_races();
    for _ in 0..=6 {
        let id = fleets.alloc().ok_or("missing fleet slot")?;
        fleets.get_mut(id).crew_level = 10;
        fleets.put(id);
    }
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
            cx.kernel.set_status_message_mode(2);
            cx.world.set_activity(4);
            cx.world.set_story_state(StoryKey::FOUND_PLUTO_SPATHI, 1);
            cx.world.set_story_state(StoryKey::SHOFIXTI_STACK2, 3);
            cx.world.set_story_state(StoryKey::MAIDENS_ON_SHIP, 1);
            cx.world.set_story_state(StoryKey::SLYLANDRO_PROBE_ID, 3);
            cx.world.set_story_state(StoryKey::MELNORME_YACK_STACK2, 5);
            cx.world.set_story_state(StoryKey::MELNORME_YACK_STACK1, 1);
            cx.world.set_story_state(StoryKey::MELNORME_CREDIT0, 255);
            cx.world.data.flagship.element_amounts[0] = 1;
            cx.world.data.flagship.total_element_mass = 1;
            cx.world.data.game.element_worth[0] = 3;
            cx.world.data.game.module_cost[10] = 0;
            if (conversation, variant) == (13, 1) {
                cx.world.data.flagship.module_slots[..4]
                    .fill(uqm_rust::starbase::sis_modules::ModuleType::CrewPod as u8);
                cx.world.data.flagship.crew_enlisted = 200;
                cx.world.data.game.crew_cost = 5;
                cx.world
                    .set_story_state(StoryKey::CREW_SOLD_TO_DRUUGE0, 100);
            }
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
            use uqm_rust::comm::lifecycle::{self, DialogueKind};
            let host_kind = match (conversation, variant) {
                (2, 1) => Some(DialogueKind::Starbas),
                (6, 1) => Some(DialogueKind::Spahome),
                _ => None,
            };
            if let Some(kind) = host_kind {
                lifecycle::initialize(&mut comm, kind, &mut cx);
            }
            // SAFETY: the phrase owner and display outlive all callbacks and session teardown.
            unsafe { comm.install_descriptor(descriptor) };
            if let Some(kind) = host_kind {
                lifecycle::intro(&mut comm, kind, &mut cx);
            } else {
                intro(&mut comm, &mut cx);
            }
            // References below are the actual scripts' offered phrase indices.
            let path: &[u32] = match (conversation, variant) {
                (2, 1) => &[234],                                   // HAVE_MINERALS
                (5, _) => &[58, 66, 70], // IMPORTANT_DUTY, PROCREATING_WILDLY, FEMALES
                (6, 0) => &[4, 29, 31, 37, 43, 47, 56, 48, 59, 61], // Fwiffo recruitment
                (12, _) => &[17, 111, 151, 195], // GET_ON_WITH_BUSINESS through BUY_NEW_TECH
                (13, 1) => &[75, 26, 78, 77, 81, 73], // Buy artifact and fuel, then leave
                (16, _) => &[28],        // WE_ARE_US_4, including the coordinate splice
                _ => &[],
            };
            respond(&mut comm, &mut cx, path)?;
            match (conversation, variant) {
                (2, 1) => assert_eq!(cx.world.data.flagship.total_element_mass, 0),
                (5, _) => assert_eq!(cx.world.story_state(StoryKey::SHOFIXTI_RECRUITED), 1),
                (6, 0) => assert_eq!(cx.world.built_ships_ref().count(), 1),
                (12, _) => assert_eq!(cx.world.data.game.module_cost[10], 80),
                (13, 1) => {
                    assert_eq!(cx.world.story_state(StoryKey::ARTIFACT_2_ON_SHIP), 1);
                    assert_eq!(cx.world.story_state(StoryKey::CREW_SOLD_TO_DRUUGE0), 210);
                    assert_eq!(cx.world.story_state(StoryKey::CREW_SOLD_TO_DRUUGE1), 0);
                    assert_eq!(cx.world.data.flagship.crew_enlisted, 90);
                    assert_eq!(cx.world.data.game.crew_cost, 12);
                }
                (16, _) => assert_eq!(cx.world.story_state(StoryKey::PROBE_EXHIBITED_BUG), 1),
                _ => {}
            }
            if let Some(kind) = host_kind {
                lifecycle::post_encounter(&mut comm, kind, &mut cx);
            } else {
                post(&mut comm, &mut cx);
            }
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
