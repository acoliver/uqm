//! VControl: the bindings from keys and joystick inputs to control words,
//! and the control words themselves.

use super::frames::{InputState, Target};
use super::gesture::Gesture;
use super::joystick::Joystick;
use super::keyboard::{KeyboardBindings, COUNT_MASK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VControlError {
    JoystickNotFound(u32),
    InvalidParameter(String),
}

impl std::fmt::Display for VControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VControlError::JoystickNotFound(idx) => write!(f, "Joystick {idx} not found"),
            VControlError::InvalidParameter(s) => write!(f, "Invalid parameter: {s}"),
        }
    }
}

impl std::error::Error for VControlError {}

pub type VControlResult<T> = Result<T, VControlError>;

/// The control words and every binding that writes them.
#[derive(Debug)]
pub struct VControl {
    initialized: bool,
    /// The words the bindings write; owned here so no binding holds an address.
    state: InputState,
    keyboard: KeyboardBindings,
    joysticks: Vec<Option<Joystick>>,
    last_gesture: Option<Gesture>,
}

impl Default for VControl {
    fn default() -> Self {
        Self::new()
    }
}

impl VControl {
    pub const MAX_JOYSTICKS: usize = 8;

    pub fn new() -> Self {
        Self {
            initialized: false,
            state: InputState::default(),
            keyboard: KeyboardBindings::new(),
            joysticks: vec![None; Self::MAX_JOYSTICKS],
            last_gesture: None,
        }
    }

    pub fn init(&mut self) {
        if self.initialized {
            return;
        }
        self.keyboard = KeyboardBindings::new();
        self.joysticks = vec![None; Self::MAX_JOYSTICKS];
        self.initialized = true;
    }

    /// Drop every binding and the remembered gesture. The control words keep
    /// their values.
    pub fn uninit(&mut self) {
        if !self.initialized {
            return;
        }
        self.keyboard.clear();
        self.joysticks.fill(None);
        self.last_gesture = None;
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn state(&self) -> &InputState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut InputState {
        &mut self.state
    }

    /// Zero every bound control word and centre every joystick.
    pub fn reset_states(&mut self) {
        for target in self.keyboard.targets() {
            *self.state.slot(target) = 0;
        }
        for joystick in self.joysticks.iter_mut().flatten() {
            joystick.reset_all(&mut self.state);
        }
    }

    /// Start a frame: clear the start bit that presses of the last one set.
    pub fn begin_frame(&mut self) {
        let state = &mut self.state;
        state
            .key
            .iter_mut()
            .flatten()
            .chain(state.menu.iter_mut())
            .for_each(|word| *word &= COUNT_MASK);
    }

    // === Keyboard ===

    pub fn add_key_binding(&mut self, keycode: i32, target: Target) -> bool {
        self.keyboard.add_binding(keycode, target)
    }

    pub fn remove_key_binding(&mut self, keycode: i32, target: Target) -> bool {
        self.keyboard.remove_binding(keycode, target)
    }

    pub fn clear_key_bindings(&mut self) {
        self.keyboard.clear();
    }

    pub fn has_key_bindings(&self, keycode: i32) -> bool {
        self.keyboard.has_bindings(keycode)
    }

    pub fn handle_key_down(&mut self, keycode: i32) {
        self.keyboard.handle_key_down(keycode, &mut self.state);
    }

    pub fn handle_key_up(&mut self, keycode: i32) {
        self.keyboard.handle_key_up(keycode, &mut self.state);
    }

    // === Joysticks ===

    pub fn init_joystick(
        &mut self,
        index: u32,
        name: String,
        num_axes: i32,
        num_buttons: i32,
        num_hats: i32,
    ) -> VControlResult<()> {
        let slot = self.joysticks.get_mut(index as usize).ok_or_else(|| {
            VControlError::InvalidParameter(format!(
                "Joystick index {index} exceeds max {}",
                Self::MAX_JOYSTICKS
            ))
        })?;
        *slot = Some(Joystick::new(name, num_axes, num_buttons, num_hats));
        Ok(())
    }

    pub fn uninit_joystick(&mut self, index: u32) -> VControlResult<()> {
        let slot = self
            .joysticks
            .get_mut(index as usize)
            .ok_or(VControlError::JoystickNotFound(index))?;
        *slot = None;
        Ok(())
    }

    pub fn num_joysticks(&self) -> u32 {
        self.joysticks.iter().flatten().count() as u32
    }

    pub fn joystick(&self, index: u32) -> Option<&Joystick> {
        self.joysticks.get(index as usize)?.as_ref()
    }

    fn joystick_mut(&mut self, index: u32) -> VControlResult<&mut Joystick> {
        self.joysticks
            .get_mut(index as usize)
            .and_then(Option::as_mut)
            .ok_or(VControlError::JoystickNotFound(index))
    }

    pub fn set_joy_threshold(&mut self, joy: u32, threshold: i32) -> VControlResult<()> {
        self.joystick_mut(joy)?.set_threshold(threshold);
        Ok(())
    }

    pub fn clear_joy_bindings(&mut self, joy: u32) -> VControlResult<()> {
        self.joystick_mut(joy)?.clear_bindings();
        Ok(())
    }

    /// Drop every key and joystick binding.
    pub fn remove_all_bindings(&mut self) {
        self.keyboard.clear();
        self.joysticks
            .iter_mut()
            .flatten()
            .for_each(Joystick::clear_bindings);
    }

    pub fn handle_joy_button(&mut self, joy: u32, button: i32, pressed: bool) {
        if let Some(Some(joystick)) = self.joysticks.get_mut(joy as usize) {
            joystick.handle_button(button, pressed, &mut self.state);
        }
    }

    pub fn handle_joy_axis(&mut self, joy: u32, axis: i32, value: i16) {
        if let Some(Some(joystick)) = self.joysticks.get_mut(joy as usize) {
            joystick.handle_axis(axis, value, &mut self.state);
        }
    }

    pub fn handle_joy_hat(&mut self, joy: u32, hat: i32, value: u8) {
        if let Some(Some(joystick)) = self.joysticks.get_mut(joy as usize) {
            joystick.handle_hat(hat, value, &mut self.state);
        }
    }

    // === Gesture bindings ===

    /// Make `gesture` drive `target`; `false` when it is already bound or
    /// names a joystick input that does not exist.
    pub fn bind(&mut self, gesture: Gesture, target: Target) -> bool {
        match gesture {
            Gesture::Key(key) => self.keyboard.add_binding(key, target),
            Gesture::JoyAxis {
                port,
                axis,
                polarity,
            } => self
                .joystick_mut(port)
                .is_ok_and(|joy| joy.add_axis_binding(axis, polarity, target)),
            Gesture::JoyButton { port, button } => self
                .joystick_mut(port)
                .is_ok_and(|joy| joy.add_button_binding(button, target)),
            Gesture::JoyHat { port, hat, dir } => self
                .joystick_mut(port)
                .is_ok_and(|joy| joy.add_hat_binding(hat, dir, target)),
        }
    }

    pub fn unbind(&mut self, gesture: Gesture, target: Target) -> bool {
        match gesture {
            Gesture::Key(key) => self.keyboard.remove_binding(key, target),
            Gesture::JoyAxis {
                port,
                axis,
                polarity,
            } => self
                .joystick_mut(port)
                .is_ok_and(|joy| joy.remove_axis_binding(axis, polarity, target)),
            Gesture::JoyButton { port, button } => self
                .joystick_mut(port)
                .is_ok_and(|joy| joy.remove_button_binding(button, target)),
            Gesture::JoyHat { port, hat, dir } => self
                .joystick_mut(port)
                .is_ok_and(|joy| joy.remove_hat_binding(hat, dir, target)),
        }
    }

    // === Gesture tracking, for rebinding ===

    pub fn clear_gesture(&mut self) {
        self.last_gesture = None;
    }

    pub fn last_gesture(&self) -> Option<Gesture> {
        self.last_gesture
    }

    pub fn set_last_gesture(&mut self, gesture: Gesture) {
        self.last_gesture = Some(gesture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_port::gesture::hat;
    use crate::input_port::keyboard::START_BIT;

    const A: Target = Target::Menu(0);
    const B: Target = Target::Flight {
        template: 1,
        key: 2,
    };

    fn started() -> VControl {
        let mut vc = VControl::new();
        vc.init();
        vc
    }

    fn with_pad(axes: i32, buttons: i32, hats: i32) -> VControl {
        let mut vc = started();
        vc.init_joystick(0, "Pad".into(), axes, buttons, hats)
            .unwrap();
        vc
    }

    #[test]
    fn initialised_only_between_init_and_uninit() {
        let mut vc = VControl::new();
        assert!(!vc.is_initialized());
        vc.init();
        assert!(vc.is_initialized());
        vc.uninit();
        assert!(!vc.is_initialized());
    }

    #[test]
    fn a_key_feeds_two_words_and_each_is_removed_on_its_own() {
        let mut vc = started();
        assert!(vc.add_key_binding(32, A));
        assert!(vc.add_key_binding(32, B));
        assert!(!vc.add_key_binding(32, A));
        vc.handle_key_down(32);
        assert_eq!(vc.state().menu[0], START_BIT | 1);
        assert_eq!(vc.state().key[1][2], START_BIT | 1);
        assert!(vc.remove_key_binding(32, A));
        vc.handle_key_up(32);
        assert_eq!(vc.state().menu[0], START_BIT | 1, "no longer bound");
        assert_eq!(vc.state().key[1][2], START_BIT);
        vc.clear_key_bindings();
        assert!(!vc.has_key_bindings(32));
    }

    #[test]
    fn the_start_bit_lasts_until_the_next_frame() {
        let mut vc = started();
        vc.add_key_binding(32, A);
        vc.handle_key_down(32);
        vc.begin_frame();
        assert_eq!(vc.state().menu[0], 1);
        vc.handle_key_up(32);
        assert_eq!(vc.state().menu[0], 0);
    }

    #[test]
    fn reset_releases_bound_keys_and_pads_but_not_unbound_words() {
        let mut vc = with_pad(0, 1, 0);
        vc.add_key_binding(32, A);
        vc.bind(Gesture::JoyButton { port: 0, button: 0 }, B);
        vc.handle_key_down(32);
        vc.handle_joy_button(0, 0, true);
        vc.state_mut().menu[5] = 1;
        vc.reset_states();
        assert_eq!(vc.state().menu[0], 0);
        assert_eq!(vc.state().key[1][2], 0);
        assert_eq!(vc.state().menu[5], 1);
    }

    #[test]
    fn the_registry_counts_and_bounds_its_pads() {
        let mut vc = started();
        assert_eq!(vc.num_joysticks(), 0);
        assert!(vc.init_joystick(0, "A".into(), 1, 1, 1).is_ok());
        assert!(vc.init_joystick(7, "B".into(), 1, 1, 1).is_ok());
        assert!(matches!(
            vc.init_joystick(8, "C".into(), 1, 1, 1),
            Err(VControlError::InvalidParameter(_))
        ));
        assert_eq!(vc.num_joysticks(), 2);
        assert_eq!(vc.joystick(7).unwrap().name, "B");
        assert!(vc.uninit_joystick(7).is_ok());
        assert_eq!(
            vc.uninit_joystick(8),
            Err(VControlError::JoystickNotFound(8))
        );
        assert_eq!(vc.num_joysticks(), 1);
        assert_eq!(
            vc.set_joy_threshold(3, 5),
            Err(VControlError::JoystickNotFound(3))
        );
    }

    #[test]
    fn gestures_bind_and_unbind_each_kind_of_control() {
        let mut vc = with_pad(1, 1, 1);
        let gestures = [
            Gesture::Key(32),
            Gesture::JoyButton { port: 0, button: 0 },
            Gesture::JoyAxis {
                port: 0,
                axis: 0,
                polarity: 1,
            },
            Gesture::JoyHat {
                port: 0,
                hat: 0,
                dir: hat::UP,
            },
        ];
        for gesture in gestures {
            assert!(vc.bind(gesture, A), "{gesture:?}");
            assert!(!vc.bind(gesture, A), "{gesture:?} twice");
        }
        vc.handle_key_down(32);
        assert_eq!(vc.state().menu[0] & COUNT_MASK, 1);
        for gesture in gestures {
            assert!(vc.unbind(gesture, A), "{gesture:?}");
            assert!(!vc.unbind(gesture, A), "{gesture:?} twice");
        }
    }

    #[test]
    fn binding_to_a_missing_pad_or_input_is_refused() {
        let mut vc = with_pad(1, 1, 1);
        assert!(!vc.bind(Gesture::JoyButton { port: 4, button: 0 }, A));
        assert!(!vc.bind(Gesture::JoyButton { port: 0, button: 9 }, A));
        assert!(!vc.bind(
            Gesture::JoyHat {
                port: 0,
                hat: 0,
                dir: 3
            },
            A
        ));
    }

    #[test]
    fn clearing_one_pad_leaves_the_keyboard_and_other_pads_bound() {
        let mut vc = with_pad(0, 1, 0);
        vc.init_joystick(1, "Two".into(), 0, 1, 0).unwrap();
        vc.add_key_binding(32, A);
        vc.bind(Gesture::JoyButton { port: 0, button: 0 }, A);
        vc.bind(Gesture::JoyButton { port: 1, button: 0 }, B);
        vc.clear_joy_bindings(0).unwrap();
        vc.handle_joy_button(0, 0, true);
        assert_eq!(vc.state().menu[0], 0);
        vc.handle_joy_button(1, 0, true);
        assert_eq!(vc.state().key[1][2], 1);
        vc.handle_key_down(32);
        assert_eq!(vc.state().menu[0] & COUNT_MASK, 1);
        vc.remove_all_bindings();
        vc.handle_key_up(32);
        vc.begin_frame();
        assert_eq!(vc.state().menu[0], 1, "the key is no longer bound");
        vc.handle_joy_button(1, 0, false);
        assert_eq!(vc.state().key[1][2], 1, "the pad is no longer bound");
    }

    #[test]
    fn uninit_forgets_bindings_and_the_last_gesture_but_not_the_words() {
        let mut vc = started();
        vc.add_key_binding(32, A);
        vc.handle_key_down(32);
        vc.set_last_gesture(Gesture::Key(32));
        vc.uninit();
        assert_eq!(vc.last_gesture(), None);
        assert!(!vc.has_key_bindings(32));
        assert_eq!(vc.state().menu[0], START_BIT | 1);
        vc.set_last_gesture(Gesture::Key(1));
        vc.clear_gesture();
        assert_eq!(vc.last_gesture(), None);
    }
}
