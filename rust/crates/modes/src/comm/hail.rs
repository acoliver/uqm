//! Resource admission, encounter execution and ordered teardown.

use super::assets::Assets;
use super::sis_graphics::ConversationDisplay;
use super::sis_graphics::ConversationResources;
use super::Comm;
use uqm_engine::graphics::{
    context::ContextLease, drawable::ContextFrame, geometry::Extent, geometry::Rect,
};
use uqm_engine::{battle::Cx, observe::Io, StoryKey};

pub(super) fn load_conversation_text(comm: &mut Comm) {
    let phrases = comm
        .data
        .definition
        .conversation_phrases_res
        .and_then(uqm_engine::strings::conversation::ConversationTable::load);
    comm.set_conversation_phrases(phrases);
}

/// Load and admit retained resources using the supplied campaign graphics.
pub fn prepare(
    comm: &mut Comm,
    world: &uqm_engine::World,
    kernel: &uqm_engine::Kernel,
    dimensions: (i32, i32),
) -> Result<(), String> {
    assert!(
        comm.display.assets.is_none(),
        "conversation assets already loaded"
    );
    comm.presentation = super::presentation::Presentation::from(kernel);
    let mut retained = ConversationResources::new(world).map_err(|error| error.to_string())?;
    comm.reset_for_encounter();
    comm.set_talking_finished(false);
    let space = ContextLease::require(comm.presentation.space_context);
    let mut assets = space
        .with_selected(|| Assets::load(&comm.data.definition, dimensions))
        .map_err(str::to_owned)?;
    let display = ConversationDisplay::new(comm, &assets, kernel)?;
    load_conversation_text(comm);
    comm.set_comm_data(super::locdata::session_view(&comm.data));
    super::comm_abi::setup_subtitle_text_from_comm_data(comm);
    assets.old_font = space.font_lease();
    space.select();
    space.set_font(&assets.player.lease());
    let animation = assets
        .animation
        .as_ref()
        .unwrap_or_else(|| panic!("animation context not loaded"))
        .lease();
    animation.select();
    animation.set_frame(ContextFrame::retain(comm.presentation.screen));
    comm.display.window.width = (dimensions.0 - 78) as i16;
    comm.display.window.height = assets.frame.lease().context_frame(0).rect().extent.height;
    if let Some(window) = &comm.options.window {
        window.set(comm.display.window);
    }
    animation.transition_source(None);
    assets.intro_batch = Some(animation.batch());
    if world.activity() & 0xff != 5 && comm.options.window.is_none() {
        comm.display.window.corner = super::locdata::CPoint { x: 7, y: 10 };
    }
    animation.set_clip(Rect {
        corner: comm.display.window.corner,
        extent: Extent {
            width: comm.display.window.width,
            height: comm.display.window.height,
        },
    });
    retained.display = Some(display);
    comm.display.resources = Some(retained);
    comm.display.assets = Some(assets);
    Ok(())
}

/// Finish callbacks and release the admitted conversation in resource order.
pub fn finish(comm: &mut Comm, cx: &mut Cx<'_>) {
    let space = comm.conversation_display().space.clone();
    drop(
        comm.display
            .resources
            .as_mut()
            .unwrap_or_else(|| panic!("conversation resources not admitted"))
            .display
            .take(),
    );
    space.select();
    drop(
        comm.display
            .assets
            .as_mut()
            .unwrap_or_else(|| panic!("conversation assets not loaded"))
            .intro_batch
            .take(),
    );
    drop(
        comm.display
            .assets
            .as_mut()
            .unwrap_or_else(|| panic!("conversation assets not loaded"))
            .animation
            .take(),
    );
    space.finish_color_transforms();
    super::ffi::rust_clear_subtitles(comm);
    crate::ui::settings::stop_music();
    uqm_engine::sound::heart_ffi::stop_sound();
    uqm_engine::sound::heart_ffi::stop_track();
    let fade_end = uqm_engine::sound::heart_ffi::fade_music(128, 0);
    uqm_engine::threading::pacing::GameThreadPacer::new(space.clone())
        .sleep_until(fade_end.wrapping_add(840 / 60));
    super::dispatch::finish_callbacks(comm, cx);
    let mut assets = comm
        .display
        .assets
        .take()
        .unwrap_or_else(|| panic!("conversation assets not loaded"));
    space.select();
    space.replace_font(assets.old_font.take());
    drop(comm.display.phrases.take());
    assets.release(&space);
    comm.data.definition.conversation_phrases_res = None;
    comm.clear_comm_data();
}

pub fn hail_alien(comm: &mut Comm, cx: &mut Cx<'_>, io: &mut Io<'_>) {
    use uqm_engine::observe::CheckpointView;
    let input = cx.kernel.input_runtime().input().clone();
    let activity = cx.world.activity();
    prepare(comm, cx.world, cx.kernel, cx.screen_dimensions())
        .unwrap_or_else(|error| panic!("HailAlien: {error}"));
    {
        if activity & 0xff != 5 {
            let title = starbase_title(cx.world);
            let dimensions = cx.screen_dimensions();
            crate::ui::border::draw_conversation_frame(cx.kernel, cx.world, dimensions, title);
        }
        super::sis_graphics::draw_sis_com_window(comm, activity);
        cx.kernel.last_activity |= 0x1000;
        if let Some(callback) = comm.data.init_encounter_func {
            callback(comm, cx);
            super::ffi::observe_responses(comm, io);
        }
        input.set_menu_sounds((1 << 0) | (1 << 1), 1 << 4);
        io.observer
            .checkpoint(&CheckpointView::CommunicationStarted);
        comm.run_input(cx, io, false, |comm, cx, io| {
            super::ffi::rust_do_communication(comm, &input, cx, io) != 0
        });
        io.observer.checkpoint(&CheckpointView::CommunicationEnded);
    }
    finish(comm, cx);
    admit_completion(comm, cx, io);
}

/// The game-string index of the "Starbase" title when talking to the allied
/// Starbase; its "Starbase Commander" message is the next string.
pub(super) fn starbase_title(world: &uqm_engine::World) -> Option<usize> {
    (world.story_state(StoryKey::GLOBAL_FLAGS_AND_DATA) == 0xff
        && world.story_state(StoryKey::STARBASE_AVAILABLE) != 0)
        .then_some(crate::starbase::ids::STARBASE_STRING_BASE as usize)
}

fn admit_completion(comm: &mut Comm, cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let admitted = uqm_engine::input::admit_observer(
        cx,
        io,
        uqm_engine::observe::InputView::CommunicationCompleted,
    );
    comm.apply_input(admitted);
}

#[cfg(test)]
mod tests {
    use uqm_engine::{
        observe::{CheckpointView, Injected, InputView, Io, Observer, Presented},
        Kernel, World,
    };

    struct CompletionObserver {
        world: *mut World,
        kernel: *mut Kernel,
        calls: usize,
    }

    impl Observer for CompletionObserver {
        fn input_admitted(
            &mut self,
            view: &InputView,
            cx: &mut uqm_engine::battle::Cx<'_>,
        ) -> Injected {
            assert!(matches!(view, InputView::CommunicationCompleted));
            assert_eq!(cx.world.activity(), 17);
            assert_eq!(std::ptr::from_mut(cx.world), self.world);
            assert_eq!(std::ptr::from_mut(cx.kernel), self.kernel);
            self.calls += 1;
            Injected {
                comm_response: Some(1),
                ..Injected::default()
            }
        }

        fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
        fn presented(&mut self, _: &mut Presented) {}
    }

    #[test]
    #[serial_test::serial]
    fn completion_admission_preserves_the_current_input_owner_contract() {
        crate::comm::with_test_context(|cx| {
            cx.world.set_activity(17);
            let mut comm = crate::comm::Comm::new();
            comm.add_response(17, "First", None);
            comm.add_response(23, "Second", None);
            let mut observer = CompletionObserver {
                world: std::ptr::from_mut(cx.world),
                kernel: std::ptr::from_mut(cx.kernel),
                calls: 0,
            };
            super::admit_completion(
                &mut comm,
                cx,
                &mut Io {
                    observer: &mut observer,
                },
            );
            assert_eq!(observer.calls, 1);
            assert_eq!(comm.selected_response(), 1);
        });
    }

    #[test]
    #[serial_test::serial]
    fn conversation_text_admission_retains_the_supplied_game_labels() {
        super::super::sis_graphics_font_tests::with_display(
            "comm::hail::tests::conversation_text_admission_retains_the_supplied_game_labels",
            |comm, _| {
                super::load_conversation_text(comm);
                assert_eq!(comm.conversation_display().game_strings.label(256).0, c"A");
                assert!(comm.display.phrases.is_none());
            },
        );
    }
}
