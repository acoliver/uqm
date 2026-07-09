//! RestartMenu, TryStartGame, and StartGame orchestration.
//!
//! These functions replace the C `RestartMenu()`, `TryStartGame()`,
//! and `StartGame()` in restart.c:262-423.
//!
//! @plan PLAN-20260707-RESTARTMENU.P07
//! @requirement REQ-RM-007

use std::os::raw::c_int;

use super::c_extern;
use super::do_restart::{self, DoRestartState};
use super::restart_ops::RestartMenuOps;
use super::types::RestartMenuItem;

// ===========================================================================
//  Constants (from C headers)
// ===========================================================================

/// `CHECK_ABORT = MAKE_WORD(0, 1<<6) = 0x4000`.
const CHECK_ABORT: u16 = 0x4000;
/// `CHECK_LOAD = MAKE_WORD(0, 1<<4) = 0x1000`.
const CHECK_LOAD: u16 = 0x1000;
/// `CHECK_RESTART = MAKE_WORD(0, 1<<5) = 0x2000`.
const CHECK_RESTART: u16 = 0x2000;
/// `SUPER_MELEE = 0`.
const SUPER_MELEE: u16 = 0;
/// `WON_LAST_BATTLE = 5` (globdata.h activity enum).
const WON_LAST_BATTLE: u16 = 5;
/// `FadeAllToWhite = 250` (gfxlib.h).
const FADE_ALL_TO_WHITE: u32 = c_extern::FADE_ALL_TO_WHITE;
/// `HUMAN_CONTROL = 1 << 0 = 1` (intel.h).
const HUMAN_CONTROL: u8 = 1;
/// `COMPUTER_CONTROL = (1<<1) | (1<<2) = 6` (intel.h).
const COMPUTER_CONTROL: u8 = 6;
/// `STANDARD_RATING = 1 << 4 = 16` (intel.h).
const STANDARD_RATING: u8 = 16;
/// `AWESOME_RATING = 1 << 6 = 64` (intel.h).
const AWESOME_RATING: u8 = 64;

// ===========================================================================
//  RestartMenu — restart.c:262-343
// ===========================================================================

/// Orchestrates the restart menu display, input loop, and cleanup.
///
/// Returns `true` if the player selected New Game or Load Game,
/// `false` if they quit, timed out, or the game ended.
///
/// Matches `RestartMenu()` in restart.c:262-343.
///
/// @plan PLAN-20260707-RESTARTMENU.P07
/// @requirement REQ-RM-007
pub fn restart_menu_impl<O: RestartMenuOps + ?Sized>(
    ops: &O,
    menu_state: &mut DoRestartState,
) -> bool {
    let mut time_out: u32;

    // Reinit race queues (restart.c:264-265)
    ops.reinit_race_queues();
    ops.set_screen_context();

    // Set CHECK_ABORT during menu (restart.c:267)
    let activity = ops.get_current_activity();
    ops.set_current_activity(activity | CHECK_ABORT);

    // Utwig bomb self-destruct check (restart.c:268-281)
    let crew = ops.get_crew_enlisted();
    let bomb_on_ship = ops.get_utwig_bomb_on_ship();
    let bomb_used = ops.get_utwig_bomb();

    if crew == 0xFFFF && bomb_on_ship != 0 && bomb_used == 0 {
        // Player blew himself up with Utwig bomb
        ops.set_utwig_bomb_on_ship(0);

        let fade_result = ops.fade_screen(FADE_ALL_TO_WHITE, (c_extern::ONE_SECOND / 8) as i16);
        ops.sleep_thread_until(fade_result.wrapping_add(c_extern::ONE_SECOND / 60));

        ops.set_bg_color(c_extern::Color::white_gray());
        ops.clear_drawable();
        ops.flush_color_xforms();

        time_out = c_extern::ONE_SECOND / 8;
    } else {
        time_out = c_extern::ONE_SECOND / 2;

        // Victory check (restart.c:288-296)
        let last_activity = ops.get_last_activity();
        if (last_activity & 0xFF) as u8 as u16 == WON_LAST_BATTLE {
            ops.set_current_activity(WON_LAST_BATTLE);
            ops.victory();
            ops.credits(true);
            ops.free_game_data();
            ops.set_current_activity(CHECK_ABORT);
        }
    }

    // Clear activity history (restart.c:298-299)
    ops.set_last_activity(0);
    ops.set_next_activity(0);

    // Fade to black (restart.c:302-304)
    let fade_result = ops.fade_screen(c_extern::FADE_ALL_TO_BLACK, time_out as i16);
    ops.sleep_thread_until(fade_result);

    // Extra wait for utwig bomb path (restart.c:305-306)
    if time_out == c_extern::ONE_SECOND / 8 {
        ops.sleep_thread(c_extern::ONE_SECOND * 3);
    }

    // Load and draw menu graphic (restart.c:308-309)
    let load_result = ops.load_menu_graphic();
    let cur_frame = ops.capture_drawable(load_result);
    // Sync CurFrame to C MENU_STATE — DrawRestartMenuGraphic reads pMS->CurFrame
    ops.sync_cur_frame(cur_frame);
    ops.draw_restart_menu_graphic();

    // Clear CHECK_ABORT and configure menu (restart.c:310-312)
    let activity = ops.get_current_activity();
    ops.set_current_activity(activity & !CHECK_ABORT);
    ops.set_menu_sounds(
        c_extern::MENU_SOUND_UP | c_extern::MENU_SOUND_DOWN,
        c_extern::MENU_SOUND_SELECT,
    );
    ops.set_default_menu_repeat_delay();

    // Reset menu for DoInput loop (restart.c:310-312)
    // Only reset initialized flag; cur_state is preserved across calls
    menu_state.initialized = false;

    // Run the input loop (restart.c:313)
    // DoInput calls our do_restart_frame callback via the InputFunc trampoline
    ops.run_do_input(true);

    // Cleanup (restart.c:315-322)
    ops.stop_music();
    if menu_state.music_handle != 0 {
        ops.destroy_music(menu_state.music_handle);
        menu_state.music_handle = 0;
    }
    if menu_state.flash_context != 0 {
        ops.flash_terminate(menu_state.flash_context);
        menu_state.flash_context = 0;
    }
    let drawable = ops.release_drawable(cur_frame);
    ops.destroy_drawable(drawable);

    // Check timeout (restart.c:324-325)
    let activity = ops.get_current_activity();
    if activity == !0u16 {
        return false;
    }

    // Check quit (restart.c:327-328)
    if activity & CHECK_ABORT != 0 {
        return false;
    }

    // Final fade and flush (restart.c:330-342)
    let fade_result = ops.fade_screen(c_extern::FADE_ALL_TO_BLACK, (c_extern::ONE_SECOND / 2) as i16);
    ops.sleep_thread_until(fade_result);
    ops.flush_color_xforms();
    ops.seed_random();

    // Return: did the player pick something other than Super Melee?
    (activity & 0xFF) != SUPER_MELEE
}

// ===========================================================================
//  TryStartGame — restart.c:355-387
// ===========================================================================

/// Attempts to start or load a game via the restart menu.
///
/// Loops `RestartMenu` until the player starts/loads a game, plays
/// Super Melee, quits, or times out.
///
/// Returns `true` if a game was started/loaded, `false` if quit or timed out.
///
/// Matches `TryStartGame()` in restart.c:355-387.
///
/// @plan PLAN-20260707-RESTARTMENU.P07
/// @requirement REQ-RM-007
pub fn try_start_game_impl<O: RestartMenuOps + ?Sized>(
    ops: &O,
) -> bool {
    // Create and initialize MENU_STATE (restart.c:357-359)
    let menu_ptr = ops.create_menu_state();
    let mut menu_state = DoRestartState::default();

    // Set MENU_STATE.InputFunc and privData (via C bridge)
    ops.set_menu_state_ptr(menu_ptr);
    ops.set_menu_priv_data(
        menu_ptr,
        (&mut menu_state as *mut DoRestartState) as usize,
    );

    // Save LastActivity, clear CurrentActivity (restart.c:360-361)
    let activity = ops.get_current_activity();
    ops.set_last_activity(activity);
    ops.set_current_activity(0);

    loop {
        let started_game = restart_menu_impl(ops, &mut menu_state);

        if started_game {
            ops.destroy_menu_state(menu_ptr);
            return true;
        }

        let activity = ops.get_current_activity();

        if (activity & 0xFF) == SUPER_MELEE && (activity & CHECK_ABORT) == 0 {
            // Super Melee selected (restart.c:365-369)
            ops.free_game_data();
            ops.melee();
            menu_state.initialized = false;
            // Loop back to show menu again
        } else if activity == !0u16 {
            // Timeout (restart.c:371-374)
            let fade_result = ops.fade_screen(c_extern::FADE_ALL_TO_BLACK, (c_extern::ONE_SECOND / 2) as i16);
            ops.sleep_thread_until(fade_result);
            ops.destroy_menu_state(menu_ptr);
            return false;
        } else if activity & CHECK_ABORT != 0 {
            // Quit (restart.c:376-377)
            ops.destroy_menu_state(menu_ptr);
            return false;
        }
        // Otherwise loop again
    }
}

// ===========================================================================
//  StartGame — restart.c:390-423
// ===========================================================================

/// Outermost game-start loop.
///
/// Returns `true` if a game was started, `false` if the player quit.
///
/// Matches `StartGame()` in restart.c:390-423.
///
/// @plan PLAN-20260707-RESTARTMENU.P07
/// @requirement REQ-RM-007
pub fn start_game_impl<O: RestartMenuOps + ?Sized>(
    ops: &O,
) -> bool {
    loop {
        // Inner loop: try to start a game (restart.c:392-405)
        loop {
            let started = try_start_game_impl(ops);

            if started {
                break;
            }

            let activity = ops.get_current_activity();

            if activity == !0u16 {
                // Timeout: show splash + credits (restart.c:398-401)
                ops.set_current_activity(0);
                ops.splash_screen();
                ops.credits(false);
            }

            if ops.get_current_activity() & CHECK_ABORT != 0 {
                return false;
            }
        }

        // Show introduction for new games (restart.c:407-409)
        let last_activity = ops.get_last_activity();
        if last_activity & CHECK_RESTART != 0 {
            ops.introduction();
        }

        // Loop if CHECK_ABORT was set during Introduction (restart.c:411)
        if ops.get_current_activity() & CHECK_ABORT == 0 {
            break;
        }
    }

    // Assign global arrays (restart.c:413-421)
    ops.assign_global_arrays();

    // Set player controls (restart.c:423-424)
    ops.set_player_control(0, HUMAN_CONTROL as u16 | STANDARD_RATING as u16);
    ops.set_player_control(1, COMPUTER_CONTROL as u16 | AWESOME_RATING as u16);

    true
}

// ===========================================================================
//  C entry point (non-test only)
// ===========================================================================

/// C-callable entry point for `StartGame` when `USE_RUST_RESTART` is defined.
///
/// @plan PLAN-20260707-RESTARTMENU.P07
/// @requirement REQ-RM-007
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn rust_start_game() -> c_int {
    let ops = super::restart_ops::CffiOps::new();
    if start_game_impl(&ops) {
        1
    } else {
        0
    }
}
// ===========================================================================
//  Tests — Tier 1 (pure Rust, MockOps, no C linkage)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::restart_ops::RestartMenuOps;
    use super::super::c_extern::Color;
    use std::cell::Cell;
    use std::os::raw::{c_int, c_short};

    // Re-use the MockOps pattern from do_restart tests
    struct MockOps {
        current_activity: Cell<u16>,
        last_activity: Cell<u16>,
        next_activity: Cell<u16>,
        crew_enlisted: Cell<u16>,
        utwig_bomb_on_ship: Cell<u8>,
        utwig_bomb: Cell<u8>,
        time_counter: Cell<u32>,
        do_input_activity_after: Cell<u16>,
        // Tracking
        victory_called: Cell<bool>,
        credits_called: Cell<bool>,
        credits_victory: Cell<bool>,
        free_game_data_called: Cell<bool>,
        melee_called: Cell<bool>,
        introduction_called: Cell<bool>,
        splash_called: Cell<bool>,
        assign_globals_called: Cell<bool>,
        player_control_0: Cell<u16>,
        player_control_1: Cell<u16>,
        fade_screen_type: Cell<u32>,
        first_fade_type: Cell<u32>,
        first_fade_recorded: Cell<bool>,
        set_bg_color_called: Cell<bool>,
        clear_drawable_called: Cell<bool>,
        seed_random_called: Cell<bool>,
    }

    impl MockOps {
        fn new() -> Self {
            Self {
                current_activity: Cell::new(0),
                last_activity: Cell::new(0),
                next_activity: Cell::new(0),
                crew_enlisted: Cell::new(0),
                utwig_bomb_on_ship: Cell::new(0),
                utwig_bomb: Cell::new(0),
                time_counter: Cell::new(100),
                do_input_activity_after: Cell::new(0),
                victory_called: Cell::new(false),
                credits_called: Cell::new(false),
                credits_victory: Cell::new(false),
                free_game_data_called: Cell::new(false),
                melee_called: Cell::new(false),
                introduction_called: Cell::new(false),
                splash_called: Cell::new(false),
                assign_globals_called: Cell::new(false),
                player_control_0: Cell::new(0),
                player_control_1: Cell::new(0),
                fade_screen_type: Cell::new(0),
                first_fade_type: Cell::new(0),
                first_fade_recorded: Cell::new(false),
                set_bg_color_called: Cell::new(false),
                clear_drawable_called: Cell::new(false),
                seed_random_called: Cell::new(false),
            }
        }
    }

    impl RestartMenuOps for MockOps {
        fn get_current_activity(&self) -> u16 { self.current_activity.get() }
        fn set_current_activity(&self, v: u16) { self.current_activity.set(v); }
        fn get_last_activity(&self) -> u16 { self.last_activity.get() }
        fn set_last_activity(&self, v: u16) { self.last_activity.set(v); }
        fn set_next_activity(&self, v: u16) { self.next_activity.set(v); }
        fn set_menu_state_ptr(&self, _ptr: usize) {}
        fn get_menu_state_ptr(&self) -> usize { 0 }
        fn get_menu_frame(&self) -> usize { 0 }
        fn create_menu_state(&self) -> usize { 0 }
        fn set_menu_priv_data(&self, _ptr: usize, _data: usize) {}
        fn destroy_menu_state(&self, _ptr: usize) {}
        fn sync_flash_context(&self, _ctx: usize) {}
        fn sync_initialized(&self, _val: bool) {}
        fn sync_cur_state(&self, _state: u8) {}
        fn sync_cur_frame(&self, _frame: usize) {}
        fn sync_h_music(&self, _handle: usize) {}
        fn get_menu_input(&self) -> super::super::types::MenuInputState { Default::default() }
        fn get_time_counter(&self) -> u32 { self.time_counter.get() }
        fn get_utwig_bomb_on_ship(&self) -> u8 { self.utwig_bomb_on_ship.get() }
        fn set_utwig_bomb_on_ship(&self, v: u8) { self.utwig_bomb_on_ship.set(v); }
        fn get_utwig_bomb(&self) -> u8 { self.utwig_bomb.get() }
        fn get_crew_enlisted(&self) -> u16 { self.crew_enlisted.get() }
        fn set_game_paused(&self, _val: bool) {}
        fn reinit_race_queues(&self) {}
        fn set_screen_context(&self) {}
        fn fade_screen(&self, fade_type: u32, _duration: c_short) -> u32 {
            if !self.first_fade_recorded.get() {
                self.first_fade_type.set(fade_type);
                self.first_fade_recorded.set(true);
            }
            self.fade_screen_type.set(fade_type);
            200
        }
        fn sleep_thread_until(&self, _time: u32) {}
        fn sleep_thread(&self, _duration: u32) {}
        fn batch_graphics(&self) {}
        fn unbatch_graphics(&self) {}
        fn clear_drawable(&self) { self.clear_drawable_called.set(true); }
        fn flush_color_xforms(&self) {}
        fn screen_transition(&self, _a: c_int) {}
        fn set_bg_color(&self, _color: Color) { self.set_bg_color_called.set(true); }
        fn seed_random(&self) { self.seed_random_called.set(true); }
        fn load_menu_graphic(&self) -> usize { 1 }
        fn capture_drawable(&self, _load_result: usize) -> usize { 1 }
        fn destroy_drawable(&self, _handle: usize) {}
        fn release_drawable(&self, _handle: usize) -> usize { 0 }
        fn draw_restart_menu_graphic(&self) {}
        fn draw_restart_menu_state(&self, _state: u8) {}
        fn set_menu_sounds(&self, _s0: u16, _s1: u16) {}
        fn set_default_menu_repeat_delay(&self) {}
        fn set_transition_source_null(&self) {}
        fn run_do_input(&self, _reset_input: bool) {
            // Simulate DoInput setting activity via user selection.
            // For new game (IN_INTERPLANETARY), DoRestart also sets
            // LastActivity = CHECK_LOAD | CHECK_RESTART.
            let new_activity = self.do_input_activity_after.get();
            self.current_activity.set(new_activity);
            if new_activity == 4 { // IN_INTERPLANETARY
                self.last_activity.set(CHECK_LOAD | CHECK_RESTART);
            }
        }
        fn load_menu_music(&self) -> usize { 0 }
        fn play_music(&self, _handle: usize) {}
        fn stop_music(&self) {}
        fn destroy_music(&self, _handle: usize) {}
        fn fade_music(&self, _end_vol: u8, _time_interval: c_short) -> u32 { 200 }
        fn create_flash_overlay(&self) -> usize { 0 }
        fn flash_process(&self, _ctx: usize) {}
        fn flash_pause(&self, _ctx: usize) {}
        fn flash_continue(&self, _ctx: usize) {}
        fn flash_start(&self, _ctx: usize) {}
        fn flash_terminate(&self, _ctx: usize) {}
        fn flash_set_merge_factors(&self, _ctx: usize, _a: c_int, _b: c_int, _c: c_int) {}
        fn flash_set_speed(&self, _ctx: usize, _a: u32, _b: u32, _c: u32, _d: u32) {}
        fn flash_set_frame_time(&self, _ctx: usize, _t: u32) {}
        fn flash_set_state_fade_in(&self, _ctx: usize, _duration: u32) {}
        fn flash_set_overlay(&self, _ctx: usize, _frame_idx: u32) {}
        fn melee(&self) { self.melee_called.set(true); }
        fn setup_menu(&self) {}
        fn free_game_data(&self) { self.free_game_data_called.set(true); }
        fn introduction(&self) { self.introduction_called.set(true); }
        fn credits(&self, victory: bool) { self.credits_called.set(true); self.credits_victory.set(victory); }
        fn victory(&self) { self.victory_called.set(true); }
        fn splash_screen(&self) {
            self.splash_called.set(true);
            // After splash, simulate user quitting on next menu
            self.do_input_activity_after.set(CHECK_ABORT);
        }
        fn do_popup_window_msg(&self, _string_id: u16) {}
        fn set_player_control(&self, player: u8, control: u16) {
            if player == 0 { self.player_control_0.set(control); }
            else { self.player_control_1.set(control); }
        }
        fn assign_global_arrays(&self) { self.assign_globals_called.set(true); }
        fn set_main_exited(&self, _val: bool) {}
    }

    // ---- restart_menu_impl tests -----------------------------------------

    #[test]
    fn restart_menu_new_game_returns_true() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(IN_INTERPLANETARY_ACTIVITY());

        let mut state = DoRestartState::default();
        let result = restart_menu_impl(&ops, &mut state);

        assert!(result, "non-melee selection should return true");
        assert!(ops.seed_random_called.get());
    }

    #[test]
    fn restart_menu_super_melee_returns_false() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(SUPER_MELEE);

        let mut state = DoRestartState::default();
        let result = restart_menu_impl(&ops, &mut state);

        assert!(!result, "Super Melee should return false from RestartMenu");
    }

    #[test]
    fn restart_menu_quit_returns_false() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(CHECK_ABORT);

        let mut state = DoRestartState::default();
        let result = restart_menu_impl(&ops, &mut state);

        assert!(!result, "quit should return false");
    }

    #[test]
    fn restart_menu_timeout_returns_false() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(!0u16);

        let mut state = DoRestartState::default();
        let result = restart_menu_impl(&ops, &mut state);

        assert!(!result, "timeout should return false");
    }

    #[test]
    fn restart_menu_utwig_bomb_path_sets_bg_color() {
        let ops = MockOps::new();
        ops.crew_enlisted.set(0xFFFF);
        ops.utwig_bomb_on_ship.set(1);
        ops.utwig_bomb.set(0);
        ops.do_input_activity_after.set(4); // IN_INTERPLANETARY

        let mut state = DoRestartState::default();
        let _ = restart_menu_impl(&ops, &mut state);

        assert_eq!(ops.first_fade_type.get(), FADE_ALL_TO_WHITE);
        assert!(ops.set_bg_color_called.get(), "bg color should be set for utwig bomb path");
        assert!(ops.clear_drawable_called.get(), "drawable should be cleared");
        assert_eq!(ops.utwig_bomb_on_ship.get(), 0, "bomb on ship should be cleared");
    }

    #[test]
    fn restart_menu_victory_path_calls_victory_and_credits() {
        let ops = MockOps::new();
        ops.last_activity.set(WON_LAST_BATTLE);
        ops.do_input_activity_after.set(4); // IN_INTERPLANETARY

        let mut state = DoRestartState::default();
        let _ = restart_menu_impl(&ops, &mut state);

        assert!(ops.victory_called.get(), "Victory should be called");
        assert!(ops.credits_called.get(), "Credits should be called");
        assert!(ops.credits_victory.get(), "Credits should be called with victory=true");
        assert!(ops.free_game_data_called.get(), "FreeGameData should be called");
    }

    // ---- try_start_game_impl tests ---------------------------------------

    #[test]
    fn try_start_game_succeeds() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(4); // IN_INTERPLANETARY → started

        let result = try_start_game_impl(&ops);
        assert!(result, "should return true when game starts");
    }

    #[test]
    fn try_start_game_quit_returns_false() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(CHECK_ABORT);

        let result = try_start_game_impl(&ops);
        assert!(!result, "quit should return false");
    }

    #[test]
    fn try_start_game_timeout_returns_false() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(!0u16);

        let result = try_start_game_impl(&ops);
        assert!(!result, "timeout should return false");
    }

    // ---- start_game_impl tests -------------------------------------------

    #[test]
    fn start_game_new_game_calls_introduction() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(4); // IN_INTERPLANETARY
        // Set last_activity to include CHECK_RESTART
        ops.last_activity.set(CHECK_RESTART);

        let result = start_game_impl(&ops);
        assert!(result, "should return true");
        assert!(ops.introduction_called.get(), "Introduction should be called for new game");
    }

    #[test]
    fn start_game_sets_player_controls() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(4); // IN_INTERPLANETARY
        ops.last_activity.set(0); // No CHECK_RESTART → loaded game

        let result = start_game_impl(&ops);
        assert!(result, "should return true");
        assert_eq!(ops.player_control_0.get(), (HUMAN_CONTROL as u16) | (STANDARD_RATING as u16));
        assert_eq!(ops.player_control_1.get(), (COMPUTER_CONTROL as u16) | (AWESOME_RATING as u16));
        assert!(ops.assign_globals_called.get());
    }

    #[test]
    fn start_game_quit_returns_false() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(CHECK_ABORT);

        let result = start_game_impl(&ops);
        assert!(!result, "quit should return false");
    }

    #[test]
    fn start_game_timeout_shows_splash_and_credits() {
        let ops = MockOps::new();
        ops.do_input_activity_after.set(!0u16); // Timeout

        let result = start_game_impl(&ops);
        assert!(!result, "timeout should eventually return false");
        assert!(ops.splash_called.get(), "splash should be called on timeout");
        assert!(ops.credits_called.get(), "credits(false) should be called");
    }

    /// Helper: IN_INTERPLANETARY activity value (4) as used in restart.c.
    const fn IN_INTERPLANETARY_ACTIVITY() -> u16 { 4 }
}
