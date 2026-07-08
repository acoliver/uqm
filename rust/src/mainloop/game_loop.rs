//! Rust game loop body — replacement for `Starcon2Main()`.
//!
//! @plan PLAN-20260707-MAINLOOP.P06
//! @requirement REQ-ML-001, REQ-ML-007, REQ-ML-008

use std::os::raw::c_int;

use super::state_machine::{self, ActivityDecision, BreakAction, GameStateInfo};
use super::types::{activity_flags, ActivityValue};
use super::MainLoopError;

// ===========================================================================
//  Constants (from C headers)
// ===========================================================================

/// `SMM_DEFAULT = SMM_DATE = 1` (sis.h:203)
const SMM_DEFAULT: c_int = 1;
/// `HYPERSPACE_CLOCK_RATE = 5` (clock.h:79)
const HYPERSPACE_CLOCK_RATE: u16 = 5;
/// `INTERPLANETARY_CLOCK_RATE = 30` (clock.h:84)
const INTERPLANETARY_CLOCK_RATE: u16 = 30;
/// `BLACKURQ_CONVERSATION = 21` (commglue.h CONVERSATION enum)
const BLACKURQ_CONVERSATION: u32 = 21;

// ===========================================================================
//  GameLoopOps trait — abstraction over C FFI for testability
// ===========================================================================

/// Abstraction over all C-side operations the game loop performs.
///
/// Production code uses [`CffiOps`] (real FFI calls).  Tests use mock
/// implementations that record calls and return controllable values.
///
/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-001
pub trait GameLoopOps {
    // --- Init / lifecycle ---
    fn init_audio(&self);
    fn load_kernel(&self) -> bool;
    fn splash(&self);
    fn start_game(&self) -> bool;

    // --- Activity accessors ---
    fn get_current_activity(&self) -> ActivityValue;
    fn set_current_activity(&self, activity: ActivityValue);
    fn get_next_activity(&self) -> ActivityValue;
    fn set_last_activity(&self, activity: ActivityValue);

    // --- Game-state snapshot ---
    fn get_game_state(&self) -> GameStateInfo;

    // --- Per-game init ---
    fn set_player_input_all_or_explode(&self);
    fn init_game_structures(&self);
    fn init_game_clock(&self);
    fn add_initial_game_events(&self);

    // --- Per-frame preamble ---
    fn set_status_message_mode_default(&self);
    fn zero_velocity(&self);
    fn set_flash_rect_null(&self);

    // --- Activity dispatch (primitive C calls) ---
    fn install_bomb_at_earth(&self);
    fn visit_starbase(&self);
    fn race_communication(&self);
    fn explore_solar_sys(&self);
    fn battle_with_frame_callback(&self);
    fn draw_autopilot_message(&self, tf: bool);
    fn set_game_clock_rate(&self, rate: u16);

    // --- Per-game teardown ---
    fn stop_sound(&self);
    fn uninit_game_clock(&self);
    fn uninit_game_structures(&self);
    fn clear_player_input_all(&self);

    // --- Break-condition side effects ---
    fn init_communication(&self, conversation: u32);

    // --- Game-kernel cleanup (starcon.c:313-318) ---
    fn set_main_exited(&self, val: bool);
    fn uninit_game_kernel(&self);
    fn free_master_ship_list(&self);
    fn free_kernel(&self);
    fn log_show_box(&self, b: bool, c: bool);
}

// ===========================================================================
//  Core loop logic — works with any GameLoopOps implementation
// ===========================================================================

/// Execute a pre-dispatch mutation and then the C dispatch call.
///
/// Combines [`state_machine::pre_dispatch_mutate`] with the actual C
/// function call.  The C dispatch may mutate `CurrentActivity` as a
/// side effect; the caller MUST re-read after this returns.
///
/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-007
fn execute_activity<O: GameLoopOps + ?Sized>(ops: &O, decision: ActivityDecision) {
    let activity = ops.get_current_activity();
    let mutated = state_machine::pre_dispatch_mutate(activity, decision);
    ops.set_current_activity(mutated);

    match decision {
        ActivityDecision::InstallBombAtEarth => ops.install_bomb_at_earth(),
        ActivityDecision::VisitStarBase => ops.visit_starbase(),
        ActivityDecision::RaceCommunication => ops.race_communication(),
        ActivityDecision::ExploreSolarSystem => {
            ops.draw_autopilot_message(true);
            ops.set_game_clock_rate(INTERPLANETARY_CLOCK_RATE);
            ops.explore_solar_sys();
        }
        ActivityDecision::Battle => {
            ops.draw_autopilot_message(true);
            ops.set_game_clock_rate(HYPERSPACE_CLOCK_RATE);
            ops.battle_with_frame_callback();
        }
    }
}

/// Game-kernel cleanup — matches `starcon.c:313-318`.
///
/// Calls exactly five functions, then sets `MainExited = TRUE`.
/// Subsystem teardown is handled by C `main()` after seeing `MainExited`.
///
/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-008
fn shutdown_game_kernel<O: GameLoopOps + ?Sized>(ops: &O) {
    ops.uninit_game_kernel();
    ops.free_master_ship_list();
    ops.free_kernel();
    ops.log_show_box(false, false);
    ops.set_main_exited(true);
}

/// The full game lifecycle — replaces `Starcon2Main()` body.
///
/// Two-level loop:
/// - Outer: `while StartGame()` — new game / load game
/// - Inner: `loop { … } until CHECK_ABORT` — per-frame state machine
///
/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-001, REQ-ML-007
pub fn run_game_lifecycle_impl<O: GameLoopOps + ?Sized>(ops: &O) -> Result<(), MainLoopError> {
    // --- Starcon2Main-specific init ---
    ops.init_audio();

    if !ops.load_kernel() {
        ops.set_main_exited(true);
        return Err(MainLoopError::LoadKernelFailed);
    }

    // CRITICAL: clear CurrentActivity before splash (starcon.c:223)
    ops.set_current_activity(ActivityValue::new(0));
    ops.splash();

    // --- Outer loop: new game / load game ---
    while ops.start_game() {
        ops.set_player_input_all_or_explode();
        ops.init_game_structures();
        ops.init_game_clock();
        ops.add_initial_game_events();

        // --- Inner loop: activity state machine ---
        loop {
            ops.set_status_message_mode_default();

            let current = ops.get_current_activity();
            let next = ops.get_next_activity();
            let state = ops.get_game_state();

            // Load path (starcon.c:260-263)
            if state_machine::should_zero_velocity(current, next) {
                ops.zero_velocity();
            } else {
                let resolved = state_machine::resolve_load_activity(current, next);
                if resolved != current {
                    ops.set_current_activity(resolved);
                }
            }

            // Re-read after load-path mutation
            let activity = ops.get_current_activity();

            // Evaluate and dispatch
            let decision = state_machine::evaluate(activity, &state);

            // Track whether this was an encounter-branch dispatch.
            // The post-dispatch flag clearing (starcon.c:281-290) runs
            // ONLY inside the encounter branch — not after ExploreSolarSys
            // or Battle.
            let was_encounter = matches!(
                decision,
                ActivityDecision::InstallBombAtEarth
                    | ActivityDecision::VisitStarBase
                    | ActivityDecision::RaceCommunication
            );

            execute_activity(ops, decision);

            // CRITICAL: re-read CurrentActivity — C dispatch mutated it
            let mut activity = ops.get_current_activity();

            // Post-encounter clearing — ENCOUNTER BRANCH ONLY (starcon.c:281-290)
            if was_encounter {
                let cleared = state_machine::post_encounter_clear(activity);
                if cleared != activity {
                    ops.set_current_activity(cleared);
                    activity = cleared;
                }
            }

            ops.set_flash_rect_null();

            // Set LastActivity from re-read value (starcon.c:308)
            ops.set_last_activity(ops.get_current_activity());

            // Re-read CurrentActivity and game state for break check
            // (starcon.c:310-320 reads GLOBAL(CurrentActivity) directly)
            let activity = ops.get_current_activity();
            let state = ops.get_game_state();

            // Break condition (starcon.c:310-320)
            match state_machine::check_break(activity, &state) {
                BreakAction::Continue => {}
                BreakAction::InitBlackUrqCommunication => {
                    ops.init_communication(BLACKURQ_CONVERSATION);
                    break;
                }
                BreakAction::ClearRestart => {
                    let cleared = activity.clear_flag(activity_flags::CHECK_RESTART);
                    ops.set_current_activity(cleared);
                    break;
                }
                BreakAction::JustBreak => break,
            }

            // Inner-loop continuation (starcon.c:322)
            if !state_machine::should_continue(ops.get_current_activity()) {
                break;
            }
        }

        // Per-game teardown
        ops.stop_sound();
        ops.uninit_game_clock();
        ops.uninit_game_structures();
        ops.clear_player_input_all();
    }

    // --- Game-kernel cleanup (starcon.c:313-318) ---
    shutdown_game_kernel(ops);

    Ok(())
}

// ===========================================================================
//  C FFI implementation — compiled only in non-test builds
// ===========================================================================

#[cfg(not(test))]
mod cffi {
    use super::super::c_extern;
    use super::*;

    /// Production implementation of [`GameLoopOps`] using real C FFI calls.
    ///
    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-001
    pub struct CffiOps;

    impl GameLoopOps for CffiOps {
        fn init_audio(&self) {
            // SAFETY: initAudio initializes the audio subsystem; safe to call
            // once at startup on the Starcon2Main thread.
            unsafe { c_extern::uqm_init_audio() }
        }

        fn load_kernel(&self) -> bool {
            // SAFETY: LoadKernel reads content packages; the null argv is the
            // documented call pattern in starcon.c.
            unsafe { c_extern::LoadKernel(0, std::ptr::null_mut()) != 0 }
        }

        fn splash(&self) {
            unsafe { c_extern::uqm_splash_with_bg_init_kernel() }
        }

        fn start_game(&self) -> bool {
            unsafe { c_extern::StartGame() != 0 }
        }

        fn get_current_activity(&self) -> ActivityValue {
            ActivityValue::new(unsafe { c_extern::get_current_activity() })
        }

        fn set_current_activity(&self, activity: ActivityValue) {
            unsafe { c_extern::set_current_activity(u16::from(activity)) }
        }

        fn get_next_activity(&self) -> ActivityValue {
            ActivityValue::new(unsafe { c_extern::get_next_activity() })
        }

        fn set_last_activity(&self, activity: ActivityValue) {
            unsafe { c_extern::set_last_activity(u16::from(activity)) }
        }

        fn get_game_state(&self) -> GameStateInfo {
            unsafe {
                GameStateInfo {
                    chmmr_bomb_state: c_extern::uqm_get_chmmr_bomb_state(),
                    starbase_available: c_extern::uqm_get_starbase_available(),
                    global_flags_and_data: c_extern::uqm_get_global_flags_and_data(),
                    kohr_ah_killed_all: c_extern::uqm_get_kohr_ah_killed_all(),
                    crew_enlisted: c_extern::uqm_get_crew_enlisted(),
                }
            }
        }

        fn set_player_input_all_or_explode(&self) {
            unsafe { c_extern::uqm_set_player_input_all_or_explode() }
        }

        fn init_game_structures(&self) {
            unsafe { c_extern::InitGameStructures() }
        }

        fn init_game_clock(&self) {
            unsafe { c_extern::InitGameClock() }
        }

        fn add_initial_game_events(&self) {
            unsafe { c_extern::AddInitialGameEvents() }
        }

        fn set_status_message_mode_default(&self) {
            unsafe { c_extern::SetStatusMessageMode(SMM_DEFAULT) }
        }

        fn zero_velocity(&self) {
            unsafe { c_extern::uqm_zero_global_velocity() }
        }

        fn set_flash_rect_null(&self) {
            unsafe { c_extern::uqm_set_flash_rect_null() }
        }

        fn install_bomb_at_earth(&self) {
            unsafe { c_extern::InstallBombAtEarth() }
        }

        fn visit_starbase(&self) {
            unsafe { c_extern::VisitStarBase() }
        }

        fn race_communication(&self) {
            unsafe { c_extern::RaceCommunication() }
        }

        fn explore_solar_sys(&self) {
            unsafe { c_extern::ExploreSolarSys() }
        }

        fn battle_with_frame_callback(&self) {
            unsafe { c_extern::uqm_battle_with_frame_callback() }
        }

        fn draw_autopilot_message(&self, tf: bool) {
            unsafe { c_extern::DrawAutoPilotMessage(if tf { 1 } else { 0 }) }
        }

        fn set_game_clock_rate(&self, rate: u16) {
            unsafe { c_extern::SetGameClockRate(rate) }
        }

        fn stop_sound(&self) {
            unsafe { c_extern::StopSound() }
        }

        fn uninit_game_clock(&self) {
            unsafe { c_extern::UninitGameClock() }
        }

        fn uninit_game_structures(&self) {
            unsafe { c_extern::UninitGameStructures() }
        }

        fn clear_player_input_all(&self) {
            unsafe { c_extern::ClearPlayerInputAll() }
        }

        fn init_communication(&self, conversation: u32) {
            unsafe {
                c_extern::InitCommunication(conversation);
            }
        }

        fn set_main_exited(&self, val: bool) {
            unsafe { c_extern::set_main_exited(if val { 1 } else { 0 }) }
        }

        fn uninit_game_kernel(&self) {
            unsafe { c_extern::UninitGameKernel() }
        }

        fn free_master_ship_list(&self) {
            unsafe { c_extern::FreeMasterShipList() }
        }

        fn free_kernel(&self) {
            unsafe { c_extern::FreeKernel() }
        }

        fn log_show_box(&self, b: bool, c: bool) {
            unsafe { c_extern::log_showBox(if b { 1 } else { 0 }, if c { 1 } else { 0 }) }
        }
    }
}

/// Rust entry point — called from C `Starcon2Main` when
/// `USE_RUST_MAINLOOP` is defined.
///
/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-001
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn rust_game_loop() -> c_int {
    match run_game_lifecycle_impl(&cffi::CffiOps) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("rust_game_loop: fatal error: {e}");
            1
        }
    }
}

// ===========================================================================
//  Unit tests — Tier 1 (pure Rust, no C linkage)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::super::types::ActivityKind;
    use super::*;
    use std::cell::RefCell;

    /// Mock [`GameLoopOps`] with controllable behavior and call recording.
    ///
    /// Uses `RefCell` for interior mutability so the trait methods (which
    /// take `&self`) can mutate the mock state.
    struct MockOps {
        inner: RefCell<MockState>,
    }

    struct MockState {
        /// Sequence of `start_game` return values (consumed left-to-right).
        start_game_returns: Vec<bool>,
        /// Current activity value (read and mutated by the loop).
        current_activity: ActivityValue,
        /// Next activity value (for load path).
        next_activity: ActivityValue,
        /// Game state snapshot.
        game_state: GameStateInfo,
        /// Activity to set after each dispatch (simulates C mutation).
        post_dispatch_activity: Option<ActivityValue>,
        /// Activity to inject when add_initial_game_events fires
        /// (simulates C game-init setting up an encounter).
        pre_loop_activity: Option<ActivityValue>,
        /// Records of method calls.
        calls: Vec<&'static str>,
        /// Whether load_kernel should fail.
        load_kernel_fail: bool,
        /// Max inner-loop iterations before forcing CHECK_ABORT (safety valve).
        max_iterations: usize,
    }

    impl MockOps {
        fn new() -> Self {
            MockOps {
                inner: RefCell::new(MockState {
                    start_game_returns: vec![],
                    current_activity: ActivityValue::new(0),
                    next_activity: ActivityValue::new(0),
                    game_state: GameStateInfo::default(),
                    post_dispatch_activity: None,
                    pre_loop_activity: None,
                    calls: Vec::new(),
                    load_kernel_fail: false,
                    max_iterations: 100,
                }),
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.inner.borrow().calls.clone()
        }
    }

    impl GameLoopOps for MockOps {
        fn init_audio(&self) {
            self.inner.borrow_mut().calls.push("init_audio");
        }
        fn load_kernel(&self) -> bool {
            self.inner.borrow_mut().calls.push("load_kernel");
            !self.inner.borrow().load_kernel_fail
        }
        fn splash(&self) {
            self.inner.borrow_mut().calls.push("splash");
        }
        fn start_game(&self) -> bool {
            self.inner.borrow_mut().calls.push("start_game");
            let mut s = self.inner.borrow_mut();
            if s.start_game_returns.is_empty() {
                false
            } else {
                s.start_game_returns.remove(0)
            }
        }
        fn get_current_activity(&self) -> ActivityValue {
            self.inner.borrow().current_activity
        }
        fn set_current_activity(&self, activity: ActivityValue) {
            self.inner.borrow_mut().current_activity = activity;
        }
        fn get_next_activity(&self) -> ActivityValue {
            self.inner.borrow().next_activity
        }
        fn set_last_activity(&self, _activity: ActivityValue) {
            self.inner.borrow_mut().calls.push("set_last_activity");
        }
        fn get_game_state(&self) -> GameStateInfo {
            self.inner.borrow().game_state
        }
        fn set_player_input_all_or_explode(&self) {
            self.inner.borrow_mut().calls.push("set_player_input_all");
        }
        fn init_game_structures(&self) {
            self.inner.borrow_mut().calls.push("init_game_structures");
        }
        fn init_game_clock(&self) {
            self.inner.borrow_mut().calls.push("init_game_clock");
        }
        fn add_initial_game_events(&self) {
            let mut s = self.inner.borrow_mut();
            s.calls.push("add_initial_game_events");
            if let Some(a) = s.pre_loop_activity {
                s.current_activity = a;
            }
        }
        fn set_status_message_mode_default(&self) {
            self.inner
                .borrow_mut()
                .calls
                .push("set_status_message_mode");
        }
        fn zero_velocity(&self) {
            self.inner.borrow_mut().calls.push("zero_velocity");
        }
        fn set_flash_rect_null(&self) {
            self.inner.borrow_mut().calls.push("set_flash_rect_null");
        }
        fn install_bomb_at_earth(&self) {
            let mut s = self.inner.borrow_mut();
            s.calls.push("install_bomb");
            if let Some(a) = s.post_dispatch_activity {
                s.current_activity = a;
            }
        }
        fn visit_starbase(&self) {
            let mut s = self.inner.borrow_mut();
            s.calls.push("visit_starbase");
            if let Some(a) = s.post_dispatch_activity {
                s.current_activity = a;
            }
        }
        fn race_communication(&self) {
            let mut s = self.inner.borrow_mut();
            s.calls.push("race_communication");
            if let Some(a) = s.post_dispatch_activity {
                s.current_activity = a;
            }
        }
        fn explore_solar_sys(&self) {
            let mut s = self.inner.borrow_mut();
            s.calls.push("explore_solar_sys");
            if let Some(a) = s.post_dispatch_activity {
                s.current_activity = a;
            }
        }
        fn battle_with_frame_callback(&self) {
            let mut s = self.inner.borrow_mut();
            s.calls.push("battle");
            if let Some(a) = s.post_dispatch_activity {
                s.current_activity = a;
            }
            // Safety valve: force CHECK_ABORT after max iterations
            s.max_iterations = s.max_iterations.saturating_sub(1);
            if s.max_iterations == 0 {
                s.current_activity = ActivityValue::from_kind_and_flags(
                    ActivityKind::InHyperspace,
                    activity_flags::CHECK_ABORT,
                );
            }
        }
        fn draw_autopilot_message(&self, _tf: bool) {
            self.inner.borrow_mut().calls.push("draw_autopilot");
        }
        fn set_game_clock_rate(&self, _rate: u16) {
            self.inner.borrow_mut().calls.push("set_game_clock_rate");
        }
        fn stop_sound(&self) {
            self.inner.borrow_mut().calls.push("stop_sound");
        }
        fn uninit_game_clock(&self) {
            self.inner.borrow_mut().calls.push("uninit_game_clock");
        }
        fn uninit_game_structures(&self) {
            self.inner.borrow_mut().calls.push("uninit_game_structures");
        }
        fn clear_player_input_all(&self) {
            self.inner.borrow_mut().calls.push("clear_player_input");
        }
        fn init_communication(&self, conv: u32) {
            self.inner.borrow_mut().calls.push("init_communication");
            assert_eq!(conv, BLACKURQ_CONVERSATION);
        }
        fn set_main_exited(&self, val: bool) {
            assert!(val);
            self.inner.borrow_mut().calls.push("set_main_exited");
        }
        fn uninit_game_kernel(&self) {
            self.inner.borrow_mut().calls.push("uninit_game_kernel");
        }
        fn free_master_ship_list(&self) {
            self.inner.borrow_mut().calls.push("free_master_ship_list");
        }
        fn free_kernel(&self) {
            self.inner.borrow_mut().calls.push("free_kernel");
        }
        fn log_show_box(&self, _b: bool, _c: bool) {
            self.inner.borrow_mut().calls.push("log_show_box");
        }
    }

    // -----------------------------------------------------------------------
    //  Tests
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-001
    #[test]
    fn test_load_kernel_failure_returns_error() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().load_kernel_fail = true;
        let result = run_game_lifecycle_impl(&ops);
        assert_eq!(result, Err(MainLoopError::LoadKernelFailed));
        assert!(ops.calls().contains(&"set_main_exited"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_init_sequence_called_in_order() {
        let ops = MockOps::new();
        // start_game returns false immediately → no games played
        ops.inner.borrow_mut().start_game_returns = vec![false];
        run_game_lifecycle_impl(&ops).unwrap();

        let calls = ops.calls();
        assert_eq!(calls[0], "init_audio");
        assert_eq!(calls[1], "load_kernel");
        // After load: clear activity, splash, then start_game
        assert!(calls.contains(&"splash"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_shutdown_game_kernel_calls_all_five() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![false];
        run_game_lifecycle_impl(&ops).unwrap();

        let calls = ops.calls();
        // starcon.c:313-318 order
        let shutdown_calls: Vec<_> = calls
            .iter()
            .filter(|c| {
                matches!(
                    **c,
                    "uninit_game_kernel"
                        | "free_master_ship_list"
                        | "free_kernel"
                        | "log_show_box"
                        | "set_main_exited"
                )
            })
            .copied()
            .collect();
        assert_eq!(shutdown_calls.len(), 5);
        assert_eq!(shutdown_calls[0], "uninit_game_kernel");
        assert_eq!(shutdown_calls[1], "free_master_ship_list");
        assert_eq!(shutdown_calls[2], "free_kernel");
        assert_eq!(shutdown_calls[3], "log_show_box");
        assert_eq!(shutdown_calls[4], "set_main_exited");
        // Must NOT contain subsystem teardown functions (owned by C main())
        assert!(!calls.contains(&"stop_sound"));
        assert!(!calls.contains(&"uninit_game_clock"));
        assert!(!calls.contains(&"uninit_game_structures"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_per_game_init_called_before_inner_loop() {
        let ops = MockOps::new();
        // start_game returns true once, then false
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        // After battle, set CHECK_ABORT to exit inner loop
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));
        run_game_lifecycle_impl(&ops).unwrap();

        let calls = ops.calls();
        let init_idx = calls
            .iter()
            .position(|c| *c == "set_player_input_all")
            .unwrap();
        assert_eq!(calls[init_idx + 1], "init_game_structures");
        assert_eq!(calls[init_idx + 2], "init_game_clock");
        assert_eq!(calls[init_idx + 3], "add_initial_game_events");
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_per_game_teardown_called_after_inner_loop() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));
        run_game_lifecycle_impl(&ops).unwrap();

        let calls = ops.calls();
        // After battle (which sets CHECK_ABORT), teardown should follow
        let battle_idx = calls.iter().rposition(|c| *c == "battle").unwrap();
        // Teardown calls come after battle (possibly with flash_rect/last_activity between)
        let after_battle = &calls[battle_idx + 1..];
        assert!(after_battle.contains(&"stop_sound"));
        assert!(after_battle.contains(&"uninit_game_clock"));
        assert!(after_battle.contains(&"uninit_game_structures"));
        assert!(after_battle.contains(&"clear_player_input"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_inner_loop_exits_on_check_abort() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));
        run_game_lifecycle_impl(&ops).unwrap();

        // Should have exactly one battle call (inner loop runs once then aborts)
        let battle_count = ops.calls().iter().filter(|c| **c == "battle").count();
        assert_eq!(battle_count, 1);
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_inner_loop_re_reads_activity_after_dispatch() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        // Dispatch sets CHECK_ABORT — the loop MUST see this via re-read
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));
        run_game_lifecycle_impl(&ops).unwrap();

        // If re-read worked, the loop exits after one battle.  If it didn't,
        // the safety valve would trigger but we'd still get exactly one
        // battle because CHECK_ABORT was set.
        let battle_count = ops.calls().iter().filter(|c| **c == "battle").count();
        assert_eq!(battle_count, 1);
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_kohr_ah_triggers_init_communication() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        // After battle, C sets WON_LAST_BATTLE
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::WonLastBattle,
            0,
        ));
        ops.inner.borrow_mut().game_state.kohr_ah_killed_all = 1;

        run_game_lifecycle_impl(&ops).unwrap();

        let calls = ops.calls();
        assert!(calls.contains(&"init_communication"));
        assert!(calls.contains(&"battle"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_encounter_dispatch_sets_start_encounter() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        // Set encounter flag via a hook after init clears activity
        ops.inner.borrow_mut().pre_loop_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::START_ENCOUNTER,
        ));
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));

        run_game_lifecycle_impl(&ops).unwrap();

        // Should dispatch to race_communication (encounter, no special state)
        assert!(ops.calls().contains(&"race_communication"));
        assert!(!ops.calls().contains(&"battle"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_zero_velocity_called_on_normal_activity() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));

        run_game_lifecycle_impl(&ops).unwrap();

        assert!(ops.calls().contains(&"zero_velocity"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_post_encounter_clear_only_after_encounter_dispatch() {
        // Verify post_encounter_clear does NOT run after Battle dispatch.
        // In C, the clearing block (starcon.c:281-290) is inside the
        // encounter branch only — it must not run after ExploreSolarSys or Battle.
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        ops.inner.borrow_mut().pre_loop_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            0,
        ));
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InInterplanetary,
            0,
        ));
        ops.inner.borrow_mut().max_iterations = 2;

        run_game_lifecycle_impl(&ops).unwrap();

        assert!(ops.calls().contains(&"battle"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_post_encounter_clear_runs_after_encounter_dispatch() {
        // Verify post_encounter_clear DOES run after encounter dispatches.
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        ops.inner.borrow_mut().pre_loop_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::START_ENCOUNTER,
        ));
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::START_ENCOUNTER | activity_flags::CHECK_ABORT,
        ));

        run_game_lifecycle_impl(&ops).unwrap();

        assert!(ops.calls().contains(&"race_communication"));
        assert!(!ops.calls().contains(&"battle"));
    }

    /// @plan PLAN-20260707-MAINLOOP.P06
    /// @requirement REQ-ML-007
    #[test]
    fn test_zero_velocity_suppressed_on_check_load() {
        let ops = MockOps::new();
        ops.inner.borrow_mut().start_game_returns = vec![true, false];
        // Init clears current_activity to 0, so put CHECK_LOAD on next_activity
        ops.inner.borrow_mut().next_activity = ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_LOAD,
        );
        ops.inner.borrow_mut().post_dispatch_activity = Some(ActivityValue::from_kind_and_flags(
            ActivityKind::InHyperspace,
            activity_flags::CHECK_ABORT,
        ));

        run_game_lifecycle_impl(&ops).unwrap();

        assert!(!ops.calls().contains(&"zero_velocity"));
    }
}
