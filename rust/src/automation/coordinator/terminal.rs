//! First-terminal publication and propagation to the game loop.

use super::*;

impl AutomationObserver {
    /// Publish the first terminal outcome and request an activity-loop abort.
    pub(super) fn set_terminal(&self, inner: &mut CoordInner, class: TerminalClass) {
        let class = self.runtime.mirror.terminal.load().unwrap_or(class);
        if !record_first_terminal(&mut inner.terminal_class, class) {
            return;
        }
        inner.replay_final_activity = crate::mainloop::ffi::get_current_activity().0;
        self.runtime.mirror.terminal.try_set(class);

        // CHECK_ABORT is consumed by the activity state machine.
        #[cfg(not(test))]
        {
            crate::mainloop::ffi::set_current_activity(crate::mainloop::types::ActivityValue(
                crate::mainloop::ffi::get_current_activity().0 | 0x4000,
            ));
        }
    }
}
