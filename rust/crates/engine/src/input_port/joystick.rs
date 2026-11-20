//! Joystick button, axis and hat bindings to control words.

use super::frames::{InputState, Target};
use super::gesture::hat;

/// The control words one input drives.
type Bindings = Vec<Target>;

fn add_unique(bindings: &mut Bindings, target: Target) -> bool {
    if bindings.contains(&target) {
        return false;
    }
    bindings.push(target);
    true
}

fn remove(bindings: &mut Bindings, target: Target) -> bool {
    let before = bindings.len();
    bindings.retain(|bound| *bound != target);
    bindings.len() != before
}

fn set_all(bindings: &Bindings, state: &mut InputState, value: i32) {
    for &target in bindings {
        *state.slot(target) = value;
    }
}

/// An axis with bindings for each direction of travel.
#[derive(Debug, Default, Clone)]
pub struct JoystickAxis {
    neg: Bindings,
    pos: Bindings,
    /// -1 negative, 0 centred, 1 positive.
    polarity: i32,
}

impl JoystickAxis {
    pub fn handle_value(&mut self, value: i16, threshold: i32, state: &mut InputState) {
        let threshold = threshold as i16;
        if value < -threshold {
            if self.polarity >= 0 {
                set_all(&self.pos, state, 0);
                set_all(&self.neg, state, 1);
            }
            self.polarity = -1;
        } else if value > threshold {
            if self.polarity <= 0 {
                set_all(&self.neg, state, 0);
                set_all(&self.pos, state, 1);
            }
            self.polarity = 1;
        } else {
            if self.polarity != 0 {
                set_all(&self.neg, state, 0);
                set_all(&self.pos, state, 0);
            }
            self.polarity = 0;
        }
    }

    fn reset(&mut self, state: &mut InputState) {
        set_all(&self.neg, state, 0);
        set_all(&self.pos, state, 0);
        self.polarity = 0;
    }
}

/// A hat with bindings for each of its four directions.
#[derive(Debug, Default, Clone)]
pub struct JoystickHat {
    up: Bindings,
    down: Bindings,
    left: Bindings,
    right: Bindings,
    last: u8,
}

impl JoystickHat {
    fn directions(&self) -> [(u8, &Bindings); 4] {
        [
            (hat::UP, &self.up),
            (hat::DOWN, &self.down),
            (hat::LEFT, &self.left),
            (hat::RIGHT, &self.right),
        ]
    }

    fn direction_mut(&mut self, direction: u8) -> Option<&mut Bindings> {
        match direction {
            hat::UP => Some(&mut self.up),
            hat::DOWN => Some(&mut self.down),
            hat::LEFT => Some(&mut self.left),
            hat::RIGHT => Some(&mut self.right),
            _ => None,
        }
    }

    /// Update the bindings of every direction whose state changed.
    pub fn handle_value(&mut self, value: u8, state: &mut InputState) {
        for (bit, bindings) in self.directions() {
            let now = value & bit != 0;
            if now != (self.last & bit != 0) {
                set_all(bindings, state, i32::from(now));
            }
        }
        self.last = value;
    }

    fn reset(&mut self, state: &mut InputState) {
        for (_, bindings) in self.directions() {
            set_all(bindings, state, 0);
        }
        self.last = 0;
    }
}

/// A connected joystick with its bindings.
#[derive(Debug, Clone)]
pub struct Joystick {
    pub name: String,
    /// Axis dead zone.
    threshold: i32,
    axes: Vec<JoystickAxis>,
    buttons: Vec<Bindings>,
    hats: Vec<JoystickHat>,
}

impl Joystick {
    pub fn new(name: String, num_axes: i32, num_buttons: i32, num_hats: i32) -> Self {
        let count = |n: i32| n.max(0) as usize;
        Self {
            name,
            threshold: 10000,
            axes: vec![JoystickAxis::default(); count(num_axes)],
            buttons: vec![Bindings::new(); count(num_buttons)],
            hats: vec![JoystickHat::default(); count(num_hats)],
        }
    }

    pub fn threshold(&self) -> i32 {
        self.threshold
    }

    pub fn set_threshold(&mut self, threshold: i32) {
        self.threshold = threshold.clamp(0, 32767);
    }

    fn button_mut(&mut self, button: i32) -> Option<&mut Bindings> {
        usize::try_from(button)
            .ok()
            .and_then(|index| self.buttons.get_mut(index))
    }

    fn axis_mut(&mut self, axis: i32) -> Option<&mut JoystickAxis> {
        usize::try_from(axis)
            .ok()
            .and_then(|index| self.axes.get_mut(index))
    }

    fn hat_mut(&mut self, hat: i32) -> Option<&mut JoystickHat> {
        usize::try_from(hat)
            .ok()
            .and_then(|index| self.hats.get_mut(index))
    }

    pub fn add_button_binding(&mut self, button: i32, target: Target) -> bool {
        self.button_mut(button)
            .is_some_and(|bindings| add_unique(bindings, target))
    }

    pub fn remove_button_binding(&mut self, button: i32, target: Target) -> bool {
        self.button_mut(button)
            .is_some_and(|bindings| remove(bindings, target))
    }

    pub fn add_axis_binding(&mut self, axis: i32, polarity: i32, target: Target) -> bool {
        self.axis_mut(axis).is_some_and(|axis| {
            add_unique(
                if polarity < 0 {
                    &mut axis.neg
                } else {
                    &mut axis.pos
                },
                target,
            )
        })
    }

    pub fn remove_axis_binding(&mut self, axis: i32, polarity: i32, target: Target) -> bool {
        self.axis_mut(axis).is_some_and(|axis| {
            remove(
                if polarity < 0 {
                    &mut axis.neg
                } else {
                    &mut axis.pos
                },
                target,
            )
        })
    }

    pub fn add_hat_binding(&mut self, hat: i32, direction: u8, target: Target) -> bool {
        self.hat_mut(hat)
            .and_then(|hat| hat.direction_mut(direction))
            .is_some_and(|bindings| add_unique(bindings, target))
    }

    pub fn remove_hat_binding(&mut self, hat: i32, direction: u8, target: Target) -> bool {
        self.hat_mut(hat)
            .and_then(|hat| hat.direction_mut(direction))
            .is_some_and(|bindings| remove(bindings, target))
    }

    pub fn handle_button(&mut self, button: i32, pressed: bool, state: &mut InputState) {
        if let Some(bindings) = self.button_mut(button) {
            set_all(bindings, state, i32::from(pressed));
        }
    }

    pub fn handle_axis(&mut self, axis: i32, value: i16, state: &mut InputState) {
        let threshold = self.threshold;
        if let Some(axis) = self.axis_mut(axis) {
            axis.handle_value(value, threshold, state);
        }
    }

    pub fn handle_hat(&mut self, hat: i32, value: u8, state: &mut InputState) {
        if let Some(hat) = self.hat_mut(hat) {
            hat.handle_value(value, state);
        }
    }

    /// Release every bound control and centre every axis and hat.
    pub fn reset_all(&mut self, state: &mut InputState) {
        for bindings in &self.buttons {
            set_all(bindings, state, 0);
        }
        for axis in &mut self.axes {
            axis.reset(state);
        }
        for hat in &mut self.hats {
            hat.reset(state);
        }
    }

    pub fn clear_bindings(&mut self) {
        for bindings in &mut self.buttons {
            bindings.clear();
        }
        for axis in &mut self.axes {
            axis.neg.clear();
            axis.pos.clear();
        }
        for hat in &mut self.hats {
            for direction in [hat::UP, hat::DOWN, hat::LEFT, hat::RIGHT] {
                if let Some(bindings) = hat.direction_mut(direction) {
                    bindings.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Target = Target::Menu(0);
    const B: Target = Target::Menu(1);

    fn pad(axes: i32, buttons: i32, hats: i32) -> Joystick {
        Joystick::new("Pad".into(), axes, buttons, hats)
    }

    #[test]
    fn bindings_are_bounded_by_the_pad_and_unique() {
        let mut pad = pad(2, 3, 1);
        assert!(pad.add_button_binding(2, A));
        assert!(!pad.add_button_binding(2, A));
        assert!(!pad.add_button_binding(3, A));
        assert!(!pad.add_button_binding(-1, A));
        assert!(pad.add_axis_binding(1, -1, A));
        assert!(pad.add_axis_binding(1, 1, A));
        assert!(!pad.add_axis_binding(2, 1, A));
        assert!(pad.add_hat_binding(0, hat::UP, A));
        assert!(!pad.add_hat_binding(0, hat::RIGHT | hat::UP, A));
        assert!(pad.remove_button_binding(2, A));
        assert!(!pad.remove_button_binding(2, A));
        assert!(pad.remove_axis_binding(1, -1, A));
        assert!(pad.remove_hat_binding(0, hat::UP, A));
    }

    #[test]
    fn buttons_follow_the_press() {
        let mut pad = pad(0, 3, 0);
        let mut state = InputState::default();
        pad.add_button_binding(2, A);
        pad.handle_button(2, true, &mut state);
        assert_eq!(state.menu[0], 1);
        pad.handle_button(2, false, &mut state);
        assert_eq!(state.menu[0], 0);
        pad.handle_button(9, true, &mut state);
        assert_eq!(state.menu[0], 0);
    }

    #[test]
    fn an_axis_switches_sides_through_the_dead_zone() {
        let mut pad = pad(1, 0, 0);
        let mut state = InputState::default();
        pad.add_axis_binding(0, -1, A);
        pad.add_axis_binding(0, 1, B);
        pad.handle_axis(0, -15000, &mut state);
        assert_eq!(state.menu[..2], [1, 0]);
        pad.handle_axis(0, 5000, &mut state);
        assert_eq!(state.menu[..2], [0, 0]);
        pad.handle_axis(0, 15000, &mut state);
        assert_eq!(state.menu[..2], [0, 1]);
        pad.handle_axis(0, -15000, &mut state);
        assert_eq!(state.menu[..2], [1, 0]);
    }

    #[test]
    fn the_threshold_is_clamped_and_moves_the_dead_zone() {
        let mut pad = pad(1, 0, 0);
        let mut state = InputState::default();
        pad.add_axis_binding(0, 1, A);
        pad.set_threshold(-5);
        assert_eq!(pad.threshold(), 0);
        pad.set_threshold(99_999);
        assert_eq!(pad.threshold(), 32767);
        pad.set_threshold(20_000);
        pad.handle_axis(0, 15_000, &mut state);
        assert_eq!(state.menu[0], 0);
        pad.handle_axis(0, 25_000, &mut state);
        assert_eq!(state.menu[0], 1);
    }

    #[test]
    fn a_hat_updates_only_the_directions_that_changed() {
        let mut pad = pad(0, 0, 1);
        let mut state = InputState::default();
        pad.add_hat_binding(0, hat::UP, A);
        pad.add_hat_binding(0, hat::RIGHT, B);
        pad.handle_hat(0, hat::UP | hat::RIGHT, &mut state);
        assert_eq!(state.menu[..2], [1, 1]);
        pad.handle_hat(0, hat::RIGHT, &mut state);
        assert_eq!(state.menu[..2], [0, 1]);
        pad.handle_hat(0, 0, &mut state);
        assert_eq!(state.menu[..2], [0, 0]);
    }

    #[test]
    fn reset_releases_everything_and_centres_axes_and_hats() {
        let mut pad = pad(1, 1, 1);
        let mut state = InputState::default();
        pad.add_button_binding(0, A);
        pad.add_axis_binding(0, 1, B);
        pad.add_hat_binding(0, hat::UP, Target::Menu(2));
        pad.handle_button(0, true, &mut state);
        pad.handle_axis(0, 30_000, &mut state);
        pad.handle_hat(0, hat::UP, &mut state);
        assert_eq!(state.menu[..3], [1, 1, 1]);
        pad.reset_all(&mut state);
        assert_eq!(state.menu[..3], [0, 0, 0]);
        // Centred again, so the same push is a fresh transition.
        pad.handle_axis(0, 30_000, &mut state);
        pad.handle_hat(0, hat::UP, &mut state);
        assert_eq!(state.menu[1..3], [1, 1]);
    }

    #[test]
    fn clearing_forgets_every_binding() {
        let mut pad = pad(1, 1, 1);
        let mut state = InputState::default();
        pad.add_button_binding(0, A);
        pad.add_axis_binding(0, 1, A);
        pad.add_hat_binding(0, hat::UP, A);
        pad.clear_bindings();
        pad.handle_button(0, true, &mut state);
        pad.handle_axis(0, 30_000, &mut state);
        pad.handle_hat(0, hat::UP, &mut state);
        assert_eq!(state, InputState::default());
    }
}
