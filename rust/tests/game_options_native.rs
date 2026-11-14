//! The real options/settings/name chain borrows one campaign through nested input loops.
#[path = "support/campaign_display.rs"]
mod campaign_display;
#[path = "support/native_process.rs"]
mod native_test_process;

use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use uqm_rust::{
    engine::input::frames::{
        KEY_MENU_CANCEL, KEY_MENU_EDIT_CANCEL, KEY_MENU_RIGHT, KEY_MENU_SELECT,
    },
    engine::{
        battle::Cx,
        input::{frames::InputState, Frame, InputRuntime, InputSession},
        observe::{CampaignView, Io, Observer, Unobserved},
        Game,
    },
};

struct ScriptedInput {
    input: InputSession,
    keys: RefCell<VecDeque<usize>>,
}

impl InputRuntime for ScriptedInput {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {}
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {}
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("options must not poll battle pacing")
    }
    fn flush(&self) {
        self.input.clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected quit confirmation")
    }
    fn update(&self, _: &mut Cx<'_>) {}
    fn battle_input(&self, _: &uqm_rust::engine::battle::input::InputContext) -> u8 {
        panic!("options must not read ship controls")
    }
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, _: bool, frame: &mut dyn Frame) {
        loop {
            let key = self
                .keys
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| panic!("options consumed its input script"));
            let mut pulse = InputState::default();
            pulse.menu[key] = 1;
            self.input.advance(pulse, pulse, 0);
            if !frame.frame(cx, io) {
                break;
            }
        }
    }
}

fn options(keys: &[u32]) -> (u32, u8, [u8; 16], [u8; 16]) {
    use campaign_display::CampaignDisplay;
    use uqm_rust::graphics::context_abi::{ContextFrame, ContextLease};
    use uqm_rust::starbase::ids::{
        MAINMENU_STRING_BASE, MONTHS_STRING_BASE, NAMING_STRING_BASE, OPTION_STRING_BASE,
    };
    let mut game = Game::new();
    let display = CampaignDisplay::install(&mut game)
        .unwrap_or_else(|error| panic!("options display: {error}"));
    uqm_rust::callback::ffi::Alarm_init();
    game.kernel.screen_context = game.kernel.status_context;
    game.kernel.space_context = game.kernel.status_context;
    game.kernel.star_con_font = game.kernel.tiny_font;
    game.kernel.configuration.update_preferences(|preferences| {
        preferences.menu = 2;
        preferences.fonts = 1;
    });
    game.kernel
        .install_game_text(uqm_rust::strings::owned_table::LoadedTextTable::fixture(
            vec![
                b"A\0".to_vec();
                (OPTION_STRING_BASE
                    .max(NAMING_STRING_BASE)
                    .max(MONTHS_STRING_BASE)
                    .max(MAINMENU_STRING_BASE)
                    + 100) as usize
            ],
        ));
    game.world.data.game.game_clock.month_index = 1;
    game.world.data.game.game_clock.day_index = 7;
    game.world.data.game.game_clock.year_index = 2155;
    game.world.data.flagship.commander_name[..2].copy_from_slice(b"A\0");
    game.world.data.flagship.ship_name[..2].copy_from_slice(b"A\0");
    let context = ContextLease::require(game.kernel.status_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    let runtime = Rc::new(ScriptedInput {
        input: InputSession::default(),
        keys: RefCell::new(keys.iter().map(|&key| key as usize).collect()),
    });
    game.kernel.install_input_runtime(runtime.clone());
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut battle = uqm_rust::engine::battle::Battle::new();
    let result = uqm_rust::front_end::gameopt::game_options(
        &mut Cx {
            b: &mut battle,
            world: &mut game.world,
            kernel: &mut game.kernel,
            rng: || panic!("settings must not consume RNG"),
            solar_flight: None,
            solar_frame: None,
        },
        &mut io,
        None,
    );
    assert!(runtime.keys.borrow().is_empty());
    assert!(game.kernel.sis_highlight.context.borrow().is_none());
    let state = (
        result,
        game.world.data.game.glob_flags,
        game.world.data.flagship.commander_name,
        game.world.data.flagship.ship_name,
    );
    drop(context);
    uqm_rust::callback::ffi::Alarm_uninit();
    display.finish(game);
    state
}

#[test]
#[serial_test::serial]
fn options_cancel_returns_without_changing_campaign_settings() {
    if !native_test_process::own_graphics_process(
        "options_cancel_returns_without_changing_campaign_settings",
    ) {
        return;
    }
    let (result, flags, captain, ship) = options(&[KEY_MENU_CANCEL]);
    assert_eq!((result, flags), (1, 0));
    assert_eq!(&captain[..2], b"A\0");
    assert_eq!(&ship[..2], b"A\0");
}

#[test]
#[serial_test::serial]
fn nested_settings_toggle_audio_and_cycle_every_cyborg_speed() {
    if !native_test_process::own_graphics_process(
        "nested_settings_toggle_audio_and_cycle_every_cyborg_speed",
    ) {
        return;
    }
    let next = KEY_MENU_RIGHT;
    let select = KEY_MENU_SELECT;
    let cancel = KEY_MENU_CANCEL;
    let (result, flags, _, _) = options(&[
        next, next, next, select, select, select, next, select, select, next, select, select,
        select, select, cancel, cancel,
    ]);
    assert_eq!((result, flags), (1, 0));
}

#[test]
#[serial_test::serial]
fn captain_and_ship_editors_keep_cancelled_names_and_accept_empty_names() {
    if !native_test_process::own_graphics_process(
        "captain_and_ship_editors_keep_cancelled_names_and_accept_empty_names",
    ) {
        return;
    }
    for accept in [false, true] {
        let next = KEY_MENU_RIGHT;
        let select = KEY_MENU_SELECT;
        let finish = if accept { select } else { KEY_MENU_EDIT_CANCEL };
        let (result, flags, captain, ship) = options(&[
            next,
            next,
            next,
            select,
            next,
            next,
            next,
            select,
            finish,
            next,
            select,
            finish,
            KEY_MENU_CANCEL,
            KEY_MENU_CANCEL,
        ]);
        assert_eq!((result, flags), (1, 0));
        assert_eq!(captain[0], if accept { 0 } else { b'A' });
        assert_eq!(ship[0], if accept { 0 } else { b'A' });
    }
}

#[test]
#[serial_test::serial]
fn supplied_graphics_keeps_fonts_and_text_after_the_campaign_is_dropped() {
    if !native_test_process::own_graphics_process(
        "supplied_graphics_keeps_fonts_and_text_after_the_campaign_is_dropped",
    ) {
        return;
    }
    use uqm_rust::{
        graphics::{
            context_abi::{ContextFrame, ContextLease},
            native_abi::POINT,
        },
        starbase::{
            gfx::{Align, ContextId, FontId, Gfx, Point, Rect, Rgba, TextDraw},
            graphics::Graphics,
        },
    };
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game)
        .unwrap_or_else(|error| panic!("retained graphics fixture: {error}"));
    let context = ContextLease::require(game.kernel.status_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    context.replace_font(None);
    let graphics = Graphics::new(&game.kernel, (320, 240));
    drop(game);
    graphics.set_context(ContextId::Status);
    let previous = graphics.set_font(FontId::Tiny);
    assert!(graphics.font_leading() > 0);
    assert_eq!(graphics.game_string(0), "A");
    graphics.set_bg_color(Rgba::rgb15(0, 0, 0));
    graphics.clear_drawable();
    graphics.set_fg_color(Rgba::rgb15(31, 31, 31));
    graphics.draw_text(&TextDraw {
        baseline: Point { x: 4, y: 10 },
        align: Align::Left,
        text: graphics.game_string(0),
    });
    context.flush();
    let target = context.target();
    assert!(
        (0..16).any(|y| (0..16).any(|x| {
            let pixel = target.pixel(POINT { x, y });
            (pixel.r, pixel.g, pixel.b) == (255, 255, 255)
        })),
        "the retained font must still render its glyph"
    );
    graphics.set_fg_color(Rgba::rgb15(31, 0, 0));
    graphics.draw_rect(Rect {
        corner: Point { x: 20, y: 20 },
        width: 5,
        height: 5,
    });
    graphics.draw_point(Point { x: 27, y: 22 });
    context.flush();
    for (x, y) in [(20, 20), (24, 20), (20, 24), (24, 24), (27, 22)] {
        let pixel = target.pixel(POINT { x, y });
        assert_eq!((pixel.r, pixel.g, pixel.b), (255, 0, 0));
    }
    let interior = target.pixel(POINT { x: 22, y: 22 });
    assert_eq!((interior.r, interior.g, interior.b), (0, 0, 0));
    graphics.restore_font(previous);
    drop(graphics);
    drop(context);
    drop(display);
}

#[test]
#[serial_test::serial]
fn supplied_graphics_restores_context_selection_and_retained_font_effects() {
    if !native_test_process::own_graphics_process(
        "supplied_graphics_restores_context_selection_and_retained_font_effects",
    ) {
        return;
    }
    use uqm_rust::{
        graphics::{
            context_abi::{ContextLease, LoadedContext},
            drawable_abi::LoadedDrawable,
        },
        starbase::{
            gfx::{ContextId, FontId, Gfx},
            graphics::Graphics,
        },
    };
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game)
        .unwrap_or_else(|error| panic!("retained graphics fixture: {error}"));
    let status = ContextLease::require(game.kernel.status_context);
    let space = LoadedContext::create(c"retained menu space");
    let space_raw = space.lease().as_raw();
    game.kernel.space_context = space_raw;
    let frames = LoadedDrawable::create(8, 1, 1, 3)
        .unwrap_or_else(|| panic!("retained menu frames allocation failed"));
    let first = frames.lease().context_frame(0).as_raw();
    game.kernel.play_frame = first;
    game.kernel.font_grad_frame = first;
    let graphics = Graphics::new(&game.kernel, (320, 240));
    drop(frames);
    drop(game);
    assert_eq!(graphics.screen_size(), (320, 240));
    assert_eq!(graphics.off_screen_context().as_raw(), status.as_raw());
    status.select();
    let old_context = graphics.set_context(ContextId::Space);
    assert_eq!(status.selected_context().unwrap().as_raw(), space_raw);
    graphics.restore_context(old_context);
    assert_eq!(status.selected_context().unwrap().as_raw(), status.as_raw());
    let first = graphics.play_frame();
    assert_eq!(graphics.frame_count(first), 3);
    let second = graphics.inc_frame(first);
    assert_eq!(graphics.frame_index(second), 1);
    assert_eq!(graphics.dec_frame(second), first);
    let old_effect = graphics.set_font_effect(Some(1));
    assert_eq!(status.font_effect().as_raw() as usize, second);
    let nested_effect = graphics.set_font_effect(None);
    assert!(status.font_effect().as_raw().is_null());
    graphics.restore_font_effect(nested_effect);
    assert_eq!(status.font_effect().as_raw() as usize, second);
    graphics.restore_font_effect(old_effect);
    assert!(status.font_effect().as_raw().is_null());
    let old_font = graphics.set_font(FontId::Micro);
    assert!(graphics.font_leading() > 0);
    graphics.restore_font(old_font);
    drop(graphics);
    drop(space);
    drop(status);
    drop(display);
}
