//! Init sequence orchestration for Rust-owned main().
//!
//! @plan PLAN-20260707-BINARY-INVERSION.P05
//! @requirement REQ-BI-003

use std::os::raw::c_int;

use super::c_main_extern;
use super::teardown;

/// Run the full UQM lifecycle: init -> game loop -> teardown.
///
/// This is the Rust-owned equivalent of C `main()` + `Starcon2Main()`.
///
/// @plan PLAN-20260707-BINARY-INVERSION.P05
/// # Safety
/// `argv` must point to `argc` valid, NUL-terminated argument strings plus
/// a null sentinel, and remain live during initialization. Call on the main thread.
pub unsafe fn run_uqm(argc: c_int, argv: *mut *mut std::os::raw::c_char) -> c_int {
    // Initialize subsystems using the Rust-parsed configuration.
    let init_result = unsafe { c_main_extern::initialize(argc, argv) };

    if init_result != 0 {
        // -1 means version/usage was printed (clean early exit)
        // Non-zero non-negative means option parsing failure
        if init_result < 0 {
            return 0; // version/usage — clean exit
        }
        return init_result; // error
    }

    // Phase 2: Game loop (runs directly on main thread, no StartThread)
    // rust_game_loop calls run_game_lifecycle_impl with CffiOps.
    // It handles LoadKernel, Splash, StartGame loop, and game-kernel cleanup.
    #[cfg(not(test))]
    let game_result = super::game_loop::rust_game_loop();
    #[cfg(test)]
    let game_result: c_int = 0; // stub for test builds

    // Phase 3: Subsystem teardown (mirrors C main() shutdown block)
    teardown::teardown_subsystems();

    // Return the game loop's exit code
    game_result as c_int
}
