//! Raw `extern "C"` declarations for the UQM main loop FFI bridge.
//!
//! These declarations correspond to C wrapper functions created in P02b
//! (`sc2/src/uqm/rust_bridge_mainloop.c` / `.h`) and to existing C
//! functions (`LoadKernel`). Each Rust signature here MUST match the C
//! ABI exactly.
//!
//! # ABI Reference (verified against `libs/compiler.h`, `globdata.h`)
//!
//! | C type      | Rust type | Size  |
//! |-------------|-----------|-------|
//! | `BOOLEAN`   | `c_int`   | 4 B   |
//! | `ACTIVITY`  | `u16`     | 2 B   |
//! | `UWORD`     | `u16`     | 2 B   |
//! | `COUNT`     | `u16`     | 2 B   |
//! | `BYTE`      | `u8`      | 1 B   |
//! | C99 `bool`  | `bool`    | 1 B   |
//!
//! # Safety
//!
//! All functions in the `extern "C"` block are `unsafe` to call because
//! they cross the FFI boundary into C code. The safe wrappers in
//! [`crate::mainloop::ffi`] provide the public API.
//!
//! @plan PLAN-20260707-MAINLOOP.P03
//! @requirement REQ-ML-003

use std::ffi::c_char;
use std::os::raw::c_int;

use super::types::CBoolean;

// ---------------------------------------------------------------------------
// Activity accessors (from P02b: rust_bridge_mainloop.c)
// ---------------------------------------------------------------------------

extern "C" {
    /// Read `GLOBAL(CurrentActivity)` — the current activity value.
    ///
    /// C: `UWORD get_current_activity(void)` (defined in rust_bridge_macros.c).
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    pub fn get_current_activity() -> u16;

    /// Write `GLOBAL(CurrentActivity)`.
    ///
    /// C: `void set_current_activity(UWORD v)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    pub fn set_current_activity(val: u16);

    /// Read `NextActivity` — standalone global (`save.h:66`).
    ///
    /// Used by the load/restart path: `CurrentActivity | NextActivity & CHECK_LOAD`.
    ///
    /// C: `ACTIVITY get_next_activity(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    pub fn get_next_activity() -> u16;

    /// Write `NextActivity`.
    ///
    /// C: `void set_next_activity(ACTIVITY v)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    pub fn set_next_activity(val: u16);

    /// Read `LastActivity` — standalone global (`setup.h:60`).
    ///
    /// C: `ACTIVITY get_last_activity(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn get_last_activity() -> u16;

    /// Write `LastActivity`.
    ///
    /// C: `void set_last_activity(ACTIVITY v)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn set_last_activity(val: u16);
}

// ---------------------------------------------------------------------------
// Named game-state accessors (bit-packed via GET_GAME_STATE / SET_GAME_STATE)
// ---------------------------------------------------------------------------

extern "C" {
    /// Read `GET_GAME_STATE(CHMMR_BOMB_STATE)`.
    ///
    /// C: `BYTE uqm_get_chmmr_bomb_state(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-010
    pub fn uqm_get_chmmr_bomb_state() -> u8;

    /// Write `SET_GAME_STATE(CHMMR_BOMB_STATE, v)`.
    ///
    /// C: `void uqm_set_chmmr_bomb_state(BYTE v)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-010
    pub fn uqm_set_chmmr_bomb_state(val: u8);

    /// Read `GET_GAME_STATE(STARBASE_AVAILABLE)`.
    ///
    /// C: `BYTE uqm_get_starbase_available(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-010
    pub fn uqm_get_starbase_available() -> u8;

    /// Read `GET_GAME_STATE(GLOBAL_FLAGS_AND_DATA)`.
    ///
    /// C: `BYTE uqm_get_global_flags_and_data(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-010
    pub fn uqm_get_global_flags_and_data() -> u8;

    /// Read `GET_GAME_STATE(KOHR_AH_KILLED_ALL)`.
    ///
    /// C: `BYTE uqm_get_kohr_ah_killed_all(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-010
    pub fn uqm_get_kohr_ah_killed_all() -> u8;
}

// ---------------------------------------------------------------------------
// SIS (player ship) state accessors
// ---------------------------------------------------------------------------

extern "C" {
    /// Read `GLOBAL_SIS(CrewEnlisted)` as a `COUNT` (`UWORD`).
    ///
    /// Used for death detection (`starcon.c:295`).
    ///
    /// C: `COUNT uqm_get_crew_enlisted(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn uqm_get_crew_enlisted() -> u16;
}

// ---------------------------------------------------------------------------
// Macro / global wrappers
// ---------------------------------------------------------------------------

extern "C" {
    /// Wraps `ZeroVelocityComponents(&GLOBAL(velocity))`.
    ///
    /// C: `void uqm_zero_global_velocity(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn uqm_zero_global_velocity();

    /// Wraps `SetFlashRect(NULL)`.
    ///
    /// C: `void uqm_set_flash_rect_null(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn uqm_set_flash_rect_null();

    /// Calls `SetPlayerInputAll()`; on failure logs fatal and calls `explode()`.
    ///
    /// **Does not return** on failure (mirrors C main loop behavior).
    ///
    /// C: `void uqm_set_player_input_all_or_explode(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn uqm_set_player_input_all_or_explode();

    /// Sets the `MainExited` global.
    ///
    /// C: `void set_main_exited(BOOLEAN b)`.
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn set_main_exited(b: CBoolean);

    /// Calls `SplashScreen(BackgroundInitKernel)`.
    ///
    /// C: `void uqm_splash_with_bg_init_kernel(void)` (defined in starcon.c).
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn uqm_splash_with_bg_init_kernel();

    /// Calls `Battle(&on_battle_frame)`.
    ///
    /// C: `void uqm_battle_with_frame_callback(void)` (defined in starcon.c).
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn uqm_battle_with_frame_callback();
}

// ---------------------------------------------------------------------------
// Activity dispatch functions (void(void) — starcon.c, starbase.h, etc.)
//
// These are the top-level game-mode entry points invoked from the
// Starcon2Main inner loop. Each one runs its respective subsystem to
// completion (blocking) and may mutate GLOBAL(CurrentActivity) and
// game state as a side effect. After any of these returns, the caller
// MUST re-read CurrentActivity before making further decisions.
//
// @plan PLAN-20260707-MAINLOOP.P05
// @requirement REQ-ML-004
// ---------------------------------------------------------------------------

extern "C" {
    /// `extern void VisitStarBase(void)` — starbase encounter.
    ///
    /// Defined in `sc2/src/uqm/starbase.c` (declared in `starbase.h:39`).
    ///
    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    pub fn VisitStarBase();

    /// `extern void ExploreSolarSys(void)` — interplanetary exploration.
    ///
    /// Defined in `sc2/src/uqm/planets/` (declared in `planets/planets.h:284`).
    ///
    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    pub fn ExploreSolarSys();

    /// `extern void InstallBombAtEarth(void)` — BGD (bomb) mode.
    ///
    /// Defined in `sc2/src/uqm/starbase.c` (declared in `starbase.h:38`).
    ///
    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    pub fn InstallBombAtEarth();

    /// `extern void RaceCommunication(void)` — alien conversation.
    ///
    /// Defined in `sc2/src/uqm/comm/` (declared in `comm.h:116`).
    ///
    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    pub fn RaceCommunication();

    /// `extern COUNT InitCommunication(CONVERSATION which_comm)`.
    ///
    /// Begins the named conversation. `which_comm` is a `CONVERSATION`
    /// enum value (COUNT / unsigned). Returns a COUNT status.
    ///
    /// Used for the `KOHR_AH_KILLED_ALL` → `BLACKURQ_CONVERSATION` path
    /// in the combined win/loss check (starcon.c:315-316).
    ///
    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    pub fn InitCommunication(which_comm: u32) -> u16;
}

// ---------------------------------------------------------------------------
// LoadKernel — kernel initialization
// ---------------------------------------------------------------------------

extern "C" {
    /// Load the UQM kernel.
    ///
    /// C signature: `BOOLEAN LoadKernel(int argc, char *argv[])`.
    ///
    /// Returns `TRUE` (non-zero `CBoolean`) on success, `FALSE` on failure.
    ///
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn LoadKernel(argc: c_int, argv: *mut *mut c_char) -> CBoolean;
}

// ---------------------------------------------------------------------------
// Game-loop lifecycle functions (P06)
//
// These are the top-level game-mode functions called from the Rust
// game loop body. Each matches the C signature exactly.
//
// @plan PLAN-20260707-MAINLOOP.P06
// @requirement REQ-ML-001
// ---------------------------------------------------------------------------

extern "C" {
    /// `initAudio(snddriver, soundflags)` — C wrapper passes the globals.
    ///
    /// C: `void uqm_init_audio(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn uqm_init_audio();

    /// `BOOLEAN StartGame(void)` — outer-loop controller.
    ///
    /// Returns non-zero (true) to start/load a game, zero to quit.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn StartGame() -> CBoolean;

    /// `void InitGameStructures(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn InitGameStructures();

    /// `void InitGameClock(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn InitGameClock();

    /// `void AddInitialGameEvents(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn AddInitialGameEvents();

    /// `void SetStatusMessageMode(int mode)` — pass `SMM_DEFAULT` (1).
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn SetStatusMessageMode(mode: c_int);

    /// `void DrawAutoPilotMessage(BOOLEAN tf)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn DrawAutoPilotMessage(tf: CBoolean);

    /// `void SetGameClockRate(COUNT rate)` — `COUNT` = `u16`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn SetGameClockRate(rate: u16);

    /// `void StopSound(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn StopSound();

    /// `void UninitGameClock(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn UninitGameClock();

    /// `void UninitGameStructures(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn UninitGameStructures();

    /// `void ClearPlayerInputAll(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    pub fn ClearPlayerInputAll();

    /// `void UninitGameKernel(void)` — game-kernel cleanup.
    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-008
    pub fn UninitGameKernel();

    /// `void FreeMasterShipList(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-008
    pub fn FreeMasterShipList();

    /// `void FreeKernel(void)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-008
    pub fn FreeKernel();

    /// `void log_showBox(BOOLEAN b, BOOLEAN c)`.
    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-008
    pub fn log_showBox(b: CBoolean, c: CBoolean);
}

// ---------------------------------------------------------------------------
// Test-shim declarations — only linked in `cfg(test)` via rust_test_bridge.c
// ---------------------------------------------------------------------------

#[cfg(test)]
extern "C" {
    /// Test shim: set the test-local current activity.
    ///
    /// This writes to a **test-local** global in `rust_test_bridge.c`,
    /// NOT to the real `GlobData.Game_state.CurrentActivity`.
    ///
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn test_set_activity(val: u16);

    /// Test shim: get the test-local current activity.
    ///
    /// @plan PLAN-20260707-MAINLOOP.P03
    pub fn test_get_activity() -> u16;
}
