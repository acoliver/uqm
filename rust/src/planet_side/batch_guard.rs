//! Batch-depth policy for synchronous surface pickup callbacks.
//!
//! A discovery report runs a nested input loop. It must enter unbatched so the
//! draw queue can publish the text before waiting for input. A callback that
//! returns with changed depth is reported after restoring the queue, allowing
//! the failure itself to be drawn. Panics propagate without recovery.

use super::runtime::AdapterError;

/// What a callback did to the graphics batch depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchVerdict {
    /// Entered unbatched and left the depth as it found it.
    Balanced,
    /// Entry depth was not zero, so a report could not have been presented.
    EntryNotUnbatched(i32),
    /// The callback returned with the depth changed by this many levels.
    Drift(i32),
}

/// Classify a callback's effect on the batch depth.
#[must_use]
pub const fn classify(entry: i32, exit: i32) -> BatchVerdict {
    if entry != 0 {
        BatchVerdict::EntryNotUnbatched(entry)
    } else if exit != entry {
        BatchVerdict::Drift(exit - entry)
    } else {
        BatchVerdict::Balanced
    }
}

use crate::graphics::batch_depth::{correct_by, depth};

/// Invoke a native callback that must run unbatched, and hold it to that.
///
/// Fails if the depth is not zero on entry, or if the callback returns having
/// changed it. `operation` names the boundary for diagnostics.
pub fn calling_native_unbatched<T>(
    context: &crate::graphics::context_abi::ContextLease,
    operation: &'static str,
    body: impl FnOnce() -> T,
) -> Result<T, AdapterError> {
    let entry = depth(context);
    if let BatchVerdict::EntryNotUnbatched(_) = classify(entry, entry) {
        return Err(AdapterError::new(operation));
    }

    let value = body();

    match classify(entry, depth(context)) {
        BatchVerdict::Balanced => Ok(value),
        BatchVerdict::EntryNotUnbatched(_) => Err(AdapterError::new(operation)),
        BatchVerdict::Drift(levels) => {
            // Put the queue back before reporting, so the failure can be drawn.
            correct_by(context, levels);
            Err(AdapterError {
                operation,
                batch_depth_corrections: levels,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn batch_observation_rejects_an_uninitialized_queue() {
        let context = crate::graphics::context_abi::LoadedContext::create(c"queue-admission");
        let context = context.lease();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| depth(&context)));
        assert!(
            result.is_err(),
            "batch observation accepted an absent queue"
        );
    }

    #[test]
    #[serial_test::serial]
    fn production_boundary_rejects_drift_and_restores_the_queue() {
        use crate::graphics::{
            dcqueue_abi::{Init_DrawCommandQueue, Uninit_DrawCommandQueue},
            gfx_common_abi::{BatchGraphics, GetBatchDepth, UnbatchGraphics},
        };
        let context = crate::graphics::context_abi::LoadedContext::create(c"queue-callback");
        let context = context.lease();
        struct Queue;
        impl Drop for Queue {
            fn drop(&mut self) {
                // SAFETY: this serialized fixture owns the initialized queue.
                unsafe { Uninit_DrawCommandQueue() };
            }
        }
        // SAFETY: the serialized fixture owns the queue through its guard.
        unsafe { Init_DrawCommandQueue() };
        let _queue = Queue;
        let error = calling_native_unbatched(&context, "pickup_drift", || {
            // SAFETY: the serialized callback deliberately introduces one leaked level.
            unsafe { BatchGraphics() };
        });
        assert_eq!(error.unwrap_err().batch_depth_corrections, 1);
        // SAFETY: the serialized fixture owns this queue.
        assert_eq!(unsafe { GetBatchDepth() }, 0);
        assert_eq!(calling_native_unbatched(&context, "balanced", || 7), Ok(7));
        // SAFETY: the serialized fixture balances this level before dropping the queue.
        unsafe { BatchGraphics() };
        assert!(
            calling_native_unbatched(&context, "held", || panic!("held callback ran")).is_err()
        );
        // SAFETY: balance the level acquired above.
        unsafe { UnbatchGraphics() };
    }

    #[test]
    fn entering_unbatched_and_leaving_it_alone_is_balanced() {
        assert_eq!(classify(0, 0), BatchVerdict::Balanced);
    }

    #[test]
    fn a_held_batch_on_entry_is_rejected_before_the_callback_runs() {
        // A report presented under a held batch would wait for input behind a
        // frame that is never published.
        assert_eq!(classify(1, 1), BatchVerdict::EntryNotUnbatched(1));
        assert_eq!(classify(3, 0), BatchVerdict::EntryNotUnbatched(3));
    }

    #[test]
    fn a_callback_that_rebatches_without_holding_one_is_drift() {
        // The retired native report helper against a Rust caller: its
        // UnbatchGraphics is a no-op at depth zero, its BatchGraphics is not.
        assert_eq!(classify(0, 1), BatchVerdict::Drift(1));
    }

    #[test]
    fn a_callback_that_over_releases_is_also_drift() {
        assert_eq!(classify(0, -1), BatchVerdict::Drift(-1));
    }

    #[test]
    fn drift_is_never_reported_as_balanced() {
        for exit in -4..=4 {
            let verdict = classify(0, exit);
            if exit == 0 {
                assert_eq!(verdict, BatchVerdict::Balanced);
            } else {
                assert_eq!(
                    verdict,
                    BatchVerdict::Drift(exit),
                    "a changed depth must be reported, not absorbed"
                );
            }
        }
    }
}
