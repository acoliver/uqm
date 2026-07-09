//! FFI externs for C functions called from Rust-owned main().
//!
//! @plan PLAN-20260707-BINARY-INVERSION.P02
//! @requirement REQ-BI-002
//!
//! These declarations cover the init sequence, event pump, and teardown
//! functions that C `main()` (in `uqm.c`) currently owns. When the Rust
//! binary owns `main()`, these are called directly instead of through C.

use std::os::raw::{c_char, c_float, c_int};

// ===========================================================================
//  Init sequence — direct C library functions
// ===========================================================================

extern "C" {
    /// SDL pre-init (video subsystem). `sdl2_common.c:42`
    pub fn TFB_PreInit();

    /// Full C init sequence for Rust-owned main().
    /// Does option parsing, config loading, global setup, all subsystem init.
    /// Returns 0 on success, non-zero on failure, -1 for version/usage modes.
    /// @plan PLAN-20260707-BINARY-INVERSION.P05
    pub fn uqm_c_do_init(argc: c_int, argv: *mut *mut c_char) -> c_int;

    /// Log system init. `uqmlog.h:25`. Sets max log level.
    pub fn log_init(level: c_int);

    /// Thread-specific log setup. `uqmlog.c:147`
    pub fn log_initThreads();

    /// Task system init. `tasklib.c:120`
    pub fn InitTaskSystem();

    /// Alarm/timer system. `alarm.c:56`
    pub fn Alarm_init();

    /// Callback system. `callback.c:63`
    pub fn Callback_init();

    /// Color map system. `cmap.c:72`
    pub fn InitColorMaps();

    /// Set gamma correction. `options.c:639`. Returns 0 on success.
    pub fn setGammaCorrection(gamma: c_float) -> bool;

    /// Initialize SDL-based input. `input.c:265`. Returns 0 on success.
    pub fn TFB_InitInput(driver: c_int, flags: c_int) -> c_int;

    /// Initialize graphics driver. `sdl_common.c:95`
    /// Returns 0 on success, non-zero on failure.
    pub fn TFB_InitGraphics(
        driver: c_int,
        flags: c_int,
        renderer: *const c_char,
        width: c_int,
        height: c_int,
    ) -> c_int;

    /// Memory init. `w_memlib.c:33`
    pub fn mem_init() -> bool;

    /// Memory uninit. Returns true on success.
    pub fn mem_uninit() -> bool;

    /// IO init. `setup.c:311`. Returns 0 on success.
    pub fn initIO() -> c_int;

    /// IO uninit.
    pub fn uninitIO();

    /// Thread system init. `threadlib.h:56`
    pub fn InitThreadSystem();

    /// Thread system uninit.
    pub fn UnInitThreadSystem();

    /// Reap finished threads. `threadlib.h:157`
    /// With Rust thread backend, only reaps pendingDeath entries.
    pub fn ProcessThreadLifecycles();

    /// Time system init. `timecommon.c`
    pub fn InitTimeSystem();

    /// Time system uninit.
    pub fn UnInitTimeSystem();

    /// Communication init. `comm.c` (Rust-backed under USE_RUST_COMM)
    pub fn init_communication();

    /// Communication uninit.
    pub fn uninit_communication();
}

// ===========================================================================
//  Init/teardown via C bridge (rust_bridge_main2.c)
// ===========================================================================

extern "C" {
    // -- Global option setters --
    pub fn uqm_set_snddriver(val: c_int);
    pub fn uqm_set_soundflags(val: c_int);
    pub fn uqm_set_player_control_template(idx: c_int, val: c_int);
    pub fn uqm_set_opt3doMusic(val: c_int);
    pub fn uqm_set_optRemixMusic(val: c_int);
    pub fn uqm_set_optSpeech(val: c_int);
    pub fn uqm_set_optSubtitles(val: c_int);
    pub fn uqm_set_optStereoSFX(val: c_int);
    pub fn uqm_set_optKeepAspectRatio(val: c_int);
    pub fn uqm_set_optWhichCoarseScan(val: c_int);
    pub fn uqm_set_optWhichMenu(val: c_int);
    pub fn uqm_set_optWhichFonts(val: c_int);
    pub fn uqm_set_optWhichIntro(val: c_int);
    pub fn uqm_set_optWhichShield(val: c_int);
    pub fn uqm_set_optSmoothScroll(val: c_int);
    pub fn uqm_set_optMeleeScale(val: c_int);
    pub fn uqm_set_optGamma(val: c_float);
    pub fn uqm_set_optAddons(addons: *const *const c_char);
    pub fn uqm_set_musicVolumeScale(val: c_float);
    pub fn uqm_set_sfxVolumeScale(val: c_float);
    pub fn uqm_set_speechVolumeScale(val: c_float);

    // -- Config loading --
    pub fn uqm_init_config_dir(config_dir: *const c_char);
    pub fn uqm_load_resource_index();

    // -- Directory preparation --
    pub fn uqm_prepare_content_dir(
        content: *const c_char,
        addon: *const c_char,
        exec: *const c_char,
    );
    pub fn uqm_prepare_melee_dir();
    pub fn uqm_prepare_save_dir();
    pub fn uqm_prepare_shadow_addons(addons: *const *const c_char);
    pub fn uqm_unprepare_all_dirs();

    // -- Init/teardown wrappers --
    pub fn uqm_log_init_threads();
    pub fn uqm_init_task_system();
    pub fn uqm_alarm_init();
    pub fn uqm_callback_init();
    pub fn uqm_init_color_maps();
    pub fn uqm_cleanup_task_system();
    pub fn uqm_callback_uninit();
    pub fn uqm_alarm_uninit();

    // -- Control template cleanup --
    pub fn uqm_remove_old_control_templates();

    // -- Config options parsing --
    /// Parse config/uqm.cfg into the C global `options` struct.
    pub fn uqm_get_user_config_options();

    // -- Addon cleanup --
    /// Free options.addons (uqm.c:507).
    pub fn uqm_free_options_addons();

    // -- Input setup --
    pub fn uqm_set_player_controls(p1: c_int, p2: c_int);
    pub fn uqm_setup_input_vectors();
}

// ===========================================================================
//  Event pump — called from Rust main thread (and from DoInput under
//  RUST_OWNS_MAIN)
// ===========================================================================

extern "C" {
    /// SDL event polling. `sdl_common.c:208`
    pub fn TFB_ProcessEvents();

    /// Utility key handling (fullscreen, abort, debug).
    /// `starcon.c:136`
    pub fn ProcessUtilityKeys();

    /// Drain DCQ, present frame. `dcqueue.c:323`
    pub fn TFB_FlushGraphics();
}

// ===========================================================================
//  Teardown sequence — direct C library functions
// ===========================================================================

extern "C" {
    /// Input teardown. `input.c:295`
    pub fn TFB_UninitInput();

    /// Audio teardown.
    pub fn unInitAudio();

    /// Purge dangling graphics resources. `dcqueue.c:627`
    pub fn TFB_PurgeDanglingGraphics();

    /// Color map teardown. `cmap.c:89`
    pub fn UninitColorMaps();

    /// Graphics teardown. `sdl_common.c:181`
    pub fn TFB_UninitGraphics();

    /// Task system cleanup. `tasklib.c:130`
    pub fn CleanupTaskSystem();
}

// ===========================================================================
//  Graphics constants (from gfx_common.h)
// ===========================================================================

/// SDL pure software renderer (no OpenGL)
pub const TFB_GFXDRIVER_SDL_PURE: c_int = 0;
/// SDL with OpenGL
pub const TFB_GFXDRIVER_SDL_OPENGL: c_int = 1;

/// Fullscreen flag
pub const TFB_GFXFLAGS_FULLSCREEN: c_int = 1 << 0;
/// Scanlines flag
pub const TFB_GFXFLAGS_SCANLINES: c_int = 1 << 1;
/// Show FPS flag
pub const TFB_GFXFLAGS_SHOWFPS: c_int = 1 << 2;
/// xBRZ 3x scaler
pub const TFB_GFXFLAGS_SCALE_XBRZ3: c_int = 1 << 8;
/// xBRZ 4x scaler
pub const TFB_GFXFLAGS_SCALE_XBRZ4: c_int = 1 << 9;
/// HQxx scaler
pub const TFB_GFXFLAGS_SCALE_HQXX: c_int = 1 << 10;

// ===========================================================================
//  Input constants
// ===========================================================================

/// SDL input driver
pub const TFB_INPUTDRIVER_SDL: c_int = 0;

// ===========================================================================
//  Control template constants (from controls.h)
// ===========================================================================

/// Keyboard control template 1
pub const CONTROL_TEMPLATE_KB_1: c_int = 0;
/// Joystick control template 1
pub const CONTROL_TEMPLATE_JOY_1: c_int = 1;

// ===========================================================================
//  Out-of-scope for Phase 1 (documented)
// ===========================================================================
//
// The following C main() calls are intentionally NOT bridged in P02:
//
// - StartThread(Starcon2Main, ...) — In Rust-owned main(), the game loop runs
//   directly on the main thread (no game thread). StartThread is not needed.
// - Network_init / NetManager_init — NETPLAY support is Phase 2 scope.
//   These are #ifdef NETPLAY in C main() and absent from this build.
// - QuitPosted global access — checked via TFB_ProcessEvents return side effect;
//   Rust main loop checks GameActive via the game loop's existing mechanisms.
// - MainExited global — set by the game loop's shutdown_game_kernel() via
//   set_main_exited(), already declared in c_extern.rs.
// - HibernateThread — replaced by the Rust game loop's own pacing.
