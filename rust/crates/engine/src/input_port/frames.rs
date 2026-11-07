//! Per-runtime controller snapshots and deterministic key-repeat state.

pub const KEY_UP: u32 = 0;
pub const KEY_DOWN: u32 = 1;
pub const KEY_LEFT: u32 = 2;
pub const KEY_RIGHT: u32 = 3;
pub const KEY_WEAPON: u32 = 4;
pub const KEY_SPECIAL: u32 = 5;
pub const KEY_ESCAPE: u32 = 6;
pub const NUM_KEYS: u32 = 7;
pub const KEY_PAUSE: u32 = 0;
pub const KEY_EXIT: u32 = 1;
pub const KEY_ABORT: u32 = 2;
pub const KEY_DEBUG: u32 = 3;
pub const KEY_FULLSCREEN: u32 = 4;
pub const KEY_MENU_UP: u32 = 5;
pub const KEY_MENU_DOWN: u32 = 6;
pub const KEY_MENU_LEFT: u32 = 7;
pub const KEY_MENU_RIGHT: u32 = 8;
pub const KEY_MENU_SELECT: u32 = 9;
pub const KEY_MENU_CANCEL: u32 = 10;
pub const KEY_MENU_SPECIAL: u32 = 11;
pub const KEY_MENU_PAGE_UP: u32 = 12;
pub const KEY_MENU_PAGE_DOWN: u32 = 13;
pub const KEY_MENU_HOME: u32 = 14;
pub const KEY_MENU_END: u32 = 15;
pub const KEY_MENU_ZOOM_IN: u32 = 16;
pub const KEY_MENU_ZOOM_OUT: u32 = 17;
pub const KEY_MENU_DELETE: u32 = 18;
pub const KEY_MENU_BACKSPACE: u32 = 19;
pub const KEY_MENU_EDIT_CANCEL: u32 = 20;
pub const KEY_MENU_SEARCH: u32 = 21;
pub const KEY_MENU_NEXT: u32 = 22;
pub const KEY_MENU_ANY: u32 = 23;
pub const NUM_MENU_KEYS: u32 = 24;

pub const TEMPLATES: usize = 6;
pub const FLIGHT_KEYS: usize = 7;
pub const MENU_KEYS: usize = 24;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    pub key: [[i32; FLIGHT_KEYS]; TEMPLATES],
    pub menu: [i32; MENU_KEYS],
}

const EMPTY: InputState = InputState {
    key: [[0; FLIGHT_KEYS]; TEMPLATES],
    menu: [0; MENU_KEYS],
};

#[derive(Clone, Copy, Default)]
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

pub struct InputFrames {
    pub current: InputState,
    pub pulsed: InputState,
    cached: InputState,
    repeats: [Repeat; TEMPLATES * FLIGHT_KEYS + MENU_KEYS],
    gestalt: Repeat,
    cached_gestalt: i32,
    minimum: u32,
    maximum: u32,
    step: u32,
    gestalt_keys: bool,
}

impl Default for InputFrames {
    fn default() -> Self {
        Self {
            current: EMPTY,
            pulsed: EMPTY,
            cached: EMPTY,
            repeats: [Repeat::default(); TEMPLATES * FLIGHT_KEYS + MENU_KEYS],
            gestalt: Repeat::default(),
            cached_gestalt: 0,
            minimum: 0,
            maximum: 0,
            step: 0,
            gestalt_keys: false,
        }
    }
}

impl InputFrames {
    pub fn clear_menu_state(&mut self) {
        self.pulsed = EMPTY;
        self.cached = EMPTY;
        self.cached_gestalt = 0;
    }

    pub fn reset_repeat(&mut self, now: u32) {
        let repeat = Repeat {
            delay: self.maximum,
            time: now,
        };
        self.repeats = [repeat; TEMPLATES * FLIGHT_KEYS + MENU_KEYS];
        self.gestalt = repeat;
    }

    pub fn configure_repeat(
        &mut self,
        minimum: u32,
        maximum: u32,
        step: u32,
        gestalt: bool,
        now: u32,
    ) {
        self.minimum = minimum;
        self.maximum = maximum;
        self.step = step;
        self.gestalt_keys = gestalt;
        self.reset_repeat(now);
    }

    pub fn repeat_config(&self) -> (u32, u32, u32, bool) {
        (self.minimum, self.maximum, self.step, self.gestalt_keys)
    }

    pub fn advance(&mut self, before_frame: InputState, after_frame: InputState, now: u32) {
        self.current = before_frame;
        let old = self.cached;
        self.cached = before_frame;
        if self.gestalt_keys {
            let current = after_frame
                .key
                .into_iter()
                .flatten()
                .chain(after_frame.menu)
                .fold(0, |a, b| a | b);
            let old_gestalt = self.cached_gestalt;
            self.cached_gestalt = current;
            let pulse = self.gestalt.pulse(
                current,
                old_gestalt,
                now,
                self.minimum,
                self.maximum,
                self.step,
            );
            self.pulsed = if pulse != 0 { self.cached } else { EMPTY };
        } else {
            let values = self
                .cached
                .key
                .into_iter()
                .flatten()
                .chain(self.cached.menu);
            let old_values = old.key.into_iter().flatten().chain(old.menu);
            let outputs = self
                .pulsed
                .key
                .iter_mut()
                .flatten()
                .chain(self.pulsed.menu.iter_mut());
            for (((value, old_value), repeat), output) in values
                .zip(old_values)
                .zip(self.repeats.iter_mut())
                .zip(outputs)
            {
                *output =
                    repeat.pulse(value, old_value, now, self.minimum, self.maximum, self.step);
            }
        }
    }
}

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
