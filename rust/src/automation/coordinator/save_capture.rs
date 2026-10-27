//! One-shot V3b load/capture lifecycle at production game-loop boundaries.

use super::*;

impl Coordinator {
    /// Called by the existing post-InitGameStructures start-scene hook.
    pub(super) fn activate_save_capture() {
        let Some(coord) = Self::get() else {
            return;
        };
        let mut inner = coord.inner.lock();
        let Some(request) = inner.save_capture.as_mut() else {
            return;
        };
        #[cfg(not(test))]
        if let Err(error) = request.load() {
            coord.write_trace_labeled(
                &mut inner,
                RecordKind::SemanticAssertion,
                format!("save_capture_load_failed:{error:#}"),
            );
            coord.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
        }
        #[cfg(test)]
        panic!("runtime save load cannot run in unit tests: {request:?}");
    }

    pub(super) fn save_input(&self, inner: &mut CoordInner, button_wait: bool) -> bool {
        let Some(request) = inner.save_capture.as_mut() else {
            return false;
        };
        if !request.admits_capture(crate::mainloop::ffi::get_current_activity().0, button_wait) {
            return false;
        }
        #[cfg(not(test))]
        {
            let result = request.capture(inner.input_seen);
            let (class, label) = match result {
                Ok(()) => (TerminalClass::Success, "save_capture_complete".to_string()),
                Err(error) => (
                    TerminalClass::TraceFailure,
                    format!("save_capture_failed:{error:#}"),
                ),
            };
            self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
            self.set_terminal(inner, class);
            true
        }
        #[cfg(test)]
        panic!("runtime save capture cannot run in unit tests");
    }
}
