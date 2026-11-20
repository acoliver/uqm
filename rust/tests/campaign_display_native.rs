//! Campaign labels borrow unpublished owners and retain their display resources.
#![forbid(unsafe_code)]
use std::ffi::CString;
use uqm_engine::graphics::context::{ContextLease, LoadedContext};
use uqm_engine::graphics::{
    drawable::LoadedDrawable, geometry::Color, geometry::Extent, geometry::Point, geometry::Rect,
    geometry::TextAlign, menu::select as SetContext, present::GraphicsOptions,
};
use uqm_engine::test_support::native_probe::{frame_pixel as GetFramePixel, selected_clip};
use uqm_engine::Game;

#[test]
fn retained_campaign_panels_clear_only_their_boxes_and_restore_selection_and_clip() {
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&fixture_root).unwrap();
    let directory = tempfile::tempdir_in(fixture_root.canonicalize().unwrap()).unwrap();
    std::fs::create_dir(directory.path().join("font")).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
        .save(directory.path().join("font/0041.png"))
        .unwrap();
    std::fs::write(
        directory.path().join("fixture.rmp"),
        "fixture.tiny = FONTRES:font\n",
    )
    .unwrap();
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    {
        uqm_engine::graphics::present::init_graphics(1, 0, GraphicsOptions::default(), 320, 240);
        let content = uqm_engine::test_support::native_session::ContentMount::new(directory.path());
        content.init_resources("fixture.rmp");
        let font = uqm_engine::graphics::font::LoadedFont::load(c"fixture.tiny").unwrap();
        let glyph = font
            .lease()
            .measure(c"A", Point { x: 27, y: 6 }, TextAlign::Center, u16::MAX)
            .rect;
        drop(content);
        let context_owner = LoadedContext::create(c"campaign title");
        let previous_owner = LoadedContext::create(c"previous context");
        let (context, previous) = (
            Some(context_owner.context()),
            Some(previous_owner.context()),
        );
        let drawable = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        let screen = drawable.first_frame();
        let selected = ContextLease::require(context);
        selected.set_frame(drawable.lease().context_frame(0));
        selected.set_background(Color {
            r: 9,
            g: 17,
            b: 25,
            a: 255,
        });
        selected.clear();
        SetContext(previous);
        let mut game = Game::new();
        game.kernel.off_screen_context = context;
        game.kernel.screen = screen;
        game.kernel.install_tiny_font(font);
        uqm_modes::ui::sis::draw_title(&game.kernel, 320, c"A");
        assert_eq!(SetContext(previous), previous);
        uqm_engine::graphics::state::flush();
        for (point, expected) in [
            (Point { x: 193, y: 2 }, (0, 0, 165)),
            (Point { x: 247, y: 8 }, (0, 0, 165)),
            (Point { x: 192, y: 2 }, (9, 17, 25)),
            (Point { x: 193, y: 9 }, (9, 17, 25)),
        ] {
            let pixel = GetFramePixel(screen, point);
            assert_eq!((pixel.r, pixel.g, pixel.b), expected);
        }
        let text_pixel = GetFramePixel(
            screen,
            Point {
                x: 193 + glyph.corner.x,
                y: 2 + glyph.corner.y,
            },
        );
        assert_eq!((text_pixel.r, text_pixel.g, text_pixel.b), (222, 0, 222));
        SetContext(context);
        let (mut clip, set) = selected_clip();
        assert!(!set);
        assert_eq!((clip.extent.width, clip.extent.height), (320, 240));
        let status = ContextLease::require(previous);
        status.set_frame(drawable.lease().context_frame(0));
        status.set_clip(Rect {
            corner: Point { x: 10, y: 20 },
            extent: Extent {
                width: 64,
                height: 180,
            },
        });
        game.kernel.status_context = previous;
        SetContext(context);
        uqm_modes::ui::sis::draw_status_message(&game.kernel, &game.world, Some(c"A"));
        assert_eq!(SetContext(context), context);
        uqm_engine::graphics::state::flush();
        for (point, expected) in [
            (Point { x: 12, y: 150 }, (0, 66, 0)),
            (Point { x: 71, y: 156 }, (0, 66, 0)),
            (Point { x: 11, y: 150 }, (9, 17, 25)),
            (
                Point {
                    x: 12 + glyph.corner.x + 3,
                    y: 150 + glyph.corner.y,
                },
                (0, 132, 0),
            ),
        ] {
            let pixel = GetFramePixel(screen, point);
            assert_eq!((pixel.r, pixel.g, pixel.b), expected);
        }
        let set;
        (clip, set) = selected_clip();
        assert!(!set);
        assert_eq!((clip.extent.width, clip.extent.height), (320, 240));
        assert!(uqm_modes::ui::sis::draw_message(
            &game.kernel,
            320,
            c"A",
            -1,
            -1,
            0
        ));
        assert_eq!(SetContext(context), context);
        uqm_engine::graphics::state::flush();
        let pixel = GetFramePixel(screen, Point { x: 8, y: 3 });
        assert_eq!((pixel.r, pixel.g, pixel.b), (0, 0, 165));
        let long_label = CString::new("A".repeat(100))
            .unwrap_or_else(|error| panic!("invalid fixture label: {error}"));
        assert!(!uqm_modes::ui::sis::draw_message(
            &game.kernel,
            320,
            &long_label,
            0,
            -1,
            0
        ));
        assert_eq!(SetContext(context), context);
        assert!(!selected_clip().1);
        let icons = uqm_engine::graphics::drawable::LoadedDrawable::create(2, 1, 1, 8)
            .unwrap_or_else(|| panic!("fixture icon allocation failed"));
        let retained = icons.lease();
        game.kernel.play_frame = retained
            .frame(0)
            .unwrap_or_else(|| panic!("fixture icon frame is absent"));
        drop(icons);
        assert_eq!(
            retained
                .context_frame(0)
                .wrapped_frame(u16::MAX)
                .rect()
                .extent
                .width,
            1
        );
        status.set_clip(Rect {
            corner: Point { x: 256, y: 0 },
            extent: Extent {
                width: 64,
                height: 240,
            },
        });
        SetContext(context);
        uqm_modes::ui::menu::draw_menu_state(
            &game.kernel,
            &game.world,
            (320, 240),
            uqm_modes::ui::menu_state::PM_CONVERSE,
            0,
        );
        assert_eq!(SetContext(context), context);
        uqm_engine::graphics::state::flush();
        let pixel = GetFramePixel(screen, Point { x: 259, y: 173 });
        assert_eq!((pixel.r, pixel.g, pixel.b), (82, 82, 82));
        game.kernel
            .configuration
            .update_preferences(|preferences| preferences.menu = 2);
        let menu_text_count = uqm_modes::starbase::ids::MAINMENU_STRING_BASE as usize
            + usize::from(uqm_modes::ui::menu_state::PM_ENCOUNTER_GAME_MENU)
            + 1;
        game.kernel
            .install_game_text(uqm_engine::strings::owned_table::LoadedTextTable::fixture(
                vec![b"A\0".to_vec(); menu_text_count],
            ));
        game.kernel.star_con_font = game.kernel.tiny_font;
        uqm_modes::ui::menu::draw_menu_state(
            &game.kernel,
            &game.world,
            (320, 240),
            uqm_modes::ui::menu_state::PM_CONVERSE,
            -1,
        );
        assert_eq!(SetContext(context), context);
        game.kernel.space_context = context;
        SetContext(previous);
        uqm_modes::ui::border::init_siscontexts(&game.kernel, (320, 240));
        assert_eq!(SetContext(context), context);
        let set;
        (clip, set) = selected_clip();
        assert!(set);
        assert_eq!((clip.corner.x, clip.corner.y), (7, 10));
        assert_eq!((clip.extent.width, clip.extent.height), (242, 227));
        drop(status);
        SetContext(None);
        drop(game);
        drop(selected);
        drop((context_owner, previous_owner));
        uqm_engine::resource::uninit();
        drop(drawable);
        uqm_engine::graphics::present::uninit_graphics();
    }
}

#[test]
fn status_labels_read_the_supplied_story_preferences_date_and_resource_units() {
    use uqm_engine::strings::owned_table::LoadedTextTable;
    use uqm_engine::StoryKey;
    use uqm_modes::starbase::ids::{MONTHS_STRING_BASE, STATUS_STRING_BASE};
    use uqm_modes::ui::sis::{format_date, status_message_text};
    let mut strings = vec![vec![0]; (STATUS_STRING_BASE + 3).max(MONTHS_STRING_BASE + 12) as usize];
    strings[STATUS_STRING_BASE as usize] = b"Cr\0".to_vec();
    strings[(STATUS_STRING_BASE + 1) as usize] = b"RU\0".to_vec();
    strings[(STATUS_STRING_BASE + 2) as usize] = b"Unlimited\0".to_vec();
    strings[MONTHS_STRING_BASE as usize] = b"Jan\0".to_vec();
    let mut game = Game::new();
    game.kernel
        .install_game_text(LoadedTextTable::fixture(strings.clone()));
    game.world.story = Some(uqm_engine::world::story::StoryState::new());
    game.kernel.set_status_message_mode(2);
    game.world.data.flagship.res_units = 42;
    assert_eq!(
        status_message_text(&game.kernel, &game.world),
        c"42 RU".to_owned()
    );
    game.world.set_story_state(StoryKey::CHMMR_BOMB_STATE, 2);
    game.kernel
        .configuration
        .update_preferences(|preferences| preferences.menu = 2);
    assert_eq!(
        status_message_text(&game.kernel, &game.world),
        c"Unlimited RU".to_owned()
    );
    game.kernel
        .configuration
        .update_preferences(|preferences| preferences.menu = 0);
    assert_eq!(
        status_message_text(&game.kernel, &game.world),
        c"∞ RU".to_owned()
    );
    game.kernel.set_status_message_mode(3);
    game.world.set_story_state(StoryKey::MELNORME_CREDIT0, 255);
    game.world.set_story_state(StoryKey::MELNORME_CREDIT1, 171);
    assert_eq!(
        status_message_text(&game.kernel, &game.world),
        c"44031 Cr".to_owned()
    );
    game.kernel.set_status_message_mode(1);
    game.world.data.game.game_clock.month_index = 1;
    game.world.data.game.game_clock.day_index = 2;
    game.world.data.game.game_clock.year_index = 2155;
    assert_eq!(
        status_message_text(&game.kernel, &game.world),
        c"Jan 02·2155".to_owned()
    );
    assert_eq!(
        format_date(c"Jul", 31, u16::MAX),
        c"Jul 31·65535".to_owned()
    );
    strings[STATUS_STRING_BASE as usize] = [vec![b'x'; 140], vec![0]].concat();
    game.kernel
        .install_game_text(LoadedTextTable::fixture(strings));
    game.kernel.set_status_message_mode(3);
    let bounded = status_message_text(&game.kernel, &game.world);
    assert_eq!(bounded.as_bytes().len(), 127);
    assert!(bounded.as_bytes().starts_with(b"44031 "));
    assert!(bounded.as_bytes()[6..].iter().all(|&byte| byte == b'x'));
}

#[test]
#[forbid(unsafe_code)]
fn message_labels_use_the_borrowed_catalog_and_hyperspace_story() {
    use uqm_engine::battle::flags::IN_LAST_BATTLE;
    use uqm_engine::strings::owned_table::LoadedTextTable;
    use uqm_engine::world::activity::activity_kinds::{IN_ENCOUNTER, IN_INTERPLANETARY};
    use uqm_engine::world::IN_HYPERSPACE;
    use uqm_engine::{
        world::{CStarDesc, StarId, CATALOG_LEN},
        StoryKey,
    };
    use uqm_modes::starbase::ids::{NAVIGATION_STRING_BASE, STAR_NUMBER_BASE};
    use uqm_modes::ui::sis::default_message_text;
    let mut strings =
        vec![vec![0]; (NAVIGATION_STRING_BASE as usize + 2).max(STAR_NUMBER_BASE as usize + 1)];
    strings[NAVIGATION_STRING_BASE as usize] = b"Hyperspace\0".to_vec();
    strings[(NAVIGATION_STRING_BASE + 1) as usize] = b"Quasispace\0".to_vec();
    strings[STAR_NUMBER_BASE as usize] = b"Alpha\0".to_vec();
    strings[2] = b"Centauri\0".to_vec();
    let catalog = [CStarDesc {
        star_pt: Point { x: 0, y: 0 },
        star_type: 0,
        index: 0,
        prefix: 1,
        postfix: 2,
    }; CATALOG_LEN];
    let mut game = Game::new();
    game.kernel
        .install_game_text(LoadedTextTable::fixture(strings));
    game.world.story = Some(uqm_engine::world::story::StoryState::new());
    game.world.set_catalog(std::borrow::Cow::Owned(catalog));
    game.world.current_star = Some(StarId(0));
    for activity in [IN_INTERPLANETARY, IN_LAST_BATTLE as u16] {
        game.world.set_activity(activity);
        assert_eq!(
            default_message_text(&game.kernel, &game.world),
            c"Alpha Centauri".to_owned()
        );
    }
    game.world.set_activity(IN_HYPERSPACE as u16);
    assert_eq!(
        default_message_text(&game.kernel, &game.world),
        c"Hyperspace".to_owned()
    );
    game.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 2);
    assert_eq!(
        default_message_text(&game.kernel, &game.world),
        c"Quasispace".to_owned()
    );
    game.world.set_activity(IN_ENCOUNTER);
    assert_eq!(
        default_message_text(&game.kernel, &game.world),
        c"".to_owned()
    );
}
