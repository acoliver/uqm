//! Subsystem teardown sequence — mirrors C `main()` shutdown (uqm.c:479-507).
//!
//! @plan PLAN-20260707-BINARY-INVERSION.P04
//! @requirement REQ-BI-004
//!
//! # Order Matters
//!
//! The teardown order is critical — subsystems are torn down in reverse
//! dependency order. For example, graphics must be torn down before color
//! maps, which must be torn down before the task system (which runs the
//! graphics consumer thread in two-thread mode).

use super::c_main_extern;

/// Teardown the full UQM subsystem stack.
///
/// Mirrors the `if (MainExited)` block in C `main()` (uqm.c:479-507).
/// Called by Rust `main()` after the game loop returns.
///
/// # Safety
///
/// Calls many C FFI functions that access global state. Must only be called
/// after the game kernel has been shut down (game loop returned, threads
/// joined).
///
/// @plan PLAN-20260707-BINARY-INVERSION.P04
pub fn teardown_subsystems() {
    // SAFETY: All these functions are safe to call in sequence after the
    // game loop has ended. The order matches C main() exactly.
    unsafe {
        // 1. Input
        c_main_extern::TFB_UninitInput();

        // 2. Audio
        c_main_extern::unInitAudio();

        // 3. Communication
        c_main_extern::uninit_communication();

        // 4. Graphics: purge dangling resources, then tear down
        c_main_extern::TFB_PurgeDanglingGraphics();
        c_main_extern::UninitColorMaps();
        c_main_extern::TFB_UninitGraphics();

        // 5. NETPLAY teardown — Phase 2 scope (skipped)

        // 6. Callback / alarm systems
        c_main_extern::uqm_callback_uninit();
        c_main_extern::uqm_alarm_uninit();

        // 7. Task system, time system
        c_main_extern::CleanupTaskSystem();
        c_main_extern::UnInitTimeSystem();

        // 8. Directory / IO cleanup
        c_main_extern::uqm_unprepare_all_dirs();
        c_main_extern::uninitIO();

        // 9. Thread system
        c_main_extern::UnInitThreadSystem();

        // 10. Memory (USE_RUST_MEM=1 maps mem_uninit() to rust_mem_uninit())
        let _ = crate::memory::rust_mem_uninit();
    }

    // 11. Free options.addons (C-managed memory via HFree)
    unsafe {
        c_main_extern::uqm_free_options_addons();
    }
}

/// Constants from C headers needed by the init/teardown sequence.
///
/// @plan PLAN-20260707-BINARY-INVERSION.P04
pub mod constants {
    use std::os::raw::c_int;

    /// EXIT_SUCCESS from <stdlib.h>
    pub const EXIT_SUCCESS: c_int = 0;
    /// EXIT_FAILURE from <stdlib.h>
    pub const EXIT_FAILURE: c_int = 1;

    /// Default log level (uqm.c:285)
    pub const DEFAULT_LOG_LEVEL: c_int = 15;

    /// Default stack size for Starcon2Main thread (uqm.c:454).
    /// Not used in Rust-owned main (no game thread), but documented.
    pub const STARCON2_MAIN_STACK_SIZE: c_int = 1024;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_constants() {
        assert_eq!(constants::EXIT_SUCCESS, 0);
        assert_eq!(constants::EXIT_FAILURE, 1);
    }

    #[test]
    fn test_default_log_level() {
        assert_eq!(constants::DEFAULT_LOG_LEVEL, 15);
    }
}
