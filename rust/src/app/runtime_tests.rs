use super::run_campaign;
use crate::mainloop::MainLoopError;
use uqm_content::ships::{master_native::engine, types::SpeciesId};
use uqm_engine::{
    battle::{Battle, Cx},
    graphics::{context::LoadedContext, drawable::LoadedDrawable},
    observe::Io,
    test_support::native_session::{remove_resource, ContentMount},
    Game,
};

#[test]
#[serial_test::serial]
fn required_navigation_failure_returns_before_clock_events_or_dispatch_and_allows_retry() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content");
    let content = ContentMount::new(&directory);
    content.init_resources("uqm.rmp");
    let screen = LoadedDrawable::create(8, 320, 240, 1).unwrap();
    let status = LoadedContext::create(c"admission status");
    let space = LoadedContext::create(c"admission space");
    let mut game = Game::new();
    let mut battle = Battle::new();
    game.kernel.screen = screen.first_frame();
    game.kernel.player_control = [1, 1];
    game.kernel.status_context = Some(status.context());
    game.kernel.space_context = Some(space.context());
    for species in SpeciesId::Arilou as u32..=SpeciesId::KohrAh as u32 {
        engine::load_entry(&mut game.world, &mut game.kernel, species);
    }
    remove_resource("graphics.miscdata");
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 7);
    let mut observer = uqm_engine::observe_port::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    for _ in 0..2 {
        let error = run_campaign(&mut cx, &mut io, None).unwrap_err();
        assert!(
            matches!(error, MainLoopError::Navigation(error) if error.resource == "graphics.miscdata")
        );
        assert_eq!(cx.world.data.game.game_clock.year_index, 0);
        assert_eq!(cx.world.activity(), 0);
        assert_eq!(cx.world.built_ships_ref().count(), 0);
        assert!(cx.kernel.play_frame.is_null());
        assert!(cx.kernel.misc_data_frame.is_null());
    }
    engine::free(cx.world, cx.kernel);
    uqm_engine::resource::uninit();
}
