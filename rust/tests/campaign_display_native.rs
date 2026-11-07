//! Campaign labels borrow unpublished owners and retain their display resources.
use std::{ffi::CString, ptr};
use uqm_rust::{
    engine::Game,
    graphics::{
        context_abi::*,
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::*,
        native_abi::*,
        sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
    },
    io::uio_bridge as uio,
};

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
    let host = CString::new(directory.path().to_str().unwrap()).unwrap();
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    // SAFETY: this test process owns SDL, its UIO mount and all native fixture allocations.
    unsafe {
        assert_eq!(
            TFB_InitGraphics(
                1,
                0,
                ptr::from_ref(&GraphicsOptions::default()).cast(),
                320,
                240
            ),
            0
        );
        uio::uio_init();
        let repo = uio::uio_openRepository(0);
        assert!(!uio::uio_mountDir(
            repo,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut()
        )
        .is_null());
        let content = uio::uio_openDir(repo, c"/".as_ptr(), 0);
        let previous_content = uqm_rust::mainloop::dir_prep::contentDir;
        uqm_rust::mainloop::dir_prep::contentDir = content.cast();
        uqm_rust::resource::ffi_bridge::InitResourceSystem();
        uqm_rust::resource::ffi_bridge::LoadResourceIndex(
            content.cast(),
            c"fixture.rmp".as_ptr(),
            ptr::null(),
        );
        let font = uqm_rust::graphics::native_font::LoadedFont::load(c"fixture.tiny").unwrap();
        let glyph = font
            .lease()
            .measure(
                c"A",
                POINT { x: 27, y: 6 },
                TEXT_ALIGN_ALIGN_CENTER,
                u16::MAX,
            )
            .rect;
        uqm_rust::mainloop::dir_prep::contentDir = previous_content;
        uio::uio_closeDir(content);
        uio::uio_closeRepository(repo);
        uio::uio_unInit();
        #[cfg(feature = "debug-process")]
        let context = CreateContextAux(c"campaign title".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let context = CreateContextAux();
        #[cfg(feature = "debug-process")]
        let previous = CreateContextAux(c"previous context".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let previous = CreateContextAux();
        let drawable = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        let screen = drawable.lease().frame(0).unwrap();
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
        uqm_rust::ui::sis::draw_title(&game.kernel, 320, c"A");
        assert_eq!(SetContext(previous), previous);
        TFB_FlushGraphics();
        for (point, expected) in [
            (POINT { x: 193, y: 2 }, (0, 0, 165)),
            (POINT { x: 247, y: 8 }, (0, 0, 165)),
            (POINT { x: 192, y: 2 }, (9, 17, 25)),
            (POINT { x: 193, y: 9 }, (9, 17, 25)),
        ] {
            let pixel = GetFramePixel(screen, point);
            assert_eq!((pixel.r, pixel.g, pixel.b), expected);
        }
        let text_pixel = GetFramePixel(
            screen,
            POINT {
                x: 193 + glyph.corner.x,
                y: 2 + glyph.corner.y,
            },
        );
        assert_eq!((text_pixel.r, text_pixel.g, text_pixel.b), (222, 0, 222));
        SetContext(context);
        let mut clip = RECT::default();
        assert_eq!(GetContextClipRect(&mut clip), 0);
        assert_eq!((clip.extent.width, clip.extent.height), (320, 240));
        let status = ContextLease::require(previous);
        status.set_frame(drawable.lease().context_frame(0));
        status.set_clip(RECT {
            corner: POINT { x: 10, y: 20 },
            extent: EXTENT {
                width: 64,
                height: 180,
            },
        });
        game.kernel.status_context = previous;
        SetContext(context);
        uqm_rust::ui::sis::draw_status_message(&game.kernel, &game.world, Some(c"A"));
        assert_eq!(SetContext(context), context);
        TFB_FlushGraphics();
        for (point, expected) in [
            (POINT { x: 12, y: 150 }, (0, 66, 0)),
            (POINT { x: 71, y: 156 }, (0, 66, 0)),
            (POINT { x: 11, y: 150 }, (9, 17, 25)),
            (
                POINT {
                    x: 12 + glyph.corner.x + 3,
                    y: 150 + glyph.corner.y,
                },
                (0, 132, 0),
            ),
        ] {
            let pixel = GetFramePixel(screen, point);
            assert_eq!((pixel.r, pixel.g, pixel.b), expected);
        }
        assert_eq!(GetContextClipRect(&mut clip), 0);
        assert_eq!((clip.extent.width, clip.extent.height), (320, 240));
        drop(status);
        SetContext(ptr::null_mut());
        drop(game);
        drop(selected);
        DestroyContext(context);
        DestroyContext(previous);
        uqm_rust::resource::ffi_bridge::UninitResourceSystem();
        drop(drawable);
        TFB_UninitGraphics();
    }
}

#[test]
fn status_labels_read_the_supplied_story_preferences_date_and_resource_units() {
    use uqm_rust::{
        engine::StoryKey,
        ships::native::abi::{MONTHS_STRING_BASE, STATUS_STRING_BASE},
        strings::owned_table::LoadedTextTable,
        ui::sis::{format_date, status_message_text},
    };
    let mut strings = vec![vec![0]; (STATUS_STRING_BASE + 3).max(MONTHS_STRING_BASE + 12) as usize];
    strings[STATUS_STRING_BASE as usize] = b"Cr\0".to_vec();
    strings[(STATUS_STRING_BASE + 1) as usize] = b"RU\0".to_vec();
    strings[(STATUS_STRING_BASE + 2) as usize] = b"Unlimited\0".to_vec();
    strings[MONTHS_STRING_BASE as usize] = b"Jan\0".to_vec();
    let mut game = Game::new();
    game.kernel
        .install_game_text(LoadedTextTable::fixture(strings.clone()));
    game.world.story = Some(uqm_rust::engine::world::story::GameState::new());
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
