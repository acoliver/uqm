use super::globals::{init_game_structures, uninit_game_structures};
use std::borrow::Cow;
use uqm_content::ships::{master_native::engine, types::SpeciesId};
use uqm_engine::{
    battle::{Battle, Cx},
    graphics::{context::LoadedContext, drawable::LoadedDrawable, geometry::Rect},
    strings::owned_table::LoadedTextTable,
    test_support::native_session::{remove_resource, ContentMount},
    Game,
};

fn with_content(test: impl FnOnce(&ContentMount)) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sc2/content");
    let content = ContentMount::new(&directory);
    content.init_resources("uqm.rmp");
    test(&content);
    uqm_engine::resource::uninit();
}

fn with_campaign(test: impl FnOnce(&mut Cx<'_>, &ContentMount)) {
    with_content(|content| {
        let screen = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        let status = LoadedContext::create(c"admission status");
        let space = LoadedContext::create(c"admission space");
        let mut game = Game::new();
        let mut battle = Battle::new();
        game.kernel.screen = screen.first_frame();
        game.kernel.status_context = Some(status.context());
        game.kernel.space_context = Some(space.context());
        game.kernel.install_game_text(LoadedTextTable::fixture(vec![
            b"Name\0".to_vec();
            crate::starbase::ids::NAMING_STRING_BASE
                as usize
                + 4
        ]));
        game.world
            .set_catalog(Cow::Borrowed(&crate::planets::tables::STARMAP_ARRAY));
        for species in SpeciesId::Arilou as u32..=SpeciesId::KohrAh as u32 {
            engine::load_entry(&mut game.world, &mut game.kernel, species);
        }
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 7);
        test(&mut cx, content);
        uninit_game_structures(&mut cx);
        cx.kernel.release_navigation_graphics();
        engine::free(cx.world, cx.kernel);
    });
}

#[test]
#[serial_test::serial]
fn missing_misc_never_becomes_ready_on_kernel_retry() {
    with_content(|_| {
        assert!(LoadedDrawable::load(c"graphics.flagshipstatus").is_some());
        remove_resource("graphics.miscdata");
        let mut kernel = uqm_engine::Kernel::new();
        assert_eq!(
            kernel
                .load_navigation_graphics(Rect::default())
                .unwrap_err()
                .resource,
            "graphics.miscdata"
        );
        assert_eq!(
            kernel
                .load_navigation_graphics(Rect::default())
                .unwrap_err()
                .resource,
            "graphics.miscdata",
            "retry admitted missing misc"
        );
        assert!(kernel.flag_stat_frame.is_null());
        assert!(kernel.misc_data_frame.is_null());
        assert!(kernel.radar_context.is_none());
    });
}

#[test]
#[serial_test::serial]
fn repaired_misc_is_acquired_on_kernel_retry() {
    with_content(|content| {
        remove_resource("graphics.miscdata");
        let mut kernel = uqm_engine::Kernel::new();
        assert_eq!(
            kernel
                .load_navigation_graphics(Rect::default())
                .unwrap_err()
                .resource,
            "graphics.miscdata"
        );
        content.load_index("uqm.rmp");
        kernel.load_navigation_graphics(Rect::default()).unwrap();
        assert!(!kernel.misc_data_frame.is_null(), "repair was never loaded");
        assert!(!kernel.flag_stat_frame.is_null());
        assert!(kernel.radar_context.is_some());
        kernel.release_navigation_graphics();
        assert!(kernel.misc_data_frame.is_null());
        assert!(kernel.flag_stat_frame.is_null());
    });
}

#[test]
#[serial_test::serial]
fn missing_misc_denies_campaign_and_repaired_retry_starts_it() {
    with_campaign(|cx, content| {
        remove_resource("graphics.miscdata");
        assert_eq!(
            init_game_structures(cx).unwrap_err().resource,
            "graphics.miscdata",
            "campaign admitted missing misc"
        );
        assert!(
            init_game_structures(cx).is_err(),
            "unrepaired retry admitted missing misc"
        );
        assert_eq!(cx.world.avail_races_ref().count(), 0);
        assert_eq!(cx.world.activity(), 0);
        assert_eq!(cx.world.built_ships_ref().count(), 0);
        assert!(cx.kernel.play_frame.is_null());
        assert!(cx.kernel.misc_data_frame.is_null());
        content.load_index("uqm.rmp");
        init_game_structures(cx).unwrap();
        assert!(!cx.kernel.misc_data_frame.is_null());
        assert_eq!(cx.world.built_ships_ref().count(), 1);
    });
}

#[test]
#[serial_test::serial]
fn missing_optional_gradient_allows_campaign_and_second_campaign() {
    with_campaign(|cx, _| {
        remove_resource("graphics.fontgradient");
        init_game_structures(cx).unwrap();
        assert!(!cx.kernel.flag_stat_frame.is_null());
        assert!(!cx.kernel.misc_data_frame.is_null());
        assert!(cx.kernel.font_grad_frame.is_null());
        let status = cx.kernel.flag_stat_frame;
        let misc = cx.kernel.misc_data_frame;
        uninit_game_structures(cx);
        assert!(cx.kernel.play_frame.is_null());
        assert_eq!(cx.kernel.flag_stat_frame, status);
        assert_eq!(cx.kernel.misc_data_frame, misc);
        init_game_structures(cx).unwrap();
        assert_eq!(cx.kernel.flag_stat_frame, status);
        assert_eq!(cx.kernel.misc_data_frame, misc);
    });
}
