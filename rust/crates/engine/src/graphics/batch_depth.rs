//! Draw-queue batch accounting at the engine graphics boundary.

use super::{
    context_abi::ContextLease,
    dcqueue_abi::require_queue,
    gfx_common_abi::{BatchGraphics, GetBatchDepth, UnbatchGraphics},
};

pub fn depth(context: &ContextLease) -> i32 {
    require_queue(context);
    // SAFETY: the supplied graphics owner admits this thread to the initialized queue.
    unsafe { GetBatchDepth() }
}

/// Apply exactly `levels` corrections: positive releases, negative acquires.
///
/// The count is computed once from an observed depth rather than by re-reading
/// the global in a loop, so this always terminates.
pub fn correct_by(context: &ContextLease, levels: i32) {
    require_queue(context);
    // SAFETY: the supplied owner retains producer-thread access to the initialized queue.
    unsafe {
        for _ in 0..levels.max(0) {
            UnbatchGraphics();
        }
        for _ in 0..(-levels).max(0) {
            BatchGraphics();
        }
    }
}
