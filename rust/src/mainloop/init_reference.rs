//! C startup sequence — reference documentation ONLY.
//!
//! **This module contains NO executable code.** It exists solely to
//! document the C `main()` startup sequence that `sc2/src/uqm.c` owns, so
//! that future Rust main-loop work has an accurate reference for what
//! happens before `Starcon2Main()` (and therefore before any Rust
//! `rust_game_loop()` entry point) is ever invoked.
//!
//! # Architectural Rule
//!
//! **Rust must NOT call, replicate, or wrap the C startup sequence.**
//!
//! Per `PLAN-20260707-MAINLOOP.P04` (REQ-ML-002), the full init sequence
//! — option parsing, config loading, graphics/input/audio subsystem
//! bring-up, threading, and shutdown — remains entirely in C `main()`.
//! Rust only replaces the *body* of `Starcon2Main()` (the per-frame game
//! loop), which is launched as a separate thread *after* startup is
//! complete. Anything that must happen before the first frame belongs to
//! C `main()`, not to Rust.
//!
//! # Verified C Init Order
//!
//! The sequence below is verified against `sc2/src/uqm.c` lines 283–452.
//! Every step is executed by C `main()` on the main thread, in this exact
//! order, before `StartThread(Starcon2Main, ...)` is called.
//!
//! ## 1. Option parsing & logging bootstrap
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 1 | `parseOptions(argc, argv, &options)` | CLI flags parsed first; logging not yet available |
//! | 2 | `log_init(15)` | Logging subsystem initialized after options parsed |
//! | 3 | `rust_bridge_init()` *(USE_RUST_BRIDGE only)* | Rust bridge logging bootstrap |
//! | 4 | `freopen(options.logFile, "w", stderr)` | Optional: redirect stderr to log file |
//! | 5 | Version / usage / error short-circuits | `runMode_version` prints version and exits; `runMode_usage` prints help and exits |
//!
//! ## 2. Low-level system & I/O
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 6 | `TFB_PreInit()` | Graphics pre-init (platform probing) |
//! | 7 | `mem_init()` | Memory allocator init |
//! | 8 | `InitThreadSystem()` | Threading primitives init |
//! | 9 | `log_initThreads()` | Per-thread logging init |
//! | 10 | `initIO()` | File I/O subsystem |
//! | 11 | `prepareConfigDir(options.configDir)` | Resolve config directory |
//! | 12 | `PlayerControls[0..1] = ...` | Default keyboard/joy control templates |
//!
//! ## 3. Config & content loading
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 13 | `LoadResourceIndex(configDir, "uqm.cfg", "config.")` | Load `uqm.cfg` *(skipped in safe mode)* |
//! | 14 | `getUserConfigOptions(&options)` | Merge config-file values into options struct |
//! | 15 | Remove legacy `config.keys.N.name` entries | Backwards-compat cleanup |
//! | 16 | Copy options into globals | `snddriver`, `soundflags`, `opt3doMusic`, `optRemixMusic`, `optSpeech`, `optWhichCoarseScan`, `optWhichMenu`, `optWhichFonts`, `optWhichIntro`, `optWhichShield`, `optSmoothScroll`, `optMeleeScale`, `optKeepAspectRatio`, `optSubtitles`, `optStereoSFX`, volume scales, `optAddons` |
//! | 17 | `prepareContentDir(...)` | Resolve content/addon directories |
//! | 18 | `prepareMeleeDir()` | Melee supermelee dir |
//! | 19 | `prepareSaveDir()` | Save-game dir |
//! | 20 | `prepareShadowAddons(options.addons)` | Shadow addon resolution |
//!
//! ## 4. Timing, tasks, callbacks, network
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 21 | `InitTimeSystem()` | High-resolution timer init |
//! | 22 | `InitTaskSystem()` | Task (coroutine) scheduler init |
//! | 23 | `Alarm_init()` | Alarm/timer callback system |
//! | 24 | `Callback_init()` | Global callback registry init |
//! | 25 | `Network_init()` *(NETPLAY only)* | Network subsystem |
//! | 26 | `NetManager_init()` *(NETPLAY only)* | Netplay session manager |
//!
//! ## 5. Graphics
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 27 | Resolve `gfxDriver` | `TFB_GFXDRIVER_SDL_OPENGL` or `TFB_GFXDRIVER_SDL_PURE` based on `options.opengl` |
//! | 28 | Compute `gfxFlags` | Scaler + optional `FULLSCREEN`, `SCANLINES`, `SHOWFPS` |
//! | 29 | `TFB_InitGraphics(driver, flags, backend, w, h)` | **Full graphics subsystem init** — creates window, renderer, SDL context |
//! | 30 | `setGammaCorrection(...)` / `optGamma = 1.0` | Apply gamma if requested |
//! | 31 | `InitColorMaps()` | Color map / palette tables |
//! | 32 | `init_communication()` | Inter-thread communication channels |
//!
//! ## 6. Input
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 33 | `assert(sizeof(int[NUM_TEMPLATES*NUM_KEYS]) == ...)` | Compile-time array-layout check |
//! | 34 | `TFB_SetInputVectors(menu, key, ...)` | Wire input vectors into immediate input state |
//! | 35 | `TFB_InitInput(TFB_INPUTDRIVER_SDL, 0)` | **Input subsystem init** — SDL event capture begins |
//!
//! ## 7. Launch game thread
//!
//! | # | Call | Notes |
//! |---|------|-------|
//! | 36 | `StartThread(Starcon2Main, NULL, 1024, "Starcon2Main")` | **Game loop thread starts here** — this is the Rust entry boundary |
//!
//! > **Note:** `initAudio(snddriver, soundflags)` is intentionally NOT
//! > called in `main()`. It is deferred into `Starcon2Main()` because it
//! > calls `AssignTask`, which cannot run on the main thread. This is why
//! > `snddriver`/`soundflags` are global. See the TODO comment in
//! > `uqm.c`. When Rust replaces `Starcon2Main()`, it inherits this
//! > responsibility.
//!
//! # Main-thread event pump (after thread launch)
//!
//! After `StartThread`, C `main()` enters its own loop (`uqm.c:456-472`)
//! that is **distinct from the game loop**. Rust does not own this either.
//! Its responsibilities:
//!
//! 1. If `QuitPosted`: call `SignalStopMainThread()` (up to 2000 retries).
//! 2. Else if `!GameActive`: `HibernateThread(ONE_SECOND/4)` to throttle.
//! 3. `TFB_ProcessEvents()` — pump SDL/window events.
//! 4. `ProcessUtilityKeys()` — global hotkeys.
//! 5. `ProcessThreadLifecycles()` — task scheduler tick.
//! 6. `TFB_FlushGraphics()` — present frame.
//!
//! # Shutdown sequence (C `main()`, after game thread exits)
//!
//! Owned entirely by C `main()` (`uqm.c:479-507`), executed only if
//! `MainExited` is true. Rust does not participate in shutdown.
//!
//! 1. `TFB_UninitInput()`
//! 2. `unInitAudio()`
//! 3. `uninit_communication()`
//! 4. `TFB_PurgeDanglingGraphics()`
//! 5. `UninitColorMaps()`
//! 6. `TFB_UninitGraphics()`
//! 7. `NetManager_uninit()` *(NETPLAY only)*
//! 8. `Network_uninit()` *(NETPLAY only)*
//! 9. `Callback_uninit()`
//! 10. `Alarm_uninit()`
//! 11. `CleanupTaskSystem()`
//! 12. `UnInitTimeSystem()`
//! 13. `unprepareAllDirs()`
//! 14. `uninitIO()`
//! 15. `UnInitThreadSystem()`
//! 16. `mem_uninit()`
//!
//! # What Rust IS responsible for
//!
//! Once P05+ wire `rust_game_loop()` into the `Starcon2Main()` thread
//! body, Rust owns only what `Starcon2Main()` historically did:
//!
//! - `initAudio(snddriver, soundflags)` — audio bring-up (deferred from main)
//! - `LoadKernel()` — load game kernel data
//! - Splash screen + kernel init
//! - The per-frame activity state machine (`ActivityKind`, `next_activity`)
//! - Game clock, input dispatch, rendering dispatch within a frame
//!
//! Everything above — options, config, graphics window, input driver,
//! threading, networking, shutdown — stays in C.
//!
//! # Verification (P04 gate)
//!
//! As of this phase, the following have been verified against the built
//! `sc2/uqm` binary (v0.8.0, arm64 Mach-O):
//!
//! - `./uqm --help` prints version + options (proves C `main()` + startup
//!   path works unchanged with all `USE_RUST_*` flags enabled).
//! - All `Starcon2Main`-specific init symbols are present and exported:
//!   `Starcon2Main`, `LoadKernel`, `StartGame`, `InitGameStructures`,
//!   `InitGameClock`, `AddInitialGameEvents`, `SetPlayerInputAll`,
//!   `initAudio`.
//! - Startup-path symbols are present: `TFB_InitGraphics`, `TFB_InitInput`,
//!   `Callback_init`, `Network_init`, `NetManager_init`.
//! - No `uqm_rust_safe_startup` wrapper symbol exists in the binary —
//!   Rust does not wrap or replicate C startup.
//!
//! @plan PLAN-20260707-MAINLOOP.P04
//! @req REQ-ML-002
