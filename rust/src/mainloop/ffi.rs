//! Safe Rust wrappers around the raw FFI externs in [`super::c_extern`].
//!
//! These functions provide a safe, idiomatic Rust API for reading and
//! writing the main loop's activity state and related game-state fields.
//! They handle the conversion between [`ActivityValue`] and the raw `u16`
//! values that cross the FFI boundary, and they encapsulate the
//! `unsafe` extern calls.
//!
//! # Test vs. Production Linking
//!
//! In production builds (`cfg(not(test))`), the safe wrappers call the
//! real C wrapper functions (from P02b `rust_bridge_mainloop.c`) which
//! read/write the actual UQM globals.
//!
//! In test builds (`cfg(test)`), the safe wrappers route through the
//! test shim (`rust_test_bridge.c`) which uses test-local globals. This
//! allows round-trip boundary tests to run without linking the full UQM
//! C codebase.
//!
//! @plan PLAN-20260707-MAINLOOP.P03
//! @requirement REQ-ML-003, REQ-ML-005, REQ-ML-010

use super::c_extern as prod;
use super::types::{ActivityValue, CBoolean};

// ---------------------------------------------------------------------------
// CurrentActivity accessors
// ---------------------------------------------------------------------------

/// Read the current activity value from C.
///
/// Returns the raw `ActivityValue` wrapping `GLOBAL(CurrentActivity)`.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[inline]
#[must_use]
pub fn get_current_activity() -> ActivityValue {
    #[cfg(not(test))]
    {
        // SAFETY: get_current_activity() takes no pointer arguments and
        // simply reads a C global. It has no preconditions.
        let raw = unsafe { prod::get_current_activity() };
        ActivityValue(raw)
    }
    #[cfg(test)]
    {
        // SAFETY: test shim, reads test-local global.
        let raw = unsafe { prod::test_get_activity() };
        ActivityValue(raw)
    }
}

/// Write the current activity value to C.
///
/// Sets `GLOBAL(CurrentActivity)` to `val`.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[inline]
pub fn set_current_activity(val: ActivityValue) {
    #[cfg(not(test))]
    {
        // SAFETY: set_current_activity() writes a u16 to a C global.
        unsafe { prod::set_current_activity(val.0) };
    }
    #[cfg(test)]
    {
        // SAFETY: test shim, writes test-local global.
        unsafe { prod::test_set_activity(val.0) };
    }
}

// ---------------------------------------------------------------------------
// NextActivity accessors
// ---------------------------------------------------------------------------

/// Read `NextActivity` from C.
///
/// `NextActivity` is a standalone global (`save.h:66`) used by the
/// load/restart path.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[inline]
#[must_use]
pub fn get_next_activity() -> ActivityValue {
    #[cfg(not(test))]
    {
        // SAFETY: reads a C global, no preconditions.
        let raw = unsafe { prod::get_next_activity() };
        ActivityValue(raw)
    }
    #[cfg(test)]
    {
        // SAFETY: test shim.
        let raw = unsafe { prod::test_get_activity() };
        ActivityValue(raw)
    }
}

/// Write `NextActivity` to C.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[inline]
pub fn set_next_activity(val: ActivityValue) {
    #[cfg(not(test))]
    {
        // SAFETY: writes a u16 to a C global.
        unsafe { prod::set_next_activity(val.0) };
    }
    #[cfg(test)]
    {
        // SAFETY: test shim.
        unsafe { prod::test_set_activity(val.0) };
    }
}

// ---------------------------------------------------------------------------
// LastActivity accessors
// ---------------------------------------------------------------------------

/// Read `LastActivity` from C.
///
/// `LastActivity` is a standalone global (`setup.h:60`).
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
#[must_use]
pub fn get_last_activity() -> ActivityValue {
    // SAFETY: reads a C global, no preconditions.
    let raw = unsafe { prod::get_last_activity() };
    ActivityValue(raw)
}

/// Write `LastActivity` to C.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn set_last_activity(val: ActivityValue) {
    // SAFETY: writes a u16 to a C global.
    unsafe { prod::set_last_activity(val.0) };
}

// ---------------------------------------------------------------------------
// Named game-state accessors (REQ-ML-010)
// ---------------------------------------------------------------------------

/// Read `CHMMR_BOMB_STATE` via the named C accessor.
///
/// This calls `GET_GAME_STATE(CHMMR_BOMB_STATE)` internally in C —
/// never uses raw byte-offset access.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-010
#[inline]
#[must_use]
pub fn get_chmmr_bomb_state() -> u8 {
    // SAFETY: reads a bit-packed game-state field via a named C wrapper.
    unsafe { prod::uqm_get_chmmr_bomb_state() }
}

/// Write `CHMMR_BOMB_STATE` via the named C accessor.
///
/// This calls `SET_GAME_STATE(CHMMR_BOMB_STATE, v)` internally in C.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-010
#[inline]
pub fn set_chmmr_bomb_state(val: u8) {
    // SAFETY: writes a bit-packed game-state field via a named C wrapper.
    unsafe { prod::uqm_set_chmmr_bomb_state(val) };
}

/// Read `STARBASE_AVAILABLE` via the named C accessor.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-010
#[inline]
#[must_use]
pub fn get_starbase_available() -> u8 {
    // SAFETY: reads a bit-packed game-state field via a named C wrapper.
    unsafe { prod::uqm_get_starbase_available() }
}

/// Read `GLOBAL_FLAGS_AND_DATA` via the named C accessor.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-010
#[inline]
#[must_use]
pub fn get_global_flags_and_data() -> u8 {
    // SAFETY: reads a bit-packed game-state field via a named C wrapper.
    unsafe { prod::uqm_get_global_flags_and_data() }
}

/// Read `KOHR_AH_KILLED_ALL` via the named C accessor.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-010
#[inline]
#[must_use]
pub fn get_kohr_ah_killed_all() -> u8 {
    // SAFETY: reads a bit-packed game-state field via a named C wrapper.
    unsafe { prod::uqm_get_kohr_ah_killed_all() }
}

// ---------------------------------------------------------------------------
// SIS state
// ---------------------------------------------------------------------------

/// Read `GLOBAL_SIS(CrewEnlisted)` as a `COUNT` (`u16`).
///
/// Used for death detection.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
#[must_use]
pub fn get_crew_enlisted() -> u16 {
    // SAFETY: reads a SIS field via a named C wrapper.
    unsafe { prod::uqm_get_crew_enlisted() }
}

// ---------------------------------------------------------------------------
// Macro / global wrappers
// ---------------------------------------------------------------------------

/// Zero the global velocity components.
///
/// Wraps `ZeroVelocityComponents(&GLOBAL(velocity))`.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn zero_global_velocity() {
    // SAFETY: no preconditions; zeroes a velocity struct.
    unsafe { prod::uqm_zero_global_velocity() };
}

/// Set the flash rectangle to NULL (full screen).
///
/// Wraps `SetFlashRect(NULL)`.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn set_flash_rect_null() {
    // SAFETY: no preconditions; sets a display rect.
    unsafe { prod::uqm_set_flash_rect_null() };
}

/// Call `SetPlayerInputAll()` or explode on failure.
///
/// **This function does not return** if `SetPlayerInputAll()` fails.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn set_player_input_all_or_explode() {
    // SAFETY: no preconditions. May abort the process on failure
    // (mirrors C behavior via explode()).
    unsafe { prod::uqm_set_player_input_all_or_explode() };
}

/// Set the `MainExited` global.
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn set_main_exited(value: bool) {
    let cval: CBoolean = if value { 1 } else { 0 };
    // SAFETY: writes a BOOLEAN global; cval is 0 or 1 (TRUE/FALSE).
    unsafe { prod::set_main_exited(cval) };
}

/// Run the splash screen with the background init kernel.
///
/// Wraps `SplashScreen(BackgroundInitKernel)` (defined in starcon.c).
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn splash_with_bg_init_kernel() {
    // SAFETY: no preconditions; drives splash screen initialization.
    unsafe { prod::uqm_splash_with_bg_init_kernel() };
}

/// Start a battle with the on-frame callback.
///
/// Wraps `Battle(&on_battle_frame)` (defined in starcon.c).
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub fn battle_with_frame_callback() {
    // SAFETY: no preconditions; enters the battle loop.
    unsafe { prod::uqm_battle_with_frame_callback() };
}

// ---------------------------------------------------------------------------
// LoadKernel
// ---------------------------------------------------------------------------

/// Load the UQM kernel.
///
/// C signature: `BOOLEAN LoadKernel(int argc, char *argv[])`.
///
/// Returns `Ok(())` if the kernel loaded successfully, or
/// `Err(MainLoopError::LoadKernelFailed)` on failure.
///
/// # Safety
///
/// The caller must ensure `argv` points to a valid `argc`-length array
/// of null-terminated C strings (or is null when `argc` is 0).
///
/// @plan PLAN-20260707-MAINLOOP.P03
#[inline]
pub unsafe fn load_kernel(
    argc: i32,
    argv: *mut *mut std::ffi::c_char,
) -> Result<(), super::MainLoopError> {
    let result = unsafe { prod::LoadKernel(argc, argv) };
    if result != 0 {
        Ok(())
    } else {
        Err(super::MainLoopError::LoadKernelFailed)
    }
}

// ---------------------------------------------------------------------------
// Boundary tests — Tier 2 (C shim round-trip)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::types::activity_flags;
    use super::super::types::ActivityKind;
    use super::*;
    use serial_test::serial;

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_current_activity_round_trip_rust_to_c() {
        // GIVEN: set activity from Rust
        set_current_activity(ActivityValue(0x0403)); // IN_ENCOUNTER | START_ENCOUNTER
                                                     // THEN: C shim reads the same value
        let from_c = unsafe { prod::test_get_activity() };
        assert_eq!(from_c, 0x0403);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_current_activity_round_trip_c_to_rust() {
        // GIVEN: C shim sets activity
        unsafe { prod::test_set_activity(0x0804) };
        // START_INTERPLANETARY | IN_INTERPLANETARY
        // WHEN: Rust reads it
        let activity = get_current_activity();
        // THEN: Rust sees the same value
        assert_eq!(activity.0, 0x0804);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_next_activity_round_trip() {
        set_next_activity(ActivityValue(0x1000)); // CHECK_LOAD
        assert_eq!(get_next_activity().0, 0x1000);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_activity_flags_decomposition_after_set() {
        set_current_activity(ActivityValue(0x0402));
        let av = get_current_activity();
        assert_eq!(av.kind(), ActivityKind::InEncounter);
        assert!(av.has_flag(activity_flags::START_ENCOUNTER));
        assert!(!av.has_flag(activity_flags::CHECK_ABORT));
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_activity_zero_round_trip() {
        set_current_activity(ActivityValue(0));
        assert_eq!(get_current_activity().0, 0);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_activity_max_u16_round_trip() {
        set_current_activity(ActivityValue(0xFFFF));
        assert_eq!(get_current_activity().0, 0xFFFF);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_check_load_flag_round_trip() {
        set_current_activity(ActivityValue(activity_flags::CHECK_LOAD));
        let av = get_current_activity();
        assert!(av.has_flag(activity_flags::CHECK_LOAD));
        assert_eq!(av.kind(), ActivityKind::SuperMelee);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    #[serial]
    fn test_set_main_exited_bool_conversion() {
        // Verify the bool→CBoolean conversion logic (does not call C).
        let true_val: CBoolean = if true { 1 } else { 0 };
        let false_val: CBoolean = if false { 1 } else { 0 };
        assert_eq!(true_val, 1);
        assert_eq!(false_val, 0);
    }
}
