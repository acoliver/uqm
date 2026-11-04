//! Init sequence orchestration for Rust-owned main().
//!
//! @plan PLAN-20260707-BINARY-INVERSION.P05
//! @requirement REQ-BI-003

#[cfg(not(test))]
use std::os::raw::c_int;

#[cfg(not(test))]
use super::c_main_extern;
#[cfg(not(test))]
use super::teardown;

/// Run the full UQM lifecycle: init -> game loop -> teardown.
///
/// This is the Rust-owned equivalent of C `main()` + `Starcon2Main()`.
///
/// @plan PLAN-20260707-BINARY-INVERSION.P05
/// # Safety
/// `argv` must point to `argc` valid, NUL-terminated argument strings plus
/// a null sentinel, and remain live during initialization. Call on the main thread.
#[cfg(not(test))]
pub unsafe fn run_uqm(
    application: &mut crate::app::Application,
    argc: c_int,
    argv: *mut *mut std::os::raw::c_char,
) -> c_int {
    // Initialize subsystems using the Rust-parsed configuration.
    let observer = application.observer().cloned();
    // SAFETY: argv remains live and the application suspends Game for initialization.
    let init_result =
        unsafe { application.legacy(|| c_main_extern::initialize(argc, argv, observer)) };

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
    let game_result = {
        let observer = application.observer().cloned();
        super::game_loop::run_game_loop(&mut application.game, observer)
    };
    #[cfg(test)]
    let game_result: c_int = 0; // stub for test builds

    // Phase 3: Subsystem teardown (mirrors C main() shutdown block)
    // SAFETY: the game loop has stopped, and teardown returns before Game is released.
    unsafe { application.legacy(teardown::teardown_subsystems) };

    // Return the game loop's exit code
    game_result as c_int
}
