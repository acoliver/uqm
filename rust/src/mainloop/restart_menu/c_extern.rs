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
    pub fn uqm_get_utwig_bomb_on_ship() -> u8;
    pub fn uqm_set_utwig_bomb_on_ship(v: u8);
    pub fn uqm_get_utwig_bomb() -> u8;
    pub fn uqm_set_player_control(player: u8, control: u16);
    pub fn uqm_get_pulsed_menu_key(key_index: u8) -> c_int;
    pub fn uqm_get_mouse_button_down() -> c_int;
    pub fn uqm_get_time_counter() -> u32;
    pub fn uqm_sleep_thread_until(time: u32);
    pub fn uqm_sleep_thread(duration: u32);
    pub fn uqm_set_game_paused(val: c_int);
    pub fn uqm_reinit_race_queues();
    pub fn uqm_assign_star_planet_globals();
    pub fn uqm_do_popup_window_msg(string_id: u16);

    // --- Menu state lifecycle (rust_bridge_restart.c) ---
    pub fn uqm_get_menu_cur_frame(pms: *mut c_void) -> usize;
    pub fn uqm_get_menu_priv_data(pms: *mut c_void) -> *mut c_void;
    pub fn uqm_set_menu_flash_context(pms: *mut c_void, ctx: usize);
    pub fn uqm_set_menu_initialized(pms: *mut c_void, val: i16);
    pub fn uqm_set_menu_cur_state(pms: *mut c_void, state: u8);
    pub fn uqm_set_menu_cur_frame(pms: *mut c_void, frame: usize);
    pub fn uqm_set_menu_h_music(pms: *mut c_void, handle: usize);
    pub fn uqm_create_menu_state() -> *mut c_void;
    pub fn uqm_set_menu_priv_data(pms: *mut c_void, data: *mut c_void);
    pub fn uqm_destroy_menu_state(pms: *mut c_void);

    // --- Menu drawing (de-staticized in restart.c under USE_RUST_RESTART) ---
    pub fn DrawRestartMenuGraphic(pms: *mut c_void);
    pub fn DrawRestartMenu(pms: *mut c_void, new_state: u8, frame: *mut c_void);
}

// ===========================================================================
//  C wrappers from rust_bridge_mainloop.c (reused)
// ===========================================================================

extern "C" {
    pub fn get_current_activity() -> u16;
    pub fn set_current_activity(v: u16);
    pub fn get_last_activity() -> u16;
    pub fn set_last_activity(v: u16);
    pub fn set_next_activity(v: u16);
    pub fn uqm_get_crew_enlisted() -> u16;
    pub fn set_main_exited(b: c_int);
}

// ===========================================================================
//  Directly linkable C functions (no wrapper needed)
// ===========================================================================

extern "C" {
    // --- Graphics ---
    /// `DWORD FadeScreen(ScreenFadeType, SIZE)` — SIZE = SWORD = i16.
    pub fn FadeScreen(fade_type: u32, time_interval: c_short) -> u32;
    /// `CONTEXT SetContext(CONTEXT)` — returns the previous context.
    pub fn SetContext(ctx: *mut c_void) -> *mut c_void;
    pub fn BatchGraphics();
    pub fn UnbatchGraphics();
    pub fn ClearDrawable();
    pub fn FlushColorXForms();
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn ScreenTransition(a: c_int, b: *mut c_void);
    /// `Color SetContextBackGroundColor(Color)` — Color is a 4-byte struct.
    pub fn SetContextBackGroundColor(color: Color) -> Color;
    /// `DWORD SeedRandomNumbers(void)` — returns a seed value.
    pub fn SeedRandomNumbers() -> u32;

    // --- Resources ---
    // LoadGraphic and LoadMusic are macros that expand to LoadXXXInstance,
    // which take RESOURCE = const char* and return a handle.
    pub fn LoadGraphicInstance(res: *const c_char) -> *mut c_void;
    pub fn LoadMusicInstance(res: *const c_char) -> *mut c_void;
    pub fn CaptureDrawable(load_result: *mut c_void) -> *mut c_void;
    /// `BOOLEAN DestroyDrawable(DRAWABLE)` — returns c_int.
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub fn DestroyDrawable(handle: *mut c_void) -> c_int;
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
    pub fn StopMusic();
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
    pub fn FadeMusic(end_vol: u8, time_interval: c_short) -> u32;

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
    pub fn FreeGameData();
    pub fn Introduction();
    pub fn Credits(victory: c_int);
    pub fn Victory();
    /// `void SplashScreen(void (*DoProcessing)(DWORD TimeOut))` — function pointer.
    pub fn SplashScreen(callback: Option<extern "C" fn(u32)>);
    pub fn DoPopupWindow(string: *const c_char);
}
