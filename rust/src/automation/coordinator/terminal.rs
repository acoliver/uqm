//! First-terminal publication and propagation to the game loop.

use super::*;

impl Coordinator {
    /// Set a terminal outcome on both the coordinator and the runtime mirror.
    /// Also propagates the stop to the game loop by setting CHECK_ABORT
    /// and MainExited to force the game loop to exit.
    pub(super) fn set_terminal(&self, inner: &mut CoordInner, class: TerminalClass) {
        let class = self.runtime.mirror.terminal.load().unwrap_or(class);
        if !record_first_terminal(&mut inner.terminal_class, class) {
            return;
        }
        inner.replay_final_activity = crate::mainloop::ffi::get_current_activity().0;
        self.runtime.mirror.terminal.try_set(class);

        // Propagate stop to the C game loop: set CHECK_ABORT so the
        // activity state machine exits, and set MainExited so the
        // outer game loop stops requesting new games.
        // CHECK_ABORT = 0x4000 (setup.h).
        // SAFETY: terminal publication runs on the game thread that owns activity state.
        #[cfg(not(test))]
        unsafe {
            crate::mainloop::c_extern::set_current_activity(
                crate::mainloop::c_extern::get_current_activity() | 0x4000,
            );
            crate::mainloop::ffi::set_main_exited(true);
        }
    }
}
