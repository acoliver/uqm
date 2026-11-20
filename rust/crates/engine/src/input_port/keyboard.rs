//! Keyboard bindings from keycodes to control words.

use std::collections::HashMap;

use super::frames::{InputState, Target};

/// Control words count presses in their low byte and carry a start bit set by
/// the press that began the frame.
pub const START_BIT: i32 = 0x100;
pub const COUNT_MASK: i32 = 0xFF;

#[derive(Debug, Default)]
pub struct KeyboardBindings {
    bindings: HashMap<i32, Vec<Target>>,
}

impl KeyboardBindings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind `keycode` to `target`; `false` when that exact binding exists.
    pub fn add_binding(&mut self, keycode: i32, target: Target) -> bool {
        let targets = self.bindings.entry(keycode).or_default();
        if targets.contains(&target) {
            return false;
        }
        targets.push(target);
        true
    }

    pub fn remove_binding(&mut self, keycode: i32, target: Target) -> bool {
        let Some(targets) = self.bindings.get_mut(&keycode) else {
            return false;
        };
        let before = targets.len();
        targets.retain(|bound| *bound != target);
        let removed = targets.len() != before;
        if targets.is_empty() {
            self.bindings.remove(&keycode);
        }
        removed
    }

    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    pub fn has_bindings(&self, keycode: i32) -> bool {
        self.bindings.contains_key(&keycode)
    }

    pub fn binding_count(&self) -> usize {
        self.bindings.values().map(Vec::len).sum()
    }

    pub fn targets(&self) -> impl Iterator<Item = Target> + '_ {
        self.bindings.values().flatten().copied()
    }

    /// Count the press and mark it as the frame's start.
    pub fn handle_key_down(&self, keycode: i32, state: &mut InputState) {
        for &target in self.bindings.get(&keycode).into_iter().flatten() {
            let slot = state.slot(target);
            *slot = (*slot + 1) | START_BIT;
        }
    }

    /// Count the release, keeping the start bit.
    pub fn handle_key_up(&self, keycode: i32, state: &mut InputState) {
        for &target in self.bindings.get(&keycode).into_iter().flatten() {
            let slot = state.slot(target);
            let count = *slot & COUNT_MASK;
            if count > 0 {
                *slot = (count - 1) | (*slot & START_BIT);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Target = Target::Menu(0);
    const B: Target = Target::Menu(1);

    #[test]
    fn bindings_are_unique_per_key_and_target() {
        let mut bindings = KeyboardBindings::new();
        assert!(bindings.add_binding(32, A));
        assert!(!bindings.add_binding(32, A));
        assert!(bindings.add_binding(32, B));
        assert_eq!(bindings.binding_count(), 2);
        assert!(bindings.has_bindings(32));
    }

    #[test]
    fn removing_one_target_keeps_the_others() {
        let mut bindings = KeyboardBindings::new();
        bindings.add_binding(32, A);
        bindings.add_binding(32, B);
        assert!(bindings.remove_binding(32, A));
        assert!(!bindings.remove_binding(32, A));
        assert!(bindings.has_bindings(32));
        assert!(bindings.remove_binding(32, B));
        assert!(!bindings.has_bindings(32));
    }

    #[test]
    fn clear_forgets_every_key() {
        let mut bindings = KeyboardBindings::new();
        bindings.add_binding(32, A);
        bindings.add_binding(65, B);
        bindings.clear();
        assert_eq!(bindings.binding_count(), 0);
        assert!(!bindings.has_bindings(65));
    }

    #[test]
    fn press_counts_and_sets_the_start_bit_and_release_keeps_it() {
        let mut bindings = KeyboardBindings::new();
        let mut state = InputState::default();
        bindings.add_binding(32, A);
        bindings.add_binding(32, B);
        bindings.handle_key_down(32, &mut state);
        assert_eq!(state.menu[..2], [START_BIT | 1; 2]);
        bindings.handle_key_up(32, &mut state);
        assert_eq!(state.menu[..2], [START_BIT; 2]);
        bindings.handle_key_up(32, &mut state);
        assert_eq!(state.menu[0], START_BIT, "the count never goes below zero");
    }

    #[test]
    fn unbound_keys_change_nothing() {
        let bindings = KeyboardBindings::new();
        let mut state = InputState::default();
        bindings.handle_key_down(32, &mut state);
        bindings.handle_key_up(32, &mut state);
        assert_eq!(state, InputState::default());
    }
}
