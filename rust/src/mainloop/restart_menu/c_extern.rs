//! FFI extern declarations for C functions used by the restart menu.
//!
//! @plan PLAN-20260707-RESTARTMENU.P05
//! @requirement REQ-RM-004

use std::os::raw::{c_char, c_int, c_short, c_void};

// ===========================================================================
//  Constants (from C headers, verified against source)
// ===========================================================================

/// Menu key indices (controls.h enum, starting after KEY_FULLSCREEN=4).
pub const KEY_MENU_UP: u8 = 5;
pub const KEY_MENU_DOWN: u8 = 6;
pub const KEY_MENU_LEFT: u8 = 7;
pub const KEY_MENU_RIGHT: u8 = 8;
pub const KEY_MENU_SELECT: u8 = 9;

/// `ONE_SECOND` (timelib.h:35) — LCM of all frame-rate fractions.
pub const ONE_SECOND: u32 = 840;

/// `NORMAL_VOLUME` (sndlib.h:66).
pub const NORMAL_VOLUME: u32 = 160;

/// `FlashState_fadeIn` (flash.h:99).
pub const FLASH_STATE_FADE_IN: u32 = 0;

/// `FadeAllToWhite` (gfxlib.h:282).
pub const FADE_ALL_TO_WHITE: u32 = 250;
/// `FadeAllToBlack` (gfxlib.h:284, after FadeSomeToWhite=251).
pub const FADE_ALL_TO_BLACK: u32 = 252;
/// `FadeAllToColor` (gfxlib.h:285).
pub const FADE_ALL_TO_COLOR: u32 = 253;

/// `MENU_SOUND_UP` (sounds.h:53).
pub const MENU_SOUND_UP: u16 = 1 << 0;
/// `MENU_SOUND_DOWN` (sounds.h:54).
pub const MENU_SOUND_DOWN: u16 = 1 << 1;
/// `MENU_SOUND_SELECT` (sounds.h:57).
pub const MENU_SOUND_SELECT: u16 = 1 << 4;

/// C `Color` struct (gfxlib.h:26) — four `BYTE` fields, `#[repr(C)]`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// `BUILD_COLOR(MAKE_RGB15(0x1F, 0x1F, 0x1F), 0x0F)` — the gray
    /// background used after the Utwig bomb self-destruct fade.
    /// RGB15(0x1F, 0x1F, 0x1F) = (255, 255, 255) with ramp 0x0F.
    pub const fn white_gray() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 0x0F,
        }
    }
}

/// C `POINT` struct (gfxlib.h:157) — two `COORD` (`SWORD` = i16) fields.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

// ===========================================================================
//  C wrappers from rust_bridge_restart.c
// ===========================================================================

extern "C" {
    #[cfg(test)]
    pub fn uqm_get_utwig_bomb_on_ship() -> u8;
    #[cfg(test)]
    pub fn uqm_set_utwig_bomb_on_ship(v: u8);
    #[cfg(test)]
    pub fn uqm_get_utwig_bomb() -> u8;
    #[cfg(test)]
    pub fn uqm_set_player_control(player: u8, control: u16);
    #[cfg(test)]
    pub fn uqm_get_pulsed_menu_key(key_index: u8) -> c_int;
    #[cfg(test)]
    pub fn uqm_get_mouse_button_down() -> c_int;
    #[cfg(test)]
    pub fn uqm_sleep_thread_until(time: u32);
    #[cfg(test)]
    pub fn uqm_sleep_thread(duration: u32);
    #[cfg(test)]
    pub fn uqm_set_game_paused(val: c_int);
    #[cfg(test)]
    pub fn uqm_reinit_race_queues();
    #[cfg(test)]
    pub fn uqm_assign_star_planet_globals();
    #[cfg(test)]
    pub fn uqm_do_popup_window_msg(string_id: u16);

    // --- Menu state lifecycle (rust_bridge_restart.c) ---
    #[cfg(test)]
    pub fn uqm_get_menu_cur_frame(pms: *mut c_void) -> usize;
    #[cfg(test)]
    pub fn uqm_get_menu_priv_data(pms: *mut c_void) -> *mut c_void;
    #[cfg(test)]
    pub fn uqm_set_menu_flash_context(pms: *mut c_void, ctx: usize);
    #[cfg(test)]
    pub fn uqm_set_menu_initialized(pms: *mut c_void, val: i16);
    #[cfg(test)]
    pub fn uqm_set_menu_cur_state(pms: *mut c_void, state: u8);
    #[cfg(test)]
    pub fn uqm_set_menu_cur_frame(pms: *mut c_void, frame: usize);
    #[cfg(test)]
    pub fn uqm_set_menu_h_music(pms: *mut c_void, handle: usize);
    #[cfg(test)]
    pub fn uqm_create_menu_state() -> *mut c_void;
    #[cfg(test)]
    pub fn uqm_set_menu_priv_data(pms: *mut c_void, data: *mut c_void);
    #[cfg(test)]
    pub fn uqm_destroy_menu_state(pms: *mut c_void);

    // --- Menu drawing (de-staticized in restart.c under USE_RUST_RESTART) ---
    #[cfg(test)]
    pub fn DrawRestartMenuGraphic(pms: *mut c_void);
    #[cfg(test)]
    pub fn DrawRestartMenu(pms: *mut c_void, new_state: u8, frame: *mut c_void);
}
/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_utwig_bomb_on_ship() -> u8 {
    (crate::mainloop::restart_menu::menu_abi::uqm_get_utwig_bomb_on_ship()) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_utwig_bomb_on_ship(v: u8) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_utwig_bomb_on_ship(v as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_utwig_bomb() -> u8 {
    (crate::mainloop::restart_menu::menu_abi::uqm_get_utwig_bomb()) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_player_control(player: u8, control: u16) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_player_control(player as _, control as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_pulsed_menu_key(key_index: u8) -> c_int {
    (crate::mainloop::restart_menu::menu_abi::uqm_get_pulsed_menu_key(key_index as _)) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_mouse_button_down() -> c_int {
    (crate::mainloop::restart_menu::menu_abi::uqm_get_mouse_button_down()) as _
}

pub use crate::time::counter_abi::GetTimeCounter as uqm_get_time_counter;

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_sleep_thread_until(time: u32) {
    crate::mainloop::restart_menu::menu_abi::uqm_sleep_thread_until(time as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_sleep_thread(duration: u32) {
    crate::mainloop::restart_menu::menu_abi::uqm_sleep_thread(duration as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_game_paused(val: c_int) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_game_paused(val as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_reinit_race_queues() {
    crate::mainloop::restart_menu::menu_abi::uqm_reinit_race_queues();
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_assign_star_planet_globals() {
    crate::mainloop::restart_menu::menu_abi::uqm_assign_star_planet_globals();
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_do_popup_window_msg(string_id: u16) {
    crate::mainloop::restart_menu::menu_abi::uqm_do_popup_window_msg(string_id as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_menu_cur_frame(pms: *mut c_void) -> usize {
    (crate::mainloop::restart_menu::menu_abi::uqm_get_menu_cur_frame(pms as _)) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_menu_priv_data(pms: *mut c_void) -> *mut c_void {
    (crate::mainloop::restart_menu::menu_abi::uqm_get_menu_priv_data(pms as _)) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_menu_flash_context(pms: *mut c_void, ctx: usize) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_menu_flash_context(pms as _, ctx as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_menu_initialized(pms: *mut c_void, val: i16) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_menu_initialized(pms as _, val as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_menu_cur_state(pms: *mut c_void, state: u8) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_menu_cur_state(pms as _, state as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_menu_cur_frame(pms: *mut c_void, frame: usize) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_menu_cur_frame(pms as _, frame as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_menu_h_music(pms: *mut c_void, handle: usize) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_menu_h_music(pms as _, handle as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_create_menu_state() -> *mut c_void {
    (crate::mainloop::restart_menu::menu_abi::uqm_create_menu_state()) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_set_menu_priv_data(pms: *mut c_void, data: *mut c_void) {
    crate::mainloop::restart_menu::menu_abi::uqm_set_menu_priv_data(pms as _, data as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_destroy_menu_state(pms: *mut c_void) {
    crate::mainloop::restart_menu::menu_abi::uqm_destroy_menu_state(pms as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn draw_restart_menu_graphic(pms: *mut c_void) {
    crate::mainloop::restart_menu::render_abi::DrawRestartMenuGraphic(pms as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn draw_restart_menu(pms: *mut c_void, new_state: u8, frame: *mut c_void) {
    crate::mainloop::restart_menu::render_abi::DrawRestartMenu(
        pms as _,
        new_state as _,
        frame as _,
    );
}

// ===========================================================================
//  C wrappers from rust_bridge_mainloop.c (reused)
// ===========================================================================

#[cfg(not(test))]
pub use crate::game_init::macro_abi::get_current_activity;
#[cfg(test)]
extern "C" {
    pub fn get_current_activity() -> u16;
}
extern "C" {

    #[cfg(test)]
    pub fn set_current_activity(v: u16);
    #[cfg(test)]
    pub fn get_last_activity() -> u16;
    #[cfg(test)]
    pub fn set_last_activity(v: u16);
    #[cfg(test)]
    pub fn set_next_activity(v: u16);
    #[cfg(test)]
    pub fn uqm_get_crew_enlisted() -> u16;
    #[cfg(test)]
    pub fn set_main_exited(b: c_int);
}
/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn set_current_activity(v: u16) {
    crate::mainloop::state_abi::set_current_activity(v as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn get_last_activity() -> u16 {
    (crate::mainloop::state_abi::get_last_activity()) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn set_last_activity(v: u16) {
    crate::mainloop::state_abi::set_last_activity(v as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn set_next_activity(v: u16) {
    crate::mainloop::state_abi::set_next_activity(v as _);
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn uqm_get_crew_enlisted() -> u16 {
    (crate::mainloop::state_abi::uqm_get_crew_enlisted()) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
pub unsafe fn set_main_exited(b: c_int) {
    crate::mainloop::state_abi::set_main_exited(b as _);
}

// ===========================================================================
//  Directly linkable C functions (no wrapper needed)
// ===========================================================================

#[cfg(not(test))]
pub use crate::game_init::cleanup::FreeGameData;
#[cfg(test)]
extern "C" {
    pub fn FreeGameData();
}
#[cfg(not(test))]
pub use crate::ui::settings::StopMusic;
#[cfg(test)]
extern "C" {
    pub fn StopMusic();
}
pub use crate::sound::heart_ffi::fade_music as FadeMusic;
extern "C" {
    // --- Graphics ---
    /// The whole-screen drawing context created during graphics startup.
    pub static ScreenContext: *mut c_void;
    /// `DWORD FadeScreen(ScreenFadeType, SIZE)` — SIZE = SWORD = i16.
    pub fn FadeScreen(fade_type: u32, time_interval: c_short) -> u32;
    /// `CONTEXT SetContext(CONTEXT)` — returns the previous context.
    pub fn SetContext(ctx: *mut c_void) -> *mut c_void;
    pub fn ClearDrawable();
    pub fn FlushColorXForms();
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn ScreenTransition(a: c_int, b: *mut c_void);
    /// `Color SetContextBackGroundColor(Color)` — Color is a 4-byte struct.
    #[allow(clashing_extern_declarations)]
    pub fn SetContextBackGroundColor(color: Color) -> Color;
    /// `DWORD SeedRandomNumbers(void)` — returns a seed value.
    pub fn SeedRandomNumbers() -> u32;

    // --- Resources ---
    // LoadGraphic and LoadMusic are macros that expand to LoadXXXInstance,
    // which take RESOURCE = const char* and return a handle.
    pub fn LoadGraphicInstance(res: *const c_char) -> *mut c_void;
    pub fn LoadMusicInstance(res: *const c_char) -> *mut c_void;
    pub fn CaptureDrawable(load_result: *mut c_void) -> *mut c_void;
    /// `BOOLEAN DestroyDrawable(DRAWABLE)` returns an unsigned int.
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn DestroyDrawable(handle: *mut c_void) -> u32;
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn ReleaseDrawable(handle: *mut c_void) -> *mut c_void;
    /// `FRAME SetAbsFrameIndex(FRAME, COUNT)` — COUNT = u16.
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;

    // --- Menu ---
    pub fn SetMenuSounds(s0: u16, s1: u16);
    pub fn SetDefaultMenuRepeatDelay();
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn DoInput(p_input_state: *mut c_void, reset_input: c_int);
    pub fn SetTransitionSource(src: *mut c_void);

    // --- Music ---

    /// `BOOLEAN DestroyMusic(MUSIC_REF)` — returns c_int.
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn DestroyMusic(handle: *mut c_void) -> c_int;
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn PlayMusic(handle: *mut c_void, do_loop: c_int, volume: u8);
    /// `DWORD FadeMusic(BYTE end_vol, SIZE TimeInterval)` — returns a TimeCount.
    // --- Flash ---
    pub fn Flash_createOverlay(context: *mut c_void, a: *mut c_void, b: *mut c_void)
        -> *mut c_void;
    pub fn Flash_process(ctx: *mut c_void);
    pub fn Flash_pause(ctx: *mut c_void);
    pub fn Flash_continue(ctx: *mut c_void);
    pub fn Flash_start(ctx: *mut c_void);
    pub fn Flash_terminate(ctx: *mut c_void);
    pub fn Flash_setMergeFactors(ctx: *mut c_void, a: c_int, b: c_int, c: c_int);
    pub fn Flash_setSpeed(ctx: *mut c_void, a: u32, b: u32, c: u32, d: u32);
    pub fn Flash_setFrameTime(ctx: *mut c_void, t: u32);
    pub fn Flash_setState(ctx: *mut c_void, state: u32, t: u32);
    /// `void Flash_setOverlay(FlashContext*, const POINT*, FRAME)` — POINT must be valid.
    pub fn Flash_setOverlay(ctx: *mut c_void, origin: *const Point, frame: *mut c_void);

    // --- Lifecycle ---
    pub fn Melee();
    pub fn SetupMenu();

    pub fn Credits(victory: c_int);
    pub fn Victory();
    /// `void SplashScreen(void (*DoProcessing)(DWORD TimeOut))` — function pointer.
    pub fn SplashScreen(callback: Option<extern "C" fn(u32)>);
    pub fn DoPopupWindow(string: *const c_char);
}
pub use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

#[cfg(not(test))]
pub use draw_restart_menu_graphic as DrawRestartMenuGraphic;

#[cfg(not(test))]
pub use draw_restart_menu as DrawRestartMenu;
