const GAMESTR_COUNT: usize = 579;
#[path = "../../../tests/support/conversation_display.rs"]
mod conversation_display;
use super::*;
use crate::graphics::{
    context_abi::{ContextFrame, LoadedContext},
    drawable_abi::LoadedDrawable,
};
use crate::{campaign_display_test::CampaignDisplay, engine::Game};

fn rgba(color: Color) -> (u8, u8, u8, u8) {
    (color.r, color.g, color.b, color.a)
}

fn gradient(game: &mut Game) -> LoadedDrawable {
    let drawable = LoadedDrawable::create(8, 1, 1, 6).unwrap();
    let first = drawable.lease().frame(0).unwrap();
    let owner = LoadedContext::create(c"equipment gradient fixture");
    let context = owner.lease();
    let first = ContextFrame::retain(std::ptr::NonNull::new(first));
    for index in 0..6 {
        context.set_frame(first.indexed_frame(index));
        context.fill_rectangle(
            rgb15(31, 31, 31),
            RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 1,
                    height: 1,
                },
            },
        );
    }
    context.flush();
    game.kernel.font_grad_frame = drawable.lease().frame(0).unwrap();
    drawable
}

fn check_equipment(game: &Game, context: &ContextLease, screen: &ContextFrame) {
    let font = game.kernel.battle_font().unwrap();
    let point = font
        .measure(
            c"A",
            POINT {
                x: 40,
                y: 227 - 3 * font.leading(),
            },
            TEXT_ALIGN_ALIGN_RIGHT,
            u16::MAX,
        )
        .rect
        .corner;
    let foreground = rgb15(3, 4, 5);
    context.set_foreground(foreground);
    draw_flagship_stats(&game.kernel, &game.world, (320, 240));
    context.flush();
    assert_eq!(rgba(screen.pixel(point)), rgba(rgb15(31, 31, 31)));
    assert_eq!(rgba(context.foreground()), rgba(foreground));
}

#[test]
#[serial_test::serial]
fn real_sis_and_menu_services_draw_with_retained_resources() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::real_sis_and_menu_services_draw_with_retained_resources"
    )) {
        return;
    }
    let mut game = Game::new();
    let display = CampaignDisplay::install(&mut game).unwrap();
    game.kernel.space_context = game.kernel.status_context;
    game.kernel.screen_context = game.kernel.status_context;
    game.kernel.play_frame = game.kernel.screen;
    game.kernel
        .configuration
        .update_preferences(|preferences| preferences.menu = 2);
    game.kernel
        .install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(
            vec![b"A\0".to_vec(); GAMESTR_COUNT as usize],
        ));
    let effect = gradient(&mut game);
    let context = ContextLease::require(game.kernel.status_context);
    let screen = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen));
    context.set_frame(screen.clone());
    context.clear_clip();
    repair_sis_border(&game.kernel, (320, 240));
    context.flush();
    for (point, color) in [
        (POINT { x: 6, y: 9 }, rgb15(16, 16, 16)),
        (POINT { x: 249, y: 9 }, rgb15(8, 8, 8)),
        (POINT { x: 6, y: 237 }, rgb15(8, 8, 8)),
    ] {
        assert_eq!(rgba(screen.pixel(point)), rgba(color));
    }
    let mut battle = crate::engine::battle::Battle::new();
    {
        let cx =
            crate::engine::battle::Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
                panic!("menu draw sampled RNG")
            });
        crate::ui::menu::draw_campaign_menu(&cx, crate::ui::menu_state::PM_CREW, 0);
    };
    context.flush();
    assert_eq!(
        rgba(screen.pixel(POINT { x: 258, y: 193 })),
        rgba(rgb15(15, 15, 15))
    );
    assert_eq!(
        rgba(screen.pixel(POINT { x: 270, y: 196 })),
        rgba(rgb15(10, 10, 31))
    );
    let foreground = rgb15(2, 3, 4);
    context.set_foreground(foreground);
    clear_sis_rect(&game.kernel, &mut game.world, (320, 240), 4);
    context.flush();
    assert_eq!(
        rgba(screen.pixel(POINT { x: 270, y: 196 })),
        rgba(rgb15(0, 0, 21))
    );
    assert_eq!(rgba(context.foreground()), rgba(foreground));
    check_equipment(&game, &context, &screen);
    assert_eq!(
        campaign_date(&game.kernel, 1, 9, 2155),
        c"A 09\xc2\xb72155".to_owned()
    );
    let rect = RECT {
        corner: POINT { x: 10, y: 10 },
        extent: EXTENT {
            width: 5,
            height: 5,
        },
    };
    crate::callback::ffi::Alarm_init();
    {
        use crate::ui::shipyard_selection::{dms_set_mode, ModifyMode};
        let input = crate::engine::input::InputSession::default();
        let mut cx =
            crate::engine::battle::Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
                panic!("shipyard selection sampled RNG")
            });
        for (mode, state, sounds, highlighted) in [
            (ModifyMode::Navigate, 0xf0, (15, 16), true),
            (ModifyMode::Navigate, 11, (15, 16), true),
            (ModifyMode::AddEscort, 0, (15, 16), true),
            (ModifyMode::EditCrew, 0xf0, (3, 48), true),
            (ModifyMode::EditCrew, 11, (3, 48), true),
            (ModifyMode::Exit, 0, (15, 16), false),
        ] {
            dms_set_mode(&mut cx, &input, state, mode);
            assert_eq!(input.menu_sounds(), sounds);
            assert_eq!(
                cx.kernel.sis_highlight.context.borrow().is_some(),
                highlighted
            );
        }
    }
    highlight_rect(&game.kernel, rect);
    assert!(game.kernel.sis_highlight.context.borrow().is_some());
    set_menu_flash(&game.kernel, (320, 240), false);
    assert!(game.kernel.sis_highlight.context.borrow().is_none());
    set_menu_flash(&game.kernel, (320, 240), true);
    assert!(game.kernel.sis_highlight.context.borrow().is_some());
    clear_flash(&game.kernel);
    assert!(game.kernel.sis_highlight.context.borrow().is_none());
    crate::callback::ffi::Alarm_uninit();
    drop(screen);
    drop(context);
    drop(effect);
    display.finish(game);
}

#[test]
#[serial_test::serial]
fn conversation_headers_use_the_campaign_display_resources() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::conversation_headers_use_the_campaign_display_resources"
    )) {
        return;
    }
    let mut game = Game::new();
    let display = conversation_display::ConversationDisplay::install(&mut game).unwrap();
    let frames = LoadedDrawable::create(8, 64, 140, 100).unwrap();
    game.kernel.flag_stat_frame = frames.lease().frame(0).unwrap();
    game.kernel.star_con_font = game.kernel.tiny_font;
    game.kernel.play_frame = game.kernel.screen;
    game.world.story = Some(crate::state::game_state::StoryState::new());
    game.world.data.flagship.planet_name[..2].copy_from_slice(b"A\0");
    let effect = gradient(&mut game);
    let screen = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen));
    for raw in [
        game.kernel.status_context,
        game.kernel.screen_context,
        game.kernel.space_context,
    ] {
        let context = ContextLease::require(raw);
        context.set_frame(screen.clone());
        context.clear_clip();
    }
    let context = ContextLease::require(game.kernel.screen_context);
    for title in [None, Some(0)] {
        crate::ui::border::draw_conversation_frame(
            &game.kernel,
            &mut game.world,
            (320, 240),
            title,
        );
        context.flush();
        let glyph = game
            .kernel
            .tiny_font_lease()
            .unwrap()
            .measure(
                c"A",
                POINT {
                    x: SIS_TITLE_WIDTH >> 1,
                    y: SIS_TITLE_HEIGHT - 2,
                },
                TEXT_ALIGN_ALIGN_CENTER,
                u16::MAX,
            )
            .rect
            .corner;
        let title_point = POINT {
            x: (i32::from(SIS_ORG_X) + 320 - 78 - i32::from(SIS_TITLE_BOX_WIDTH) + 1) as i16
                + glyph.x,
            y: SIS_ORG_Y - SIS_TITLE_HEIGHT + glyph.y,
        };
        assert_eq!(rgba(screen.pixel(title_point)), rgba(rgb15(27, 0, 27)));

        assert_eq!(
            rgba(screen.pixel(POINT { x: 6, y: 9 })),
            rgba(rgb15(16, 16, 16))
        );
        let space = ContextLease::require(game.kernel.space_context);
        assert_eq!(space.clip_rect().corner.x, 7);
        assert_eq!(space.clip_rect().corner.y, 10);
        assert_eq!(space.clip_rect().extent.width, 242);
        assert_eq!(space.clip_rect().extent.height, 227);
    }
    drop(screen);
    drop(context);
    drop(effect);
    drop(frames);
    display.finish(game);
}
