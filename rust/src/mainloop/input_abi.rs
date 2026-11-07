//! Native input repeat, nested loops and battle controls.

#[cfg(not(test))]
use crate::threading::pacing::yield_input_loop;

#[cfg(not(test))]
pub(crate) mod live {
    #![allow(non_snake_case)]
    use crate::engine::{
        battle::Cx,
        input::{InputControls, InputSession},
    };
    use crate::ships::native::abi::*;

    pub fn ResetKeyRepeat(input: &InputControls) {
        input.reset_repeat(crate::time::counter_abi::GetTimeCounter());
    }

    /// # Safety
    /// The initialized input runtime is retained on the game thread during nested UI calls.
    pub unsafe fn UpdateInputState(cx: &mut Cx<'_>, input: &InputSession) {
        if crate::graphics::sdl::lifecycle_abi::GameActive == 0 {
            cx.with_legacy(|| crate::ui::util::live::SleepGame());
        }
        if input.paused() {
            cx.with_legacy(|| crate::ui::util::live::pause_game(input));
        }
        if input.exit_requested() {
            ConfirmExit(cx, input);
        }
        let before_frame = input.immediate();
        crate::input::sdl_input::BeginInputFrame();
        let after_frame = input.immediate();
        let now = crate::time::counter_abi::GetTimeCounter();
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
            cx.with_legacy(|| crate::front_end::uqmdebug::debug_key_pressed_synchronous());
        }
    }

    pub fn SetInputCallback(
        input: &InputControls,
        callback: Option<crate::engine::input::InputFrameHook>,
    ) -> Option<crate::engine::input::InputFrameHook> {
        input.replace_callback(callback)
    }

    pub fn SetMenuRepeatDelay(input: &InputControls, min: u32, max: u32, step: u32, gestalt: u32) {
        input.configure_repeat(
            min,
            max,
            step,
            gestalt != 0,
            crate::time::counter_abi::GetTimeCounter(),
        );
    }

    pub fn SetDefaultMenuRepeatDelay(input: &InputControls) {
        SetMenuRepeatDelay(input, 840 / 12, 840 / 2, 840 / 12, 0);
    }

    /// # Safety
    /// The provider's initialized input vectors borrow this runtime's live buffer.
    pub unsafe fn FlushInput(input: &InputControls) {
        crate::input::sdl_input::TFB_ResetControls();
        input.clear_menu_state();
    }

    pub(super) fn menu_sound_flags(state: &crate::engine::input::frames::InputState) -> u16 {
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

    /// # Safety
    /// The initialized game-thread context must be published by the calling legacy UI.
    pub unsafe fn DoInput(reset: u32, frame: &mut dyn crate::engine::input::Frame) {
        crate::app::bridge::enter(|cx| {
            let runtime = cx.kernel.input_runtime();
            let mut observer = runtime.observer();
            let mut io = crate::engine::observe::Io {
                observer: &mut *observer,
            };
            runtime.run(cx, &mut io, reset != 0, frame);
        });
    }

    pub fn SetMenuSounds(input: &InputControls, first: u16, second: u16) {
        input.set_menu_sounds(first, second);
    }

    /// # Safety
    /// Both output pointers must be writable for the synchronous call.
    pub unsafe fn GetMenuSounds(input: &InputControls, first: *mut u16, second: *mut u16) {
        (*first, *second) = input.menu_sounds();
    }

    fn battle_input(keys: &[i32; crate::engine::input::frames::FLIGHT_KEYS]) -> u8 {
        [
            (KEY_UP, 2),
            (KEY_LEFT, 0),
            (KEY_RIGHT, 1),
            (KEY_WEAPON, 3),
            (KEY_SPECIAL, 4),
            (KEY_ESCAPE, 5),
            (KEY_DOWN, 6),
        ]
        .into_iter()
        .fold(0, |flags, (key, bit)| {
            flags | (u8::from(keys[key as usize] != 0) << bit)
        })
    }

    pub fn CurrentInputToBattleInput(input: &InputControls, player: u16) -> u8 {
        battle_input(&input.current().key[input.player_controls()[player as usize] as usize])
    }

    pub fn PulsedInputToBattleInput(input: &InputControls, player: u16) -> u8 {
        battle_input(&input.pulsed().key[input.player_controls()[player as usize] as usize])
    }

    /// # Safety
    /// Input, including the nested pause/exit UI, is initialized on the game thread.
    pub unsafe fn AnyButtonPress(cx: &mut Cx<'_>, input: &InputSession, _special: u32) -> u32 {
        UpdateInputState(cx, input);
        u32::from(input.current().any_button_pressed())
    }

    /// # Safety
    /// The underlying confirmation UI retains its resources and this input owner.
    pub unsafe fn ConfirmExit(cx: &mut Cx<'_>, input: &InputSession) -> u32 {
        let old = input.repeat_config();
        SetDefaultMenuRepeatDelay(input);
        let result = crate::ui::confirm::do_confirm_exit(cx, input);
        SetMenuRepeatDelay(input, old.0, old.1, old.2, u32::from(old.3));
        result
    }
}
#[path = "input_driver.rs"]
mod driver;
#[cfg(not(test))]
pub use driver::NativeInputRuntime;
#[cfg(not(test))]
pub use live::*;
