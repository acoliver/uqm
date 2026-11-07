//! Retained campaign panels select location text and preserve edit/highlight state.
use uqm_rust::{engine, ships, ui};
use uqm_rust::{
    engine::{
        battle::{Battle, Cx},
        world::{CStarDesc, StarId, CATALOG_LEN},
        Game,
    },
    graphics::{
        context_abi::{ContextFrame, ContextLease},
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::GetFramePixel,
        native_abi::*,
    },
    ships::native::abi::{
        DSME_BLOCKCUR, DSME_CLEARFR, DSME_MYCOLOR, DSME_SETFR, IN_HYPERSPACE, IN_INTERPLANETARY,
        OPT_PC,
    },
};

#[path = "tests/support/campaign_display.rs"]
mod campaign_display;
// The encounter engine is excluded from lib unit tests. This crate-root harness shares
// its private helper's source path, so LLVM attributes these real-owner tests to that file.
#[path = "src/battle/encount_native/engine/display.rs"]
mod encounter_display;

fn title_pixels(game: &Game) -> Vec<(u8, u8, u8)> {
    // SAFETY: each serialized fixture retains its screen and drains its drawing queue before reading.
    unsafe {
        TFB_FlushGraphics();
        (2..9)
            .flat_map(|y| {
                (193..248).map(move |x| {
                    let pixel = GetFramePixel(game.kernel.screen, POINT { x, y });
                    (pixel.r, pixel.g, pixel.b)
                })
            })
            .collect()
    }
}

#[test]
#[serial_test::serial]
fn ui_encounter_title_uses_ship_star_or_planet_from_the_supplied_world() {
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game).unwrap();
    let mut battle = Battle::new();
    let ship = POINT { x: 1111, y: 1111 };
    let star = POINT { x: 2222, y: 2222 };
    let catalog = [CStarDesc {
        star_pt: star,
        star_type: 0,
        index: 0,
        prefix: 0,
        postfix: 0,
    }; CATALOG_LEN];
    game.world.set_activity(IN_HYPERSPACE as u16);
    game.world.data.game.ship_stamp.origin = ship;
    game.world.data.game.ip_planet = 1;
    game.world.data.flagship.planet_name[..2].copy_from_slice(b"A\0");
    encounter_display::draw_title(
        &mut Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0),
        None,
    );
    let ship_pixels = title_pixels(&game);
    ui::sis::draw_hyper_coords(&game.kernel, 320, ship);
    assert_eq!(title_pixels(&game), ship_pixels);
    ui::sis::draw_hyper_coords(&game.kernel, 320, star);
    assert_ne!(title_pixels(&game), ship_pixels);

    game.world.set_activity(IN_INTERPLANETARY as u16);
    game.world.data.game.ip_planet = 0;
    game.world.current_star = Some(StarId(0));
    encounter_display::draw_title(
        &mut Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0),
        Some(&catalog),
    );
    let star_pixels = title_pixels(&game);
    ui::sis::draw_hyper_coords(&game.kernel, 320, star);
    assert_eq!(title_pixels(&game), star_pixels);
    assert_ne!(star_pixels, ship_pixels);

    game.world.data.game.ip_planet = 1;
    game.world.current_star = None;
    encounter_display::draw_title(
        &mut Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0),
        None,
    );
    let planet_pixels = title_pixels(&game);
    ui::sis::draw_title(&game.kernel, 320, c"A");
    assert_eq!(title_pixels(&game), planet_pixels);
    assert_ne!(planet_pixels, star_pixels);
    display.finish(game);
}

#[test]
#[serial_test::serial]
fn ui_message_edits_draw_bounded_cursor_and_split_color_then_restore_the_context() {
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game).unwrap();
    uqm_rust::callback::ffi::Alarm_init();
    game.kernel.screen_context = game.kernel.off_screen_context;
    let context = ContextLease::require(game.kernel.off_screen_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    let font = game.kernel.tiny_font_lease().unwrap();
    let measurement = font.measure(c"AA", POINT { x: 3, y: 6 }, TEXT_ALIGN_ALIGN_LEFT, u16::MAX);
    let glyph = measurement.rect.corner;
    assert_eq!(measurement.characters, 2);
    let second = i16::from(measurement.deltas[0]);
    for (cursor, extra, flags) in [
        (0, 0, DSME_BLOCKCUR),
        (1, 1, DSME_BLOCKCUR | DSME_CLEARFR),
        (2, -1, 0),
        (99, 99, 0),
        (-1, 0, 0),
        (0, -1, 0),
    ] {
        assert!(ui::sis::draw_message(
            &game.kernel,
            320,
            c"AA",
            cursor,
            extra,
            flags as u16
        ));
        assert_eq!(context.clip_rect().extent.width, 320);
        // SAFETY: the fixture owns the retained screen; flushing completes the edit before pixel reads.
        unsafe {
            TFB_FlushGraphics();
            for (offset, color) in [
                (
                    0,
                    if extra == 0 {
                        (148, 0, 148)
                    } else {
                        (222, 0, 222)
                    },
                ),
                (
                    second,
                    if extra == 0 || extra == 1 {
                        (148, 0, 148)
                    } else {
                        (222, 0, 222)
                    },
                ),
            ] {
                let pixel = GetFramePixel(
                    game.kernel.screen,
                    POINT {
                        x: 8 + glyph.x + offset,
                        y: 2 + glyph.y,
                    },
                );
                assert_eq!((pixel.r, pixel.g, pixel.b), color);
            }
            if (0..=2).contains(&cursor) {
                let x = 8
                    + glyph.x
                    + measurement.deltas[..cursor as usize]
                        .iter()
                        .map(|&x| i16::from(x))
                        .sum::<i16>()
                    - i16::from(cursor < 2);
                let pixel = GetFramePixel(game.kernel.screen, POINT { x, y: 2 });
                assert_eq!((pixel.r, pixel.g, pixel.b), (0, 0, 0));
            }
        }
        assert_eq!(
            game.kernel.sis_highlight.context.borrow().is_some(),
            flags & DSME_SETFR != 0
        );
    }
    assert!(ui::sis::draw_message(
        &game.kernel,
        320,
        c"AA",
        -1,
        -1,
        DSME_SETFR as u16
    ));
    assert!(game.kernel.sis_highlight.context.borrow().is_some());
    assert!(ui::sis::draw_message(
        &game.kernel,
        320,
        c"AA",
        -1,
        -1,
        DSME_CLEARFR as u16
    ));
    assert!(game.kernel.sis_highlight.context.borrow().is_none());
    context.set_foreground(Color {
        r: 19,
        g: 37,
        b: 53,
        a: 255,
    });
    assert!(ui::sis::draw_message(
        &game.kernel,
        320,
        c"AA",
        -1,
        -1,
        DSME_MYCOLOR as u16
    ));
    assert_eq!(context.foreground().r, 19);
    game.kernel
        .configuration
        .update_preferences(|preferences| preferences.menu = OPT_PC as i32);
    ui::sis::set_menu_flash(&game.kernel, (320, 240), true);
    assert!(game.kernel.sis_highlight.context.borrow().is_some());
    ui::sis::set_menu_flash(&game.kernel, (320, 240), false);
    assert!(game.kernel.sis_highlight.context.borrow().is_none());
    game.kernel
        .configuration
        .update_preferences(|preferences| preferences.menu = ships::native::abi::OPT_3DO as i32);
    ui::sis::set_menu_flash(&game.kernel, (320, 240), false);
    assert!(game.kernel.sis_highlight.context.borrow().is_some());
    game.kernel.sis_highlight.context.borrow_mut().take();
    uqm_rust::callback::ffi::Alarm_uninit();
    drop(font);
    drop(context);
    display.finish(game);
}
