//! Input repeat, nested input loops and battle controls, ported from gameinp.c.
#[derive(Clone, Copy)]
struct Repeat {
    delay: u32,
    time: u32,
}
impl Repeat {
    fn pulse(
        &mut self,
        current: i32,
        old: i32,
        now: u32,
        minimum: u32,
        maximum: u32,
        step: u32,
    ) -> i32 {
        if current != 0 && old != 0 {
            if now.wrapping_sub(self.time) < self.delay {
                return 0;
            }
            if self.delay > minimum {
                self.delay = self.delay.wrapping_sub(step);
            }
            if self.delay < minimum {
                self.delay = minimum;
            }
            self.time = now;
        } else {
            self.time = now;
            self.delay = maximum;
        }
        current
    }
}

/// Wait at DoInput's native yield boundary without flushing partial draws.
pub fn yield_input_loop(native_yield: impl FnOnce()) {
    if crate::time::source::is_replay() {
        // The native 1 ms sleep rounds up to one 840-Hz replay tick.
        crate::time::source::advance_to(crate::time::source::now_ticks().saturating_add(1));
    } else {
        native_yield();
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    #![allow(non_snake_case)]
    use super::*;
    use crate::c_bindings::controller_input::{
        CONTROLLER_INPUT_STATE as InputState, NUM_KEYS as FLIGHT_KEYS, NUM_MENU_KEYS as MENU_KEYS,
    };
    const TEMPLATES: u32 = 6;
    use crate::ships::native::abi::*;
    use std::{ffi::c_void, ptr};

    const EMPTY: InputState = InputState {
        key: [[0; FLIGHT_KEYS as usize]; TEMPLATES as usize],
        menu: [0; MENU_KEYS as usize],
    };
    const ZERO_REPEAT: Repeat = Repeat { time: 0, delay: 0 };
    #[no_mangle]
    pub static mut CurrentInputState: InputState = EMPTY;
    #[no_mangle]
    pub static mut PulsedInputState: InputState = EMPTY;
    #[no_mangle]
    pub static mut ImmediateInputState: InputState = EMPTY;
    #[no_mangle]
    pub static mut PlayerControls: [u32; 2] = [0; 2];
    #[no_mangle]
    pub static mut ExitRequested: u32 = 0;
    #[no_mangle]
    pub static mut GamePaused: u32 = 0;
    static mut CACHED: InputState = EMPTY;
    static mut REPEATS: [Repeat; (TEMPLATES * FLIGHT_KEYS + MENU_KEYS) as usize] =
        [ZERO_REPEAT; (TEMPLATES * FLIGHT_KEYS + MENU_KEYS) as usize];
    static mut GESTALT: Repeat = ZERO_REPEAT;
    static mut CACHED_GESTALT: i32 = 0;
    static mut MINIMUM: u32 = 0;
    static mut MAXIMUM: u32 = 0;
    static mut STEP: u32 = 0;
    static mut GESTALT_KEYS: bool = false;
    static mut SOUND_0: u16 = 0;
    static mut SOUND_1: u16 = 0;
    static mut INPUT_CALLBACK: Option<unsafe extern "C" fn()> = None;

    /// # Safety
    /// Automation calls on the game thread with externally supplied input indices.
    #[no_mangle]
    pub unsafe extern "C" fn c_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
        if !(0..MENU_KEYS as i32).contains(&index) {
            return -1;
        }
        ImmediateInputState.menu[index as usize] = i32::from(value != 0);
        0
    }
    /// # Safety
    /// Automation calls on the game thread with externally supplied input indices.
    #[no_mangle]
    pub unsafe extern "C" fn c_automation_set_immediate_player_key(index: i32, value: i32) -> i32 {
        if !(0..FLIGHT_KEYS as i32).contains(&index) || PlayerControls[0] >= TEMPLATES {
            return -1;
        }
        ImmediateInputState.key[PlayerControls[0] as usize][index as usize] = i32::from(value != 0);
        0
    }

    unsafe fn clear_menu_state() {
        PulsedInputState = EMPTY;
        CACHED = EMPTY;
        CACHED_GESTALT = 0;
    }
    /// # Safety
    /// Game-thread access to repeat state.
    #[no_mangle]
    pub unsafe extern "C" fn ResetKeyRepeat() {
        let now = crate::time::counter_abi::GetTimeCounter();
        ptr::addr_of_mut!(REPEATS).write(
            [Repeat {
                delay: MAXIMUM,
                time: now,
            }; (TEMPLATES * FLIGHT_KEYS + MENU_KEYS) as usize],
        );
        GESTALT = Repeat {
            delay: MAXIMUM,
            time: now,
        };
    }

    /// # Safety
    /// Game-thread access to input and repeat state.
    #[no_mangle]
    pub unsafe extern "C" fn UpdateInputState() {
        if crate::graphics::sdl::lifecycle_abi::GameActive == 0 {
            crate::ui::util::live::SleepGame();
        }
        if GamePaused != 0 {
            crate::ui::util::live::PauseGame();
        }
        if ExitRequested != 0 {
            ConfirmExit();
        }
        CurrentInputState = ptr::addr_of!(ImmediateInputState).read_volatile();
        let old = CACHED;
        CACHED = ptr::addr_of!(ImmediateInputState).read_volatile();
        crate::input::sdl_input::BeginInputFrame();
        let now = crate::time::counter_abi::GetTimeCounter();
        if GESTALT_KEYS {
            let immediate = ptr::addr_of!(ImmediateInputState).read_volatile();
            let current = immediate
                .key
                .into_iter()
                .flatten()
                .chain(immediate.menu)
                .fold(0, |a, b| a | b);
            let old_gestalt = CACHED_GESTALT;
            CACHED_GESTALT = current;
            let pulse = (&mut *ptr::addr_of_mut!(GESTALT)).pulse(
                current,
                old_gestalt,
                now,
                MINIMUM,
                MAXIMUM,
                STEP,
            );
            PulsedInputState = if pulse != 0 { CACHED } else { EMPTY };
        } else {
            let values = CACHED.key.into_iter().flatten().chain(CACHED.menu);
            let old_values = old.key.into_iter().flatten().chain(old.menu);
            let repeats = &mut *ptr::addr_of_mut!(REPEATS);
            let pulsed = &mut *ptr::addr_of_mut!(PulsedInputState);
            let outputs = pulsed
                .key
                .iter_mut()
                .flatten()
                .chain(pulsed.menu.iter_mut());
            for (((value, old_value), repeat), output) in
                values.zip(old_values).zip(repeats.iter_mut()).zip(outputs)
            {
                *output = repeat.pulse(value, old_value, now, MINIMUM, MAXIMUM, STEP);
            }
        }
        if CurrentInputState.menu[KEY_PAUSE as usize] != 0 {
            GamePaused = 1;
        }
        if CurrentInputState.menu[KEY_EXIT as usize] != 0 {
            ExitRequested = 1;
        }
        #[cfg(feature = "debug-process")]
        if PulsedInputState.menu[KEY_DEBUG as usize] != 0 {
            crate::front_end::uqmdebug::debugKeyPressedSynchronous();
        }
    }

    /// # Safety
    /// Callback must outlive its installation, and must not replace an active callback.
    #[no_mangle]
    pub unsafe extern "C" fn SetInputCallback(
        callback: Option<unsafe extern "C" fn()>,
    ) -> Option<unsafe extern "C" fn()> {
        let old = INPUT_CALLBACK;
        assert!(old.is_none() || callback.is_none());
        INPUT_CALLBACK = callback;
        old
    }
    /// # Safety
    /// Game-thread access to repeat configuration.
    #[no_mangle]
    pub unsafe extern "C" fn SetMenuRepeatDelay(min: u32, max: u32, step: u32, gestalt: u32) {
        MINIMUM = min;
        MAXIMUM = max;
        STEP = step;
        GESTALT_KEYS = gestalt != 0;
        ResetKeyRepeat();
    }
    /// # Safety
    /// Game-thread access to repeat configuration.
    #[no_mangle]
    pub unsafe extern "C" fn SetDefaultMenuRepeatDelay() {
        SetMenuRepeatDelay(840 / 12, 840 / 2, 840 / 12, 0);
    }
    /// # Safety
    /// Game-thread access to initialized SDL input vectors.
    #[no_mangle]
    pub unsafe extern "C" fn FlushInput() {
        crate::input::sdl_input::TFB_ResetControls();
        clear_menu_state();
    }

    fn menu_sound_flags(state: &InputState) -> u16 {
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
    /// State starts with a live non-null C ABI input callback; nested loops run on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn DoInput(state: *mut c_void, reset: u32) {
        if reset != 0 {
            FlushInput();
        }
        crate::mainloop::input_frame::rust_begin_input_loop();
        loop {
            crate::callback::ffi::Async_process();
            crate::graphics::sdl::lifecycle_abi::TFB_ProcessEvents();
            crate::mainloop::lifecycle_abi::ProcessUtilityKeys();
            super::yield_input_loop(|| crate::threading::legacy_abi::TaskSwitch());
            if crate::automation::input_ffi::rust_automation_service_do_input() != 0 {
                break;
            }
            UpdateInputState();
            if crate::automation::input_ffi::rust_automation_after_input_update() != 0 {
                break;
            }
            let flags = menu_sound_flags(&ptr::addr_of!(PulsedInputState).read());
            let mut sound = Kernel::current().menu_sounds;
            if !sound.is_null() && flags & (SOUND_0 | SOUND_1) != 0 {
                if flags & SOUND_1 != 0 {
                    sound = crate::strings::string_table::SetAbsStringTableIndex(sound.cast(), 1)
                        .cast();
                }
                crate::ui::settings::PlaySoundEffect(
                    sound.cast(),
                    0,
                    crate::navigation::abi::SoundPosition {
                        positional: 0,
                        x: 0,
                        y: 0,
                    },
                    ptr::null_mut(),
                    0,
                );
            }
            if let Some(callback) = INPUT_CALLBACK {
                callback();
            }
            let callback = *state.cast::<unsafe extern "C" fn(*mut c_void) -> u32>();
            let keep = callback(state);
            if crate::mainloop::input_frame::rust_complete_input_frame(keep as i32) == 0 {
                break;
            }
        }
        if reset != 0 {
            FlushInput();
        }
    }
    /// # Safety
    /// Game-thread access to menu sounds.
    #[no_mangle]
    pub unsafe extern "C" fn SetMenuSounds(first: u16, second: u16) {
        SOUND_0 = first;
        SOUND_1 = second;
    }
    /// # Safety
    /// Both output pointers must be writable on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn GetMenuSounds(first: *mut u16, second: *mut u16) {
        *first = SOUND_0;
        *second = SOUND_1;
    }

    fn battle_input(keys: &[i32; FLIGHT_KEYS as usize]) -> u8 {
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
    /// # Safety
    /// Player is valid and its control template is installed.
    #[no_mangle]
    pub unsafe extern "C" fn CurrentInputToBattleInput(player: u16) -> u8 {
        battle_input(&CurrentInputState.key[PlayerControls[player as usize] as usize])
    }
    /// # Safety
    /// Player is valid and its control template is installed.
    #[no_mangle]
    pub unsafe extern "C" fn PulsedInputToBattleInput(player: u16) -> u8 {
        battle_input(&PulsedInputState.key[PlayerControls[player as usize] as usize])
    }
    /// # Safety
    /// Game-thread access to input, including nested pause and exit loops.
    #[no_mangle]
    pub unsafe extern "C" fn AnyButtonPress(_special: u32) -> u32 {
        UpdateInputState();
        let current = ptr::addr_of!(CurrentInputState).read();
        u32::from(
            current
                .key
                .into_iter()
                .flatten()
                .chain(current.menu)
                .any(|key| key != 0),
        )
    }
    /// # Safety
    /// Game-thread access to input; underlying confirmation loop owns its resources.
    #[no_mangle]
    pub unsafe extern "C" fn ConfirmExit() -> u32 {
        let old = (MINIMUM, MAXIMUM, STEP, GESTALT_KEYS);
        SetDefaultMenuRepeatDelay();
        let result = crate::ui::confirm::DoConfirmExit();
        SetMenuRepeatDelay(old.0, old.1, old.2, u32::from(old.3));
        result
    }
}
#[cfg(not(test))]
pub use live::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pulse_has_initial_delay_then_accelerates_and_resets_after_release() {
        let mut repeat = Repeat { time: 0, delay: 0 };
        assert_eq!(repeat.pulse(1, 0, 10, 70, 420, 70), 1);
        assert_eq!(repeat.pulse(1, 1, 429, 70, 420, 70), 0);
        assert_eq!(repeat.pulse(1, 1, 430, 70, 420, 70), 1);
        assert_eq!(repeat.delay, 350);
        assert_eq!(repeat.pulse(0, 1, 431, 70, 420, 70), 0);
        assert_eq!(repeat.delay, 420);
    }
    #[test]
    fn pulse_uses_wrapping_tick_subtraction() {
        let mut repeat = Repeat {
            time: u32::MAX - 9,
            delay: 20,
        };
        assert_eq!(repeat.pulse(1, 1, 9, 5, 20, 5), 0);
        assert_eq!(repeat.pulse(1, 1, 10, 5, 20, 5), 1);
        assert_eq!(repeat.delay, 15);
    }
}
