//! Release-before-repeat handling for the retained replay menu keys.

#[cfg(not(test))]
use super::c_bridge;
use std::ffi::c_int;

/// Tracks whether a menu key has been released since it last acted.
///
/// A key still held when a nested `DoInput` loop is entered keeps
/// producing repeat pulses inside that loop. Without this latch the loop
/// reads those repeats as fresh presses, re-triggering its action and
/// resetting its timeout indefinitely.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct FreshPressLatch {
    armed: c_int,
}

impl FreshPressLatch {
    /// Arm only when the key is already up; an inherited hold must be
    /// released before it can act.
    #[cfg(not(test))]
    pub(super) fn entering(input: &super::InputSession, key: c_int) -> Self {
        // SAFETY: the replay session reads initialized input on the game thread.
        Self::from_held(c_bridge::current_menu_key(input, key) != 0)
    }

    fn from_held(held: bool) -> Self {
        Self {
            armed: i32::from(!held),
        }
    }

    /// Report a fresh press, re-arming whenever the key is released.
    #[cfg(not(test))]
    pub(super) fn pressed(&mut self, input: &super::InputSession, key: c_int) -> bool {
        // SAFETY: the replay session reads initialized input on the game thread.
        let held = c_bridge::current_menu_key(input, key) != 0;
        self.sample(held, || {
            // SAFETY: the same session owns the pulsed input snapshot.
            (c_bridge::pulsed_menu_key(input, key)) != 0
        })
    }

    fn sample(&mut self, held: bool, pulse: impl FnOnce() -> bool) -> bool {
        if !held {
            self.armed = 1;
            return false;
        }
        if self.armed != 0 && pulse() {
            self.armed = 0;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_hold_and_repeats_require_a_release_before_each_action() {
        let mut latch = FreshPressLatch::from_held(true);
        assert!(!latch.sample(true, || panic!("an inherited hold must not sample a pulse")));
        assert!(!latch.sample(false, || panic!("a release must not sample a pulse")));
        assert!(!latch.sample(true, || false));
        assert!(latch.sample(true, || true));
        assert!(!latch.sample(true, || panic!("a repeat must not sample a pulse")));
        assert!(!latch.sample(false, || false));
        assert!(latch.sample(true, || true));
    }

    #[test]
    fn entering_with_a_released_key_accepts_the_first_press() {
        let mut latch = FreshPressLatch::from_held(false);
        assert!(latch.sample(true, || true));
    }
}
