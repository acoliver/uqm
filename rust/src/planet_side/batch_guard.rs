//! Rust ownership of the graphics batch depth across transitional callbacks.
//!
//! Selected-system pickup callbacks are still native, and some of them present
//! a discovery report that runs its own input loop. Those helpers were written
//! for the retired native lander loop, which held a batch open across the whole
//! surface frame. Rust PlanetSide holds no such ambient batch: it batches only
//! inside its own render.
//!
//! Two things follow, and both are invariants Rust owns rather than hopes for.
//!
//! A report cannot be presented underneath a held batch. While the depth is
//! non-zero the draw command queue never publishes, so a report that waits for
//! a keypress would wait behind a frame the player never sees. The depth is
//! therefore checked *before* the callback runs, not merely repaired after.
//!
//! A callback that changes the depth is a defect in project-owned code, not
//! unpredictable input, so it is reported rather than absorbed. The depth is
//! still returned to its entry value first, because leaving it wrong would stop
//! the failure itself from ever being drawn.
//!
//! Restoration applies to a normal return. A panic cannot cross the `extern "C"`
//! boundary these callbacks live behind without aborting, so there is no
//! unwinding path to restore.

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

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn GetBatchDepth() -> i32;
    fn BatchGraphics();
    fn UnbatchGraphics();
}

#[cfg(feature = "linked_c_archive")]
fn depth() -> i32 {
    // SAFETY: reads one int under the draw queue's own recursive mutex.
    unsafe { GetBatchDepth() }
}

/// Apply exactly `levels` corrections: positive releases, negative acquires.
///
/// The count is computed once from an observed depth rather than by re-reading
/// the global in a loop, so this always terminates.
#[cfg(feature = "linked_c_archive")]
fn correct_by(levels: i32) {
    // SAFETY: these are the queue's own reference-counting operations.
    unsafe {
        for _ in 0..levels.max(0) {
            UnbatchGraphics();
        }
        for _ in 0..(-levels).max(0) {
            BatchGraphics();
        }
    }
}

/// Invoke a native callback that must run unbatched, and hold it to that.
///
/// Fails if the depth is not zero on entry, or if the callback returns having
/// changed it. `operation` names the boundary for diagnostics.
#[cfg(feature = "linked_c_archive")]
pub fn calling_native_unbatched<T>(
    operation: &'static str,
    body: impl FnOnce() -> T,
) -> Result<T, AdapterError> {
    let entry = depth();
    if let BatchVerdict::EntryNotUnbatched(_) = classify(entry, entry) {
        return Err(AdapterError::new(operation));
    }

    let value = body();

    match classify(entry, depth()) {
        BatchVerdict::Balanced => Ok(value),
        BatchVerdict::EntryNotUnbatched(_) => Err(AdapterError::new(operation)),
        BatchVerdict::Drift(levels) => {
            // Put the queue back before reporting, so the failure can be drawn.
            correct_by(levels);
            super::telemetry::batch_depth_corrected(levels);
            Err(AdapterError::new(operation))
        }
    }
}

#[cfg(not(feature = "linked_c_archive"))]
pub fn calling_native_unbatched<T>(
    _operation: &'static str,
    body: impl FnOnce() -> T,
) -> Result<T, AdapterError> {
    Ok(body())
}

#[cfg(test)]
mod tests {
    use super::*;

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
