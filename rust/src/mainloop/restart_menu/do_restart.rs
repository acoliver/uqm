//! DoRestart per-frame callback — replaces `DoRestart()` in restart.c.
//!
//! @plan PLAN-20260707-RESTARTMENU.P06
//! @requirement REQ-RM-006

use super::menu_logic::{self, check_timeout};
use super::restart_ops::RestartMenuOps;
use super::types::{MenuInputState, RestartMenuItem, SelectionResult};
use crate::{
    engine::input::frames::{MENU_SOUND_DOWN, MENU_SOUND_SELECT, MENU_SOUND_UP},
    graphics::cmap_abi::{FADE_ALL_TO_BLACK, FADE_ALL_TO_COLOR},
    sound::types::NORMAL_VOLUME,
};
use uqm_engine::time::counter_abi::ONE_SECOND;

// ===========================================================================
//  DoRestartState — replaces C static locals
// ===========================================================================

/// Persistent state for `do_restart_frame`, replacing the C `static`
/// locals and MENU_STATE fields accessed by DoRestart.
///
/// @plan PLAN-20260707-RESTARTMENU.P06
/// @requirement REQ-RM-006
#[derive(Debug, Clone)]
pub struct DoRestartState {
    /// Whether the menu has been initialized (first frame completed).
    pub initialized: bool,
    pub scene_replaces_introduction: bool,
    /// Last time input was received (for inactivity timeout).
    pub last_input_time: u32,
    /// Inactivity timeout duration in ticks.
    pub inact_timeout: u32,
    /// Current menu item index (C `pMS->CurState`).
    pub cur_state: u8,
}

impl Default for DoRestartState {
    fn default() -> Self {
        Self {
            initialized: false,
            scene_replaces_introduction: false,
            last_input_time: 0,
            inact_timeout: 0,
            cur_state: RestartMenuItem::NewGame.as_u8(),
        }
    }
}

// ===========================================================================
//  Activity constants (from state.h / setup.h)
// ===========================================================================

/// `CHECK_ABORT = 0x4000` (setup.h activity flags).
const CHECK_ABORT: u16 = 0x4000;
/// `CHECK_LOAD = 0x1000` (setup.h activity flags).
const CHECK_LOAD: u16 = 0x1000;
/// `CHECK_RESTART = 0x2000` (setup.h activity flags).
const CHECK_RESTART: u16 = 0x2000;
/// `IN_INTERPLANETARY = 4` (globdata.h activity enum).
const IN_INTERPLANETARY: u16 = 4;
/// `SUPER_MELEE = 0` (setup.h activity kind).
const SUPER_MELEE: u16 = 0;

// ===========================================================================
//  DoRestart frame logic
// ===========================================================================

/// Process one frame of the restart menu.
///
/// Returns `true` to continue the menu loop, `false` to exit.
///
/// Matches `DoRestart()` in restart.c:107-258.
///
/// @plan PLAN-20260707-RESTARTMENU.P06
/// @requirement REQ-RM-006
pub fn do_restart_frame<O: RestartMenuOps + ?Sized>(
    ops: &mut O,
    state: &mut DoRestartState,
) -> bool {
    let time_in = ops.get_time_counter();

    // Cancel any Pause key presses (restart.c:113)
    ops.set_game_paused(false);

    if state.initialized {
        // Process flash overlay on subsequent frames (restart.c:116)
        ops.flash_process();
    }

    if !state.initialized {
        // --- First-frame initialization (restart.c:117-137) ---
        init_first_frame(ops, state);
    } else {
        // Check CHECK_ABORT (restart.c:139-142)
        let activity = ops.get_current_activity();
        if activity & CHECK_ABORT != 0 {
            return false;
        }

        let input = ops.get_menu_input();

        if input.select {
            // Selection dispatch (restart.c:144-177)
            if !handle_select(ops, state) {
                return false;
            }
        } else if input.up || input.down {
            // Navigation (restart.c:179-214)
            handle_navigate(ops, state, input);
        } else if input.left || input.right {
            // No-op but counts as input (restart.c:216-218)
            state.last_input_time = ops.get_time_counter();
        } else if input.mouse_down {
            // Mouse not supported popup (restart.c:220-234)
            handle_mouse_popup(ops, state);
        } else {
            // Timeout check (restart.c:236-244)
            let now = ops.get_time_counter();
            if check_timeout(now, state.last_input_time, state.inact_timeout) {
                handle_timeout(ops);
                return false;
            }
        }
    }

    // Frame rate limiting (restart.c:255)
    ops.sleep_thread_until(time_in.wrapping_add(ONE_SECOND / 30));

    true
}

/// First-frame initialization — loads music, creates flash overlay,
/// draws the initial menu, and fades in.
///
/// Matches restart.c:117-137.
fn init_first_frame<O: RestartMenuOps + ?Sized>(ops: &mut O, state: &mut DoRestartState) {
    // Clean up existing music (restart.c:119-124)
    if ops.has_menu_music() {
        ops.stop_music();
        ops.destroy_music();
    }

    // Load main menu music (restart.c:125)
    let music_loaded = ops.load_menu_music();
    state.inact_timeout = if music_loaded { 120 } else { 20 } * ONE_SECOND;

    // Create and configure flash overlay (restart.c:126-132)
    ops.create_flash_overlay();
    ops.flash_set_merge_factors(-3, 3, 16);
    ops.flash_set_speed((6 * ONE_SECOND) / 16, 0, (6 * ONE_SECOND) / 16, 0);
    ops.flash_set_frame_time(ONE_SECOND / 16);
    ops.flash_set_state_fade_in((3 * ONE_SECOND) / 16);

    // Draw initial menu state (restart.c:133-134)
    ops.draw_restart_menu_graphic();
    ops.draw_restart_menu_state(state.cur_state);

    // Start flash and music (restart.c:135-137)
    ops.flash_start();
    if music_loaded {
        ops.play_music();
    }

    state.last_input_time = ops.get_time_counter();
    state.initialized = true;

    // Fade in (restart.c:138)
    let fade_result = ops.fade_screen(FADE_ALL_TO_COLOR, (ONE_SECOND / 2) as i16);
    ops.sleep_thread_until(fade_result);
    state.scene_replaces_introduction = ops
        .input(crate::engine::observe::InputView::ReplacesIntroduction)
        .stop;
    let _stop = ops
        .input(crate::engine::observe::InputView::MainMenuReady)
        .stop;
}

/// Handle menu selection dispatch.
///
/// Returns `true` if the menu should continue (SETUP_GAME case),
/// `false` if it should exit.
///
/// Matches restart.c:144-188.
fn handle_select<O: RestartMenuOps + ?Sized>(ops: &mut O, state: &mut DoRestartState) -> bool {
    let cur_item = RestartMenuItem::from_u8(state.cur_state).unwrap_or(RestartMenuItem::NewGame);

    let result = menu_logic::apply_selection(cur_item);

    match result {
        SelectionResult::StartGame { new_game } => {
            if new_game {
                // START_NEW_GAME: LastActivity = CHECK_LOAD | CHECK_RESTART
                ops.set_last_activity(CHECK_LOAD | CHECK_RESTART);
            } else {
                // LOAD_SAVED_GAME: LastActivity = CHECK_LOAD
                ops.set_last_activity(CHECK_LOAD);
            }
            ops.set_current_activity(IN_INTERPLANETARY);
            // Flash_pause then return FALSE (restart.c:186-188)
            ops.flash_pause();
            false
        }
        SelectionResult::SuperMelee => {
            ops.set_current_activity(SUPER_MELEE);
            ops.flash_pause();
            false
        }
        SelectionResult::StayInMenu => {
            // SETUP_GAME path (restart.c:156-178)
            ops.flash_pause();
            ops.flash_set_state_fade_in((3 * ONE_SECOND) / 16);
            ops.setup_menu();
            ops.set_menu_sounds(MENU_SOUND_UP | MENU_SOUND_DOWN, MENU_SOUND_SELECT);
            state.last_input_time = ops.get_time_counter();
            ops.set_transition_source_null();
            ops.batch_graphics();
            ops.draw_restart_menu_graphic();
            ops.screen_transition(3);
            ops.draw_restart_menu_state(state.cur_state);
            ops.flash_continue();
            ops.unbatch_graphics();
            true
        }
        SelectionResult::Quit => {
            // QUIT_GAME (restart.c:172-175)
            let fade_result = ops.fade_screen(FADE_ALL_TO_BLACK, (ONE_SECOND / 2) as i16);
            ops.sleep_thread_until(fade_result);
            ops.set_current_activity(CHECK_ABORT);
            if ops.input(crate::engine::observe::InputView::Active).stop {
                ops.input(crate::engine::observe::InputView::QuitSelected);
            }
            ops.flash_pause();
            false
        }
    }
}

/// Handle up/down navigation.
///
/// Matches restart.c:179-214.
fn handle_navigate<O: RestartMenuOps + ?Sized>(
    ops: &mut O,
    state: &mut DoRestartState,
    input: MenuInputState,
) {
    let cur_item = RestartMenuItem::from_u8(state.cur_state).unwrap_or(RestartMenuItem::NewGame);

    let new_item = if input.up {
        menu_logic::navigate_up(cur_item)
    } else {
        menu_logic::navigate_down(cur_item)
    };

    if new_item.as_u8() != state.cur_state {
        // Redraw with new selection (restart.c:206-212)
        ops.batch_graphics();
        ops.draw_restart_menu_state(new_item.as_u8());
        ops.unbatch_graphics();
        // @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
        // @requirement REQ-SEM-001
        // The selection changes after drawing and before the transition observation.
        state.cur_state = new_item.as_u8();

        // Typed observer: returns Continue or Stop.

        // Feed the menu transition to the automation coordinator if active.
        // This drives the scheduler's WaitingSemantic state and semantic
        // assertion matching.
        if ops.input(crate::engine::observe::InputView::Active).stop {
            let _stop = ops
                .input(crate::engine::observe::InputView::MenuTransition {
                    from: cur_item.as_u8(),
                    to: new_item.as_u8(),
                })
                .stop;
        }
        // In full integration, Stop would propagate through do_restart_frame
        // and rust_do_restart_frame before sleep/later work.
    }

    state.last_input_time = ops.get_time_counter();
}

/// Handle mouse click popup ("mouse not supported").
///
/// Matches restart.c:220-234.
fn handle_mouse_popup<O: RestartMenuOps + ?Sized>(ops: &mut O, state: &mut DoRestartState) {
    ops.flash_pause();

    // Offset 54 from MAINMENU_STRING_BASE = "mouse not supported" message.
    // The C wrapper adds MAINMENU_STRING_BASE internally.
    const MOUSE_NOT_SUPPORTED_STRING: u16 = 54;

    ops.do_popup_window_msg(MOUSE_NOT_SUPPORTED_STRING);
    ops.set_menu_sounds(MENU_SOUND_UP | MENU_SOUND_DOWN, MENU_SOUND_SELECT);
    ops.set_transition_source_null();
    ops.batch_graphics();
    ops.draw_restart_menu_graphic();
    ops.draw_restart_menu_state(state.cur_state);
    ops.screen_transition(3);
    ops.unbatch_graphics();

    ops.flash_continue();

    state.last_input_time = ops.get_time_counter();
}

/// Handle inactivity timeout.
///
/// Matches restart.c:236-244.
fn handle_timeout<O: RestartMenuOps + ?Sized>(ops: &mut O) {
    // Fade music out, stop, restore volume (restart.c:237-239)
    let fade_result = ops.fade_music(0, ONE_SECOND as i16);
    ops.sleep_thread_until(fade_result);
    ops.stop_music();
    ops.fade_music(NORMAL_VOLUME as u8, 0);

    // Set activity to ~0 (all bits set) (restart.c:243)
    ops.set_current_activity(!0u16);
}

// ===========================================================================
//  Tests — Tier 1 (pure Rust, MockOps, no C linkage)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::super::restart_ops::RestartMenuOps;
    use super::*;
    use crate::graphics::native_abi::Color;
    use std::cell::Cell;
    use std::os::raw::{c_int, c_short};

    /// Mock implementation of RestartMenuOps for unit testing.
    /// Uses interior mutability via Cell to track state.
    struct MockOps {
        current_activity: Cell<u16>,
        music_loaded: Cell<bool>,
        last_activity: Cell<u16>,
        time_counter: Cell<u32>,
        game_paused: Cell<bool>,
        menu_input: Cell<MenuInputState>,
        // Tracking
        fade_screen_called: Cell<bool>,
        fade_screen_type: Cell<u32>,
        fade_result: Cell<u32>,
        sleep_until_called: Cell<bool>,
        play_music_called: Cell<bool>,
        stop_music_called: Cell<bool>,
        set_activity_called: Cell<bool>,
        nav_count: Cell<u32>,
        draw_menu_called: Cell<bool>,
        popup_called: Cell<bool>,
        flash_process_called: Cell<bool>,
        flash_created: Cell<bool>,
        flash_started: Cell<bool>,
        flash_pause_called: Cell<bool>,
        flash_continue_called: Cell<bool>,
        setup_menu_called: Cell<bool>,
        destroy_music_called: Cell<bool>,
    }

    impl MockOps {
        fn new() -> Self {
            Self {
                current_activity: Cell::new(0),
                music_loaded: Cell::new(false),
                last_activity: Cell::new(0),
                time_counter: Cell::new(100),
                game_paused: Cell::new(false),
                menu_input: Cell::new(MenuInputState::default()),
                fade_screen_called: Cell::new(false),
                fade_screen_type: Cell::new(0),
                fade_result: Cell::new(200),
                sleep_until_called: Cell::new(false),
                play_music_called: Cell::new(false),
                stop_music_called: Cell::new(false),
                set_activity_called: Cell::new(false),
                nav_count: Cell::new(0),
                draw_menu_called: Cell::new(false),
                popup_called: Cell::new(false),
                flash_process_called: Cell::new(false),
                flash_created: Cell::new(false),
                flash_started: Cell::new(false),
                flash_pause_called: Cell::new(false),
                flash_continue_called: Cell::new(false),
                setup_menu_called: Cell::new(false),
                destroy_music_called: Cell::new(false),
            }
        }
    }

    impl RestartMenuOps for MockOps {
        fn get_current_activity(&mut self) -> u16 {
            self.current_activity.get()
        }
        fn set_current_activity(&mut self, v: u16) {
            self.current_activity.set(v);
            self.set_activity_called.set(true);
        }
        fn get_last_activity(&mut self) -> u16 {
            self.last_activity.get()
        }
        fn set_last_activity(&mut self, v: u16) {
            self.last_activity.set(v);
        }
        fn set_next_activity(&mut self, _v: u16) {}
        fn get_menu_input(&mut self) -> MenuInputState {
            self.menu_input.get()
        }
        fn get_time_counter(&mut self) -> u32 {
            self.time_counter.get()
        }
        fn get_utwig_bomb_on_ship(&mut self) -> u8 {
            0
        }
        fn set_utwig_bomb_on_ship(&mut self, _v: u8) {}
        fn get_utwig_bomb(&mut self) -> u8 {
            0
        }
        fn get_crew_enlisted(&mut self) -> u16 {
            0
        }
        fn set_game_paused(&mut self, val: bool) {
            self.game_paused.set(val);
        }
        fn reinit_race_queues(&mut self) {}
        fn set_screen_context(&mut self) {}
        fn fade_screen(&mut self, fade_type: u32, _duration: c_short) -> u32 {
            self.fade_screen_called.set(true);
            self.fade_screen_type.set(fade_type);
            self.fade_result.get()
        }
        fn sleep_thread_until(&mut self, _time: u32) {
            self.sleep_until_called.set(true);
        }
        fn sleep_thread(&mut self, _duration: u32) {}
        fn batch_graphics(&mut self) {}
        fn unbatch_graphics(&mut self) {
            self.nav_count.set(self.nav_count.get() + 1);
        }
        fn clear_drawable(&mut self) {}
        fn flush_color_xforms(&mut self) {}
        fn screen_transition(&mut self, _a: c_int) {}
        fn set_bg_color(&mut self, _color: Color) {}
        fn seed_random(&mut self) {}
        fn load_menu_graphic(&mut self) {}
        fn destroy_menu_graphic(&mut self) {}
        fn draw_restart_menu_graphic(&mut self) {
            self.draw_menu_called.set(true);
        }
        fn draw_restart_menu_state(&mut self, _state: u8) {
            self.draw_menu_called.set(true);
        }
        fn set_menu_sounds(&mut self, _s0: u16, _s1: u16) {}
        fn set_default_menu_repeat_delay(&mut self) {}
        fn set_transition_source_null(&mut self) {}
        fn run_do_input(&mut self, _reset_input: bool, _state: &mut DoRestartState) {}
        fn has_menu_music(&mut self) -> bool {
            self.music_loaded.get()
        }
        fn load_menu_music(&mut self) -> bool {
            self.music_loaded.set(true);
            true
        }
        fn play_music(&mut self) {
            self.play_music_called.set(true);
        }
        fn stop_music(&mut self) {
            self.stop_music_called.set(true);
        }
        fn destroy_music(&mut self) {
            self.music_loaded.set(false);
            self.destroy_music_called.set(true);
        }
        fn fade_music(&mut self, _end_vol: u8, _time_interval: c_short) -> u32 {
            200
        }
        fn create_flash_overlay(&mut self) {
            self.flash_created.set(true);
        }
        fn flash_process(&mut self) {
            self.flash_process_called.set(true);
        }
        fn flash_pause(&mut self) {
            self.flash_pause_called.set(true);
        }
        fn flash_continue(&mut self) {
            self.flash_continue_called.set(true);
        }
        fn flash_start(&mut self) {
            self.flash_started.set(true);
        }
        fn flash_terminate(&mut self) {}
        fn flash_set_merge_factors(&mut self, _a: c_int, _b: c_int, _c: c_int) {}
        fn flash_set_speed(&mut self, _a: u32, _b: u32, _c: u32, _d: u32) {}
        fn flash_set_frame_time(&mut self, _t: u32) {}
        fn flash_set_state_fade_in(&mut self, _duration: u32) {}
        fn melee(&mut self) {}
        fn setup_menu(&mut self) {
            self.setup_menu_called.set(true);
        }
        fn free_game_data(&mut self) {}
        fn credits(&mut self, _victory: bool) {}
        fn victory(&mut self) {}
        fn introduction(&mut self) {}
        fn splash_screen(&mut self) {}
        fn do_popup_window_msg(&mut self, _string_id: u16) {
            self.popup_called.set(true);
        }
        fn set_player_control(&mut self, _player: u8, _control: u16) {}
    }

    // ---- Tests ------------------------------------------------------------

    fn initialized_state() -> DoRestartState {
        DoRestartState {
            initialized: true,
            ..Default::default()
        }
    }

    #[test]
    fn first_frame_initializes_and_returns_true() {
        let mut ops = MockOps::new();
        let mut state = DoRestartState::default();
        let result = do_restart_frame(&mut ops, &mut state);

        assert!(result, "first frame should return true (continue)");
        assert!(state.initialized, "state should be initialized");
        assert!(ops.play_music_called.get(), "music should play");
    }

    #[test]
    fn abort_returns_false() {
        let mut ops = MockOps::new();
        ops.current_activity.set(CHECK_ABORT);

        let mut state = initialized_state();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(!result, "CHECK_ABORT should exit menu");
    }
    #[test]
    fn explore_down_then_select_requests_load_without_entering_hyperspace() {
        let mut ops = MockOps::new();
        let mut state = DoRestartState::default();
        assert_eq!(state.cur_state, RestartMenuItem::NewGame.as_u8());
        assert!(do_restart_frame(&mut ops, &mut state));
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });
        assert!(do_restart_frame(&mut ops, &mut state));
        assert_eq!(state.cur_state, RestartMenuItem::LoadGame.as_u8());
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        assert!(!do_restart_frame(&mut ops, &mut state));
        assert_eq!(ops.last_activity.get(), CHECK_LOAD);
        assert_eq!(ops.current_activity.get(), IN_INTERPLANETARY);
        assert_ne!(ops.last_activity.get() & CHECK_RESTART, CHECK_RESTART);
    }

    #[test]
    fn select_new_game_sets_activity_and_exits() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.last_activity.get(), CHECK_LOAD | CHECK_RESTART);
        assert_eq!(ops.current_activity.get(), IN_INTERPLANETARY);
    }

    #[test]
    fn select_load_game_sets_activity_and_exits() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::LoadGame.as_u8();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.last_activity.get(), CHECK_LOAD);
        assert_eq!(ops.current_activity.get(), IN_INTERPLANETARY);
    }

    #[test]
    fn select_super_melee_sets_activity_zero() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::SuperMelee.as_u8();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.current_activity.get(), SUPER_MELEE);
    }

    #[test]
    fn select_quit_sets_check_abort() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Quit.as_u8();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.current_activity.get(), CHECK_ABORT);
        assert_eq!(ops.fade_screen_type.get(), FADE_ALL_TO_BLACK);
    }

    #[test]
    fn select_setup_stays_in_menu() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Setup.as_u8();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(result, "SETUP_GAME should stay in menu");
    }

    #[test]
    fn navigate_down_from_new_game_changes_state() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert_eq!(
            state.cur_state,
            RestartMenuItem::LoadGame.as_u8(),
            "down from NewGame should go to LoadGame"
        );
    }

    #[test]
    fn navigate_up_from_load_game_changes_state() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            up: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::LoadGame.as_u8();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert_eq!(
            state.cur_state,
            RestartMenuItem::NewGame.as_u8(),
            "up from LoadGame should go to NewGame"
        );
    }

    #[test]
    fn navigate_updates_last_input_time() {
        let mut ops = MockOps::new();
        ops.time_counter.set(500);
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.last_input_time = 0;

        let _ = do_restart_frame(&mut ops, &mut state);
        assert_eq!(state.last_input_time, 500);
    }

    #[test]
    fn left_right_updates_last_input_time() {
        let mut ops = MockOps::new();
        ops.time_counter.set(42);
        ops.menu_input.set(MenuInputState {
            left: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.last_input_time = 0;

        let _ = do_restart_frame(&mut ops, &mut state);
        assert_eq!(state.last_input_time, 42);
    }

    #[test]
    fn mouse_click_triggers_popup() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            mouse_down: true,
            ..Default::default()
        });

        let mut state = initialized_state();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(result, "mouse popup should continue menu");
        assert!(ops.popup_called.get(), "popup window should be shown");
    }

    #[test]
    fn timeout_exits_menu() {
        let mut ops = MockOps::new();
        ops.time_counter.set(1000);
        // No input
        ops.menu_input.set(MenuInputState::default());

        let mut state = initialized_state();
        state.last_input_time = 0;
        state.inact_timeout = 100;

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(!result, "timeout should exit menu");
        assert!(ops.stop_music_called.get(), "music should stop");
        assert_eq!(ops.current_activity.get(), !0u16, "activity should be ~0");
    }

    #[test]
    fn no_timeout_when_within_window() {
        let mut ops = MockOps::new();
        ops.time_counter.set(50);
        ops.menu_input.set(MenuInputState::default());

        let mut state = initialized_state();
        state.last_input_time = 0;
        state.inact_timeout = 100;

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(result, "should continue when within timeout window");
    }

    #[test]
    fn game_paused_cleared_each_frame() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState::default());

        let mut state = initialized_state();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(!ops.game_paused.get(), "game paused should be cleared");
    }

    #[test]
    fn sleep_thread_called_for_frame_limit() {
        let mut ops = MockOps::new();
        ops.time_counter.set(100);

        let mut state = initialized_state();
        ops.menu_input.set(MenuInputState::default());
        state.inact_timeout = 10000;

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(ops.sleep_until_called.get(), "frame rate limit should fire");
    }

    #[test]
    fn flash_process_called_on_initialized_frame() {
        let mut ops = MockOps::new();
        let mut state = initialized_state();
        state.inact_timeout = 10000;
        ops.menu_input.set(MenuInputState::default());

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(
            ops.flash_process_called.get(),
            "flash_process should be called on initialized frame"
        );
    }

    #[test]
    fn select_quit_calls_flash_pause() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Quit.as_u8();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(
            ops.flash_pause_called.get(),
            "Flash_pause should be called on quit"
        );
    }

    #[test]
    fn select_new_game_calls_flash_pause() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(
            ops.flash_pause_called.get(),
            "Flash_pause should be called on new game"
        );
    }

    #[test]
    fn select_setup_calls_setup_menu_and_flash_continue() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Setup.as_u8();

        let result = do_restart_frame(&mut ops, &mut state);
        assert!(result, "SETUP should stay in menu");
        assert!(ops.setup_menu_called.get(), "SetupMenu should be called");
        assert!(ops.flash_pause_called.get(), "Flash_pause should be called");
        assert!(
            ops.flash_continue_called.get(),
            "Flash_continue should be called"
        );
    }

    #[test]
    fn mouse_popup_calls_flash_pause_and_continue() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            mouse_down: true,
            ..Default::default()
        });
        let mut state = initialized_state();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(
            ops.flash_pause_called.get(),
            "Flash_pause should be called for mouse popup"
        );
        assert!(
            ops.flash_continue_called.get(),
            "Flash_continue should be called after mouse popup"
        );
    }

    #[test]
    fn first_frame_cleans_up_existing_music() {
        let mut ops = MockOps::new();
        ops.music_loaded.set(true);
        let mut state = DoRestartState::default();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(
            ops.destroy_music_called.get(),
            "existing music should be destroyed"
        );
        assert!(
            ops.music_loaded.get(),
            "replacement music should be retained"
        );
    }

    #[test]
    fn first_frame_creates_and_starts_flash() {
        let mut ops = MockOps::new();
        let mut state = DoRestartState::default();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert!(ops.flash_created.get());
        assert!(ops.flash_started.get());
    }

    #[test]
    fn navigate_up_from_new_game_wraps_to_quit() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            up: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert_eq!(state.cur_state, RestartMenuItem::Quit.as_u8());
    }

    #[test]
    fn navigate_down_from_quit_wraps_to_new_game() {
        let mut ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Quit.as_u8();

        let _ = do_restart_frame(&mut ops, &mut state);
        assert_eq!(state.cur_state, RestartMenuItem::NewGame.as_u8());
    }
}
