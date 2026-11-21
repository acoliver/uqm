//! Native input repeat, nested loops and battle controls.

use uqm_engine::threading::pacing::yield_input_loop;

#[cfg(feature = "debug-process")]
use uqm_engine::battle::statistics::{delta_crew, delta_energy};
use uqm_engine::input_port::frames::{
    KEY_DEBUG, KEY_EXIT, KEY_MENU_BACKSPACE, KEY_MENU_CANCEL, KEY_MENU_DELETE, KEY_MENU_DOWN,
    KEY_MENU_LEFT, KEY_MENU_PAGE_DOWN, KEY_MENU_PAGE_UP, KEY_MENU_RIGHT, KEY_MENU_SELECT,
    KEY_MENU_SPECIAL, KEY_MENU_UP, KEY_PAUSE,
};
use uqm_engine::{
    battle::Cx,
    input::{InputControls, InputSession},
};
#[cfg(feature = "debug-process")]
use uqm_modes::front_end::uqmdebug::debug_key_pressed_synchronous;

pub(crate) mod live {
    use super::*;

    pub fn reset_key_repeat(input: &InputControls) {
        input.reset_repeat(crate::time::counter_abi::get_time_counter());
    }

    pub fn update_input_state(cx: &mut Cx<'_>, input: &InputSession) {
        if !uqm_engine::graphics::state::game_active() {
            uqm_modes::ui::util::sleep_game(cx);
        }
        if input.paused() {
            uqm_modes::ui::util::pause_game(cx, input);
        }
        if input.exit_requested() {
            confirm_exit(cx, input);
        }
        let before_frame = input.immediate();
        uqm_engine::input_port::sdl_input::begin_input_frame();
        let after_frame = input.immediate();
        let now = crate::time::counter_abi::get_time_counter();
        input.advance(before_frame, after_frame, now);
        let current = input.current();
        if current.menu[KEY_PAUSE as usize] != 0 {
            input.set_paused(true);
        }
        if current.menu[KEY_EXIT as usize] != 0 {
            input.set_exit_requested(true);
        }
        #[cfg(feature = "debug-process")]
        if input.pulsed().menu[KEY_DEBUG as usize] != 0 {
            debug_key_pressed_synchronous(cx, delta_crew, delta_energy);
        }
    }

    pub fn set_input_callback(
        input: &InputSession,
        callback: Option<uqm_engine::input::InputFrameHook>,
    ) -> Option<uqm_engine::input::InputFrameHook> {
        input.replace_callback(callback)
    }

    pub fn set_menu_repeat_delay(
        input: &InputControls,
        min: u32,
        max: u32,
        step: u32,
        gestalt: u32,
    ) {
        input.configure_repeat(
            min,
            max,
            step,
            gestalt != 0,
            crate::time::counter_abi::get_time_counter(),
        );
    }

    pub fn flush_input(input: &InputControls) {
        uqm_engine::input_port::sdl_input::reset_controls();
        input.clear_menu_state();
    }

    pub(super) fn menu_sound_flags(state: &uqm_engine::input::frames::InputState) -> u16 {
        [
            KEY_MENU_UP,
            KEY_MENU_DOWN,
            KEY_MENU_LEFT,
            KEY_MENU_RIGHT,
            KEY_MENU_SELECT,
            KEY_MENU_CANCEL,
            KEY_MENU_SPECIAL,
            KEY_MENU_PAGE_UP,
            KEY_MENU_PAGE_DOWN,
            KEY_MENU_DELETE,
        ]
        .into_iter()
        .enumerate()
        .fold(0, |flags, (bit, key)| {
            flags | (u16::from(state.menu[key as usize] != 0) << bit)
        }) | (u16::from(state.menu[KEY_MENU_BACKSPACE as usize] != 0) << 9)
    }

    pub fn any_button_press(cx: &mut Cx<'_>, input: &InputSession, _special: u32) -> u32 {
        update_input_state(cx, input);
        u32::from(input.current().any_button_pressed())
    }

    pub fn confirm_exit(cx: &mut Cx<'_>, input: &InputSession) -> u32 {
        let old = input.repeat_config();
        input.set_default_menu_repeat(crate::time::counter_abi::get_time_counter());
        let result = uqm_modes::ui::confirm::do_confirm_exit(cx, input);
        set_menu_repeat_delay(input, old.0, old.1, old.2, u32::from(old.3));
        result
    }
}
#[path = "input_driver.rs"]
mod driver;
pub use driver::NativeInputRuntime;
pub use live::*;
