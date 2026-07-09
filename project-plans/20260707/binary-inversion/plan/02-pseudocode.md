# Binary Inversion — P02 Pseudocode

## Rust main.rs target structure

```rust
fn main() -> Result<()> {
    // ── Phase 1: Parse options ──
    let cli = Cli::parse();
    log_init(15);
    
    let options = parse_config(&cli)?;  // replaces parseOptions + getUserConfigOptions
    
    // ── Phase 2: Init sequence ──
    // Replaces C main() lines 348-452
    tfb_preinit();
    mem_init();
    init_thread_system();
    log_init_threads();
    init_io();
    prepare_config_dir(options.config_dir);
    
    if !options.safe_mode {
        load_resource_index(configDir, "uqm.cfg", "config.");
        apply_config_options(&options);  // sets C globals from parsed options
    }
    
    set_player_controls(options.player1_control, options.player2_control);
    set_audio_globals(options.sound_driver, options.sound_quality);
    set_game_globals(&options);  // opt3doMusic, optRemixMusic, etc.
    
    prepare_content_dir(options.content_dir, options.addon_dir, &argv[0]);
    prepare_melee_dir();
    prepare_save_dir();
    prepare_shadow_addons(&options.addons);
    
    init_time_system();
    init_task_system();
    alarm_init();
    callback_init();
    
    let gfx_driver = if options.opengl { TFB_GFXDRIVER_SDL_OPENGL } else { TFB_GFXDRIVER_SDL_PURE };
    let gfx_flags = build_gfx_flags(&options);
    tfb_init_graphics(gfx_driver, gfx_flags, options.graphics_backend, width, height);
    
    if options.gamma.set {
        set_gamma_correction(options.gamma.value);
    }
    
    init_color_maps();
    init_communication();
    
    // NOTE: initAudio is called from inside rust_game_loop (existing behavior)
    // It was moved there because it uses AssignTask. Keep as-is for now.
    
    tfb_set_input_vectors(...);
    tfb_init_input(TFB_INPUTDRIVER_SDL, 0);
    
    // ── Phase 3: Game loop + event pump (SAME thread) ──
    // Replaces StartThread(Starcon2Main) + main thread event pump
    //
    // KEY CHANGE: No StartThread. The game loop runs directly on main thread.
    // The event pump is called INSIDE the game loop, after each frame.
    
    run_game_with_event_pump();
    
    // ── Phase 4: Teardown ──
    // Replaces C main() lines 477-504
    tfb_uninit_input();
    uninit_audio();
    uninit_communication();
    tfb_purge_dangling_graphics();
    uninit_color_maps();
    tfb_uninit_graphics();
    callback_uninit();
    alarm_uninit();
    cleanup_task_system();
    uninit_time_system();
    unprepare_all_dirs();
    uninit_io();
    uninit_thread_system();
    mem_uninit();
    
    Ok(())
}
```

## Game loop with interleaved event pump

```rust
fn run_game_with_event_pump() {
    // The game loop (rust_game_loop) already exists.
    // It currently calls init_audio, load_kernel, splash, then
    // the activity state machine loop.
    //
    // We need to interleave the event pump INTO the game loop.
    // Two approaches:
    
    // APPROACH A (simplest): Wrap the existing rust_game_loop
    // and pump events from a callback.
    //
    // The GameLoopOps trait gets a new method:
    //   fn pump_events(&self)  // calls TFB_ProcessEvents + ProcessUtilityKeys + TFB_FlushGraphics
    //
    // The game loop calls ops.pump_events() at appropriate points:
    //   - After each activity dispatch (VisitStarBase, Battle, etc.)
    //   - During splash screen
    //   - During the start_game menu loop
    
    // APPROACH B (cleaner, more work): Restructure the loop so
    // the event pump is called explicitly between frames.
    //
    // For Phase 1, use Approach A — it's less invasive.
}
```

## Event pump wrapper

```rust
// In CffiOps impl:
fn pump_events(&self) {
    unsafe {
        c_extern::TFB_ProcessEvents();
        c_extern::ProcessUtilityKeys();
        // NO ProcessThreadLifecycles — eliminated
        c_extern::TFB_FlushGraphics();
    }
}

// Also need a quit checker:
fn quit_posted(&self) -> bool {
    unsafe { c_extern::QuitPosted() != 0 }
}
```

## FFI extern module (c_extern additions)

```rust
extern "C" {
    // Init
    pub fn TFB_PreInit();
    pub fn log_initThreads();
    pub fn prepareConfigDir(configDir: *const c_char);
    pub fn LoadResourceIndex(dir: *mut c_void, rmpfile: *const c_char, prefix: *const c_char);
    pub fn prepareContentDir(content: *const c_char, addon: *const c_char, exec: *const c_char);
    pub fn prepareMeleeDir();
    pub fn prepareSaveDir();
    pub fn prepareShadowAddons(addons: *const *const c_char);
    pub fn InitTaskSystem();
    pub fn Alarm_init();
    pub fn Callback_init();
    pub fn InitColorMaps();
    pub fn setGammaCorrection(gamma: f32) -> c_int;
    pub fn TFB_SetInputVectors(menu: *mut c_int, nmenu: c_int, key: *mut c_int, ntemplates: c_int, nkeys: c_int);
    pub fn TFB_InitInput(driver: c_int, flags: c_int);
    pub fn TFB_InitGraphics(driver: c_int, flags: c_int, renderer: *const c_char, w: c_int, h: c_int) -> c_int;
    
    // Event pump
    pub fn TFB_ProcessEvents();
    pub fn ProcessUtilityKeys();
    pub fn TFB_FlushGraphics();
    
    // Teardown
    pub fn TFB_UninitInput();
    pub fn unInitAudio();
    pub fn uninit_communication();
    pub fn TFB_PurgeDanglingGraphics();
    pub fn UninitColorMaps();
    pub fn TFB_UninitGraphics();
    pub fn Callback_uninit();
    pub fn Alarm_uninit();
    pub fn CleanupTaskSystem();
    pub fn UnInitTimeSystem();
    pub fn unprepareAllDirs();
    pub fn uninitIO();
}
```

## Build changes

### Cargo.toml
```toml
[[bin]]
name = "uqm"
path = "src/main.rs"

[lib]
name = "uqm_rust"
crate-type = ["staticlib", "rlib"]
```

### C side: Disable C main()
```c
// In uqm.c:
#ifndef RUST_OWNS_MAIN
int main(int argc, char *argv[]) {
    ...existing code...
}
#endif
```

### Build system: Link C objects into Rust binary
The C build produces .o files. The Rust binary links them.
This requires updating build.rs to find and link the C object files.
```

## What changes in game_loop.rs

The `GameLoopOps` trait gets two new methods:
```rust
fn pump_events(&self);       // SDL events + graphics flush
fn quit_posted(&self) -> bool; // check if user hit quit
```

The game loop calls `ops.pump_events()` at key points. This replaces the
separate main-thread event pump loop entirely.
