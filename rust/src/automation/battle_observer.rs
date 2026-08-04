//! Narrow monotonic read-only battle-frame observer for S3 semantic proof.
//!
//! This module provides a typed, bounds-safe battle-frame counter that can be
//! incremented from C code via FFI and observed/asserted from Rust tests.
//! It is a read-only observation hook — it does not modify battle logic.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION S3 (authorized cross-domain correction #12)

use portable_atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Global monotonic battle-frame counter.
///
/// Incremented by `rust_battle_frame_advance()` from the C battle loop.
/// Read by Rust automation/proof code to assert battle progress.
static BATTLE_FRAME_COUNT: AtomicU64 = AtomicU64::new(0);

/// Reset the battle-frame counter to zero.
///
/// Called at the start of a battle. Safe to call multiple times.
///
/// # Safety
///
/// This function is safe to call from C.
#[no_mangle]
pub extern "C" fn rust_battle_frame_reset() {
    BATTLE_FRAME_COUNT.store(0, Ordering::Release);
}

/// Advance the battle-frame counter by one.
///
/// Called from the stable frame seam in the C battle loop.
/// This is a monotonic counter — it only goes up until reset.
///
/// # Safety
///
/// This function is safe to call from C.
#[no_mangle]
pub extern "C" fn rust_battle_frame_advance() {
    BATTLE_FRAME_COUNT.fetch_add(1, Ordering::AcqRel);
}

/// Read the current battle-frame count.
///
/// Returns the monotonic frame count since the last reset.
#[must_use]
pub fn current_frame() -> u64 {
    BATTLE_FRAME_COUNT.load(Ordering::Acquire)
}

/// Typed assertion that the battle has progressed at least `min_frames`.
///
/// Returns `Ok(())` if the frame count is >= `min_frames`, or an error
/// message describing the shortfall.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION S3
pub fn assert_progress(min_frames: u64) -> Result<u64, String> {
    let actual = current_frame();
    if actual >= min_frames {
        Ok(actual)
    } else {
        Err(format!(
            "battle frame assertion failed: expected >= {min_frames}, got {actual}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn reset_for_test() {
        BATTLE_FRAME_COUNT.store(0, Ordering::Release);
    }

    #[test]
    #[serial]
    fn frame_counter_starts_at_zero_after_reset() {
        reset_for_test();
        assert_eq!(current_frame(), 0);
    }

    #[test]
    #[serial]
    fn frame_counter_advances_monotonically() {
        reset_for_test();
        rust_battle_frame_advance();
        rust_battle_frame_advance();
        rust_battle_frame_advance();
        assert_eq!(current_frame(), 3);
    }

    #[test]
    #[serial]
    fn ffi_advance_matches_rust_advance() {
        reset_for_test();
        for _ in 0..10 {
            rust_battle_frame_advance();
        }
        assert_eq!(current_frame(), 10);
    }

    #[test]
    #[serial]
    fn ffi_reset_clears_counter() {
        reset_for_test();
        for _ in 0..5 {
            rust_battle_frame_advance();
        }
        assert_eq!(current_frame(), 5);
        rust_battle_frame_reset();
        assert_eq!(current_frame(), 0);
    }

    #[test]
    #[serial]
    fn assert_progress_succeeds_when_met() {
        reset_for_test();
        for _ in 0..20 {
            rust_battle_frame_advance();
        }
        assert_eq!(assert_progress(20), Ok(20));
        assert_eq!(assert_progress(15), Ok(20));
    }

    #[test]
    #[serial]
    fn assert_progress_fails_with_shortfall() {
        reset_for_test();
        for _ in 0..5 {
            rust_battle_frame_advance();
        }
        let err = assert_progress(10).unwrap_err();
        assert!(err.contains("expected >= 10"));
        assert!(err.contains("got 5"));
    }
}
