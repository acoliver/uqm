//! Release-before-repeat handling for the retained replay menu keys.

#[cfg(not(test))]
use super::c_bridge;
#[cfg(not(test))]
use std::ffi::c_int;

/// Tracks whether a menu key has been released since it last acted.
///
/// A key still held when a nested `DoInput` loop is entered keeps
/// producing repeat pulses inside that loop. Without this latch the loop
/// reads those repeats as fresh presses, re-triggering its action and
/// resetting its timeout indefinitely.
#[repr(C)]
#[derive(Clone, Copy)]
#[cfg(not(test))]
pub(super) struct FreshPressLatch {
    armed: c_int,
}

#[cfg(not(test))]
impl FreshPressLatch {
    /// Arm only when the key is already up; an inherited hold must be
    /// released before it can act.
    pub(super) fn entering(key: c_int) -> Self {
        Self {
            // SAFETY: the replay session reads initialized input on the game thread.
            armed: (unsafe { c_bridge::current_menu_key(key) } == 0) as c_int,
        }
    }

    /// Report a fresh press, re-arming whenever the key is released.
    pub(super) fn pressed(&mut self, key: c_int) -> bool {
        // SAFETY: the replay session reads initialized input on the game thread.
        if unsafe { c_bridge::current_menu_key(key) } == 0 {
            self.armed = 1;
            return false;
        }
        // SAFETY: the same session owns the pulsed input snapshot.
        if self.armed != 0 && unsafe { c_bridge::pulsed_menu_key(key) } != 0 {
            self.armed = 0;
            return true;
        }
        false
    }
}
