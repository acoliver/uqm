//! The real options/settings/name chain borrows one campaign through nested input loops.
#[path = "support/campaign_display.rs"]
mod campaign_display;
#[path = "support/native_process.rs"]
mod native_test_process;

use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use uqm_rust::{
    engine::{
        battle::Cx,
        input::{frames::InputState, Frame, InputRuntime, InputSession},
        observe::{CampaignView, Io, Observer, Unobserved},
        Game,
    },
    navigation::abi::{KEY_MENU_CANCEL, KEY_MENU_EDIT_CANCEL, KEY_MENU_RIGHT, KEY_MENU_SELECT},
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
    use uqm_rust::navigation::abi::*;
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
    // SAFETY: this serial fixture lends the actual campaign for the whole modal chain.
    let result = unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::app::bridge::enter(|cx| {
                uqm_rust::front_end::gameopt::game_options(cx, &mut io, None)
            })
        })
    };
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
