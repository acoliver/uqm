//! DoRestart per-frame callback — replaces `DoRestart()` in restart.c.
//!
//! @plan PLAN-20260707-RESTARTMENU.P06
//! @requirement REQ-RM-006

#[cfg(not(test))]
use std::os::raw::c_int;

use super::c_extern;
use super::menu_logic::{self, check_timeout};
use super::restart_ops::RestartMenuOps;
use super::types::{MenuInputState, RestartMenuItem, SelectionResult};

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
    /// Last time input was received (for inactivity timeout).
    pub last_input_time: u32,
    /// Inactivity timeout duration in ticks.
    pub inact_timeout: u32,
    /// Current menu item index (C `pMS->CurState`).
    pub cur_state: u8,
    /// Flash overlay handle (C `pMS->flashContext`).
    pub flash_context: usize,
    /// Menu music handle (C `pMS->hMusic`), 0 if none.
    pub music_handle: usize,
}

impl Default for DoRestartState {
    fn default() -> Self {
        Self {
            initialized: false,
            last_input_time: 0,
            inact_timeout: 0,
            cur_state: RestartMenuItem::NewGame.as_u8(),
            flash_context: 0,
            music_handle: 0,
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
pub fn do_restart_frame<O: RestartMenuOps + ?Sized>(ops: &O, state: &mut DoRestartState) -> bool {
    let time_in = ops.get_time_counter();

    // Cancel any Pause key presses (restart.c:113)
    ops.set_game_paused(false);

    if state.initialized {
        // Process flash overlay on subsequent frames (restart.c:116)
        ops.flash_process(state.flash_context);
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
    ops.sleep_thread_until(time_in.wrapping_add(c_extern::ONE_SECOND / 30));

    true
}

/// First-frame initialization — loads music, creates flash overlay,
/// draws the initial menu, and fades in.
///
/// Matches restart.c:117-137.
fn init_first_frame<O: RestartMenuOps + ?Sized>(ops: &O, state: &mut DoRestartState) {
    // Clean up existing music (restart.c:119-124)
    if state.music_handle != 0 {
        ops.stop_music();
        ops.destroy_music(state.music_handle);
        state.music_handle = 0;
    }

    // Load main menu music (restart.c:125)
    state.music_handle = ops.load_menu_music();
    state.inact_timeout = if state.music_handle != 0 { 120 } else { 20 } * c_extern::ONE_SECOND;

    // Create and configure flash overlay (restart.c:126-132)
    state.flash_context = ops.create_flash_overlay();
    let fc = state.flash_context;
    ops.flash_set_merge_factors(fc, -3, 3, 16);
    ops.flash_set_speed(
        fc,
        (6 * c_extern::ONE_SECOND) / 16,
        0,
        (6 * c_extern::ONE_SECOND) / 16,
        0,
    );
    ops.flash_set_frame_time(fc, c_extern::ONE_SECOND / 16);
    ops.flash_set_state_fade_in(fc, (3 * c_extern::ONE_SECOND) / 16);

    // Draw initial menu state (restart.c:133-134)
    // Sync critical fields to C MENU_STATE before drawing — DrawRestartMenu
    // reads pMS->flashContext and DrawRestartMenuGraphic reads pMS->CurFrame.
    ops.sync_flash_context(state.flash_context);
    ops.sync_h_music(state.music_handle);
    ops.sync_cur_state(state.cur_state);
    ops.sync_initialized(true);
    ops.draw_restart_menu_graphic();
    ops.draw_restart_menu_state(state.cur_state);

    // Start flash and music (restart.c:135-137)
    ops.flash_start(fc);
    if state.music_handle != 0 {
        ops.play_music(state.music_handle);
    }

    state.last_input_time = ops.get_time_counter();
    state.initialized = true;

    // Fade in (restart.c:138)
    let fade_result = ops.fade_screen(
        c_extern::FADE_ALL_TO_COLOR,
        (c_extern::ONE_SECOND / 2) as i16,
    );
    ops.sleep_thread_until(fade_result);
}

/// Handle menu selection dispatch.
///
/// Returns `true` if the menu should continue (SETUP_GAME case),
/// `false` if it should exit.
///
/// Matches restart.c:144-188.
fn handle_select<O: RestartMenuOps + ?Sized>(ops: &O, state: &mut DoRestartState) -> bool {
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
            ops.flash_pause(state.flash_context);
            false
        }
        SelectionResult::SuperMelee => {
            ops.set_current_activity(SUPER_MELEE);
            ops.flash_pause(state.flash_context);
            false
        }
        SelectionResult::StayInMenu => {
            // SETUP_GAME path (restart.c:156-178)
            let fc = state.flash_context;
            ops.flash_pause(fc);
            ops.flash_set_state_fade_in(fc, (3 * c_extern::ONE_SECOND) / 16);
            ops.setup_menu();
            ops.set_menu_sounds(
                c_extern::MENU_SOUND_UP | c_extern::MENU_SOUND_DOWN,
                c_extern::MENU_SOUND_SELECT,
            );
            state.last_input_time = ops.get_time_counter();
            ops.set_transition_source_null();
            ops.batch_graphics();
            ops.draw_restart_menu_graphic();
            ops.screen_transition(3);
            ops.draw_restart_menu_state(state.cur_state);
            ops.flash_continue(fc);
            ops.unbatch_graphics();
            true
        }
        SelectionResult::Quit => {
            // QUIT_GAME (restart.c:172-175)
            let fade_result = ops.fade_screen(
                c_extern::FADE_ALL_TO_BLACK,
                (c_extern::ONE_SECOND / 2) as i16,
            );
            ops.sleep_thread_until(fade_result);
            ops.set_current_activity(CHECK_ABORT);
            ops.flash_pause(state.flash_context);
            false
        }
    }
}

/// Handle up/down navigation.
///
/// Matches restart.c:179-214.
fn handle_navigate<O: RestartMenuOps + ?Sized>(
    ops: &O,
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
        // Exact order: draw → assign cur_state → sync → observe.
        state.cur_state = new_item.as_u8();
        ops.sync_cur_state(state.cur_state);

        // Typed observer: returns Continue or Stop.
        let _control = crate::automation::input::observe_main_menu_transition(
            cur_item.as_u8(),
            new_item.as_u8(),
            None,
        );

        // Feed the menu transition to the automation coordinator if active.
        // This drives the scheduler's WaitingSemantic state and semantic
        // assertion matching.
        if crate::automation::coordinator::Coordinator::is_active() {
            let _stop = crate::automation::coordinator::Coordinator::process_menu_transition(
                new_item.as_u8(),
            );
        }
        // In full integration, Stop would propagate through do_restart_frame
        // and rust_do_restart_frame before sleep/later work.
    }

    state.last_input_time = ops.get_time_counter();
}

/// Handle mouse click popup ("mouse not supported").
///
/// Matches restart.c:220-234.
fn handle_mouse_popup<O: RestartMenuOps + ?Sized>(ops: &O, state: &mut DoRestartState) {
    let fc = state.flash_context;
    ops.flash_pause(fc);

    // Offset 54 from MAINMENU_STRING_BASE = "mouse not supported" message.
    // The C wrapper adds MAINMENU_STRING_BASE internally.
    const MOUSE_NOT_SUPPORTED_STRING: u16 = 54;

    ops.do_popup_window_msg(MOUSE_NOT_SUPPORTED_STRING);
    ops.set_menu_sounds(
        c_extern::MENU_SOUND_UP | c_extern::MENU_SOUND_DOWN,
        c_extern::MENU_SOUND_SELECT,
    );
    ops.set_transition_source_null();
    ops.batch_graphics();
    ops.draw_restart_menu_graphic();
    ops.draw_restart_menu_state(state.cur_state);
    ops.screen_transition(3);
    ops.unbatch_graphics();

    ops.flash_continue(fc);

    state.last_input_time = ops.get_time_counter();
}

/// Handle inactivity timeout.
///
/// Matches restart.c:236-244.
fn handle_timeout<O: RestartMenuOps + ?Sized>(ops: &O) {
    // Fade music out, stop, restore volume (restart.c:237-239)
    let fade_result = ops.fade_music(0, c_extern::ONE_SECOND as i16);
    ops.sleep_thread_until(fade_result);
    ops.stop_music();
    ops.fade_music(c_extern::NORMAL_VOLUME as u8, 0);

    // Set activity to ~0 (all bits set) (restart.c:243)
    ops.set_current_activity(!0u16);
}

// ===========================================================================
//  C entry point (non-test only)
// ===========================================================================

/// C-callable entry point for `DoRestart` when `USE_RUST_RESTART` is defined.
///
/// Called by the C `DoInput` loop via the `InputFunc` trampoline.
///
/// @plan PLAN-20260707-RESTARTMENU.P06
/// @requirement REQ-RM-006
#[cfg(not(test))]
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn rust_do_restart_frame(menu_state_ptr: *mut std::os::raw::c_void) -> c_int {
    if menu_state_ptr.is_null() {
        return 0;
    }
    let ops = super::restart_ops::CffiOps::new();
    // Set the Cell to the real C MENU_STATE pointer so all subsequent
    // draw/input calls pass the correct pointer.
    ops.set_menu_state_ptr(menu_state_ptr as usize);

    // Extract DoRestartState from MENU_STATE.privData.
    // SAFETY: privData was set by try_start_game_impl to point to a
    // DoRestartState that outlives the DoInput loop.
    let priv_data = unsafe { super::c_extern::uqm_get_menu_priv_data(menu_state_ptr) };
    if priv_data.is_null() {
        return 0;
    }
    // SAFETY: priv_data was set by the Rust side to point to a DoRestartState.
    let state = unsafe { &mut *(priv_data as *mut DoRestartState) };

    if do_restart_frame(&ops, state) {
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
    use super::super::c_extern::Color;
    use super::super::restart_ops::RestartMenuOps;
    use super::*;
    use std::cell::Cell;
    use std::os::raw::{c_int, c_short};

    /// Mock implementation of RestartMenuOps for unit testing.
    /// Uses interior mutability via Cell to track state.
    struct MockOps {
        current_activity: Cell<u16>,
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
        menu_state_ptr: Cell<usize>,
        nav_count: Cell<u32>,
        draw_menu_called: Cell<bool>,
        popup_called: Cell<bool>,
        flash_process_called: Cell<bool>,
        flash_pause_called: Cell<bool>,
        flash_continue_called: Cell<bool>,
        setup_menu_called: Cell<bool>,
        destroy_music_called: Cell<bool>,
    }

    impl MockOps {
        fn new() -> Self {
            Self {
                current_activity: Cell::new(0),
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
                menu_state_ptr: Cell::new(0),
                nav_count: Cell::new(0),
                draw_menu_called: Cell::new(false),
                popup_called: Cell::new(false),
                flash_process_called: Cell::new(false),
                flash_pause_called: Cell::new(false),
                flash_continue_called: Cell::new(false),
                setup_menu_called: Cell::new(false),
                destroy_music_called: Cell::new(false),
            }
        }
    }

    impl RestartMenuOps for MockOps {
        fn get_current_activity(&self) -> u16 {
            self.current_activity.get()
        }
        fn set_current_activity(&self, v: u16) {
            self.current_activity.set(v);
            self.set_activity_called.set(true);
        }
        fn get_last_activity(&self) -> u16 {
            self.last_activity.get()
        }
        fn set_last_activity(&self, v: u16) {
            self.last_activity.set(v);
        }
        fn set_next_activity(&self, _v: u16) {}
        fn set_menu_state_ptr(&self, ptr: usize) {
            self.menu_state_ptr.set(ptr);
        }
        fn get_menu_state_ptr(&self) -> usize {
            self.menu_state_ptr.get()
        }
        fn get_menu_frame(&self) -> usize {
            0
        }
        fn create_menu_state(&self) -> usize {
            0
        }
        fn set_menu_priv_data(&self, _ptr: usize, _data: usize) {}
        fn destroy_menu_state(&self, _ptr: usize) {}
        fn sync_flash_context(&self, _ctx: usize) {}
        fn sync_initialized(&self, _val: bool) {}
        fn sync_cur_state(&self, _state: u8) {}
        fn sync_cur_frame(&self, _frame: usize) {}
        fn sync_h_music(&self, _handle: usize) {}
        fn get_menu_input(&self) -> MenuInputState {
            self.menu_input.get()
        }
        fn get_time_counter(&self) -> u32 {
            self.time_counter.get()
        }
        fn get_utwig_bomb_on_ship(&self) -> u8 {
            0
        }
        fn set_utwig_bomb_on_ship(&self, _v: u8) {}
        fn get_utwig_bomb(&self) -> u8 {
            0
        }
        fn get_crew_enlisted(&self) -> u16 {
            0
        }
        fn set_game_paused(&self, val: bool) {
            self.game_paused.set(val);
        }
        fn reinit_race_queues(&self) {}
        fn set_screen_context(&self) {}
        fn fade_screen(&self, fade_type: u32, _duration: c_short) -> u32 {
            self.fade_screen_called.set(true);
            self.fade_screen_type.set(fade_type);
            self.fade_result.get()
        }
        fn sleep_thread_until(&self, _time: u32) {
            self.sleep_until_called.set(true);
        }
        fn sleep_thread(&self, _duration: u32) {}
        fn batch_graphics(&self) {}
        fn unbatch_graphics(&self) {
            self.nav_count.set(self.nav_count.get() + 1);
        }
        fn clear_drawable(&self) {}
        fn flush_color_xforms(&self) {}
        fn screen_transition(&self, _a: c_int) {}
        fn set_bg_color(&self, _color: Color) {}
        fn seed_random(&self) {}
        fn load_menu_graphic(&self) -> usize {
            1
        }
        fn capture_drawable(&self, _load_result: usize) -> usize {
            1
        }
        fn destroy_drawable(&self, _handle: usize) {}
        fn release_drawable(&self, _handle: usize) -> usize {
            0
        }
        fn draw_restart_menu_graphic(&self) {
            self.draw_menu_called.set(true);
        }
        fn draw_restart_menu_state(&self, _state: u8) {
            self.draw_menu_called.set(true);
        }
        fn set_menu_sounds(&self, _s0: u16, _s1: u16) {}
        fn set_default_menu_repeat_delay(&self) {}
        fn set_transition_source_null(&self) {}
        fn run_do_input(&self, _reset_input: bool) {}
        fn load_menu_music(&self) -> usize {
            1
        }
        fn play_music(&self, _handle: usize) {
            self.play_music_called.set(true);
        }
        fn stop_music(&self) {
            self.stop_music_called.set(true);
        }
        fn destroy_music(&self, _handle: usize) {
            self.destroy_music_called.set(true);
        }
        fn fade_music(&self, _end_vol: u8, _time_interval: c_short) -> u32 {
            200
        }
        fn create_flash_overlay(&self) -> usize {
            1
        }
        fn flash_process(&self, _ctx: usize) {
            self.flash_process_called.set(true);
        }
        fn flash_pause(&self, _ctx: usize) {
            self.flash_pause_called.set(true);
        }
        fn flash_continue(&self, _ctx: usize) {
            self.flash_continue_called.set(true);
        }
        fn flash_start(&self, _ctx: usize) {}
        fn flash_terminate(&self, _ctx: usize) {}
        fn flash_set_merge_factors(&self, _ctx: usize, _a: c_int, _b: c_int, _c: c_int) {}
        fn flash_set_speed(&self, _ctx: usize, _a: u32, _b: u32, _c: u32, _d: u32) {}
        fn flash_set_frame_time(&self, _ctx: usize, _t: u32) {}
        fn flash_set_state_fade_in(&self, _ctx: usize, _duration: u32) {}
        fn flash_set_overlay(&self, _ctx: usize, _frame_idx: u32) {}
        fn melee(&self) {}
        fn setup_menu(&self) {
            self.setup_menu_called.set(true);
        }
        fn free_game_data(&self) {}
        fn introduction(&self) {}
        fn credits(&self, _victory: bool) {}
        fn victory(&self) {}
        fn splash_screen(&self) {}
        fn do_popup_window_msg(&self, _string_id: u16) {
            self.popup_called.set(true);
        }
        fn set_player_control(&self, _player: u8, _control: u16) {}
        fn assign_global_arrays(&self) {}
        fn set_main_exited(&self, _val: bool) {}
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
        let ops = MockOps::new();
        let mut state = DoRestartState::default();
        let result = do_restart_frame(&ops, &mut state);

        assert!(result, "first frame should return true (continue)");
        assert!(state.initialized, "state should be initialized");
        assert!(ops.play_music_called.get(), "music should play");
    }

    #[test]
    fn abort_returns_false() {
        let ops = MockOps::new();
        ops.current_activity.set(CHECK_ABORT);

        let mut state = initialized_state();

        let result = do_restart_frame(&ops, &mut state);
        assert!(!result, "CHECK_ABORT should exit menu");
    }

    #[test]
    fn select_new_game_sets_activity_and_exits() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let result = do_restart_frame(&ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.last_activity.get(), CHECK_LOAD | CHECK_RESTART);
        assert_eq!(ops.current_activity.get(), IN_INTERPLANETARY);
    }

    #[test]
    fn select_load_game_sets_activity_and_exits() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::LoadGame.as_u8();

        let result = do_restart_frame(&ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.last_activity.get(), CHECK_LOAD);
        assert_eq!(ops.current_activity.get(), IN_INTERPLANETARY);
    }

    #[test]
    fn select_super_melee_sets_activity_zero() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::SuperMelee.as_u8();

        let result = do_restart_frame(&ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.current_activity.get(), SUPER_MELEE);
    }

    #[test]
    fn select_quit_sets_check_abort() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Quit.as_u8();

        let result = do_restart_frame(&ops, &mut state);
        assert!(!result, "select should exit menu");
        assert_eq!(ops.current_activity.get(), CHECK_ABORT);
        assert_eq!(ops.fade_screen_type.get(), c_extern::FADE_ALL_TO_BLACK);
    }

    #[test]
    fn select_setup_stays_in_menu() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Setup.as_u8();

        let result = do_restart_frame(&ops, &mut state);
        assert!(result, "SETUP_GAME should stay in menu");
    }

    #[test]
    fn navigate_down_from_new_game_changes_state() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let _ = do_restart_frame(&ops, &mut state);
        assert_eq!(
            state.cur_state,
            RestartMenuItem::LoadGame.as_u8(),
            "down from NewGame should go to LoadGame"
        );
    }

    #[test]
    fn navigate_up_from_load_game_changes_state() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            up: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::LoadGame.as_u8();

        let _ = do_restart_frame(&ops, &mut state);
        assert_eq!(
            state.cur_state,
            RestartMenuItem::NewGame.as_u8(),
            "up from LoadGame should go to NewGame"
        );
    }

    #[test]
    fn navigate_updates_last_input_time() {
        let ops = MockOps::new();
        ops.time_counter.set(500);
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.last_input_time = 0;

        let _ = do_restart_frame(&ops, &mut state);
        assert_eq!(state.last_input_time, 500);
    }

    #[test]
    fn left_right_updates_last_input_time() {
        let ops = MockOps::new();
        ops.time_counter.set(42);
        ops.menu_input.set(MenuInputState {
            left: true,
            ..Default::default()
        });

        let mut state = initialized_state();
        state.last_input_time = 0;

        let _ = do_restart_frame(&ops, &mut state);
        assert_eq!(state.last_input_time, 42);
    }

    #[test]
    fn mouse_click_triggers_popup() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            mouse_down: true,
            ..Default::default()
        });

        let mut state = initialized_state();

        let result = do_restart_frame(&ops, &mut state);
        assert!(result, "mouse popup should continue menu");
        assert!(ops.popup_called.get(), "popup window should be shown");
    }

    #[test]
    fn timeout_exits_menu() {
        let ops = MockOps::new();
        ops.time_counter.set(1000);
        // No input
        ops.menu_input.set(MenuInputState::default());

        let mut state = initialized_state();
        state.last_input_time = 0;
        state.inact_timeout = 100;

        let result = do_restart_frame(&ops, &mut state);
        assert!(!result, "timeout should exit menu");
        assert!(ops.stop_music_called.get(), "music should stop");
        assert_eq!(ops.current_activity.get(), !0u16, "activity should be ~0");
    }

    #[test]
    fn no_timeout_when_within_window() {
        let ops = MockOps::new();
        ops.time_counter.set(50);
        ops.menu_input.set(MenuInputState::default());

        let mut state = initialized_state();
        state.last_input_time = 0;
        state.inact_timeout = 100;

        let result = do_restart_frame(&ops, &mut state);
        assert!(result, "should continue when within timeout window");
    }

    #[test]
    fn game_paused_cleared_each_frame() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState::default());

        let mut state = initialized_state();

        let _ = do_restart_frame(&ops, &mut state);
        assert!(!ops.game_paused.get(), "game paused should be cleared");
    }

    #[test]
    fn sleep_thread_called_for_frame_limit() {
        let ops = MockOps::new();
        ops.time_counter.set(100);

        let mut state = initialized_state();
        ops.menu_input.set(MenuInputState::default());
        state.inact_timeout = 10000;

        let _ = do_restart_frame(&ops, &mut state);
        assert!(ops.sleep_until_called.get(), "frame rate limit should fire");
    }

    #[test]
    fn flash_process_called_on_initialized_frame() {
        let ops = MockOps::new();
        let mut state = initialized_state();
        state.inact_timeout = 10000;
        ops.menu_input.set(MenuInputState::default());

        let _ = do_restart_frame(&ops, &mut state);
        assert!(
            ops.flash_process_called.get(),
            "flash_process should be called on initialized frame"
        );
    }

    #[test]
    fn select_quit_calls_flash_pause() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Quit.as_u8();

        let _ = do_restart_frame(&ops, &mut state);
        assert!(
            ops.flash_pause_called.get(),
            "Flash_pause should be called on quit"
        );
    }

    #[test]
    fn select_new_game_calls_flash_pause() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let _ = do_restart_frame(&ops, &mut state);
        assert!(
            ops.flash_pause_called.get(),
            "Flash_pause should be called on new game"
        );
    }

    #[test]
    fn select_setup_calls_setup_menu_and_flash_continue() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            select: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Setup.as_u8();

        let result = do_restart_frame(&ops, &mut state);
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
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            mouse_down: true,
            ..Default::default()
        });
        let mut state = initialized_state();

        let _ = do_restart_frame(&ops, &mut state);
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
        let ops = MockOps::new();
        let mut state = DoRestartState {
            music_handle: 42, // Simulate pre-existing music
            ..Default::default()
        };

        let _ = do_restart_frame(&ops, &mut state);
        assert!(
            ops.destroy_music_called.get(),
            "existing music should be destroyed"
        );
        assert_ne!(state.music_handle, 0, "new music handle should be stored");
    }

    #[test]
    fn first_frame_stores_flash_context() {
        let ops = MockOps::new();
        let mut state = DoRestartState::default();

        let _ = do_restart_frame(&ops, &mut state);
        assert_ne!(state.flash_context, 0, "flash context should be stored");
    }

    #[test]
    fn navigate_up_from_new_game_wraps_to_quit() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            up: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::NewGame.as_u8();

        let _ = do_restart_frame(&ops, &mut state);
        assert_eq!(state.cur_state, RestartMenuItem::Quit.as_u8());
    }

    #[test]
    fn navigate_down_from_quit_wraps_to_new_game() {
        let ops = MockOps::new();
        ops.menu_input.set(MenuInputState {
            down: true,
            ..Default::default()
        });
        let mut state = initialized_state();
        state.cur_state = RestartMenuItem::Quit.as_u8();

        let _ = do_restart_frame(&ops, &mut state);
        assert_eq!(state.cur_state, RestartMenuItem::NewGame.as_u8());
    }
}
