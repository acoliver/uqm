//! RestartMenuOps trait — abstraction over C-side operations.
//!
//! @plan PLAN-20260707-RESTARTMENU.P05
//! @requirement REQ-RM-005

#[cfg(not(test))]
use std::ffi::CString;
#[cfg(not(test))]
use std::os::raw::c_void;
use std::os::raw::{c_int, c_short};

use super::c_extern::Color;
#[cfg(not(test))]
use super::c_extern::{self, Point};
use super::types::MenuInputState;

// ===========================================================================
//  Constants
// ===========================================================================

/// `"music.mainmenu"` (imusicre.h:9).
pub const MAINMENU_MUSIC: &str = "music.mainmenu";

/// `"graphics.newgame"` (igfxres.h:251).
pub const RESTART_PMAP_ANIM: &str = "graphics.newgame";

/// Seed the generator for a new campaign (restart.c ~350-360).
///
/// A proof run supplies a fixed seed through the automation coordinator so a
/// campaign replays; outside automation the coordinator returns the `0`
/// sentinel and `clock_seed` seeds from the wall clock as before.
pub fn seed_new_campaign(automation_seed: u32, clock_seed: impl FnOnce()) {
    if automation_seed != 0 {
        crate::math::seed_random(automation_seed);
    } else {
        clock_seed();
    }
}

// ===========================================================================
//  RestartMenuOps trait
// ===========================================================================

/// Abstraction over all C-side operations the restart menu performs.
///
/// Production code uses CffiOps (real FFI). Tests use mock implementations.
///
/// @plan PLAN-20260707-RESTARTMENU.P05
/// @requirement REQ-RM-005
pub trait RestartMenuOps {
    fn input(&self, _view: crate::engine::observe::InputView) -> crate::engine::observe::Injected {
        crate::engine::observe::Injected::default()
    }

    // --- Activity globals ---
    fn get_current_activity(&self) -> u16;
    fn set_current_activity(&self, v: u16);
    fn get_last_activity(&self) -> u16;
    fn set_last_activity(&self, v: u16);
    fn set_next_activity(&self, v: u16);

    // --- Input ---
    fn get_menu_input(&self) -> MenuInputState;
    fn get_time_counter(&self) -> u32;

    // --- Game state ---
    fn get_utwig_bomb_on_ship(&self) -> u8;
    fn set_utwig_bomb_on_ship(&self, v: u8);
    fn get_utwig_bomb(&self) -> u8;
    fn get_crew_enlisted(&self) -> u16;

    // --- Game paused ---
    fn set_game_paused(&self, val: bool);

    // --- Race queues ---
    fn reinit_race_queues(&self);

    // --- Graphics ---
    fn set_screen_context(&self);
    /// `FadeScreen(ScreenFadeType, SIZE)` — returns a TimeCount for SleepThreadUntil.
    fn fade_screen(&self, fade_type: u32, duration: c_short) -> u32;
    fn sleep_thread_until(&self, time: u32);
    fn sleep_thread(&self, duration: u32);
    fn batch_graphics(&self);
    fn unbatch_graphics(&self);
    fn clear_drawable(&self);
    fn flush_color_xforms(&self);
    fn screen_transition(&self, a: c_int);
    fn set_bg_color(&self, color: Color);
    fn seed_random(&self);

    // --- Resources ---
    fn load_menu_graphic(&self) -> usize;
    fn capture_drawable(&self, load_result: usize) -> usize;
    fn destroy_drawable(&self, handle: usize);
    fn release_drawable(&self, handle: usize) -> usize;

    // --- Menu ---
    /// Set the MENU_STATE pointer for subsequent draw/input calls.
    fn set_menu_state_ptr(&self, ptr: usize);
    /// Get the MENU_STATE pointer.
    fn get_menu_state_ptr(&self) -> usize;
    /// Get the current frame from MENU_STATE.
    fn get_menu_frame(&self) -> usize;
    /// Create the drawing record for the restart menu.
    /// Returns a pointer (0 in test/mock mode).
    fn create_menu_state(&self) -> usize;
    /// Destroy a C MENU_STATE created by create_menu_state.
    fn destroy_menu_state(&self, ptr: usize);

    // --- MENU_STATE field sync (Rust → C) ---
    // These write DoRestartState values back to the C MENU_STATE so that
    // C drawing functions (DrawRestartMenu, DrawRestartMenuGraphic) see
    // the correct field values.
    fn sync_flash_context(&self, ctx: usize);
    fn sync_initialized(&self, val: bool);
    fn sync_cur_state(&self, state: u8);
    fn sync_cur_frame(&self, frame: usize);
    fn sync_h_music(&self, handle: usize);
    fn draw_restart_menu_graphic(&self);
    fn draw_restart_menu_state(&self, state: u8);
    fn set_menu_sounds(&self, s0: u16, s1: u16);
    fn set_default_menu_repeat_delay(&self);
    fn set_transition_source_null(&self);
    fn run_do_input(&self, reset_input: bool, state: &mut super::do_restart::DoRestartState);

    // --- Music ---
    fn load_menu_music(&self) -> usize;
    fn play_music(&self, handle: usize);
    fn stop_music(&self);
    fn destroy_music(&self, handle: usize);
    /// `FadeMusic(BYTE, SIZE)` — returns a TimeCount for SleepThreadUntil.
    fn fade_music(&self, end_vol: u8, time_interval: c_short) -> u32;

    // --- Flash ---
    fn create_flash_overlay(&self) -> usize;
    fn flash_process(&self, ctx: usize);
    fn flash_pause(&self, ctx: usize);
    fn flash_continue(&self, ctx: usize);
    fn flash_start(&self, ctx: usize);
    fn flash_terminate(&self, ctx: usize);
    fn flash_set_merge_factors(&self, ctx: usize, a: c_int, b: c_int, c: c_int);
    fn flash_set_speed(&self, ctx: usize, a: u32, b: u32, c: u32, d: u32);
    fn flash_set_frame_time(&self, ctx: usize, t: u32);
    fn flash_set_state_fade_in(&self, ctx: usize, duration: u32);
    fn flash_set_overlay(&self, ctx: usize, frame_idx: u32);

    // --- Lifecycle ---
    fn melee(&self);
    fn setup_menu(&self);
    fn free_game_data(&self);
    fn credits(&self, victory: bool);
    fn victory(&self);
    fn introduction(&self);
    fn splash_screen(&self);
    fn do_popup_window_msg(&self, string_id: u16);

    // --- Player control ---
    fn set_player_control(&self, player: u8, control: u16);
    fn assign_global_arrays(&self);
    fn set_main_exited(&self, val: bool);
}

// ===========================================================================
//  CffiOps — production implementation using real FFI (non-test only)
// ===========================================================================

#[cfg(not(test))]
struct RestartContext<'a, 'b, 'c> {
    cx: &'a mut crate::engine::battle::Cx<'b>,
    io: &'a mut crate::engine::observe::Io<'c>,
}

#[cfg(not(test))]
pub struct CffiOps<'a, 'b, 'c> {
    menu_state: std::cell::Cell<usize>,
    context: std::cell::RefCell<RestartContext<'a, 'b, 'c>>,
}

#[cfg(not(test))]
impl<'a, 'b, 'c> CffiOps<'a, 'b, 'c> {
    pub fn new(
        cx: &'a mut crate::engine::battle::Cx<'b>,
        io: &'a mut crate::engine::observe::Io<'c>,
    ) -> Self {
        Self {
            menu_state: std::cell::Cell::new(0),
            context: std::cell::RefCell::new(RestartContext { cx, io }),
        }
    }

    fn owned<R>(
        &self,
        call: impl FnOnce(&mut crate::engine::battle::Cx<'_>, &mut crate::engine::observe::Io<'_>) -> R,
    ) -> R {
        let mut context = self.context.borrow_mut();
        let RestartContext { cx, io } = &mut *context;
        call(cx, io)
    }

    fn legacy<R>(&self, call: impl FnOnce() -> R) -> R {
        self.owned(|cx, _| cx.with_legacy(call))
    }
}

#[cfg(not(test))]
impl RestartMenuOps for CffiOps<'_, '_, '_> {
    fn input(&self, view: crate::engine::observe::InputView) -> crate::engine::observe::Injected {
        self.owned(|cx, io| {
            let campaign = crate::engine::observe::CampaignView::for_input(view, || {
                crate::engine::observe::CampaignView::capture(cx.world)
            });
            cx.with_legacy(|| io.observer.input_admitted(&view, &campaign))
        })
    }

    fn get_current_activity(&self) -> u16 {
        self.owned(|cx, _| cx.world.activity())
    }
    fn set_current_activity(&self, v: u16) {
        self.owned(|cx, _| cx.world.set_activity(v));
    }
    fn get_last_activity(&self) -> u16 {
        self.owned(|cx, _| cx.kernel.last_activity)
    }
    fn set_last_activity(&self, v: u16) {
        self.owned(|cx, _| cx.kernel.last_activity = v);
    }
    fn set_next_activity(&self, v: u16) {
        self.legacy(|| {
            crate::mainloop::ffi::set_next_activity(crate::mainloop::types::ActivityValue(v));
        })
    }

    fn set_menu_state_ptr(&self, ptr: usize) {
        self.menu_state.set(ptr);
    }
    fn get_menu_state_ptr(&self) -> usize {
        self.menu_state.get()
    }
    fn get_menu_frame(&self) -> usize {
        // Read pMS->CurFrame from the MENU_STATE struct.
        // MENU_STATE layout (menustat.h): the CurFrame field is at a known
        // offset. We read it via the C wrapper.
        let pms = self.menu_state.get();
        if pms == 0 {
            return 0;
        }
        // SAFETY: pms was set by set_menu_state_ptr to a valid MENU_STATE*.
        // CurFrame is accessed through the MENU_STATE struct.
        unsafe { super::c_extern::uqm_get_menu_cur_frame(pms as *mut std::os::raw::c_void) }
    }
    fn create_menu_state(&self) -> usize {
        self.legacy(|| {
            // SAFETY: the menu's drawing record remains owned until destroy_menu_state.
            unsafe { super::c_extern::uqm_create_menu_state() as usize }
        })
    }
    fn destroy_menu_state(&self, ptr: usize) {
        self.legacy(|| {
            // SAFETY: Frees the MENU_STATE allocated by create_menu_state.
            unsafe { super::c_extern::uqm_destroy_menu_state(ptr as *mut std::os::raw::c_void) }
        })
    }

    fn sync_flash_context(&self, ctx: usize) {
        self.legacy(|| {
            let pms = self.get_menu_state_ptr();
            if pms != 0 {
                // SAFETY: pms is a valid MENU_STATE* set by set_menu_state_ptr.
                unsafe {
                    super::c_extern::uqm_set_menu_flash_context(
                        pms as *mut std::os::raw::c_void,
                        ctx,
                    )
                }
            }
        })
    }
    fn sync_initialized(&self, val: bool) {
        self.legacy(|| {
            let pms = self.get_menu_state_ptr();
            if pms != 0 {
                unsafe {
                    super::c_extern::uqm_set_menu_initialized(
                        pms as *mut std::os::raw::c_void,
                        if val { 1 } else { 0 },
                    )
                }
            }
        })
    }
    fn sync_cur_state(&self, state: u8) {
        self.legacy(|| {
            let pms = self.get_menu_state_ptr();
            if pms != 0 {
                unsafe {
                    super::c_extern::uqm_set_menu_cur_state(pms as *mut std::os::raw::c_void, state)
                }
            }
        })
    }
    fn sync_cur_frame(&self, frame: usize) {
        self.legacy(|| {
            let pms = self.get_menu_state_ptr();
            if pms != 0 {
                unsafe {
                    super::c_extern::uqm_set_menu_cur_frame(pms as *mut std::os::raw::c_void, frame)
                }
            }
        })
    }
    fn sync_h_music(&self, handle: usize) {
        self.legacy(|| {
            let pms = self.get_menu_state_ptr();
            if pms != 0 {
                unsafe {
                    super::c_extern::uqm_set_menu_h_music(pms as *mut std::os::raw::c_void, handle)
                }
            }
        })
    }

    fn get_menu_input(&self) -> MenuInputState {
        self.legacy(|| {
            // SAFETY: Reads global input state arrays via C accessors.
            unsafe {
                MenuInputState {
                    select: c_extern::uqm_get_pulsed_menu_key(c_extern::KEY_MENU_SELECT) != 0,
                    up: c_extern::uqm_get_pulsed_menu_key(c_extern::KEY_MENU_UP) != 0,
                    down: c_extern::uqm_get_pulsed_menu_key(c_extern::KEY_MENU_DOWN) != 0,
                    left: c_extern::uqm_get_pulsed_menu_key(c_extern::KEY_MENU_LEFT) != 0,
                    right: c_extern::uqm_get_pulsed_menu_key(c_extern::KEY_MENU_RIGHT) != 0,
                    mouse_down: c_extern::uqm_get_mouse_button_down() != 0,
                }
            }
        })
    }

    fn get_time_counter(&self) -> u32 {
        self.legacy(|| c_extern::uqm_get_time_counter())
    }

    fn get_utwig_bomb_on_ship(&self) -> u8 {
        self.legacy(|| {
            // SAFETY: Reads a global game-state byte.
            unsafe { c_extern::uqm_get_utwig_bomb_on_ship() }
        })
    }
    fn set_utwig_bomb_on_ship(&self, v: u8) {
        self.legacy(|| {
            // SAFETY: Writes a global game-state byte.
            unsafe { c_extern::uqm_set_utwig_bomb_on_ship(v) }
        })
    }
    fn get_utwig_bomb(&self) -> u8 {
        self.legacy(|| {
            // SAFETY: Reads a global game-state byte.
            unsafe { c_extern::uqm_get_utwig_bomb() }
        })
    }
    fn get_crew_enlisted(&self) -> u16 {
        self.owned(|cx, _| cx.world.data.flagship.crew_enlisted)
    }

    fn set_game_paused(&self, val: bool) {
        self.legacy(|| {
            // SAFETY: Writes a global pause flag.
            unsafe { c_extern::uqm_set_game_paused(if val { 1 } else { 0 }) }
        })
    }

    fn reinit_race_queues(&self) {
        self.legacy(|| {
            // SAFETY: Reinitializes the race queue arrays.
            unsafe { c_extern::uqm_reinit_race_queues() }
        })
    }

    fn set_screen_context(&self) {
        let context = self.owned(|cx, _| cx.kernel.screen_context);
        // SAFETY: the drawing context belongs to the initialized kernel on this thread.
        self.legacy(|| unsafe { c_extern::SetContext(context) });
    }

    fn fade_screen(&self, fade_type: u32, duration: c_short) -> u32 {
        self.legacy(|| {
            // SAFETY: FadeScreen is safe to call on the main rendering thread.
            unsafe { c_extern::FadeScreen(fade_type, duration) }
        })
    }
    fn sleep_thread_until(&self, time: u32) {
        self.legacy(|| {
            // SAFETY: SleepThreadUntil blocks until the given time counter.
            unsafe { c_extern::uqm_sleep_thread_until(time) }
        })
    }
    fn sleep_thread(&self, duration: u32) {
        self.legacy(|| {
            // SAFETY: SleepThread blocks for the given duration.
            unsafe { c_extern::uqm_sleep_thread(duration) }
        })
    }
    fn batch_graphics(&self) {
        self.legacy(|| {
            // SAFETY: BatchGraphics is safe to call on the main rendering thread.
            unsafe { c_extern::BatchGraphics() }
        })
    }
    fn unbatch_graphics(&self) {
        self.legacy(|| {
            // SAFETY: UnbatchGraphics is safe to call after BatchGraphics.
            unsafe { c_extern::UnbatchGraphics() }
        })
    }
    fn clear_drawable(&self) {
        self.legacy(|| {
            // SAFETY: ClearDrawable clears the current rendering context.
            unsafe { c_extern::ClearDrawable() }
        })
    }
    fn flush_color_xforms(&self) {
        self.legacy(|| {
            // SAFETY: FlushColorXForms flushes pending color transforms.
            unsafe { c_extern::FlushColorXForms() }
        })
    }
    fn screen_transition(&self, a: c_int) {
        self.legacy(|| {
            // SAFETY: ScreenTransition with null drawable is a no-op transition.
            unsafe { c_extern::ScreenTransition(a, std::ptr::null_mut()) }
        })
    }
    fn set_bg_color(&self, color: Color) {
        self.legacy(|| {
            // SAFETY: SetContextBackGroundColor sets the background color for the current context.
            unsafe {
                c_extern::SetContextBackGroundColor(color);
            }
        })
    }
    fn seed_random(&self) {
        let seed = self
            .input(crate::engine::observe::InputView::Seed {
                domain: 3,
                fallback: 0,
            })
            .seed
            .unwrap_or(0);
        self.legacy(|| {
            seed_new_campaign(seed, || unsafe {
                let _ = c_extern::SeedRandomNumbers();
            })
        });
    }

    fn load_menu_graphic(&self) -> usize {
        self.legacy(|| {
            // SAFETY: RESTART_PMAP_ANIM is a compile-time string constant;
            // CString allocation is infallible for ASCII without NUL bytes.
            let res = CString::new(RESTART_PMAP_ANIM).unwrap_or_default();
            // SAFETY: LoadGraphicInstance reads a resource by name; safe on main thread.
            unsafe { c_extern::LoadGraphicInstance(res.as_ptr()) as usize }
        })
    }
    fn capture_drawable(&self, load_result: usize) -> usize {
        self.legacy(|| {
            // SAFETY: load_result was obtained from LoadGraphicInstance.
            unsafe { c_extern::CaptureDrawable((load_result as *mut c_void).cast()) as usize }
        })
    }
    fn destroy_drawable(&self, handle: usize) {
        self.legacy(|| {
            // SAFETY: handle was obtained from CaptureDrawable and is valid.
            unsafe {
                c_extern::DestroyDrawable((handle as *mut c_void).cast());
            }
        })
    }
    fn release_drawable(&self, handle: usize) -> usize {
        self.legacy(|| {
            // SAFETY: handle was obtained from CaptureDrawable.
            unsafe { c_extern::ReleaseDrawable((handle as *mut c_void).cast()) as usize }
        })
    }

    fn draw_restart_menu_graphic(&self) {
        self.legacy(|| {
            // SAFETY: pMS must be set by the caller via set_menu_state_ptr before calling.
            // This calls the de-staticized DrawRestartMenuGraphic in restart.c.
            let pms = self.get_menu_state_ptr();
            unsafe { c_extern::draw_restart_menu_graphic(pms as *mut c_void) }
        })
    }
    fn draw_restart_menu_state(&self, state: u8) {
        self.legacy(|| {
            // SAFETY: pMS must be set by the caller. Frame comes from pMS->CurFrame.
            let pms = self.get_menu_state_ptr();
            let frame = self.get_menu_frame();
            unsafe { c_extern::draw_restart_menu(pms as *mut c_void, state, frame as *mut c_void) }
        })
    }
    fn set_menu_sounds(&self, s0: u16, s1: u16) {
        self.legacy(|| {
            // SAFETY: SetMenuSounds configures menu sound effects.
            unsafe { c_extern::SetMenuSounds(s0, s1) }
        })
    }
    fn set_default_menu_repeat_delay(&self) {
        self.legacy(|| {
            // SAFETY: SetDefaultMenuRepeatDelay resets menu timing.
            unsafe { c_extern::SetDefaultMenuRepeatDelay() }
        })
    }
    fn set_transition_source_null(&self) {
        self.legacy(|| {
            // SAFETY: Passing null is the documented no-source pattern.
            unsafe { c_extern::SetTransitionSource(std::ptr::null_mut()) }
        })
    }
    fn run_do_input(&self, reset_input: bool, state: &mut super::do_restart::DoRestartState) {
        let menu = self.get_menu_state_ptr();
        self.owned(|cx, io| {
            cx.run_input(io, reset_input, |cx, io| {
                let ops = CffiOps::new(cx, io);
                ops.set_menu_state_ptr(menu);
                super::do_restart::do_restart_frame(&ops, state)
            })
        });
    }

    fn load_menu_music(&self) -> usize {
        self.legacy(|| {
            // SAFETY: MAINMENU_MUSIC is a compile-time string constant.
            let res = CString::new(MAINMENU_MUSIC).unwrap_or_default();
            // SAFETY: LoadMusicInstance reads a resource by name; safe on main thread.
            unsafe { c_extern::LoadMusicInstance(res.as_ptr()) as usize }
        })
    }
    fn play_music(&self, handle: usize) {
        self.legacy(|| {
            // SAFETY: handle was obtained from LoadMusicInstance and is valid.
            unsafe { c_extern::PlayMusic((handle as *mut c_void).cast(), 1, 1) }
        })
    }
    fn stop_music(&self) {
        self.legacy(|| {
            // SAFETY: StopMusic is safe to call any time.
            unsafe { c_extern::stop_music() }
        })
    }
    fn destroy_music(&self, handle: usize) {
        self.legacy(|| {
            if handle != 0 {
                // SAFETY: handle was obtained from LoadMusicInstance and is valid.
                unsafe {
                    c_extern::DestroyMusic(handle as *mut c_void);
                }
            }
        })
    }
    fn fade_music(&self, end_vol: u8, time_interval: c_short) -> u32 {
        self.legacy(|| {
            // SAFETY: fade_music is safe to call on the main thread.
            c_extern::FadeMusic(end_vol, time_interval)
        })
    }

    fn create_flash_overlay(&self) -> usize {
        let context = self.owned(|cx, _| cx.kernel.screen_context);
        // SAFETY: the drawing context belongs to the initialized kernel on this thread.
        self.legacy(|| unsafe {
            c_extern::Flash_createOverlay(context, std::ptr::null_mut(), std::ptr::null_mut())
                as usize
        })
    }
    fn flash_process(&self, ctx: usize) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_process((ctx as *mut c_void).cast()) }
        })
    }
    fn flash_pause(&self, ctx: usize) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_pause((ctx as *mut c_void).cast()) }
        })
    }
    fn flash_continue(&self, ctx: usize) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_continue((ctx as *mut c_void).cast()) }
        })
    }
    fn flash_start(&self, ctx: usize) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_start((ctx as *mut c_void).cast()) }
        })
    }
    fn flash_terminate(&self, ctx: usize) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_terminate((ctx as *mut c_void).cast()) }
        })
    }
    fn flash_set_merge_factors(&self, ctx: usize, a: c_int, b: c_int, c: c_int) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_setMergeFactors((ctx as *mut c_void).cast(), a, b, c) }
        })
    }
    fn flash_set_speed(&self, ctx: usize, a: u32, b: u32, c: u32, d: u32) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_setSpeed((ctx as *mut c_void).cast(), a, b, c, d) }
        })
    }
    fn flash_set_frame_time(&self, ctx: usize, t: u32) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe { c_extern::Flash_setFrameTime((ctx as *mut c_void).cast(), t) }
        })
    }
    fn flash_set_state_fade_in(&self, ctx: usize, duration: u32) {
        self.legacy(|| {
            // SAFETY: ctx was obtained from create_flash_overlay.
            unsafe {
                c_extern::Flash_setState(
                    (ctx as *mut c_void).cast(),
                    c_extern::FLASH_STATE_FADE_IN,
                    duration,
                )
            }
        })
    }
    fn flash_set_overlay(&self, ctx: usize, frame_idx: u32) {
        self.legacy(|| {
            // SetAbsFrameIndex takes COUNT (u16), so we cast.
            let idx: u16 = frame_idx as u16;
            let pms = self.get_menu_state_ptr();
            if pms == 0 {
                return;
            }
            // CRITICAL: Flash_setOverlay dereferences origin, so we must pass a
            // valid POINT, not null. C reference passes {0, 0} (restart.c:99).
            let origin = Point { x: 0, y: 0 };
            // SAFETY: pms was set by set_menu_state_ptr. We read CurFrame from
            // the MENU_STATE struct via the C accessor, then set its absolute
            // frame index for the flash overlay.
            unsafe {
                let cur_frame = self.get_menu_frame();
                let new_frame = c_extern::SetAbsFrameIndex((cur_frame as *mut c_void).cast(), idx);
                c_extern::Flash_setOverlay((ctx as *mut c_void).cast(), &origin, new_frame)
            }
        })
    }

    fn melee(&self) {
        self.legacy(|| {
            // SAFETY: Melee starts a Super Melee session on the main thread.
            unsafe { c_extern::Melee() }
        })
    }
    fn setup_menu(&self) {
        self.legacy(|| {
            // SAFETY: SetupMenu initializes menu infrastructure.
            unsafe { c_extern::SetupMenu() }
        })
    }
    fn free_game_data(&self) {
        // SAFETY: the stopped game session owns its resources and suspends itself for callbacks.
        unsafe { self.owned(|cx, _| c_extern::free_game_data(cx)) }
    }

    fn credits(&self, victory: bool) {
        // SAFETY: the restart session runs after graphics, audio and input initialization.
        self.owned(|cx, io| unsafe { crate::front_end::credits::credits(cx, io, victory) });
    }
    fn victory(&self) {
        // SAFETY: the restart session owns the initialized presentation systems.
        self.owned(|cx, io| unsafe { crate::front_end::fmv::victory(cx, io) });
    }
    fn introduction(&self) {
        // SAFETY: the restart session owns the initialized presentation systems.
        self.owned(|cx, io| unsafe { crate::front_end::fmv::introduction(cx, io) });
    }
    fn splash_screen(&self) {
        self.legacy(|| {
            // SAFETY: SplashScreen with None callback is safe (null function pointer).
            unsafe { c_extern::SplashScreen(None) }
        })
    }
    fn do_popup_window_msg(&self, string_id: u16) {
        self.legacy(|| {
            // SAFETY: Popup window message with a valid string ID.
            unsafe { c_extern::uqm_do_popup_window_msg(string_id) }
        })
    }

    fn set_player_control(&self, player: u8, control: u16) {
        self.legacy(|| {
            // SAFETY: Sets the PlayerControl array entry.
            unsafe { c_extern::uqm_set_player_control(player, control) }
        })
    }
    fn assign_global_arrays(&self) {
        self.legacy(|| {
            // SAFETY: Assigns star/planet global array pointers.
            unsafe { c_extern::uqm_assign_star_planet_globals() }
        })
    }
    fn set_main_exited(&self, val: bool) {
        self.legacy(|| {
            // SAFETY: Sets the MainExited flag for C main() shutdown.
            crate::mainloop::ffi::set_main_exited(val)
        })
    }
}
