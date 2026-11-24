//! Production load/render/teardown against real registered resources.
use super::{commanim_abi::runtime, hail, races::descriptor::CommDescriptor, Comm};
use uqm_engine::battle::Battle;
use uqm_engine::battle::Cx;
use uqm_engine::graphics::{
    context::ContextLease, drawable::ContextFrame, geometry::Color, geometry::Point,
};
use uqm_engine::Game;

fn descriptor() -> CommDescriptor {
    CommDescriptor {
        alien_frame_res: Some(c"fixture.comm.graphics"),
        alien_font_res: Some(c"fixture.comm.font"),
        alien_text_width: 100,
        ..Default::default()
    }
}

fn with_game(name: &str, run: impl FnOnce(&mut Game, &ContextLease)) {
    with_game_options(name, Default::default(), run);
}

fn with_game_options(
    name: &str,
    options: uqm_engine::graphics::present::GraphicsOptions,
    run: impl FnOnce(&mut Game, &ContextLease),
) {
    if !uqm_engine::test_support::native_process::own_graphics_process(name) {
        return;
    }
    let mut game = Game::new();
    game.world.story = Some(uqm_engine::state::game_state::StoryState::new());
    let fixture =
        uqm_engine::test_support::campaign_display::CampaignDisplay::install_prepared_with_options(
            &mut game,
            |_| {},
            |game| {
                let space = ContextLease::require(game.kernel.status_context);
                space.set_frame(ContextFrame::retain(game.kernel.screen));
                game.kernel.space_context = Some(space.context());
                game.kernel.radar_context = Some(space.context());
                space.set_font(&uqm_engine::graphics::font::FontLease::retain(
                    game.kernel.tiny_font.unwrap(),
                ));
                run(game, &space);
            },
            options,
        )
        .unwrap();
    fixture.finish(game);
}

#[test]
#[serial_test::serial]
fn two_loaded_conversations_keep_kernel_scope_and_release_assets_after_leases() {
    with_game("comm::assets_tests::two_loaded_conversations_keep_kernel_scope_and_release_assets_after_leases", |game, space| {
        let original = space.font_lease().unwrap();
        let mut scope = None;
        for activity in [0, 0x4000, 0x1000] {
            game.world.set_activity(activity);
            let mut comm = Comm::new();
            comm.presentation = super::presentation::Presentation::from(&game.kernel);
            comm.data.definition = descriptor();
            comm.data.post_encounter_func = Some(|comm, cx| {
                assert_eq!(comm.assets().frame.lease().context_frame(0).frame_count(), 3);
                cx.world.set_story_state(uqm_engine::StoryKey::CHMMR_BOMB_STATE, 1);
            });
            comm.data.uninit_encounter_func = Some(|comm, cx| {
                assert_eq!(comm.assets().frame.lease().context_frame(0).frame_count(), 3);
                assert_eq!(cx.world.story_state(uqm_engine::StoryKey::CHMMR_BOMB_STATE), u32::from(cx.world.activity() & 0x5000 == 0));
                cx.world.set_story_state(uqm_engine::StoryKey::CHMMR_BOMB_STATE, 2);
                0
            });
            game.world.set_story_state(uqm_engine::StoryKey::CHMMR_BOMB_STATE, 0);
            comm.data.definition.num_animations = 1;
            comm.data.definition.alien_ambient_array[0] = super::AnimationDescriptor {
                start_index: 1, num_frames: 2, anim_flags: 2, ..Default::default()
            };
            hail::prepare(&mut comm, &game.world, &game.kernel, (320, 240)).unwrap();
            let animation = comm.animation_context();
            // Intro owns the initial cross-function batch; this fixture completes it before drawing.
            drop(comm.display.assets.as_mut().unwrap().intro_batch.take());
            runtime::initialize_at(&mut comm, 0, &mut || 0);
            assert_eq!(runtime::draw_alien_frame(&mut comm, 3, 1), 1);
            animation.flush();
            let pixel = space.target().pixel(Point { x: 8, y: 11 });
            assert_eq!((pixel.r, pixel.g, pixel.b), (0, 0, 180));
            let activity = uqm_engine::graphics::drawable::LoadedDrawable::create(6, 8, 6, 10).unwrap();
            comm.presentation.activity_frame = activity.lease().frame(0).unwrap();
            super::speech_graphics::SpeechGraphics::init(&mut comm);
            let work = comm.scope.scope_work.as_ref().unwrap().frame();
            if let Some(previous) = scope { assert_eq!(work, previous); }
            scope = Some(work);
            let retained = comm.conversation_display().clone();
            let player = retained.player_font.clone();
            let frame = comm.assets().frame.lease().context_frame(2);
            let mut battle = Battle::new();
            let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, uqm_engine::math::random);
            hail::finish(&mut comm, &mut cx);
            assert!(comm.display.assets.is_none());
            assert!(comm.display.phrases.is_none());
            assert!(comm.comm_data().is_none());
            assert!(space.font_lease().unwrap().same_font(&original));
            // Both queued glyph storage and sprite pixels remain usable after logical owner release.
            retained.space.draw_text(&player, c"A", Point { x: 20, y: 20 }, uqm_engine::graphics::geometry::TextAlign::Left, Color { r: 255, g: 255, b: 255, a: 255 });
            retained.space.draw_stamp(&frame, Point { x: 30, y: 30 });
            retained.space.flush();
            assert_eq!(space.target().pixel(Point { x: 31, y: 31 }).b, 180);
            drop(retained);
            space.set_font(&original);
            drop(comm);
            assert!(game.kernel.comm_scope_buffer.get().unwrap().is_some());
        }
    });
}

#[test]
#[serial_test::serial]
fn failed_loads_drop_partial_owners_and_duplicate_admission_is_rejected() {
    with_game(
        "comm::assets_tests::failed_loads_drop_partial_owners_and_duplicate_admission_is_rejected",
        |game, space| {
            let mut comm = Comm::new();
            comm.presentation = super::presentation::Presentation::from(&game.kernel);
            comm.data.definition = descriptor();
            comm.data.definition.alien_font_res = Some(c"missing.font");
            space.select();
            let before = space.selected_context().unwrap().context();
            assert_eq!(
                hail::prepare(&mut comm, &game.world, &game.kernel, (320, 240)),
                Err("alien font failed to load".into())
            );
            assert!(comm.display.assets.is_none());
            assert_eq!(space.selected_context().unwrap().context(), before);
            comm.data.definition = descriptor();
            hail::prepare(&mut comm, &game.world, &game.kernel, (320, 240)).unwrap();
            let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                hail::prepare(&mut comm, &game.world, &game.kernel, (320, 240)).unwrap();
            }));
            assert!(duplicate.is_err());
            drop(comm.display.assets.as_mut().unwrap().intro_batch.take());
            comm.data.uninit_encounter_func = Some(|comm, _| {
                ContextLease::require(comm.presentation.space_context).draw_text(
                    &comm.assets().player.lease(),
                    c"A",
                    Point { x: 100, y: 100 },
                    uqm_engine::graphics::geometry::TextAlign::Left,
                    Color {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                );
                0
            });
            let mut battle = Battle::new();
            let mut cx = Cx::new(
                &mut battle,
                &mut game.world,
                &mut game.kernel,
                uqm_engine::math::random,
            );
            hail::finish(&mut comm, &mut cx);
            assert!((94..102).any(|y| (98..105).any(|x| {
                let pixel = space.target().pixel(Point { x, y });
                pixel.r == 255 && pixel.g == 255 && pixel.b == 255
            })));
        },
    );
}

#[test]
#[serial_test::serial]
fn releasing_assets_drains_pixels_without_presenting_a_frame() {
    let presentations = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = presentations.clone();
    struct Counter(std::rc::Rc<std::cell::Cell<u64>>);
    impl uqm_engine::graphics::present::PresentObserver for Counter {
        fn suppress_present(&mut self) -> bool {
            false
        }
        fn capture(
            &mut self,
            count: u64,
            _: &[uqm_engine::graphics::presentation_diagnostics::PresentationSample],
        ) -> Option<u64> {
            self.0.set(count);
            None
        }
        fn presented(&mut self, _: Option<uqm_engine::graphics::presented_frame::PresentedFrame>) {}
    }
    let options = uqm_engine::graphics::present::GraphicsOptions {
        observer: Some(Box::new(Counter(observed))),
        ..Default::default()
    };
    with_game_options(
        "comm::assets_tests::releasing_assets_drains_pixels_without_presenting_a_frame",
        options,
        |game, space| {
            let mut comm = Comm::new();
            comm.presentation = super::presentation::Presentation::from(&game.kernel);
            comm.data.definition = descriptor();
            hail::prepare(&mut comm, &game.world, &game.kernel, (320, 240)).unwrap();
            let mut assets = comm.display.assets.take().unwrap();
            drop(assets.intro_batch.take());
            space.draw_stamp(&assets.frame.lease().context_frame(0), Point { x: 0, y: 0 });
            let display = uqm_engine::test_support::native_probe::Display::create();
            let screen = ContextFrame::retain(display.frame());
            let before = presentations.get();
            space.with_frame(screen, || {
                space.draw_stamp(&assets.frame.lease().context_frame(0), Point { x: 0, y: 0 });
                assets.release(space);
            });
            drop(display);
            assert_eq!(
                presentations.get(),
                before,
                "resource drain presented a frame"
            );
            let pixel = space.target().pixel(Point { x: 0, y: 0 });
            assert_eq!((pixel.r, pixel.g, pixel.b), (180, 0, 0));
        },
    );
}
