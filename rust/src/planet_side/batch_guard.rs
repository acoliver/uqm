//! Rust ownership of the graphics batch depth across transitional callbacks.
//!
//! Selected-system pickup callbacks are still native, and some of them present
//! a discovery report that runs its own input loop. Those helpers were written
//! for the retired native lander loop, which held a batch open across the whole
//! surface frame, so they release and re-acquire a batch level around the
//! report. Rust PlanetSide holds no such ambient batch: it batches only inside
//! its own render.
//!
//! A leaked batch level is not a cosmetic problem. While the depth is non-zero
//! the draw command queue never publishes queued commands, so the game keeps
//! advancing but stops presenting entirely.
//!
//! Rust therefore owns the invariant at the boundary it controls: record the
//! depth before calling into a native callback and restore exactly that depth
//! afterwards, whatever the callback did.

use super::runtime::AdapterError;

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn GetBatchDepth() -> i32;
    fn BatchGraphics();
    fn UnbatchGraphics();
}

/// Read the current batch depth.
#[cfg(feature = "linked_c_archive")]
fn depth() -> i32 {
    // SAFETY: reads an integer guarded by the draw queue's own mutex.
    unsafe { GetBatchDepth() }
}

/// Drive the depth to `target`, returning the number of corrections applied.
#[cfg(feature = "linked_c_archive")]
fn restore_to(target: i32) -> i32 {
    let mut corrections = 0;
    // SAFETY: batch/unbatch are the queue's own reference-counting operations.
    unsafe {
        while depth() > target {
            UnbatchGraphics();
            corrections += 1;
        }
        while depth() < target {
            BatchGraphics();
            corrections += 1;
        }
    }
    corrections
}

/// Run `body` and restore the graphics batch depth it was entered with.
///
/// Returns the body's value. `operation` names the boundary for diagnostics if
/// the depth could not be restored.
#[cfg(feature = "linked_c_archive")]
pub fn preserving_batch_depth<T>(
    operation: &'static str,
    body: impl FnOnce() -> T,
) -> Result<T, AdapterError> {
    let entry = depth();
    let value = body();
    let corrections = restore_to(entry);
    if corrections > 0 {
        super::telemetry::batch_depth_corrected(corrections);
    }
    if depth() == entry {
        Ok(value)
    } else {
        Err(AdapterError::new(operation))
    }
}

#[cfg(not(feature = "linked_c_archive"))]
pub fn preserving_batch_depth<T>(
    _operation: &'static str,
    body: impl FnOnce() -> T,
) -> Result<T, AdapterError> {
    Ok(body())
}

/// Number of batch levels needed to get from `entry` back to `exit`.
///
/// Positive means the callback leaked levels that must be released; negative
/// means it released levels that must be re-acquired.
#[must_use]
pub const fn drift(entry: i32, exit: i32) -> i32 {
    exit - entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_balanced_callback_needs_no_correction() {
        assert_eq!(drift(0, 0), 0);
        assert_eq!(drift(2, 2), 0);
    }

    #[test]
    fn a_callback_that_rebatches_without_holding_one_leaks_a_level() {
        // This is exactly the retired native report helper against a Rust
        // caller: UnbatchGraphics is a no-op at depth zero while BatchGraphics
        // still increments.
        assert_eq!(drift(0, 1), 1);
    }

    #[test]
    fn a_callback_that_releases_too_much_is_also_drift() {
        assert_eq!(drift(1, 0), -1);
    }
}
