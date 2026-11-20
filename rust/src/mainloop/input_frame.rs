//! Presentation boundary for Rust-main synchronous input loops.
//!
//! Legacy UQM normally has a render thread draining committed draw commands.
//! When Rust owns `main`, `DoInput` is synchronous and therefore must present
//! after each complete input callback. Presenting after the callback preserves
//! erase/redraw batching while making callback-produced first frames visible.

/// Result of a completed input callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputFrameResult {
    /// Whether `DoInput` should run another callback iteration.
    pub keep_running: bool,
    /// Whether the completed callback constitutes a presentation boundary.
    pub present: bool,
}

/// Complete one input callback and declare its presentation boundary.
///
/// The callback is evaluated exactly once. Its full drawing transaction is
/// complete before `present` becomes true, avoiding an intermediate frame
/// between menu erase and redraw commands.
pub fn complete_input_frame(callback: impl FnOnce() -> bool) -> InputFrameResult {
    InputFrameResult {
        keep_running: callback(),
        present: true,
    }
}

pub use crate::engine::host::display::drain_committed_draws;

/// Present drawing committed before a synchronous `DoInput` loop begins.
///
/// Nested menus often draw their initial state immediately before calling
/// `DoInput`; the outer callback cannot return while that nested loop runs.
pub fn rust_begin_input_loop() {
    crate::engine::host::display::present_committed_draws();
}

/// Complete the typed input callback and notify its supplied observer.
///
/// Returns the callback's original continue value after presenting its complete
/// draw transaction in Rust-owned-main builds.
pub fn rust_complete_input_frame(
    cx: &crate::engine::battle::Cx<'_>,
    keep_running: i32,
    io: &mut crate::engine::observe::Io<'_>,
) -> i32 {
    let result = complete_input_frame(|| keep_running != 0);

    if result.present {
        crate::engine::host::display::present_committed_draws();
    }

    io.observer.simulation(cx, "input-loop");
    i32::from(result.keep_running)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn complete_frame_presents_after_callback_returns() {
        let callback_returned = Cell::new(false);
        let result = complete_input_frame(|| {
            assert!(!callback_returned.get());
            callback_returned.set(true);
            true
        });

        assert!(callback_returned.get());
        assert!(result.keep_running);
        assert!(result.present);
    }

    #[test]
    fn terminating_callback_still_presents_its_final_frame() {
        let result = complete_input_frame(|| false);

        assert!(!result.keep_running);
        assert!(result.present);
    }
}
